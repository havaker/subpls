use flate2::read::GzDecoder;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;

pub mod error;
pub mod hash;

pub use error::*;
use hash::*;

#[derive(Debug, Clone)]
pub struct Subtitles {
    pub id: String,
    pub lang: String,
    pub format: String,
    pub rating: f64, // <1,10> + 0
    pub b64gz: Option<String>,
}

#[derive(Debug)]
pub struct Movie {
    pub path: PathBuf,
    pub subs: Vec<Subtitles>,
    pub os_info: Option<Hash>,
}

impl Movie {
    pub fn new(path: PathBuf) -> Movie {
        Movie {
            path: path,
            os_info: None,
            subs: Vec::new(),
        }
    }

    pub fn collection(files: &Vec<&str>) -> Vec<Movie> {
        let mut ret = Vec::new();
        for f in files {
            ret.push(Movie::new(PathBuf::from(f)));
        }
        ret
    }

    pub fn compute_os_hash(&mut self) -> Result<(), Error> {
        self.os_info = Some(os_hash(self.path.as_path())?);
        Ok(())
    }

    pub fn filter_subs(&mut self) {
        let mut highest = -1f64;
        let mut id = String::new();
        for sub in &self.subs {
            if sub.rating > highest {
                highest = sub.rating;
                id = sub.id.clone();
            }
        }
        self.subs.retain(|sub| sub.id == id);
    }

    pub fn save_subs(&self) -> Result<(), Error> {
        for sub in &self.subs {
            if sub.b64gz.is_none() {
                continue;
            }
            let decoded = base64::decode(&sub.b64gz.as_ref().unwrap())?;
            let extension = format!("{}.{}", sub.lang, sub.format);
            let mut sub_path = self.path.clone();
            if sub_path.set_extension(extension) == false {
                return Err(Error::BadPath);
            }
            let mut file = File::create(sub_path.as_path())?;
            let extracted = Movie::decode_reader(decoded)?;
            file.write_all(extracted.as_slice())?;
            return Ok(());
        }
        Err(Error::NothingToSave)
    }

    pub fn present_rating(&self) -> Option<f64> {
        if self.subs.len() > 0 {
            Some(self.subs[0].rating)
        } else {
            None
        }
    }

    pub fn path_str(&self) -> &str {
        match self.path.to_str() {
            Some(x) => x,
            None => "",
        }
    }

    fn decode_reader(bytes: Vec<u8>) -> io::Result<Vec<u8>> {
        let mut gz = GzDecoder::new(&bytes[..]);
        let mut s = Vec::new();
        gz.read_to_end(&mut s)?;
        Ok(s)
    }
}
