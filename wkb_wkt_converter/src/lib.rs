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
    wkt_to_wkb(wkt).map(|b| encode_hex(&b))
}

/// Converts a hex-encoded WKB/EWKB string to a WKT/EWKT string.
pub fn hex_wkb_to_wkt(hex: &str) -> Result<String> {
    let bytes = decode_hex(hex)?;
    wkb_to_wkt(&bytes)
}

/// Controls SRID handling in the output of [`text_to_wkb`] and [`text_to_wkt`].
pub enum SridMode {
    /// Mirror the input: SRID is kept if present, absent if not.
    Auto,
    /// Always strip any SRID from the output.
    Strip,
    /// Always embed this SRID in the output, overriding whatever the input contains.
    Set(u32),
}

/// Converts any WKT/EWKT string or hex-encoded WKB/EWKB string to WKB bytes.
///
/// The input format is detected automatically: a string composed entirely of
/// hexadecimal characters is treated as hex WKB; anything else is treated as
/// WKT.
///
/// `srid` controls SRID handling in the output:
/// - [`SridMode::Auto`] (default): mirror the input — SRID kept if present, absent if not.
/// - [`SridMode::Strip`]: always strip the SRID from the output.
/// - [`SridMode::Set(n)`]: always embed SRID `n`, overriding any SRID in the input.
pub fn text_to_wkb(text: &str, srid: SridMode) -> Result<Vec<u8>> {
    let trimmed = text.trim();
    match srid {
        SridMode::Auto => {
            if is_hex_str(trimmed) {
                decode_hex(trimmed)
            } else {
                wkt_to_wkb(trimmed)
            }
        }
        SridMode::Strip => {
            if is_hex_str(trimmed) {
                let bytes = decode_hex(trimmed)?;
                let (wkt, _) = wkb_to_wkt_split_srid(&bytes)?;
                wkt_to_wkb(&wkt)
            } else {
                wkt_to_wkb_split_srid(trimmed).map(|(b, _)| b)
            }
        }
        SridMode::Set(srid_val) => {
            let plain = plain_wkt_from(trimmed)?;
            wkt_to_wkb(&format!("SRID={srid_val};{plain}"))
        }
    }
}

/// Converts any WKT/EWKT string or hex-encoded WKB/EWKB string to a WKT
/// string.
///
/// The input format is detected automatically: a string composed entirely of
/// hexadecimal characters is treated as hex WKB; anything else is treated as
/// WKT (and is normalised by a round-trip through WKB).
///
/// `srid` controls SRID handling in the output:
/// - [`SridMode::Auto`] (default): mirror the input — `SRID=N;` prefix kept if present.
/// - [`SridMode::Strip`]: always strip the `SRID=N;` prefix from the output.
/// - [`SridMode::Set(n)`]: always prepend `SRID=n;`, overriding any SRID in the input.
pub fn text_to_wkt(text: &str, srid: SridMode) -> Result<String> {
    let trimmed = text.trim();
    match srid {
        SridMode::Auto => {
            if is_hex_str(trimmed) {
                hex_wkb_to_wkt(trimmed)
            } else {
                let wkb = wkt_to_wkb(trimmed)?;
                wkb_to_wkt(&wkb)
            }
        }
        SridMode::Strip => {
            if is_hex_str(trimmed) {
                let bytes = decode_hex(trimmed)?;
                wkb_to_wkt_split_srid(&bytes).map(|(s, _)| s)
            } else {
                let (wkb, _) = wkt_to_wkb_split_srid(trimmed)?;
                wkb_to_wkt(&wkb)
            }
        }
        SridMode::Set(srid_val) => {
            let plain = plain_wkt_from(trimmed)?;
            Ok(format!("SRID={srid_val};{plain}"))
        }
    }
}

/// Converts any WKT/EWKT string or hex-encoded WKB/EWKB string to an uppercase
/// hex-encoded EWKB string.
///
/// The input format is detected automatically.
///
/// `srid` controls SRID handling in the output — see [`SridMode`].
pub fn text_to_hex_wkb(text: &str, srid: SridMode) -> Result<String> {
    text_to_wkb(text, srid).map(|b| encode_hex(&b))
}

/// Returns a normalised, SRID-free WKT string from either hex WKB or WKT input.
fn plain_wkt_from(trimmed: &str) -> Result<String> {
    if is_hex_str(trimmed) {
        let bytes = decode_hex(trimmed)?;
        wkb_to_wkt_split_srid(&bytes).map(|(s, _)| s)
    } else {
        let (wkb, _) = wkt_to_wkb_split_srid(trimmed)?;
        wkb_to_wkt(&wkb)
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        use std::fmt::Write as _;
        write!(s, "{b:02X}").unwrap();
        s
    })
}

fn is_hex_str(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit())
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
