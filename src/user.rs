use std::collections::HashMap;

mod hash;

#[derive(Debug)]
pub struct User {
    api: String,
    token: String,
    sublanguageid: String,
}

#[derive(Debug, Clone)]
pub struct Subtitles {
    moviehash: String,
    filename: Option<String>,
    format: String,
    subid: String,
    rating: f64, // <1,10> + 0
    sublanguageid: String,
    b64gz: Option<String>,
}

impl User {
    // login to OS server, should be called always when starting talking with
    // server. It returns token, which must be used in later communication.
    pub fn login(
        username: &str,
        password: &str,
        language: &str,
    ) -> Result<User, String> {
        match User::login_request(username, password, language) {
            Ok(response) => {
                let mut user = User {
                    api: String::new(),
                    token: String::new(),
                    sublanguageid: language.to_owned(),
                };
                match response.get("token").and_then(|x| x.as_str()) {
                    Some(token) => user.token = token.to_string(),
                    None => return Err("failed to get token".to_owned()),
                }
                user.api = response
                    .get("data")
                    .and_then(|val| {
                        val.as_struct().and_then(|data| {
                            data.get("Content-Location")
                                .and_then(|cl| cl.as_str())
                        })
                    })
                    .unwrap_or(User::LOGIN_LOCATION)
                    .to_string();
                Ok(user)
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn search(&self, filenames: Vec<String>) -> Vec<Subtitles> {
        let mut query = Vec::new();
        let mut names = HashMap::new();
        for filename in filenames {
            match hash::os_hash(&filename) {
                Ok((h, s)) => {
                    names.insert(h.clone(), filename.clone());
                    query.push((h, s));
                }
                Err(e) => println!("{}", e.to_string()),
            }
        }
        match self.search_request(&query) {
            Ok(response) => self
                .choose_best(User::extract_subids(response))
                .values_mut()
                .map(|x| {
                    x.filename = names.get(&x.moviehash).map(|x| x.clone());
                    x.clone()
                })
                .collect::<Vec<Subtitles>>(),
            Err(e) => {
                println!("{}", e.to_string());
                Vec::new()
            }
        }
    }

    fn download_request(
        &self,
        sub_ids: Vec<String>,
    ) -> Result<xmlrpc::Value, xmlrpc::Error> {
        let request = xmlrpc::Request::new("DownloadSubtitles")
            .arg(self.token.as_str())
            .arg(xmlrpc::Value::Array(
                sub_ids
                    .into_iter()
                    .map(|x| xmlrpc::Value::from(x))
                    .collect(),
            ));
        Ok(request.call_url(&self.api)?)
    }

    pub fn download(&self, mut subs: Vec<Subtitles>) {
        let mut ids = Vec::new();
        for sub in &subs {
            ids.push(sub.subid.clone());
        }
        let request_result = self.download_request(ids);

        match request_result {
            Ok(value) => {
                let results =
                    value.get("data").and_then(|data| data.as_array());
                let mut extract_item = |item: &xmlrpc::Value| {
                    let item = item.as_struct();
                    if item.is_none() {
                        return;
                    }
                    let item = item.unwrap();
                    let b64gz = item.get("data").and_then(|x| x.as_str());
                    let subid =
                        item.get("idsubtitlefile").and_then(|x| x.as_str());
                    if b64gz.is_some() && subid.is_some() {
                        for mut x in &mut subs {
                            if x.subid == subid.unwrap() {
                                x.b64gz = b64gz.map(|x| x.to_owned());
                            }
                        }
                    }
                };
                results.map(|array| {
                    for item in array {
                        extract_item(item)
                    }
                });
            }
            Err(e) => println!("{}", e.to_string()),
        }
        //println!("{:?}", subs);
    }

    const LOGIN_LOCATION: &'static str =
        "https://api.opensubtitles.org/xml-rpc";

    fn login_request(
        username: &str,
        password: &str,
        language: &str,
    ) -> Result<xmlrpc::Value, xmlrpc::Error> {
        let request = xmlrpc::Request::new("LogIn")
            .arg(username)
            .arg(password)
            .arg(language)
            .arg("TemporaryUserAgent");
        Ok(request.call_url(User::LOGIN_LOCATION)?)
    }

    fn search_request(
        &self,
        query: &Vec<(String, u64)>,
    ) -> Result<xmlrpc::Value, xmlrpc::Error> {
        let mut prepared = Vec::new();

        for (moviehash, moviebytesizesize) in query {
            let entry = xmlrpc::Value::Struct(
                vec![
                    (
                        "moviehash".to_string(),
                        xmlrpc::Value::from(moviehash.as_str()),
                    ),
                    (
                        "moviebytesize".to_string(),
                        xmlrpc::Value::from(*moviebytesizesize as i64),
                    ),
                    (
                        "sublanguageid".to_string(),
                        xmlrpc::Value::from(self.sublanguageid.as_str()),
                    ),
                ]
                .into_iter()
                .collect(),
            );

            prepared.push(entry);
        }

        let request = xmlrpc::Request::new("SearchSubtitles")
            .arg(self.token.as_str())
            .arg(xmlrpc::Value::Array(prepared));

        Ok(request.call_url(self.api.as_str())?)
    }

    fn extract_subids(response: xmlrpc::Value) -> Vec<Subtitles> {
        let mut ret = Vec::new();
        let results = response.get("data").and_then(|data| data.as_array());
        let mut extract_item = |item: &xmlrpc::Value| {
            let item = item.as_struct();
            if item.is_none() {
                return;
            }
            let item = item.unwrap();
            let hash = item.get("MovieHash").and_then(|x| x.as_str());
            let subid = item.get("IDSubtitleFile").and_then(|x| x.as_str());
            let format = item.get("SubFormat").and_then(|x| x.as_str());
            let rating = item
                .get("SubRating")
                .and_then(|x| x.as_str().map(|x| x.parse().unwrap_or(0f64)));
            let lang = item.get("ISO639").and_then(|x| x.as_str());
            if hash.is_some()
                && subid.is_some()
                && lang.is_some()
                && format.is_some()
            {
                ret.push(Subtitles {
                    moviehash: hash.unwrap().to_owned(),
                    filename: None,
                    format: format.unwrap().to_owned(),
                    subid: subid.unwrap().to_owned(),
                    rating: rating.unwrap_or(0f64),
                    sublanguageid: lang.unwrap().to_owned(),
                    b64gz: None,
                })
            }
        };
        results.map(|array| {
            for item in array {
                extract_item(item)
            }
        });
        ret
    }

    fn choose_best(
        &self,
        candidates: Vec<Subtitles>,
    ) -> HashMap<std::string::String, Subtitles> {
        let mut best = HashMap::new();
        for cand in candidates {
            if cand.sublanguageid != self.sublanguageid {
                continue;
            }
            let tmp = best.get(&cand.moviehash);
            if tmp.is_none() {
                best.insert(cand.moviehash.clone(), cand);
            } else {
                let tmp = tmp.unwrap();
                if tmp.rating < cand.rating {
                    best.insert(cand.moviehash.clone(), cand);
                }
            }
        }
        best
    }
}
