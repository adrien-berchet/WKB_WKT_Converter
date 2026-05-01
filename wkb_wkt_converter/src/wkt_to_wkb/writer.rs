use crate::types::{Dimension, GeomType, EWKB_SRID};

pub(super) struct WkbWriter {
    buf: Vec<u8>,
    little_endian: bool,
}

impl WkbWriter {
    pub fn new(little_endian: bool) -> Self {
        Self {
            buf: Vec::new(),
            little_endian,
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn write_u32(&mut self, v: u32) {
        let bytes = if self.little_endian {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        };
        self.buf.extend_from_slice(&bytes);
    }

    pub fn write_f64(&mut self, v: f64) {
        let bytes = if self.little_endian {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        };
        self.buf.extend_from_slice(&bytes);
    }

    /// Writes a u32 placeholder (0x00000000) and returns its byte offset.
    /// Call `patch_u32` later to fill in the real value.
    pub fn reserve_u32(&mut self) -> usize {
        let pos = self.buf.len();
        self.buf.extend_from_slice(&[0u8; 4]);
        pos
    }

    /// Seeks back to `pos` and overwrites the placeholder with `v`.
    pub fn patch_u32(&mut self, pos: usize, v: u32) {
        let bytes = if self.little_endian {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        };
        self.buf[pos..pos + 4].copy_from_slice(&bytes);
    }

    /// Writes the EWKB geometry header:
    ///   - byte order byte
    ///   - type code (geometry type | dimension flags | SRID flag)
    ///   - SRID value (if srid is Some)
    pub fn write_header(&mut self, geom_type: GeomType, dim: Dimension, srid: Option<u32>) {
        self.write_u8(if self.little_endian { 1 } else { 0 });
        let type_code = geom_type as u32 | dim.ewkb_flags() | srid.map_or(0, |_| EWKB_SRID);
        self.write_u32(type_code);
        if let Some(s) = srid {
            self.write_u32(s);
        }
    }
}
