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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_wkt_display() {
        assert_eq!(
            Error::InvalidWkt("bad input".into()).to_string(),
            "invalid WKT: bad input"
        );
    }

    #[test]
    fn invalid_wkb_display() {
        assert_eq!(
            Error::InvalidWkb("truncated".into()).to_string(),
            "invalid WKB: truncated"
        );
    }

    #[test]
    fn unsupported_type_display() {
        assert_eq!(
            Error::UnsupportedGeometryType(99).to_string(),
            "unsupported geometry type code: 99"
        );
    }
}
