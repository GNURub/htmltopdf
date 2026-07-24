#!/usr/bin/env python3
"""Raster-level quality guard for the premium invoice snapshot.

This is not a pixel-perfect target comparison. It protects against the class of
failure shown in the broken baseline: mostly plain text, low visual richness, or
missing premium regions such as the dark hero and blue total band.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
IMAGE = ROOT / "docs/visual-regression/artifacts/invoice-current.png"


def run(args: list[str]) -> str:
    try:
        return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT).strip()
    except FileNotFoundError:
        print(f"missing dependency: {args[0]}", file=sys.stderr)
        raise SystemExit(1)
    except subprocess.CalledProcessError as exc:
        print(exc.output, file=sys.stderr)
        raise SystemExit(exc.returncode)


def image_stats(path: Path, crop: str | None = None) -> dict[str, float]:
    command = ["convert", str(path)]
    if crop:
        command += ["-crop", crop]
    command += [
        "-format",
        "colors=%k mean=%[fx:mean] std=%[fx:standard_deviation] w=%w h=%h",
        "info:",
    ]
    raw = run(command)
    stats: dict[str, float] = {}
    for token in raw.split():
        key, value = token.split("=", 1)
        stats[key] = float(value)
    return stats


def main() -> int:
    if not IMAGE.is_file() or IMAGE.stat().st_size == 0:
        print(f"missing invoice raster snapshot: {IMAGE}", file=sys.stderr)
        return 1

    errors: list[str] = []
    full = image_stats(IMAGE)
    hero = image_stats(IMAGE, "1090x365+73+80")
    total = image_stats(IMAGE, "350x75+760+1547")
    table = image_stats(IMAGE, "1035x420+135+925")

    if (full["w"], full["h"]) != (1240.0, 1755.0):
        errors.append(f"unexpected snapshot size: {int(full['w'])}x{int(full['h'])}")
    if full["colors"] < 5000:
        errors.append(f"not enough color richness: {full['colors']:.0f} < 5000")
    if full["std"] < 0.24:
        errors.append(f"global contrast too low: {full['std']:.3f} < 0.240")
    if hero["mean"] > 0.50:
        errors.append(f"hero crop is too light; dark hero likely missing: {hero['mean']:.3f} > 0.500")
    if hero["colors"] < 1000:
        errors.append(f"hero crop is too flat: {hero['colors']:.0f} colors < 1000")
    if total["std"] < 0.12:
        errors.append(f"total band contrast too low: {total['std']:.3f} < 0.120")
    if table["std"] < 0.20:
        errors.append(f"table region contrast too low: {table['std']:.3f} < 0.200")

    if errors:
        print("invoice image quality audit failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(
        "invoice image quality audit passed "
        f"colors={full['colors']:.0f} global_std={full['std']:.3f} "
        f"hero_mean={hero['mean']:.3f} hero_colors={hero['colors']:.0f} "
        f"total_std={total['std']:.3f}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
