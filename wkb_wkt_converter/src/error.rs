use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidWkt(String),
    InvalidWkb(String),
    UnsupportedGeometryType(u32),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidWkt(msg) => write!(f, "invalid WKT: {msg}"),
            Error::InvalidWkb(msg) => write!(f, "invalid WKB: {msg}"),
            Error::UnsupportedGeometryType(code) => {
                write!(f, "unsupported geometry type code: {code}")
            }
        }
    }
}

impl std::error::Error for Error {}
