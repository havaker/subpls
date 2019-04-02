use flate2::read::GzDecoder;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

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
    pub filename: String,
    pub subs: Vec<Subtitles>,
    pub os_info: Option<Hash>,
}

impl Movie {
    pub fn new(filename: String) -> Movie {
        Movie {
            filename: filename,
            os_info: None,
            subs: Vec::new(),
        }
    }

    pub fn collection(filenames: &Vec<&str>) -> Vec<Movie> {
        let mut ret = Vec::new();
        for f in filenames {
            ret.push(Movie::new(f.to_string()));
        }
        ret
    }

    pub fn compute_os_hash(&mut self) -> Result<(), Error> {
        self.os_info = Some(os_hash(&self.filename)?);
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
            let stem = Path::new(&self.filename)
                .file_stem()
                .map(|x| x.to_str().unwrap_or(&self.filename))
                .unwrap_or(&self.filename);
            let sub_filename = format!("{}.{}.{}", stem, sub.lang, sub.format);
            let mut file = File::create(sub_filename)?;
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

    fn decode_reader(bytes: Vec<u8>) -> io::Result<Vec<u8>> {
        let mut gz = GzDecoder::new(&bytes[..]);
        let mut s = Vec::new();
        gz.read_to_end(&mut s)?;
        Ok(s)
    }
}
