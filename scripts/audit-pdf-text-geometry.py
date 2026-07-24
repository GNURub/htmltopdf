#!/usr/bin/env python3
"""Compare word bounding boxes between two PDFs.

This is a geometry-focused companion to the raster RMSE audit. It extracts
`pdftotext -bbox-layout` XHTML, aligns equal words in reading order, and reports
where text position/size diverges. The goal is evidence: font and layout changes
should be driven by measured word deltas instead of visual guessing.
"""
from __future__ import annotations

import argparse
import difflib
import html
import json
import re
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class WordBox:
    page: int
    text: str
    norm: str
    x_min: float
    y_min: float
    x_max: float
    y_max: float
    index: int

    @property
    def width(self) -> float:
        return self.x_max - self.x_min

    @property
    def height(self) -> float:
        return self.y_max - self.y_min


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


def normalize_word(value: str) -> str:
    value = html.unescape(value).replace("\u00a0", " ").strip().casefold()
    return re.sub(r"\s+", " ", value)


def extract_bbox_xml(pdf: Path) -> str:
    with tempfile.NamedTemporaryFile(suffix=".html", delete=False) as tmp:
        tmp_path = Path(tmp.name)
    try:
        require_ok(
            run(["pdftotext", "-bbox-layout", str(pdf), str(tmp_path)]),
            f"pdftotext -bbox-layout {pdf}",
        )
        return tmp_path.read_text(errors="replace")
    finally:
        tmp_path.unlink(missing_ok=True)


def words_from_pdf(pdf: Path) -> list[WordBox]:
    xml = extract_bbox_xml(pdf)
    root = ET.fromstring(xml)
    words: list[WordBox] = []
    page_number = 0
    index = 0
    for page in root.iter():
        if not page.tag.endswith("page"):
            continue
        page_number += 1
        for node in page.iter():
            if not node.tag.endswith("word"):
                continue
            text = "".join(node.itertext()).strip()
            norm = normalize_word(text)
            if not norm:
                continue
            try:
                box = WordBox(
                    page=page_number,
                    text=text,
                    norm=norm,
                    x_min=float(node.attrib["xMin"]),
                    y_min=float(node.attrib["yMin"]),
                    x_max=float(node.attrib["xMax"]),
                    y_max=float(node.attrib["yMax"]),
                    index=index,
                )
            except (KeyError, ValueError) as exc:
                raise SystemExit(f"invalid word bbox in {pdf}: {exc}") from exc
            words.append(box)
            index += 1
    return words


def aligned_pairs(left: list[WordBox], right: list[WordBox]) -> list[tuple[WordBox, WordBox]]:
    left_tokens = [word.norm for word in left]
    right_tokens = [word.norm for word in right]
    matcher = difflib.SequenceMatcher(None, left_tokens, right_tokens, autojunk=False)
    pairs: list[tuple[WordBox, WordBox]] = []
    for tag, left_start, left_end, right_start, right_end in matcher.get_opcodes():
        if tag != "equal":
            continue
        for left_idx, right_idx in zip(range(left_start, left_end), range(right_start, right_end)):
            pairs.append((left[left_idx], right[right_idx]))
    return pairs


def summarize_pairs(pairs: list[tuple[WordBox, WordBox]], limit: int) -> dict:
    if not pairs:
        return {
            "aligned_words": 0,
            "mean_abs_dx": None,
            "mean_abs_dy": None,
            "mean_width_ratio": None,
            "largest_deltas": [],
        }

    rows = []
    for left, right in pairs:
        dx = left.x_min - right.x_min
        dy = left.y_min - right.y_min
        width_ratio = left.width / right.width if right.width else None
        height_ratio = left.height / right.height if right.height else None
        score = abs(dx) + abs(dy) + abs(left.width - right.width)
        rows.append(
            {
                "page": left.page,
                "word": left.text,
                "left_index": left.index,
                "right_index": right.index,
                "left": {
                    "x_min": round(left.x_min, 3),
                    "y_min": round(left.y_min, 3),
                    "width": round(left.width, 3),
                    "height": round(left.height, 3),
                },
                "right": {
                    "x_min": round(right.x_min, 3),
                    "y_min": round(right.y_min, 3),
                    "width": round(right.width, 3),
                    "height": round(right.height, 3),
                },
                "dx": round(dx, 3),
                "dy": round(dy, 3),
                "width_delta": round(left.width - right.width, 3),
                "width_ratio": round(width_ratio, 4) if width_ratio is not None else None,
                "height_ratio": round(height_ratio, 4) if height_ratio is not None else None,
                "_score": score,
            }
        )

    width_ratios = [
        left.width / right.width
        for left, right in pairs
        if right.width > 0 and left.width > 0
    ]
    report_rows = sorted(rows, key=lambda row: row["_score"], reverse=True)[:limit]
    for row in report_rows:
        del row["_score"]
    return {
        "aligned_words": len(pairs),
        "mean_abs_dx": sum(abs(left.x_min - right.x_min) for left, right in pairs) / len(pairs),
        "mean_abs_dy": sum(abs(left.y_min - right.y_min) for left, right in pairs) / len(pairs),
        "mean_abs_width_delta": sum(abs(left.width - right.width) for left, right in pairs)
        / len(pairs),
        "mean_width_ratio": sum(width_ratios) / len(width_ratios) if width_ratios else None,
        "largest_deltas": report_rows,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("left", type=Path, help="First PDF, usually htmlpdf output")
    parser.add_argument("right", type=Path, help="Second PDF, usually Chromium reference")
    parser.add_argument("--label-left", default="htmlpdf")
    parser.add_argument("--label-right", default="chromium")
    parser.add_argument("--limit", type=int, default=30)
    parser.add_argument("--out", type=Path, default=None)
    args = parser.parse_args()

    left = args.left.resolve()
    right = args.right.resolve()
    if not left.exists():
        sys.stderr.write(f"missing PDF: {left}\n")
        return 2
    if not right.exists():
        sys.stderr.write(f"missing PDF: {right}\n")
        return 2

    left_words = words_from_pdf(left)
    right_words = words_from_pdf(right)
    pairs = aligned_pairs(left_words, right_words)
    summary = summarize_pairs(pairs, args.limit)
    report = {
        f"{args.label_left}_pdf": relpath(left),
        f"{args.label_right}_pdf": relpath(right),
        f"{args.label_left}_word_count": len(left_words),
        f"{args.label_right}_word_count": len(right_words),
        **summary,
    }
    if report["mean_abs_dx"] is not None:
        report["mean_abs_dx"] = round(report["mean_abs_dx"], 4)
        report["mean_abs_dy"] = round(report["mean_abs_dy"], 4)
        report["mean_abs_width_delta"] = round(report["mean_abs_width_delta"], 4)
        report["mean_width_ratio"] = round(report["mean_width_ratio"], 4)

    text = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(text)
    print(text, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
