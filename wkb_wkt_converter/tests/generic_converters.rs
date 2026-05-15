/// Tests for the generic to_wkb / to_wkt / to_hex_wkb helpers.
use wkb_wkt_converter::{
    hex_wkb_to_wkt, to_hex_wkb as core_to_hex_wkb, to_wkb as core_to_wkb, to_wkt as core_to_wkt,
    wkb_to_wkt as core_wkb_to_wkt, wkt_to_hex_wkb as core_wkt_to_hex_wkb,
    wkt_to_wkb as core_wkt_to_wkb, Input, Result, SridMode,
};

// Big-endian WKB for POINT (1 2): byte-order=0x00, type=0x00000001 (BE),
// X=1.0 (BE f64), Y=2.0 (BE f64).
const BE_POINT_HEX: &str = "00000000013FF00000000000004000000000000000";

// Little-endian WKB with the EWKB SRID flag set in the type word but only 5
// bytes total (no room for the 4-byte SRID value).  Used to exercise the
// malformed-WKB fallback path.
const LE_SHORT_SRID_HEX: &str = "0103000020";

// Little-endian POINT header with no coordinate body.
const LE_SHORT_POINT_HEX: &str = "0101000000";

// Uppercase hex-encoded little-endian WKB for POINT (1 2) without SRID.
// byte-order=0x01, type=0x00000001 (LE), X=1.0 (LE f64), Y=2.0 (LE f64).
const POINT_HEX: &str = "0101000000000000000000F03F0000000000000040";

// Uppercase hex-encoded little-endian EWKB for SRID=4326;POINT (1 2).
// byte-order=0x01, type=0x20000001 (LE, SRID flag), SRID=4326 (LE u32),
// X=1.0 (LE f64), Y=2.0 (LE f64).
const SRID_POINT_HEX: &str = "0101000020E6100000000000000000F03F0000000000000040";

// ── helpers ──────────────────────────────────────────────────────────────────

fn wkb_to_wkt(wkb: &[u8]) -> Result<String> {
    core_wkb_to_wkt(wkb, SridMode::Auto)
}

fn wkt_to_wkb(wkt: &str) -> Result<Vec<u8>> {
    core_wkt_to_wkb(wkt, SridMode::Auto)
}

fn wkt_to_hex_wkb(wkt: &str) -> Result<String> {
    core_wkt_to_hex_wkb(wkt, SridMode::Auto)
}

fn to_wkb(text: &str, srid: SridMode) -> Result<Vec<u8>> {
    core_to_wkb(Input::Text(text), srid)
}

fn to_wkb_bytes(wkb: &[u8], srid: SridMode) -> Result<Vec<u8>> {
    core_to_wkb(Input::Wkb(wkb), srid)
}

fn to_wkt(text: &str, srid: SridMode, normalize_wkt: bool) -> Result<String> {
    core_to_wkt(Input::Text(text), srid, normalize_wkt)
}

fn to_wkt_bytes(wkb: &[u8], srid: SridMode, normalize_wkt: bool) -> Result<String> {
    core_to_wkt(Input::Wkb(wkb), srid, normalize_wkt)
}

fn to_hex_wkb(text: &str, srid: SridMode) -> Result<String> {
    core_to_hex_wkb(Input::Text(text), srid)
}

fn to_hex_wkb_bytes(wkb: &[u8], srid: SridMode) -> Result<String> {
    core_to_hex_wkb(Input::Wkb(wkb), srid)
}

fn hex(wkt: &str) -> String {
    wkt_to_hex_wkb(wkt).unwrap()
}

fn le_point_hex_with_type(type_u32: u32, srid: Option<u32>) -> String {
    let mut wkb = vec![0x01u8];
    wkb.extend_from_slice(&type_u32.to_le_bytes());
    if let Some(srid) = srid {
        wkb.extend_from_slice(&srid.to_le_bytes());
    }
    wkb.extend_from_slice(&1.0f64.to_le_bytes());
    wkb.extend_from_slice(&2.0f64.to_le_bytes());
    hex::encode_upper(wkb)
}

fn le_point_hex_with_coords(coords: &[f64]) -> String {
    let mut wkb = vec![0x01u8];
    wkb.extend_from_slice(&1u32.to_le_bytes());
    for coord in coords {
        wkb.extend_from_slice(&coord.to_le_bytes());
    }
    hex::encode_upper(wkb)
}

fn le_linestring_hex_with_coords(coords: &[(f64, f64)]) -> String {
    let mut wkb = vec![0x01u8];
    wkb.extend_from_slice(&2u32.to_le_bytes());
    wkb.extend_from_slice(&(coords.len() as u32).to_le_bytes());
    for (x, y) in coords {
        wkb.extend_from_slice(&x.to_le_bytes());
        wkb.extend_from_slice(&y.to_le_bytes());
    }
    hex::encode_upper(wkb)
}

// ── to_wkb: input detection ─────────────────────────────────────────────

#[test]
fn wkb_from_wkt_matches_direct() {
    let via_text = to_wkb("POINT (1 2)", SridMode::Auto).unwrap();
    let direct = wkt_to_wkb("POINT (1 2)").unwrap();
    assert_eq!(via_text, direct);
}

#[test]
fn wkb_from_hex_wkb_matches_direct() {
    let h = hex("POINT (1 2)");
    let via_text = to_wkb(&h, SridMode::Auto).unwrap();
    let direct = wkt_to_wkb("POINT (1 2)").unwrap();
    assert_eq!(via_text, direct);
}

#[test]
fn wkb_from_wkt_with_leading_whitespace() {
    let via_text = to_wkb("  POINT (1 2)  ", SridMode::Auto).unwrap();
    let direct = wkt_to_wkb("POINT (1 2)").unwrap();
    assert_eq!(via_text, direct);
}

#[test]
fn wkb_input_auto_returns_raw_bytes_without_validation() {
    let raw = POINT_HEX.as_bytes();
    let wkb = to_wkb_bytes(raw, SridMode::Auto).unwrap();
    assert_eq!(wkb, raw);
}

#[test]
fn wkb_input_auto_does_not_decode_ascii_hex_bytes() {
    let raw = POINT_HEX.as_bytes();
    let wkb = to_wkb_bytes(raw, SridMode::Auto).unwrap();
    assert_ne!(wkb, hex::decode(POINT_HEX).unwrap());
    assert_eq!(
        to_hex_wkb_bytes(raw, SridMode::Auto).unwrap(),
        hex::encode_upper(raw)
    );
}

// ── to_wkb: SridMode::Auto ──────────────────────────────────────────────

#[test]
fn wkb_auto_preserves_srid_from_ewkt() {
    let wkb = to_wkb("SRID=4326;POINT (1 2)", SridMode::Auto).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn wkb_auto_preserves_no_srid_from_wkt() {
    let wkb = to_wkb("POINT (1 2)", SridMode::Auto).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkb_auto_preserves_srid_from_hex_ewkb() {
    let h = hex("SRID=4326;POINT (1 2)");
    let wkb = to_wkb(&h, SridMode::Auto).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn wkb_auto_preserves_no_srid_from_plain_hex_wkb() {
    let h = hex("POINT (1 2)");
    let wkb = to_wkb(&h, SridMode::Auto).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

// ── to_wkb: SridMode::Strip ─────────────────────────────────────────────

#[test]
fn wkb_strip_removes_srid_from_ewkt() {
    let wkb = to_wkb("SRID=4326;POINT (1 2)", SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkb_strip_removes_srid_from_hex_ewkb() {
    let h = hex("SRID=4326;POINT (1 2)");
    let wkb = to_wkb(&h, SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkb_strip_noop_when_no_srid_in_wkt() {
    let wkb = to_wkb("POINT (1 2)", SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

// ── to_wkb: SridMode::Set ───────────────────────────────────────────────

#[test]
fn wkb_set_adds_srid_to_plain_wkt() {
    let wkb = to_wkb("POINT (1 2)", SridMode::Set(4326)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn wkb_set_overrides_existing_srid_in_ewkt() {
    let wkb = to_wkb("SRID=4326;POINT (1 2)", SridMode::Set(3857)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=3857;POINT (1 2)");
}

#[test]
fn wkb_set_adds_srid_to_plain_hex_wkb() {
    let h = hex("POINT (1 2)");
    let wkb = to_wkb(&h, SridMode::Set(4326)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn wkb_set_overrides_srid_in_hex_ewkb() {
    let h = hex("SRID=4326;POINT (1 2)");
    let wkb = to_wkb(&h, SridMode::Set(3857)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=3857;POINT (1 2)");
}

#[test]
fn wkb_input_strip_and_set_use_raw_bytes() {
    let srid_wkb = hex::decode(SRID_POINT_HEX).unwrap();

    let stripped = to_wkb_bytes(&srid_wkb, SridMode::Strip).unwrap();
    assert_eq!(stripped, hex::decode(POINT_HEX).unwrap());

    let set = to_wkb_bytes(&srid_wkb, SridMode::Set(3857)).unwrap();
    assert_eq!(wkb_to_wkt(&set).unwrap(), "SRID=3857;POINT (1 2)");
}

// ── edge cases ───────────────────────────────────────────────────────────────

#[test]
fn empty_string_errors() {
    assert!(to_wkb("", SridMode::Auto).is_err());
    assert!(to_wkt("", SridMode::Auto, true).is_err());
    assert!(to_hex_wkb("", SridMode::Auto).is_err());
    assert!(to_wkt("", SridMode::Auto, false).is_err());
}

#[test]
fn decode_hex_lowercase_accepted() {
    // parse_hex_nibble must handle b'a'..=b'f'
    let lower = hex("POINT (1 2)").to_lowercase();
    let via_lower = to_wkb(&lower, SridMode::Auto).unwrap();
    assert_eq!(via_lower, wkt_to_wkb("POINT (1 2)").unwrap());
}

#[test]
fn decode_hex_invalid_low_nibble_errors() {
    // '0' is a valid high nibble; 'Z' is an invalid low nibble.
    // try_decode_hex("0Z") returns None (not valid hex), so to_wkb
    // would silently route to WKT parsing.  hex_wkb_to_wkt uses decode_hex
    // directly and therefore returns a descriptive error.
    assert!(hex_wkb_to_wkt("0Z", SridMode::Auto).is_err());
}

#[test]
fn strip_ewkt_prefix_no_semicolon_returns_unchanged() {
    // "SRID=" present but no semicolon → strip_ewkt_prefix falls through unchanged
    assert_eq!(
        to_wkt("SRID=4326POINT (1 2)", SridMode::Strip, false).unwrap(),
        "SRID=4326POINT (1 2)"
    );
}

#[test]
fn odd_length_hex_string_errors() {
    // Odd-length all-hex input: try_decode_hex returns None (odd length) so
    // the input falls through to WKT parsing, which also rejects it.
    assert!(to_wkb("ABC", SridMode::Auto).is_err());
    assert!(to_wkt("ABC", SridMode::Auto, true).is_err());
    assert!(to_hex_wkb("ABC", SridMode::Auto).is_err());
}

/// SRID=0 is "unknown" per PostGIS convention; Set(0) behaves like Strip.
#[test]
fn srid_set_zero_acts_as_strip() {
    let wkb = to_wkb("SRID=4326;POINT (1 2)", SridMode::Set(0)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

// ── EWKT with negative/zero SRID prefix ─────────────────────────────────

/// EWKT with SRID=0 prefix: tokenizer normalises to no-SRID, treated as plain WKT.
#[test]
fn ewkt_zero_srid_prefix_acts_as_plain_wkt_for_to_wkb() {
    let wkb = to_wkb("SRID=0;POINT (1 2)", SridMode::Auto).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

/// EWKT with SRID=-1 prefix: tokenizer normalises to no-SRID, treated as plain WKT.
#[test]
fn ewkt_negative_srid_prefix_acts_as_plain_wkt_for_to_wkb() {
    let wkb = to_wkb("SRID=-1;POINT (1 2)", SridMode::Auto).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn ewkt_negative_srid_prefix_acts_as_plain_wkt_for_to_wkt() {
    assert_eq!(
        to_wkt("SRID=-1;POINT (1 2)", SridMode::Auto, true).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn ewkt_negative_srid_prefix_acts_as_plain_wkt_for_to_hex_wkb() {
    let h = to_hex_wkb("SRID=-1;POINT (1 2)", SridMode::Auto).unwrap();
    assert_eq!(
        wkb_to_wkt(&hex::decode(&h).unwrap()).unwrap(),
        "POINT (1 2)"
    );
}

// ── to_wkt: input detection and normalisation ───────────────────────────

#[test]
fn wkt_from_wkt_normalises_casing_and_spacing() {
    assert_eq!(
        to_wkt("point(1 2)", SridMode::Auto, true).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_from_hex_wkb_roundtrip() {
    let h = hex("LINESTRING (0 0, 1 1)");
    assert_eq!(
        to_wkt(&h, SridMode::Auto, true).unwrap(),
        "LINESTRING (0 0, 1 1)"
    );
}

// ── to_wkt: SridMode::Auto ──────────────────────────────────────────────

#[test]
fn wkt_auto_preserves_srid_from_ewkt() {
    assert_eq!(
        to_wkt("SRID=4326;POINT (1 2)", SridMode::Auto, true).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

#[test]
fn wkt_auto_preserves_no_srid_from_wkt() {
    assert_eq!(
        to_wkt("POINT (1 2)", SridMode::Auto, true).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_auto_preserves_srid_from_hex_ewkb() {
    let h = hex("SRID=4326;POINT (1 2)");
    assert_eq!(
        to_wkt(&h, SridMode::Auto, true).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

// ── to_wkt: SridMode::Strip ─────────────────────────────────────────────

#[test]
fn wkt_strip_removes_srid_from_ewkt() {
    assert_eq!(
        to_wkt("SRID=4326;POINT (1 2)", SridMode::Strip, true).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_strip_removes_srid_from_hex_ewkb() {
    let h = hex("SRID=4326;POINT (1 2)");
    assert_eq!(to_wkt(&h, SridMode::Strip, true).unwrap(), "POINT (1 2)");
}

// ── to_wkt: SridMode::Set ───────────────────────────────────────────────

#[test]
fn wkt_set_adds_srid_to_plain_wkt() {
    assert_eq!(
        to_wkt("POINT (1 2)", SridMode::Set(4326), true).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

#[test]
fn wkt_set_overrides_existing_srid_in_ewkt() {
    assert_eq!(
        to_wkt("SRID=4326;POINT (1 2)", SridMode::Set(3857), true).unwrap(),
        "SRID=3857;POINT (1 2)"
    );
}

#[test]
fn wkt_set_adds_srid_to_plain_hex_wkb() {
    let h = hex("POINT (1 2)");
    assert_eq!(
        to_wkt(&h, SridMode::Set(4326), true).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

// ── to_wkt: normalize_wkt=false ─────────────────────────────────────────

#[test]
fn wkt_no_normalize_returns_wkt_as_is() {
    assert_eq!(
        to_wkt("point(1 2)", SridMode::Auto, false).unwrap(),
        "point(1 2)"
    );
}

#[test]
fn wkt_no_normalize_auto_preserves_srid_prefix() {
    assert_eq!(
        to_wkt("SRID=4326;POINT (1 2)", SridMode::Auto, false).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_auto_strips_negative_srid_prefix() {
    assert_eq!(
        to_wkt("SRID=-1;POINT (1 2)", SridMode::Auto, false).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_auto_strips_zero_srid_prefix() {
    assert_eq!(
        to_wkt("SRID=0;POINT (1 2)", SridMode::Auto, false).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_auto_unparseable_srid_returned_as_is() {
    // SRID value is not a valid i32 → strip_unknown_srid_prefix falls through unchanged
    assert_eq!(
        to_wkt("SRID=notanint;POINT (1 2)", SridMode::Auto, false).unwrap(),
        "SRID=notanint;POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_auto_srid_prefix_without_semicolon_returned_as_is() {
    // "SRID=" present but no ";" → strip_unknown_srid_prefix falls through unchanged
    assert_eq!(
        to_wkt("SRID=4326POINT (1 2)", SridMode::Auto, false).unwrap(),
        "SRID=4326POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_strip_removes_srid_prefix() {
    assert_eq!(
        to_wkt("SRID=4326;POINT (1 2)", SridMode::Strip, false).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_strip_noop_when_no_srid() {
    assert_eq!(
        to_wkt("POINT (1 2)", SridMode::Strip, false).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_set_adds_srid_prefix() {
    assert_eq!(
        to_wkt("POINT (1 2)", SridMode::Set(4326), false).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_set_overrides_existing_srid() {
    assert_eq!(
        to_wkt("SRID=4326;POINT (1 2)", SridMode::Set(3857), false).unwrap(),
        "SRID=3857;POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_hex_input_still_normalises() {
    // normalize_wkt has no effect on hex WKB input; result is always normalised WKT.
    let h = hex("POINT (1 2)");
    assert_eq!(to_wkt(&h, SridMode::Auto, false).unwrap(), "POINT (1 2)");
}

#[test]
fn wkb_input_to_wkt_ignores_normalize_wkt() {
    let wkb = wkt_to_wkb("SRID=4326;POINT (1 2)").unwrap();
    assert_eq!(
        to_wkt_bytes(&wkb, SridMode::Auto, false).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
    assert_eq!(
        to_wkt_bytes(&wkb, SridMode::Strip, true).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkb_input_to_wkt_invalid_bytes_error_even_without_normalize() {
    assert!(to_wkt_bytes(b"0101000000", SridMode::Auto, false).is_err());
}

// ── to_hex_wkb ──────────────────────────────────────────────────────────

#[test]
fn hex_wkb_from_wkt_is_uppercase() {
    let h = to_hex_wkb("POINT (1 2)", SridMode::Auto).unwrap();
    assert_eq!(h, h.to_uppercase());
}

#[test]
fn hex_wkb_from_wkt_roundtrip() {
    let h = to_hex_wkb("POINT (1 2)", SridMode::Auto).unwrap();
    assert_eq!(
        wkb_to_wkt(&hex::decode(&h).unwrap()).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn hex_wkb_from_hex_wkb_roundtrip() {
    let original = hex("LINESTRING (0 0, 1 1)");
    let result = to_hex_wkb(&original, SridMode::Auto).unwrap();
    assert_eq!(result, original);
}

#[test]
fn hex_wkb_strip_from_hex_without_srid_reencodes_same_bytes() {
    assert_eq!(to_hex_wkb(POINT_HEX, SridMode::Strip).unwrap(), POINT_HEX);
}

#[test]
fn hex_wkb_strip_from_hex_removes_srid_header() {
    assert_eq!(
        to_hex_wkb(SRID_POINT_HEX, SridMode::Strip).unwrap(),
        POINT_HEX
    );
}

#[test]
fn hex_wkb_strip_from_big_endian_hex_falls_back_to_round_trip() {
    assert_eq!(
        to_hex_wkb(BE_POINT_HEX, SridMode::Strip).unwrap(),
        POINT_HEX
    );
}

#[test]
fn hex_wkb_set_adds_srid() {
    let h = to_hex_wkb("POINT (1 2)", SridMode::Set(4326)).unwrap();
    let wkt = wkb_to_wkt(&hex::decode(&h).unwrap()).unwrap();
    assert_eq!(wkt, "SRID=4326;POINT (1 2)");
}

#[test]
fn hex_wkb_set_from_hex_adds_srid_header() {
    assert_eq!(
        to_hex_wkb(POINT_HEX, SridMode::Set(4326)).unwrap(),
        SRID_POINT_HEX
    );
}

#[test]
fn hex_wkb_set_from_hex_noops_when_srid_already_matches() {
    assert_eq!(
        to_hex_wkb(SRID_POINT_HEX, SridMode::Set(4326)).unwrap(),
        SRID_POINT_HEX
    );
}

#[test]
fn hex_wkb_set_from_big_endian_hex_falls_back_to_round_trip() {
    assert_eq!(
        to_hex_wkb(BE_POINT_HEX, SridMode::Set(4326)).unwrap(),
        SRID_POINT_HEX
    );
}

#[test]
fn hex_wkb_strip_removes_srid() {
    let h = to_hex_wkb("SRID=4326;POINT (1 2)", SridMode::Strip).unwrap();
    let wkt = wkb_to_wkt(&hex::decode(&h).unwrap()).unwrap();
    assert_eq!(wkt, "POINT (1 2)");
}

#[test]
fn hex_wkb_from_wkb_input_auto_encodes_without_validation() {
    let malformed = hex::decode(LE_SHORT_POINT_HEX).unwrap();
    assert_eq!(
        to_hex_wkb_bytes(&malformed, SridMode::Auto).unwrap(),
        LE_SHORT_POINT_HEX
    );
}

#[test]
fn hex_wkb_from_wkb_input_strip_and_set_use_raw_bytes() {
    let srid_wkb = hex::decode(SRID_POINT_HEX).unwrap();
    assert_eq!(
        to_hex_wkb_bytes(&srid_wkb, SridMode::Strip).unwrap(),
        POINT_HEX
    );
    assert_eq!(
        to_hex_wkb_bytes(&hex::decode(POINT_HEX).unwrap(), SridMode::Set(4326)).unwrap(),
        SRID_POINT_HEX
    );
}

// ── try_strip_srid_from_le_wkb / try_set_srid_in_le_wkb coverage ─────────────

#[test]
fn wkb_strip_noop_when_no_srid_in_hex_wkb() {
    // hex WKB without SRID flag: fast binary path returns bytes unchanged.
    let h = hex("POINT (1 2)");
    let wkb = to_wkb(&h, SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkb_strip_big_endian_hex_falls_back_to_round_trip() {
    // Big-endian WKB: try_strip_srid_from_le_wkb returns None, so to_wkb
    // falls back to the full WKB→WKT→WKB round-trip which normalises to LE.
    let wkb = to_wkb(BE_POINT_HEX, SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkb_input_big_endian_falls_back_to_round_trip_for_strip_and_set() {
    let be_wkb = hex::decode(BE_POINT_HEX).unwrap();

    let stripped = to_wkb_bytes(&be_wkb, SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&stripped).unwrap(), "POINT (1 2)");
    assert_eq!(stripped[0], 0x01);

    let set = to_wkb_bytes(&be_wkb, SridMode::Set(4326)).unwrap();
    assert_eq!(wkb_to_wkt(&set).unwrap(), "SRID=4326;POINT (1 2)");
    assert_eq!(set[0], 0x01);
}

#[test]
fn wkb_strip_malformed_short_srid_errors() {
    // LE WKB with SRID flag but only 5 bytes (no room for 4-byte SRID value):
    // try_strip_srid_from_le_wkb returns None, fallback round-trip fails.
    assert!(to_wkb(LE_SHORT_SRID_HEX, SridMode::Strip).is_err());
}

#[test]
fn wkb_set_big_endian_hex_falls_back_to_round_trip() {
    // Big-endian WKB: try_set_srid_in_le_wkb returns None, fallback works.
    let wkb = to_wkb(BE_POINT_HEX, SridMode::Set(4326)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn wkb_set_malformed_short_srid_errors() {
    // LE WKB with SRID flag but too short: try_set_srid_in_le_wkb returns None,
    // fallback round-trip fails.
    assert!(to_wkb(LE_SHORT_SRID_HEX, SridMode::Set(4326)).is_err());
}

#[test]
fn wkb_input_strip_and_set_malformed_short_srid_errors() {
    let malformed = hex::decode(LE_SHORT_SRID_HEX).unwrap();
    assert!(to_wkb_bytes(&malformed, SridMode::Strip).is_err());
    assert!(to_wkb_bytes(&malformed, SridMode::Set(4326)).is_err());
}

#[test]
fn wkb_strip_short_point_header_passes_through() {
    let wkb = to_wkb(LE_SHORT_POINT_HEX, SridMode::Strip).unwrap();
    assert_eq!(wkb, hex::decode(LE_SHORT_POINT_HEX).unwrap());
}

#[test]
fn wkb_set_short_point_header_patches_header_only() {
    let wkb = to_wkb(LE_SHORT_POINT_HEX, SridMode::Set(4326)).unwrap();
    assert_eq!(wkb.len(), 9);
    assert_eq!(wkb[0], 1);
    assert_eq!(
        u32::from_le_bytes(wkb[1..5].try_into().unwrap()),
        0x2000_0001
    );
    assert_eq!(u32::from_le_bytes(wkb[5..9].try_into().unwrap()), 4326);
}

#[test]
fn wkb_input_short_point_header_uses_same_fast_path() {
    let malformed = hex::decode(LE_SHORT_POINT_HEX).unwrap();
    assert_eq!(
        to_wkb_bytes(&malformed, SridMode::Strip).unwrap(),
        malformed
    );

    let with_srid = to_wkb_bytes(&malformed, SridMode::Set(4326)).unwrap();
    assert_eq!(with_srid.len(), 9);
    assert_eq!(
        u32::from_le_bytes(with_srid[1..5].try_into().unwrap()),
        0x2000_0001
    );
    assert_eq!(
        u32::from_le_bytes(with_srid[5..9].try_into().unwrap()),
        4326
    );
}

#[test]
fn wkb_auto_hex_preserves_malformed_bytes_without_validation() {
    let wkb = to_wkb(LE_SHORT_POINT_HEX, SridMode::Auto).unwrap();
    assert_eq!(wkb, hex::decode(LE_SHORT_POINT_HEX).unwrap());
    assert_eq!(
        to_hex_wkb(&LE_SHORT_POINT_HEX.to_lowercase(), SridMode::Auto).unwrap(),
        LE_SHORT_POINT_HEX
    );
}

#[test]
fn wkb_strip_and_set_allow_non_finite_simple_fast_path_coords() {
    let partial_nan = le_point_hex_with_coords(&[f64::NAN, 2.0]);
    let stripped = to_wkb(&partial_nan, SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&stripped).unwrap(), "POINT (NaN 2)");
    let with_srid = to_wkb(&partial_nan, SridMode::Set(4326)).unwrap();
    assert_eq!(wkb_to_wkt(&with_srid).unwrap(), "SRID=4326;POINT (NaN 2)");

    let infinity = le_point_hex_with_coords(&[f64::INFINITY, 2.0]);
    let stripped = to_wkb(&infinity, SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&stripped).unwrap(), "POINT (inf 2)");
    let with_srid = to_wkb(&infinity, SridMode::Set(4326)).unwrap();
    assert_eq!(wkb_to_wkt(&with_srid).unwrap(), "SRID=4326;POINT (inf 2)");

    let linestring_nan = le_linestring_hex_with_coords(&[(0.0, 0.0), (1.0, f64::NAN)]);
    let stripped = to_wkb(&linestring_nan, SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&stripped).unwrap(), "LINESTRING (0 0, 1 NaN)");
    let with_srid = to_wkb(&linestring_nan, SridMode::Set(4326)).unwrap();
    assert_eq!(
        wkb_to_wkt(&with_srid).unwrap(),
        "SRID=4326;LINESTRING (0 0, 1 NaN)"
    );
}

#[test]
fn wkb_strip_and_set_accept_all_nan_point_empty() {
    let empty_point = le_point_hex_with_coords(&[f64::NAN, f64::NAN]);
    let stripped = to_wkb(&empty_point, SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&stripped).unwrap(), "POINT EMPTY");
    let with_srid = to_wkb(&empty_point, SridMode::Set(4326)).unwrap();
    assert_eq!(wkb_to_wkt(&with_srid).unwrap(), "SRID=4326;POINT EMPTY");
}

#[test]
fn wkb_strip_plain_point_with_trailing_bytes_passes_through() {
    let input = format!("{POINT_HEX}DEADBEEF");
    assert_eq!(
        to_wkb(&input, SridMode::Strip).unwrap(),
        hex::decode(input).unwrap()
    );
}

#[test]
fn wkb_set_plain_point_with_trailing_bytes_patches_header_only() {
    let input = format!("{POINT_HEX}DEADBEEF");
    let wkb = to_wkb(&input, SridMode::Set(4326)).unwrap();
    assert_eq!(wkb.len(), hex::decode(input).unwrap().len() + 4);
    assert_eq!(
        u32::from_le_bytes(wkb[1..5].try_into().unwrap()),
        0x2000_0001
    );
    assert_eq!(u32::from_le_bytes(wkb[5..9].try_into().unwrap()), 4326);
    assert_eq!(&wkb[wkb.len() - 4..], &[0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn to_wkt_hex_with_trailing_bytes_errors() {
    let input = format!("{POINT_HEX}DEADBEEF");
    assert!(to_wkt(&input, SridMode::Auto, true).is_err());
    assert!(to_wkt(&input, SridMode::Strip, true).is_err());
    assert!(to_wkt(&input, SridMode::Set(4326), true).is_err());
}

#[test]
fn wkb_strip_short_linestring_header_passes_through() {
    let wkb = to_wkb("0102000000", SridMode::Strip).unwrap();
    assert_eq!(wkb, hex::decode("0102000000").unwrap());
}

#[test]
fn wkb_strip_valid_empty_linestring_uses_fast_path() {
    let wkb = to_wkb(&hex("LINESTRING EMPTY"), SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "LINESTRING EMPTY");
}

#[test]
fn wkb_strip_valid_polygon_uses_fast_path() {
    let wkb = to_wkb(&hex("POLYGON ((0 0, 1 0, 1 1, 0 0))"), SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POLYGON ((0 0, 1 0, 1 1, 0 0))");
}

#[test]
fn wkb_strip_canonicalizes_unknown_high_type_bits() {
    let h = le_point_hex_with_type(0x1000_0001, None);
    let wkb = to_wkb(&h, SridMode::Strip).unwrap();
    assert_eq!(u32::from_le_bytes(wkb[1..5].try_into().unwrap()), 1);
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkb_set_canonicalizes_unknown_high_type_bits_when_srid_matches() {
    let h = le_point_hex_with_type(0x3000_0001, Some(4326));
    let wkb = to_wkb(&h, SridMode::Set(4326)).unwrap();
    assert_eq!(
        u32::from_le_bytes(wkb[1..5].try_into().unwrap()),
        0x2000_0001
    );
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn wkb_set_canonicalizes_unknown_high_type_bits_when_adding_srid() {
    let h = le_point_hex_with_type(0x1000_0001, None);
    let wkb = to_wkb(&h, SridMode::Set(4326)).unwrap();
    assert_eq!(
        u32::from_le_bytes(wkb[1..5].try_into().unwrap()),
        0x2000_0001
    );
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

// ── to_wkt: normalize_wkt=false with hex-digit-starting non-hex input ───

#[test]
fn wkt_no_normalize_hex_start_non_hex_body_returned_as_is() {
    // Input begins with a hex digit ('0') but is not valid hex (contains 'X').
    // The fast O(1) first-char check does NOT apply (first byte IS a hex digit),
    // so try_decode_hex is attempted, fails, and the WKT is returned as-is.
    assert_eq!(to_wkt("0XTEST", SridMode::Auto, false).unwrap(), "0XTEST");
    assert_eq!(to_wkt("0XTEST", SridMode::Strip, false).unwrap(), "0XTEST");
    assert_eq!(
        to_wkt("0XTEST", SridMode::Set(4326), false).unwrap(),
        "SRID=4326;0XTEST"
    );
}

#[test]
fn wkt_no_normalize_hex_start_later_non_hex_body_returned_as_is() {
    assert_eq!(to_wkt("00X0", SridMode::Auto, false).unwrap(), "00X0");
}

#[test]
fn wkt_no_normalize_odd_length_all_hex_returned_as_unvalidated_wkt() {
    assert_eq!(to_wkt("ABC", SridMode::Auto, false).unwrap(), "ABC");
    assert_eq!(to_wkt("ABC", SridMode::Strip, false).unwrap(), "ABC");
    assert_eq!(
        to_wkt("ABC", SridMode::Set(4326), false).unwrap(),
        "SRID=4326;ABC"
    );
}

// ── new tests for Change 2–5 ──────────────────────────────────────────────────

#[test]
fn decode_hex_empty_string_errors() {
    assert!(hex_wkb_to_wkt("", SridMode::Auto).is_err());
}

#[test]
fn auto_be_hex_input_returned_as_be_bytes() {
    // Big-endian WKB passes through SridMode::Auto without normalisation.
    // The first byte of the result must be 0x00 (big-endian byte-order marker).
    let result = to_wkb(BE_POINT_HEX, SridMode::Auto).unwrap();
    assert_eq!(result[0], 0x00, "expected big-endian marker byte 0x00");
}

#[test]
fn to_hex_wkb_lowercase_hex_normalised_to_uppercase() {
    // Lowercase hex input under SridMode::Auto is validated as hex text and
    // uppercased without WKB structure validation.
    let lowercase = POINT_HEX.to_lowercase();
    let result = to_hex_wkb(&lowercase, SridMode::Auto).unwrap();
    assert_eq!(result, POINT_HEX);
}

#[test]
fn to_hex_wkb_auto_uppercases_without_wkb_validation() {
    assert_eq!(
        to_hex_wkb("0101000000deadbeef", SridMode::Auto).unwrap(),
        "0101000000DEADBEEF"
    );
}

#[test]
fn to_hex_wkb_auto_invalid_hex_falls_back_to_wkt_errors() {
    assert!(to_hex_wkb("ZZ", SridMode::Auto).is_err());
    assert!(to_hex_wkb("0Z", SridMode::Auto).is_err());
}

#[test]
fn set_srid_noop_when_srid_already_matches() {
    // Calling to_wkb with SridMode::Set(4326) on hex EWKB that already
    // has SRID=4326 should return the same WKB bytes (Cow::Borrowed fast path).
    let input_bytes = hex::decode(SRID_POINT_HEX).unwrap();
    let result = to_wkb(SRID_POINT_HEX, SridMode::Set(4326)).unwrap();
    assert_eq!(result, input_bytes);
    // Verify the round-trip still produces the correct WKT.
    assert_eq!(wkb_to_wkt(&result).unwrap(), "SRID=4326;POINT (1 2)");
}

// ── Fix 1: ISO dimensional types and collection types fall back to round-trip ──

#[test]
fn strip_iso_xyz_point_normalizes_to_ewkb() {
    // ISO WKB POINT Z uses type code 1001; the fast path must not pass it
    // through — it falls back to the round-trip which normalises to EWKB.
    const ISO_XYZ: &str = "01E9030000000000000000F03F00000000000000400000000000000840";
    let wkb = to_wkb(ISO_XYZ, SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT Z (1 2 3)");
    // EWKB POINT Z type word in LE is 0x80000001 → bytes [01, 00, 00, 80]
    assert_eq!(&wkb[1..5], &[0x01, 0x00, 0x00, 0x80]);
}

#[test]
fn set_iso_xyz_point_normalizes_to_ewkb_with_srid() {
    const ISO_XYZ: &str = "01E9030000000000000000F03F00000000000000400000000000000840";
    let wkb = to_wkb(ISO_XYZ, SridMode::Set(4326)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT Z (1 2 3)");
}

#[test]
fn strip_geometry_collection_fast_path_preserves_nested_srids() {
    const NESTED_SRID_GC: &str = concat!(
        "0107000020E6100000", // GC, SRID=4326
        "01000000",           // 1 sub-geometry
        "01",                 // sub-geom LE marker
        "01000020",           // POINT with SRID flag (malformed — sub-geometries
        "E6100000",           //   shouldn't carry SRIDs per EWKB spec)
        "000000000000F03F",   // x=1.0
        "0000000000000040",   // y=2.0
    );
    let wkb = to_wkb(NESTED_SRID_GC, SridMode::Strip).unwrap();
    // The fast path rewrites only the top-level EWKB header; the nested
    // sub-geometry's SRID flag is passed through unchanged.  The geometry
    // still round-trips correctly because the WKT serialiser tolerates the
    // (non-standard) nested SRID.
    assert_eq!(
        wkb_to_wkt(&wkb).unwrap(),
        "GEOMETRYCOLLECTION (POINT (1 2))"
    );
    assert_eq!(u32::from_le_bytes(wkb[1..5].try_into().unwrap()), 7);
    // Sub-geometry type word retains its SRID flag (0x20000001 = POINT|EWKB_SRID).
    assert_eq!(
        u32::from_le_bytes(wkb[10..14].try_into().unwrap()),
        0x2000_0001
    );
}

// ── Fix 2: Invalid SRID prefix errors under SridMode::Set (WKT path) ──────────

#[test]
fn set_srid_with_invalid_wkt_srid_prefix_errors() {
    // Old code silently accepted SRID=abc; under Set because it stripped the
    // prefix without validating it. The fix validates via wkt_to_wkb_split_srid.
    assert!(to_wkb("SRID=abc;POINT (1 2)", SridMode::Set(4326)).is_err());
    assert!(to_hex_wkb("SRID=abc;POINT (1 2)", SridMode::Set(4326)).is_err());
}
