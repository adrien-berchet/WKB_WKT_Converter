mod builder;
mod reader;

use crate::error::Result;

pub(crate) fn convert(wkb: &[u8]) -> Result<String> {
    let mut reader = reader::WkbReader::new(wkb);
    let mut builder = builder::WktBuilder::with_capacity(wkb.len().saturating_mul(3).min(16 << 20));
    reader.read_geometry(&mut builder, true)?;
    reader.expect_eof()?;
    Ok(builder.finish())
}

pub(crate) fn convert_split_srid(wkb: &[u8]) -> Result<(String, Option<u32>)> {
    let mut reader = reader::WkbReader::new(wkb);
    let mut builder = builder::WktBuilder::with_capacity(wkb.len().saturating_mul(3).min(16 << 20));
    let srid = reader.read_geometry(&mut builder, false)?;
    reader.expect_eof()?;
    Ok((builder.finish(), srid))
}
