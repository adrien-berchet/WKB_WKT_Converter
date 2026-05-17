# Fuzzing

This workspace keeps fuzzing isolated in `fuzz/` so normal `cargo test --workspace`
and coverage jobs do not pull in libFuzzer dependencies.

## Prerequisites

```sh
cargo install cargo-fuzz
```

## Run targets

From the repository root:

```sh
cd fuzz

# Raw WKB/EWKB bytes and SRID helper fast paths.
cargo fuzz run wkb_bytes corpus/wkb_bytes

# Arbitrary UTF-8 text through WKT/EWKT and auto-detected hex-text entry points.
cargo fuzz run wkt_text corpus/wkt_text

# Generic Input::Text / Input::Wkb dispatch, SridMode, and normalize_wkt behavior.
cargo fuzz run generic_input corpus/generic_input

# Bounded valid-geometry generator for parser/encoder round-trips.
cargo fuzz run structured_roundtrip corpus/structured_roundtrip

# Near-limit and over-limit nested GeometryCollection inputs for depth guards.
cargo fuzz run deep_nesting corpus/deep_nesting
```

Useful maintenance commands:

```sh
cd fuzz

# Minimize a crashing input.
cargo fuzz tmin wkb_bytes artifacts/wkb_bytes/crash-...

# Shrink the checked-in corpus to the smallest useful set.
cargo fuzz cmin wkt_text corpus/wkt_text
```

## Target notes

- Harnesses accept legitimate `Err` results and only assert documented invariants on
  successful conversions.
- Inputs are size-capped inside the harnesses to avoid fuzzer-driven allocation spikes.
- `wkb_bytes` and `generic_input` accept checked-in corpus seeds with a `HEX:` prefix so
  binary fixtures can stay reviewable as ASCII text.
- `deep_nesting` constructs nested `GEOMETRYCOLLECTION` WKT and WKB up to and
  just past the parser depth limit.
- The GitHub Actions fuzz workflow runs bounded smoke campaigns on pull requests
  and longer scheduled runs on `main`; run longer local or continuous campaigns
  for new vulnerability discovery.
