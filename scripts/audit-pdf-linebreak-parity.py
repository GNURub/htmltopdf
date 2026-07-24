#!/usr/bin/env python3
"""Compare line breaks between two PDFs using word bounding boxes.

Raster RMSE says "different"; this script says *where wrapping differs*.
It groups `pdftotext -bbox-layout` words into visual lines, normalizes text,
and reports line-count and sequence differences per page.
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
class Word:
    page: int
    text: str
    x_min: float
    y_min: float
    x_max: float
    y_max: float


@dataclass(frozen=True)
class Line:
    page: int
    text: str
    key: str
    x_min: float
    y_min: float
    x_max: float
    y_max: float


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


def normalize_text(text: str) -> str:
    text = html.unescape(text).replace("\u00a0", " ").casefold()
    text = re.sub(r"\s+", " ", text)
    return text.strip()


def comparison_text(text: str, ignore_word_spacing: bool) -> str:
    """Canonical text key for matching extracted PDF lines.

    Browser-produced PDFs often expose letter-spaced uppercase text as
    separate glyph "words" (`p d f`) while our PDF backend can expose the same
    visual text as one word (`pdf`). For line-break auditing this is a noisy
    extraction detail, not a wrapping difference. The comparison key removes
    whitespace between word characters so the sequence matcher focuses on
    which content landed on each visual line.
    """
    text = normalize_text(text)
    if ignore_word_spacing:
        return re.sub(r"(?<=\w)\s+(?=\w)", "", text)
    return text


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


def words_from_pdf(pdf: Path) -> list[Word]:
    root = ET.fromstring(extract_bbox_xml(pdf))
    words: list[Word] = []
    page_number = 0
    for page in root.iter():
        if not page.tag.endswith("page"):
            continue
        page_number += 1
        for node in page.iter():
            if not node.tag.endswith("word"):
                continue
            text = "".join(node.itertext()).strip()
            if not normalize_text(text):
                continue
            words.append(
                Word(
                    page=page_number,
                    text=html.unescape(text),
                    x_min=float(node.attrib["xMin"]),
                    y_min=float(node.attrib["yMin"]),
                    x_max=float(node.attrib["xMax"]),
                    y_max=float(node.attrib["yMax"]),
                )
            )
    return words


def split_bucket_by_x_gap(bucket: list[Word], x_gap_threshold: float) -> list[list[Word]]:
    """Split same-baseline words into visual line segments.

    `pdftotext -bbox-layout` gives us word boxes, not CSS line boxes. A naïve
    "same y == same line" grouping incorrectly merges unrelated columns,
    badges, totals and metadata that happen to share a baseline. Splitting on a
    large horizontal gap keeps the audit focused on wrapping differences inside
    each visual run instead of inventing mega-lines across the page.
    """
    if not bucket:
        return []

    ordered = sorted(bucket, key=lambda word: word.x_min)
    segments: list[list[Word]] = [[ordered[0]]]
    previous = ordered[0]
    for word in ordered[1:]:
        gap = word.x_min - previous.x_max
        if gap > x_gap_threshold:
            segments.append([word])
        else:
            segments[-1].append(word)
        previous = word
    return segments


def group_lines(
    words: list[Word],
    y_tolerance: float,
    x_gap_threshold: float,
    ignore_word_spacing: bool,
) -> list[Line]:
    lines: list[Line] = []
    for page in sorted({word.page for word in words}):
        page_words = sorted(
            [word for word in words if word.page == page],
            key=lambda word: (word.y_min, word.x_min),
        )
        buckets: list[list[Word]] = []
        for word in page_words:
            for bucket in buckets:
                bucket_y = sum(item.y_min for item in bucket) / len(bucket)
                if abs(word.y_min - bucket_y) <= y_tolerance:
                    bucket.append(word)
                    break
            else:
                buckets.append([word])
        for bucket in buckets:
            for segment in split_bucket_by_x_gap(bucket, x_gap_threshold):
                text = normalize_text(" ".join(word.text for word in segment))
                if not text:
                    continue
                lines.append(
                    Line(
                        page=page,
                        text=text,
                        key=comparison_text(text, ignore_word_spacing),
                        x_min=min(word.x_min for word in segment),
                        y_min=min(word.y_min for word in segment),
                        x_max=max(word.x_max for word in segment),
                        y_max=max(word.y_max for word in segment),
                    )
                )
    return lines


def page_metrics(left: list[Line], right: list[Line], limit: int) -> list[dict]:
    pages = sorted({line.page for line in left} | {line.page for line in right})
    metrics = []
    for page in pages:
        left_lines = [line for line in left if line.page == page]
        right_lines = [line for line in right if line.page == page]
        left_text = [line.text for line in left_lines]
        right_text = [line.text for line in right_lines]
        left_keys = [line.key for line in left_lines]
        right_keys = [line.key for line in right_lines]
        matcher = difflib.SequenceMatcher(None, left_keys, right_keys, autojunk=False)
        diffs = []
        for tag, i1, i2, j1, j2 in matcher.get_opcodes():
            if tag == "equal":
                continue
            diffs.append(
                {
                    "op": tag,
                    "left_range": [i1, i2],
                    "right_range": [j1, j2],
                    "left": left_text[i1:i2],
                    "right": right_text[j1:j2],
                }
            )
            if len(diffs) >= limit:
                break
        ratio = matcher.ratio()
        metrics.append(
            {
                "page": page,
                "left_line_count": len(left_lines),
                "right_line_count": len(right_lines),
                "sequence_ratio": ratio,
                "diffs": diffs,
            }
        )
    return metrics


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("left", type=Path)
    parser.add_argument("right", type=Path)
    parser.add_argument("--label-left", default="htmlpdf")
    parser.add_argument("--label-right", default="chromium")
    parser.add_argument("--y-tolerance", type=float, default=2.0)
    parser.add_argument(
        "--x-gap-threshold",
        type=float,
        default=160.0,
        help=(
            "Maximum horizontal gap in PDF points before same-baseline words "
            "are treated as separate visual line segments."
        ),
    )
    parser.add_argument(
        "--ignore-word-spacing",
        action="store_true",
        help=(
            "Match line content after removing whitespace between word characters. "
            "Useful for diagnosing pdftotext extraction artifacts from letter-spacing, "
            "but disabled by default because it can hide real spacing differences."
        ),
    )
    parser.add_argument("--limit", type=int, default=8)
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

    left_lines = group_lines(
        words_from_pdf(left),
        args.y_tolerance,
        args.x_gap_threshold,
        args.ignore_word_spacing,
    )
    right_lines = group_lines(
        words_from_pdf(right),
        args.y_tolerance,
        args.x_gap_threshold,
        args.ignore_word_spacing,
    )
    metrics = page_metrics(left_lines, right_lines, args.limit)
    mean_ratio = (
        sum(metric["sequence_ratio"] for metric in metrics) / len(metrics) if metrics else 1.0
    )
    worst = min(metrics, key=lambda metric: metric["sequence_ratio"]) if metrics else None
    report = {
        f"{args.label_left}_pdf": relpath(left),
        f"{args.label_right}_pdf": relpath(right),
        "y_tolerance": args.y_tolerance,
        "x_gap_threshold": args.x_gap_threshold,
        "ignore_word_spacing": args.ignore_word_spacing,
        f"{args.label_left}_line_count": len(left_lines),
        f"{args.label_right}_line_count": len(right_lines),
        "mean_sequence_ratio": round(mean_ratio, 4),
        "min_sequence_ratio": round(worst["sequence_ratio"], 4) if worst else 1.0,
        "min_sequence_ratio_page": worst["page"] if worst else None,
        "page_metrics": metrics,
    }
    text = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(text)
    print(text, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
