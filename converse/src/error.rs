use serde_cbor::error as cbor;

use std::error;
use std::fmt;
use std::string::FromUtf8Error;
use std::io;


#[derive(Debug)]
pub enum Error {
    Server(String),
    Client(String),
    ProcessDirectory(String),
    IOError(io::Error),
    FromUtf8Error(FromUtf8Error),
    Serialize(cbor::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Server(s) => write!(f, "Server error: {}", s),
            Error::Client(s) => write!(f, "Client error: {}", s),
            Error::ProcessDirectory(s) => write!(f, "Process directory error: {}", s),
            Error::IOError(e) => write!(f, "{}", e),
            Error::FromUtf8Error(e) => write!(f, "{}", e),
            Error::Serialize(e) => write!(f, "{}", e),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::IOError(e) => Some(e),
            Error::FromUtf8Error(e) => Some(e),
            Error::Serialize(e) => Some(e),
            _ => None,
        }
    }
}

macro_rules! from_error {
    ($enum: ty, $type: ty, $path: path) => {
        impl From<$type> for $enum {
            fn from(e: $type) -> Self {
                $path(e)
            }
        }
    }
}

from_error!(Error, io::Error, Error::IOError);
from_error!(Error, FromUtf8Error, Error::FromUtf8Error);
from_error!(Error, cbor::Error, Error::Serialize);
