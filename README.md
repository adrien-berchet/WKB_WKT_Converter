# wkb_wkt_converter

A high-performance, zero-dependency Rust library for streaming conversion between
**WKT/EWKT** and **WKB/EWKB** geometry formats used in GIS systems (PostGIS, GDAL, etc.).

Exposes both a Rust API and Python bindings (via PyO3/maturin).

---

## Features

- All 7 OGC geometry types: `Point`, `LineString`, `Polygon`, `MultiPoint`, `MultiLineString`, `MultiPolygon`, `GeometryCollection`
- All coordinate dimensions: XY, XYZ, M, ZM
- Both **EWKB** (PostGIS flag-bit encoding) and **ISO WKB** (type+1000 offset encoding) as input
- **SRID** preservation (EWKT ↔ EWKB); option to split SRID from geometry
- Big-endian and little-endian WKB input; little-endian EWKB output
- `EMPTY` geometry support
- Hex WKB convenience helpers
- Strict error handling: malformed input returns a descriptive error

### Conversion strategy

WKB → WKT is a straightforward streaming read. WKT → WKB uses a
**seekable-buffer approach**: count fields (ring count, point count, etc.) are
written as `0` placeholders and patched in-place after the coordinates are
streamed, avoiding a two-pass scan of the input.

---

## Rust API

Add to `Cargo.toml`:

```toml
[dependencies]
wkb_wkt_converter = { path = "wkb_wkt_converter" }
```

### Functions

```rust
// WKB/EWKB → WKT/EWKT (SRID embedded as "SRID=N;" prefix when present)
pub fn wkb_to_wkt(wkb: &[u8]) -> Result<String>

// WKB/EWKB → WKT, SRID returned separately (not in the string)
pub fn wkb_to_wkt_split_srid(wkb: &[u8]) -> Result<(String, Option<u32>)>

// WKT/EWKT → EWKB bytes (SRID embedded in bytes when present)
pub fn wkt_to_wkb(wkt: &str) -> Result<Vec<u8>>

// WKT/EWKT → EWKB bytes, SRID returned separately (not in bytes)
pub fn wkt_to_wkb_split_srid(wkt: &str) -> Result<(Vec<u8>, Option<u32>)>

// WKT/EWKT → uppercase hex-encoded EWKB string
pub fn wkt_to_hex_wkb(wkt: &str) -> Result<String>

// Hex-encoded WKB/EWKB → WKT/EWKT string
pub fn hex_wkb_to_wkt(hex: &str) -> Result<String>
```

#### Generic converters

These functions accept **either** a WKT/EWKT string **or** a hex-encoded WKB/EWKB
string and detect the format automatically (a string composed entirely of hex
characters is treated as hex WKB; anything else as WKT).

```rust
pub fn text_to_wkb(text: &str, srid: SridMode) -> Result<Vec<u8>>
pub fn text_to_wkt(text: &str, srid: SridMode) -> Result<String>
pub fn text_to_hex_wkb(text: &str, srid: SridMode) -> Result<String>
```

`SridMode` controls SRID handling in the output:

| Variant | Behaviour |
|---|---|
| `SridMode::Auto` | Mirror the input — SRID kept if present, absent if not |
| `SridMode::Strip` | Always strip the SRID from the output |
| `SridMode::Set(n)` | Always embed SRID `n`, overriding whatever the input contains |

`text_to_wkt` also **normalises** WKT input (casing, whitespace) via a
round-trip through WKB.

### Example

```rust
use wkb_wkt_converter::{wkt_to_wkb, wkb_to_wkt, wkt_to_wkb_split_srid};
use wkb_wkt_converter::{text_to_wkt, text_to_hex_wkb, SridMode};

// Basic round-trip
let wkb = wkt_to_wkb("POINT (1 2)")?;
let wkt = wkb_to_wkt(&wkb)?;
assert_eq!(wkt, "POINT (1 2)");

// With SRID embedded
let wkb = wkt_to_wkb("SRID=4326;POINT Z (1 2 3)")?;
let wkt = wkb_to_wkt(&wkb)?;
assert_eq!(wkt, "SRID=4326;POINT Z (1 2 3)");

// SRID split from geometry
let (wkb, srid) = wkt_to_wkb_split_srid("SRID=4326;LINESTRING (0 0, 1 1)")?;
assert_eq!(srid, Some(4326));
// wkb contains a plain (non-EWKB) LineString

// All geometry types and dimensions work the same way
let wkb = wkt_to_wkb("MULTIPOLYGON ZM (((0 0 0 1, 1 0 0 1, 1 1 0 1, 0 0 0 1)))")?;
let wkt = wkb_to_wkt(&wkb)?;
assert_eq!(wkt, "MULTIPOLYGON ZM (((0 0 0 1, 1 0 0 1, 1 1 0 1, 0 0 0 1)))");

// Generic converters: input format (WKT or hex WKB) detected automatically
// Normalise WKT (casing, whitespace) — SridMode::Auto mirrors the input
let wkt = text_to_wkt("point(1 2)", SridMode::Auto)?;
assert_eq!(wkt, "POINT (1 2)");

// Add or override an SRID regardless of what the input contains
let hex = text_to_hex_wkb("POINT (1 2)", SridMode::Set(4326))?;
// hex is an EWKB string encoding SRID=4326;POINT (1 2)

// Strip the SRID even when the input is already EWKT
let wkt = text_to_wkt("SRID=4326;POINT (1 2)", SridMode::Strip)?;
assert_eq!(wkt, "POINT (1 2)");
```

### Error handling

All functions return `Result<_, wkb_wkt_converter::Error>`:

```rust
pub enum Error {
    InvalidWkt(String),
    InvalidWkb(String),
    UnsupportedGeometryType(u32),
}
```

---

## Python API

### Build and install

Requires [maturin](https://github.com/PyO3/maturin) and a Rust toolchain.

```sh
pip install maturin
maturin develop          # install into the current virtualenv (dev mode)
maturin build --release  # build a wheel
```

### Functions

```python
from wkb_wkt_converter import (
    wkb_to_wkt,
    wkb_to_wkt_split_srid,
    wkt_to_wkb,
    wkt_to_wkb_split_srid,
    wkt_to_hex_wkb,
    hex_wkb_to_wkt,
    # generic converters
    text_to_wkb,
    text_to_wkt,
    text_to_hex_wkb,
)
```

| Function | Input | Output |
|---|---|---|
| `wkb_to_wkt(wkb)` | `bytes` | `str` |
| `wkb_to_wkt_split_srid(wkb)` | `bytes` | `(str, int \| None)` |
| `wkt_to_wkb(wkt)` | `str` | `bytes` |
| `wkt_to_wkb_split_srid(wkt)` | `str` | `(bytes, int \| None)` |
| `wkt_to_hex_wkb(wkt)` | `str` | `str` |
| `hex_wkb_to_wkt(hex_wkb)` | `str` | `str` |

All functions raise `ValueError` on invalid input.

#### Generic converters

These three functions accept **either** a WKT/EWKT string **or** a hex-encoded
WKB/EWKB string and detect the format automatically.

| Function | Output |
|---|---|
| `text_to_wkb(text, srid=None)` | `bytes` |
| `text_to_wkt(text, srid=None)` | `str` |
| `text_to_hex_wkb(text, srid=None)` | `str` |

The `srid` keyword argument controls SRID handling in the output:

| Value | Behaviour |
|---|---|
| `None` *(default)* | Mirror the input — SRID kept if present, absent if not |
| `False` | Always strip the SRID from the output |
| `int` | Always embed this SRID, overriding whatever the input contains |

`text_to_wkt` also **normalises** WKT input (casing, whitespace) via a
round-trip through WKB.

### Example

```python
from wkb_wkt_converter import wkt_to_wkb, wkb_to_wkt, wkt_to_hex_wkb
from wkb_wkt_converter import text_to_wkt, text_to_hex_wkb

wkb = wkt_to_wkb("POINT (1 2)")
wkt = wkb_to_wkt(wkb)
assert wkt == "POINT (1 2)"

# EWKT with SRID
wkb = wkt_to_wkb("SRID=4326;POLYGON ((0 0, 1 0, 1 1, 0 0))")
wkt = wkb_to_wkt(wkb)
assert wkt == "SRID=4326;POLYGON ((0 0, 1 0, 1 1, 0 0))"

# Hex WKB (common PostGIS text format)
hex_wkb = wkt_to_hex_wkb("POINT (1 2)")
wkt = hex_wkb_to_wkt(hex_wkb)
assert wkt == "POINT (1 2)"

# Generic converters: input format detected automatically
wkt = text_to_wkt("point(1 2)")                      # normalise WKT
assert wkt == "POINT (1 2)"

wkt = text_to_wkt(hex_wkb)                           # hex WKB → WKT
assert wkt == "POINT (1 2)"

hex_out = text_to_hex_wkb("POINT (1 2)", srid=4326)  # add SRID
wkt = text_to_wkt(hex_out)
assert wkt == "SRID=4326;POINT (1 2)"

wkt = text_to_wkt("SRID=4326;POINT (1 2)", srid=False)  # strip SRID
assert wkt == "POINT (1 2)"
```

---

## Project layout

```
wkb_wkt_converter/          # core Rust library (zero runtime dependencies)
  src/
    lib.rs                  # public API
    error.rs
    types.rs                # GeomType, Dimension
    wkb_to_wkt/             # WKB reader + WKT builder
    wkt_to_wkb/             # WKT tokenizer + seekable WKB writer
  tests/
    wkb_to_wkt.rs
    wkt_to_wkb.rs
    text_to.rs

wkb_wkt_converter_py/       # Python bindings (PyO3 / maturin)
  src/lib.rs

pyproject.toml              # maturin build config
```

---

## Running tests

```sh
cargo test                  # run all Rust tests
cargo clippy -- -D warnings # lints
cargo fmt --check           # formatting
```
