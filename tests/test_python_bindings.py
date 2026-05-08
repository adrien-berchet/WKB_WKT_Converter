"""
Python binding tests for wkb_wkt_converter.

These tests verify the Python layer: correct types, round-trips, SRID
handling, and that errors surface as ValueError with a readable message.
"""

import pytest
import wkb_wkt_converter as m


# ── helpers ───────────────────────────────────────────────────────────────────

def roundtrip(wkt: str) -> str:
    return m.wkb_to_wkt(m.wkt_to_wkb(wkt))


# ── wkt_to_wkb ────────────────────────────────────────────────────────────────

def test_wkt_to_wkb_returns_bytes():
    result = m.wkt_to_wkb("POINT (1 2)")
    assert isinstance(result, bytes)
    assert len(result) > 0


def test_wkt_to_wkb_little_endian_marker():
    # First byte of WKB is the byte-order marker; 0x01 = little-endian
    result = m.wkt_to_wkb("POINT (1 2)")
    assert result[0] == 0x01


def test_wkt_to_wkb_invalid_raises_value_error():
    with pytest.raises(ValueError, match="invalid WKT"):
        m.wkt_to_wkb("NOT_A_GEOMETRY (1 2)")


# ── wkb_to_wkt ────────────────────────────────────────────────────────────────

def test_wkb_to_wkt_returns_str():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    result = m.wkb_to_wkt(wkb)
    assert isinstance(result, str)


def test_wkb_to_wkt_invalid_raises_value_error():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.wkb_to_wkt(b"\x99")


# ── round-trips ───────────────────────────────────────────────────────────────

@pytest.mark.parametrize("wkt", [
    "POINT (1 2)",
    "POINT Z (1 2 3)",
    "POINT M (1 2 3)",
    "POINT ZM (1 2 3 4)",
    "POINT EMPTY",
    "LINESTRING (0 0, 1 1, 2 2)",
    "POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))",
    "MULTIPOINT ((0 0), (1 1))",
    "MULTILINESTRING ((0 0, 1 1), (2 2, 3 3))",
    "MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)), ((2 2, 3 2, 3 3, 2 2)))",
    "GEOMETRYCOLLECTION (POINT (1 2), LINESTRING (0 0, 1 1))",
    "GEOMETRYCOLLECTION EMPTY",
])
def test_roundtrip(wkt):
    assert roundtrip(wkt) == wkt


# ── SRID handling ─────────────────────────────────────────────────────────────

def test_wkt_to_wkb_embeds_srid():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    assert m.wkb_to_wkt(wkb) == "SRID=4326;POINT (1 2)"


def test_wkt_to_wkb_split_srid_returns_tuple():
    wkb, srid = m.wkt_to_wkb_split_srid("SRID=4326;POINT (1 2)")
    assert isinstance(wkb, bytes)
    assert srid == 4326
    # SRID must not be embedded in the bytes
    assert m.wkb_to_wkt(wkb) == "POINT (1 2)"


def test_wkt_to_wkb_split_srid_no_srid():
    wkb, srid = m.wkt_to_wkb_split_srid("POINT (1 2)")
    assert srid is None
    assert m.wkb_to_wkt(wkb) == "POINT (1 2)"


def test_wkb_to_wkt_split_srid_returns_tuple():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    wkt, srid = m.wkb_to_wkt_split_srid(wkb)
    assert wkt == "POINT (1 2)"
    assert srid == 4326


def test_wkb_to_wkt_split_srid_no_srid():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    wkt, srid = m.wkb_to_wkt_split_srid(wkb)
    assert wkt == "POINT (1 2)"
    assert srid is None


# ── whitespace and case tolerance ────────────────────────────────────────────

@pytest.mark.parametrize("wkt,expected", [
    # no space before opening parenthesis
    ("POINT(1 2)",                          "POINT (1 2)"),
    ("LINESTRING(0 0, 1 1)",                "LINESTRING (0 0, 1 1)"),
    ("POLYGON((0 0, 1 0, 1 1, 0 0))",       "POLYGON ((0 0, 1 0, 1 1, 0 0))"),
    ("MULTIPOINT((0 0),(1 1))",             "MULTIPOINT ((0 0), (1 1))"),
    ("GEOMETRYCOLLECTION(POINT(1 2))",      "GEOMETRYCOLLECTION (POINT (1 2))"),
    # lowercase and mixed-case keywords
    ("point (1 2)",                         "POINT (1 2)"),
    ("Point (1 2)",                         "POINT (1 2)"),
    ("linestring (0 0, 1 1)",               "LINESTRING (0 0, 1 1)"),
    ("MultiPolygon (((0 0, 1 0, 1 1, 0 0)))","MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))"),
    # tabs and newlines as whitespace
    ("POINT\t(1\t2)",                       "POINT (1 2)"),
    ("LINESTRING\n(\n0 0,\n1 1\n)",         "LINESTRING (0 0, 1 1)"),
])
def test_wkt_input_variants(wkt, expected):
    assert roundtrip(wkt) == expected


# ── hex helpers ───────────────────────────────────────────────────────────────

def test_wkt_to_hex_wkb_returns_uppercase_str():
    result = m.wkt_to_hex_wkb("POINT (1 2)")
    assert isinstance(result, str)
    assert result == result.upper()


def test_hex_wkb_to_wkt_roundtrip():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    assert m.hex_wkb_to_wkt(hex_wkb) == "POINT (1 2)"


def test_hex_wkb_to_wkt_srid_none_is_default():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    assert m.hex_wkb_to_wkt(hex_wkb) == m.hex_wkb_to_wkt(hex_wkb, srid=None)


def test_hex_wkb_to_wkt_srid_none_mirrors_input_with_srid():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    assert m.hex_wkb_to_wkt(hex_wkb, srid=None) == "SRID=4326;POINT (1 2)"


def test_hex_wkb_to_wkt_srid_false_strips_srid():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    assert m.hex_wkb_to_wkt(hex_wkb, srid=False) == "POINT (1 2)"


def test_hex_wkb_to_wkt_srid_false_no_srid_unchanged():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    assert m.hex_wkb_to_wkt(hex_wkb, srid=False) == "POINT (1 2)"


def test_hex_wkb_to_wkt_srid_int_adds_srid():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    assert m.hex_wkb_to_wkt(hex_wkb, srid=4326) == "SRID=4326;POINT (1 2)"


def test_hex_wkb_to_wkt_srid_int_overrides_srid():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    assert m.hex_wkb_to_wkt(hex_wkb, srid=3857) == "SRID=3857;POINT (1 2)"


def test_hex_wkb_to_wkt_srid_zero_is_accepted():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    assert m.hex_wkb_to_wkt(hex_wkb, srid=0) == "SRID=0;POINT (1 2)"


def test_hex_wkb_to_wkt_srid_true_raises_value_error():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    with pytest.raises(ValueError, match="srid=True is not valid"):
        m.hex_wkb_to_wkt(hex_wkb, srid=True)


def test_hex_wkb_to_wkt_srid_invalid_type_raises_value_error():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    with pytest.raises(ValueError, match="srid must be None, False, or a non-negative integer"):
        m.hex_wkb_to_wkt(hex_wkb, srid=3.14)


def test_hex_wkb_to_wkt_invalid_raises_value_error():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.hex_wkb_to_wkt("ZZ")


def test_hex_wkb_to_wkt_invalid_raises_value_error_with_srid_control():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.hex_wkb_to_wkt("ZZ", srid=False)
    with pytest.raises(ValueError, match="invalid WKB"):
        m.hex_wkb_to_wkt("ZZ", srid=4326)


def test_hex_wkb_to_wkt_trailing_bytes_raise_with_srid_control():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)") + "DEADBEEF"
    with pytest.raises(ValueError, match="invalid WKB"):
        m.hex_wkb_to_wkt(hex_wkb, srid=False)
    with pytest.raises(ValueError, match="invalid WKB"):
        m.hex_wkb_to_wkt(hex_wkb, srid=4326)


# ── text_to_wkb ───────────────────────────────────────────────────────────────

def test_text_to_wkb_from_wkt_returns_bytes():
    result = m.text_to_wkb("POINT (1 2)")
    assert isinstance(result, bytes)
    assert result == m.wkt_to_wkb("POINT (1 2)")


def test_text_to_wkb_from_hex_wkb_returns_bytes():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    result = m.text_to_wkb(hex_wkb)
    assert isinstance(result, bytes)
    assert result == m.wkt_to_wkb("POINT (1 2)")


def test_text_to_wkb_preserves_srid_from_wkt():
    result = m.text_to_wkb("SRID=4326;POINT (1 2)")
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_text_to_wkb_preserves_srid_from_hex_wkb():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    result = m.text_to_wkb(hex_wkb)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_text_to_wkb_invalid_raises_value_error():
    with pytest.raises(ValueError):
        m.text_to_wkb("NOT_A_GEOMETRY (1 2)")


def test_text_to_wkb_empty_string_raises():
    with pytest.raises(ValueError):
        m.text_to_wkb("")


def test_text_to_wkb_odd_length_hex_raises():
    with pytest.raises(ValueError):
        m.text_to_wkb("ABC")


def test_text_to_wkb_srid_zero_is_accepted():
    result = m.text_to_wkb("POINT (1 2)", srid=0)
    assert m.wkb_to_wkt(result) == "SRID=0;POINT (1 2)"


def test_text_to_wkb_srid_none_is_default():
    ewkt = "SRID=4326;POINT (1 2)"
    assert m.text_to_wkb(ewkt) == m.text_to_wkb(ewkt, srid=None)


def test_text_to_wkb_srid_none_mirrors_input_with_srid():
    result = m.text_to_wkb("SRID=4326;POINT (1 2)", srid=None)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_text_to_wkb_srid_none_mirrors_input_without_srid():
    result = m.text_to_wkb("POINT (1 2)", srid=None)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_text_to_wkb_srid_false_strips_srid_from_wkt():
    result = m.text_to_wkb("SRID=4326;POINT (1 2)", srid=False)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_text_to_wkb_srid_false_strips_srid_from_hex_wkb():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    result = m.text_to_wkb(hex_wkb, srid=False)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_text_to_wkb_srid_false_no_srid_unchanged():
    assert m.wkb_to_wkt(m.text_to_wkb("POINT (1 2)", srid=False)) == "POINT (1 2)"


def test_text_to_wkb_srid_int_adds_srid_to_plain_wkt():
    result = m.text_to_wkb("POINT (1 2)", srid=4326)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_text_to_wkb_srid_int_overrides_srid_in_ewkt():
    result = m.text_to_wkb("SRID=4326;POINT (1 2)", srid=3857)
    assert m.wkb_to_wkt(result) == "SRID=3857;POINT (1 2)"


def test_text_to_wkb_srid_int_adds_srid_to_plain_hex_wkb():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    result = m.text_to_wkb(hex_wkb, srid=4326)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_text_to_wkb_srid_int_overrides_srid_in_hex_ewkb():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    result = m.text_to_wkb(hex_wkb, srid=3857)
    assert m.wkb_to_wkt(result) == "SRID=3857;POINT (1 2)"


def test_text_to_wkb_srid_true_raises_value_error():
    with pytest.raises(ValueError, match="srid=True is not valid"):
        m.text_to_wkb("POINT (1 2)", srid=True)


def test_text_to_wkb_srid_invalid_type_raises_value_error():
    with pytest.raises(ValueError, match="srid must be None, False, or a non-negative integer"):
        m.text_to_wkb("POINT (1 2)", srid=3.14)


# ── text_to_wkt ───────────────────────────────────────────────────────────────

def test_text_to_wkt_from_wkt_returns_str():
    result = m.text_to_wkt("POINT (1 2)")
    assert isinstance(result, str)
    assert result == "POINT (1 2)"


def test_text_to_wkt_from_hex_wkb_returns_str():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    result = m.text_to_wkt(hex_wkb)
    assert isinstance(result, str)
    assert result == "POINT (1 2)"


def test_text_to_wkt_normalises_wkt():
    # WKT with non-canonical whitespace/casing is normalised when normalize_wkt=True
    assert m.text_to_wkt("point(1 2)", normalize_wkt=True) == "POINT (1 2)"


def test_text_to_wkt_preserves_srid_from_wkt():
    assert m.text_to_wkt("SRID=4326;POINT (1 2)") == "SRID=4326;POINT (1 2)"


def test_text_to_wkt_preserves_srid_from_hex_wkb():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    assert m.text_to_wkt(hex_wkb) == "SRID=4326;POINT (1 2)"


@pytest.mark.parametrize("wkt", [
    "POINT (1 2)",
    "POINT Z (1 2 3)",
    "LINESTRING (0 0, 1 1)",
    "POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))",
    "MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)), ((2 2, 3 2, 3 3, 2 2)))",
    "GEOMETRYCOLLECTION (POINT (1 2), LINESTRING (0 0, 1 1))",
])
def test_text_to_wkt_roundtrip_from_wkt(wkt):
    assert m.text_to_wkt(wkt) == wkt


@pytest.mark.parametrize("wkt", [
    "POINT (1 2)",
    "POINT Z (1 2 3)",
    "LINESTRING (0 0, 1 1)",
    "POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))",
    "SRID=4326;POINT (1 2)",
])
def test_text_to_wkt_roundtrip_from_hex(wkt):
    hex_wkb = m.wkt_to_hex_wkb(wkt)
    assert m.text_to_wkt(hex_wkb) == wkt


def test_text_to_wkt_invalid_raises_value_error():
    with pytest.raises(ValueError):
        m.text_to_wkt("NOT_A_GEOMETRY (1 2)", normalize_wkt=True)


def test_text_to_wkt_empty_string_raises():
    with pytest.raises(ValueError):
        m.text_to_wkt("", normalize_wkt=True)


def test_text_to_wkt_odd_length_hex_raises():
    with pytest.raises(ValueError):
        m.text_to_wkt("ABC", normalize_wkt=True)


def test_text_to_wkt_srid_zero_is_accepted():
    result = m.text_to_wkt("POINT (1 2)", srid=0, normalize_wkt=True)
    assert result == "SRID=0;POINT (1 2)"


def test_text_to_wkt_srid_none_is_default():
    ewkt = "SRID=4326;POINT (1 2)"
    assert m.text_to_wkt(ewkt) == m.text_to_wkt(ewkt, srid=None)


def test_text_to_wkt_srid_none_mirrors_input_with_srid():
    assert m.text_to_wkt("SRID=4326;POINT (1 2)", srid=None) == "SRID=4326;POINT (1 2)"


def test_text_to_wkt_srid_none_mirrors_input_without_srid():
    assert m.text_to_wkt("POINT (1 2)", srid=None) == "POINT (1 2)"


def test_text_to_wkt_srid_false_strips_srid_from_wkt():
    assert m.text_to_wkt("SRID=4326;POINT (1 2)", srid=False) == "POINT (1 2)"


def test_text_to_wkt_srid_false_strips_srid_from_hex_wkb():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    assert m.text_to_wkt(hex_wkb, srid=False) == "POINT (1 2)"


def test_text_to_wkt_srid_false_normalises_wkt():
    assert m.text_to_wkt("point(1 2)", srid=False, normalize_wkt=True) == "POINT (1 2)"


def test_text_to_wkt_srid_false_no_srid_unchanged():
    assert m.text_to_wkt("POINT (1 2)", srid=False) == "POINT (1 2)"


def test_text_to_wkt_srid_int_adds_srid_to_plain_wkt():
    assert m.text_to_wkt("POINT (1 2)", srid=4326) == "SRID=4326;POINT (1 2)"


def test_text_to_wkt_srid_int_overrides_srid_in_ewkt():
    assert m.text_to_wkt("SRID=4326;POINT (1 2)", srid=3857) == "SRID=3857;POINT (1 2)"


def test_text_to_wkt_srid_int_adds_srid_to_plain_hex_wkb():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    assert m.text_to_wkt(hex_wkb, srid=4326) == "SRID=4326;POINT (1 2)"


def test_text_to_wkt_srid_int_overrides_srid_in_hex_ewkb():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    assert m.text_to_wkt(hex_wkb, srid=3857) == "SRID=3857;POINT (1 2)"


def test_text_to_wkt_srid_true_raises_value_error():
    with pytest.raises(ValueError, match="srid=True is not valid"):
        m.text_to_wkt("POINT (1 2)", srid=True)


# ── text_to_wkt: normalize_wkt parameter ─────────────────────────────────────

def test_text_to_wkt_normalize_wkt_false_is_default():
    assert m.text_to_wkt("point(1 2)") == m.text_to_wkt("point(1 2)", normalize_wkt=False)


def test_text_to_wkt_normalize_wkt_true_normalises_casing():
    assert m.text_to_wkt("point(1 2)", normalize_wkt=True) == "POINT (1 2)"


def test_text_to_wkt_normalize_wkt_false_returns_wkt_as_is():
    assert m.text_to_wkt("point(1 2)", normalize_wkt=False) == "point(1 2)"


def test_text_to_wkt_normalize_wkt_false_auto_preserves_srid_prefix():
    assert m.text_to_wkt("SRID=4326;POINT (1 2)", normalize_wkt=False) == "SRID=4326;POINT (1 2)"


def test_text_to_wkt_normalize_wkt_false_strip_removes_srid_prefix():
    assert m.text_to_wkt("SRID=4326;POINT (1 2)", srid=False, normalize_wkt=False) == "POINT (1 2)"


def test_text_to_wkt_normalize_wkt_false_strip_noop_when_no_srid():
    assert m.text_to_wkt("POINT (1 2)", srid=False, normalize_wkt=False) == "POINT (1 2)"


def test_text_to_wkt_normalize_wkt_false_set_adds_srid_prefix():
    assert m.text_to_wkt("POINT (1 2)", srid=4326, normalize_wkt=False) == "SRID=4326;POINT (1 2)"


def test_text_to_wkt_normalize_wkt_false_set_overrides_srid_prefix():
    assert m.text_to_wkt("SRID=4326;POINT (1 2)", srid=3857, normalize_wkt=False) == "SRID=3857;POINT (1 2)"


def test_text_to_wkt_normalize_wkt_false_hex_input_still_normalises():
    # normalize_wkt has no effect on hex WKB input; output is always normalised.
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    assert m.text_to_wkt(hex_wkb, normalize_wkt=False) == "POINT (1 2)"


# ── text_to_hex_wkb ───────────────────────────────────────────────────────────

def test_text_to_hex_wkb_from_wkt_returns_uppercase_str():
    result = m.text_to_hex_wkb("POINT (1 2)")
    assert isinstance(result, str)
    assert result == result.upper()


def test_text_to_hex_wkb_from_wkt_matches_wkt_to_hex_wkb():
    assert m.text_to_hex_wkb("POINT (1 2)") == m.wkt_to_hex_wkb("POINT (1 2)")


def test_text_to_hex_wkb_from_hex_wkb_is_identity():
    original = m.wkt_to_hex_wkb("LINESTRING (0 0, 1 1)")
    assert m.text_to_hex_wkb(original) == original


def test_text_to_hex_wkb_srid_none_is_default():
    ewkt = "SRID=4326;POINT (1 2)"
    assert m.text_to_hex_wkb(ewkt) == m.text_to_hex_wkb(ewkt, srid=None)


def test_text_to_hex_wkb_srid_none_preserves_srid():
    result = m.text_to_hex_wkb("SRID=4326;POINT (1 2)", srid=None)
    assert m.hex_wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_text_to_hex_wkb_srid_false_strips_srid():
    result = m.text_to_hex_wkb("SRID=4326;POINT (1 2)", srid=False)
    assert m.hex_wkb_to_wkt(result) == "POINT (1 2)"


def test_text_to_hex_wkb_srid_int_adds_srid_to_plain_wkt():
    result = m.text_to_hex_wkb("POINT (1 2)", srid=4326)
    assert m.hex_wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_text_to_hex_wkb_srid_int_overrides_srid():
    result = m.text_to_hex_wkb("SRID=4326;POINT (1 2)", srid=3857)
    assert m.hex_wkb_to_wkt(result) == "SRID=3857;POINT (1 2)"


def test_text_to_hex_wkb_srid_true_raises_value_error():
    with pytest.raises(ValueError, match="srid=True is not valid"):
        m.text_to_hex_wkb("POINT (1 2)", srid=True)


def test_text_to_hex_wkb_invalid_raises_value_error():
    with pytest.raises(ValueError):
        m.text_to_hex_wkb("NOT_A_GEOMETRY (1 2)")
