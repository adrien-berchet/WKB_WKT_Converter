/// Tests for the generic text_to_wkb / text_to_wkt / text_to_hex_wkb helpers.
use wkb_wkt_converter::{
    text_to_hex_wkb, text_to_wkb, text_to_wkt, wkb_to_wkt, wkt_to_hex_wkb, wkt_to_wkb, SridMode,
};

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
