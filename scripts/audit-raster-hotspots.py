#!/usr/bin/env python3
"""Locate visual-diff hotspots between two rasterized PDF pages.

Browser-parity RMSE gives one score for a whole page. That is useful as a
gate, but it hides *where* the regression lives. This script tiles two PNGs and
computes RMSE per tile using ImageMagick's `compare`, then reports the worst
regions as stable JSON for follow-up layout/paint work.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RMSE_RE = re.compile(r"(?P<absolute>[0-9.]+)\s+\((?P<normalized>[0-9.]+)\)")


def run(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)


def require_ok(result: subprocess.CompletedProcess[str], label: str) -> str:
    if result.returncode != 0:
        sys.stderr.write(f"{label} failed with exit code {result.returncode}\n")
        if result.stdout:
            sys.stderr.write(result.stdout)
        if result.stderr:
            sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)
    return result.stdout


def relpath(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(ROOT))
    except ValueError:
        return str(path)


def image_size(path: Path) -> tuple[int, int]:
    raw = require_ok(
        run(["identify", "-format", "%w %h", str(path)]),
        f"identify {path}",
    )
    width, height = raw.split()
    return int(width), int(height)


def crop_rmse(left: Path, right: Path, width: int, height: int, x: int, y: int) -> dict:
    crop = f"{width}x{height}+{x}+{y}"
    result = run(
        [
            "compare",
            "-metric",
            "RMSE",
            f"{left}[{crop}]",
            f"{right}[{crop}]",
            "null:",
        ]
    )
    raw = result.stderr.strip() or result.stdout.strip()
    if result.returncode not in (0, 1):
        require_ok(result, f"compare {crop}")
    match = RMSE_RE.search(raw)
    if not match:
        sys.stderr.write(f"unexpected compare output for {crop}: {raw}\n")
        raise SystemExit(1)
    return {
        "x": x,
        "y": y,
        "width": width,
        "height": height,
        "rmse_absolute": round(float(match.group("absolute")), 2),
        "rmse_normalized": round(float(match.group("normalized")), 6),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("left", type=Path)
    parser.add_argument("right", type=Path)
    parser.add_argument("--tile-width", type=int, default=160)
    parser.add_argument("--tile-height", type=int, default=160)
    parser.add_argument("--limit", type=int, default=20)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()

    left = args.left.resolve()
    right = args.right.resolve()
    if not left.exists():
        sys.stderr.write(f"missing image: {left}\n")
        return 2
    if not right.exists():
        sys.stderr.write(f"missing image: {right}\n")
        return 2

    left_size = image_size(left)
    right_size = image_size(right)
    page_width = min(left_size[0], right_size[0])
    page_height = min(left_size[1], right_size[1])
    hotspots: list[dict] = []
    for y in range(0, page_height, args.tile_height):
        tile_height = min(args.tile_height, page_height - y)
        for x in range(0, page_width, args.tile_width):
            tile_width = min(args.tile_width, page_width - x)
            hotspots.append(crop_rmse(left, right, tile_width, tile_height, x, y))

    hotspots.sort(key=lambda item: item["rmse_normalized"], reverse=True)
    report = {
        "left": relpath(left),
        "right": relpath(right),
        "left_size": list(left_size),
        "right_size": list(right_size),
        "compared_size": [page_width, page_height],
        "size_delta": [left_size[0] - right_size[0], left_size[1] - right_size[1]],
        "tile_size": [args.tile_width, args.tile_height],
        "hotspots": hotspots[: args.limit],
    }
    text = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(text)
    print(text, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
