#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;
use wkb_wkt_converter::{
    hex_wkb_to_wkt, to_hex_wkb, to_wkb, to_wkt, wkb_header_srid, wkb_to_wkt, wkb_to_wkt_split_srid,
    wkt_to_hex_wkb, wkt_to_wkb, wkt_to_wkb_split_srid, Input, SridMode,
};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let mut cursor = Cursor::new(data);
    let wkt = cursor.geometry_with_optional_srid();

    let wkb = wkt_to_wkb(&wkt, SridMode::Auto).expect("generated WKT must encode");
    let canonical = wkb_to_wkt(&wkb, SridMode::Auto).expect("generated WKB must decode");
    common::assert_stable_wkt_roundtrip(&canonical);

    let (plain_wkt, parsed_srid) = wkb_to_wkt_split_srid(&wkb).expect("generated WKB must split");
    let (split_wkb, split_srid) = wkt_to_wkb_split_srid(&wkt).expect("generated WKT must split");
    assert_eq!(parsed_srid, split_srid);
    assert_eq!(
        wkb_header_srid(&wkb).expect("generated WKB header must parse"),
        split_srid
    );
    assert_eq!(
        wkb_header_srid(&split_wkb).expect("generated split WKB header must parse"),
        None
    );
    assert_eq!(
        wkb_to_wkt(&split_wkb, SridMode::Auto).expect("generated split WKB must decode"),
        plain_wkt
    );

    let hex = wkt_to_hex_wkb(&wkt, SridMode::Auto).expect("generated WKT must hex encode");
    assert_eq!(
        hex_wkb_to_wkt(&hex, SridMode::Auto).expect("generated hex must decode"),
        canonical
    );
    assert_eq!(
        to_wkb(Input::Text(&wkt), SridMode::Auto).expect("generic WKT must encode"),
        wkb
    );
    assert_eq!(
        to_wkt(Input::Text(&wkt), SridMode::Auto, true).expect("generic WKT must normalize"),
        canonical
    );
    assert_eq!(
        to_hex_wkb(Input::Text(&wkt), SridMode::Auto).expect("generic WKT must hex encode"),
        hex
    );
});

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dim {
    Xy,
    Xyz,
    Xym,
    Xyzm,
}

impl Dim {
    fn coord_size(self) -> usize {
        match self {
            Self::Xy => 2,
            Self::Xyz | Self::Xym => 3,
            Self::Xyzm => 4,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Self::Xy => "",
            Self::Xyz => " Z",
            Self::Xym => " M",
            Self::Xyzm => " ZM",
        }
    }
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn geometry_with_optional_srid(&mut self) -> String {
        let dim = self.dim();
        let body = self.geometry(0, dim);
        match self.byte() % 5 {
            0 => format!("SRID={};{body}", self.positive_srid()),
            1 => format!("SRID=0;{body}"),
            _ => body,
        }
    }

    fn geometry(&mut self, depth: usize, dim: Dim) -> String {
        let geom = if depth >= 2 {
            self.byte() % 3
        } else {
            self.byte() % 7
        };

        match geom {
            0 => self.point(dim),
            1 => self.linestring(dim),
            2 => self.polygon(dim),
            3 => self.multi_point(dim),
            4 => self.multi_linestring(dim),
            5 => self.multi_polygon(dim),
            _ => self.collection(depth, dim),
        }
    }

    fn point(&mut self, dim: Dim) -> String {
        if self.byte() % 16 == 0 {
            return format!("POINT{} EMPTY", dim.tag());
        }
        format!("POINT{} ({})", dim.tag(), self.coord_tuple(dim, 0))
    }

    fn linestring(&mut self, dim: Dim) -> String {
        if self.byte() % 16 == 0 {
            return format!("LINESTRING{} EMPTY", dim.tag());
        }
        let count = self.count(2, 5);
        let coords = (0..count)
            .map(|i| self.coord_tuple(dim, i))
            .collect::<Vec<_>>()
            .join(", ");
        format!("LINESTRING{} ({coords})", dim.tag())
    }

    fn polygon(&mut self, dim: Dim) -> String {
        if self.byte() % 16 == 0 {
            return format!("POLYGON{} EMPTY", dim.tag());
        }
        format!("POLYGON{} (({}))", dim.tag(), self.closed_ring(dim))
    }

    fn multi_point(&mut self, dim: Dim) -> String {
        if self.byte() % 16 == 0 {
            return format!("MULTIPOINT{} EMPTY", dim.tag());
        }
        let count = self.count(1, 4);
        let points = (0..count)
            .map(|i| format!("({})", self.coord_tuple(dim, i)))
            .collect::<Vec<_>>()
            .join(", ");
        format!("MULTIPOINT{} ({points})", dim.tag())
    }

    fn multi_linestring(&mut self, dim: Dim) -> String {
        if self.byte() % 16 == 0 {
            return format!("MULTILINESTRING{} EMPTY", dim.tag());
        }
        let count = self.count(1, 3);
        let lines = (0..count)
            .map(|_| {
                let point_count = self.count(2, 4);
                let coords = (0..point_count)
                    .map(|i| self.coord_tuple(dim, i))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({coords})")
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("MULTILINESTRING{} ({lines})", dim.tag())
    }

    fn multi_polygon(&mut self, dim: Dim) -> String {
        if self.byte() % 16 == 0 {
            return format!("MULTIPOLYGON{} EMPTY", dim.tag());
        }
        let count = self.count(1, 3);
        let polygons = (0..count)
            .map(|_| format!("(({}))", self.closed_ring(dim)))
            .collect::<Vec<_>>()
            .join(", ");
        format!("MULTIPOLYGON{} ({polygons})", dim.tag())
    }

    fn collection(&mut self, depth: usize, dim: Dim) -> String {
        if self.byte() % 16 == 0 {
            return format!("GEOMETRYCOLLECTION{} EMPTY", dim.tag());
        }
        let count = self.count(1, 3);
        let members = (0..count)
            .map(|_| {
                let child_dim = if dim == Dim::Xy { self.dim() } else { dim };
                self.geometry(depth + 1, child_dim)
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("GEOMETRYCOLLECTION{} ({members})", dim.tag())
    }

    fn closed_ring(&mut self, dim: Dim) -> String {
        let base = (self.coord(), self.coord());
        let width = (self.byte() % 9 + 1) as f64;
        let height = (self.byte() % 9 + 1) as f64;
        let mut points = Vec::with_capacity(5);
        points.push(self.coord_tuple_from_xy(dim, base.0, base.1, 0));
        points.push(self.coord_tuple_from_xy(dim, base.0 + width, base.1, 1));
        points.push(self.coord_tuple_from_xy(dim, base.0 + width, base.1 + height, 2));
        points.push(self.coord_tuple_from_xy(dim, base.0, base.1 + height, 3));
        points.push(self.coord_tuple_from_xy(dim, base.0, base.1, 0));
        points.join(", ")
    }

    fn coord_tuple(&mut self, dim: Dim, index: usize) -> String {
        let x = self.coord();
        let y = self.coord();
        self.coord_tuple_from_xy(dim, x, y, index)
    }

    fn coord_tuple_from_xy(&mut self, dim: Dim, x: f64, y: f64, index: usize) -> String {
        let mut coords = vec![x, y];
        while coords.len() < dim.coord_size() {
            coords.push(self.coord() + index as f64);
        }
        coords
            .into_iter()
            .map(|value| format!("{value:.3}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn dim(&mut self) -> Dim {
        match self.byte() % 4 {
            0 => Dim::Xy,
            1 => Dim::Xyz,
            2 => Dim::Xym,
            _ => Dim::Xyzm,
        }
    }

    fn count(&mut self, min: usize, max: usize) -> usize {
        min + self.byte() as usize % (max - min + 1)
    }

    fn positive_srid(&mut self) -> i32 {
        1 + i32::from(self.byte()) * 257
    }

    fn coord(&mut self) -> f64 {
        let raw = i16::from_be_bytes([self.byte(), self.byte()]);
        f64::from(raw) / 64.0
    }

    fn byte(&mut self) -> u8 {
        let byte = self.data[self.pos % self.data.len()];
        self.pos += 1;
        byte
    }
}
