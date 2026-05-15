pub mod error;
pub mod types;

mod wkb_to_wkt;
mod wkt_to_wkb;

use std::borrow::Cow;

use types::{EWKB_M, EWKB_SRID, EWKB_Z};

pub use error::{Error, Result};
pub use types::{Dimension, GeomType};

/// Converts WKB/EWKB bytes to a WKT/EWKT string.
///
/// `srid` controls SRID handling in the output:
/// - [`SridMode::Auto`]: mirror the input — `SRID=N;` prefix kept if present.
/// - [`SridMode::Strip`]: always strip the `SRID=N;` prefix from the output.
/// - [`SridMode::Set(n)`]: always prepend `SRID=n;`, overriding any SRID in the input.
pub fn wkb_to_wkt(wkb: &[u8], srid: SridMode) -> Result<String> {
    let srid = normalise_srid_mode(srid);
    match srid {
        SridMode::Auto => wkb_to_wkt::convert(wkb),
        SridMode::Strip => wkb_to_wkt_split_srid(wkb).map(|(wkt, _)| wkt),
        SridMode::Set(srid_val) => {
            let (wkt, _) = wkb_to_wkt_split_srid(wkb)?;
            Ok(format!("SRID={srid_val};{wkt}"))
        }
    }
}

/// Converts WKB/EWKB bytes to a WKT string, returning the SRID separately.
/// The returned WKT string does not include a `SRID=N;` prefix.
pub fn wkb_to_wkt_split_srid(wkb: &[u8]) -> Result<(String, Option<i32>)> {
    wkb_to_wkt::convert_split_srid(wkb)
}

/// Converts a WKT/EWKT string to EWKB bytes.
///
/// `srid` controls SRID handling in the output:
/// - [`SridMode::Auto`]: mirror the input — SRID kept if present, absent if not.
/// - [`SridMode::Strip`]: always strip the SRID from the output.
/// - [`SridMode::Set(n)`]: always embed SRID `n`, overriding any SRID in the input.
pub fn wkt_to_wkb(wkt: &str, srid: SridMode) -> Result<Vec<u8>> {
    let srid = normalise_srid_mode(srid);
    match srid {
        SridMode::Auto => wkt_to_wkb::convert(wkt),
        SridMode::Strip => wkt_to_wkb_split_srid(wkt).map(|(wkb, _)| wkb),
        SridMode::Set(srid_val) => wkt_to_wkb::convert_with_forced_srid(wkt, srid_val),
    }
}

/// Converts a WKT/EWKT string to EWKB bytes, returning the SRID separately.
/// The SRID is not embedded in the returned bytes.
pub fn wkt_to_wkb_split_srid(wkt: &str) -> Result<(Vec<u8>, Option<i32>)> {
    wkt_to_wkb::convert_split_srid(wkt)
}

/// Converts a WKT/EWKT string to an uppercase hex-encoded EWKB string.
///
/// `srid` controls SRID handling in the output:
/// - [`SridMode::Auto`]: mirror the input — SRID kept if present, absent if not.
/// - [`SridMode::Strip`]: always strip the SRID from the output.
/// - [`SridMode::Set(n)`]: always embed SRID `n`, overriding any SRID in the input.
pub fn wkt_to_hex_wkb(wkt: &str, srid: SridMode) -> Result<String> {
    let srid = normalise_srid_mode(srid);
    match srid {
        SridMode::Auto => wkt_to_wkb::convert_to_hex(wkt),
        SridMode::Strip => wkt_to_wkb::convert_to_hex_split_srid(wkt),
        SridMode::Set(srid_val) => wkt_to_wkb::convert_to_hex_with_forced_srid(wkt, srid_val),
    }
}

/// Converts a hex-encoded WKB/EWKB string to a WKT/EWKT string.
///
/// `srid` controls SRID handling in the output:
/// - [`SridMode::Auto`]: mirror the input — `SRID=N;` prefix kept if present.
/// - [`SridMode::Strip`]: always strip the `SRID=N;` prefix from the output.
/// - [`SridMode::Set(n)`]: always prepend `SRID=n;`, overriding any SRID in the input.
pub fn hex_wkb_to_wkt(hex: &str, srid: SridMode) -> Result<String> {
    let bytes = decode_hex(hex)?;
    let srid = normalise_srid_mode(srid);
    match srid {
        SridMode::Auto => wkb_to_wkt::convert(&bytes),
        SridMode::Strip => wkb_to_wkt_split_srid(&bytes).map(|(wkt, _)| wkt),
        SridMode::Set(srid_val) => {
            let (wkt, _) = wkb_to_wkt_split_srid(&bytes)?;
            Ok(format!("SRID={srid_val};{wkt}"))
        }
    }
}

/// Controls SRID handling in the output of direct and generic converters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SridMode {
    /// Mirror the input: SRID is kept if present, absent if not.
    #[default]
    Auto,
    /// Always strip any SRID from the output.
    Strip,
    /// Always embed this SRID in the output, overriding whatever the input contains.
    /// Values ≤ 0 are treated as unknown per the PostGIS convention
    /// (`SRID_IS_UNKNOWN(x) ((int)x<=0)`) and behave identically to [`SridMode::Strip`].
    Set(i32),
}

/// Reads the SRID embedded in the top-level EWKB header without converting
/// the full geometry.
///
/// For any byte order and any geometry type whose high type-word bits contain
/// only the three canonical EWKB flags (Z, M, SRID), the SRID is read
/// directly from the 9-byte header without parsing the geometry body.  This
/// covers EWKB types 1–7, plain WKB, and ISO-dimensional WKB (type codes
/// such as 1001 have no SRID flag set, so `None` is returned immediately
/// without a full parse).  Only inputs with unknown flag bits or an invalid
/// byte-order marker fall back to a full [`wkb_to_wkt_split_srid`] parse.
///
/// Returns `None` when no SRID is embedded in the top-level header.
pub fn wkb_header_srid(wkb: &[u8]) -> Result<Option<i32>> {
    match try_read_ewkb_srid_fast(wkb) {
        Some(r) => r,
        None => wkb_to_wkt_split_srid(wkb).map(|(_, srid)| srid),
    }
}

/// Strips the top-level SRID flag and SRID field from a WKB/EWKB byte slice.
///
/// For canonical little-endian EWKB with any geometry type 1–7 (Point through
/// GeometryCollection), the header is rewritten without parsing the geometry
/// body.  Big-endian and ISO-dimensional inputs fall back to a full
/// WKB→WKT→WKB round-trip which normalises the representation.
///
/// Returns the input unchanged when no SRID is present.
pub fn wkb_strip_srid(wkb: &[u8]) -> Result<Vec<u8>> {
    apply_srid_to_wkb(wkb, SridMode::Strip).map(|c| c.into_owned())
}

/// Embeds or replaces the SRID in a WKB/EWKB byte slice.
///
/// SRID values ≤ 0 are treated as unknown per the PostGIS convention
/// (`SRID_IS_UNKNOWN(x) ((int)x<=0)`) and strip the SRID instead, identical
/// to [`wkb_strip_srid`].
///
/// For canonical little-endian EWKB with any geometry type 1–7 (Point through
/// GeometryCollection), the header is rewritten without parsing the geometry
/// body.  Big-endian and ISO-dimensional inputs fall back to a full
/// WKB→WKT→WKB round-trip which normalises the representation.
pub fn wkb_set_srid(wkb: &[u8], srid: i32) -> Result<Vec<u8>> {
    apply_srid_to_wkb(wkb, normalise_srid_mode(SridMode::Set(srid))).map(|c| c.into_owned())
}

/// Normalises `SridMode::Set(n)` with n ≤ 0 to `SridMode::Strip`.
///
/// Per the PostGIS convention (`SRID_IS_UNKNOWN(x) ((int)x<=0)`), SRID values ≤ 0
/// mean "unknown / no spatial reference" and must not appear in EWKT output.
fn normalise_srid_mode(mode: SridMode) -> SridMode {
    match mode {
        SridMode::Set(n) if n <= 0 => SridMode::Strip,
        other => other,
    }
}

/// Explicit input selector for generic converters.
///
/// [`Input::Text`] preserves the existing generic converter behavior: WKT/EWKT
/// text is parsed as text, while non-empty even-length all-hex text is treated
/// as hex-encoded WKB/EWKB.
///
/// [`Input::Wkb`] is always treated as raw WKB/EWKB bytes. The bytes are never
/// interpreted as hex text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input<'a> {
    /// WKT/EWKT text, or hex-encoded WKB/EWKB text.
    Text(&'a str),
    /// Raw WKB/EWKB bytes.
    Wkb(&'a [u8]),
}

/// Converts WKT/EWKT text, hex-encoded WKB/EWKB text, or raw WKB/EWKB bytes to WKB bytes.
///
/// The input format is detected automatically: a non-empty string with even
/// length composed entirely of hexadecimal characters is treated as hex WKB;
/// anything else (including odd-length all-hex strings) is treated as WKT.
/// Raw [`Input::Wkb`] bytes are always treated as WKB/EWKB, never as hex text.
///
/// `srid` controls SRID handling in the output:
/// - [`SridMode::Auto`] (default): mirror the input — SRID kept if present, absent if not.
///   When the input is hex WKB text or raw WKB bytes, the bytes are returned
///   as-is without WKB structure validation.
/// - [`SridMode::Strip`]: always strip the SRID from the output.
/// - [`SridMode::Set(n)`]: always embed SRID `n`, overriding any SRID in the input.
///
/// **Note on validation:** for canonical little-endian EWKB hex text or raw
/// WKB input of any type 1–7 under `Strip` and `Set`, the fast path rewrites
/// only the top-level EWKB header. Big-endian and ISO-dimensional inputs fall
/// back to a full parse round-trip which normalises the geometry body.
pub fn to_wkb(input: Input<'_>, srid: SridMode) -> Result<Vec<u8>> {
    match input {
        Input::Text(text) => to_wkb_text(text, srid),
        Input::Wkb(wkb) => to_wkb_bytes(wkb, srid),
    }
}

fn to_wkb_text(text: &str, srid: SridMode) -> Result<Vec<u8>> {
    let srid = normalise_srid_mode(srid);
    let trimmed = text.trim();
    match srid {
        SridMode::Auto => {
            if let Some(bytes) = try_decode_hex(trimmed) {
                Ok(bytes)
            } else {
                wkt_to_wkb::convert(trimmed)
            }
        }
        SridMode::Strip => {
            if let Some(bytes) = try_decode_hex(trimmed) {
                to_wkb_owned_bytes(bytes, srid)
            } else {
                wkt_to_wkb_split_srid(trimmed).map(|(b, _)| b)
            }
        }
        SridMode::Set(srid_val) => {
            if let Some(bytes) = try_decode_hex(trimmed) {
                to_wkb_owned_bytes(bytes, srid)
            } else {
                wkt_to_wkb::convert_with_forced_srid(trimmed, srid_val)
            }
        }
    }
}

fn to_wkb_owned_bytes(wkb: Vec<u8>, srid: SridMode) -> Result<Vec<u8>> {
    match apply_srid_to_wkb(&wkb, srid)? {
        Cow::Borrowed(_) => Ok(wkb),
        Cow::Owned(out) => Ok(out),
    }
}

fn to_wkb_bytes(wkb: &[u8], srid: SridMode) -> Result<Vec<u8>> {
    Ok(apply_srid_to_wkb(wkb, normalise_srid_mode(srid))?.into_owned())
}

fn apply_srid_to_wkb(wkb: &[u8], srid: SridMode) -> Result<Cow<'_, [u8]>> {
    match srid {
        SridMode::Auto => Ok(Cow::Borrowed(wkb)),
        SridMode::Strip => match try_strip_srid_from_le_wkb(wkb) {
            Some(Cow::Borrowed(_)) => Ok(Cow::Borrowed(wkb)),
            Some(Cow::Owned(out)) => Ok(Cow::Owned(out)),
            None => {
                let (wkt, _) = wkb_to_wkt_split_srid(wkb)?;
                wkt_to_wkb::convert(&wkt).map(Cow::Owned)
            }
        },
        SridMode::Set(srid_val) => match try_set_srid_in_le_wkb(wkb, srid_val) {
            Some(Cow::Borrowed(_)) => Ok(Cow::Borrowed(wkb)),
            Some(Cow::Owned(out)) => Ok(Cow::Owned(out)),
            None => {
                let (wkt, _) = wkb_to_wkt_split_srid(wkb)?;
                wkt_to_wkb::convert_with_forced_srid(&wkt, srid_val).map(Cow::Owned)
            }
        },
    }
}

/// Converts any WKT/EWKT string, hex-encoded WKB/EWKB string, or raw WKB bytes to a WKT
/// string.
///
/// The input format is detected automatically: a non-empty string with even
/// length composed entirely of hexadecimal characters is treated as hex WKB;
/// anything else (including odd-length all-hex strings) is treated as WKT.
/// Raw [`Input::Wkb`] bytes are always treated as WKB/EWKB, never as hex text.
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
/// input is hex WKB or raw WKB, which is always decoded to normalised WKT.
pub fn to_wkt(input: Input<'_>, srid: SridMode, normalize_wkt: bool) -> Result<String> {
    match input {
        Input::Text(text) => to_wkt_text(text, srid, normalize_wkt),
        Input::Wkb(wkb) => wkb_to_wkt(wkb, srid),
    }
}

fn to_wkt_text(text: &str, srid: SridMode, normalize_wkt: bool) -> Result<String> {
    let srid = normalise_srid_mode(srid);
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
    // returns None for non-hex body characters.  They are then handled as WKT-
    // like text: returned as-is when normalisation is disabled, or parsed as
    // WKT when normalisation is enabled.)
    if !normalize_wkt
        && trimmed
            .as_bytes()
            .first()
            .is_some_and(|b| !b.is_ascii_hexdigit())
    {
        return Ok(match srid {
            SridMode::Auto => strip_unknown_srid_prefix(trimmed).to_owned(),
            SridMode::Strip => strip_ewkt_prefix(trimmed).to_owned(),
            SridMode::Set(srid_val) => {
                format!("SRID={srid_val};{}", strip_ewkt_prefix(trimmed))
            }
        });
    }

    // Hex WKB is always decoded to normalised WKT.
    if let Some(decoded) = try_decode_hex(trimmed) {
        return match srid {
            SridMode::Auto => wkb_to_wkt::convert(&decoded),
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
            let wkb = wkt_to_wkb::convert(trimmed)?;
            wkb_to_wkt::convert(&wkb)
        }
        SridMode::Strip => {
            let (wkb, _) = wkt_to_wkb_split_srid(trimmed)?;
            wkb_to_wkt::convert(&wkb)
        }
        SridMode::Set(srid_val) => {
            let wkb = wkt_to_wkb::convert_with_forced_srid(trimmed, srid_val)?;
            wkb_to_wkt::convert(&wkb)
        }
    }
}

/// Converts any WKT/EWKT string, hex-encoded WKB/EWKB string, or raw WKB bytes to an uppercase
/// hex-encoded EWKB string.
///
/// The text input format is detected automatically. Raw [`Input::Wkb`] bytes
/// are always treated as WKB/EWKB, never as hex text.
///
/// `srid` controls SRID handling in the output — see [`SridMode`].
///
/// **Note:** hex text input under [`SridMode::Auto`] is validated as hex text
/// and uppercased without WKB structure validation. Raw WKB input under
/// [`SridMode::Auto`] is hex-encoded without WKB structure validation.
pub fn to_hex_wkb(input: Input<'_>, srid: SridMode) -> Result<String> {
    match input {
        Input::Text(text) => to_hex_wkb_text(text, srid),
        Input::Wkb(wkb) => to_hex_wkb_bytes(wkb, srid),
    }
}

fn to_hex_wkb_text(text: &str, srid: SridMode) -> Result<String> {
    let srid = normalise_srid_mode(srid);
    let trimmed = text.trim();
    if srid == SridMode::Auto {
        if let Some(hex) = try_normalize_hex_uppercase(trimmed) {
            return Ok(hex);
        }
    }
    match srid {
        SridMode::Auto => {}
        SridMode::Strip => {
            if let Some(bytes) = try_decode_hex(trimmed) {
                return to_hex_wkb_bytes(&bytes, srid);
            }
        }
        SridMode::Set(_) => {
            if let Some(bytes) = try_decode_hex(trimmed) {
                return to_hex_wkb_bytes(&bytes, srid);
            }
        }
    }
    match srid {
        SridMode::Auto => wkt_to_wkb::convert_to_hex(trimmed),
        SridMode::Strip => wkt_to_wkb::convert_to_hex_split_srid(trimmed),
        SridMode::Set(srid_val) => wkt_to_wkb::convert_to_hex_with_forced_srid(trimmed, srid_val),
    }
}

fn to_hex_wkb_bytes(wkb: &[u8], srid: SridMode) -> Result<String> {
    let srid = normalise_srid_mode(srid);
    match srid {
        SridMode::Auto => Ok(encode_hex(wkb)),
        SridMode::Strip => match try_strip_srid_from_le_wkb(wkb) {
            Some(Cow::Borrowed(_)) => Ok(encode_hex(wkb)),
            Some(Cow::Owned(out)) => Ok(encode_hex(&out)),
            None => {
                let (wkt, _) = wkb_to_wkt_split_srid(wkb)?;
                wkt_to_wkb::convert_to_hex_split_srid(&wkt)
            }
        },
        SridMode::Set(srid_val) => match try_set_srid_in_le_wkb(wkb, srid_val) {
            Some(Cow::Borrowed(_)) => Ok(encode_hex(wkb)),
            Some(Cow::Owned(out)) => Ok(encode_hex(&out)),
            None => {
                let (wkt, _) = wkb_to_wkt_split_srid(wkb)?;
                wkt_to_wkb::convert_to_hex_with_forced_srid(&wkt, srid_val)
            }
        },
    }
}

/// Strips a `SRID=N;` prefix only when N ≤ 0 (PostGIS "unknown SRID").
/// Returns the input unchanged if the prefix is absent or has a positive SRID.
fn strip_unknown_srid_prefix(wkt: &str) -> &str {
    if wkt
        .get(..5)
        .is_some_and(|p| p.eq_ignore_ascii_case("SRID="))
    {
        if let Some(rel) = wkt[5..].find(';') {
            if let Ok(n) = wkt[5..5 + rel].trim().parse::<i32>() {
                if n <= 0 {
                    return wkt[6 + rel..].trim_start();
                }
            }
        }
    }
    wkt
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

/// Fast path for reading the SRID from LE or BE WKB/EWKB headers.
///
/// Handles any byte order and any geometry type with only the three canonical
/// EWKB flag bits (Z, M, SRID) in the high half of the type word.  This
/// covers standard EWKB (types 1–7), ISO WKB (type codes ≥ 1000 which have
/// no high bits set and therefore no SRID flag), and plain WKB.
///
/// Returns `None` only when the high bits contain flags beyond Z/M/SRID
/// (unrecognised format) or the byte-order marker is invalid, so the caller
/// can fall back to a full parse.
///
/// Returns `Some(Err(...))` when the SRID flag is set but the slice is too
/// short to contain the 4-byte SRID field.
fn try_read_ewkb_srid_fast(wkb: &[u8]) -> Option<Result<Option<i32>>> {
    if wkb.len() < 5 {
        return None;
    }
    let little_endian = match wkb[0] {
        1 => true,
        0 => false,
        _ => return None,
    };
    let type_bytes: [u8; 4] = wkb[1..5].try_into().unwrap();
    let type_u32 = if little_endian {
        u32::from_le_bytes(type_bytes)
    } else {
        u32::from_be_bytes(type_bytes)
    };
    let high_bits = type_u32 & !0x0000_FFFF;
    if high_bits & !(EWKB_Z | EWKB_M | EWKB_SRID) != 0 {
        return None; // Unknown flag bits — format unrecognised, fall back
    }
    // If the SRID flag is not set there is no SRID in this header.  This
    // covers plain WKB, EWKB without SRID, and ISO WKB (no EWKB flags at all).
    if type_u32 & EWKB_SRID == 0 {
        return Some(Ok(None));
    }
    if wkb.len() < 9 {
        return Some(Err(Error::InvalidWkb(
            "EWKB header has SRID flag but is too short to contain the SRID field".into(),
        )));
    }
    let srid_bytes: [u8; 4] = wkb[5..9].try_into().unwrap();
    let srid = if little_endian {
        i32::from_le_bytes(srid_bytes)
    } else {
        i32::from_be_bytes(srid_bytes)
    };
    Some(Ok(Some(srid)))
}

/// Encode bytes as an uppercase hexadecimal string using a lookup table.
///
/// This is substantially faster than the `write!(s, "{b:02X}")` approach
/// because it avoids invoking the format machinery for every byte.
pub fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = vec![0u8; bytes.len().saturating_mul(2)];
    for (chunk, &b) in out.chunks_exact_mut(2).zip(bytes) {
        chunk[0] = HEX[(b >> 4) as usize];
        chunk[1] = HEX[(b & 0xF) as usize];
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

    let mut out = vec![0u8; bytes.len() / 2];
    out[0] = (hi << 4) | lo;
    for (dst, chunk) in out[1..].iter_mut().zip(bytes[2..].chunks_exact(2)) {
        let hi = HEX_NIBBLE_LUT[chunk[0] as usize];
        let lo = HEX_NIBBLE_LUT[chunk[1] as usize];
        if (hi | lo) > 0x0F {
            return None;
        }
        *dst = (hi << 4) | lo;
    }
    Some(out)
}

fn try_normalize_hex_uppercase(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    if bytes.is_empty() || !bytes.len().is_multiple_of(2) {
        return None;
    }

    let first = HEX_NIBBLE_LUT[bytes[0] as usize];
    if first > 0x0F {
        return None;
    }

    let mut out = vec![0u8; bytes.len()];
    out[0] = bytes[0].to_ascii_uppercase();
    for (dst, &byte) in out[1..].iter_mut().zip(&bytes[1..]) {
        if HEX_NIBBLE_LUT[byte as usize] > 0x0F {
            return None;
        }
        *dst = byte.to_ascii_uppercase();
    }
    // SAFETY: every byte is an ASCII hex digit, uppercased in place.
    Some(unsafe { String::from_utf8_unchecked(out) })
}

/// Decode a hex string with detailed position-aware error messages.
/// Used by [`hex_wkb_to_wkt`] where the caller knows the input must be hex.
pub fn decode_hex(hex: &str) -> Result<Vec<u8>> {
    let bytes = hex.as_bytes();
    if bytes.is_empty() {
        return Err(Error::InvalidWkb("empty hex string".into()));
    }
    if !bytes.len().is_multiple_of(2) {
        return Err(Error::InvalidWkb("hex string has odd length".into()));
    }
    let mut out = vec![0u8; bytes.len() / 2];
    for (i, (dst, chunk)) in out.iter_mut().zip(bytes.chunks_exact(2)).enumerate() {
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
        *dst = (hi << 4) | lo;
    }
    Ok(out)
}

struct LeFastPathHeader {
    type_u32: u32,
    canonical_type_without_srid: u32,
}

fn try_read_le_fast_path_header(bytes: &[u8]) -> Option<LeFastPathHeader> {
    if bytes.len() < 5 || bytes[0] != 1 {
        return None;
    }
    let type_u32 = u32::from_le_bytes(bytes[1..5].try_into().unwrap());
    let high_bits = type_u32 & !0x0000_FFFF;
    if high_bits & !(EWKB_Z | EWKB_M | EWKB_SRID) != 0 {
        return None;
    }
    let base_geom_type = type_u32 & 0x0000_FFFF;
    if !(1..=7).contains(&base_geom_type) {
        return None;
    }

    if type_u32 & EWKB_SRID != 0 && bytes.len() < 9 {
        return None;
    }

    Some(LeFastPathHeader {
        type_u32,
        canonical_type_without_srid: type_u32 & !EWKB_SRID,
    })
}

/// Strip the SRID from a canonical little-endian EWKB byte slice without
/// parsing the geometry body.
///
/// Returns `None` when the bytes are not recognisable as little-endian EWKB
/// with canonical EWKB type flags. In those cases the caller should fall back
/// to a full WKB→WKT→WKB round-trip, which normalises type codes and handles
/// big-endian or ISO-dimensional input.
fn try_strip_srid_from_le_wkb(bytes: &[u8]) -> Option<Cow<'_, [u8]>> {
    let header = try_read_le_fast_path_header(bytes)?;
    if header.type_u32 & EWKB_SRID == 0 {
        return Some(Cow::Borrowed(bytes));
    }
    let mut out = Vec::with_capacity(bytes.len() - 4);
    out.push(1u8); // little-endian marker
    out.extend_from_slice(&header.canonical_type_without_srid.to_le_bytes());
    out.extend_from_slice(&bytes[9..]); // skip the 4 SRID bytes
    Some(Cow::Owned(out))
}

/// Inject or replace the SRID in a canonical little-endian EWKB byte slice
/// without parsing the geometry body.
///
/// Returns `None` when the bytes are not recognisable as little-endian EWKB
/// with canonical EWKB type flags. In those cases the caller should fall back
/// to a full WKB→WKT→WKB round-trip, which normalises type codes and handles
/// big-endian or ISO-dimensional input.
fn try_set_srid_in_le_wkb(bytes: &[u8], srid: i32) -> Option<Cow<'_, [u8]>> {
    let header = try_read_le_fast_path_header(bytes)?;
    let canonical_type_with_srid = header.canonical_type_without_srid | EWKB_SRID;
    if header.type_u32 & EWKB_SRID != 0 {
        // SRID already present — check if it already equals the requested SRID.
        let stored_srid = i32::from_le_bytes(bytes[5..9].try_into().unwrap());
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
