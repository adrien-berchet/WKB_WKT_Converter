use std::fmt::Write as _;

pub(super) struct WktBuilder {
    buf: String,
}

impl WktBuilder {
    pub fn with_capacity(n: usize) -> Self {
        Self {
            buf: String::with_capacity(n),
        }
    }

    pub fn finish(self) -> String {
        self.buf
    }

    pub fn push_str(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    pub fn push_char(&mut self, c: char) {
        self.buf.push(c);
    }

    /// Writes space-separated coordinate values.
    pub fn push_coord(&mut self, coords: &[f64]) {
        for (i, &v) in coords.iter().enumerate() {
            if i > 0 {
                self.buf.push(' ');
            }
            push_f64(&mut self.buf, v);
        }
    }
}

fn push_f64(buf: &mut String, v: f64) {
    if v.is_finite() && v != 0.0 && v == v.trunc() && v.abs() < 1e15 {
        write!(buf, "{}", v as i64).unwrap();
    } else {
        write!(buf, "{v}").unwrap();
    }
}
