#!/usr/bin/env python3
"""No-build content audit for the generated premium invoice PDF.

This checks the actual `out.pdf` artifact for the expected PDF operators and
business-critical invoice text. It complements the visual guard: visual diff
prevents regression to the broken screenshot; this audit prevents shipping a PDF
that looks non-empty but lacks invoice content.
"""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
PDF = ROOT / "out.pdf"

if not PDF.exists() or PDF.stat().st_size == 0:
    print(f"missing PDF artifact: {PDF}", file=sys.stderr)
    sys.exit(1)

raw = PDF.read_bytes()
text = raw.decode("latin1", errors="replace")

required_text = [
    "Northstar Labs",
    "DESIGN SYSTEMS - AUTOMATION - PDF",
    "PAID INVOICE",
    "Invoice #1001",
    "INV-2026-1001",
    "Ada Lovelace Analytics Inc.",
    "PDF Engine MVP",
    "Renderer architecture sprint",
    "HTML/CSS fixture system",
    "Deterministic scripting layer",
    "CLI and service packaging",
    "PAYMENT DETAILS",
    "AUTHORIZED SIGNATURE",
    "TOTAL DUE",
    "EUR 5,904.80",
]

required_pdf_markers = [
    "%PDF-1.4",
    "/BaseFont /Helvetica",
    "/BaseFont /Helvetica-Bold",
    " re f ",
    " c f Q",
    " l S Q",
    " Tj ET",
    "xref",
    "%%EOF",
]

errors: list[str] = []
for token in required_text:
    if token not in text:
        errors.append(f"missing invoice text in PDF bytes: {token}")

for marker in required_pdf_markers:
    if marker not in text:
        errors.append(f"missing PDF drawing marker: {marker}")

page_count = text.count("/Type /Page /Parent")
if page_count < 1:
    errors.append("PDF does not declare any page objects")

text_runs = text.count(" Tj ET")
if text_runs < 55:
    errors.append(f"too few text runs: {text_runs} < 55")

rect_ops = text.count(" re f Q")
curve_ops = text.count(" c f Q")
line_ops = text.count(" l S Q")
if rect_ops < 80:
    errors.append(f"too few filled rectangle operations: {rect_ops} < 80")
if curve_ops < 40:
    errors.append(f"too few curved fill operations: {curve_ops} < 40")
if line_ops < 16:
    errors.append(f"too few stroked line operations: {line_ops} < 16")

if errors:
    print("invoice PDF content audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)

print("invoice PDF content audit passed")
print(f"text_runs={text_runs} rect_ops={rect_ops} curve_ops={curve_ops} line_ops={line_ops} pages={page_count}")
