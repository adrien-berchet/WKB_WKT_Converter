"""
Python binding tests for wkb_wkt_converter.

These tests verify the Python layer: correct types, round-trips, SRID
handling, and that errors surface as ValueError with a readable message.
"""

from array import array

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


def test_wkt_to_wkb_srid_none_is_default():
    ewkt = "SRID=4326;POINT (1 2)"
    assert m.wkt_to_wkb(ewkt) == m.wkt_to_wkb(ewkt, srid=None)


def test_wkt_to_wkb_srid_false_strips_srid():
    result = m.wkt_to_wkb("SRID=4326;POINT (1 2)", srid=False)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_wkt_to_wkb_srid_false_no_srid_unchanged():
    result = m.wkt_to_wkb("POINT (1 2)", srid=False)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_wkt_to_wkb_srid_int_adds_srid():
    result = m.wkt_to_wkb("POINT (1 2)", srid=4326)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_wkt_to_wkb_srid_int_overrides_srid():
    result = m.wkt_to_wkb("SRID=4326;POINT (1 2)", srid=3857)
    assert m.wkb_to_wkt(result) == "SRID=3857;POINT (1 2)"


def test_wkt_to_wkb_srid_zero_acts_as_strip():
    result = m.wkt_to_wkb("SRID=4326;POINT (1 2)", srid=0)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_wkt_to_wkb_srid_minus_one_acts_as_strip():
    result = m.wkt_to_wkb("SRID=4326;POINT (1 2)", srid=-1)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_ewkt_with_negative_srid_strips_srid():
    result = m.wkt_to_wkb("SRID=-1;POINT (1 2)")
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_wkt_to_wkb_srid_true_raises_value_error():
    with pytest.raises(ValueError, match="srid=True is not valid"):
        m.wkt_to_wkb("POINT (1 2)", srid=True)


def test_wkt_to_wkb_srid_invalid_type_raises_value_error():
    with pytest.raises(ValueError, match="srid must be None, False, or an integer"):
        m.wkt_to_wkb("POINT (1 2)", srid=3.14)


def test_wkt_to_wkb_srid_out_of_range_raises_value_error():
    with pytest.raises(ValueError, match="32-bit"):
        m.wkt_to_wkb("POINT (1 2)", srid=2**40)


def test_wkt_to_wkb_srid_huge_int_raises_range_value_error():
    with pytest.raises(ValueError, match="32-bit"):
        m.wkt_to_wkb("POINT (1 2)", srid=2**80)


def test_wkt_to_wkb_srid_index_error_propagates():
    class BadIndex:
        def __index__(self):
            raise RuntimeError("boom")

    with pytest.raises(RuntimeError, match="boom"):
        m.wkt_to_wkb("POINT (1 2)", srid=BadIndex())


def test_wkt_to_wkb_srid_index_attribute_error_propagates():
    class BadIndex:
        def __index__(self):
            raise AttributeError("boom")

    with pytest.raises(AttributeError, match="boom"):
        m.wkt_to_wkb("POINT (1 2)", srid=BadIndex())


def test_wkt_to_wkb_srid_index_lookup_error_propagates():
    class BadIndexLookup:
        def __getattribute__(self, name):
            if name == "__index__":
                raise RuntimeError("boom")
            return super().__getattribute__(name)

    with pytest.raises(RuntimeError, match="boom"):
        m.wkt_to_wkb("POINT (1 2)", srid=BadIndexLookup())


def test_wkt_to_wkb_srid_index_returning_non_int_raises_type_error():
    class NonIntegerIndex:
        def __index__(self):
            return "4326"

    with pytest.raises(ValueError, match="srid must be None, False, or an integer"):
        m.wkt_to_wkb("POINT (1 2)", srid=NonIntegerIndex())


@pytest.mark.parametrize("index_value", [False, True])
def test_wkt_to_wkb_srid_index_returning_bool_raises_type_error(index_value):
    class BoolIndex:
        def __index__(self):
            return index_value

    with pytest.raises(ValueError, match="srid must be None, False, or an integer"):
        m.wkt_to_wkb("POINT (1 2)", srid=BoolIndex())


def test_wkt_to_wkb_srid_index_returning_huge_int_raises_range_value_error():
    class HugeIndex:
        def __index__(self):
            return 2**80

    with pytest.raises(ValueError, match="32-bit"):
        m.wkt_to_wkb("POINT (1 2)", srid=HugeIndex())


def test_wkt_to_wkb_srid_index_overflow_calls_index_once():
    class CountingHugeIndex:
        def __init__(self):
            self.calls = 0

        def __index__(self):
            self.calls += 1
            return 2**80

    srid = CountingHugeIndex()

    with pytest.raises(ValueError, match="32-bit"):
        m.wkt_to_wkb("POINT (1 2)", srid=srid)

    assert srid.calls == 1


def test_wkt_to_wkb_srid_index_success_calls_index_once():
    class CountingIndex:
        def __init__(self):
            self.calls = 0

        def __index__(self):
            self.calls += 1
            return 4326

    srid = CountingIndex()

    result = m.wkt_to_wkb("POINT (1 2)", srid=srid)

    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"
    assert srid.calls == 1


def test_wkt_to_wkb_invalid_raises_value_error_with_srid_control():
    with pytest.raises(ValueError, match="invalid WKT"):
        m.wkt_to_wkb("NOT_A_GEOMETRY (1 2)", srid=False)
    with pytest.raises(ValueError, match="invalid WKT"):
        m.wkt_to_wkb("NOT_A_GEOMETRY (1 2)", srid=4326)


# ── wkb_to_wkt ────────────────────────────────────────────────────────────────

def test_wkb_to_wkt_returns_str():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    result = m.wkb_to_wkt(wkb)
    assert isinstance(result, str)


@pytest.mark.parametrize("make_input", [
    pytest.param(bytearray, id="bytearray"),
    pytest.param(memoryview, id="memoryview"),
    pytest.param(lambda wkb: memoryview(bytearray(wkb)), id="bytearray-memoryview"),
    pytest.param(lambda wkb: memoryview(b"\x00" + wkb)[1:], id="sliced-memoryview"),
    pytest.param(lambda wkb: array("B", wkb), id="unsigned-byte-array"),
    pytest.param(
        lambda wkb: array("b", (byte if byte < 128 else byte - 256 for byte in wkb)),
        id="signed-byte-array",
    ),
])
def test_wkb_to_wkt_accepts_bytes_like(make_input):
    wkb = m.wkt_to_wkb("POINT (1 2)")
    assert m.wkb_to_wkt(make_input(wkb)) == "POINT (1 2)"


@pytest.mark.parametrize("make_input", [
    pytest.param(bytearray, id="bytearray"),
    pytest.param(memoryview, id="memoryview"),
])
def test_wkb_to_wkt_invalid_bytes_like_raises_value_error(make_input):
    with pytest.raises(ValueError, match="invalid WKB"):
        m.wkb_to_wkt(make_input(b"\x99"))


@pytest.mark.parametrize("make_input", [
    pytest.param(bytearray, id="bytearray"),
    pytest.param(memoryview, id="memoryview"),
])
def test_wkb_to_wkt_empty_bytes_like_raises_value_error(make_input):
    with pytest.raises(ValueError, match="invalid WKB"):
        m.wkb_to_wkt(make_input(b""))


def test_wkb_to_wkt_rejects_non_buffer_input():
    with pytest.raises(BufferError, match="contiguous one-byte buffer"):
        m.wkb_to_wkt(123)


def test_wkb_to_wkt_preserves_buffer_acquisition_errors():
    view = memoryview(bytearray(b"\x99"))
    view.release()
    with pytest.raises(ValueError, match="released memoryview"):
        m.wkb_to_wkt(view)


def test_wkb_to_wkt_rejects_non_contiguous_memoryview():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    with pytest.raises(BufferError, match="contiguous one-byte buffer"):
        m.wkb_to_wkt(memoryview(wkb)[::2])


def test_wkb_to_wkt_rejects_non_byte_buffer():
    with pytest.raises(BufferError, match="contiguous one-byte buffer"):
        m.wkb_to_wkt(array("H", [1, 2, 3]))


def test_wkb_to_wkt_invalid_raises_value_error():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.wkb_to_wkt(b"\x99")


def test_wkb_to_wkt_srid_none_is_default():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    assert m.wkb_to_wkt(wkb) == m.wkb_to_wkt(wkb, srid=None)


def test_wkb_to_wkt_srid_none_mirrors_input_with_srid():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    assert m.wkb_to_wkt(wkb, srid=None) == "SRID=4326;POINT (1 2)"


def test_wkb_to_wkt_srid_false_strips_srid():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    assert m.wkb_to_wkt(wkb, srid=False) == "POINT (1 2)"


def test_wkb_to_wkt_srid_false_no_srid_unchanged():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    assert m.wkb_to_wkt(wkb, srid=False) == "POINT (1 2)"


def test_wkb_to_wkt_srid_int_adds_srid():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    assert m.wkb_to_wkt(wkb, srid=4326) == "SRID=4326;POINT (1 2)"


def test_wkb_to_wkt_srid_int_overrides_srid():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    assert m.wkb_to_wkt(wkb, srid=3857) == "SRID=3857;POINT (1 2)"


def test_wkb_to_wkt_srid_zero_acts_as_strip():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    assert m.wkb_to_wkt(wkb, srid=0) == "POINT (1 2)"


def test_wkb_to_wkt_srid_negative_one_acts_as_strip():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    assert m.wkb_to_wkt(wkb, srid=-1) == "POINT (1 2)"


def test_wkb_to_wkt_srid_true_raises_value_error():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    with pytest.raises(ValueError, match="srid=True is not valid"):
        m.wkb_to_wkt(wkb, srid=True)


def test_wkb_to_wkt_srid_invalid_type_raises_value_error():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    with pytest.raises(ValueError, match="srid must be None, False, or an integer"):
        m.wkb_to_wkt(wkb, srid=3.14)


def test_wkb_to_wkt_invalid_raises_value_error_with_srid_control():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.wkb_to_wkt(b"\x99", srid=False)
    with pytest.raises(ValueError, match="invalid WKB"):
        m.wkb_to_wkt(b"\x99", srid=4326)


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


@pytest.mark.parametrize("make_input", [
    pytest.param(bytearray, id="bytearray"),
    pytest.param(memoryview, id="memoryview"),
    pytest.param(lambda wkb: memoryview(bytearray(wkb)), id="bytearray-memoryview"),
    pytest.param(lambda wkb: memoryview(b"\x00" + wkb)[1:], id="sliced-memoryview"),
    pytest.param(lambda wkb: array("B", wkb), id="unsigned-byte-array"),
])
def test_wkb_to_wkt_split_srid_accepts_bytes_like(make_input):
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    wkt, srid = m.wkb_to_wkt_split_srid(make_input(wkb))
    assert wkt == "POINT (1 2)"
    assert srid == 4326


def test_wkb_to_wkt_split_srid_rejects_non_buffer_input():
    with pytest.raises(BufferError, match="contiguous one-byte buffer"):
        m.wkb_to_wkt_split_srid(123)


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


def test_wkt_to_hex_wkb_srid_none_is_default():
    ewkt = "SRID=4326;POINT (1 2)"
    assert m.wkt_to_hex_wkb(ewkt) == m.wkt_to_hex_wkb(ewkt, srid=None)


def test_wkt_to_hex_wkb_srid_false_strips_srid():
    result = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)", srid=False)
    assert m.hex_wkb_to_wkt(result) == "POINT (1 2)"


def test_wkt_to_hex_wkb_srid_false_no_srid_unchanged():
    result = m.wkt_to_hex_wkb("POINT (1 2)", srid=False)
    assert m.hex_wkb_to_wkt(result) == "POINT (1 2)"


def test_wkt_to_hex_wkb_srid_int_adds_srid():
    result = m.wkt_to_hex_wkb("POINT (1 2)", srid=4326)
    assert m.hex_wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_wkt_to_hex_wkb_srid_int_overrides_srid():
    result = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)", srid=3857)
    assert m.hex_wkb_to_wkt(result) == "SRID=3857;POINT (1 2)"


def test_wkt_to_hex_wkb_srid_zero_acts_as_strip():
    result = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)", srid=0)
    assert m.hex_wkb_to_wkt(result) == "POINT (1 2)"


def test_wkt_to_hex_wkb_srid_true_raises_value_error():
    with pytest.raises(ValueError, match="srid=True is not valid"):
        m.wkt_to_hex_wkb("POINT (1 2)", srid=True)


def test_wkt_to_hex_wkb_srid_invalid_type_raises_value_error():
    with pytest.raises(ValueError, match="srid must be None, False, or an integer"):
        m.wkt_to_hex_wkb("POINT (1 2)", srid=3.14)


def test_wkt_to_hex_wkb_invalid_raises_value_error_with_srid_control():
    with pytest.raises(ValueError, match="invalid WKT"):
        m.wkt_to_hex_wkb("NOT_A_GEOMETRY (1 2)", srid=False)
    with pytest.raises(ValueError, match="invalid WKT"):
        m.wkt_to_hex_wkb("NOT_A_GEOMETRY (1 2)", srid=4326)


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


def test_hex_wkb_to_wkt_srid_zero_acts_as_strip():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    assert m.hex_wkb_to_wkt(hex_wkb, srid=0) == "POINT (1 2)"


def test_hex_wkb_to_wkt_srid_true_raises_value_error():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    with pytest.raises(ValueError, match="srid=True is not valid"):
        m.hex_wkb_to_wkt(hex_wkb, srid=True)


def test_hex_wkb_to_wkt_srid_invalid_type_raises_value_error():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    with pytest.raises(ValueError, match="srid must be None, False, or an integer"):
        m.hex_wkb_to_wkt(hex_wkb, srid=3.14)


def test_hex_wkb_to_wkt_invalid_raises_value_error():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.hex_wkb_to_wkt("ZZ")


def test_hex_wkb_to_wkt_accepts_hex_wkb_keyword():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    assert m.hex_wkb_to_wkt(hex_wkb=hex_wkb) == "POINT (1 2)"


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


@pytest.mark.parametrize(
    "old_name",
    ["text_to_wkb", "text_to_wkt", "text_to_hex_wkb"],
)
def test_legacy_text_to_names_are_not_exported(old_name):
    assert not hasattr(m, old_name)


# ── to_wkb ───────────────────────────────────────────────────────────────

def test_to_wkb_from_wkt_returns_bytes():
    result = m.to_wkb("POINT (1 2)")
    assert isinstance(result, bytes)
    assert result == m.wkt_to_wkb("POINT (1 2)")


def test_to_wkb_from_hex_wkb_returns_bytes():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    result = m.to_wkb(hex_wkb)
    assert isinstance(result, bytes)
    assert result == m.wkt_to_wkb("POINT (1 2)")


@pytest.mark.parametrize("make_input", [
    pytest.param(bytes, id="bytes"),
    pytest.param(bytearray, id="bytearray"),
    pytest.param(memoryview, id="memoryview"),
    pytest.param(lambda wkb: memoryview(b"\x00" + wkb)[1:], id="sliced-memoryview"),
    pytest.param(lambda wkb: array("B", wkb), id="unsigned-byte-array"),
])
def test_to_wkb_from_bytes_like_wkb_returns_bytes(make_input):
    wkb = m.wkt_to_wkb("POINT (1 2)")
    result = m.to_wkb(make_input(wkb))
    assert isinstance(result, bytes)
    assert result == wkb


def test_to_wkb_from_bytes_like_wkb_preserves_srid():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    result = m.to_wkb(bytearray(wkb))
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_wkb_preserves_srid_from_wkt():
    result = m.to_wkb("SRID=4326;POINT (1 2)")
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_wkb_preserves_srid_from_hex_wkb():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    result = m.to_wkb(hex_wkb)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_wkb_invalid_raises_value_error():
    with pytest.raises(ValueError):
        m.to_wkb("NOT_A_GEOMETRY (1 2)")


def test_to_wkb_empty_string_raises():
    with pytest.raises(ValueError):
        m.to_wkb("")


def test_to_wkb_odd_length_hex_raises():
    with pytest.raises(ValueError):
        m.to_wkb("ABC")


def test_to_wkb_srid_zero_acts_as_strip():
    result = m.to_wkb("SRID=4326;POINT (1 2)", srid=0)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_to_wkb_srid_none_is_default():
    ewkt = "SRID=4326;POINT (1 2)"
    assert m.to_wkb(ewkt) == m.to_wkb(ewkt, srid=None)


def test_to_wkb_srid_none_mirrors_input_with_srid():
    result = m.to_wkb("SRID=4326;POINT (1 2)", srid=None)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_wkb_srid_none_mirrors_input_without_srid():
    result = m.to_wkb("POINT (1 2)", srid=None)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_to_wkb_srid_false_strips_srid_from_wkt():
    result = m.to_wkb("SRID=4326;POINT (1 2)", srid=False)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_to_wkb_srid_false_strips_srid_from_hex_wkb():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    result = m.to_wkb(hex_wkb, srid=False)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_to_wkb_srid_false_strips_srid_from_bytes_like_wkb():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    result = m.to_wkb(memoryview(wkb), srid=False)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_to_wkb_srid_false_no_srid_unchanged():
    assert m.wkb_to_wkt(m.to_wkb("POINT (1 2)", srid=False)) == "POINT (1 2)"


def test_to_wkb_srid_int_adds_srid_to_plain_wkt():
    result = m.to_wkb("POINT (1 2)", srid=4326)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_wkb_srid_int_overrides_srid_in_ewkt():
    result = m.to_wkb("SRID=4326;POINT (1 2)", srid=3857)
    assert m.wkb_to_wkt(result) == "SRID=3857;POINT (1 2)"


def test_to_wkb_srid_int_adds_srid_to_plain_hex_wkb():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    result = m.to_wkb(hex_wkb, srid=4326)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_wkb_srid_int_adds_srid_to_bytes_like_wkb():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    result = m.to_wkb(bytearray(wkb), srid=4326)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_wkb_srid_int_overrides_srid_in_hex_ewkb():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    result = m.to_wkb(hex_wkb, srid=3857)
    assert m.wkb_to_wkt(result) == "SRID=3857;POINT (1 2)"


def test_to_wkb_srid_none_does_not_validate_bytes_like_wkb():
    assert m.to_wkb(b"\x99") == b"\x99"


def test_to_wkb_bytes_like_ascii_hex_is_raw_wkb_not_hex_text():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    assert m.to_wkb(hex_wkb.encode("ascii")) == hex_wkb.encode("ascii")


def test_to_wkb_accepts_source_keyword():
    assert m.to_wkb(source="POINT (1 2)") == m.wkt_to_wkb("POINT (1 2)")


def test_to_wkb_rejects_non_buffer_non_str_input():
    with pytest.raises(BufferError, match="contiguous one-byte buffer"):
        m.to_wkb(123)


def test_to_wkb_rejects_non_byte_buffer():
    with pytest.raises(BufferError, match="contiguous one-byte buffer"):
        m.to_wkb(array("H", [1, 2, 3]))


def test_to_wkb_srid_true_raises_value_error():
    with pytest.raises(ValueError, match="srid=True is not valid"):
        m.to_wkb("POINT (1 2)", srid=True)


def test_to_wkb_srid_invalid_type_raises_value_error():
    with pytest.raises(ValueError, match="srid must be None, False, or an integer"):
        m.to_wkb("POINT (1 2)", srid=3.14)


# ── to_wkt ───────────────────────────────────────────────────────────────

def test_to_wkt_from_wkt_returns_str():
    result = m.to_wkt("POINT (1 2)")
    assert isinstance(result, str)
    assert result == "POINT (1 2)"


def test_to_wkt_from_hex_wkb_returns_str():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    result = m.to_wkt(hex_wkb)
    assert isinstance(result, str)
    assert result == "POINT (1 2)"


@pytest.mark.parametrize("make_input", [
    pytest.param(bytes, id="bytes"),
    pytest.param(bytearray, id="bytearray"),
    pytest.param(memoryview, id="memoryview"),
    pytest.param(lambda wkb: memoryview(b"\x00" + wkb)[1:], id="sliced-memoryview"),
    pytest.param(lambda wkb: array("B", wkb), id="unsigned-byte-array"),
])
def test_to_wkt_from_bytes_like_wkb_returns_str(make_input):
    wkb = m.wkt_to_wkb("POINT (1 2)")
    result = m.to_wkt(make_input(wkb))
    assert isinstance(result, str)
    assert result == "POINT (1 2)"


def test_to_wkt_normalises_wkt():
    # WKT with non-canonical whitespace/casing is normalised when normalize_wkt=True
    assert m.to_wkt("point(1 2)", normalize_wkt=True) == "POINT (1 2)"


def test_to_wkt_preserves_srid_from_wkt():
    assert m.to_wkt("SRID=4326;POINT (1 2)") == "SRID=4326;POINT (1 2)"


def test_to_wkt_preserves_srid_from_hex_wkb():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    assert m.to_wkt(hex_wkb) == "SRID=4326;POINT (1 2)"


def test_to_wkt_preserves_srid_from_bytes_like_wkb():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    assert m.to_wkt(bytearray(wkb)) == "SRID=4326;POINT (1 2)"


@pytest.mark.parametrize("wkt", [
    "POINT (1 2)",
    "POINT Z (1 2 3)",
    "LINESTRING (0 0, 1 1)",
    "POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))",
    "MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)), ((2 2, 3 2, 3 3, 2 2)))",
    "GEOMETRYCOLLECTION (POINT (1 2), LINESTRING (0 0, 1 1))",
])
def test_to_wkt_roundtrip_from_wkt(wkt):
    assert m.to_wkt(wkt) == wkt


@pytest.mark.parametrize("wkt", [
    "POINT (1 2)",
    "POINT Z (1 2 3)",
    "LINESTRING (0 0, 1 1)",
    "POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))",
    "SRID=4326;POINT (1 2)",
])
def test_to_wkt_roundtrip_from_hex(wkt):
    hex_wkb = m.wkt_to_hex_wkb(wkt)
    assert m.to_wkt(hex_wkb) == wkt


def test_to_wkt_invalid_raises_value_error():
    with pytest.raises(ValueError):
        m.to_wkt("NOT_A_GEOMETRY (1 2)", normalize_wkt=True)


def test_to_wkt_empty_string_raises():
    with pytest.raises(ValueError):
        m.to_wkt("", normalize_wkt=True)


def test_to_wkt_odd_length_hex_raises():
    with pytest.raises(ValueError):
        m.to_wkt("ABC", normalize_wkt=True)


def test_to_wkt_srid_zero_acts_as_strip():
    result = m.to_wkt("SRID=4326;POINT (1 2)", srid=0, normalize_wkt=True)
    assert result == "POINT (1 2)"


def test_to_wkt_srid_none_is_default():
    ewkt = "SRID=4326;POINT (1 2)"
    assert m.to_wkt(ewkt) == m.to_wkt(ewkt, srid=None)


def test_to_wkt_srid_none_mirrors_input_with_srid():
    assert m.to_wkt("SRID=4326;POINT (1 2)", srid=None) == "SRID=4326;POINT (1 2)"


def test_to_wkt_srid_none_mirrors_input_without_srid():
    assert m.to_wkt("POINT (1 2)", srid=None) == "POINT (1 2)"


def test_to_wkt_srid_false_strips_srid_from_wkt():
    assert m.to_wkt("SRID=4326;POINT (1 2)", srid=False) == "POINT (1 2)"


def test_to_wkt_srid_false_strips_srid_from_hex_wkb():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    assert m.to_wkt(hex_wkb, srid=False) == "POINT (1 2)"


def test_to_wkt_srid_false_strips_srid_from_bytes_like_wkb():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    assert m.to_wkt(memoryview(wkb), srid=False) == "POINT (1 2)"


def test_to_wkt_srid_false_normalises_wkt():
    assert m.to_wkt("point(1 2)", srid=False, normalize_wkt=True) == "POINT (1 2)"


def test_to_wkt_srid_false_no_srid_unchanged():
    assert m.to_wkt("POINT (1 2)", srid=False) == "POINT (1 2)"


def test_to_wkt_srid_int_adds_srid_to_plain_wkt():
    assert m.to_wkt("POINT (1 2)", srid=4326) == "SRID=4326;POINT (1 2)"


def test_to_wkt_srid_int_overrides_srid_in_ewkt():
    assert m.to_wkt("SRID=4326;POINT (1 2)", srid=3857) == "SRID=3857;POINT (1 2)"


def test_to_wkt_srid_int_adds_srid_to_plain_hex_wkb():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    assert m.to_wkt(hex_wkb, srid=4326) == "SRID=4326;POINT (1 2)"


def test_to_wkt_srid_int_adds_srid_to_bytes_like_wkb():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    assert m.to_wkt(bytearray(wkb), srid=4326) == "SRID=4326;POINT (1 2)"


def test_to_wkt_srid_int_overrides_srid_in_hex_ewkb():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    assert m.to_wkt(hex_wkb, srid=3857) == "SRID=3857;POINT (1 2)"


def test_to_wkt_normalize_wkt_false_bytes_like_wkb_still_normalises():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    assert m.to_wkt(wkb, normalize_wkt=False) == "POINT (1 2)"


def test_to_wkt_bytes_like_wkb_invalid_raises_value_error():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.to_wkt(b"\x99")


def test_to_wkt_bytes_like_ascii_hex_is_raw_wkb_not_hex_text():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    with pytest.raises(ValueError, match="invalid WKB"):
        m.to_wkt(hex_wkb.encode("ascii"))


def test_to_wkt_bytes_like_wkb_rejects_non_contiguous_memoryview():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    with pytest.raises(BufferError, match="contiguous one-byte buffer"):
        m.to_wkt(memoryview(wkb)[::2])


def test_to_wkt_preserves_buffer_acquisition_errors():
    view = memoryview(bytearray(b"\x99"))
    view.release()
    with pytest.raises(ValueError, match="released memoryview"):
        m.to_wkt(view)


def test_to_wkt_accepts_source_keyword():
    assert m.to_wkt(source="POINT (1 2)") == "POINT (1 2)"


def test_to_wkt_srid_true_raises_value_error():
    with pytest.raises(ValueError, match="srid=True is not valid"):
        m.to_wkt("POINT (1 2)", srid=True)


# ── to_wkt: normalize_wkt parameter ─────────────────────────────────────

def test_to_wkt_normalize_wkt_false_is_default():
    assert m.to_wkt("point(1 2)") == m.to_wkt("point(1 2)", normalize_wkt=False)


def test_to_wkt_normalize_wkt_true_normalises_casing():
    assert m.to_wkt("point(1 2)", normalize_wkt=True) == "POINT (1 2)"


def test_to_wkt_normalize_wkt_false_returns_wkt_as_is():
    assert m.to_wkt("point(1 2)", normalize_wkt=False) == "point(1 2)"


def test_to_wkt_normalize_wkt_false_auto_preserves_srid_prefix():
    assert m.to_wkt("SRID=4326;POINT (1 2)", normalize_wkt=False) == "SRID=4326;POINT (1 2)"


def test_to_wkt_normalize_wkt_false_strip_removes_srid_prefix():
    assert m.to_wkt("SRID=4326;POINT (1 2)", srid=False, normalize_wkt=False) == "POINT (1 2)"


def test_to_wkt_normalize_wkt_false_strip_noop_when_no_srid():
    assert m.to_wkt("POINT (1 2)", srid=False, normalize_wkt=False) == "POINT (1 2)"


def test_to_wkt_normalize_wkt_false_set_adds_srid_prefix():
    assert m.to_wkt("POINT (1 2)", srid=4326, normalize_wkt=False) == "SRID=4326;POINT (1 2)"


def test_to_wkt_normalize_wkt_false_set_overrides_srid_prefix():
    assert m.to_wkt("SRID=4326;POINT (1 2)", srid=3857, normalize_wkt=False) == "SRID=3857;POINT (1 2)"


def test_to_wkt_normalize_wkt_false_hex_input_still_normalises():
    # normalize_wkt has no effect on hex WKB input; output is always normalised.
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    assert m.to_wkt(hex_wkb, normalize_wkt=False) == "POINT (1 2)"


# ── to_hex_wkb ───────────────────────────────────────────────────────────

def test_to_hex_wkb_from_wkt_returns_uppercase_str():
    result = m.to_hex_wkb("POINT (1 2)")
    assert isinstance(result, str)
    assert result == result.upper()


def test_to_hex_wkb_from_wkt_matches_wkt_to_hex_wkb():
    assert m.to_hex_wkb("POINT (1 2)") == m.wkt_to_hex_wkb("POINT (1 2)")


def test_to_hex_wkb_from_hex_wkb_is_identity():
    original = m.wkt_to_hex_wkb("LINESTRING (0 0, 1 1)")
    assert m.to_hex_wkb(original) == original


@pytest.mark.parametrize("make_input", [
    pytest.param(bytes, id="bytes"),
    pytest.param(bytearray, id="bytearray"),
    pytest.param(memoryview, id="memoryview"),
    pytest.param(lambda wkb: memoryview(b"\x00" + wkb)[1:], id="sliced-memoryview"),
    pytest.param(lambda wkb: array("B", wkb), id="unsigned-byte-array"),
])
def test_to_hex_wkb_from_bytes_like_wkb_returns_uppercase_hex(make_input):
    wkb = m.wkt_to_wkb("POINT (1 2)")
    assert m.to_hex_wkb(make_input(wkb)) == m.wkt_to_hex_wkb("POINT (1 2)")


def test_to_hex_wkb_srid_none_is_default():
    ewkt = "SRID=4326;POINT (1 2)"
    assert m.to_hex_wkb(ewkt) == m.to_hex_wkb(ewkt, srid=None)


def test_to_hex_wkb_srid_none_preserves_srid():
    result = m.to_hex_wkb("SRID=4326;POINT (1 2)", srid=None)
    assert m.hex_wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_hex_wkb_srid_false_strips_srid():
    result = m.to_hex_wkb("SRID=4326;POINT (1 2)", srid=False)
    assert m.hex_wkb_to_wkt(result) == "POINT (1 2)"


def test_to_hex_wkb_srid_false_strips_srid_from_bytes_like_wkb():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    result = m.to_hex_wkb(memoryview(wkb), srid=False)
    assert m.hex_wkb_to_wkt(result) == "POINT (1 2)"


def test_to_hex_wkb_srid_int_adds_srid_to_plain_wkt():
    result = m.to_hex_wkb("POINT (1 2)", srid=4326)
    assert m.hex_wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_hex_wkb_srid_int_adds_srid_to_bytes_like_wkb():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    result = m.to_hex_wkb(bytearray(wkb), srid=4326)
    assert m.hex_wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_hex_wkb_srid_int_overrides_srid():
    result = m.to_hex_wkb("SRID=4326;POINT (1 2)", srid=3857)
    assert m.hex_wkb_to_wkt(result) == "SRID=3857;POINT (1 2)"


def test_to_hex_wkb_srid_none_does_not_validate_bytes_like_wkb():
    assert m.to_hex_wkb(b"\x99") == "99"


def test_to_hex_wkb_bytes_like_ascii_hex_is_raw_wkb_not_hex_text():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    assert m.to_hex_wkb(hex_wkb.encode("ascii")) == hex_wkb.encode("ascii").hex().upper()


def test_to_hex_wkb_accepts_source_keyword():
    assert m.to_hex_wkb(source="POINT (1 2)") == m.wkt_to_hex_wkb("POINT (1 2)")


def test_to_hex_wkb_rejects_non_buffer_non_str_input():
    with pytest.raises(BufferError, match="contiguous one-byte buffer"):
        m.to_hex_wkb(123)


def test_to_hex_wkb_srid_true_raises_value_error():
    with pytest.raises(ValueError, match="srid=True is not valid"):
        m.to_hex_wkb("POINT (1 2)", srid=True)


def test_to_hex_wkb_invalid_raises_value_error():
    with pytest.raises(ValueError):
        m.to_hex_wkb("NOT_A_GEOMETRY (1 2)")


# ── wkb_header_srid ──────────────────────────────────────────────────────────

# Big-endian EWKB for SRID=4326;POINT (1 2)
# 00=BE, 20000001=POINT|SRID (BE), 000010E6=4326 (BE), 3FF0…=1.0, 4000…=2.0
_BE_SRID_POINT_HEX = "0020000001000010E63FF00000000000004000000000000000"
# Big-endian plain WKB for POINT (1 2)
_BE_POINT_HEX = "00000000013FF00000000000004000000000000000"


def test_wkb_header_srid_returns_int_from_binary():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    assert m.wkb_header_srid(wkb) == 4326


def test_wkb_header_srid_returns_none_when_no_srid():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    assert m.wkb_header_srid(wkb) is None


def test_wkb_header_srid_accepts_hex_string():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    assert m.wkb_header_srid(hex_wkb) == 4326


def test_wkb_header_srid_hex_string_returns_none_when_no_srid():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    assert m.wkb_header_srid(hex_wkb) is None


def test_wkb_header_srid_big_endian_fast_path():
    assert m.wkb_header_srid(bytes.fromhex(_BE_SRID_POINT_HEX)) == 4326


def test_wkb_header_srid_big_endian_no_srid():
    assert m.wkb_header_srid(bytes.fromhex(_BE_POINT_HEX)) is None


def test_wkb_header_srid_big_endian_hex_string():
    assert m.wkb_header_srid(_BE_SRID_POINT_HEX) == 4326


def test_wkb_header_srid_z_geometry():
    wkb = m.wkt_to_wkb("SRID=4326;POINT Z (1 2 3)")
    assert m.wkb_header_srid(wkb) == 4326


def test_wkb_header_srid_multipolygon():
    wkb = m.wkt_to_wkb("SRID=4326;MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))")
    assert m.wkb_header_srid(wkb) == 4326


def test_wkb_header_srid_geometry_collection():
    wkb = m.wkt_to_wkb("SRID=4326;GEOMETRYCOLLECTION (POINT (1 2))")
    assert m.wkb_header_srid(wkb) == 4326


@pytest.mark.parametrize("make_input", [
    pytest.param(bytearray, id="bytearray"),
    pytest.param(memoryview, id="memoryview"),
    pytest.param(lambda wkb: array("B", wkb), id="unsigned-byte-array"),
])
def test_wkb_header_srid_accepts_bytes_like(make_input):
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    assert m.wkb_header_srid(make_input(wkb)) == 4326


def test_wkb_header_srid_iso_point_z_no_srid():
    import struct
    # ISO WKB type code 1001 (POINT Z, no SRID flag) — falls back to full parse
    wkb = struct.pack("<BIddd", 1, 1001, 1.0, 2.0, 3.0)
    assert m.wkb_header_srid(wkb) is None


def test_wkb_header_srid_truncated_srid_field_errors():
    # EWKB with SRID flag but truncated (only 5 bytes, no room for SRID)
    assert pytest.raises(ValueError, m.wkb_header_srid, bytes.fromhex("0103000020"))


def test_wkb_header_srid_empty_bytes_errors():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.wkb_header_srid(b"")


def test_wkb_header_srid_invalid_hex_string_errors():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.wkb_header_srid("ZZ")


def test_wkb_header_srid_rejects_non_buffer_input():
    with pytest.raises(BufferError, match="contiguous one-byte buffer"):
        m.wkb_header_srid(123)


# ── to_wkb_no_srid_header ────────────────────────────────────────────────────

def test_to_wkb_no_srid_header_binary_returns_bytes():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    result = m.to_wkb_no_srid_header(wkb)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_to_wkb_no_srid_header_hex_returns_str():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    result = m.to_wkb_no_srid_header(hex_wkb)
    assert isinstance(result, str)
    assert m.hex_wkb_to_wkt(result) == "POINT (1 2)"


def test_to_wkb_no_srid_header_hex_output_is_uppercase():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    result = m.to_wkb_no_srid_header(hex_wkb)
    assert result == result.upper()


def test_to_wkb_no_srid_header_noop_when_no_srid_binary():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    result = m.to_wkb_no_srid_header(wkb)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_to_wkb_no_srid_header_noop_when_no_srid_hex():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    result = m.to_wkb_no_srid_header(hex_wkb)
    assert isinstance(result, str)
    assert m.hex_wkb_to_wkt(result) == "POINT (1 2)"


def test_to_wkb_no_srid_header_big_endian_binary():
    be = bytes.fromhex(_BE_SRID_POINT_HEX)
    result = m.to_wkb_no_srid_header(be)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_to_wkb_no_srid_header_big_endian_hex():
    result = m.to_wkb_no_srid_header(_BE_SRID_POINT_HEX)
    assert isinstance(result, str)
    assert m.hex_wkb_to_wkt(result) == "POINT (1 2)"


def test_to_wkb_no_srid_header_multipolygon_binary():
    wkb = m.wkt_to_wkb("SRID=4326;MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))")
    result = m.to_wkb_no_srid_header(wkb)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))"


def test_to_wkb_no_srid_header_geometry_collection_binary():
    wkb = m.wkt_to_wkb("SRID=4326;GEOMETRYCOLLECTION (POINT (1 2), LINESTRING (0 0, 1 1))")
    result = m.to_wkb_no_srid_header(wkb)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "GEOMETRYCOLLECTION (POINT (1 2), LINESTRING (0 0, 1 1))"


@pytest.mark.parametrize("make_input", [
    pytest.param(bytearray, id="bytearray"),
    pytest.param(memoryview, id="memoryview"),
    pytest.param(lambda wkb: array("B", wkb), id="unsigned-byte-array"),
])
def test_to_wkb_no_srid_header_accepts_bytes_like(make_input):
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    result = m.to_wkb_no_srid_header(make_input(wkb))
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_to_wkb_no_srid_header_iso_point_z_binary():
    import struct
    wkb = struct.pack("<BIddd", 1, 1001, 1.0, 2.0, 3.0)
    result = m.to_wkb_no_srid_header(wkb)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "POINT Z (1 2 3)"


def test_to_wkb_no_srid_header_empty_bytes_errors():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.to_wkb_no_srid_header(b"")


def test_to_wkb_no_srid_header_invalid_hex_errors():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.to_wkb_no_srid_header("ZZ")


def test_to_wkb_no_srid_header_rejects_non_buffer_non_str():
    with pytest.raises(BufferError, match="contiguous one-byte buffer"):
        m.to_wkb_no_srid_header(123)


# ── to_ewkb_header ───────────────────────────────────────────────────────────

def test_to_ewkb_header_binary_adds_srid():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    result = m.to_ewkb_header(wkb, 4326)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_ewkb_header_hex_adds_srid():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    result = m.to_ewkb_header(hex_wkb, 4326)
    assert isinstance(result, str)
    assert m.hex_wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_ewkb_header_hex_output_is_uppercase():
    hex_wkb = m.wkt_to_hex_wkb("POINT (1 2)")
    result = m.to_ewkb_header(hex_wkb, 4326)
    assert result == result.upper()


def test_to_ewkb_header_binary_replaces_existing_srid():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    result = m.to_ewkb_header(wkb, 3857)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "SRID=3857;POINT (1 2)"


def test_to_ewkb_header_hex_replaces_existing_srid():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    result = m.to_ewkb_header(hex_wkb, 3857)
    assert isinstance(result, str)
    assert m.hex_wkb_to_wkt(result) == "SRID=3857;POINT (1 2)"


def test_to_ewkb_header_hex_false_strips_srid():
    hex_wkb = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    result = m.to_ewkb_header(hex_wkb, False)
    assert isinstance(result, str)
    assert m.hex_wkb_to_wkt(result) == "POINT (1 2)"


def test_to_ewkb_header_does_not_double_srid():
    # Replacing SRID must not embed a second SRID field.
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    result = m.to_ewkb_header(wkb, 3857)
    # Round-trip via wkb_to_wkt_split_srid must return exactly one SRID.
    wkt, srid = m.wkb_to_wkt_split_srid(result)
    assert srid == 3857
    assert wkt == "POINT (1 2)"


def test_to_ewkb_header_noop_when_srid_already_matches():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    result = m.to_ewkb_header(wkb, 4326)
    assert isinstance(result, bytes)
    assert result == wkb


def test_to_ewkb_header_srid_zero_strips_srid():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    result = m.to_ewkb_header(wkb, 0)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_to_ewkb_header_srid_negative_strips_srid():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    result = m.to_ewkb_header(wkb, -1)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "POINT (1 2)"


def test_to_ewkb_header_srid_false_strips_and_matches_helper():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    result = m.to_ewkb_header(wkb, False)
    assert isinstance(result, bytes)
    assert result == m.to_wkb_no_srid_header(wkb)


def test_to_ewkb_header_srid_true_raises_value_error():
    with pytest.raises(ValueError, match="srid=True is not valid"):
        m.to_ewkb_header(m.wkt_to_wkb("POINT (1 2)"), True)


def test_to_ewkb_header_srid_huge_int_raises_range_value_error():
    with pytest.raises(ValueError, match="32-bit"):
        m.to_ewkb_header(m.wkt_to_wkb("POINT (1 2)"), 2**80)


def test_to_ewkb_header_srid_index_success_calls_index_once():
    class CountingIndex:
        def __init__(self):
            self.calls = 0

        def __index__(self):
            self.calls += 1
            return 4326

    srid = CountingIndex()
    wkb = m.wkt_to_wkb("POINT (1 2)")
    result = m.to_ewkb_header(wkb, srid)

    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"
    assert srid.calls == 1


def test_to_ewkb_header_big_endian_binary():
    be = bytes.fromhex(_BE_POINT_HEX)
    result = m.to_ewkb_header(be, 4326)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_ewkb_header_big_endian_hex():
    result = m.to_ewkb_header(_BE_POINT_HEX, 4326)
    assert isinstance(result, str)
    assert m.hex_wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_ewkb_header_multipolygon_binary():
    wkb = m.wkt_to_wkb("MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))")
    result = m.to_ewkb_header(wkb, 4326)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "SRID=4326;MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)))"


def test_to_ewkb_header_geometry_collection_binary():
    wkb = m.wkt_to_wkb("GEOMETRYCOLLECTION (POINT (1 2), LINESTRING (0 0, 1 1))")
    result = m.to_ewkb_header(wkb, 4326)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "SRID=4326;GEOMETRYCOLLECTION (POINT (1 2), LINESTRING (0 0, 1 1))"


@pytest.mark.parametrize("make_input", [
    pytest.param(bytearray, id="bytearray"),
    pytest.param(memoryview, id="memoryview"),
    pytest.param(lambda wkb: array("B", wkb), id="unsigned-byte-array"),
])
def test_to_ewkb_header_accepts_bytes_like(make_input):
    wkb = m.wkt_to_wkb("POINT (1 2)")
    result = m.to_ewkb_header(make_input(wkb), 4326)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT (1 2)"


def test_to_ewkb_header_iso_point_z_binary():
    import struct
    wkb = struct.pack("<BIddd", 1, 1001, 1.0, 2.0, 3.0)
    result = m.to_ewkb_header(wkb, 4326)
    assert isinstance(result, bytes)
    assert m.wkb_to_wkt(result) == "SRID=4326;POINT Z (1 2 3)"


def test_to_ewkb_header_empty_bytes_errors():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.to_ewkb_header(b"", 4326)


def test_to_ewkb_header_invalid_hex_errors():
    with pytest.raises(ValueError, match="invalid WKB"):
        m.to_ewkb_header("ZZ", 4326)


def test_to_ewkb_header_rejects_non_buffer_non_str():
    with pytest.raises(BufferError, match="contiguous one-byte buffer"):
        m.to_ewkb_header(123, 4326)


# ── cross-function consistency ────────────────────────────────────────────────

def test_header_srid_consistent_with_to_wkb_no_srid_header():
    wkb = m.wkt_to_wkb("SRID=4326;POINT (1 2)")
    assert m.wkb_header_srid(wkb) == 4326
    stripped = m.to_wkb_no_srid_header(wkb)
    assert m.wkb_header_srid(stripped) is None


def test_to_ewkb_header_then_wkb_header_srid_round_trips():
    wkb = m.wkt_to_wkb("POINT (1 2)")
    with_srid = m.to_ewkb_header(wkb, 4326)
    assert m.wkb_header_srid(with_srid) == 4326


def test_strip_then_set_hex_round_trip():
    original_hex = m.wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
    stripped_hex = m.to_wkb_no_srid_header(original_hex)
    restored_hex = m.to_ewkb_header(stripped_hex, 4326)
    assert m.hex_wkb_to_wkt(restored_hex) == "SRID=4326;POINT (1 2)"


def test_helpers_semantically_equivalent_to_existing_api():
    # wkb_header_srid == wkb_to_wkt_split_srid SRID component
    wkb = m.wkt_to_wkb("SRID=4326;LINESTRING (0 0, 1 1)")
    _, srid = m.wkb_to_wkt_split_srid(wkb)
    assert m.wkb_header_srid(wkb) == srid

    # to_wkb_no_srid_header == to_wkb(source, srid=False)
    stripped = m.to_wkb_no_srid_header(wkb)
    assert stripped == m.to_wkb(wkb, srid=False)

    # to_ewkb_header(source, 3857) == to_wkb(source, srid=3857)
    with_srid = m.to_ewkb_header(wkb, 3857)
    assert with_srid == m.to_wkb(wkb, srid=3857)
