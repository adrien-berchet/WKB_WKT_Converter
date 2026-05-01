use super::builder::WktBuilder;
use crate::error::{Error, Result};
use crate::types::{Dimension, GeomType, EWKB_M, EWKB_SRID, EWKB_Z};

pub(super) struct WkbReader<'a> {
    data: &'a [u8],
    pos: usize,
    little_endian: bool,
}

impl<'a> WkbReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            little_endian: true,
        }
    }

    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn read_u8(&mut self) -> Result<u8> {
        if self.pos >= self.data.len() {
            return Err(Error::InvalidWkb("unexpected end of data".into()));
        }
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }

    fn read_u32(&mut self) -> Result<u32> {
        if self.remaining() < 4 {
            return Err(Error::InvalidWkb(
                "unexpected end of data reading u32".into(),
            ));
        }
        let bytes: [u8; 4] = self.data[self.pos..self.pos + 4].try_into().unwrap();
        self.pos += 4;
        Ok(if self.little_endian {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        })
    }

    fn read_f64(&mut self) -> Result<f64> {
        if self.remaining() < 8 {
            return Err(Error::InvalidWkb(
                "unexpected end of data reading f64".into(),
            ));
        }
        let bytes: [u8; 8] = self.data[self.pos..self.pos + 8].try_into().unwrap();
        self.pos += 8;
        Ok(if self.little_endian {
            f64::from_le_bytes(bytes)
        } else {
            f64::from_be_bytes(bytes)
        })
    }

    fn read_byte_order(&mut self) -> Result<()> {
        match self.read_u8()? {
            0 => {
                self.little_endian = false;
                Ok(())
            }
            1 => {
                self.little_endian = true;
                Ok(())
            }
            b => Err(Error::InvalidWkb(format!("invalid byte order marker: {b}"))),
        }
    }

    /// Decodes the 4-byte type code, handling both ISO WKB and EWKB encodings.
    /// Returns (GeomType, Dimension, Option<SRID>).
    fn read_type(&mut self) -> Result<(GeomType, Dimension, Option<u32>)> {
        let raw = self.read_u32()?;

        let has_z = raw & EWKB_Z != 0;
        let has_m = raw & EWKB_M != 0;
        let has_srid = raw & EWKB_SRID != 0;
        // Strip EWKB flag bits; the base type sits in the lower 16 bits
        let base = raw & 0x0000_FFFF;

        let (geom_code, dim) = if has_z || has_m {
            // EWKB dimension encoding via flag bits
            let dim = match (has_z, has_m) {
                (true, false) => Dimension::XYZ,
                (false, true) => Dimension::XYM,
                (true, true) => Dimension::XYZM,
                _ => unreachable!(),
            };
            (base, dim)
        } else if base > 3000 {
            // ISO WKB XYZM: type + 3000
            (base - 3000, Dimension::XYZM)
        } else if base > 2000 {
            // ISO WKB XYM: type + 2000
            (base - 2000, Dimension::XYM)
        } else if base > 1000 {
            // ISO WKB XYZ: type + 1000
            (base - 1000, Dimension::XYZ)
        } else {
            (base, Dimension::XY)
        };

        let geom_type = GeomType::from_u32(geom_code)?;
        let srid = if has_srid {
            Some(self.read_u32()?)
        } else {
            None
        };

        Ok((geom_type, dim, srid))
    }

    /// Reads one complete WKB geometry and appends its WKT representation to `out`.
    /// Returns the top-level SRID if the geometry was EWKB-encoded with one.
    pub fn read_geometry(&mut self, out: &mut WktBuilder) -> Result<Option<u32>> {
        self.read_byte_order()?;
        let (geom_type, dim, srid) = self.read_type()?;

        out.push_str(geom_type.wkt_name());
        out.push_str(dim.wkt_tag());

        match geom_type {
            GeomType::Point => self.read_point_body(out, dim)?,
            GeomType::LineString => self.read_linestring_body(out, dim)?,
            GeomType::Polygon => self.read_polygon_body(out, dim)?,
            GeomType::MultiPoint => self.read_multi_point_body(out)?,
            GeomType::MultiLineString => self.read_multi_linestring_body(out)?,
            GeomType::MultiPolygon => self.read_multi_polygon_body(out)?,
            GeomType::GeometryCollection => self.read_collection_body(out)?,
        }

        Ok(srid)
    }

    fn read_point_body(&mut self, out: &mut WktBuilder, dim: Dimension) -> Result<()> {
        let n = dim.coord_size();
        let mut coords = Vec::with_capacity(n);
        for _ in 0..n {
            coords.push(self.read_f64()?);
        }
        if coords.iter().all(|v| v.is_nan()) {
            out.push_str(" EMPTY");
        } else {
            out.push_str(" (");
            out.push_coord(&coords);
            out.push_char(')');
        }
        Ok(())
    }

    fn read_linestring_body(&mut self, out: &mut WktBuilder, dim: Dimension) -> Result<()> {
        let num_pts = self.read_u32()? as usize;
        if num_pts == 0 {
            out.push_str(" EMPTY");
            return Ok(());
        }
        out.push_str(" (");
        self.read_coord_seq(out, dim, num_pts)?;
        out.push_char(')');
        Ok(())
    }

    fn read_polygon_body(&mut self, out: &mut WktBuilder, dim: Dimension) -> Result<()> {
        let num_rings = self.read_u32()? as usize;
        if num_rings == 0 {
            out.push_str(" EMPTY");
            return Ok(());
        }
        out.push_str(" (");
        for i in 0..num_rings {
            if i > 0 {
                out.push_str(", ");
            }
            let num_pts = self.read_u32()? as usize;
            out.push_char('(');
            self.read_coord_seq(out, dim, num_pts)?;
            out.push_char(')');
        }
        out.push_char(')');
        Ok(())
    }

    /// Each sub-geometry in a MULTI* has its own full WKB header but is rendered
    /// in WKT without a type keyword: `((x y), (x y))`.
    fn read_multi_point_body(&mut self, out: &mut WktBuilder) -> Result<()> {
        let num_geoms = self.read_u32()? as usize;
        if num_geoms == 0 {
            out.push_str(" EMPTY");
            return Ok(());
        }
        out.push_str(" (");
        for i in 0..num_geoms {
            if i > 0 {
                out.push_str(", ");
            }
            self.read_byte_order()?;
            let (geom_type, dim, _) = self.read_type()?;
            if geom_type != GeomType::Point {
                return Err(Error::InvalidWkb(format!(
                    "expected Point sub-geometry in MultiPoint, got type code {geom_type:?}",
                )));
            }
            let n = dim.coord_size();
            let mut coords = Vec::with_capacity(n);
            for _ in 0..n {
                coords.push(self.read_f64()?);
            }
            if coords.iter().all(|v| v.is_nan()) {
                out.push_str("EMPTY");
            } else {
                out.push_char('(');
                out.push_coord(&coords);
                out.push_char(')');
            }
        }
        out.push_char(')');
        Ok(())
    }

    fn read_multi_linestring_body(&mut self, out: &mut WktBuilder) -> Result<()> {
        let num_geoms = self.read_u32()? as usize;
        if num_geoms == 0 {
            out.push_str(" EMPTY");
            return Ok(());
        }
        out.push_str(" (");
        for i in 0..num_geoms {
            if i > 0 {
                out.push_str(", ");
            }
            self.read_byte_order()?;
            let (geom_type, dim, _) = self.read_type()?;
            if geom_type != GeomType::LineString {
                return Err(Error::InvalidWkb(format!(
                    "expected LineString sub-geometry in MultiLineString, got type code {geom_type:?}",
                )));
            }
            let num_pts = self.read_u32()? as usize;
            if num_pts == 0 {
                out.push_str("EMPTY");
            } else {
                out.push_char('(');
                self.read_coord_seq(out, dim, num_pts)?;
                out.push_char(')');
            }
        }
        out.push_char(')');
        Ok(())
    }

    fn read_multi_polygon_body(&mut self, out: &mut WktBuilder) -> Result<()> {
        let num_geoms = self.read_u32()? as usize;
        if num_geoms == 0 {
            out.push_str(" EMPTY");
            return Ok(());
        }
        out.push_str(" (");
        for i in 0..num_geoms {
            if i > 0 {
                out.push_str(", ");
            }
            self.read_byte_order()?;
            let (geom_type, dim, _) = self.read_type()?;
            if geom_type != GeomType::Polygon {
                return Err(Error::InvalidWkb(format!(
                    "expected Polygon sub-geometry in MultiPolygon, got type code {geom_type:?}",
                )));
            }
            let num_rings = self.read_u32()? as usize;
            if num_rings == 0 {
                out.push_str("EMPTY");
            } else {
                out.push_char('(');
                for j in 0..num_rings {
                    if j > 0 {
                        out.push_str(", ");
                    }
                    let num_pts = self.read_u32()? as usize;
                    out.push_char('(');
                    self.read_coord_seq(out, dim, num_pts)?;
                    out.push_char(')');
                }
                out.push_char(')');
            }
        }
        out.push_char(')');
        Ok(())
    }

    /// GeometryCollection members are full WKT geometries with type keywords.
    fn read_collection_body(&mut self, out: &mut WktBuilder) -> Result<()> {
        let num_geoms = self.read_u32()? as usize;
        if num_geoms == 0 {
            out.push_str(" EMPTY");
            return Ok(());
        }
        out.push_str(" (");
        for i in 0..num_geoms {
            if i > 0 {
                out.push_str(", ");
            }
            self.read_geometry(out)?;
        }
        out.push_char(')');
        Ok(())
    }

    fn read_coord_seq(
        &mut self,
        out: &mut WktBuilder,
        dim: Dimension,
        num_pts: usize,
    ) -> Result<()> {
        let n = dim.coord_size();
        let mut coords = Vec::with_capacity(n);
        for i in 0..num_pts {
            if i > 0 {
                out.push_str(", ");
            }
            coords.clear();
            for _ in 0..n {
                coords.push(self.read_f64()?);
            }
            out.push_coord(&coords);
        }
        Ok(())
    }
}
