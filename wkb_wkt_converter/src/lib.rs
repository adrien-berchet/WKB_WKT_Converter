pub mod error;
pub mod types;

mod wkb_to_wkt;
mod wkt_to_wkb;

use types::EWKB_SRID;

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

/// Controls SRID handling in the output of [`text_to_wkb`], [`text_to_wkt`], and [`text_to_hex_wkb`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SridMode {
    /// Mirror the input: SRID is kept if present, absent if not.
    #[default]
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
///   When the input is hex WKB, the bytes are returned as-is without WKB structure validation.
/// - [`SridMode::Strip`]: always strip the SRID from the output.
/// - [`SridMode::Set(n)`]: always embed SRID `n`, overriding any SRID in the input.
pub fn text_to_wkb(text: &str, srid: SridMode) -> Result<Vec<u8>> {
    let trimmed = text.trim();
    match srid {
        SridMode::Auto => {
            if let Some(bytes) = try_decode_hex(trimmed) {
                Ok(bytes)
            } else {
                wkt_to_wkb(trimmed)
            }
        }
        SridMode::Strip => {
            if let Some(bytes) = try_decode_hex(trimmed) {
                if let Some(out) = try_strip_srid_from_le_wkb(&bytes) {
                    Ok(out)
                } else {
                    // Big-endian or malformed: fall back to full round-trip which
                    // normalises byte order and validates structure.
                    let (wkt, _) = wkb_to_wkt_split_srid(&bytes)?;
                    wkt_to_wkb(&wkt)
                }
            } else {
                wkt_to_wkb_split_srid(trimmed).map(|(b, _)| b)
            }
        }
        SridMode::Set(srid_val) => {
            if let Some(bytes) = try_decode_hex(trimmed) {
                if let Some(out) = try_set_srid_in_le_wkb(&bytes, srid_val) {
                    Ok(out)
                } else {
                    let (wkt, _) = wkb_to_wkt_split_srid(&bytes)?;
                    wkt_to_wkb(&format!("SRID={srid_val};{wkt}"))
                }
            } else {
                wkt_to_wkb(&format!("SRID={srid_val};{}", strip_ewkt_prefix(trimmed)))
            }
        }
    }
}

/// Converts any WKT/EWKT string or hex-encoded WKB/EWKB string to a WKT
/// string.
///
/// The input format is detected automatically: a string composed entirely of
/// hexadecimal characters is treated as hex WKB; anything else is treated as
/// WKT.
///
/// `srid` controls SRID handling in the output:
/// - [`SridMode::Auto`] (default): mirror the input — `SRID=N;` prefix kept if present.
/// - [`SridMode::Strip`]: always strip the `SRID=N;` prefix from the output.
/// - [`SridMode::Set(n)`]: always prepend `SRID=n;`, overriding any SRID in the input.
///
/// `normalize_wkt` controls whether WKT input is normalised (canonical casing,
/// spacing, and coordinate formatting) via a round-trip through WKB.  Pass
/// `false` to skip normalisation (only the SRID prefix is adjusted), which
/// avoids the encoding overhead.  **When `false`, WKT input is not validated —
/// malformed WKT is returned without error.**  Note that leading/trailing
/// whitespace is always trimmed regardless of this flag.  Has no effect when
/// the input is hex WKB, which is always decoded to normalised WKT.
pub fn text_to_wkt(text: &str, srid: SridMode, normalize_wkt: bool) -> Result<String> {
    let trimmed = text.trim();

    // Fast path for WKT input with normalisation disabled.
    // All valid WKT geometry type keywords (POINT, LINESTRING, …) and the
    // SRID= prefix start with characters that are not ASCII hex digits, so
    // checking the first byte is a reliable O(1) discriminator.
    if !normalize_wkt
        && trimmed
            .as_bytes()
            .first()
            .is_some_and(|b| !b.is_ascii_hexdigit())
    {
        return Ok(match srid {
            SridMode::Auto => trimmed.to_owned(),
            SridMode::Strip => strip_ewkt_prefix(trimmed).to_owned(),
            SridMode::Set(srid_val) => {
                format!("SRID={srid_val};{}", strip_ewkt_prefix(trimmed))
            }
        });
    }

    // Single-pass hex detection and decoding (hex WKB is always normalised).
    if let Some(decoded) = try_decode_hex(trimmed) {
        return match srid {
            SridMode::Auto => wkb_to_wkt(&decoded),
            SridMode::Strip => wkb_to_wkt_split_srid(&decoded).map(|(s, _)| s),
            SridMode::Set(srid_val) => {
                let (plain, _) = wkb_to_wkt_split_srid(&decoded)?;
                Ok(format!("SRID={srid_val};{plain}"))
            }
        };
    }

    // WKT input whose first byte happens to be a hex digit (e.g. starts with
    // a digit), but failed hex decoding.  With normalisation disabled, return
    // as-is (no validation).
    if !normalize_wkt {
        return Ok(match srid {
            SridMode::Auto => trimmed.to_owned(),
            SridMode::Strip => strip_ewkt_prefix(trimmed).to_owned(),
            SridMode::Set(srid_val) => {
                format!("SRID={srid_val};{}", strip_ewkt_prefix(trimmed))
            }
        });
    }

    // WKT input, normalisation enabled.
    match srid {
        SridMode::Auto => {
            let wkb = wkt_to_wkb(trimmed)?;
            wkb_to_wkt(&wkb)
        }
        SridMode::Strip => {
            let (wkb, _) = wkt_to_wkb_split_srid(trimmed)?;
            wkb_to_wkt(&wkb)
        }
        SridMode::Set(srid_val) => {
            let (wkb, _) = wkt_to_wkb_split_srid(trimmed)?;
            let plain = wkb_to_wkt(&wkb)?;
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

fn strip_ewkt_prefix(wkt: &str) -> &str {
    if wkt
        .get(..5)
        .is_some_and(|p| p.eq_ignore_ascii_case("SRID="))
    {
        if let Some(pos) = wkt.find(';') {
            return wkt[pos + 1..].trim_start();
        }
    }
    wkt
}

/// Encode bytes as an uppercase hexadecimal string using a lookup table.
///
/// This is substantially faster than the `write!(s, "{b:02X}")` approach
/// because it avoids invoking the format machinery for every byte.
fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = vec![0u8; bytes.len() * 2];
    for (i, &b) in bytes.iter().enumerate() {
        out[2 * i] = HEX[(b >> 4) as usize];
        out[2 * i + 1] = HEX[(b & 0xF) as usize];
    }
    // SAFETY: every byte written is an ASCII hex digit (0–9, A–F).
    unsafe { String::from_utf8_unchecked(out) }
}

/// 256-entry lookup table mapping ASCII bytes to their hex nibble value (0–15)
/// or to 0xFF for invalid characters.  Built at compile time; a single array
/// index replaces the 3-arm range match in the hot decode loop, eliminating
/// branch mispredictions for inputs whose hex characters are not clustered in
/// the '0'–'9' range (e.g. trig-value coordinates where A–F appear ~50 % of
/// the time).
const fn build_hex_nibble_lut() -> [u8; 256] {
    let mut lut = [0xFF_u8; 256];
    let mut i = 0u8;
    while i <= 9 {
        lut[(b'0' + i) as usize] = i;
        i += 1;
    }
    let mut i = 0u8;
    while i < 6 {
        lut[(b'a' + i) as usize] = 10 + i;
        lut[(b'A' + i) as usize] = 10 + i;
        i += 1;
    }
    lut
}
static HEX_NIBBLE_LUT: [u8; 256] = build_hex_nibble_lut();

/// Try to decode `s` as a hex string in a single pass, combining detection and
/// decoding.
///
/// Returns `None` if `s` is empty, has odd length, or contains any
/// non-hexadecimal character.  This replaces the previous two-pass pattern of
/// `is_hex_str(s)` followed by `decode_hex(s)`.
fn try_decode_hex(s: &str) -> Option<Vec<u8>> {
    let bytes = s.as_bytes();
    if bytes.is_empty() || !bytes.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let hi = HEX_NIBBLE_LUT[chunk[0] as usize];
        let lo = HEX_NIBBLE_LUT[chunk[1] as usize];
        if (hi | lo) > 0x0F {
            return None;
        }
        out.push((hi << 4) | lo);
    }
    Some(out)
}

/// Decode a hex string with detailed position-aware error messages.
/// Used by [`hex_wkb_to_wkt`] where the caller knows the input must be hex.
fn decode_hex(hex: &str) -> Result<Vec<u8>> {
    let bytes = hex.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err(Error::InvalidWkb("hex string has odd length".into()));
    }
    bytes
        .chunks(2)
        .enumerate()
        .map(|(idx, pair)| {
            let hi = HEX_NIBBLE_LUT[pair[0] as usize];
            if hi > 0x0F {
                return Err(Error::InvalidWkb(format!(
                    "invalid hex digit at position {}",
                    idx * 2
                )));
            }
            let lo = HEX_NIBBLE_LUT[pair[1] as usize];
            if lo > 0x0F {
                return Err(Error::InvalidWkb(format!(
                    "invalid hex digit at position {}",
                    idx * 2 + 1
                )));
            }
            Ok((hi << 4) | lo)
        })
        .collect()
}

/// Strip the SRID from a little-endian EWKB byte slice without parsing
/// coordinates.
///
/// Returns `None` when the bytes are not recognisable as little-endian EWKB
/// (too short, big-endian marker, or SRID flag set but fewer than 9 bytes
/// available).  The caller should then fall back to a full WKB→WKT→WKB
/// round-trip, which handles big-endian input and normalises to LE output.
fn try_strip_srid_from_le_wkb(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < 5 || bytes[0] != 1 {
        return None;
    }
    let type_u32 = u32::from_le_bytes(bytes[1..5].try_into().unwrap());
    if type_u32 & EWKB_SRID == 0 {
        // No SRID present — return a copy unchanged.
        return Some(bytes.to_vec());
    }
    if bytes.len() < 9 {
        return None;
    }
    let new_type = type_u32 & !EWKB_SRID;
    let mut out = Vec::with_capacity(bytes.len() - 4);
    out.push(1u8); // little-endian marker
    out.extend_from_slice(&new_type.to_le_bytes());
    out.extend_from_slice(&bytes[9..]); // skip the 4 SRID bytes
    Some(out)
}

/// Inject or replace the SRID in a little-endian EWKB byte slice without
/// parsing coordinates.
///
/// Returns `None` under the same conditions as [`try_strip_srid_from_le_wkb`].
fn try_set_srid_in_le_wkb(bytes: &[u8], srid: u32) -> Option<Vec<u8>> {
    if bytes.len() < 5 || bytes[0] != 1 {
        return None;
    }
    let type_u32 = u32::from_le_bytes(bytes[1..5].try_into().unwrap());
    if type_u32 & EWKB_SRID != 0 {
        // SRID already present — overwrite it in-place.
        if bytes.len() < 9 {
            return None;
        }
        let mut out = bytes.to_vec();
        out[5..9].copy_from_slice(&srid.to_le_bytes());
        Some(out)
    } else {
        // No SRID yet — insert 4 bytes after the type code and set the flag.
        let new_type = type_u32 | EWKB_SRID;
        let mut out = Vec::with_capacity(bytes.len() + 4);
        out.push(1u8);
        out.extend_from_slice(&new_type.to_le_bytes());
        out.extend_from_slice(&srid.to_le_bytes());
        out.extend_from_slice(&bytes[5..]);
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_nibble_lut_spot_check() {
        // Call build_hex_nibble_lut() at runtime so llvm-cov instruments it.
        let lut = build_hex_nibble_lut();
        assert_eq!(lut[b'0' as usize], 0);
        assert_eq!(lut[b'9' as usize], 9);
        assert_eq!(lut[b'a' as usize], 10);
        assert_eq!(lut[b'f' as usize], 15);
        assert_eq!(lut[b'A' as usize], 10);
        assert_eq!(lut[b'F' as usize], 15);
        assert_eq!(lut[b'G' as usize], 0xFF);
        assert_eq!(lut[b' ' as usize], 0xFF);
    }
}
