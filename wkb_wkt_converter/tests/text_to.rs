/// Tests for the generic text_to_wkb / text_to_wkt / text_to_hex_wkb helpers.
use wkb_wkt_converter::{
    hex_wkb_to_wkt, text_to_hex_wkb, text_to_wkb, text_to_wkt, wkb_to_wkt, wkt_to_hex_wkb,
    wkt_to_wkb, SridMode,
};

// Big-endian WKB for POINT (1 2): byte-order=0x00, type=0x00000001 (BE),
// X=1.0 (BE f64), Y=2.0 (BE f64).
const BE_POINT_HEX: &str = "00000000013FF00000000000004000000000000000";

// Little-endian WKB with the EWKB SRID flag set in the type word but only 5
// bytes total (no room for the 4-byte SRID value).  Used to exercise the
// malformed-WKB fallback path.
const LE_SHORT_SRID_HEX: &str = "0103000020";

// Uppercase hex-encoded little-endian WKB for POINT (1 2) without SRID.
// byte-order=0x01, type=0x00000001 (LE), X=1.0 (LE f64), Y=2.0 (LE f64).
const POINT_HEX: &str = "0101000000000000000000F03F0000000000000040";

// Uppercase hex-encoded little-endian EWKB for SRID=4326;POINT (1 2).
// byte-order=0x01, type=0x20000001 (LE, SRID flag), SRID=4326 (LE u32),
// X=1.0 (LE f64), Y=2.0 (LE f64).
const SRID_POINT_HEX: &str = "0101000020E6100000000000000000F03F0000000000000040";

// ── helpers ──────────────────────────────────────────────────────────────────

fn hex(wkt: &str) -> String {
    wkt_to_hex_wkb(wkt).unwrap()
}

// ── text_to_wkb: input detection ─────────────────────────────────────────────

#[test]
fn wkb_from_wkt_matches_direct() {
    let via_text = text_to_wkb("POINT (1 2)", SridMode::Auto).unwrap();
    let direct = wkt_to_wkb("POINT (1 2)").unwrap();
    assert_eq!(via_text, direct);
}

#[test]
fn wkb_from_hex_wkb_matches_direct() {
    let h = hex("POINT (1 2)");
    let via_text = text_to_wkb(&h, SridMode::Auto).unwrap();
    let direct = wkt_to_wkb("POINT (1 2)").unwrap();
    assert_eq!(via_text, direct);
}

#[test]
fn wkb_from_wkt_with_leading_whitespace() {
    let via_text = text_to_wkb("  POINT (1 2)  ", SridMode::Auto).unwrap();
    let direct = wkt_to_wkb("POINT (1 2)").unwrap();
    assert_eq!(via_text, direct);
}

// ── text_to_wkb: SridMode::Auto ──────────────────────────────────────────────

#[test]
fn wkb_auto_preserves_srid_from_ewkt() {
    let wkb = text_to_wkb("SRID=4326;POINT (1 2)", SridMode::Auto).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn wkb_auto_preserves_no_srid_from_wkt() {
    let wkb = text_to_wkb("POINT (1 2)", SridMode::Auto).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkb_auto_preserves_srid_from_hex_ewkb() {
    let h = hex("SRID=4326;POINT (1 2)");
    let wkb = text_to_wkb(&h, SridMode::Auto).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn wkb_auto_preserves_no_srid_from_plain_hex_wkb() {
    let h = hex("POINT (1 2)");
    let wkb = text_to_wkb(&h, SridMode::Auto).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

// ── text_to_wkb: SridMode::Strip ─────────────────────────────────────────────

#[test]
fn wkb_strip_removes_srid_from_ewkt() {
    let wkb = text_to_wkb("SRID=4326;POINT (1 2)", SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkb_strip_removes_srid_from_hex_ewkb() {
    let h = hex("SRID=4326;POINT (1 2)");
    let wkb = text_to_wkb(&h, SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkb_strip_noop_when_no_srid_in_wkt() {
    let wkb = text_to_wkb("POINT (1 2)", SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

// ── text_to_wkb: SridMode::Set ───────────────────────────────────────────────

#[test]
fn wkb_set_adds_srid_to_plain_wkt() {
    let wkb = text_to_wkb("POINT (1 2)", SridMode::Set(4326)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn wkb_set_overrides_existing_srid_in_ewkt() {
    let wkb = text_to_wkb("SRID=4326;POINT (1 2)", SridMode::Set(3857)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=3857;POINT (1 2)");
}

#[test]
fn wkb_set_adds_srid_to_plain_hex_wkb() {
    let h = hex("POINT (1 2)");
    let wkb = text_to_wkb(&h, SridMode::Set(4326)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn wkb_set_overrides_srid_in_hex_ewkb() {
    let h = hex("SRID=4326;POINT (1 2)");
    let wkb = text_to_wkb(&h, SridMode::Set(3857)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=3857;POINT (1 2)");
}

// ── edge cases ───────────────────────────────────────────────────────────────

#[test]
fn empty_string_errors() {
    assert!(text_to_wkb("", SridMode::Auto).is_err());
    assert!(text_to_wkt("", SridMode::Auto, true).is_err());
    assert!(text_to_hex_wkb("", SridMode::Auto).is_err());
    assert!(text_to_wkt("", SridMode::Auto, false).is_err());
}

#[test]
fn decode_hex_lowercase_accepted() {
    // parse_hex_nibble must handle b'a'..=b'f'
    let lower = hex("POINT (1 2)").to_lowercase();
    let via_lower = text_to_wkb(&lower, SridMode::Auto).unwrap();
    assert_eq!(via_lower, wkt_to_wkb("POINT (1 2)").unwrap());
}

#[test]
fn decode_hex_invalid_low_nibble_errors() {
    // '0' is a valid high nibble; 'Z' is an invalid low nibble.
    // try_decode_hex("0Z") returns None (not valid hex), so text_to_wkb
    // would silently route to WKT parsing.  hex_wkb_to_wkt uses decode_hex
    // directly and therefore returns a descriptive error.
    assert!(hex_wkb_to_wkt("0Z").is_err());
}

#[test]
fn strip_ewkt_prefix_no_semicolon_returns_unchanged() {
    // "SRID=" present but no semicolon → strip_ewkt_prefix falls through unchanged
    assert_eq!(
        text_to_wkt("SRID=4326POINT (1 2)", SridMode::Strip, false).unwrap(),
        "SRID=4326POINT (1 2)"
    );
}

#[test]
fn odd_length_hex_string_errors() {
    // Odd-length all-hex input: try_decode_hex returns None (odd length) so
    // the input falls through to WKT parsing, which also rejects it.
    assert!(text_to_wkb("ABC", SridMode::Auto).is_err());
    assert!(text_to_wkt("ABC", SridMode::Auto, true).is_err());
    assert!(text_to_hex_wkb("ABC", SridMode::Auto).is_err());
}

#[test]
fn srid_set_zero_is_accepted() {
    let wkb = text_to_wkb("POINT (1 2)", SridMode::Set(0)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=0;POINT (1 2)");
}

// ── text_to_wkt: input detection and normalisation ───────────────────────────

#[test]
fn wkt_from_wkt_normalises_casing_and_spacing() {
    assert_eq!(
        text_to_wkt("point(1 2)", SridMode::Auto, true).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_from_hex_wkb_roundtrip() {
    let h = hex("LINESTRING (0 0, 1 1)");
    assert_eq!(
        text_to_wkt(&h, SridMode::Auto, true).unwrap(),
        "LINESTRING (0 0, 1 1)"
    );
}

// ── text_to_wkt: SridMode::Auto ──────────────────────────────────────────────

#[test]
fn wkt_auto_preserves_srid_from_ewkt() {
    assert_eq!(
        text_to_wkt("SRID=4326;POINT (1 2)", SridMode::Auto, true).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

#[test]
fn wkt_auto_preserves_no_srid_from_wkt() {
    assert_eq!(
        text_to_wkt("POINT (1 2)", SridMode::Auto, true).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_auto_preserves_srid_from_hex_ewkb() {
    let h = hex("SRID=4326;POINT (1 2)");
    assert_eq!(
        text_to_wkt(&h, SridMode::Auto, true).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

// ── text_to_wkt: SridMode::Strip ─────────────────────────────────────────────

#[test]
fn wkt_strip_removes_srid_from_ewkt() {
    assert_eq!(
        text_to_wkt("SRID=4326;POINT (1 2)", SridMode::Strip, true).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_strip_removes_srid_from_hex_ewkb() {
    let h = hex("SRID=4326;POINT (1 2)");
    assert_eq!(
        text_to_wkt(&h, SridMode::Strip, true).unwrap(),
        "POINT (1 2)"
    );
}

// ── text_to_wkt: SridMode::Set ───────────────────────────────────────────────

#[test]
fn wkt_set_adds_srid_to_plain_wkt() {
    assert_eq!(
        text_to_wkt("POINT (1 2)", SridMode::Set(4326), true).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

#[test]
fn wkt_set_overrides_existing_srid_in_ewkt() {
    assert_eq!(
        text_to_wkt("SRID=4326;POINT (1 2)", SridMode::Set(3857), true).unwrap(),
        "SRID=3857;POINT (1 2)"
    );
}

#[test]
fn wkt_set_adds_srid_to_plain_hex_wkb() {
    let h = hex("POINT (1 2)");
    assert_eq!(
        text_to_wkt(&h, SridMode::Set(4326), true).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

// ── text_to_wkt: normalize_wkt=false ─────────────────────────────────────────

#[test]
fn wkt_no_normalize_returns_wkt_as_is() {
    assert_eq!(
        text_to_wkt("point(1 2)", SridMode::Auto, false).unwrap(),
        "point(1 2)"
    );
}

#[test]
fn wkt_no_normalize_auto_preserves_srid_prefix() {
    assert_eq!(
        text_to_wkt("SRID=4326;POINT (1 2)", SridMode::Auto, false).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_strip_removes_srid_prefix() {
    assert_eq!(
        text_to_wkt("SRID=4326;POINT (1 2)", SridMode::Strip, false).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_strip_noop_when_no_srid() {
    assert_eq!(
        text_to_wkt("POINT (1 2)", SridMode::Strip, false).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_set_adds_srid_prefix() {
    assert_eq!(
        text_to_wkt("POINT (1 2)", SridMode::Set(4326), false).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_set_overrides_existing_srid() {
    assert_eq!(
        text_to_wkt("SRID=4326;POINT (1 2)", SridMode::Set(3857), false).unwrap(),
        "SRID=3857;POINT (1 2)"
    );
}

#[test]
fn wkt_no_normalize_hex_input_still_normalises() {
    // normalize_wkt has no effect on hex WKB input; result is always normalised WKT.
    let h = hex("POINT (1 2)");
    assert_eq!(
        text_to_wkt(&h, SridMode::Auto, false).unwrap(),
        "POINT (1 2)"
    );
}

// ── text_to_hex_wkb ──────────────────────────────────────────────────────────

#[test]
fn hex_wkb_from_wkt_is_uppercase() {
    let h = text_to_hex_wkb("POINT (1 2)", SridMode::Auto).unwrap();
    assert_eq!(h, h.to_uppercase());
}

#[test]
fn hex_wkb_from_wkt_roundtrip() {
    let h = text_to_hex_wkb("POINT (1 2)", SridMode::Auto).unwrap();
    assert_eq!(
        wkb_to_wkt(&hex::decode(&h).unwrap()).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn hex_wkb_from_hex_wkb_roundtrip() {
    let original = hex("LINESTRING (0 0, 1 1)");
    let result = text_to_hex_wkb(&original, SridMode::Auto).unwrap();
    assert_eq!(result, original);
}

#[test]
fn hex_wkb_set_adds_srid() {
    let h = text_to_hex_wkb("POINT (1 2)", SridMode::Set(4326)).unwrap();
    let wkt = wkb_to_wkt(&hex::decode(&h).unwrap()).unwrap();
    assert_eq!(wkt, "SRID=4326;POINT (1 2)");
}

#[test]
fn hex_wkb_strip_removes_srid() {
    let h = text_to_hex_wkb("SRID=4326;POINT (1 2)", SridMode::Strip).unwrap();
    let wkt = wkb_to_wkt(&hex::decode(&h).unwrap()).unwrap();
    assert_eq!(wkt, "POINT (1 2)");
}

// ── try_strip_srid_from_le_wkb / try_set_srid_in_le_wkb coverage ─────────────

#[test]
fn wkb_strip_noop_when_no_srid_in_hex_wkb() {
    // hex WKB without SRID flag: fast binary path returns bytes unchanged.
    let h = hex("POINT (1 2)");
    let wkb = text_to_wkb(&h, SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkb_strip_big_endian_hex_falls_back_to_round_trip() {
    // Big-endian WKB: try_strip_srid_from_le_wkb returns None, so text_to_wkb
    // falls back to the full WKB→WKT→WKB round-trip which normalises to LE.
    let wkb = text_to_wkb(BE_POINT_HEX, SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkb_strip_malformed_short_srid_errors() {
    // LE WKB with SRID flag but only 5 bytes (no room for 4-byte SRID value):
    // try_strip_srid_from_le_wkb returns None, fallback round-trip fails.
    assert!(text_to_wkb(LE_SHORT_SRID_HEX, SridMode::Strip).is_err());
}

#[test]
fn wkb_set_big_endian_hex_falls_back_to_round_trip() {
    // Big-endian WKB: try_set_srid_in_le_wkb returns None, fallback works.
    let wkb = text_to_wkb(BE_POINT_HEX, SridMode::Set(4326)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn wkb_set_malformed_short_srid_errors() {
    // LE WKB with SRID flag but too short: try_set_srid_in_le_wkb returns None,
    // fallback round-trip fails.
    assert!(text_to_wkb(LE_SHORT_SRID_HEX, SridMode::Set(4326)).is_err());
}

// ── text_to_wkt: normalize_wkt=false with hex-digit-starting non-hex input ───

#[test]
fn wkt_no_normalize_hex_start_non_hex_body_returned_as_is() {
    // Input begins with a hex digit ('0') but is not valid hex (contains 'X').
    // The fast O(1) first-char check does NOT apply (first byte IS a hex digit),
    // so try_decode_hex is attempted, fails, and the WKT is returned as-is.
    assert_eq!(
        text_to_wkt("0XTEST", SridMode::Auto, false).unwrap(),
        "0XTEST"
    );
    assert_eq!(
        text_to_wkt("0XTEST", SridMode::Strip, false).unwrap(),
        "0XTEST"
    );
    assert_eq!(
        text_to_wkt("0XTEST", SridMode::Set(4326), false).unwrap(),
        "SRID=4326;0XTEST"
    );
}

// ── new tests for Change 2–5 ──────────────────────────────────────────────────

#[test]
fn decode_hex_empty_string_errors() {
    assert!(hex_wkb_to_wkt("").is_err());
}

#[test]
fn auto_be_hex_input_returned_as_be_bytes() {
    // Big-endian WKB passes through SridMode::Auto without normalisation.
    // The first byte of the result must be 0x00 (big-endian byte-order marker).
    let result = text_to_wkb(BE_POINT_HEX, SridMode::Auto).unwrap();
    assert_eq!(result[0], 0x00, "expected big-endian marker byte 0x00");
}

#[test]
fn text_to_hex_wkb_lowercase_hex_normalised_to_uppercase() {
    // Lowercase hex input under SridMode::Auto is decoded and re-encoded as
    // uppercase — the output must equal the known uppercase constant.
    let lowercase = POINT_HEX.to_lowercase();
    let result = text_to_hex_wkb(&lowercase, SridMode::Auto).unwrap();
    assert_eq!(result, POINT_HEX);
}

#[test]
fn set_srid_noop_when_srid_already_matches() {
    // Calling text_to_wkb with SridMode::Set(4326) on hex EWKB that already
    // has SRID=4326 should return the same WKB bytes (Cow::Borrowed fast path).
    let input_bytes = hex::decode(SRID_POINT_HEX).unwrap();
    let result = text_to_wkb(SRID_POINT_HEX, SridMode::Set(4326)).unwrap();
    assert_eq!(result, input_bytes);
    // Verify the round-trip still produces the correct WKT.
    assert_eq!(wkb_to_wkt(&result).unwrap(), "SRID=4326;POINT (1 2)");
}
