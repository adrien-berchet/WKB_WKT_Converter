#!/usr/bin/env python3
"""List cargo-fuzz targets from the fuzz crate metadata."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


def fuzz_targets(repo_root: Path) -> list[str]:
    manifest = repo_root / "fuzz" / "Cargo.toml"
    metadata = subprocess.run(
        [
            "cargo",
            "metadata",
            "--manifest-path",
            str(manifest),
            "--no-deps",
            "--format-version",
            "1",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    document = json.loads(metadata.stdout)

    targets = []
    for package in document["packages"]:
        if package["manifest_path"] != str(manifest):
            continue
        for target in package["targets"]:
            if "bin" in target["kind"]:
                targets.append(target["name"])
        break

    if not targets:
        raise RuntimeError(f"no fuzz targets found in {manifest}")

    return targets


def main() -> int:
    parser = argparse.ArgumentParser(
        description="List fuzz target names from fuzz/Cargo.toml.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="emit a compact JSON array for GitHub Actions",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[1]
    targets = fuzz_targets(repo_root)
    if args.json:
        print(json.dumps(targets, separators=(",", ":")))
    else:
        print("\n".join(targets))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as error:
        print(error.stderr, file=sys.stderr, end="")
        raise SystemExit(error.returncode)
    except RuntimeError as error:
        print(error, file=sys.stderr)
        raise SystemExit(1)
