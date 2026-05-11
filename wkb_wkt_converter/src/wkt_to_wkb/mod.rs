mod tokenizer;
mod writer;

use crate::error::{Error, Result};
use crate::types::{Dimension, GeomType};
use tokenizer::{EmptyOrLparen, Tokenizer};
use writer::{HexWkbWriter, WkbWrite, WkbWriter};

const MAX_INITIAL_WKB_CAPACITY: usize = 16 << 20;
const MAX_GEOMETRY_DEPTH: usize = 128;

/// Converts WKT/EWKT to EWKB, embedding the SRID (if any) in the output.
pub(crate) fn convert(wkt: &str) -> Result<Vec<u8>> {
    let mut tok = Tokenizer::new(wkt);
    let srid = tok.read_srid_prefix()?;
    convert_tokenized(tok, wkt.len(), srid)
}

/// Converts WKT/EWKT to EWKB, embedding `srid` in the top-level output header.
///
/// Any input `SRID=` prefix is still parsed and validated, but its value is
/// ignored in favour of the forced output SRID.
pub(crate) fn convert_with_forced_srid(wkt: &str, srid: i32) -> Result<Vec<u8>> {
    let mut tok = Tokenizer::new(wkt);
    tok.read_srid_prefix()?;
    convert_tokenized(tok, wkt.len(), Some(srid))
}

pub(crate) fn convert_to_hex(wkt: &str) -> Result<String> {
    let mut tok = Tokenizer::new(wkt);
    let srid = tok.read_srid_prefix()?;
    convert_tokenized_to_hex(tok, wkt.len(), srid)
}

pub(crate) fn convert_to_hex_with_forced_srid(wkt: &str, srid: i32) -> Result<String> {
    let mut tok = Tokenizer::new(wkt);
    tok.read_srid_prefix()?;
    convert_tokenized_to_hex(tok, wkt.len(), Some(srid))
}

pub(crate) fn convert_to_hex_split_srid(wkt: &str) -> Result<String> {
    let mut tok = Tokenizer::new(wkt);
    tok.read_srid_prefix()?;
    convert_tokenized_to_hex(tok, wkt.len(), None)
}

fn convert_tokenized(mut tok: Tokenizer<'_>, wkt_len: usize, srid: Option<i32>) -> Result<Vec<u8>> {
    let mut writer = WkbWriter::with_capacity(initial_wkb_capacity(wkt_len));
    parse_geometry_at_depth(&mut tok, &mut writer, srid, 0)?;
    tok.expect_eof()?;
    Ok(writer.into_bytes())
}

fn convert_tokenized_to_hex(
    mut tok: Tokenizer<'_>,
    wkt_len: usize,
    srid: Option<i32>,
) -> Result<String> {
    let mut writer = HexWkbWriter::with_capacity(initial_wkb_capacity(wkt_len));
    parse_geometry_at_depth(&mut tok, &mut writer, srid, 0)?;
    tok.expect_eof()?;
    Ok(writer.into_string())
}

/// Converts WKT/EWKT to EWKB, returning the SRID separately (not embedded in bytes).
pub(crate) fn convert_split_srid(wkt: &str) -> Result<(Vec<u8>, Option<i32>)> {
    let mut tok = Tokenizer::new(wkt);
    let srid = tok.read_srid_prefix()?;
    let mut writer = WkbWriter::with_capacity(initial_wkb_capacity(wkt.len()));
    parse_geometry_at_depth(&mut tok, &mut writer, None, 0)?;
    tok.expect_eof()?;
    Ok((writer.into_bytes(), srid))
}

fn initial_wkb_capacity(wkt_len: usize) -> usize {
    wkt_len.min(MAX_INITIAL_WKB_CAPACITY)
}

fn parse_geometry_at_depth(
    tok: &mut Tokenizer<'_>,
    writer: &mut impl WkbWrite,
    srid: Option<i32>,
    depth: usize,
) -> Result<(GeomType, Dimension)> {
    if depth >= MAX_GEOMETRY_DEPTH {
        return Err(Error::InvalidWkt(format!(
            "geometry nesting exceeds maximum depth of {MAX_GEOMETRY_DEPTH}"
        )));
    }
    let (geom_type, dim) = tok.read_type_and_dim()?;
    match geom_type {
        GeomType::Point => parse_point(tok, writer, dim, srid)?,
        GeomType::LineString => parse_linestring(tok, writer, dim, srid)?,
        GeomType::Polygon => parse_polygon(tok, writer, dim, srid)?,
        GeomType::MultiPoint => parse_multi_point(tok, writer, dim, srid)?,
        GeomType::MultiLineString => parse_multi_linestring(tok, writer, dim, srid)?,
        GeomType::MultiPolygon => parse_multi_polygon(tok, writer, dim, srid)?,
        GeomType::GeometryCollection => parse_collection(tok, writer, dim, srid, depth)?,
    };
    Ok((geom_type, dim))
}

fn parse_point(
    tok: &mut Tokenizer<'_>,
    writer: &mut impl WkbWrite,
    dim: Dimension,
    srid: Option<i32>,
) -> Result<()> {
    writer.write_header(GeomType::Point, dim, srid);
    match tok.read_empty_or_lparen()? {
        EmptyOrLparen::Empty => {
            // POINT EMPTY: PostGIS convention — NaN for all coordinates
            write_empty_point_body(writer, dim);
            return Ok(());
        }
        EmptyOrLparen::Lparen => {}
    }
    write_coord_tuple(tok, writer, dim)?;
    tok.expect_rparen()?;
    Ok(())
}

fn parse_linestring(
    tok: &mut Tokenizer<'_>,
    writer: &mut impl WkbWrite,
    dim: Dimension,
    srid: Option<i32>,
) -> Result<()> {
    writer.write_header(GeomType::LineString, dim, srid);
    match tok.read_empty_or_lparen()? {
        EmptyOrLparen::Empty => {
            writer.write_u32(0);
            return Ok(());
        }
        EmptyOrLparen::Lparen => {}
    }
    let pts_pos = writer.reserve_u32();
    let pts = read_coord_seq(tok, writer, dim)?;
    writer.patch_u32(pts_pos, pts);
    Ok(())
}

fn parse_polygon(
    tok: &mut Tokenizer<'_>,
    writer: &mut impl WkbWrite,
    dim: Dimension,
    srid: Option<i32>,
) -> Result<()> {
    writer.write_header(GeomType::Polygon, dim, srid);
    match tok.read_empty_or_lparen()? {
        EmptyOrLparen::Empty => {
            writer.write_u32(0);
            return Ok(());
        }
        EmptyOrLparen::Lparen => {}
    }
    let rings_pos = writer.reserve_u32();
    let rings = read_rings(tok, writer, dim)?;
    writer.patch_u32(rings_pos, rings);
    Ok(())
}

fn parse_multi_point(
    tok: &mut Tokenizer<'_>,
    writer: &mut impl WkbWrite,
    dim: Dimension,
    srid: Option<i32>,
) -> Result<()> {
    writer.write_header(GeomType::MultiPoint, dim, srid);
    match tok.read_empty_or_lparen()? {
        EmptyOrLparen::Empty => {
            writer.write_u32(0);
            return Ok(());
        }
        EmptyOrLparen::Lparen => {}
    }
    let count_pos = writer.reserve_u32();
    let mut count = 0u32;
    loop {
        writer.write_header(GeomType::Point, dim, None);
        match tok.try_read_empty_or_lparen()? {
            Some(EmptyOrLparen::Empty) => write_empty_point_body(writer, dim),
            Some(EmptyOrLparen::Lparen) => {
                write_coord_tuple(tok, writer, dim)?;
                tok.expect_rparen()?;
            }
            None => write_coord_tuple(tok, writer, dim)?,
        }
        increment_count(&mut count, "MultiPoint members")?;
        if !tok.read_comma_or_rparen()? {
            break;
        }
    }
    writer.patch_u32(count_pos, count);
    Ok(())
}

fn parse_multi_linestring(
    tok: &mut Tokenizer<'_>,
    writer: &mut impl WkbWrite,
    dim: Dimension,
    srid: Option<i32>,
) -> Result<()> {
    writer.write_header(GeomType::MultiLineString, dim, srid);
    match tok.read_empty_or_lparen()? {
        EmptyOrLparen::Empty => {
            writer.write_u32(0);
            return Ok(());
        }
        EmptyOrLparen::Lparen => {}
    }
    let count_pos = writer.reserve_u32();
    let mut count = 0u32;
    loop {
        writer.write_header(GeomType::LineString, dim, None);
        match tok.read_empty_or_lparen()? {
            EmptyOrLparen::Empty => writer.write_u32(0),
            EmptyOrLparen::Lparen => {
                let pts_pos = writer.reserve_u32();
                let pts = read_coord_seq(tok, writer, dim)?;
                writer.patch_u32(pts_pos, pts);
            }
        }
        increment_count(&mut count, "MultiLineString members")?;
        if !tok.read_comma_or_rparen()? {
            break;
        }
    }
    writer.patch_u32(count_pos, count);
    Ok(())
}

fn parse_multi_polygon(
    tok: &mut Tokenizer<'_>,
    writer: &mut impl WkbWrite,
    dim: Dimension,
    srid: Option<i32>,
) -> Result<()> {
    writer.write_header(GeomType::MultiPolygon, dim, srid);
    match tok.read_empty_or_lparen()? {
        EmptyOrLparen::Empty => {
            writer.write_u32(0);
            return Ok(());
        }
        EmptyOrLparen::Lparen => {}
    }
    let count_pos = writer.reserve_u32();
    let mut count = 0u32;
    loop {
        writer.write_header(GeomType::Polygon, dim, None);
        match tok.read_empty_or_lparen()? {
            EmptyOrLparen::Empty => writer.write_u32(0),
            EmptyOrLparen::Lparen => {
                let rings_pos = writer.reserve_u32();
                let rings = read_rings(tok, writer, dim)?;
                writer.patch_u32(rings_pos, rings);
            }
        }
        increment_count(&mut count, "MultiPolygon members")?;
        if !tok.read_comma_or_rparen()? {
            break;
        }
    }
    writer.patch_u32(count_pos, count);
    Ok(())
}

fn parse_collection(
    tok: &mut Tokenizer<'_>,
    writer: &mut impl WkbWrite,
    dim: Dimension,
    srid: Option<i32>,
    depth: usize,
) -> Result<()> {
    writer.write_header(GeomType::GeometryCollection, dim, srid);
    match tok.read_empty_or_lparen()? {
        EmptyOrLparen::Empty => {
            writer.write_u32(0);
            return Ok(());
        }
        EmptyOrLparen::Lparen => {}
    }
    let count_pos = writer.reserve_u32();
    let mut count = 0u32;
    loop {
        // Members of a GeometryCollection are full WKT geometries with type keywords.
        let (_, child_dim) = parse_geometry_at_depth(tok, writer, None, depth + 1)?;
        if dim != Dimension::XY && child_dim != dim {
            return Err(Error::InvalidWkt(format!(
                "GeometryCollection {dim:?} member has incompatible dimension {child_dim:?}"
            )));
        }
        increment_count(&mut count, "GeometryCollection members")?;
        if !tok.read_comma_or_rparen()? {
            break;
        }
    }
    writer.patch_u32(count_pos, count);
    Ok(())
}

/// Reads comma-separated coordinate tuples until the closing `)` is consumed.
/// Returns the number of coordinate tuples read.
///
/// Convention: the opening `(` has already been consumed by the caller.
/// This function consumes the matching closing `)`.
fn read_coord_seq(
    tok: &mut Tokenizer<'_>,
    writer: &mut impl WkbWrite,
    dim: Dimension,
) -> Result<u32> {
    let mut count = 0u32;
    loop {
        write_coord_tuple(tok, writer, dim)?;
        increment_count(&mut count, "coordinate tuples")?;
        if !tok.read_comma_or_rparen()? {
            break;
        }
    }
    Ok(count)
}

/// Reads the ring list of a Polygon body: `(ring), (ring), ...` until the closing `)`.
/// Returns the number of rings.
///
/// Convention: the outer polygon `(` has already been consumed by the caller.
/// This function consumes the matching closing `)`.
fn read_rings(tok: &mut Tokenizer<'_>, writer: &mut impl WkbWrite, dim: Dimension) -> Result<u32> {
    let mut rings = 0u32;
    loop {
        tok.expect_lparen()?;
        let pts_pos = writer.reserve_u32();
        let pts = read_coord_seq(tok, writer, dim)?;
        writer.patch_u32(pts_pos, pts);
        increment_count(&mut rings, "polygon rings")?;
        if !tok.read_comma_or_rparen()? {
            break;
        }
    }
    Ok(rings)
}

fn write_coord_tuple(
    tok: &mut Tokenizer<'_>,
    writer: &mut impl WkbWrite,
    dim: Dimension,
) -> Result<()> {
    for _ in 0..dim.coord_size() {
        writer.write_f64(tok.read_number()?);
    }
    Ok(())
}

fn write_empty_point_body(writer: &mut impl WkbWrite, dim: Dimension) {
    for _ in 0..dim.coord_size() {
        writer.write_f64(f64::NAN);
    }
}

fn increment_count(count: &mut u32, what: &str) -> Result<()> {
    *count = count
        .checked_add(1)
        .ok_or_else(|| Error::InvalidWkt(format!("too many {what}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_wkb_capacity_uses_capped_input_len() {
        assert_eq!(initial_wkb_capacity(0), 0);
        assert_eq!(initial_wkb_capacity(40), 40);
        assert_eq!(
            initial_wkb_capacity(MAX_INITIAL_WKB_CAPACITY + 1),
            MAX_INITIAL_WKB_CAPACITY
        );
    }

    #[test]
    fn forced_srid_overrides_validated_input_srid_without_second_copy() {
        let wkb = convert_with_forced_srid("SRID=4326;POINT (1 2)", 3857).unwrap();
        assert_eq!(
            crate::wkb_to_wkt::convert(&wkb).unwrap(),
            "SRID=3857;POINT (1 2)"
        );
    }

    #[test]
    fn forced_srid_is_only_written_to_top_level_collection_header() {
        let wkb = convert_with_forced_srid("GEOMETRYCOLLECTION (POINT (1 2))", 3857).unwrap();
        assert_eq!(
            u32::from_le_bytes(wkb[1..5].try_into().unwrap()),
            0x2000_0007
        );
        assert_eq!(u32::from_le_bytes(wkb[5..9].try_into().unwrap()), 3857);
        assert_eq!(u32::from_le_bytes(wkb[14..18].try_into().unwrap()), 1);
    }

    #[test]
    fn forced_srid_still_validates_input_srid_prefix() {
        assert!(convert_with_forced_srid("SRID=abc;POINT (1 2)", 3857).is_err());
    }

    #[test]
    fn increment_count_checks_overflow() {
        let mut count = 0;
        increment_count(&mut count, "test items").unwrap();
        assert_eq!(count, 1);

        let mut count = u32::MAX;
        assert!(increment_count(&mut count, "test items").is_err());
        assert_eq!(count, u32::MAX);
    }
}
