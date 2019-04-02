use std::collections::HashMap;

pub mod movie;
pub use movie::Error;
pub use movie::Movie;
pub use movie::Subtitles;

#[derive(Debug)]
pub struct User {
    api: String,
    token: String,
    sublanguageid: String,
}

impl User {
    // login to OS server, should be called always when starting talking with
    // server. It returns token, which must be used in later communication.
    pub fn login(
        username: &str,
        password: &str,
        language: &str,
    ) -> Result<User, Error> {
        let response = User::login_request(username, password, language)?;
        let mut user = User {
            api: String::new(),
            token: String::new(),
            sublanguageid: language.to_owned(),
        };
        User::response_status(&response)?;
        match response.get("token").and_then(|x| x.as_str()) {
            Some(token) => user.token = token.to_string(),
            None => return Err(Error::NoToken),
        }
        user.api = response
            .get("data")
            .and_then(|val| {
                val.as_struct().and_then(|data| {
                    data.get("Content-Location").and_then(|cl| cl.as_str())
                })
            })
            .unwrap_or(User::LOGIN_LOCATION)
            .to_string();
        Ok(user)
    }

    pub fn search(&self, mut movies: Vec<Movie>) -> Result<Vec<Movie>, Error> {
        let response = self.search_request(&movies)?;
        User::response_status(&response)?;
        let mut subs_map = User::extract_subids(response);
        for mut movie in &mut movies {
            if let Some(os_info) = &movie.os_info {
                movie.subs = subs_map.remove(&os_info.hash).unwrap_or_default();
            }
        }
        Ok(movies)
    }

    pub fn download(
        &self,
        mut movies: Vec<Movie>,
    ) -> Result<Vec<Movie>, Error> {
        let mut ids = Vec::new();
        for movie in &movies {
            for sub in &movie.subs {
                ids.push(sub);
            }
        }
        let response = self.download_request(ids)?;
        User::response_status(&response)?;
        let mut b64gzs = HashMap::new();
        let results = response.get("data").and_then(|data| data.as_array());
        let mut extract_item = |item: &xmlrpc::Value| {
            if item.as_struct().is_none() {
                return;
            }
            let item = item.as_struct().unwrap();
            let mut fields = [("data", ""), ("idsubtitlefile", "")];
            for (ref name, ref mut val) in &mut fields {
                if let Some(v) = item.get(*name).and_then(|x| x.as_str()) {
                    *val = v;
                } else {
                    return;
                }
            }
            b64gzs.insert(fields[1].1.to_string(), fields[0].1.to_string());
        };
        results.map(|array| {
            for item in array {
                extract_item(item)
            }
        });
        for movie in &mut movies {
            for mut sub in &mut movie.subs {
                sub.b64gz = b64gzs.get(&sub.id).map(|b| b.clone());
            }
        }
        Ok(movies)
    }

    fn extract_subids(
        response: xmlrpc::Value,
    ) -> HashMap<String, Vec<Subtitles>> {
        let mut ret = HashMap::new();
        let results = response.get("data").and_then(|data| data.as_array());
        let mut extract_item = |item: &xmlrpc::Value| {
            if item.as_struct().is_none() {
                return;
            }
            let item = item.as_struct().unwrap();
            let mut fields = [
                ("MovieHash", ""),
                ("IDSubtitleFile", ""),
                ("SubFormat", ""),
                ("SubRating", ""),
                ("SubLanguageID", ""),
            ];
            for (ref name, ref mut val) in &mut fields {
                if let Some(v) = item.get(*name).and_then(|x| x.as_str()) {
                    *val = v;
                } else {
                    return;
                }
            }
            let hash = fields[0].1.to_owned();
            let subs = ret.entry(hash).or_insert(Vec::new());
            subs.push(Subtitles {
                format: String::from(fields[2].1),
                id: String::from(fields[1].1),
                rating: fields[3].1.parse().unwrap_or(0f64),
                lang: String::from(fields[4].1),
                b64gz: None,
            })
        };
        results.map(|array| {
            for item in array {
                extract_item(item)
            }
        });
        ret
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
        movies: &Vec<Movie>,
    ) -> Result<xmlrpc::Value, Error> {
        let mut prepared = Vec::new();
        for movie in movies {
            if let Some(os_info) = &movie.os_info {
                let entry = xmlrpc::Value::Struct(
                    vec![
                        (
                            "moviehash".to_string(),
                            xmlrpc::Value::from(os_info.hash.as_str()),
                        ),
                        (
                            "moviebytesize".to_string(),
                            xmlrpc::Value::from(os_info.size as i64),
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
        }
        if prepared.len() < 1 {
            return Err(Error::NothingToSearch);
        }
        let request = xmlrpc::Request::new("SearchSubtitles")
            .arg(self.token.as_str())
            .arg(xmlrpc::Value::Array(prepared));
        Ok(request.call_url(self.api.as_str())?)
    }

    fn download_request(
        &self,
        sub_ids: Vec<&Subtitles>,
    ) -> Result<xmlrpc::Value, xmlrpc::Error> {
        let request = xmlrpc::Request::new("DownloadSubtitles")
            .arg(self.token.as_str())
            .arg(xmlrpc::Value::Array(
                sub_ids
                    .into_iter()
                    .map(|x| xmlrpc::Value::from(x.id.as_str()))
                    .collect(),
            ));
        Ok(request.call_url(&self.api)?)
    }

    fn response_status(response: &xmlrpc::Value) -> Result<(), Error> {
        if let Some(status) = response.get("status").and_then(|x| x.as_str()) {
            if status == "200 OK" {
                return Ok(());
            } else {
                return Err(Error::BadStatus(status.to_string()));
            }
        }
        Err(Error::Malformed)
    }
}
