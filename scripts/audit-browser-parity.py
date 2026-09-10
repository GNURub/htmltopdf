#!/usr/bin/env python3
"""Generate a browser-vs-htmlpdf visual parity report for one HTML fixture.

This is intentionally an evidence tool, not a fake pass/fail proxy. It renders:
1. htmlpdf's PDF via the real CLI
2. Chromium's print PDF as the browser reference
3. first-page PNG rasters for both PDFs by default, or every page with --pages all
4. ImageMagick RMSE diff image(s) and JSON metrics

Use --threshold-rmse-normalized only when a fixture has an agreed acceptance
budget; otherwise the script reports evidence and exits 0.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def run(cmd: list[str], cwd: Path = ROOT) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=cwd, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)


def require_ok(result: subprocess.CompletedProcess[str], label: str) -> None:
    if result.returncode != 0:
        sys.stderr.write(f"{label} failed with exit code {result.returncode}\n")
        if result.stdout:
            sys.stderr.write(result.stdout)
        if result.stderr:
            sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)


def identify_size(path: Path) -> tuple[int, int]:
    result = run(["identify", "-format", "%w %h", str(path)])
    require_ok(result, f"identify {path}")
    width, height = result.stdout.strip().split()
    return int(width), int(height)


def pdf_page_count(path: Path) -> int:
    result = run(["pdfinfo", str(path)])
    require_ok(result, f"pdfinfo {path}")
    for line in result.stdout.splitlines():
        if line.startswith("Pages:"):
            return int(line.split(":", 1)[1].strip())
    raise SystemExit(f"could not read page count from {path}")


def parse_rmse(stderr: str) -> tuple[float, float]:
    # ImageMagick emits e.g. "1234.56 (0.018837)" to stderr.
    match = re.search(r"([0-9.]+)\s+\(([0-9.]+)\)", stderr)
    if not match:
        raise ValueError(f"could not parse ImageMagick RMSE: {stderr!r}")
    return float(match.group(1)), float(match.group(2))


def render_pdf_page(pdf: Path, page_number: int, output_png: Path, label: str) -> None:
    prefix = output_png.with_suffix("")
    require_ok(
        run([
            "pdftoppm",
            "-f",
            str(page_number),
            "-l",
            str(page_number),
            "-singlefile",
            str(pdf),
            str(prefix),
            "-png",
        ]),
        f"pdftoppm {label} page {page_number}",
    )


def compare_pngs(htmlpdf_png: Path, browser_png: Path, browser_normalized: Path, diff_png: Path) -> dict:
    htmlpdf_size = identify_size(htmlpdf_png)
    browser_size = identify_size(browser_png)
    compare_reference = browser_png
    normalized = False
    if htmlpdf_size != browser_size:
        normalized = True
        require_ok(
            run([
                "convert",
                str(browser_png),
                "-resize",
                f"{htmlpdf_size[0]}x{htmlpdf_size[1]}!",
                str(browser_normalized),
            ]),
            "normalize browser raster size",
        )
        compare_reference = browser_normalized

    compare = run(["compare", "-metric", "RMSE", str(compare_reference), str(htmlpdf_png), str(diff_png)])
    # ImageMagick compare returns 1 when images differ; that is expected evidence, not a tool failure.
    if compare.returncode not in (0, 1):
        require_ok(compare, "compare RMSE")
    rmse_absolute, rmse_normalized = parse_rmse(compare.stderr)

    return {
        "htmlpdf_png": relpath(htmlpdf_png),
        "browser_png": relpath(browser_png),
        "diff_png": relpath(diff_png),
        "htmlpdf_size": htmlpdf_size,
        "browser_size": browser_size,
        "browser_raster_was_resized_for_metric": normalized,
        "rmse_absolute": rmse_absolute,
        "rmse_normalized": rmse_normalized,
    }


def relpath(path: Path) -> str:
    return str(path.relative_to(ROOT) if path.is_relative_to(ROOT) else path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("html", type=Path, help="HTML fixture to compare against Chromium print output")
    parser.add_argument("--out-dir", type=Path, default=ROOT / "docs/visual-regression/artifacts/browser-parity")
    parser.add_argument("--browser", default=os.environ.get("BROWSER", "chromium"))
    parser.add_argument("--pages", choices=("first", "all"), default="first")
    parser.add_argument("--max-pages", type=int, default=None, help="Optional cap when --pages all is used")
    parser.add_argument("--threshold-rmse-normalized", type=float, default=None)
    args = parser.parse_args()
    if args.max_pages is not None and args.max_pages < 1:
        parser.error("--max-pages must be positive")
    if args.threshold_rmse_normalized is not None and not 0 <= args.threshold_rmse_normalized <= 1:
        parser.error("--threshold-rmse-normalized must be between 0 and 1")

    html = args.html.resolve()
    if not html.exists():
        sys.stderr.write(f"missing HTML file: {html}\n")
        return 2

    out_dir = args.out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    stem = html.stem
    htmlpdf_pdf = out_dir / f"{stem}.htmlpdf.pdf"
    browser_pdf = out_dir / f"{stem}.chromium.pdf"
    htmlpdf_png = out_dir / f"{stem}.htmlpdf.png"
    browser_png = out_dir / f"{stem}.chromium.png"
    browser_normalized = out_dir / f"{stem}.chromium.normalized.png"
    diff_png = out_dir / f"{stem}.diff.png"
    report_json = out_dir / f"{stem}.parity.json"

    require_ok(
        run(["cargo", "run", "-p", "htmlpdf-cli", "--", "render", str(html), "-o", str(htmlpdf_pdf)]),
        "htmlpdf render",
    )
    require_ok(
        run([
            args.browser,
            "--headless",
            "--no-sandbox",
            "--disable-gpu",
            "--disable-dev-shm-usage",
            "--run-all-compositor-stages-before-draw",
            "--no-pdf-header-footer",
            "--virtual-time-budget=5000",
            f"--user-data-dir={out_dir / '.chromium-profile'}",
            f"--print-to-pdf={browser_pdf}",
            html.as_uri(),
        ]),
        "chromium print-to-pdf",
    )
    htmlpdf_page_count = pdf_page_count(htmlpdf_pdf)
    browser_page_count = pdf_page_count(browser_pdf)
    pages_to_compare = 1 if args.pages == "first" else min(htmlpdf_page_count, browser_page_count)
    if args.max_pages is not None:
        pages_to_compare = min(pages_to_compare, args.max_pages)

    page_metrics = []
    for page_number in range(1, pages_to_compare + 1):
        if page_number == 1:
            current_htmlpdf_png = htmlpdf_png
            current_browser_png = browser_png
            current_browser_normalized = browser_normalized
            current_diff_png = diff_png
        else:
            current_htmlpdf_png = out_dir / f"{stem}.htmlpdf.p{page_number}.png"
            current_browser_png = out_dir / f"{stem}.chromium.p{page_number}.png"
            current_browser_normalized = out_dir / f"{stem}.chromium.p{page_number}.normalized.png"
            current_diff_png = out_dir / f"{stem}.p{page_number}.diff.png"
        render_pdf_page(htmlpdf_pdf, page_number, current_htmlpdf_png, "htmlpdf")
        render_pdf_page(browser_pdf, page_number, current_browser_png, "chromium")
        metric = compare_pngs(
            current_htmlpdf_png,
            current_browser_png,
            current_browser_normalized,
            current_diff_png,
        )
        metric["page"] = page_number
        page_metrics.append(metric)

    first_page = page_metrics[0]
    rmse_absolute = first_page["rmse_absolute"]
    rmse_normalized = first_page["rmse_normalized"]
    max_rmse_page = max(page_metrics, key=lambda metric: metric["rmse_normalized"])
    mean_rmse_normalized = sum(metric["rmse_normalized"] for metric in page_metrics) / len(page_metrics)

    report = {
        "html": relpath(html),
        "htmlpdf_pdf": relpath(htmlpdf_pdf),
        "browser_pdf": relpath(browser_pdf),
        "htmlpdf_page_count": htmlpdf_page_count,
        "browser_page_count": browser_page_count,
        "pages_mode": args.pages,
        "pages_compared": pages_to_compare,
        "page_count_delta": htmlpdf_page_count - browser_page_count,
        "htmlpdf_png": first_page["htmlpdf_png"],
        "browser_png": first_page["browser_png"],
        "diff_png": first_page["diff_png"],
        "htmlpdf_size": first_page["htmlpdf_size"],
        "browser_size": first_page["browser_size"],
        "browser_raster_was_resized_for_metric": first_page["browser_raster_was_resized_for_metric"],
        "rmse_absolute": rmse_absolute,
        "rmse_normalized": rmse_normalized,
        "mean_rmse_normalized": mean_rmse_normalized,
        "max_rmse_normalized": max_rmse_page["rmse_normalized"],
        "max_rmse_page": max_rmse_page["page"],
        "page_metrics": page_metrics,
        "threshold_rmse_normalized": args.threshold_rmse_normalized,
        "passed_threshold": None
        if args.threshold_rmse_normalized is None
        else htmlpdf_page_count == browser_page_count
        and max_rmse_page["rmse_normalized"] <= args.threshold_rmse_normalized,
    }
    report_json.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps(report, indent=2, ensure_ascii=False))

    if report["passed_threshold"] is False:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
