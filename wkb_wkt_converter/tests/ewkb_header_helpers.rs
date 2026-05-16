/// Tests for wkb_header_srid / wkb_strip_srid / wkb_set_srid / writers.
use wkb_wkt_converter::{
    wkb_header_srid, wkb_set_srid, wkb_set_srid_writer, wkb_strip_srid, wkb_strip_srid_writer,
    wkb_to_wkt, wkt_to_wkb, SridMode, WkbWriter,
};

// ── shared test helpers ───────────────────────────────────────────────────────

fn wkb(wkt: &str) -> Vec<u8> {
    wkt_to_wkb(wkt, SridMode::Auto).unwrap()
}

// Decode a hex string to bytes for constructing hand-crafted WKB fixtures.
fn from_hex(h: &str) -> Vec<u8> {
    (0..h.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
        .collect()
}

fn wkt_of(bytes: &[u8]) -> String {
    wkb_to_wkt(bytes, SridMode::Auto).unwrap()
}

// Big-endian EWKB for SRID=4326;POINT (1 2):
//   0x00              byte-order = BE
//   0x20000001        type = POINT | EWKB_SRID (BE)
//   0x000010E6        SRID = 4326 (BE i32)
//   0x3FF0…  X = 1.0 (BE f64)
//   0x4000…  Y = 2.0 (BE f64)
const BE_SRID_POINT_HEX: &str = "0020000001000010E63FF00000000000004000000000000000";
const BE_SRID3_POINT_HEX: &str = "0020000001000000033FF00000000000004000000000000000";
const BE_SRID4_POINT_HEX: &str = "0020000001000000043FF00000000000004000000000000000";

// Big-endian WKB for POINT (1 2) — no SRID.
const BE_POINT_HEX: &str = "00000000013FF00000000000004000000000000000";
const LE_SRID_ZERO_POINT_HEX: &str = "010100002000000000000000000000F03F0000000000000040";
const LE_SRID_MINUS_ONE_POINT_HEX: &str = "0101000020FFFFFFFF000000000000F03F0000000000000040";

fn make_iso_point_z() -> Vec<u8> {
    let mut wkb = vec![0x01u8]; // LE
    wkb.extend_from_slice(&1001u32.to_le_bytes()); // POINT Z
    wkb.extend_from_slice(&1.0f64.to_le_bytes());
    wkb.extend_from_slice(&2.0f64.to_le_bytes());
    wkb.extend_from_slice(&3.0f64.to_le_bytes());
    wkb
}

// ── wkb_header_srid ───────────────────────────────────────────────────────────

#[test]
fn header_srid_le_ewkb_with_srid() {
    let bytes = wkb("SRID=4326;POINT (1 2)");
    assert_eq!(wkb_header_srid(&bytes).unwrap(), Some(4326));
}

#[test]
fn header_srid_le_ewkb_without_srid() {
    let bytes = wkb("POINT (1 2)");
    assert_eq!(wkb_header_srid(&bytes).unwrap(), None);
}

#[test]
fn header_srid_le_ewkb_z_with_srid() {
    let bytes = wkb("SRID=4326;POINT Z (1 2 3)");
    assert_eq!(wkb_header_srid(&bytes).unwrap(), Some(4326));
}

#[test]
fn header_srid_le_ewkb_linestring_with_srid() {
    let bytes = wkb("SRID=4326;LINESTRING (0 0, 1 1)");
    assert_eq!(wkb_header_srid(&bytes).unwrap(), Some(4326));
}

#[test]
fn header_srid_le_ewkb_multipolygon_with_srid() {
    let bytes = wkb("SRID=4326;MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))");
    assert_eq!(wkb_header_srid(&bytes).unwrap(), Some(4326));
}

#[test]
fn header_srid_le_ewkb_geometry_collection_with_srid() {
    let bytes = wkb("SRID=4326;GEOMETRYCOLLECTION (POINT (1 2))");
    assert_eq!(wkb_header_srid(&bytes).unwrap(), Some(4326));
}

#[test]
fn header_srid_big_endian_with_srid_fast_path() {
    let bytes = from_hex(BE_SRID_POINT_HEX);
    // big-endian EWKB is handled by the fast path (types 1–7, known flags)
    assert_eq!(wkb_header_srid(&bytes).unwrap(), Some(4326));
}

#[test]
fn header_srid_big_endian_without_srid() {
    let bytes = from_hex(BE_POINT_HEX);
    assert_eq!(wkb_header_srid(&bytes).unwrap(), None);
}

#[test]
fn header_srid_iso_point_z_returns_none() {
    // ISO WKB has no SRID flag set (high bits are all zero), so the fast
    // path returns None immediately without a full parse.
    let bytes = make_iso_point_z();
    assert_eq!(wkb_header_srid(&bytes).unwrap(), None);
}

#[test]
fn header_srid_negative_srid_in_wkt_input_is_normalized_away() {
    let bytes = wkb("SRID=-1;POINT (1 2)");
    // SRID=-1 is normalised to "no SRID" by wkt_to_wkb, so the embedded
    // SRID flag is cleared — we get None.
    assert_eq!(wkb_header_srid(&bytes).unwrap(), None);
}

#[test]
fn header_srid_raw_zero_is_treated_as_unknown() {
    let bytes = from_hex(LE_SRID_ZERO_POINT_HEX);
    assert_eq!(wkb_header_srid(&bytes).unwrap(), None);
}

#[test]
fn header_srid_raw_negative_one_is_treated_as_unknown() {
    let bytes = from_hex(LE_SRID_MINUS_ONE_POINT_HEX);
    assert_eq!(wkb_header_srid(&bytes).unwrap(), None);
}

#[test]
fn header_srid_truncated_srid_field_errors() {
    // 5-byte slice that has the SRID flag set but no room for the SRID value.
    // Type word 0x20000003 = Polygon | EWKB_SRID in LE.
    let bytes = from_hex("0103000020");
    assert!(wkb_header_srid(&bytes).is_err());
}

#[test]
fn header_srid_empty_bytes_errors() {
    assert!(wkb_header_srid(&[]).is_err());
}

#[test]
fn header_srid_invalid_byte_order_errors() {
    assert!(wkb_header_srid(&[0x99]).is_err());
}

#[test]
fn header_srid_invalid_byte_order_full_header_falls_back_and_errors() {
    assert!(wkb_header_srid(&[0x99, 0x01, 0x00, 0x00, 0x00]).is_err());
}

#[test]
fn header_srid_unknown_high_bits_falls_back_and_preserves_result() {
    let mut bytes = vec![0x01u8];
    bytes.extend_from_slice(&0x1000_0001u32.to_le_bytes());
    bytes.extend_from_slice(&1.0f64.to_le_bytes());
    bytes.extend_from_slice(&2.0f64.to_le_bytes());

    assert_eq!(wkb_header_srid(&bytes).unwrap(), None);
}

// ── wkb_strip_srid ────────────────────────────────────────────────────────────

#[test]
fn strip_srid_le_ewkb_removes_srid() {
    let bytes = wkb("SRID=4326;POINT (1 2)");
    let stripped = wkb_strip_srid(&bytes).unwrap();
    assert_eq!(wkt_of(&stripped), "POINT (1 2)");
}

#[test]
fn strip_srid_le_ewkb_no_srid_unchanged() {
    let bytes = wkb("POINT (1 2)");
    let stripped = wkb_strip_srid(&bytes).unwrap();
    assert_eq!(wkt_of(&stripped), "POINT (1 2)");
}

#[test]
fn strip_srid_le_ewkb_linestring() {
    let bytes = wkb("SRID=4326;LINESTRING (0 0, 1 1)");
    let stripped = wkb_strip_srid(&bytes).unwrap();
    assert_eq!(wkt_of(&stripped), "LINESTRING (0 0, 1 1)");
}

#[test]
fn strip_srid_big_endian_preserves_header_and_payload() {
    let bytes = from_hex(BE_SRID_POINT_HEX);
    let stripped = wkb_strip_srid(&bytes).unwrap();
    assert_eq!(stripped, from_hex(BE_POINT_HEX));
}

#[test]
fn strip_srid_multipolygon_falls_back_and_preserves_geometry() {
    let bytes = wkb("SRID=4326;MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))");
    let stripped = wkb_strip_srid(&bytes).unwrap();
    assert_eq!(wkt_of(&stripped), "MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))");
}

#[test]
fn strip_srid_geometry_collection_with_nested_geometries() {
    let bytes = wkb("SRID=4326;GEOMETRYCOLLECTION (POINT (1 2), LINESTRING (0 0, 1 1))");
    let stripped = wkb_strip_srid(&bytes).unwrap();
    assert_eq!(
        wkt_of(&stripped),
        "GEOMETRYCOLLECTION (POINT (1 2), LINESTRING (0 0, 1 1))"
    );
}

#[test]
fn strip_srid_iso_point_z_normalises_no_srid() {
    let bytes = make_iso_point_z();
    let stripped = wkb_strip_srid(&bytes).unwrap();
    assert_eq!(wkt_of(&stripped), "POINT Z (1 2 3)");
}

#[test]
fn strip_srid_empty_bytes_errors() {
    assert!(wkb_strip_srid(&[]).is_err());
}

// ── wkb_set_srid ─────────────────────────────────────────────────────────────

#[test]
fn set_srid_adds_srid_to_plain_wkb() {
    let bytes = wkb("POINT (1 2)");
    let ewkb = wkb_set_srid(&bytes, 4326).unwrap();
    assert_eq!(wkt_of(&ewkb), "SRID=4326;POINT (1 2)");
}

#[test]
fn set_srid_replaces_existing_srid() {
    let bytes = wkb("SRID=4326;POINT (1 2)");
    let ewkb = wkb_set_srid(&bytes, 3857).unwrap();
    assert_eq!(wkt_of(&ewkb), "SRID=3857;POINT (1 2)");
}

#[test]
fn set_srid_noop_when_srid_already_matches() {
    let bytes = wkb("SRID=4326;POINT (1 2)");
    let ewkb = wkb_set_srid(&bytes, 4326).unwrap();
    assert_eq!(ewkb, bytes);
}

#[test]
fn set_srid_zero_strips_srid() {
    let bytes = wkb("SRID=4326;POINT (1 2)");
    let ewkb = wkb_set_srid(&bytes, 0).unwrap();
    assert_eq!(wkt_of(&ewkb), "POINT (1 2)");
}

#[test]
fn set_srid_negative_strips_srid() {
    let bytes = wkb("SRID=4326;POINT (1 2)");
    let ewkb = wkb_set_srid(&bytes, -1).unwrap();
    assert_eq!(wkt_of(&ewkb), "POINT (1 2)");
}

#[test]
fn set_srid_linestring() {
    let bytes = wkb("LINESTRING (0 0, 1 1)");
    let ewkb = wkb_set_srid(&bytes, 4326).unwrap();
    assert_eq!(wkt_of(&ewkb), "SRID=4326;LINESTRING (0 0, 1 1)");
}

#[test]
fn set_srid_big_endian_adds_srid_preserving_header_and_payload() {
    let bytes = from_hex(BE_POINT_HEX);
    let ewkb = wkb_set_srid(&bytes, 4326).unwrap();
    assert_eq!(ewkb, from_hex(BE_SRID_POINT_HEX));
}

#[test]
fn set_srid_big_endian_replaces_srid_preserving_payload() {
    let bytes = from_hex(BE_SRID3_POINT_HEX);
    let ewkb = wkb_set_srid(&bytes, 4).unwrap();
    assert_eq!(ewkb, from_hex(BE_SRID4_POINT_HEX));
}

#[test]
fn set_srid_multipolygon_falls_back_and_normalises_round_trip() {
    let bytes = wkb("MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))");
    let ewkb = wkb_set_srid(&bytes, 4326).unwrap();
    assert_eq!(
        wkt_of(&ewkb),
        "SRID=4326;MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))"
    );
}

#[test]
fn set_srid_iso_point_z_normalises_to_ewkb() {
    let bytes = make_iso_point_z();
    let ewkb = wkb_set_srid(&bytes, 4326).unwrap();
    assert_eq!(wkt_of(&ewkb), "SRID=4326;POINT Z (1 2 3)");
}

#[test]
fn set_srid_empty_bytes_errors() {
    assert!(wkb_set_srid(&[], 4326).is_err());
}

// ── round-trip consistency ────────────────────────────────────────────────────

#[test]
fn strip_then_set_round_trips_srid() {
    let original = wkb("SRID=4326;POINT (1 2)");
    let stripped = wkb_strip_srid(&original).unwrap();
    let restored = wkb_set_srid(&stripped, 4326).unwrap();
    assert_eq!(wkt_of(&restored), "SRID=4326;POINT (1 2)");
}

#[test]
fn header_srid_consistent_with_strip() {
    let bytes = wkb("SRID=4326;MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))");
    assert_eq!(wkb_header_srid(&bytes).unwrap(), Some(4326));
    let stripped = wkb_strip_srid(&bytes).unwrap();
    assert_eq!(wkb_header_srid(&stripped).unwrap(), None);
}

// ── wkb_strip_srid_writer ────────────────────────────────────────────────────

fn apply_writer(len: usize, writer: WkbWriter<'_>) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    writer.write_into(&mut buf);
    buf
}

#[test]
fn strip_writer_has_srid_fast_path() {
    let bytes = wkb("SRID=4326;POINT (1 2)");
    let (len, writer) = wkb_strip_srid_writer(&bytes).unwrap();
    let out = apply_writer(len, writer);
    assert_eq!(wkt_of(&out), "POINT (1 2)");
}

#[test]
fn strip_writer_no_srid_fast_path() {
    let bytes = wkb("POINT (1 2)");
    let (len, writer) = wkb_strip_srid_writer(&bytes).unwrap();
    let out = apply_writer(len, writer);
    assert_eq!(out, bytes);
}

#[test]
fn strip_writer_big_endian_preserves_header_and_payload() {
    let bytes = from_hex(BE_SRID_POINT_HEX);
    let (len, writer) = wkb_strip_srid_writer(&bytes).unwrap();
    let out = apply_writer(len, writer);
    assert_eq!(out, from_hex(BE_POINT_HEX));
}

#[test]
fn strip_writer_multipolygon_fallback() {
    let bytes = wkb("SRID=4326;MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))");
    let (len, writer) = wkb_strip_srid_writer(&bytes).unwrap();
    let out = apply_writer(len, writer);
    assert_eq!(wkt_of(&out), "MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))");
}

#[test]
fn strip_writer_empty_errors() {
    assert!(wkb_strip_srid_writer(&[]).is_err());
}

// ── wkb_set_srid_writer ──────────────────────────────────────────────────────

#[test]
fn set_writer_adds_srid_fast_path() {
    let bytes = wkb("POINT (1 2)");
    let (len, writer) = wkb_set_srid_writer(&bytes, 4326).unwrap();
    let out = apply_writer(len, writer);
    assert_eq!(wkt_of(&out), "SRID=4326;POINT (1 2)");
}

#[test]
fn set_writer_replaces_srid_fast_path() {
    let bytes = wkb("SRID=4326;POINT (1 2)");
    let (len, writer) = wkb_set_srid_writer(&bytes, 3857).unwrap();
    let out = apply_writer(len, writer);
    assert_eq!(wkt_of(&out), "SRID=3857;POINT (1 2)");
}

#[test]
fn set_writer_big_endian_adds_srid_preserving_header_and_payload() {
    let bytes = from_hex(BE_POINT_HEX);
    let (len, writer) = wkb_set_srid_writer(&bytes, 4326).unwrap();
    let out = apply_writer(len, writer);
    assert_eq!(out, from_hex(BE_SRID_POINT_HEX));
}

#[test]
fn set_writer_big_endian_replaces_srid_preserving_payload() {
    let bytes = from_hex(BE_SRID3_POINT_HEX);
    let (len, writer) = wkb_set_srid_writer(&bytes, 4).unwrap();
    let out = apply_writer(len, writer);
    assert_eq!(out, from_hex(BE_SRID4_POINT_HEX));
}

#[test]
fn set_writer_noop_when_srid_matches() {
    let bytes = wkb("SRID=4326;POINT (1 2)");
    let (len, writer) = wkb_set_srid_writer(&bytes, 4326).unwrap();
    let out = apply_writer(len, writer);
    assert_eq!(out, bytes);
}

#[test]
fn set_writer_negative_srid_strips() {
    let bytes = wkb("SRID=4326;POINT (1 2)");
    let (len, writer) = wkb_set_srid_writer(&bytes, -1).unwrap();
    let out = apply_writer(len, writer);
    assert_eq!(wkt_of(&out), "POINT (1 2)");
}

#[test]
fn set_writer_multipolygon_fallback() {
    let bytes = wkb("MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))");
    let (len, writer) = wkb_set_srid_writer(&bytes, 4326).unwrap();
    let out = apply_writer(len, writer);
    assert_eq!(
        wkt_of(&out),
        "SRID=4326;MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))"
    );
}

#[test]
fn set_writer_empty_errors() {
    assert!(wkb_set_srid_writer(&[], 4326).is_err());
}
