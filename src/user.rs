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
    subid: String,
    rating: f64, // <1,10> + 0
    sublanguageid: String,
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
        for filename in filenames {
            match hash::os_hash(&filename) {
                Ok(q) => query.push(q),
                Err(e) => println!("{}", e.to_string()),
            }
        }
        match self.search_request(&query) {
            Ok(response) => self
                .choose_best(User::extract_subids(response))
                .values()
                .map(|x| x.clone())
                .collect(),
            Err(e) => {
                println!("{}", e.to_string());
                Vec::new()
            }
        }
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
            let rating = item
                .get("SubRating")
                .and_then(|x| x.as_str().map(|x| x.parse().unwrap_or(0f64)));
            let lang = item.get("ISO639").and_then(|x| x.as_str());
            if hash.is_some() && subid.is_some() && lang.is_some() {
                ret.push(Subtitles {
                    moviehash: hash.unwrap().to_owned(),
                    subid: subid.unwrap().to_owned(),
                    rating: rating.unwrap_or(0f64),
                    sublanguageid: lang.unwrap().to_owned(),
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
