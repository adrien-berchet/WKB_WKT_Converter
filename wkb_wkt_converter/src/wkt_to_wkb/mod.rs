mod tokenizer;
mod writer;

use crate::error::Result;
use crate::types::{Dimension, GeomType};
use tokenizer::Tokenizer;
use writer::WkbWriter;

const MAX_INITIAL_WKB_CAPACITY: usize = 16 << 20;

/// Converts WKT/EWKT to EWKB, embedding the SRID (if any) in the output.
pub(crate) fn convert(wkt: &str) -> Result<Vec<u8>> {
    let mut tok = Tokenizer::new(wkt);
    let srid = tok.read_srid_prefix()?;
    let mut writer = WkbWriter::with_capacity(initial_wkb_capacity(wkt.len()));
    parse_geometry(&mut tok, &mut writer, srid)?;
    tok.expect_eof()?;
    Ok(writer.into_bytes())
}

/// Converts WKT/EWKT to EWKB, returning the SRID separately (not embedded in bytes).
pub(crate) fn convert_split_srid(wkt: &str) -> Result<(Vec<u8>, Option<u32>)> {
    let mut tok = Tokenizer::new(wkt);
    let srid = tok.read_srid_prefix()?;
    let mut writer = WkbWriter::with_capacity(initial_wkb_capacity(wkt.len()));
    parse_geometry(&mut tok, &mut writer, None)?;
    tok.expect_eof()?;
    Ok((writer.into_bytes(), srid))
}

fn initial_wkb_capacity(wkt_len: usize) -> usize {
    (wkt_len / 4).min(MAX_INITIAL_WKB_CAPACITY)
}

fn parse_geometry(
    tok: &mut Tokenizer<'_>,
    writer: &mut WkbWriter,
    srid: Option<u32>,
) -> Result<()> {
    let (geom_type, dim) = tok.read_type_and_dim()?;
    match geom_type {
        GeomType::Point => parse_point(tok, writer, dim, srid),
        GeomType::LineString => parse_linestring(tok, writer, dim, srid),
        GeomType::Polygon => parse_polygon(tok, writer, dim, srid),
        GeomType::MultiPoint => parse_multi_point(tok, writer, dim, srid),
        GeomType::MultiLineString => parse_multi_linestring(tok, writer, dim, srid),
        GeomType::MultiPolygon => parse_multi_polygon(tok, writer, dim, srid),
        GeomType::GeometryCollection => parse_collection(tok, writer, dim, srid),
    }
}

fn parse_point(
    tok: &mut Tokenizer<'_>,
    writer: &mut WkbWriter,
    dim: Dimension,
    srid: Option<u32>,
) -> Result<()> {
    writer.write_header(GeomType::Point, dim, srid);
    if tok.read_empty_or_lparen()? {
        // POINT EMPTY: PostGIS convention — NaN for all coordinates
        for _ in 0..dim.coord_size() {
            writer.write_f64(f64::NAN);
        }
        return Ok(());
    }
    // read_empty_or_lparen peeked '(' without consuming it; consume it now.
    tok.expect_lparen()?;
    for _ in 0..dim.coord_size() {
        writer.write_f64(tok.read_number()?);
    }
    tok.expect_rparen()?;
    Ok(())
}

fn parse_linestring(
    tok: &mut Tokenizer<'_>,
    writer: &mut WkbWriter,
    dim: Dimension,
    srid: Option<u32>,
) -> Result<()> {
    writer.write_header(GeomType::LineString, dim, srid);
    if tok.read_empty_or_lparen()? {
        writer.write_u32(0);
        return Ok(());
    }
    tok.expect_lparen()?;
    let pts_pos = writer.reserve_u32();
    let pts = read_coord_seq(tok, writer, dim)?;
    writer.patch_u32(pts_pos, pts);
    Ok(())
}

fn parse_polygon(
    tok: &mut Tokenizer<'_>,
    writer: &mut WkbWriter,
    dim: Dimension,
    srid: Option<u32>,
) -> Result<()> {
    writer.write_header(GeomType::Polygon, dim, srid);
    if tok.read_empty_or_lparen()? {
        writer.write_u32(0);
        return Ok(());
    }
    tok.expect_lparen()?;
    let rings_pos = writer.reserve_u32();
    let rings = read_rings(tok, writer, dim)?;
    writer.patch_u32(rings_pos, rings);
    Ok(())
}

fn parse_multi_point(
    tok: &mut Tokenizer<'_>,
    writer: &mut WkbWriter,
    dim: Dimension,
    srid: Option<u32>,
) -> Result<()> {
    writer.write_header(GeomType::MultiPoint, dim, srid);
    if tok.read_empty_or_lparen()? {
        writer.write_u32(0);
        return Ok(());
    }
    tok.expect_lparen()?;
    let count_pos = writer.reserve_u32();
    let mut count = 0u32;
    // Detect ISO form `((x y), (x y))` vs bare form `(x y, x y)`.
    let iso_form = tok.peek_lparen();
    loop {
        writer.write_header(GeomType::Point, dim, None);
        if iso_form {
            tok.expect_lparen()?;
        }
        for _ in 0..dim.coord_size() {
            writer.write_f64(tok.read_number()?);
        }
        if iso_form {
            tok.expect_rparen()?;
        }
        count += 1;
        if !tok.read_comma_or_rparen()? {
            break;
        }
    }
    writer.patch_u32(count_pos, count);
    Ok(())
}

fn parse_multi_linestring(
    tok: &mut Tokenizer<'_>,
    writer: &mut WkbWriter,
    dim: Dimension,
    srid: Option<u32>,
) -> Result<()> {
    writer.write_header(GeomType::MultiLineString, dim, srid);
    if tok.read_empty_or_lparen()? {
        writer.write_u32(0);
        return Ok(());
    }
    tok.expect_lparen()?;
    let count_pos = writer.reserve_u32();
    let mut count = 0u32;
    loop {
        writer.write_header(GeomType::LineString, dim, None);
        tok.expect_lparen()?;
        let pts_pos = writer.reserve_u32();
        let pts = read_coord_seq(tok, writer, dim)?;
        writer.patch_u32(pts_pos, pts);
        count += 1;
        if !tok.read_comma_or_rparen()? {
            break;
        }
    }
    writer.patch_u32(count_pos, count);
    Ok(())
}

fn parse_multi_polygon(
    tok: &mut Tokenizer<'_>,
    writer: &mut WkbWriter,
    dim: Dimension,
    srid: Option<u32>,
) -> Result<()> {
    writer.write_header(GeomType::MultiPolygon, dim, srid);
    if tok.read_empty_or_lparen()? {
        writer.write_u32(0);
        return Ok(());
    }
    tok.expect_lparen()?;
    let count_pos = writer.reserve_u32();
    let mut count = 0u32;
    loop {
        writer.write_header(GeomType::Polygon, dim, None);
        tok.expect_lparen()?;
        let rings_pos = writer.reserve_u32();
        let rings = read_rings(tok, writer, dim)?;
        writer.patch_u32(rings_pos, rings);
        count += 1;
        if !tok.read_comma_or_rparen()? {
            break;
        }
    }
    writer.patch_u32(count_pos, count);
    Ok(())
}

fn parse_collection(
    tok: &mut Tokenizer<'_>,
    writer: &mut WkbWriter,
    dim: Dimension,
    srid: Option<u32>,
) -> Result<()> {
    writer.write_header(GeomType::GeometryCollection, dim, srid);
    if tok.read_empty_or_lparen()? {
        writer.write_u32(0);
        return Ok(());
    }
    tok.expect_lparen()?;
    let count_pos = writer.reserve_u32();
    let mut count = 0u32;
    loop {
        // Members of a GeometryCollection are full WKT geometries with type keywords.
        parse_geometry(tok, writer, None)?;
        count += 1;
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
fn read_coord_seq(tok: &mut Tokenizer<'_>, writer: &mut WkbWriter, dim: Dimension) -> Result<u32> {
    let mut count = 0u32;
    loop {
        for _ in 0..dim.coord_size() {
            writer.write_f64(tok.read_number()?);
        }
        count += 1;
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
fn read_rings(tok: &mut Tokenizer<'_>, writer: &mut WkbWriter, dim: Dimension) -> Result<u32> {
    let mut rings = 0u32;
    loop {
        tok.expect_lparen()?;
        let pts_pos = writer.reserve_u32();
        let pts = read_coord_seq(tok, writer, dim)?;
        writer.patch_u32(pts_pos, pts);
        rings += 1;
        if !tok.read_comma_or_rparen()? {
            break;
        }
    }
    Ok(rings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_wkb_capacity_uses_capped_quarter_input_len() {
        assert_eq!(initial_wkb_capacity(0), 0);
        assert_eq!(initial_wkb_capacity(40), 10);
        assert_eq!(
            initial_wkb_capacity((MAX_INITIAL_WKB_CAPACITY + 1) * 4),
            MAX_INITIAL_WKB_CAPACITY
        );
    }
}
