#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/fuzz_coverage.sh [--skip-generate] [--no-html]

Generate and merge coverage for all cargo-fuzz targets.

Options:
  --skip-generate  Reuse existing fuzz/coverage/<target>/coverage.profdata files.
  --no-html        Skip writing fuzz/coverage/merged.html.
  -h, --help       Show this help.
EOF
}

generate=true
html=true

while (($#)); do
  case "$1" in
    --skip-generate)
      generate=false
      ;;
    --no-html)
      html=false
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fuzz_dir="$repo_root/fuzz"

targets=(
  wkb_bytes
  wkt_text
  generic_input
  structured_roundtrip
  deep_nesting
)

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required so the script can locate nightly LLVM tools" >&2
  exit 1
fi

if $generate && ! cargo +nightly fuzz --help >/dev/null 2>&1; then
  echo "cargo-fuzz is required. Install it with: cargo install cargo-fuzz" >&2
  exit 1
fi

host="$(rustc +nightly -vV | sed -n 's/^host: //p')"
sysroot="$(rustc +nightly --print sysroot)"
llvm_profdata="$sysroot/lib/rustlib/$host/bin/llvm-profdata"
llvm_cov="$sysroot/lib/rustlib/$host/bin/llvm-cov"

if [[ ! -x "$llvm_profdata" || ! -x "$llvm_cov" ]]; then
  echo "nightly llvm-tools-preview is required. Install it with:" >&2
  echo "  rustup component add --toolchain nightly llvm-tools-preview" >&2
  exit 1
fi

cd "$fuzz_dir"
mkdir -p coverage

if $generate; then
  for target in "${targets[@]}"; do
    echo "==> Generating coverage for $target"
    mkdir -p "corpus/$target"
    cargo +nightly fuzz coverage "$target" "corpus/$target" "seeds/$target"
  done
fi

profiles=()
objects=()

for target in "${targets[@]}"; do
  profile="coverage/$target/coverage.profdata"
  object="target/$host/coverage/$host/release/$target"

  if [[ ! -f "$profile" ]]; then
    echo "missing coverage profile: fuzz/$profile" >&2
    echo "run without --skip-generate, or generate that target first" >&2
    exit 1
  fi
  if [[ ! -x "$object" ]]; then
    echo "missing coverage binary: fuzz/$object" >&2
    echo "run without --skip-generate, or generate that target first" >&2
    exit 1
  fi

  profiles+=("$profile")
  objects+=("$object")
done

echo "==> Merging profiles"
"$llvm_profdata" merge -sparse "${profiles[@]}" -o coverage/merged.profdata

main_object="${objects[0]}"
extra_objects=()
for object in "${objects[@]:1}"; do
  extra_objects+=("-object" "$object")
done

echo "==> Writing text report to fuzz/coverage/merged.txt"
"$llvm_cov" report \
  "$main_object" \
  "${extra_objects[@]}" \
  -instr-profile=coverage/merged.profdata \
  "$repo_root/wkb_wkt_converter/src" \
  | tee coverage/merged.txt

if $html; then
  echo "==> Writing HTML report to fuzz/coverage/merged.html"
  "$llvm_cov" show \
    "$main_object" \
    "${extra_objects[@]}" \
    -instr-profile=coverage/merged.profdata \
    --format=html \
    "$repo_root/wkb_wkt_converter/src" \
    > coverage/merged.html
fi
