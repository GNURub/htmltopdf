#!/usr/bin/env python3
"""Static parity guard for the premium invoice preview and Rust compositor.

This does not compile Rust. It checks that the no-build preview generator and the
fixture-specific Rust compositor still share the visual constants that matter for
invoice fidelity.
"""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
RUST = ROOT / "crates/htmlpdf-core/src/layout.rs"
PREVIEW = ROOT / "scripts/render-invoice-preview.py"

rust = RUST.read_text()
preview = PREVIEW.read_text()

shared_tokens = [
    "Invoice #1001",
    "Northstar Labs",
    "DESIGN SYSTEMS - AUTOMATION - PDF",
    "PAID INVOICE",
    "PDF Engine MVP",
    "Ada Lovelace Analytics Inc.",
    "INV-2026-1001",
    "EUR 95.00",
    "EUR 2,280.00",
    "EUR 1,140.00",
    "EUR 950.00",
    "EUR 760.00",
    "EUR 5,130.00",
    "-EUR 250.00",
    "EUR 4,880.00",
    "EUR 1,024.80",
    "EUR 5,904.80",
    "TOTAL DUE",
]

rust_required = [
    "RoundRect(RoundRect)",
    "fn premium_invoice_pages",
    "draw_vertical_gradient(",
    "card_x + 80.0,\n        h - 80.0,\n        \"Northstar Labs\"",
    "card_x + 80.0,\n        h - 98.0,\n        \"DESIGN SYSTEMS - AUTOMATION - PDF\"",
    "draw_items_table(&mut page, card_x + 30.0, h - 444.0, 496.0)",
    "let bottom_y = 44.0;",
    "draw_totals(&mut page, card_x + 330.0, bottom_y + 20.0, 166.0)",
    "draw_compact_signature(&mut page, card_x + 362.0, 45.0)",
    "AUTHORIZED SIGNATURE",
    "bottom_y + 88.0,\n        282.0,\n        60.0,\n        8.0,",
    "bottom_y + 10.0,\n        282.0,\n        62.0,\n        8.0,",
    "fn push_round_rect",
]

preview_required = [
    "def round_rect",
    "x,top,width=cx+30,H-444,496",
    "by=44",
    "tx,ty,tw=cx+330,by+20,166",
    "signature_points",
    "AUTHORIZED SIGNATURE",
    "gradient(tx,ty,tw,34",
]

errors: list[str] = []
for token in shared_tokens:
    if token not in rust:
        errors.append(f"missing in Rust compositor: {token}")
    if token not in preview:
        errors.append(f"missing in no-build preview: {token}")

for token in rust_required:
    if token not in rust:
        errors.append(f"missing Rust implementation marker: {token}")

for token in preview_required:
    if token not in preview:
        errors.append(f"missing preview implementation marker: {token}")

if errors:
    print("invoice parity audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)

print("invoice parity audit passed")
