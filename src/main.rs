use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::mem;

fn os_hash(filename: &str) -> Result<u64, std::io::Error> {
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
    Ok(hash)
}

#[derive(Debug)]
struct User {
    api: String,
    token: String,
    sublanguageid: String,
}

#[derive(Debug)]
struct Subtitles {
    moviehash: String,
    subid: String,
    rating: f64, // <1,10> + 0
    sublanguageid: String,
}

impl User {
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

    // login to OS server, should be called always when starting talking with
    // server. It returns token, which must be used in later communication.
    fn login(
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

fn main() {
    let user = User::login("", "", "en");
    println!("{:?}", user);
    let query = vec![("18379ac9af039390".to_owned(), 366876694u64)];
    let user = user.unwrap();
    let response = user.search_request(&query);
    println!("{:?}", response);
    let subs = User::extract_subids(response.unwrap());
    println!("subs:\n{:?}", subs);
    let best = user.choose_best(subs);
    println!("best:\n{:?}", best);
    /*
    let h = os_hash("/tmp/dummy.bin");
    println!("{:?}", h);
    let h = h.unwrap_or(0);
    println!("{:01$x}", h, 16);
    */
}
