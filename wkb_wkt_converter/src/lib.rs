pub mod error;
pub mod types;

mod wkb_to_wkt;
mod wkt_to_wkb;

use std::borrow::Cow;

use types::{EWKB_M, EWKB_SRID, EWKB_Z};

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
/// The input format is detected automatically: a non-empty string with even
/// length composed entirely of hexadecimal characters is treated as hex WKB;
/// anything else (including odd-length all-hex strings) is treated as WKT.
///
/// `srid` controls SRID handling in the output:
/// - [`SridMode::Auto`] (default): mirror the input — SRID kept if present, absent if not.
///   When the input is hex WKB, the bytes are returned as-is without WKB structure validation.
/// - [`SridMode::Strip`]: always strip the SRID from the output.
/// - [`SridMode::Set(n)`]: always embed SRID `n`, overriding any SRID in the input.
///
/// **Note on validation:** for little-endian Point, LineString, and Polygon hex
/// WKB input under `Strip` and `Set`, the fast path rewrites only the EWKB header
/// after checking the byte length implied by the simple geometry body.  Big-endian,
/// collection, ISO-dimensional, or malformed simple input falls back to a full
/// parse round-trip which validates and normalises the geometry body.
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
                match try_strip_srid_from_le_wkb(&bytes) {
                    Some(Cow::Borrowed(_)) => Ok(bytes), // no SRID — decoded bytes are already correct
                    Some(Cow::Owned(out)) => Ok(out),
                    None => {
                        let (wkt, _) = wkb_to_wkt_split_srid(&bytes)?;
                        wkt_to_wkb(&wkt)
                    }
                }
            } else {
                wkt_to_wkb_split_srid(trimmed).map(|(b, _)| b)
            }
        }
        SridMode::Set(srid_val) => {
            if let Some(bytes) = try_decode_hex(trimmed) {
                match try_set_srid_in_le_wkb(&bytes, srid_val) {
                    Some(Cow::Borrowed(_)) => Ok(bytes), // SRID already correct
                    Some(Cow::Owned(out)) => Ok(out),
                    None => {
                        let (wkt, _) = wkb_to_wkt_split_srid(&bytes)?;
                        wkt_to_wkb(&format!("SRID={srid_val};{wkt}"))
                    }
                }
            } else {
                // wkt_to_wkb_split_srid validates the full WKT including any SRID= prefix.
                let (wkb, _) = wkt_to_wkb_split_srid(trimmed)?;
                // wkb is canonical LE EWKB without embedded SRID; inject srid_val.
                // wkt_to_wkb_split_srid always returns valid LE WKB with len >= 5 on success.
                let type_u32 = u32::from_le_bytes(wkb[1..5].try_into().unwrap());
                let mut out = Vec::with_capacity(wkb.len() + 4);
                out.push(1u8);
                out.extend_from_slice(&(type_u32 | EWKB_SRID).to_le_bytes());
                out.extend_from_slice(&srid_val.to_le_bytes());
                out.extend_from_slice(&wkb[5..]);
                Ok(out)
            }
        }
    }
}

/// Converts any WKT/EWKT string or hex-encoded WKB/EWKB string to a WKT
/// string.
///
/// The input format is detected automatically: a non-empty string with even
/// length composed entirely of hexadecimal characters is treated as hex WKB;
/// anything else (including odd-length all-hex strings) is treated as WKT.
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
/// whitespace is always trimmed regardless of this flag.  An empty input string
/// always returns an error regardless of this flag.  Has no effect when the
/// input is hex WKB, which is always decoded to normalised WKT.
pub fn text_to_wkt(text: &str, srid: SridMode, normalize_wkt: bool) -> Result<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidWkt("empty input".into()));
    }

    // Fast path for WKT input with normalisation disabled.
    // All geometry type keywords supported by this library (POINT, LINESTRING,
    // POLYGON, MULTI*, GEOMETRYCOLLECTION) and the SRID= prefix start with a
    // letter that is not an ASCII hex digit, so the first byte is a reliable
    // O(1) discriminator.  (Note: extended keywords like CIRCULARSTRING start
    // with 'C' which IS a hex digit; those fall through to try_decode_hex, which
    // returns None for non-hex body characters and routes them to WKT parsing.)
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

    // Hex WKB is always decoded to normalised WKT.
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
///
/// **Note:** for lowercase hex input under [`SridMode::Auto`], the output is
/// the uppercase re-encoding of the decoded bytes (not identical to the input
/// hex string).
pub fn text_to_hex_wkb(text: &str, srid: SridMode) -> Result<String> {
    text_to_wkb(text, srid).map(|b| encode_hex(&b))
}

fn strip_ewkt_prefix(wkt: &str) -> &str {
    if wkt
        .get(..5)
        .is_some_and(|p| p.eq_ignore_ascii_case("SRID="))
    {
        if let Some(rel) = wkt[5..].find(';') {
            return wkt[6 + rel..].trim_start();
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
    let mut out = Vec::with_capacity(bytes.len().saturating_mul(2));
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize]);
        out.push(HEX[(b & 0xF) as usize]);
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

/// Try to decode `s` as a hex string, combining detection and decoding.
///
/// Returns `None` if `s` is empty, has odd length, or contains any
/// non-hexadecimal character.  Allocation is delayed until the first byte pair
/// has been proven to be valid hex, so obvious non-hex input avoids allocation
/// while valid hex still decodes in one pass over the remaining pairs.
fn try_decode_hex(s: &str) -> Option<Vec<u8>> {
    let bytes = s.as_bytes();
    if bytes.is_empty() || !bytes.len().is_multiple_of(2) {
        return None;
    }

    let hi = HEX_NIBBLE_LUT[bytes[0] as usize];
    let lo = HEX_NIBBLE_LUT[bytes[1] as usize];
    if (hi | lo) > 0x0F {
        return None;
    }

    let mut out = Vec::with_capacity(bytes.len() / 2);
    out.push((hi << 4) | lo);
    for chunk in bytes[2..].chunks_exact(2) {
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
    if bytes.is_empty() {
        return Err(Error::InvalidWkb("empty hex string".into()));
    }
    if !bytes.len().is_multiple_of(2) {
        return Err(Error::InvalidWkb("hex string has odd length".into()));
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for (i, chunk) in bytes.chunks_exact(2).enumerate() {
        let hi = HEX_NIBBLE_LUT[chunk[0] as usize];
        if hi > 0x0F {
            return Err(Error::InvalidWkb(format!(
                "invalid hex digit at position {}",
                i * 2
            )));
        }
        let lo = HEX_NIBBLE_LUT[chunk[1] as usize];
        if lo > 0x0F {
            return Err(Error::InvalidWkb(format!(
                "invalid hex digit at position {}",
                i * 2 + 1
            )));
        }
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

struct LeFastPathHeader {
    type_u32: u32,
    canonical_type_without_srid: u32,
}

fn read_le_u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    Some(u32::from_le_bytes(
        bytes.get(offset..end)?.try_into().unwrap(),
    ))
}

fn exact_simple_le_wkb_len(
    bytes: &[u8],
    body_start: usize,
    base_geom_type: u32,
    known_dim_flags: u32,
) -> Option<usize> {
    debug_assert!((1..=3).contains(&base_geom_type));
    let coord_count =
        2 + usize::from(known_dim_flags & EWKB_Z != 0) + usize::from(known_dim_flags & EWKB_M != 0);
    let coord_bytes = coord_count.checked_mul(8)?;

    if base_geom_type == 1 {
        return body_start.checked_add(coord_bytes);
    }

    let mut pos = body_start.checked_add(4)?;
    let count = usize::try_from(read_le_u32_at(bytes, body_start)?).ok()?;
    if base_geom_type == 2 {
        return pos.checked_add(count.checked_mul(coord_bytes)?);
    }

    for _ in 0..count {
        let point_count = usize::try_from(read_le_u32_at(bytes, pos)?).ok()?;
        pos = pos.checked_add(4)?;
        pos = pos.checked_add(point_count.checked_mul(coord_bytes)?)?;
    }
    Some(pos)
}

fn try_read_le_fast_path_header(bytes: &[u8]) -> Option<LeFastPathHeader> {
    if bytes.len() < 5 || bytes[0] != 1 {
        return None;
    }
    let type_u32 = u32::from_le_bytes(bytes[1..5].try_into().unwrap());
    let known_dim_flags = type_u32 & (EWKB_Z | EWKB_M);
    // Strip all EWKB flag bits and unknown high bits to get the bare OGC type.
    let base_geom_type = type_u32 & !(EWKB_Z | EWKB_M | EWKB_SRID) & 0x0000_FFFF;
    if !(1..=3).contains(&base_geom_type) {
        return None;
    }

    let body_start = if type_u32 & EWKB_SRID != 0 {
        if bytes.len() < 9 {
            return None;
        }
        9
    } else {
        5
    };
    if exact_simple_le_wkb_len(bytes, body_start, base_geom_type, known_dim_flags)? != bytes.len() {
        return None;
    }

    Some(LeFastPathHeader {
        type_u32,
        canonical_type_without_srid: base_geom_type | known_dim_flags,
    })
}

/// Strip the SRID from a little-endian EWKB byte slice without parsing
/// coordinates.
///
/// Returns `None` when the bytes are not recognisable as little-endian EWKB
/// (too short, big-endian marker, or mismatched simple body length), or when the
/// base geometry type is > 3 (collection types 4–7 or ISO-dimensional codes >
/// 7).  In those cases the caller should fall back to a full WKB→WKT→WKB
/// round-trip, which normalises type codes, handles big-endian input, and strips
/// nested SRID headers in sub-geometries.
fn try_strip_srid_from_le_wkb(bytes: &[u8]) -> Option<Cow<'_, [u8]>> {
    let header = try_read_le_fast_path_header(bytes)?;
    if header.type_u32 & EWKB_SRID == 0 {
        // No SRID present. Return a borrow only if the type word is already canonical.
        if header.type_u32 == header.canonical_type_without_srid {
            return Some(Cow::Borrowed(bytes));
        }
        let mut out = bytes.to_vec();
        out[1..5].copy_from_slice(&header.canonical_type_without_srid.to_le_bytes());
        return Some(Cow::Owned(out));
    }
    let mut out = Vec::with_capacity(bytes.len() - 4);
    out.push(1u8); // little-endian marker
    out.extend_from_slice(&header.canonical_type_without_srid.to_le_bytes());
    out.extend_from_slice(&bytes[9..]); // skip the 4 SRID bytes
    Some(Cow::Owned(out))
}

/// Inject or replace the SRID in a little-endian EWKB byte slice without
/// parsing coordinates.
///
/// Returns `None` when the bytes are not recognisable as little-endian EWKB
/// (too short, big-endian marker, or mismatched simple body length), or when the
/// base geometry type is > 3 (collection types 4–7 or ISO-dimensional codes >
/// 7).  In those cases the caller should fall back to a full WKB→WKT→WKB
/// round-trip, which normalises type codes, handles big-endian input, and strips
/// nested SRID headers in sub-geometries.
fn try_set_srid_in_le_wkb(bytes: &[u8], srid: u32) -> Option<Cow<'_, [u8]>> {
    let header = try_read_le_fast_path_header(bytes)?;
    let canonical_type_with_srid = header.canonical_type_without_srid | EWKB_SRID;
    if header.type_u32 & EWKB_SRID != 0 {
        // SRID already present — check if it already equals the requested SRID.
        let stored_srid = u32::from_le_bytes(bytes[5..9].try_into().unwrap());
        if stored_srid == srid && header.type_u32 == canonical_type_with_srid {
            return Some(Cow::Borrowed(bytes));
        }
        let mut out = bytes.to_vec();
        out[1..5].copy_from_slice(&canonical_type_with_srid.to_le_bytes());
        out[5..9].copy_from_slice(&srid.to_le_bytes());
        Some(Cow::Owned(out))
    } else {
        // No SRID yet — insert 4 bytes after the type code and set the flag.
        let mut out = Vec::with_capacity(bytes.len() + 4);
        out.push(1u8);
        out.extend_from_slice(&canonical_type_with_srid.to_le_bytes());
        out.extend_from_slice(&srid.to_le_bytes());
        out.extend_from_slice(&bytes[5..]);
        Some(Cow::Owned(out))
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
