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
- Per-member `EMPTY` support inside `MULTI*` geometries
- Hex WKB convenience helpers
- Memory-safe handling for malformed input: structurally invalid input returns
  descriptive errors, while trusted-valid fast paths may pass through malformed
  geometry bodies as invalid output bytes/text.

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
// WKB/EWKB → WKT/EWKT, with SRID output control
pub fn wkb_to_wkt(wkb: &[u8], srid: SridMode) -> Result<String>

// WKB/EWKB → WKT, SRID returned separately (not in the string)
pub fn wkb_to_wkt_split_srid(wkb: &[u8]) -> Result<(String, Option<u32>)>

// WKT/EWKT → EWKB bytes, with SRID output control
pub fn wkt_to_wkb(wkt: &str, srid: SridMode) -> Result<Vec<u8>>

// WKT/EWKT → EWKB bytes, SRID returned separately (not in bytes)
pub fn wkt_to_wkb_split_srid(wkt: &str) -> Result<(Vec<u8>, Option<u32>)>

// WKT/EWKT → uppercase hex-encoded EWKB string, with SRID output control
pub fn wkt_to_hex_wkb(wkt: &str, srid: SridMode) -> Result<String>

// Hex-encoded WKB/EWKB → WKT/EWKT string, with SRID output control
pub fn hex_wkb_to_wkt(hex: &str, srid: SridMode) -> Result<String>
```

#### Generic converters

These functions accept **either** a WKT/EWKT string **or** a hex-encoded WKB/EWKB
string and detect the format automatically (a non-empty, even-length string
composed entirely of hex characters is treated as hex WKB; anything else,
including odd-length all-hex text, is treated as WKT).

```rust
pub fn to_wkb(text: &str, srid: SridMode) -> Result<Vec<u8>>
pub fn to_wkt(text: &str, srid: SridMode, normalize_wkt: bool) -> Result<String>
pub fn to_hex_wkb(text: &str, srid: SridMode) -> Result<String>
```

`SridMode` controls SRID handling in the output of direct and generic converters:

| Variant | Behaviour |
|---|---|
| `SridMode::Auto` | Mirror the input — SRID kept if present, absent if not |
| `SridMode::Strip` | Always strip the SRID from the output |
| `SridMode::Set(n)` | Always embed SRID `n`, overriding whatever the input contains |

Validation notes:

- WKT coordinates must be finite. WKB coordinate payloads are treated as
  trusted-valid: all-`NaN` Point remains `POINT EMPTY`, while other `NaN` or
  infinity values may format as invalid WKT instead of raising an error.
- Direct WKB-to-WKT entry points still reject structurally invalid WKB such as
  truncation, unsupported type codes, excessive nesting, and trailing top-level
  bytes.
- For canonical little-endian Point, LineString, and Polygon EWKB hex input,
  `to_wkb(hex, SridMode::Strip | SridMode::Set(_))` and the equivalent
  `to_hex_wkb` paths patch only the top-level header. Malformed coordinate
  bodies or trailing bytes in those simple fast paths can pass through as
  invalid output. Big-endian, ISO-dimensional, collection, and non-canonical
  type headers fall back to a full normalising round-trip.
- `to_wkb(hex, SridMode::Auto)` returns decoded bytes without WKB
  structure validation, and `to_hex_wkb(hex, SridMode::Auto)` validates
  and uppercases the hex text only.
- `GeometryCollection` dimension tags are not inherited by child geometries.
  An XY collection may contain heterogeneous immediate children. A Z, M, or ZM
  collection requires each immediate child to declare the same dimension.
- WKT and WKB parsing reject geometry nesting beyond the implementation depth
  limit of 128.

`to_wkt` accepts a `normalize_wkt: bool` parameter.  When `true`, WKT
input is normalised (canonical casing, spacing, coordinate formatting) via a
round-trip through WKB.  When `false`, only the SRID prefix is adjusted —
**no validation is performed: malformed WKT is returned without error.**
Leading/trailing whitespace is always trimmed regardless of this flag.  Hex
WKB input is always decoded to normalised WKT regardless of this flag.
Odd-length all-hex input is not detected as hex WKB; with `normalize_wkt=false`
it follows the same unvalidated WKT fast path.

### Example

```rust
use wkb_wkt_converter::{wkt_to_wkb, wkb_to_wkt, wkt_to_wkb_split_srid, hex_wkb_to_wkt};
use wkb_wkt_converter::{to_wkt, to_hex_wkb, SridMode};

// Basic round-trip
let wkb = wkt_to_wkb("POINT (1 2)", SridMode::Auto)?;
let wkt = wkb_to_wkt(&wkb, SridMode::Auto)?;
assert_eq!(wkt, "POINT (1 2)");

// With SRID embedded
let wkb = wkt_to_wkb("SRID=4326;POINT Z (1 2 3)", SridMode::Auto)?;
let wkt = wkb_to_wkt(&wkb, SridMode::Auto)?;
assert_eq!(wkt, "SRID=4326;POINT Z (1 2 3)");

// SRID split from geometry
let (wkb, srid) = wkt_to_wkb_split_srid("SRID=4326;LINESTRING (0 0, 1 1)")?;
assert_eq!(srid, Some(4326));
// wkb contains a plain (non-EWKB) LineString

// All geometry types and dimensions work the same way
let wkb = wkt_to_wkb(
    "MULTIPOLYGON ZM (((0 0 0 1, 1 0 0 1, 1 1 0 1, 0 0 0 1)))",
    SridMode::Auto,
)?;
let wkt = wkb_to_wkt(&wkb, SridMode::Auto)?;
assert_eq!(wkt, "MULTIPOLYGON ZM (((0 0 0 1, 1 0 0 1, 1 1 0 1, 0 0 0 1)))");

// Generic converters: input format (WKT or hex WKB) detected automatically
// Normalise WKT (casing, whitespace) — SridMode::Auto mirrors the input SRID
let wkt = to_wkt("point(1 2)", SridMode::Auto, true)?;
assert_eq!(wkt, "POINT (1 2)");

// Add or override an SRID regardless of what the input contains
let hex = to_hex_wkb("POINT (1 2)", SridMode::Set(4326))?;
// hex is an EWKB string encoding SRID=4326;POINT (1 2)
let wkt = hex_wkb_to_wkt(&hex, SridMode::Strip)?;
assert_eq!(wkt, "POINT (1 2)");

// Strip the SRID without re-encoding (fast path)
let wkt = to_wkt("SRID=4326;POINT (1 2)", SridMode::Strip, false)?;
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
Wheels include PEP 561 type stubs for static type checkers and IDEs.

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
    to_wkb,
    to_wkt,
    to_hex_wkb,
)
```

| Function | Input | Output |
|---|---|---|
| `wkb_to_wkt(wkb, srid=None)` | bytes-like WKB/EWKB | `str` |
| `wkb_to_wkt_split_srid(wkb)` | bytes-like WKB/EWKB | `(str, int \| None)` |
| `wkt_to_wkb(wkt, srid=None)` | `str` | `bytes` |
| `wkt_to_wkb_split_srid(wkt)` | `str` | `(bytes, int \| None)` |
| `wkt_to_hex_wkb(wkt, srid=None)` | `str` | `str` |
| `hex_wkb_to_wkt(hex_wkb, srid=None)` | `str` | `str` |

WKB arguments in the Python API accept `bytes`, `bytearray`, `memoryview`, and
other C-contiguous one-byte buffer objects. They are always treated as raw
WKB/EWKB, not as encoded text.

All functions above raise `ValueError` on invalid input. (See `to_wkt` below for an exception when `normalize_wkt=False`.)

#### Generic converters

These three functions accept **either** a WKT/EWKT string **or** a hex-encoded
WKB/EWKB string and detect the format automatically. A non-empty, even-length
string composed entirely of hex characters is treated as hex WKB; anything else,
including odd-length all-hex text, is treated as WKT.

| Function | Output |
|---|---|
| `to_wkb(text, srid=None)` | `bytes` |
| `to_wkt(text, srid=None, normalize_wkt=False)` | `str` |
| `to_hex_wkb(text, srid=None)` | `str` |

The `srid` keyword argument on direct and generic converters controls SRID
handling in the output:

| Value | Behaviour |
|---|---|
| `None` *(default)* | Mirror the input — SRID kept if present, absent if not |
| `False` | Always strip the SRID from the output |
| `int` | Always embed this SRID, overriding whatever the input contains |

Validation behavior matches the Rust API: WKT coordinates must be finite, while
WKB coordinate payloads are treated as trusted-valid. Simple little-endian EWKB
hex inputs under `srid=False` or an integer SRID are patched at the top-level
header without scanning the body, so malformed bodies or trailing bytes can pass
through as invalid output. `to_wkb(hex, srid=None)` returns decoded bytes
without WKB structure validation, and `to_hex_wkb(hex, srid=None)`
validates and uppercases the hex text only.

`to_wkt` accepts a `normalize_wkt` keyword argument (default `False`).
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
from wkb_wkt_converter import to_wkt, to_hex_wkb

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

srid_hex_wkb = wkt_to_hex_wkb("SRID=4326;POINT (1 2)")
wkt = hex_wkb_to_wkt(srid_hex_wkb, srid=False)
assert wkt == "POINT (1 2)"

# Generic converters: input format detected automatically
wkt = to_wkt("point(1 2)", normalize_wkt=True)  # normalise WKT
assert wkt == "POINT (1 2)"

wkt = to_wkt(hex_wkb)                           # hex WKB → WKT (always normalised)
assert wkt == "POINT (1 2)"

hex_out = to_hex_wkb("POINT (1 2)", srid=4326)  # add SRID
wkt = to_wkt(hex_out)
assert wkt == "SRID=4326;POINT (1 2)"

wkt = to_wkt("SRID=4326;POINT (1 2)", srid=False)  # strip SRID (fast path)
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

For local performance history across commits, use
[airspeed velocity](https://asv.readthedocs.io/):

```sh
pip install ".[asv]"
asv check --python=same
asv run --python=same --quick --show-stderr --dry-run
asv run HEAD^! --quick --show-stderr --dry-run
asv run main..HEAD --skip-existing-commits --show-stderr
asv run ALL --skip-existing-commits --show-stderr
asv publish
asv preview
asv compare main HEAD --split
```

<!-- BENCHMARK_RESULTS_START -->

*2026-05-05 — Python 3.12.3 — 12th Gen Intel(R) Core(TM) i7-12700KF*

Times are mean latency per call (lower is better). Speedup = shapely mean ÷ wkb_wkt_converter mean.

### Basic conversions

#### `wkt_to_wkb`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 140 ns | 5.2 µs | 37.3× |
| LineString (5 pts) | 233 ns | 6.2 µs | 26.7× |
| Polygon (5 pts) | 247 ns | 6.2 µs | 25.2× |
| GeometryCollection | 387 ns | 8.3 µs | 21.5× |
| MultiPolygon | 533 ns | 9.9 µs | 18.6× |
| LineString (1000 pts) | 23.3 µs | 209.1 µs | 9.0× |
| Polygon (1000 pts) | 40.7 µs | 444.0 µs | 10.9× |

#### `wkb_to_wkt`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 148 ns | 3.9 µs | 26.3× |
| LineString (5 pts) | 240 ns | 4.4 µs | 18.3× |
| Polygon (5 pts) | 140 ns | 4.4 µs | 31.2× |
| GeometryCollection | 213 ns | 5.1 µs | 24.0× |
| MultiPolygon | 667 ns | 6.9 µs | 10.3× |
| LineString (1000 pts) | 38.9 µs | 118.2 µs | 3.0× |
| Polygon (1000 pts) | 118.3 µs | 140.8 µs | 1.2× |

#### `wkt_to_hex_wkb`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 164 ns | 5.2 µs | 31.6× |
| LineString (5 pts) | 337 ns | 6.4 µs | 19.0× |
| Polygon (5 pts) | 365 ns | 6.4 µs | 17.6× |
| GeometryCollection | 494 ns | 8.7 µs | 17.5× |
| MultiPolygon | 847 ns | 9.8 µs | 11.6× |
| LineString (1000 pts) | 36.8 µs | 230.0 µs | 6.2× |
| Polygon (1000 pts) | 54.5 µs | 456.0 µs | 8.4× |

#### `hex_wkb_to_wkt`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 188 ns | 4.0 µs | 21.3× |
| LineString (5 pts) | 331 ns | 4.6 µs | 13.9× |
| Polygon (5 pts) | 219 ns | 4.6 µs | 21.1× |
| GeometryCollection | 352 ns | 5.6 µs | 15.8× |
| MultiPolygon | 854 ns | 6.8 µs | 8.0× |
| LineString (1000 pts) | 47.2 µs | 133.3 µs | 2.8× |
| Polygon (1000 pts) | 126.7 µs | 156.0 µs | 1.2× |

### Generic `to_*` converters

#### `to_wkb(wkt)`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 146 ns | 5.4 µs | 36.8× |
| LineString (5 pts) | 241 ns | 6.7 µs | 27.7× |
| Polygon (5 pts) | 273 ns | 6.5 µs | 23.9× |
| GeometryCollection | 391 ns | 8.5 µs | 21.6× |
| MultiPolygon | 565 ns | 9.6 µs | 17.0× |
| LineString (1000 pts) | 24.0 µs | 215.0 µs | 9.0× |
| Polygon (1000 pts) | 41.2 µs | 452.8 µs | 11.0× |

#### `to_wkb(hex_wkb)`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 79 ns | 4.5 µs | 57.4× |
| LineString (5 pts) | 116 ns | 4.8 µs | 41.6× |
| Polygon (5 pts) | 117 ns | 4.7 µs | 40.4× |
| GeometryCollection | 160 ns | 5.3 µs | 33.2× |
| MultiPolygon | 215 ns | 5.9 µs | 27.7× |
| LineString (1000 pts) | 7.2 µs | 42.8 µs | 5.9× |
| Polygon (1000 pts) | 7.2 µs | 43.3 µs | 6.0× |

#### `to_wkt(wkt)` — `normalize_wkt=False` (fast path, no WKB round-trip)

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 62 ns | 4.8 µs | 76.8× |
| LineString (5 pts) | 60 ns | 6.1 µs | 101.3× |
| Polygon (5 pts) | 61 ns | 6.1 µs | 100.2× |
| GeometryCollection | 65 ns | 8.4 µs | 129.9× |
| MultiPolygon | 66 ns | 10.6 µs | 160.0× |
| LineString (1000 pts) | 521 ns | 306.5 µs | 588.0× |
| Polygon (1000 pts) | 2.1 µs | 547.8 µs | 267.2× |

#### `to_wkt(wkt)` — `normalize_wkt=True` (full WKT→WKB→WKT round-trip)

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 346 ns | 4.8 µs | 13.8× |
| LineString (5 pts) | 550 ns | 6.1 µs | 11.1× |
| Polygon (5 pts) | 458 ns | 6.1 µs | 13.3× |
| GeometryCollection | 685 ns | 8.4 µs | 12.3× |
| MultiPolygon | 1.3 µs | 10.6 µs | 8.1× |
| LineString (1000 pts) | 62.9 µs | 306.5 µs | 4.9× |
| Polygon (1000 pts) | 162.3 µs | 547.8 µs | 3.4× |

#### `to_wkt(hex_wkb)`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 195 ns | 4.1 µs | 21.1× |
| LineString (5 pts) | 349 ns | 4.6 µs | 13.2× |
| Polygon (5 pts) | 231 ns | 4.7 µs | 20.1× |
| GeometryCollection | 359 ns | 5.5 µs | 15.4× |
| MultiPolygon | 873 ns | 6.7 µs | 7.7× |
| LineString (1000 pts) | 46.8 µs | 132.5 µs | 2.8× |
| Polygon (1000 pts) | 126.9 µs | 155.2 µs | 1.2× |

#### `to_hex_wkb(wkt)`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 177 ns | 5.4 µs | 30.4× |
| LineString (5 pts) | 350 ns | 6.4 µs | 18.3× |
| Polygon (5 pts) | 375 ns | 6.8 µs | 18.3× |
| GeometryCollection | 524 ns | 8.7 µs | 16.6× |
| MultiPolygon | 869 ns | 9.8 µs | 11.3× |
| LineString (1000 pts) | 37.3 µs | 228.1 µs | 6.1× |
| Polygon (1000 pts) | 54.7 µs | 463.0 µs | 8.5× |

#### `to_hex_wkb(hex_wkb)`

| Geometry | wkb_wkt_converter | shapely | Speedup |
|:---|---:|---:|---:|
| Point | 86 ns | 4.5 µs | 52.6× |
| LineString (5 pts) | 169 ns | 4.9 µs | 29.1× |
| Polygon (5 pts) | 174 ns | 4.9 µs | 28.5× |
| GeometryCollection | 238 ns | 5.5 µs | 23.0× |
| MultiPolygon | 366 ns | 6.2 µs | 17.0× |
| LineString (1000 pts) | 14.8 µs | 56.7 µs | 3.8× |
| Polygon (1000 pts) | 15.1 µs | 58.1 µs | 3.8× |

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
    generic_converters.rs

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
