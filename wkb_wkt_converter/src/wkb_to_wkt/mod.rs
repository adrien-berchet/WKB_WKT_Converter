mod builder;
mod reader;

use crate::error::Result;

pub(crate) fn convert(wkb: &[u8]) -> Result<String> {
    let mut reader = reader::WkbReader::new(wkb);
    let mut builder = builder::WktBuilder::with_capacity(wkb.len() * 3);
    let srid = reader.read_geometry(&mut builder)?;
    if let Some(srid) = srid {
        let body = builder.finish();
        let mut out = String::with_capacity(10 + body.len());
        use std::fmt::Write as _;
        write!(out, "SRID={srid};").unwrap();
        out.push_str(&body);
        return Ok(out);
    }
    Ok(builder.finish())
}

pub(crate) fn convert_split_srid(wkb: &[u8]) -> Result<(String, Option<u32>)> {
    let mut reader = reader::WkbReader::new(wkb);
    let mut builder = builder::WktBuilder::with_capacity(wkb.len() * 3);
    let srid = reader.read_geometry(&mut builder)?;
    Ok((builder.finish(), srid))
}
