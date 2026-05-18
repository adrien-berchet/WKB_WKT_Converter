# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## Commands

```sh
# Rust tests, lint, formatting
cargo test --workspace           # run all Rust tests
cargo test --package wkb_wkt_converter -- <test_name>  # run a single test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all                  # apply formatting
cargo fmt --all -- --check       # check only (CI mode)

# Fuzzing (requires cargo-fuzz; fuzz crate is isolated under fuzz/)
mkdir -p fuzz/corpus/wkb_bytes fuzz/corpus/wkt_text fuzz/corpus/generic_input fuzz/corpus/structured_roundtrip fuzz/corpus/deep_nesting
cargo fuzz run --fuzz-dir fuzz wkb_bytes fuzz/corpus/wkb_bytes fuzz/seeds/wkb_bytes
cargo fuzz run --fuzz-dir fuzz wkt_text fuzz/corpus/wkt_text fuzz/seeds/wkt_text
cargo fuzz run --fuzz-dir fuzz generic_input fuzz/corpus/generic_input fuzz/seeds/generic_input
cargo fuzz run --fuzz-dir fuzz structured_roundtrip fuzz/corpus/structured_roundtrip fuzz/seeds/structured_roundtrip
cargo fuzz run --fuzz-dir fuzz deep_nesting fuzz/corpus/deep_nesting fuzz/seeds/deep_nesting

# Coverage (must reach 100% line coverage — CI enforces this)
cargo llvm-cov --package wkb_wkt_converter --fail-under-lines 100

# Python bindings (requires maturin and a Rust toolchain)
maturin develop                  # build and install into the active virtualenv
pytest                           # run Python binding tests

# ASV local benchmark history
pip install ".[asv]"
asv check --python=same
asv run --python=same --quick --show-stderr --dry-run
asv run HEAD^! --quick --show-stderr --dry-run

# Python binding coverage (instruments Rust code via LLVM when called from Python)
source <(cargo llvm-cov show-env --export-prefix)
maturin build
pip install --force-reinstall target/wheels/*.whl
pytest
cargo llvm-cov report --package wkb_wkt_converter_py --fail-under-lines 100
```

## Test coverage

**Line coverage must be 100%** for both the `wkb_wkt_converter` crate and the
`wkb_wkt_converter_py` Python bindings crate. CI enforces this with
`--fail-under-lines 100`. When adding or changing code:

- Every new code path must be exercised by at least one test.
- Run `cargo llvm-cov --package wkb_wkt_converter --fail-under-lines 100` locally
  before pushing to confirm core library coverage is maintained.
- For Python binding coverage, use the `source <(cargo llvm-cov show-env --export-prefix)`
  approach documented in the Commands section above — this instruments the Rust
  extension module while pytest drives execution.
- If a line is genuinely unreachable (e.g. a defensive branch that cannot be
  triggered through the public API), prefer removing it over marking it as
  excluded — dead code should not exist in this codebase.

## Performance benchmarks

When adding a new public function exported by the `wkb_wkt_converter` Python
module, update the ASV benchmark suite in `asv_benchmarks/python_api.py` in the
same change. Add a converter-only `time_*` benchmark for at least one
representative successful call path, using the existing representative geometry
set when applicable.

This applies to new public Python API functions, not internal helpers,
Rust-only APIs, or behavior-only changes to existing functions unless they
introduce a materially different performance path.

For WKB/hex inputs, prefer fixed or independently generated fixtures rather than
calling the converter under test during benchmark setup. Keep importing
`wkb_wkt_converter` inside benchmark `setup` methods so ASV discovery does not
require the extension module to be installed yet. Use `asv check --python=same`
as the quick ASV sanity check when the ASV extra is installed.

## Architecture

This is a Rust workspace with two crates and a Python layer on top:

```
wkb_wkt_converter/      core Rust library (zero runtime dependencies)
wkb_wkt_converter_py/   PyO3 bindings; thin wrappers that map Rust errors to ValueError
tests/                  Python tests (pytest)
```

### Core library (`wkb_wkt_converter/src/`)

**`lib.rs`** — public API surface: direct conversion functions (`wkb_to_wkt`, `wkb_to_wkt_split_srid`, `wkt_to_wkb`, `wkt_to_wkb_split_srid`, `wkt_to_hex_wkb`, `hex_wkb_to_wkt`) plus generic input-detecting converters (`to_wkb`, `to_wkt`, `to_hex_wkb`). The `_split_srid` variants return the SRID separately rather than embedding it in the output.

**`types.rs`** — `GeomType` (7 OGC types), `Dimension` (XY/XYZ/XYM/XYZM), and the EWKB flag-bit constants (`EWKB_Z`, `EWKB_M`, `EWKB_SRID`).

**`error.rs`** — three-variant `Error` enum (`InvalidWkt`, `InvalidWkb`, `UnsupportedGeometryType`).

#### WKB → WKT (`wkb_to_wkt/`)

- **`reader.rs`** (`WkbReader`): byte-cursor that reads the byte-order marker, then the 4-byte type code. The type code is decoded for both EWKB (high-bit flags) and ISO WKB (type+1000/2000/3000 offsets). Sub-geometries in MULTI* types each carry their own full WKB header.
- **`builder.rs`** (`WktBuilder`): string accumulator. `push_f64` renders whole-valued floats as integers (`1` not `1.0`) and uses Rust's default float formatter otherwise.

#### WKT → WKB (`wkt_to_wkb/`)

- **`tokenizer.rs`** (`Tokenizer`): hand-written cursor over the WKT `&str`. Handles case-insensitive keywords, optional whitespace, dimension tags (Z / M / ZM), `EMPTY`, and the `SRID=N;` prefix.
- **`writer.rs`** (`WkbWriter`): byte buffer with a **seekable-patch pattern** — `reserve_u32()` writes a `0x00000000` placeholder and returns its offset; `patch_u32(pos, value)` seeks back to fill it in once the real count is known. This avoids a two-pass scan of the input. All output is little-endian.
- **`mod.rs`**: recursive descent parser that calls `parse_geometry` → type-specific `parse_*` functions. `POINT EMPTY` encodes all coordinates as NaN (PostGIS convention). `MULTIPOINT` auto-detects ISO form `((x y))` vs bare form `(x y)`.

### Python bindings (`wkb_wkt_converter_py/src/lib.rs`)

Each public Rust function is wrapped in a `#[pyfunction]` that converts `wkb_wkt_converter::Error` into `PyValueError`. The `#![allow(clippy::useless_conversion)]` suppression at the top is a false-positive workaround required by PyO3's macro expansion.

## Key invariants

- **Output is always little-endian EWKB** regardless of the endianness of the input WKB.
- **SRID is only in the top-level header**, never in sub-geometry headers of MULTI* or GeometryCollection.
- Both EWKB (flag-bit dimension encoding) and ISO WKB (type-offset dimension encoding) are accepted as input; only EWKB is produced as output.
