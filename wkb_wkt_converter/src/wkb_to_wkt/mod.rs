mod builder;
mod reader;

use crate::error::Result;

pub(crate) fn convert(wkb: &[u8], emit_srid: bool) -> Result<String> {
    let mut reader = reader::WkbReader::new(wkb);
    let mut builder = builder::WktBuilder::new();
    let srid = reader.read_geometry(&mut builder)?;
    if emit_srid {
        if let Some(srid) = srid {
            let mut out = format!("SRID={srid};");
            out.push_str(&builder.finish());
            return Ok(out);
        }
    }
    Ok(builder.finish())
}

pub(crate) fn convert_split_srid(wkb: &[u8]) -> Result<(String, Option<u32>)> {
    let mut reader = reader::WkbReader::new(wkb);
    let mut builder = builder::WktBuilder::new();
    let srid = reader.read_geometry(&mut builder)?;
    Ok((builder.finish(), srid))
}
