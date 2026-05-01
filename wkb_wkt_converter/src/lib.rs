pub mod error;
pub mod types;

mod wkb_to_wkt;
mod wkt_to_wkb;

pub use error::{Error, Result};
pub use types::{Dimension, GeomType};

/// Converts WKB/EWKB bytes to a WKT/EWKT string.
/// If the input is EWKB with an SRID, the output includes a `SRID=N;` prefix.
pub fn wkb_to_wkt(wkb: &[u8]) -> Result<String> {
    wkb_to_wkt::convert(wkb)
}

/// Converts WKB/EWKB bytes to a WKT string, returning the SRID separately.
/// The returned WKT string does not include a `SRID=N;` prefix.
pub fn wkb_to_wkt_split_srid(wkb: &[u8]) -> Result<(String, Option<u32>)> {
    wkb_to_wkt::convert_split_srid(wkb)
}

/// Converts a WKT/EWKT string to EWKB bytes.
/// If the input includes a `SRID=N;` prefix, the SRID is embedded in the output.
pub fn wkt_to_wkb(wkt: &str) -> Result<Vec<u8>> {
    wkt_to_wkb::convert(wkt)
}

/// Converts a WKT/EWKT string to EWKB bytes, returning the SRID separately.
/// The SRID is not embedded in the returned bytes.
pub fn wkt_to_wkb_split_srid(wkt: &str) -> Result<(Vec<u8>, Option<u32>)> {
    wkt_to_wkb::convert_split_srid(wkt)
}

/// Converts a WKT/EWKT string to an uppercase hex-encoded EWKB string.
pub fn wkt_to_hex_wkb(wkt: &str) -> Result<String> {
    let bytes = wkt_to_wkb(wkt)?;
    Ok(bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            use std::fmt::Write as _;
            write!(s, "{b:02X}").unwrap();
            s
        }))
}

/// Converts a hex-encoded WKB/EWKB string to a WKT/EWKT string.
pub fn hex_wkb_to_wkt(hex: &str) -> Result<String> {
    let bytes = decode_hex(hex)?;
    wkb_to_wkt(&bytes)
}

fn decode_hex(hex: &str) -> Result<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return Err(Error::InvalidWkb("hex string has odd length".into()));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|_| Error::InvalidWkb(format!("invalid hex byte at position {i}")))
        })
        .collect()
}
