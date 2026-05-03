use crate::types::{Dimension, GeomType, EWKB_SRID};

pub(super) struct WkbWriter {
    buf: Vec<u8>,
}

impl WkbWriter {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn with_capacity(n: usize) -> Self {
        Self {
            buf: Vec::with_capacity(n),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn write_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
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
        self.buf[pos..pos + 4].copy_from_slice(&v.to_le_bytes());
    }

    /// Writes the EWKB geometry header:
    ///   - byte order byte (always 0x01 = little-endian)
    ///   - type code (geometry type | dimension flags | SRID flag)
    ///   - SRID value (if srid is Some)
    pub fn write_header(&mut self, geom_type: GeomType, dim: Dimension, srid: Option<u32>) {
        self.write_u8(1); // little-endian
        let type_code = geom_type as u32 | dim.ewkb_flags() | srid.map_or(0, |_| EWKB_SRID);
        self.write_u32(type_code);
        if let Some(s) = srid {
            self.write_u32(s);
        }
    }
}
