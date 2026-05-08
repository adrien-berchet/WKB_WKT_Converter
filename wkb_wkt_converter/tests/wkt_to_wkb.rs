/// WKT → WKB tests. Where possible we verify the exact bytes against known
/// PostGIS output. All tests also include a round-trip assertion.
use wkb_wkt_converter::{
    wkb_to_wkt as core_wkb_to_wkt, wkt_to_hex_wkb as core_wkt_to_hex_wkb,
    wkt_to_wkb as core_wkt_to_wkb, wkt_to_wkb_split_srid, Result, SridMode,
};

fn wkb_to_wkt(wkb: &[u8]) -> Result<String> {
    core_wkb_to_wkt(wkb, SridMode::Auto)
}

fn wkt_to_wkb(wkt: &str) -> Result<Vec<u8>> {
    core_wkt_to_wkb(wkt, SridMode::Auto)
}

fn wkt_to_hex_wkb(wkt: &str) -> Result<String> {
    core_wkt_to_hex_wkb(wkt, SridMode::Auto)
}

fn roundtrip(wkt: &str) -> String {
    let wkb = wkt_to_wkb(wkt).unwrap();
    wkb_to_wkt(&wkb).unwrap()
}

// ── Point ────────────────────────────────────────────────────────────────────

#[test]
fn point_xy_roundtrip() {
    assert_eq!(roundtrip("POINT (1 2)"), "POINT (1 2)");
}

#[test]
fn point_xyz_roundtrip() {
    assert_eq!(roundtrip("POINT Z (1 2 3)"), "POINT Z (1 2 3)");
}

#[test]
fn point_xym_roundtrip() {
    assert_eq!(roundtrip("POINT M (1 2 3)"), "POINT M (1 2 3)");
}

#[test]
fn point_xyzm_roundtrip() {
    assert_eq!(roundtrip("POINT ZM (1 2 3 4)"), "POINT ZM (1 2 3 4)");
}

#[test]
fn point_empty_roundtrip() {
    assert_eq!(roundtrip("POINT EMPTY"), "POINT EMPTY");
}

#[test]
fn point_empty_z_roundtrip() {
    assert_eq!(roundtrip("POINT Z EMPTY"), "POINT Z EMPTY");
}

#[test]
fn point_negative_coords() {
    assert_eq!(roundtrip("POINT (-1.5 -2.5)"), "POINT (-1.5 -2.5)");
}

#[test]
fn point_scientific_notation() {
    // 1e-4 should survive the round-trip
    let wkb = wkt_to_wkb("POINT (1e-4 2e-4)").unwrap();
    let wkt = wkb_to_wkt(&wkb).unwrap();
    assert!(wkt.starts_with("POINT ("));
    // Verify the values are preserved numerically
    let wkb2 = wkt_to_wkb(&wkt).unwrap();
    assert_eq!(wkb, wkb2);
}

#[test]
fn point_with_srid_embedded() {
    // EWKT: SRID embedded in WKB output, re-read as EWKT
    let wkb = wkt_to_wkb("SRID=4326;POINT (1 2)").unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn point_split_srid() {
    let (wkb, srid) = wkt_to_wkb_split_srid("SRID=4326;POINT (1 2)").unwrap();
    assert_eq!(srid, Some(4326));
    // The returned bytes must NOT have the SRID flag
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn point_no_srid_split() {
    let (wkb, srid) = wkt_to_wkb_split_srid("POINT (1 2)").unwrap();
    assert_eq!(srid, None);
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkt_to_wkb_auto_preserves_srid() {
    let wkb = core_wkt_to_wkb("SRID=4326;POINT (1 2)", SridMode::Auto).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn wkt_to_wkb_strip_removes_srid() {
    let wkb = core_wkt_to_wkb("SRID=4326;POINT (1 2)", SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkt_to_wkb_strip_without_srid_is_noop() {
    let wkb = core_wkt_to_wkb("POINT (1 2)", SridMode::Strip).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "POINT (1 2)");
}

#[test]
fn wkt_to_wkb_set_adds_srid() {
    let wkb = core_wkt_to_wkb("POINT (1 2)", SridMode::Set(4326)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=4326;POINT (1 2)");
}

#[test]
fn wkt_to_wkb_set_overrides_srid() {
    let wkb = core_wkt_to_wkb("SRID=4326;POINT (1 2)", SridMode::Set(3857)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=3857;POINT (1 2)");
}

#[test]
fn wkt_to_wkb_set_zero_is_accepted() {
    let wkb = core_wkt_to_wkb("POINT (1 2)", SridMode::Set(0)).unwrap();
    assert_eq!(wkb_to_wkt(&wkb).unwrap(), "SRID=0;POINT (1 2)");
}

#[test]
fn wkt_to_hex_wkb_auto_preserves_srid() {
    let hex = core_wkt_to_hex_wkb("SRID=4326;POINT (1 2)", SridMode::Auto).unwrap();
    assert_eq!(
        wkb_to_wkt(&hex::decode(hex).unwrap()).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

#[test]
fn wkt_to_hex_wkb_strip_removes_srid() {
    let hex = core_wkt_to_hex_wkb("SRID=4326;POINT (1 2)", SridMode::Strip).unwrap();
    assert_eq!(
        wkb_to_wkt(&hex::decode(hex).unwrap()).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_to_hex_wkb_strip_without_srid_is_noop() {
    let hex = core_wkt_to_hex_wkb("POINT (1 2)", SridMode::Strip).unwrap();
    assert_eq!(
        wkb_to_wkt(&hex::decode(hex).unwrap()).unwrap(),
        "POINT (1 2)"
    );
}

#[test]
fn wkt_to_hex_wkb_set_adds_srid() {
    let hex = core_wkt_to_hex_wkb("POINT (1 2)", SridMode::Set(4326)).unwrap();
    assert_eq!(
        wkb_to_wkt(&hex::decode(hex).unwrap()).unwrap(),
        "SRID=4326;POINT (1 2)"
    );
}

#[test]
fn wkt_to_hex_wkb_set_overrides_srid() {
    let hex = core_wkt_to_hex_wkb("SRID=4326;POINT (1 2)", SridMode::Set(3857)).unwrap();
    assert_eq!(
        wkb_to_wkt(&hex::decode(hex).unwrap()).unwrap(),
        "SRID=3857;POINT (1 2)"
    );
}

#[test]
fn wkt_to_hex_wkb_set_zero_is_accepted() {
    let hex = core_wkt_to_hex_wkb("POINT (1 2)", SridMode::Set(0)).unwrap();
    assert_eq!(
        wkb_to_wkt(&hex::decode(hex).unwrap()).unwrap(),
        "SRID=0;POINT (1 2)"
    );
}

// ── LineString ───────────────────────────────────────────────────────────────

#[test]
fn linestring_xy_roundtrip() {
    assert_eq!(
        roundtrip("LINESTRING (0 0, 1 1, 2 2)"),
        "LINESTRING (0 0, 1 1, 2 2)"
    );
}

#[test]
fn linestring_xyz_roundtrip() {
    assert_eq!(
        roundtrip("LINESTRING Z (0 0 0, 1 1 1)"),
        "LINESTRING Z (0 0 0, 1 1 1)"
    );
}

#[test]
fn linestring_xyzm_roundtrip() {
    assert_eq!(
        roundtrip("LINESTRING ZM (0 0 0 1, 1 1 1 2)"),
        "LINESTRING ZM (0 0 0 1, 1 1 1 2)"
    );
}

#[test]
fn linestring_empty_roundtrip() {
    assert_eq!(roundtrip("LINESTRING EMPTY"), "LINESTRING EMPTY");
}

// ── Polygon ──────────────────────────────────────────────────────────────────

#[test]
fn polygon_one_ring_roundtrip() {
    assert_eq!(
        roundtrip("POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))"),
        "POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))"
    );
}

#[test]
fn polygon_with_hole_roundtrip() {
    let wkt = "POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0), (1 1, 1 9, 9 9, 9 1, 1 1))";
    assert_eq!(roundtrip(wkt), wkt);
}

#[test]
fn polygon_xyz_roundtrip() {
    let wkt = "POLYGON Z ((0 0 0, 1 0 0, 1 1 0, 0 0 0))";
    assert_eq!(roundtrip(wkt), wkt);
}

#[test]
fn polygon_empty_roundtrip() {
    assert_eq!(roundtrip("POLYGON EMPTY"), "POLYGON EMPTY");
}

// ── MultiPoint ───────────────────────────────────────────────────────────────

#[test]
fn multipoint_iso_form_roundtrip() {
    // ISO form: each point has parens
    assert_eq!(
        roundtrip("MULTIPOINT ((0 0), (1 1), (2 2))"),
        "MULTIPOINT ((0 0), (1 1), (2 2))"
    );
}

#[test]
fn multipoint_bare_form_roundtrip() {
    // Bare form (no inner parens) — should also parse, output is ISO form
    let wkb = wkt_to_wkb("MULTIPOINT (0 0, 1 1)").unwrap();
    let wkt = wkb_to_wkt(&wkb).unwrap();
    assert_eq!(wkt, "MULTIPOINT ((0 0), (1 1))");
}

#[test]
fn multipoint_xyz_roundtrip() {
    assert_eq!(
        roundtrip("MULTIPOINT Z ((0 0 1), (1 1 2))"),
        "MULTIPOINT Z ((0 0 1), (1 1 2))"
    );
}

#[test]
fn multipoint_empty_roundtrip() {
    assert_eq!(roundtrip("MULTIPOINT EMPTY"), "MULTIPOINT EMPTY");
}

#[test]
fn multipoint_empty_members_roundtrip() {
    assert_eq!(roundtrip("MULTIPOINT (EMPTY)"), "MULTIPOINT (EMPTY)");
    assert_eq!(
        roundtrip("MULTIPOINT (EMPTY, (1 2), EMPTY)"),
        "MULTIPOINT (EMPTY, (1 2), EMPTY)"
    );
    assert_eq!(
        roundtrip("MULTIPOINT Z (EMPTY, (1 2 3), EMPTY)"),
        "MULTIPOINT Z (EMPTY, (1 2 3), EMPTY)"
    );
}

// ── MultiLineString ───────────────────────────────────────────────────────────

#[test]
fn multilinestring_xy_roundtrip() {
    assert_eq!(
        roundtrip("MULTILINESTRING ((0 0, 1 1), (2 2, 3 3))"),
        "MULTILINESTRING ((0 0, 1 1), (2 2, 3 3))"
    );
}

#[test]
fn multilinestring_z_roundtrip() {
    assert_eq!(
        roundtrip("MULTILINESTRING Z ((0 0 1, 1 1 2), (2 2 3, 3 3 4))"),
        "MULTILINESTRING Z ((0 0 1, 1 1 2), (2 2 3, 3 3 4))"
    );
}

#[test]
fn multilinestring_empty_roundtrip() {
    assert_eq!(roundtrip("MULTILINESTRING EMPTY"), "MULTILINESTRING EMPTY");
}

#[test]
fn multilinestring_empty_members_roundtrip() {
    assert_eq!(
        roundtrip("MULTILINESTRING (EMPTY, (0 0, 1 1), EMPTY)"),
        "MULTILINESTRING (EMPTY, (0 0, 1 1), EMPTY)"
    );
    assert_eq!(
        roundtrip("MULTILINESTRING Z (EMPTY, (0 0 0, 1 1 1))"),
        "MULTILINESTRING Z (EMPTY, (0 0 0, 1 1 1))"
    );
}

// ── MultiPolygon ─────────────────────────────────────────────────────────────

#[test]
fn multipolygon_xy_roundtrip() {
    let wkt = "MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)), ((2 2, 3 2, 3 3, 2 2)))";
    assert_eq!(roundtrip(wkt), wkt);
}

#[test]
fn multipolygon_with_hole_roundtrip() {
    let wkt = "MULTIPOLYGON (((0 0, 10 0, 10 10, 0 0), (1 1, 1 9, 9 1, 1 1)), ((20 20, 30 20, 30 30, 20 20)))";
    assert_eq!(roundtrip(wkt), wkt);
}

#[test]
fn multipolygon_empty_roundtrip() {
    assert_eq!(roundtrip("MULTIPOLYGON EMPTY"), "MULTIPOLYGON EMPTY");
}

#[test]
fn multipolygon_empty_members_roundtrip() {
    assert_eq!(
        roundtrip("MULTIPOLYGON (EMPTY, ((0 0, 1 0, 1 1, 0 0)), EMPTY)"),
        "MULTIPOLYGON (EMPTY, ((0 0, 1 0, 1 1, 0 0)), EMPTY)"
    );
    assert_eq!(
        roundtrip("MULTIPOLYGON Z (EMPTY, ((0 0 0, 1 0 0, 1 1 0, 0 0 0)))"),
        "MULTIPOLYGON Z (EMPTY, ((0 0 0, 1 0 0, 1 1 0, 0 0 0)))"
    );
}

// ── GeometryCollection ───────────────────────────────────────────────────────

#[test]
fn geometrycollection_roundtrip() {
    let wkt = "GEOMETRYCOLLECTION (POINT (1 2), LINESTRING (0 0, 1 1))";
    assert_eq!(roundtrip(wkt), wkt);
}

#[test]
fn geometrycollection_nested_roundtrip() {
    let wkt = "GEOMETRYCOLLECTION (POINT (0 0), POLYGON ((0 0, 1 0, 1 1, 0 0)))";
    assert_eq!(roundtrip(wkt), wkt);
}

#[test]
fn geometrycollection_empty_roundtrip() {
    assert_eq!(
        roundtrip("GEOMETRYCOLLECTION EMPTY"),
        "GEOMETRYCOLLECTION EMPTY"
    );
}

#[test]
fn geometrycollection_xy_parent_allows_heterogeneous_children() {
    let wkt = "GEOMETRYCOLLECTION (POINT Z (1 2 3), POINT M (4 5 6))";
    assert_eq!(roundtrip(wkt), wkt);
}

#[test]
fn geometrycollection_non_xy_parent_requires_matching_child_dimensions() {
    let wkt = "GEOMETRYCOLLECTION Z (POINT Z (1 2 3), LINESTRING Z (0 0 0, 1 1 1))";
    assert_eq!(roundtrip(wkt), wkt);

    assert!(wkt_to_wkb("GEOMETRYCOLLECTION Z (POINT (1 2))").is_err());
    assert!(wkt_to_wkb("GEOMETRYCOLLECTION Z (POINT M (1 2 3))").is_err());
}

// ── Hex convenience ──────────────────────────────────────────────────────────

#[test]
fn wkt_to_hex_wkb_point() {
    let hex = wkt_to_hex_wkb("POINT (1 2)").unwrap();
    // Must be uppercase hex
    assert!(hex
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
    // Must round-trip
    use wkb_wkt_converter::hex_wkb_to_wkt;
    assert_eq!(hex_wkb_to_wkt(&hex, SridMode::Auto).unwrap(), "POINT (1 2)");
}

// ── Whitespace tolerance ──────────────────────────────────────────────────────

#[test]
fn extra_whitespace_is_accepted() {
    assert_eq!(roundtrip("  POINT  (  1   2  )  "), "POINT (1 2)");
}

#[test]
fn comma_spacing_variants_are_accepted() {
    // Spacing around commas between coordinate pairs is irrelevant
    assert_eq!(roundtrip("LINESTRING(1 2,3 4)"), "LINESTRING (1 2, 3 4)");
    assert_eq!(roundtrip("LINESTRING(1 2 , 3 4)"), "LINESTRING (1 2, 3 4)");
    assert_eq!(
        roundtrip("POLYGON((0 0,1 0,1 1,0 0))"),
        "POLYGON ((0 0, 1 0, 1 1, 0 0))"
    );
}

#[test]
fn tab_and_newline_whitespace() {
    assert_eq!(roundtrip("POINT\t(1\t2)"), "POINT (1 2)");
    assert_eq!(
        roundtrip("LINESTRING\n(\n0 0,\n1 1\n)"),
        "LINESTRING (0 0, 1 1)"
    );
}

// ── Case insensitivity ────────────────────────────────────────────────────────

#[test]
fn lowercase_keyword() {
    assert_eq!(roundtrip("point (1 2)"), "POINT (1 2)");
    assert_eq!(roundtrip("linestring (0 0, 1 1)"), "LINESTRING (0 0, 1 1)");
}

#[test]
fn mixed_case_keyword() {
    assert_eq!(roundtrip("Point (1 2)"), "POINT (1 2)");
    assert_eq!(
        roundtrip("MultiPolygon (((0 0, 1 0, 1 1, 0 0)))"),
        "MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))"
    );
}

// ── No space before opening parenthesis ──────────────────────────────────────

#[test]
fn linestring_no_space_before_paren() {
    assert_eq!(roundtrip("LINESTRING(0 0, 1 1)"), "LINESTRING (0 0, 1 1)");
}

#[test]
fn polygon_no_space_before_paren() {
    assert_eq!(
        roundtrip("POLYGON((0 0, 1 0, 1 1, 0 0))"),
        "POLYGON ((0 0, 1 0, 1 1, 0 0))"
    );
}

#[test]
fn multipoint_no_space_before_paren() {
    assert_eq!(
        roundtrip("MULTIPOINT((0 0),(1 1))"),
        "MULTIPOINT ((0 0), (1 1))"
    );
}

#[test]
fn geometrycollection_no_space_before_paren() {
    assert_eq!(
        roundtrip("GEOMETRYCOLLECTION(POINT(1 2))"),
        "GEOMETRYCOLLECTION (POINT (1 2))"
    );
}

// ── Error cases ──────────────────────────────────────────────────────────────

#[test]
fn empty_string_errors() {
    assert!(wkt_to_wkb("").is_err());
}

#[test]
fn unknown_type_errors() {
    // Starts with an unrecognised letter (T → falls through to catch-all).
    assert!(wkt_to_wkb("TRIANGLE (0 0, 1 0, 0 1)").is_err());
    // Starts with a recognised letter but is not a valid keyword.
    assert!(wkt_to_wkb("GEOGRAPHY (0 0)").is_err()); // G → not GEOMETRYCOLLECTION
    assert!(wkt_to_wkb("MESH (0 0)").is_err()); // M → not MULTI*
    assert!(wkt_to_wkb("LOCUS (0 0)").is_err()); // L → not LINESTRING
    assert!(wkt_to_wkb("PLACE (0 0)").is_err()); // P → not POLYGON/POINT
}

#[test]
fn missing_closing_paren_errors() {
    assert!(wkt_to_wkb("POINT (1 2").is_err());
}

#[test]
fn trailing_content_errors() {
    assert!(wkt_to_wkb("POINT (1 2) EXTRA").is_err());
}

#[test]
fn non_ascii_malformed_input_errors_without_panic() {
    assert!(wkt_to_wkb("☃").is_err());
    assert!(wkt_to_wkb("POINT (1 2) ☃").is_err());
    assert!(wkt_to_wkb("POINT ☃").is_err());
    assert!(wkt_to_wkb("POINTé (1 2)").is_err());
}

#[test]
fn srid_without_semicolon_errors() {
    assert!(wkt_to_wkb("SRID=4326 POINT (1 2)").is_err());
}

#[test]
fn srid_missing_integer_errors() {
    // SRID= followed immediately by ';' — no integer value present
    assert!(wkt_to_wkb("SRID=;POINT (1 2)").is_err());
}

#[test]
fn srid_overflow_errors() {
    // Value exceeds u32::MAX — triggers the integer-out-of-range error path
    assert!(wkt_to_wkb("SRID=9999999999;POINT (1 2)").is_err());
}

#[test]
fn wkt_to_wkb_invalid_input_errors_in_all_srid_modes() {
    for mode in [SridMode::Auto, SridMode::Strip, SridMode::Set(4326)] {
        assert!(core_wkt_to_wkb("NOT_A_GEOMETRY (1 2)", mode).is_err());
    }
}

#[test]
fn wkt_to_wkb_set_validates_invalid_srid_prefix() {
    assert!(core_wkt_to_wkb("SRID=abc;POINT (1 2)", SridMode::Set(4326)).is_err());
}

#[test]
fn wkt_to_hex_wkb_invalid_input_errors_in_all_srid_modes() {
    for mode in [SridMode::Auto, SridMode::Strip, SridMode::Set(4326)] {
        assert!(core_wkt_to_hex_wkb("NOT_A_GEOMETRY (1 2)", mode).is_err());
    }
}

#[test]
fn wkt_to_hex_wkb_set_validates_invalid_srid_prefix() {
    assert!(core_wkt_to_hex_wkb("SRID=abc;POINT (1 2)", SridMode::Set(4326)).is_err());
}

// ── Tokenizer edge cases ──────────────────────────────────────────────────────

#[test]
fn point_no_space_before_paren() {
    // No whitespace between keyword and '(' — still valid, no dimension tag
    assert_eq!(roundtrip("POINT(1 2)"), "POINT (1 2)");
}

#[test]
fn invalid_coordinate_nonnumeric_errors() {
    assert!(wkt_to_wkb("POINT (abc 2)").is_err());
}

#[test]
fn invalid_coordinate_double_dot_errors() {
    // "1.2.3" is consumed as a token but fails f64 parsing
    assert!(wkt_to_wkb("POINT (1.2.3 4)").is_err());
}

#[test]
fn polygon_missing_ring_paren_errors() {
    // Ring coordinates without inner parentheses — expect_lparen inside read_rings fails
    assert!(wkt_to_wkb("POLYGON (0 0, 1 0, 1 1, 0 0)").is_err());
}

#[test]
fn linestring_invalid_separator_errors() {
    // '+' is neither ',' nor ')' — read_comma_or_rparen fails
    assert!(wkt_to_wkb("LINESTRING (0 0 + 1 1)").is_err());
}

#[test]
fn geometry_invalid_body_errors() {
    // Neither EMPTY nor '(' after the type keyword — read_empty_or_lparen fails
    assert!(wkt_to_wkb("POINT INVALID").is_err());
}

#[test]
fn concatenated_keywords_and_empty_are_rejected() {
    for wkt in [
        "POINTEMPTY",
        "LINESTRINGEMPTY",
        "POLYGONEMPTY",
        "MULTIPOINTEMPTY",
        "MULTILINESTRINGEMPTY",
        "MULTIPOLYGONEMPTY",
        "GEOMETRYCOLLECTIONEMPTY",
        "POINT EMPTYX",
        "LINESTRING EMPTYX",
        "POLYGON EMPTYX",
        "MULTIPOINT (EMPTYX)",
        "MULTILINESTRING (EMPTYX)",
        "MULTIPOLYGON (EMPTYX)",
        "POINTZ",
        "POINT ZEMPTY",
        "POINT MEMPTY",
        "POINT ZMEMPTY",
    ] {
        assert!(wkt_to_wkb(wkt).is_err(), "{wkt} should be rejected");
    }
}

#[test]
fn non_finite_wkt_coordinates_are_rejected() {
    assert!(wkt_to_wkb("POINT (1e9999 2)").is_err());
    assert!(wkt_to_wkb("LINESTRING (0 0, 1 1e9999)").is_err());
}

fn nested_geometrycollection_wkt(collection_depth: usize) -> String {
    let mut wkt = String::new();
    for _ in 0..collection_depth {
        wkt.push_str("GEOMETRYCOLLECTION (");
    }
    wkt.push_str("POINT (1 2)");
    for _ in 0..collection_depth {
        wkt.push(')');
    }
    wkt
}

#[test]
fn geometry_depth_limit_is_enforced_for_wkt() {
    assert!(wkt_to_wkb(&nested_geometrycollection_wkt(127)).is_ok());
    assert!(wkt_to_wkb(&nested_geometrycollection_wkt(128)).is_err());
}
