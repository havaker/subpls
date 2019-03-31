use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::mem;

#[derive(Debug)]
pub struct User {
    api: String,
    token: String,
    sublanguageid: String,
}

pub enum Error {
    BadStatus(String),
    Io(std::io::Error),
    Xmlrpc(xmlrpc::Error),
    NoToken,
    Malformed,
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<xmlrpc::Error> for Error {
    fn from(error: xmlrpc::Error) -> Self {
        Error::Xmlrpc(error)
    }
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
                ("ISO639", ""),
            ];
            for (ref name, ref mut val) in &mut fields {
                if let Some(v) = item.get(*name).and_then(|x| x.as_str()) {
                    *val = v;
                } else {
                    return;
                }
            }
            let hash = fields[0].0.to_owned();
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

        let request = xmlrpc::Request::new("SearchSubtitles")
            .arg(self.token.as_str())
            .arg(xmlrpc::Value::Array(prepared));

        Ok(request.call_url(self.api.as_str())?)
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

#[derive(Debug)]
pub struct Hash {
    hash: String,
    size: u64,
}

pub fn os_hash(filename: &str) -> Result<Hash, std::io::Error> {
    const BLOCK: i64 = 65536;
    const ITERATIONS: i64 = BLOCK / 8;

    let file = File::open(filename)?;
    let filesize = file.metadata()?.len();

    let mut hash: u64 = filesize;
    let mut word: u64;

    let mut reader = BufReader::with_capacity(BLOCK as usize, file);
    let mut buffer = [0u8; 8];

    for _ in 0..ITERATIONS {
        reader.read_exact(&mut buffer)?;
        unsafe {
            word = mem::transmute(buffer);
        }
        hash = hash.wrapping_add(word);
    }

    reader.seek(SeekFrom::End(-BLOCK))?;

    for _ in 0..ITERATIONS {
        reader.read_exact(&mut buffer)?;
        unsafe {
            word = mem::transmute(buffer);
        }
        hash = hash.wrapping_add(word);
    }

    Ok(Hash {
        hash: format!("{:01$x}", hash, 16),
        size: filesize,
    })
}

#[derive(Debug)]
pub struct Movie {
    filename: String,
    os_info: Option<Hash>,
    subs: Vec<Subtitles>,
}

impl Movie {
    pub fn new(filename: String) -> Movie {
        Movie {
            filename: filename,
            os_info: None,
            subs: Vec::new(),
        }
    }

    pub fn compute_os_hash(&mut self) -> Result<(), Error> {
        self.os_info = Some(os_hash(&self.filename)?);
        Ok(())
    }
}

#[derive(Debug)]
pub struct Subtitles {
    id: String,
    lang: String,
    format: String,
    rating: f64, // <1,10> + 0
    b64gz: Option<String>,
}
