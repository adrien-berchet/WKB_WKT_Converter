use crate::error::{Error, Result};
use crate::types::{Dimension, GeomType};

pub(super) struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn rest(&self) -> &str {
        &self.input[self.pos..]
    }

    fn skip_whitespace(&mut self) {
        let rest = self.rest();
        self.pos += rest.len() - rest.trim_start().len();
    }

    fn peek_nonws_char(&mut self) -> Option<char> {
        self.skip_whitespace();
        self.rest().chars().next()
    }

    /// Reads `SRID=<n>;` prefix if present. Must be called before any other method.
    pub fn read_srid_prefix(&mut self) -> Result<Option<u32>> {
        self.skip_whitespace();
        // Use a case-insensitive prefix check on the first 5 ASCII bytes
        if self.rest().len() >= 5 && self.rest()[..5].eq_ignore_ascii_case("SRID=") {
            self.pos += 5;
            let srid = self.read_uint()?;
            self.skip_whitespace();
            if self.rest().starts_with(';') {
                self.pos += 1;
                Ok(Some(srid))
            } else {
                Err(Error::InvalidWkt("expected ';' after SRID value".into()))
            }
        } else {
            Ok(None)
        }
    }

    fn read_uint(&mut self) -> Result<u32> {
        let end = self
            .rest()
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(self.rest().len());
        if end == 0 {
            return Err(Error::InvalidWkt(format!(
                "expected integer at position {}",
                self.pos
            )));
        }
        let s = &self.rest()[..end];
        let v = s
            .parse::<u32>()
            .map_err(|_| Error::InvalidWkt(format!("integer out of u32 range: {s}")))?;
        self.pos += end;
        Ok(v)
    }

    /// Reads the geometry type keyword and an optional dimension tag (Z / M / ZM).
    pub fn read_type_and_dim(&mut self) -> Result<(GeomType, Dimension)> {
        self.skip_whitespace();
        let rest = self.rest();

        let first = rest
            .as_bytes()
            .first()
            .copied()
            .map(|b| b.to_ascii_uppercase());
        let (geom_type, keyword_len) = match first {
            Some(b'G') => {
                if rest
                    .get(..18)
                    .is_some_and(|s| s.eq_ignore_ascii_case("GEOMETRYCOLLECTION"))
                {
                    (GeomType::GeometryCollection, 18)
                } else {
                    return Err(Error::InvalidWkt(format!(
                        "unknown geometry type at position {}: {:?}",
                        self.pos,
                        &rest[..rest.len().min(20)]
                    )));
                }
            }
            Some(b'M') => {
                if rest
                    .get(..15)
                    .is_some_and(|s| s.eq_ignore_ascii_case("MULTILINESTRING"))
                {
                    (GeomType::MultiLineString, 15)
                } else if rest
                    .get(..12)
                    .is_some_and(|s| s.eq_ignore_ascii_case("MULTIPOLYGON"))
                {
                    (GeomType::MultiPolygon, 12)
                } else if rest
                    .get(..10)
                    .is_some_and(|s| s.eq_ignore_ascii_case("MULTIPOINT"))
                {
                    (GeomType::MultiPoint, 10)
                } else {
                    return Err(Error::InvalidWkt(format!(
                        "unknown geometry type at position {}: {:?}",
                        self.pos,
                        &rest[..rest.len().min(20)]
                    )));
                }
            }
            Some(b'L') => {
                if rest
                    .get(..10)
                    .is_some_and(|s| s.eq_ignore_ascii_case("LINESTRING"))
                {
                    (GeomType::LineString, 10)
                } else {
                    return Err(Error::InvalidWkt(format!(
                        "unknown geometry type at position {}: {:?}",
                        self.pos,
                        &rest[..rest.len().min(20)]
                    )));
                }
            }
            Some(b'P') => {
                if rest
                    .get(..7)
                    .is_some_and(|s| s.eq_ignore_ascii_case("POLYGON"))
                {
                    (GeomType::Polygon, 7)
                } else if rest
                    .get(..5)
                    .is_some_and(|s| s.eq_ignore_ascii_case("POINT"))
                {
                    (GeomType::Point, 5)
                } else {
                    return Err(Error::InvalidWkt(format!(
                        "unknown geometry type at position {}: {:?}",
                        self.pos,
                        &rest[..rest.len().min(20)]
                    )));
                }
            }
            _ => {
                return Err(Error::InvalidWkt(format!(
                    "unknown geometry type at position {}: {:?}",
                    self.pos,
                    &rest[..rest.len().min(20)]
                )));
            }
        };

        self.pos += keyword_len;
        let dim = self.read_dim_tag()?;
        Ok((geom_type, dim))
    }

    fn read_dim_tag(&mut self) -> Result<Dimension> {
        let saved = self.pos;
        // Dimension tag must be preceded by whitespace to avoid misreading
        // "POLYGON(...)" as "POLY" + dim "GON".
        let rest_before_skip = self.rest();
        if !rest_before_skip.starts_with(|c: char| c.is_ascii_whitespace()) {
            return Ok(Dimension::XY);
        }
        self.skip_whitespace();
        let rest = self.rest();
        let bytes = rest.as_bytes();
        let b0 = bytes.first().copied().map(|b| b.to_ascii_uppercase());
        let b1 = bytes.get(1).copied().map(|b| b.to_ascii_uppercase());

        let (dim, len) = if b0 == Some(b'Z') && b1 == Some(b'M') && word_ends_at(rest, 2) {
            (Dimension::XYZM, 2)
        } else if b0 == Some(b'Z') && word_ends_at(rest, 1) {
            (Dimension::XYZ, 1)
        } else if b0 == Some(b'M') && word_ends_at(rest, 1) {
            (Dimension::XYM, 1)
        } else {
            // No dim tag — restore position (was whitespace that belongs to the caller)
            self.pos = saved;
            return Ok(Dimension::XY);
        };

        self.pos += len;
        Ok(dim)
    }

    /// Reads a floating-point number, skipping leading whitespace.
    pub fn read_number(&mut self) -> Result<f64> {
        self.skip_whitespace();
        let rest = self.rest();
        // Consume sign, digits, decimal point, and exponent.
        let end = rest
            .as_bytes()
            .iter()
            .position(|&b| !matches!(b, b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-'))
            .unwrap_or(rest.len());
        if end == 0 {
            return Err(Error::InvalidWkt(format!(
                "expected number at position {}, got {:?}",
                self.pos,
                &rest[..rest.len().min(10)]
            )));
        }
        let s = &rest[..end];
        let v = s.parse::<f64>().map_err(|_| {
            Error::InvalidWkt(format!("invalid number {:?} at position {}", s, self.pos))
        })?;
        self.pos += end;
        Ok(v)
    }

    /// Returns true (without consuming) if the next non-whitespace char is `(`.
    pub fn peek_lparen(&mut self) -> bool {
        self.peek_nonws_char() == Some('(')
    }

    pub fn expect_lparen(&mut self) -> Result<()> {
        self.skip_whitespace();
        if self.rest().starts_with('(') {
            self.pos += 1;
            Ok(())
        } else {
            Err(Error::InvalidWkt(format!(
                "expected '(' at position {}, got {:?}",
                self.pos,
                self.rest().chars().next()
            )))
        }
    }

    pub fn expect_rparen(&mut self) -> Result<()> {
        self.skip_whitespace();
        if self.rest().starts_with(')') {
            self.pos += 1;
            Ok(())
        } else {
            Err(Error::InvalidWkt(format!(
                "expected ')' at position {}, got {:?}",
                self.pos,
                self.rest().chars().next()
            )))
        }
    }

    /// Reads the next non-whitespace token expecting `,` or `)`.
    /// Returns `true` for `,` (more items follow), `false` for `)` (list is done).
    /// The consumed token is not put back.
    pub fn read_comma_or_rparen(&mut self) -> Result<bool> {
        self.skip_whitespace();
        match self.rest().chars().next() {
            Some(',') => {
                self.pos += 1;
                Ok(true)
            }
            Some(')') => {
                self.pos += 1;
                Ok(false)
            }
            c => Err(Error::InvalidWkt(format!(
                "expected ',' or ')' at position {}, got {:?}",
                self.pos, c
            ))),
        }
    }

    /// Peeks at the next non-whitespace content:
    /// - If it starts with `EMPTY` (followed by a non-alphabetic char or end), consumes it and returns `true`.
    /// - If it starts with `(`, does NOT consume and returns `false`.
    /// - Otherwise, returns an error.
    pub fn read_empty_or_lparen(&mut self) -> Result<bool> {
        self.skip_whitespace();
        let rest = self.rest();
        if rest
            .get(..5)
            .is_some_and(|s| s.eq_ignore_ascii_case("EMPTY"))
            && word_ends_at(rest, 5)
        {
            self.pos += 5;
            Ok(true)
        } else if rest.starts_with('(') {
            Ok(false)
        } else {
            Err(Error::InvalidWkt(format!(
                "expected 'EMPTY' or '(' at position {}, got {:?}",
                self.pos,
                &rest[..rest.len().min(10)]
            )))
        }
    }

    pub fn expect_eof(&mut self) -> Result<()> {
        self.skip_whitespace();
        if self.rest().is_empty() {
            Ok(())
        } else {
            Err(Error::InvalidWkt(format!(
                "unexpected trailing content at position {}: {:?}",
                self.pos,
                &self.rest()[..self.rest().len().min(20)]
            )))
        }
    }
}

fn word_ends_at(s: &str, offset: usize) -> bool {
    s[offset..]
        .chars()
        .next()
        .is_none_or(|c| !c.is_alphabetic())
}
