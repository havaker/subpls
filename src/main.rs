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
}

impl User {
    fn new() -> User {
        User {
            api: String::new(),
            token: String::new(),
        }
    }
}

const LOGIN_LOCATION: &str = "http://api.opensubtitles.org/xml-rpc";

fn login_request(username: &str, password: &str, language: &str) 
    -> Result<xmlrpc::Value, xmlrpc::Error> {
    let request = xmlrpc::Request::new("LogIn")
        .arg(username)
        .arg(password)
        .arg(language)
        .arg("TemporaryUserAgent");
    Ok(request.call_url(LOGIN_LOCATION)?)
}

// login to OS server, should be called always when starting
// talking with server. It returns token, which must be 
// used in later communication. 
fn login(username: &str, password: &str, language: &str) 
    -> Result<User, String> {
    match login_request(username, password, language) {
        Ok(response) => {
            let mut user = User::new();
            match response.get("token").and_then(|x| x.as_str()) {
                Some(token) => 
                    user.token = token.to_string(),
                None => 
                    return Err("failed to get token".to_owned())
            }
            user.api = response.get("data").and_then( |val|
                    val.as_struct()
                    .and_then(|data| 
                        data.get("Content-Location").and_then( |cl|
                            cl.as_str()
                        )
                    )
                ).unwrap_or(LOGIN_LOCATION)
                .to_string();
            Ok(user)
        },
        Err(e) => {
            Err(e.to_string())
        }
    }
}

fn main() {
    let l = login("", "", "en");
    println!("{:?}",l);
    /*
    let h = os_hash("/tmp/dummy.bin");
    println!("{:?}", h);
    let h = h.unwrap_or(0);
    println!("{:01$x}", h, 16);
    */
}
