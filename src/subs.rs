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

    pub fn compute_os_hash(&mut self) -> Result<(), std::io::Error> {
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
