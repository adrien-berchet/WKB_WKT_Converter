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
string and detect the format automatically (a non-empty, even-length string
composed entirely of hex characters is treated as hex WKB; anything else,
including odd-length all-hex text, is treated as WKT).

```rust
pub fn text_to_wkb(text: &str, srid: SridMode) -> Result<Vec<u8>>
pub fn text_to_wkt(text: &str, srid: SridMode, normalize_wkt: bool) -> Result<String>
pub fn text_to_hex_wkb(text: &str, srid: SridMode) -> Result<String>
```

`SridMode` controls SRID handling in the output:

| Variant | Behaviour |
|---|---|
| `SridMode::Auto` | Mirror the input — SRID kept if present, absent if not |
| `SridMode::Strip` | Always strip the SRID from the output |
| `SridMode::Set(n)` | Always embed SRID `n`, overriding whatever the input contains |

`text_to_wkt` accepts a `normalize_wkt: bool` parameter.  When `true`, WKT
input is normalised (canonical casing, spacing, coordinate formatting) via a
round-trip through WKB.  When `false`, only the SRID prefix is adjusted —
**no validation is performed: malformed WKT is returned without error.**
Leading/trailing whitespace is always trimmed regardless of this flag.  Hex
WKB input is always decoded to normalised WKT regardless of this flag.
Odd-length all-hex input is not detected as hex WKB; with `normalize_wkt=false`
it follows the same unvalidated WKT fast path.

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
// Normalise WKT (casing, whitespace) — SridMode::Auto mirrors the input SRID
let wkt = text_to_wkt("point(1 2)", SridMode::Auto, true)?;
assert_eq!(wkt, "POINT (1 2)");

// Add or override an SRID regardless of what the input contains
let hex = text_to_hex_wkb("POINT (1 2)", SridMode::Set(4326))?;
// hex is an EWKB string encoding SRID=4326;POINT (1 2)

// Strip the SRID without re-encoding (fast path)
let wkt = text_to_wkt("SRID=4326;POINT (1 2)", SridMode::Strip, false)?;
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

All functions above raise `ValueError` on invalid input. (See `text_to_wkt` below for an exception when `normalize_wkt=False`.)

#### Generic converters

These three functions accept **either** a WKT/EWKT string **or** a hex-encoded
WKB/EWKB string and detect the format automatically. A non-empty, even-length
string composed entirely of hex characters is treated as hex WKB; anything else,
including odd-length all-hex text, is treated as WKT.

| Function | Output |
|---|---|
| `text_to_wkb(text, srid=None)` | `bytes` |
| `text_to_wkt(text, srid=None, normalize_wkt=False)` | `str` |
| `text_to_hex_wkb(text, srid=None)` | `str` |

The `srid` keyword argument controls SRID handling in the output:

| Value | Behaviour |
|---|---|
| `None` *(default)* | Mirror the input — SRID kept if present, absent if not |
| `False` | Always strip the SRID from the output |
| `int` | Always embed this SRID, overriding whatever the input contains |

`text_to_wkt` accepts a `normalize_wkt` keyword argument (default `False`).
When `True`, WKT input is normalised (canonical casing, spacing, coordinate
formatting) via a round-trip through WKB.  When `False` (the default), only
the SRID prefix is adjusted — **no validation is performed: malformed WKT is
returned without raising an error.**  Leading/trailing whitespace is always
stripped regardless of this flag.  Hex WKB input is always decoded to
normalised WKT regardless of this flag. Odd-length all-hex input is not detected
as hex WKB; with `normalize_wkt=False` it follows the same unvalidated WKT fast
path.

### Example

```python
from wkb_wkt_converter import wkt_to_wkb, wkb_to_wkt, wkt_to_hex_wkb, hex_wkb_to_wkt
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
wkt = text_to_wkt("point(1 2)", normalize_wkt=True)  # normalise WKT
assert wkt == "POINT (1 2)"

wkt = text_to_wkt(hex_wkb)                           # hex WKB → WKT (always normalised)
assert wkt == "POINT (1 2)"

hex_out = text_to_hex_wkb("POINT (1 2)", srid=4326)  # add SRID
wkt = text_to_wkt(hex_out)
assert wkt == "SRID=4326;POINT (1 2)"

wkt = text_to_wkt("SRID=4326;POINT (1 2)", srid=False)  # strip SRID (fast path)
assert wkt == "POINT (1 2)"
```

---

## Benchmarks

Comparison against [shapely](https://github.com/shapely/shapely) 2.x.
Regenerate with:

```sh
pip install ".[benchmark]"
python scripts/update_readme_benchmarks.py
# or, using an already-saved JSON:
python scripts/update_readme_benchmarks.py --json benchmark_results.json
```

<!-- BENCHMARK_RESULTS_START -->

*2026-05-02 — Python 3.11.15 — Intel(R) Xeon(R) Processor @ 2.10GHz*

Times are mean latency per call (lower is better). Speedup = shapely mean ÷ wkb_wkt_converter mean.

### Basic conversions

#### `wkt_to_wkb`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 885 ns | 14.9 µs | 16.8× |
| LineString (5 pts) | 1.5 µs | 24.9 µs | 17.0× |
| Polygon (5 pts) | 1.9 µs | 31.7 µs | 17.1× |
| GeometryCollection | 2.2 µs | 42.2 µs | 19.3× |
| MultiPolygon | 2.4 µs | 25.6 µs | 10.5× |
| LineString (1000 pts) | 91.4 µs | 539.8 µs | 5.9× |
| Polygon (1000 pts) | 133.7 µs | 842.9 µs | 6.3× |

#### `wkb_to_wkt`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 446 ns | 14.2 µs | 31.9× |
| LineString (5 pts) | 1.2 µs | 15.9 µs | 13.7× |
| Polygon (5 pts) | 1.1 µs | 19.4 µs | 17.0× |
| GeometryCollection | 1.7 µs | 25.0 µs | 14.4× |
| MultiPolygon | 2.5 µs | 17.7 µs | 7.1× |
| LineString (1000 pts) | 90.7 µs | 298.8 µs | 3.3× |
| Polygon (1000 pts) | 277.0 µs | 352.0 µs | 1.3× |

#### `wkt_to_hex_wkb`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 1.6 µs | 19.5 µs | 12.1× |
| LineString (5 pts) | 3.6 µs | 26.0 µs | 7.1× |
| Polygon (5 pts) | 5.0 µs | 28.4 µs | 5.6× |
| GeometryCollection | 7.9 µs | 46.9 µs | 6.0× |
| MultiPolygon | 10.6 µs | 46.4 µs | 4.4× |
| LineString (1000 pts) | 447.9 µs | 652.7 µs | 1.5× |
| Polygon (1000 pts) | 440.8 µs | 1.1 ms | 2.5× |

#### `hex_wkb_to_wkt`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 819 ns | 13.8 µs | 16.9× |
| LineString (5 pts) | 1.9 µs | 19.2 µs | 10.0× |
| Polygon (5 pts) | 1.7 µs | 21.8 µs | 12.8× |
| GeometryCollection | 2.8 µs | 28.3 µs | 10.2× |
| MultiPolygon | 4.3 µs | 24.9 µs | 5.9× |
| LineString (1000 pts) | 169.0 µs | 364.9 µs | 2.2× |
| Polygon (1000 pts) | 473.6 µs | 362.5 µs | 0.8× |

### Generic `text_to_*` converters

#### `text_to_wkb(wkt)`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 1.0 µs | 29.3 µs | 28.9× |
| LineString (5 pts) | 1.4 µs | 29.1 µs | 20.9× |
| Polygon (5 pts) | 1.2 µs | 35.3 µs | 30.4× |
| GeometryCollection | 2.2 µs | 27.2 µs | 12.2× |
| MultiPolygon | 2.6 µs | 48.5 µs | 19.0× |
| LineString (1000 pts) | 84.6 µs | 628.6 µs | 7.4× |
| Polygon (1000 pts) | 124.6 µs | 956.2 µs | 7.7× |

#### `text_to_wkb(hex_wkb)`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 672 ns | 15.1 µs | 22.5× |
| LineString (5 pts) | 1.3 µs | 18.2 µs | 13.7× |
| Polygon (5 pts) | 1.3 µs | 30.1 µs | 22.6× |
| GeometryCollection | 1.8 µs | 23.6 µs | 13.0× |
| MultiPolygon | 2.6 µs | 18.4 µs | 7.1× |
| LineString (1000 pts) | 96.7 µs | 116.2 µs | 1.2× |
| Polygon (1000 pts) | 429.6 µs | 129.0 µs | 0.3× |

#### `text_to_wkt(wkt)` — `normalize_wkt=False` (fast path, no WKB round-trip)

| Geometry | wkb_wkt_converter |
|:---|---:|
| Point | 479 ns |
| LineString (5 pts) | 141 ns |
| Polygon (5 pts) | 377 ns |
| GeometryCollection | 145 ns |
| MultiPolygon | 447 ns |
| LineString (1000 pts) | 1.4 µs |
| Polygon (1000 pts) | 5.8 µs |

#### `text_to_wkt(wkt)` — `normalize_wkt=True` (full WKT→WKB→WKT round-trip)

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 1.0 µs | 31.5 µs | 31.1× |
| LineString (5 pts) | 2.6 µs | 31.8 µs | 12.1× |
| Polygon (5 pts) | 2.8 µs | 18.6 µs | 6.5× |
| GeometryCollection | 4.0 µs | 23.9 µs | 6.0× |
| MultiPolygon | 5.7 µs | 31.2 µs | 5.5× |
| LineString (1000 pts) | 185.6 µs | 693.4 µs | 3.7× |
| Polygon (1000 pts) | 387.5 µs | 909.9 µs | 2.3× |

#### `text_to_wkt(hex_wkb)`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 983 ns | 16.2 µs | 16.5× |
| LineString (5 pts) | 2.0 µs | 22.0 µs | 11.2× |
| Polygon (5 pts) | 1.7 µs | 15.9 µs | 9.3× |
| GeometryCollection | 3.1 µs | 23.4 µs | 7.6× |
| MultiPolygon | 5.0 µs | 29.7 µs | 6.0× |
| LineString (1000 pts) | 165.3 µs | 366.8 µs | 2.2× |
| Polygon (1000 pts) | 644.1 µs | 411.9 µs | 0.6× |

#### `text_to_hex_wkb(wkt)`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 1.6 µs | 21.9 µs | 13.9× |
| LineString (5 pts) | 4.8 µs | 29.4 µs | 6.2× |
| Polygon (5 pts) | 4.8 µs | 23.2 µs | 4.8× |
| GeometryCollection | 7.4 µs | 31.6 µs | 4.3× |
| MultiPolygon | 11.2 µs | 44.4 µs | 4.0× |
| LineString (1000 pts) | 485.1 µs | 733.0 µs | 1.5× |
| Polygon (1000 pts) | 457.5 µs | 994.6 µs | 2.2× |

#### `text_to_hex_wkb(hex_wkb)`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 1.5 µs | 13.9 µs | 9.1× |
| LineString (5 pts) | 4.5 µs | 13.6 µs | 3.0× |
| Polygon (5 pts) | 4.3 µs | 14.5 µs | 3.4× |
| GeometryCollection | 6.8 µs | 17.5 µs | 2.6× |
| MultiPolygon | 9.5 µs | 15.5 µs | 1.6× |
| LineString (1000 pts) | 546.2 µs | 145.2 µs | 0.3× |
| Polygon (1000 pts) | 772.3 µs | 147.2 µs | 0.2× |

<!-- BENCHMARK_RESULTS_END -->

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
