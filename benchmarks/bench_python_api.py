"""
Benchmark wkb_wkt_converter Python API vs shapely.

Run with:
    python -m pytest benchmarks/bench_python_api.py -v
    python -m pytest benchmarks/bench_python_api.py -k "wkt_to_wkb"  # filter by operation
    python -m pytest benchmarks/bench_python_api.py --benchmark-save=baseline
    python -m pytest benchmarks/bench_python_api.py --benchmark-compare=baseline

shapely is an optional dependency for comparison; its tests are skipped if missing.
"""
import math

import pytest
import wkb_wkt_converter as conv

try:
    from shapely import from_wkb, from_wkt, to_wkb, to_wkt

    HAS_SHAPELY = True
except ImportError:
    HAS_SHAPELY = False

shapely_required = pytest.mark.skipif(not HAS_SHAPELY, reason="shapely not installed")


# ---------------------------------------------------------------------------
# Test geometry definitions
# ---------------------------------------------------------------------------

def _linestring_wkt(n: int) -> str:
    return "LINESTRING (" + ", ".join(f"{i} {i * 0.5}" for i in range(n)) + ")"


def _circle_polygon_wkt(n: int) -> str:
    coords = [
        f"{math.cos(2 * math.pi * i / n)} {math.sin(2 * math.pi * i / n)}"
        for i in range(n)
    ]
    coords.append(coords[0])
    return f"POLYGON (({', '.join(coords)}))"


GEOMETRIES_WKT = {
    "point": "POINT (1.5 2.5)",
    "linestring_5pts": _linestring_wkt(5),
    "linestring_1000pts": _linestring_wkt(1000),
    "polygon_5pts": "POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))",
    "polygon_1000pts": _circle_polygon_wkt(1000),
    "multipolygon": (
        "MULTIPOLYGON ("
        "((0 0, 1 0, 1 1, 0 1, 0 0), (0.2 0.2, 0.8 0.2, 0.8 0.8, 0.2 0.8, 0.2 0.2)), "
        "((2 2, 3 2, 3 3, 2 3, 2 2)))"
    ),
    "geometry_collection": (
        "GEOMETRYCOLLECTION ("
        "POINT (0 0), "
        "LINESTRING (0 0, 1 1), "
        "POLYGON ((0 0, 1 0, 1 1, 0 0)))"
    ),
}

# Pre-compute WKB/hex versions used as inputs for wkb→wkt and hex→wkt benchmarks
GEOMETRIES_WKB = {name: conv.wkt_to_wkb(wkt) for name, wkt in GEOMETRIES_WKT.items()}
GEOMETRIES_HEX_WKB = {name: conv.wkt_to_hex_wkb(wkt) for name, wkt in GEOMETRIES_WKT.items()}

_WKT_PARAMS = pytest.mark.parametrize("name", list(GEOMETRIES_WKT), ids=list(GEOMETRIES_WKT))
_WKB_PARAMS = pytest.mark.parametrize("name", list(GEOMETRIES_WKB), ids=list(GEOMETRIES_WKB))


# ---------------------------------------------------------------------------
# wkt_to_wkb
# ---------------------------------------------------------------------------

@_WKT_PARAMS
def test_conv_wkt_to_wkb(benchmark, name):
    benchmark(conv.wkt_to_wkb, GEOMETRIES_WKT[name])


@shapely_required
@_WKT_PARAMS
def test_shapely_wkt_to_wkb(benchmark, name):
    wkt = GEOMETRIES_WKT[name]
    benchmark(lambda: to_wkb(from_wkt(wkt)))


# ---------------------------------------------------------------------------
# wkb_to_wkt
# ---------------------------------------------------------------------------

@_WKB_PARAMS
def test_conv_wkb_to_wkt(benchmark, name):
    benchmark(conv.wkb_to_wkt, GEOMETRIES_WKB[name])


@shapely_required
@_WKB_PARAMS
def test_shapely_wkb_to_wkt(benchmark, name):
    wkb = GEOMETRIES_WKB[name]
    benchmark(lambda: to_wkt(from_wkb(wkb)))


# ---------------------------------------------------------------------------
# wkt_to_hex_wkb
# ---------------------------------------------------------------------------

@_WKT_PARAMS
def test_conv_wkt_to_hex_wkb(benchmark, name):
    benchmark(conv.wkt_to_hex_wkb, GEOMETRIES_WKT[name])


@shapely_required
@_WKT_PARAMS
def test_shapely_wkt_to_hex_wkb(benchmark, name):
    wkt = GEOMETRIES_WKT[name]
    benchmark(lambda: to_wkb(from_wkt(wkt)).hex().upper())


# ---------------------------------------------------------------------------
# hex_wkb_to_wkt
# ---------------------------------------------------------------------------

@_WKB_PARAMS
def test_conv_hex_wkb_to_wkt(benchmark, name):
    benchmark(conv.hex_wkb_to_wkt, GEOMETRIES_HEX_WKB[name])


@shapely_required
@_WKB_PARAMS
def test_shapely_hex_wkb_to_wkt(benchmark, name):
    hex_wkb = GEOMETRIES_HEX_WKB[name]
    benchmark(lambda: to_wkt(from_wkb(bytes.fromhex(hex_wkb))))


# ---------------------------------------------------------------------------
# text_to_wkb — WKT input path
# ---------------------------------------------------------------------------

@_WKT_PARAMS
def test_conv_text_to_wkb_from_wkt(benchmark, name):
    benchmark(conv.text_to_wkb, GEOMETRIES_WKT[name])


@shapely_required
@_WKT_PARAMS
def test_shapely_text_to_wkb_from_wkt(benchmark, name):
    wkt = GEOMETRIES_WKT[name]
    benchmark(lambda: to_wkb(from_wkt(wkt)))


# ---------------------------------------------------------------------------
# text_to_wkb — hex WKB input path
# ---------------------------------------------------------------------------

@_WKB_PARAMS
def test_conv_text_to_wkb_from_hex(benchmark, name):
    benchmark(conv.text_to_wkb, GEOMETRIES_HEX_WKB[name])


@shapely_required
@_WKB_PARAMS
def test_shapely_text_to_wkb_from_hex(benchmark, name):
    hex_wkb = GEOMETRIES_HEX_WKB[name]
    benchmark(lambda: to_wkb(from_wkb(bytes.fromhex(hex_wkb))))


# ---------------------------------------------------------------------------
# text_to_wkt — WKT input, normalize_wkt=False (fast path: no WKB round-trip)
# Shapely's nearest functional equivalent is WKT parse+serialize; it is shared
# with the normalize_wkt=True comparison below.
# ---------------------------------------------------------------------------

@_WKT_PARAMS
def test_conv_text_to_wkt_from_wkt_no_normalize(benchmark, name):
    benchmark(conv.text_to_wkt, GEOMETRIES_WKT[name])


# ---------------------------------------------------------------------------
# text_to_wkt — WKT input, normalize_wkt=True (full WKT→WKB→WKT round-trip)
# ---------------------------------------------------------------------------

@_WKT_PARAMS
def test_conv_text_to_wkt_from_wkt_normalize(benchmark, name):
    wkt = GEOMETRIES_WKT[name]
    benchmark(lambda: conv.text_to_wkt(wkt, normalize_wkt=True))


@shapely_required
@_WKT_PARAMS
def test_shapely_text_to_wkt_from_wkt(benchmark, name):
    wkt = GEOMETRIES_WKT[name]
    benchmark(lambda: to_wkt(from_wkt(wkt)))


# ---------------------------------------------------------------------------
# text_to_wkt — hex WKB input path (always full conversion)
# ---------------------------------------------------------------------------

@_WKB_PARAMS
def test_conv_text_to_wkt_from_hex(benchmark, name):
    benchmark(conv.text_to_wkt, GEOMETRIES_HEX_WKB[name])


@shapely_required
@_WKB_PARAMS
def test_shapely_text_to_wkt_from_hex(benchmark, name):
    hex_wkb = GEOMETRIES_HEX_WKB[name]
    benchmark(lambda: to_wkt(from_wkb(bytes.fromhex(hex_wkb))))


# ---------------------------------------------------------------------------
# text_to_hex_wkb — WKT input path
# ---------------------------------------------------------------------------

@_WKT_PARAMS
def test_conv_text_to_hex_wkb_from_wkt(benchmark, name):
    benchmark(conv.text_to_hex_wkb, GEOMETRIES_WKT[name])


@shapely_required
@_WKT_PARAMS
def test_shapely_text_to_hex_wkb_from_wkt(benchmark, name):
    wkt = GEOMETRIES_WKT[name]
    benchmark(lambda: to_wkb(from_wkt(wkt)).hex().upper())


# ---------------------------------------------------------------------------
# text_to_hex_wkb — hex WKB input path
# ---------------------------------------------------------------------------

@_WKB_PARAMS
def test_conv_text_to_hex_wkb_from_hex(benchmark, name):
    benchmark(conv.text_to_hex_wkb, GEOMETRIES_HEX_WKB[name])


@shapely_required
@_WKB_PARAMS
def test_shapely_text_to_hex_wkb_from_hex(benchmark, name):
    hex_wkb = GEOMETRIES_HEX_WKB[name]
    benchmark(lambda: to_wkb(from_wkb(bytes.fromhex(hex_wkb))).hex().upper())
