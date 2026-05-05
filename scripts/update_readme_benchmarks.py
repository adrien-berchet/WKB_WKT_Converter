#!/usr/bin/env python3
"""
Run the benchmark suite and update the Benchmarks section of README.md.

Usage:
    python scripts/update_readme_benchmarks.py            # run benchmarks, then update
    python scripts/update_readme_benchmarks.py --json PATH  # use existing JSON, skip run

The script rewrites the block between
    <!-- BENCHMARK_RESULTS_START -->
and
    <!-- BENCHMARK_RESULTS_END -->
in README.md.
"""

import argparse
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).parent.parent
README = ROOT / "README.md"
BENCHMARKS_DIR = ROOT / "benchmarks"

MARKER_START = "<!-- BENCHMARK_RESULTS_START -->"
MARKER_END = "<!-- BENCHMARK_RESULTS_END -->"

# Ordered geometry labels shown in every table row.
GEOMETRY_ORDER = [
    "point",
    "linestring_5pts",
    "polygon_5pts",
    "geometry_collection",
    "multipolygon",
    "linestring_1000pts",
    "polygon_1000pts",
]

GEOMETRY_LABELS = {
    "point": "Point",
    "linestring_5pts": "LineString (5 pts)",
    "polygon_5pts": "Polygon (5 pts)",
    "geometry_collection": "GeometryCollection",
    "multipolygon": "MultiPolygon",
    "linestring_1000pts": "LineString (1000 pts)",
    "polygon_1000pts": "Polygon (1000 pts)",
}

# Each entry: (section_heading, conv_op_name, shapely_op_name_or_None)
# conv_op_name and shapely_op_name are the <operation> part of
# test_(conv|shapely)_<operation>[<geom>] test names.
OPERATION_GROUPS = [
    ("Basic conversions", None, None),          # group header — no table itself
    ("  `wkt_to_wkb`",                    "wkt_to_wkb",                     "wkt_to_wkb"),
    ("  `wkb_to_wkt`",                    "wkb_to_wkt",                     "wkb_to_wkt"),
    ("  `wkt_to_hex_wkb`",                "wkt_to_hex_wkb",                 "wkt_to_hex_wkb"),
    ("  `hex_wkb_to_wkt`",                "hex_wkb_to_wkt",                 "hex_wkb_to_wkt"),
    ("Generic `text_to_*` converters", None, None),
    ("  `text_to_wkb(wkt)`",              "text_to_wkb_from_wkt",           "text_to_wkb_from_wkt"),
    ("  `text_to_wkb(hex_wkb)`",          "text_to_wkb_from_hex",           "text_to_wkb_from_hex"),
    ("  `text_to_wkt(wkt)` — `normalize_wkt=False` (fast path, no WKB round-trip)",
                                          "text_to_wkt_from_wkt_no_normalize", "text_to_wkt_from_wkt"),
    ("  `text_to_wkt(wkt)` — `normalize_wkt=True` (full WKT→WKB→WKT round-trip)",
                                          "text_to_wkt_from_wkt_normalize", "text_to_wkt_from_wkt"),
    ("  `text_to_wkt(hex_wkb)`",          "text_to_wkt_from_hex",           "text_to_wkt_from_hex"),
    ("  `text_to_hex_wkb(wkt)`",          "text_to_hex_wkb_from_wkt",       "text_to_hex_wkb_from_wkt"),
    ("  `text_to_hex_wkb(hex_wkb)`",      "text_to_hex_wkb_from_hex",       "text_to_hex_wkb_from_hex"),
]


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def fmt_time(seconds: float) -> str:
    if seconds < 1e-6:
        return f"{seconds * 1e9:.0f} ns"
    if seconds < 1e-3:
        return f"{seconds * 1e6:.1f} µs"
    if seconds < 1:
        return f"{seconds * 1e3:.1f} ms"
    return f"{seconds:.2f} s"


def fmt_speedup(conv_s: float, shapely_s: float) -> str:
    ratio = shapely_s / conv_s
    return f"{ratio:.1f}×"


def parse_name(name: str):
    """'test_conv_wkt_to_wkb[point]' → ('conv', 'wkt_to_wkb', 'point')"""
    m = re.match(r"test_(conv|shapely)_(.+)\[([^\]]+)\]$", name)
    return m.groups() if m else None


# ---------------------------------------------------------------------------
# JSON loading
# ---------------------------------------------------------------------------

def load_results(json_path: str) -> dict:
    """
    Returns nested dict:
        { op_name: { geometry: { 'conv': mean_s, 'shapely': mean_s } } }
    """
    with open(json_path) as f:
        data = json.load(f)

    results: dict = {}
    for bench in data["benchmarks"]:
        parsed = parse_name(bench["name"])
        if parsed is None:
            continue
        impl, op, geom = parsed
        mean = bench["stats"]["mean"]
        results.setdefault(op, {}).setdefault(geom, {})[impl] = mean

    return results, data


# ---------------------------------------------------------------------------
# Markdown generation
# ---------------------------------------------------------------------------

def make_table(conv_op: str, shapely_op: str | None, results: dict) -> str:
    conv_data = results.get(conv_op, {})
    shapely_data = results.get(shapely_op, {}) if shapely_op else {}
    has_shapely = bool(shapely_data)

    if has_shapely:
        lines = [
            "| Geometry | wkb_wkt_converter | shapely | Speedup |",
            "|:---|---:|---:|---:|",
        ]
    else:
        lines = [
            "| Geometry | wkb_wkt_converter |",
            "|:---|---:|",
        ]

    for geom in GEOMETRY_ORDER:
        label = GEOMETRY_LABELS.get(geom, geom)
        conv_s = conv_data.get(geom, {}).get("conv")
        if conv_s is None:
            continue
        shapely_s = shapely_data.get(geom, {}).get("shapely")

        if has_shapely:
            if shapely_s is not None:
                lines.append(
                    f"| {label} | {fmt_time(conv_s)} | {fmt_time(shapely_s)}"
                    f" | {fmt_speedup(conv_s, shapely_s)} |"
                )
            else:
                lines.append(f"| {label} | {fmt_time(conv_s)} | — | — |")
        else:
            lines.append(f"| {label} | {fmt_time(conv_s)} |")

    return "\n".join(lines)


def generate_section(results: dict, json_data: dict) -> str:
    mi = json_data.get("machine_info", {})
    cpu = mi.get("cpu", {})
    cpu_brand = cpu.get("brand_raw", mi.get("processor", "unknown CPU"))
    python_ver = mi.get("python_version", "?")
    dt = json_data.get("datetime", "")[:10]  # YYYY-MM-DD

    lines = [
        f"*{dt} — Python {python_ver} — {cpu_brand}*",
        "",
        "Times are mean latency per call (lower is better)."
        " Speedup = shapely mean ÷ wkb_wkt_converter mean.",
        "",
    ]

    current_group = None
    for heading, conv_op, shapely_op in OPERATION_GROUPS:
        if conv_op is None:
            # group header
            current_group = heading.strip()
            lines.append(f"### {current_group}")
            lines.append("")
        else:
            lines.append(f"#### {heading.strip()}")
            lines.append("")
            lines.append(make_table(conv_op, shapely_op, results))
            lines.append("")

    return "\n".join(lines).rstrip()


# ---------------------------------------------------------------------------
# README update
# ---------------------------------------------------------------------------

def update_readme(section: str) -> None:
    text = README.read_text(encoding="utf-8")
    start = text.find(MARKER_START)
    end = text.find(MARKER_END)
    if start == -1 or end == -1:
        sys.exit(f"ERROR: markers not found in {README}")
    new_text = (
        text[: start + len(MARKER_START)]
        + "\n\n"
        + section
        + "\n\n"
        + text[end:]
    )
    README.write_text(new_text, encoding="utf-8")
    print(f"Updated {README}")


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--json",
        metavar="PATH",
        help="path to an existing benchmark JSON file (skips running benchmarks)",
    )
    args = parser.parse_args()

    if args.json:
        json_path = args.json
    else:
        tmp = tempfile.NamedTemporaryFile(suffix=".json", delete=False)
        tmp.close()
        json_path = tmp.name
        cmd = [sys.executable, "-m", "pytest", str(BENCHMARKS_DIR),
               f"--benchmark-json={json_path}", "-q"]
        print("Running:", " ".join(cmd))
        proc = subprocess.run(cmd)
        if proc.returncode not in (0, 1):   # 1 = tests failed but benchmarks ran
            sys.exit(f"pytest exited with code {proc.returncode}")

    results, json_data = load_results(json_path)
    section = generate_section(results, json_data)
    update_readme(section)


if __name__ == "__main__":
    main()
