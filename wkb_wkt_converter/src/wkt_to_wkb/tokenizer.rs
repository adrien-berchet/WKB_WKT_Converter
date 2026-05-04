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
        // Use a case-insensitive prefix check on the first 5 ASCII bytes.
        // `get` keeps malformed non-ASCII prefixes on the normal error path.
        if self
            .rest()
            .get(..5)
            .is_some_and(|p| p.eq_ignore_ascii_case("SRID="))
        {
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
            Some(b'G') if matches_word(rest, "GEOMETRYCOLLECTION") => {
                (GeomType::GeometryCollection, 18)
            }
            Some(b'M') if matches_word(rest, "MULTILINESTRING") => (GeomType::MultiLineString, 15),
            Some(b'M') if matches_word(rest, "MULTIPOLYGON") => (GeomType::MultiPolygon, 12),
            Some(b'M') if matches_word(rest, "MULTIPOINT") => (GeomType::MultiPoint, 10),
            Some(b'L') if matches_word(rest, "LINESTRING") => (GeomType::LineString, 10),
            Some(b'P') if matches_word(rest, "POLYGON") => (GeomType::Polygon, 7),
            Some(b'P') if matches_word(rest, "POINT") => (GeomType::Point, 5),
            _ => {
                return Err(Error::InvalidWkt(format!(
                    "unknown geometry type at position {}: {:?}",
                    self.pos,
                    snippet(rest, 20)
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
                snippet(rest, 10)
            )));
        }
        let s = &rest[..end];
        let v = s.parse::<f64>().map_err(|_| {
            Error::InvalidWkt(format!("invalid number {:?} at position {}", s, self.pos))
        })?;
        if !v.is_finite() {
            return Err(Error::InvalidWkt(format!(
                "non-finite coordinate {:?} at position {}",
                s, self.pos
            )));
        }
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

    pub fn try_read_empty(&mut self) -> Result<bool> {
        self.skip_whitespace();
        let rest = self.rest();
        if !rest
            .get(..5)
            .is_some_and(|s| s.eq_ignore_ascii_case("EMPTY"))
        {
            return Ok(false);
        }
        if word_ends_at(rest, 5) {
            self.pos += 5;
            Ok(true)
        } else {
            Err(Error::InvalidWkt(format!(
                "expected delimiter after EMPTY at position {}",
                self.pos + 5
            )))
        }
    }

    /// Peeks at the next non-whitespace content:
    /// - If it is `EMPTY`, consumes it and returns `true`.
    /// - If it starts with `(`, does NOT consume and returns `false`.
    /// - Otherwise, returns an error.
    pub fn read_empty_or_lparen(&mut self) -> Result<bool> {
        if self.try_read_empty()? {
            Ok(true)
        } else if self.rest().starts_with('(') {
            Ok(false)
        } else {
            Err(Error::InvalidWkt(format!(
                "expected 'EMPTY' or '(' at position {}, got {:?}",
                self.pos,
                snippet(self.rest(), 10)
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
                snippet(self.rest(), 20)
            )))
        }
    }
}

fn matches_word(s: &str, word: &str) -> bool {
    s.get(..word.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(word))
        && word_ends_at(s, word.len())
}

fn word_ends_at(s: &str, offset: usize) -> bool {
    s.get(offset..)
        .and_then(|tail| tail.chars().next())
        .is_none_or(|c| !c.is_alphabetic())
}

fn snippet(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}
