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


# ── hex helpers ───────────────────────────────────────────────────────────────

def test_wkt_to_hex_wkb_returns_uppercase_str():
    result = m.wkt_to_hex_wkb("POINT (1 2)")
    assert isinstance(result, str)
    assert result == result.upper()


def test_hex_wkb_to_wkt_roundtrip():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    assert m.hex_wkb_to_wkt(hex_wkb) == "POINT (1 2)"


def test_hex_wkb_to_wkt_invalid_raises_value_error():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.hex_wkb_to_wkt("ZZ")
