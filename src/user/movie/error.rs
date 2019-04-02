#[derive(Debug)]
pub enum Error {
    BadStatus(String),
    Io(std::io::Error),
    Xmlrpc(xmlrpc::Error),
    NoToken,
    Base64,
    Malformed,
    NothingToSearch,
    NothingToSave,
    BadPath,
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

impl From<base64::DecodeError> for Error {
    fn from(_error: base64::DecodeError) -> Self {
        Error::Base64
    }
}
