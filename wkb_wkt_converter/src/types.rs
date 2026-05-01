use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeomType {
    Point = 1,
    LineString = 2,
    Polygon = 3,
    MultiPoint = 4,
    MultiLineString = 5,
    MultiPolygon = 6,
    GeometryCollection = 7,
}

impl GeomType {
    pub(crate) fn from_u32(v: u32) -> Result<Self> {
        match v {
            1 => Ok(GeomType::Point),
            2 => Ok(GeomType::LineString),
            3 => Ok(GeomType::Polygon),
            4 => Ok(GeomType::MultiPoint),
            5 => Ok(GeomType::MultiLineString),
            6 => Ok(GeomType::MultiPolygon),
            7 => Ok(GeomType::GeometryCollection),
            _ => Err(Error::UnsupportedGeometryType(v)),
        }
    }

    pub(crate) fn wkt_name(self) -> &'static str {
        match self {
            GeomType::Point => "POINT",
            GeomType::LineString => "LINESTRING",
            GeomType::Polygon => "POLYGON",
            GeomType::MultiPoint => "MULTIPOINT",
            GeomType::MultiLineString => "MULTILINESTRING",
            GeomType::MultiPolygon => "MULTIPOLYGON",
            GeomType::GeometryCollection => "GEOMETRYCOLLECTION",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    XY,
    XYZ,
    XYM,
    XYZM,
}

impl Dimension {
    /// Number of f64 values per coordinate tuple.
    pub fn coord_size(self) -> usize {
        match self {
            Dimension::XY => 2,
            Dimension::XYZ | Dimension::XYM => 3,
            Dimension::XYZM => 4,
        }
    }

    /// WKT dimension tag inserted between geometry type and coordinate list.
    pub(crate) fn wkt_tag(self) -> &'static str {
        match self {
            Dimension::XY => "",
            Dimension::XYZ => " Z",
            Dimension::XYM => " M",
            Dimension::XYZM => " ZM",
        }
    }

    /// EWKB flag bits OR-ed into the type code.
    pub(crate) fn ewkb_flags(self) -> u32 {
        match self {
            Dimension::XY => 0,
            Dimension::XYZ => EWKB_Z,
            Dimension::XYM => EWKB_M,
            Dimension::XYZM => EWKB_Z | EWKB_M,
        }
    }
}

// EWKB flag constants
pub(crate) const EWKB_Z: u32 = 0x8000_0000;
pub(crate) const EWKB_M: u32 = 0x4000_0000;
pub(crate) const EWKB_SRID: u32 = 0x2000_0000;
