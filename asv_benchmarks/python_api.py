"""Converter-only ASV benchmarks for the Python API."""

import math
import struct


def _linestring_wkt(n):
    return "LINESTRING (" + ", ".join(f"{i} {i * 0.5}" for i in range(n)) + ")"


def _circle_polygon_wkt(n):
    coords = [
        f"{math.cos(2 * math.pi * i / n)} {math.sin(2 * math.pi * i / n)}"
        for i in range(n)
    ]
    coords.append(coords[0])
    return f"POLYGON (({', '.join(coords)}))"


def _circle_polygon_coords(n):
    coords = [
        (math.cos(2 * math.pi * i / n), math.sin(2 * math.pi * i / n))
        for i in range(n)
    ]
    coords.append(coords[0])
    return coords


def _wkb_header(geom_type):
    return struct.pack("<BI", 1, geom_type)


def _wkb_points(coords):
    return b"".join(struct.pack("<dd", x, y) for x, y in coords)


def _wkb_point(coord):
    return _wkb_header(1) + struct.pack("<dd", *coord)


def _wkb_linestring(coords):
    return _wkb_header(2) + struct.pack("<I", len(coords)) + _wkb_points(coords)


def _wkb_polygon(rings):
    data = [_wkb_header(3), struct.pack("<I", len(rings))]
    for ring in rings:
        data.append(struct.pack("<I", len(ring)))
        data.append(_wkb_points(ring))
    return b"".join(data)


def _wkb_multipolygon(polygons):
    return _wkb_header(6) + struct.pack("<I", len(polygons)) + b"".join(
        _wkb_polygon(polygon) for polygon in polygons
    )


def _wkb_geometry_collection(geometries):
    return _wkb_header(7) + struct.pack("<I", len(geometries)) + b"".join(geometries)


LINESTRING_5PTS_COORDS = [(i, i * 0.5) for i in range(5)]
LINESTRING_1000PTS_COORDS = [(i, i * 0.5) for i in range(1000)]
POLYGON_5PTS_COORDS = [(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)]
POLYGON_1000PTS_COORDS = _circle_polygon_coords(1000)
MULTIPOLYGON_COORDS = [
    [
        POLYGON_5PTS_COORDS,
        [(0.2, 0.2), (0.8, 0.2), (0.8, 0.8), (0.2, 0.8), (0.2, 0.2)],
    ],
    [[(2, 2), (3, 2), (3, 3), (2, 3), (2, 2)]],
]
GEOMETRY_COLLECTION_WKB = _wkb_geometry_collection(
    [
        _wkb_point((0, 0)),
        _wkb_linestring([(0, 0), (1, 1)]),
        _wkb_polygon([[(0, 0), (1, 0), (1, 1), (0, 0)]]),
    ]
)


GEOMETRIES_WKT = {
    "point": "POINT (1.5 2.5)",
    "linestring_5pts": _linestring_wkt(5),
    "linestring_1000pts": _linestring_wkt(1000),
    "polygon_5pts": "POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))",
    "polygon_1000pts": _circle_polygon_wkt(1000),
    "multipolygon": (
        "MULTIPOLYGON ("
        "((0 0, 1 0, 1 1, 0 1, 0 0), "
        "(0.2 0.2, 0.8 0.2, 0.8 0.8, 0.2 0.8, 0.2 0.2)), "
        "((2 2, 3 2, 3 3, 2 3, 2 2)))"
    ),
    "geometry_collection": (
        "GEOMETRYCOLLECTION ("
        "POINT (0 0), "
        "LINESTRING (0 0, 1 1), "
        "POLYGON ((0 0, 1 0, 1 1, 0 0)))"
    ),
}

GEOMETRIES_WKB = {
    "point": _wkb_point((1.5, 2.5)),
    "linestring_5pts": _wkb_linestring(LINESTRING_5PTS_COORDS),
    "linestring_1000pts": _wkb_linestring(LINESTRING_1000PTS_COORDS),
    "polygon_5pts": _wkb_polygon([POLYGON_5PTS_COORDS]),
    "polygon_1000pts": _wkb_polygon([POLYGON_1000PTS_COORDS]),
    "multipolygon": _wkb_multipolygon(MULTIPOLYGON_COORDS),
    "geometry_collection": GEOMETRY_COLLECTION_WKB,
}

GEOMETRIES_HEX_WKB = {
    name: wkb.hex().upper()
    for name, wkb in GEOMETRIES_WKB.items()
}


class PythonApiSuite:
    params = [list(GEOMETRIES_WKT)]
    param_names = ["geometry"]

    def setup(self, geometry):
        import wkb_wkt_converter as conv

        self.conv = conv
        self.wkt = GEOMETRIES_WKT[geometry]
        self.wkb = GEOMETRIES_WKB[geometry]
        self.hex_wkb = GEOMETRIES_HEX_WKB[geometry]

    def time_wkt_to_wkb(self, geometry):
        self.conv.wkt_to_wkb(self.wkt)

    def time_wkb_to_wkt(self, geometry):
        self.conv.wkb_to_wkt(self.wkb)

    def time_wkt_to_hex_wkb(self, geometry):
        self.conv.wkt_to_hex_wkb(self.wkt)

    def time_hex_wkb_to_wkt(self, geometry):
        self.conv.hex_wkb_to_wkt(self.hex_wkb)

    def time_text_to_wkb_from_wkt(self, geometry):
        self.conv.text_to_wkb(self.wkt)

    def time_text_to_wkb_from_hex(self, geometry):
        self.conv.text_to_wkb(self.hex_wkb)

    def time_text_to_wkt_from_wkt_no_normalize(self, geometry):
        self.conv.text_to_wkt(self.wkt)

    def time_text_to_wkt_from_wkt_normalize(self, geometry):
        self.conv.text_to_wkt(self.wkt, normalize_wkt=True)

    def time_text_to_wkt_from_hex(self, geometry):
        self.conv.text_to_wkt(self.hex_wkb)

    def time_text_to_hex_wkb_from_wkt(self, geometry):
        self.conv.text_to_hex_wkb(self.wkt)

    def time_text_to_hex_wkb_from_hex(self, geometry):
        self.conv.text_to_hex_wkb(self.hex_wkb)
