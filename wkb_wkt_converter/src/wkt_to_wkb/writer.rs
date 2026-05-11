use crate::types::{Dimension, GeomType, EWKB_SRID};

const HEX: &[u8; 16] = b"0123456789ABCDEF";

pub(super) trait WkbWrite {
    fn write_u8(&mut self, v: u8);
    fn write_u32(&mut self, v: u32);
    fn write_f64(&mut self, v: f64);
    fn reserve_u32(&mut self) -> usize;
    fn patch_u32(&mut self, pos: usize, v: u32);

    /// Writes the EWKB geometry header:
    ///   - byte order byte (always 0x01 = little-endian)
    ///   - type code (geometry type | dimension flags | SRID flag)
    ///   - SRID value (if srid is Some)
    fn write_header(&mut self, geom_type: GeomType, dim: Dimension, srid: Option<i32>) {
        self.write_u8(1); // little-endian
        let type_code = geom_type as u32 | dim.ewkb_flags() | srid.map_or(0, |_| EWKB_SRID);
        self.write_u32(type_code);
        if let Some(s) = srid {
            self.write_u32(s as u32);
        }
    }
}

pub(super) struct WkbWriter {
    buf: Vec<u8>,
}

impl WkbWriter {
    pub fn with_capacity(n: usize) -> Self {
        Self {
            buf: Vec::with_capacity(n),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

impl WkbWrite for WkbWriter {
    fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    fn write_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn reserve_u32(&mut self) -> usize {
        let pos = self.buf.len();
        self.buf.extend_from_slice(&[0u8; 4]);
        pos
    }

    fn patch_u32(&mut self, pos: usize, v: u32) {
        self.buf[pos..pos + 4].copy_from_slice(&v.to_le_bytes());
    }
}

pub(super) struct HexWkbWriter {
    buf: Vec<u8>,
}

impl HexWkbWriter {
    pub fn with_capacity(n: usize) -> Self {
        Self {
            buf: Vec::with_capacity(n.saturating_mul(2)),
        }
    }

    pub fn into_string(self) -> String {
        // SAFETY: every byte is written by `write_hex_byte`, which only emits
        // uppercase ASCII hex digits.
        unsafe { String::from_utf8_unchecked(self.buf) }
    }

    fn write_hex_byte(&mut self, v: u8) {
        self.buf.push(HEX[(v >> 4) as usize]);
        self.buf.push(HEX[(v & 0xF) as usize]);
    }
}

impl WkbWrite for HexWkbWriter {
    fn write_u8(&mut self, v: u8) {
        self.write_hex_byte(v);
    }

    fn write_u32(&mut self, v: u32) {
        for b in v.to_le_bytes() {
            self.write_hex_byte(b);
        }
    }

    fn write_f64(&mut self, v: f64) {
        for b in v.to_le_bytes() {
            self.write_hex_byte(b);
        }
    }

    fn reserve_u32(&mut self) -> usize {
        let pos = self.buf.len() / 2;
        self.buf.extend_from_slice(b"00000000");
        pos
    }

    fn patch_u32(&mut self, pos: usize, v: u32) {
        let mut hex_pos = pos * 2;
        for b in v.to_le_bytes() {
            self.buf[hex_pos] = HEX[(b >> 4) as usize];
            self.buf[hex_pos + 1] = HEX[(b & 0xF) as usize];
            hex_pos += 2;
        }
    }
}
