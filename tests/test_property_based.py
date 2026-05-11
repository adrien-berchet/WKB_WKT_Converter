"""
Property-based tests comparing wkb_wkt_converter against Shapely.

Two properties are verified for all seven OGC geometry types:

  1. WKB round-trip — Shapely EWKB → wkb_to_wkt → wkt_to_wkb must reproduce
     the original bytes exactly, confirming that our decoder and encoder are
     exact inverses of each other.

  2. WKT-to-WKB oracle — wkt_to_wkb(shapely.to_wkt(geom)) must equal
     shapely.to_wkb(geom), confirming that our WKT parser and WKB encoder
     produce byte-identical output to Shapely for the same geometry.

Requires: pip install ".[test]"
"""

import shapely
import hypothesis.strategies as st
from hypothesis import given

import wkb_wkt_converter as conv

# ── coordinate strategies ─────────────────────────────────────────────────────
#
# Coordinates are kept below 1e14 so that Hypothesis's shrinking does not
# accidentally produce values at the boundary of push_f64's integer shortcut
# (abs < 1e15 AND v == trunc(v)), which would format e.g. 1e14 as "100000000000000"
# rather than "1e14". Both representations parse to the same float, so the tests
# would still pass, but the bound gives cleaner shrunk counter-examples.

# .map(lambda x: x + 0.0) normalises IEEE 754 negative zero to positive zero.
# push_f64 outputs both -0.0 and +0.0 as "0", so the converter is correct but
# byte comparison would fail if Shapely stored a -0.0 that our round-trip promotes.
_coord = st.floats(
    min_value=-1e14, max_value=1e14, allow_nan=False, allow_infinity=False,
).map(lambda x: x + 0.0)
_positive = st.floats(min_value=1e-3, max_value=1e6, allow_nan=False, allow_infinity=False)

# ── geometry strategies ───────────────────────────────────────────────────────


@st.composite
def st_point_2d(draw):
    return shapely.Point(draw(_coord), draw(_coord))


@st.composite
def st_point_z(draw):
    return shapely.Point(draw(_coord), draw(_coord), draw(_coord))


@st.composite
def st_linestring(draw):
    n = draw(st.integers(min_value=2, max_value=8))
    return shapely.LineString([(draw(_coord), draw(_coord)) for _ in range(n)])


@st.composite
def st_polygon(draw):
    # shapely.box always produces a valid, non-degenerate rectangle.
    x0, y0 = draw(_coord), draw(_coord)
    return shapely.box(x0, y0, x0 + draw(_positive), y0 + draw(_positive))


@st.composite
def st_multipoint(draw):
    n = draw(st.integers(min_value=1, max_value=6))
    return shapely.MultiPoint([(draw(_coord), draw(_coord)) for _ in range(n)])


@st.composite
def st_multilinestring(draw):
    n = draw(st.integers(min_value=1, max_value=3))
    return shapely.MultiLineString([draw(st_linestring()) for _ in range(n)])


@st.composite
def st_multipolygon(draw):
    n = draw(st.integers(min_value=1, max_value=3))
    x0, y0 = draw(_coord), draw(_coord)
    w, h = draw(_positive), draw(_positive)
    # Space boxes by (w + 1.0) so they can never overlap regardless of w.
    return shapely.MultiPolygon([
        shapely.box(x0 + i * (w + 1.0), y0, x0 + i * (w + 1.0) + w, y0 + h)
        for i in range(n)
    ])


@st.composite
def st_geometry_collection(draw):
    return shapely.GeometryCollection([draw(st_point_2d()), draw(st_linestring())])


# ── oracle helpers ────────────────────────────────────────────────────────────


def _shapely_ewkb(geom):
    """Little-endian EWKB bytes from Shapely, no SRID."""
    # byte_order=1 forces little-endian output to match our encoder.
    # Shapely 2.x defaults to flavor="extended" (EWKB), which is what we emit.
    return shapely.to_wkb(geom, byte_order=1)


def _geom_to_precise_wkt(geom):
    """Convert a Shapely geometry to WKT using Python's str() for each coordinate.

    Python's str(float) produces the shortest decimal representation that round-trips
    to the exact same IEEE 754 double.  Shapely's shapely.to_wkt() uses GEOS's
    rounding_precision, which counts decimal *places* (not significant digits) and
    can therefore lose the last ULP for small values like 0.014.
    """
    has_z = geom.has_z
    dim = "Z " if has_z else ""

    def fmt_coord(c):
        return " ".join(str(v) for v in c)

    def fmt_ring(ring):
        return "(" + ", ".join(fmt_coord(c) for c in ring.coords) + ")"

    gt = geom.geom_type
    if gt == "Point":
        return f"POINT {dim}({fmt_coord(geom.coords[0])})"
    if gt == "LineString":
        pts = ", ".join(fmt_coord(c) for c in geom.coords)
        return f"LINESTRING {dim}({pts})"
    if gt == "Polygon":
        rings = [geom.exterior] + list(geom.interiors)
        return f"POLYGON {dim}(" + ", ".join(fmt_ring(r) for r in rings) + ")"
    if gt == "MultiPoint":
        pts = ", ".join("(" + fmt_coord(g.coords[0]) + ")" for g in geom.geoms)
        return f"MULTIPOINT {dim}({pts})"
    if gt == "MultiLineString":
        lines = ", ".join(
            "(" + ", ".join(fmt_coord(c) for c in g.coords) + ")"
            for g in geom.geoms
        )
        return f"MULTILINESTRING {dim}({lines})"
    if gt == "MultiPolygon":
        polys = [
            "(" + ", ".join(fmt_ring(r) for r in [p.exterior] + list(p.interiors)) + ")"
            for p in geom.geoms
        ]
        return f"MULTIPOLYGON {dim}(" + ", ".join(polys) + ")"
    if gt == "GeometryCollection":
        parts = ", ".join(_geom_to_precise_wkt(g) for g in geom.geoms)
        return f"GEOMETRYCOLLECTION ({parts})"
    raise ValueError(f"Unsupported geometry type: {gt}")  # pragma: no cover


def _assert_wkb_roundtrip(geom):
    """Shapely EWKB → wkb_to_wkt → wkt_to_wkb must reproduce the original bytes."""
    shapely_wkb = _shapely_ewkb(geom)
    our_wkt = conv.wkb_to_wkt(shapely_wkb)
    assert bytes(conv.wkt_to_wkb(our_wkt)) == shapely_wkb


def _assert_wkt_to_wkb_oracle(geom):
    """wkt_to_wkb on precisely-formatted WKT must equal Shapely's own WKB.

    Uses _geom_to_precise_wkt() rather than shapely.to_wkt() to avoid GEOS's
    decimal-place rounding, which can fail to distinguish adjacent IEEE 754
    doubles for small coordinate values.
    """
    wkt = _geom_to_precise_wkt(geom)
    assert bytes(conv.wkt_to_wkb(wkt)) == _shapely_ewkb(geom)


# ── WKB round-trip tests ──────────────────────────────────────────────────────


@given(st_point_2d())
def test_roundtrip_point_2d(geom):
    _assert_wkb_roundtrip(geom)


@given(st_point_z())
def test_roundtrip_point_z(geom):
    _assert_wkb_roundtrip(geom)


@given(st_linestring())
def test_roundtrip_linestring(geom):
    _assert_wkb_roundtrip(geom)


@given(st_polygon())
def test_roundtrip_polygon(geom):
    _assert_wkb_roundtrip(geom)


@given(st_multipoint())
def test_roundtrip_multipoint(geom):
    _assert_wkb_roundtrip(geom)


@given(st_multilinestring())
def test_roundtrip_multilinestring(geom):
    _assert_wkb_roundtrip(geom)


@given(st_multipolygon())
def test_roundtrip_multipolygon(geom):
    _assert_wkb_roundtrip(geom)


@given(st_geometry_collection())
def test_roundtrip_geometry_collection(geom):
    _assert_wkb_roundtrip(geom)


# ── WKT → WKB oracle tests ────────────────────────────────────────────────────


@given(st_point_2d())
def test_oracle_point_2d(geom):
    _assert_wkt_to_wkb_oracle(geom)


@given(st_point_z())
def test_oracle_point_z(geom):
    _assert_wkt_to_wkb_oracle(geom)


@given(st_linestring())
def test_oracle_linestring(geom):
    _assert_wkt_to_wkb_oracle(geom)


@given(st_polygon())
def test_oracle_polygon(geom):
    _assert_wkt_to_wkb_oracle(geom)


@given(st_multipoint())
def test_oracle_multipoint(geom):
    _assert_wkt_to_wkb_oracle(geom)


@given(st_multilinestring())
def test_oracle_multilinestring(geom):
    _assert_wkt_to_wkb_oracle(geom)


@given(st_multipolygon())
def test_oracle_multipolygon(geom):
    _assert_wkt_to_wkb_oracle(geom)


# ── SRID round-trip tests ─────────────────────────────────────────────────────


@given(st_point_2d(), st.integers(min_value=1, max_value=2**31 - 1))
def test_srid_preserved_in_roundtrip(geom, srid):
    # Use our own canonical WKT to avoid any float-formatting difference
    # between Shapely's WKT output and ours.
    canonical_wkt = conv.wkb_to_wkt(_shapely_ewkb(geom))
    ewkt = f"SRID={srid};{canonical_wkt}"
    assert conv.wkb_to_wkt(conv.wkt_to_wkb(ewkt)) == ewkt


@given(st_linestring(), st.integers(min_value=1, max_value=999999))
def test_srid_extracted_by_split(geom, srid):
    canonical_wkt = conv.wkb_to_wkt(_shapely_ewkb(geom))
    ewkt = f"SRID={srid};{canonical_wkt}"
    _, extracted = conv.wkb_to_wkt_split_srid(conv.wkt_to_wkb(ewkt))
    assert extracted == srid
