# Fuzzing

This workspace keeps fuzzing isolated in `fuzz/` so normal `cargo test --workspace`
and coverage jobs do not pull in libFuzzer dependencies.

## Prerequisites

```sh
rustup component add --toolchain nightly llvm-tools-preview
cargo install cargo-fuzz
```

## Run targets

From the repository root:

```sh
mkdir -p fuzz/corpus/wkb_bytes fuzz/corpus/wkt_text fuzz/corpus/generic_input fuzz/corpus/structured_roundtrip fuzz/corpus/deep_nesting

# Raw WKB/EWKB bytes and SRID helper fast paths.
cargo fuzz run --fuzz-dir fuzz wkb_bytes fuzz/corpus/wkb_bytes fuzz/seeds/wkb_bytes

# Arbitrary UTF-8 text through WKT/EWKT and auto-detected hex-text entry points.
cargo fuzz run --fuzz-dir fuzz wkt_text fuzz/corpus/wkt_text fuzz/seeds/wkt_text

# Generic Input::Text / Input::Wkb dispatch, SridMode, and normalize_wkt behavior.
cargo fuzz run --fuzz-dir fuzz generic_input fuzz/corpus/generic_input fuzz/seeds/generic_input

# Bounded valid-geometry generator for parser/encoder round-trips.
cargo fuzz run --fuzz-dir fuzz structured_roundtrip fuzz/corpus/structured_roundtrip fuzz/seeds/structured_roundtrip

# Near-limit and over-limit nested GeometryCollection inputs for depth guards.
cargo fuzz run --fuzz-dir fuzz deep_nesting fuzz/corpus/deep_nesting fuzz/seeds/deep_nesting
```

Useful maintenance commands:

```sh
cd fuzz

# Minimize a crashing input.
cargo fuzz tmin wkb_bytes artifacts/wkb_bytes/crash-...

# Shrink the local generated corpus to the smallest useful set.
cargo fuzz cmin wkt_text corpus/wkt_text
```

## Seeds and generated corpus

`fuzz/seeds/<target>/` contains small, reviewable seed inputs that are committed
to git. `fuzz/corpus/<target>/` is the mutable libFuzzer corpus; it is ignored by
git and can grow during local runs or CI.

When a fuzzing run discovers an input that should become part of the stable
starting set, minimize it, give it a descriptive name, and copy only that input
into `fuzz/seeds/<target>/`. Do not commit the generated hash-named files from
`fuzz/corpus/`.

`fuzz/Cargo.toml` is the source of truth for fuzz target names. Use
`scripts/list_fuzz_targets.py` when scripts or automation need the target list.

## Coverage

Coverage is based on the current corpus for each target. For useful numbers,
run the fuzz targets first so the corpora have a chance to grow, then generate
coverage.

From the repository root, the simplest way to generate merged coverage is:

```sh
scripts/fuzz_coverage.sh
```

This runs coverage for every fuzz target, merges the target profiles, prints the
combined report, and writes:

- `fuzz/coverage/merged.profdata`
- `fuzz/coverage/merged.txt`
- `fuzz/coverage/merged.html`

To reuse already-generated `coverage.profdata` files without running
`cargo fuzz coverage` again:

```sh
scripts/fuzz_coverage.sh --skip-generate
```

The sections below show the manual commands that the script wraps.

```sh
cd fuzz

mapfile -t TARGETS < <(python3 ../scripts/list_fuzz_targets.py)

for target in "${TARGETS[@]}"; do
  mkdir -p "corpus/$target"
  cargo +nightly fuzz coverage "$target" "corpus/$target" "seeds/$target"
done
```

### Single Target

Use the nightly LLVM tools directly. This avoids depending on the `cargo cov`
wrapper, which can be sensitive to Cargo/nightly version mismatches.

```sh
HOST=$(rustc +nightly -vV | sed -n 's/^host: //p')
LLVM_COV="$(rustc +nightly --print sysroot)/lib/rustlib/$HOST/bin/llvm-cov"

"$LLVM_COV" report \
  "target/$HOST/coverage/$HOST/release/generic_input" \
  -instr-profile=coverage/generic_input/coverage.profdata \
  ../wkb_wkt_converter/src
```

Generate an HTML report for the same target:

```sh
"$LLVM_COV" show \
  "target/$HOST/coverage/$HOST/release/generic_input" \
  -instr-profile=coverage/generic_input/coverage.profdata \
  --format=html \
  ../wkb_wkt_converter/src \
  > coverage/generic_input/index.html
```

### Merged Targets

Merge all target profiles to see combined fuzz coverage across the core crate.

```sh
HOST=$(rustc +nightly -vV | sed -n 's/^host: //p')
LLVM_PROFDATA="$(rustc +nightly --print sysroot)/lib/rustlib/$HOST/bin/llvm-profdata"
LLVM_COV="$(rustc +nightly --print sysroot)/lib/rustlib/$HOST/bin/llvm-cov"

TARGETS=(
  wkb_bytes
  wkt_text
  generic_input
  structured_roundtrip
  deep_nesting
)

profiles=()
for target in "${TARGETS[@]}"; do
  profiles+=("coverage/$target/coverage.profdata")
done

"$LLVM_PROFDATA" merge -sparse "${profiles[@]}" -o coverage/merged.profdata

main_object="target/$HOST/coverage/$HOST/release/${TARGETS[0]}"

extra_objects=()
for target in "${TARGETS[@]:1}"; do
  extra_objects+=("-object" "target/$HOST/coverage/$HOST/release/$target")
done

"$LLVM_COV" report \
  "$main_object" \
  "${extra_objects[@]}" \
  -instr-profile=coverage/merged.profdata \
  ../wkb_wkt_converter/src
```

Generate merged HTML:

```sh
"$LLVM_COV" show \
  "$main_object" \
  "${extra_objects[@]}" \
  -instr-profile=coverage/merged.profdata \
  --format=html \
  ../wkb_wkt_converter/src \
  > coverage/merged.html
```

If merged coverage is low, first run the existing targets longer and regenerate
coverage. If specific lines remain untouched after the corpora have grown, add
targeted seeds or a new fuzz target for that behavior.

## Target notes

- Harnesses accept legitimate `Err` results and only assert documented invariants on
  successful conversions.
- Inputs are size-capped inside the harnesses to avoid fuzzer-driven allocation spikes.
- `wkb_bytes` and `generic_input` accept seeds with a `HEX:` prefix so binary
  fixtures can stay reviewable as ASCII text.
- `deep_nesting` constructs nested `GEOMETRYCOLLECTION` WKT and WKB up to and
  just past the parser depth limit.
- The GitHub Actions fuzz workflow runs bounded smoke campaigns on pull requests
  and longer scheduled runs on `main`; run longer local or continuous campaigns
  for new vulnerability discovery.
