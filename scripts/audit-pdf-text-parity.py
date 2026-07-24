#!/usr/bin/env python3
"""Compare per-page text extracted from two PDFs.

This is a semantic companion to visual RMSE audits. It answers: do both PDFs
put roughly the same text on the same page? Pixel metrics alone can improve or
worsen while pagination semantics drift.
"""
from __future__ import annotations

import argparse
import difflib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


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


def pdf_page_count(path: Path) -> int:
    output = require_ok(run(["pdfinfo", str(path)]), f"pdfinfo {path}")
    for line in output.splitlines():
        if line.startswith("Pages:"):
            return int(line.split(":", 1)[1].strip())
    raise SystemExit(f"could not read page count from {path}")


def extract_page_text(path: Path, page: int, mode: str) -> str:
    cmd = ["pdftotext", "-f", str(page), "-l", str(page)]
    if mode == "layout":
        cmd.append("-layout")
    elif mode == "raw":
        cmd.append("-raw")
    cmd.extend([str(path), "-"])
    return require_ok(run(cmd), f"pdftotext {path} page {page}")


def normalize_text(text: str) -> str:
    text = text.replace("\x0c", " ")
    text = text.replace("\u00a0", " ")
    text = text.casefold()
    text = re.sub(r"\s+", " ", text)
    return text.strip()


def tokens(text: str) -> list[str]:
    return re.findall(r"\w+|[^\w\s]", text, flags=re.UNICODE)


def token_jaccard(left: list[str], right: list[str]) -> float:
    left_set = set(left)
    right_set = set(right)
    if not left_set and not right_set:
        return 1.0
    if not left_set or not right_set:
        return 0.0
    return len(left_set & right_set) / len(left_set | right_set)


def compact_excerpt(text: str, max_chars: int = 220) -> str:
    text = normalize_text(text)
    return text[:max_chars] + ("…" if len(text) > max_chars else "")


def relpath(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(ROOT))
    except ValueError:
        return str(path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("left", type=Path, help="First PDF, usually htmlpdf output")
    parser.add_argument("right", type=Path, help="Second PDF, usually Chromium reference")
    parser.add_argument("--label-left", default="left")
    parser.add_argument("--label-right", default="right")
    parser.add_argument("--mode", choices=("plain", "layout", "raw"), default="layout")
    parser.add_argument("--pages", choices=("all", "first"), default="all")
    parser.add_argument("--max-pages", type=int, default=None)
    parser.add_argument("--threshold-ratio", type=float, default=None)
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

    left_pages = pdf_page_count(left)
    right_pages = pdf_page_count(right)
    pages_to_compare = 1 if args.pages == "first" else min(left_pages, right_pages)
    if args.max_pages is not None:
        pages_to_compare = min(pages_to_compare, args.max_pages)

    page_metrics = []
    for page in range(1, pages_to_compare + 1):
        left_text = extract_page_text(left, page, args.mode)
        right_text = extract_page_text(right, page, args.mode)
        left_norm = normalize_text(left_text)
        right_norm = normalize_text(right_text)
        left_tokens = tokens(left_norm)
        right_tokens = tokens(right_norm)
        ratio = difflib.SequenceMatcher(None, left_norm, right_norm, autojunk=False).ratio()
        metric = {
            "page": page,
            "sequence_ratio": ratio,
            "token_jaccard": token_jaccard(left_tokens, right_tokens),
            f"{args.label_left}_char_count": len(left_norm),
            f"{args.label_right}_char_count": len(right_norm),
            f"{args.label_left}_token_count": len(left_tokens),
            f"{args.label_right}_token_count": len(right_tokens),
            f"{args.label_left}_excerpt": compact_excerpt(left_text),
            f"{args.label_right}_excerpt": compact_excerpt(right_text),
        }
        page_metrics.append(metric)

    worst = min(page_metrics, key=lambda metric: metric["sequence_ratio"]) if page_metrics else None
    mean_ratio = (
        sum(metric["sequence_ratio"] for metric in page_metrics) / len(page_metrics)
        if page_metrics
        else 1.0
    )
    report = {
        f"{args.label_left}_pdf": relpath(left),
        f"{args.label_right}_pdf": relpath(right),
        f"{args.label_left}_page_count": left_pages,
        f"{args.label_right}_page_count": right_pages,
        "page_count_delta": left_pages - right_pages,
        "mode": args.mode,
        "pages_compared": pages_to_compare,
        "mean_sequence_ratio": mean_ratio,
        "min_sequence_ratio": worst["sequence_ratio"] if worst else 1.0,
        "min_sequence_ratio_page": worst["page"] if worst else None,
        "threshold_ratio": args.threshold_ratio,
        "passed_threshold": None
        if args.threshold_ratio is None
        else all(metric["sequence_ratio"] >= args.threshold_ratio for metric in page_metrics),
        "page_metrics": page_metrics,
    }

    text = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(text)
    print(text, end="")

    if report["passed_threshold"] is False:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
