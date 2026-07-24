#!/usr/bin/env python3
"""Static guard for generic box-model support.

This does not prove browser fidelity; it prevents the project from regressing to
text-only generic layout while the open-source renderer evolves.
"""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
CSS = ROOT / "crates/htmlpdf-core/src/css.rs"
LAYOUT = ROOT / "crates/htmlpdf-core/src/layout.rs"
FIXTURE = ROOT / "examples/generic-card.html"

checks = {
    CSS: [
        "pub padding_top: f32",
        "pub padding_right: f32",
        "pub padding_bottom: f32",
        "pub padding_left: f32",
        "pub margin_left: f32",
        "pub margin_right: f32",
        "pub border_radius: f32",
        "pub enum BoxSizing",
        '"box-sizing" =>',
        "margin_bottom: 0.0",
        '"padding" =>',
        '"padding-left" =>',
        '"border-radius" =>',
    ],
    LAYOUT: [
        "fn layout_container",
        "inset_left",
        "inset_right",
        "LayoutItem::RoundRect(RoundRect",
        "background_items(style, box_x, self.current_y, box_width, height)",
        "horizontal_box_extras",
        "self.inset_left += style.margin_left + style.padding_left",
    ],
    FIXTURE: [
        "GENERIC HTML FIXTURE",
        "box-sizing: border-box",
        "padding: 24px",
        "border-radius: 16px",
        "background-color: #eff6ff",
    ],
}

errors = []
for path, tokens in checks.items():
    if not path.exists():
        errors.append(f"missing required file: {path.relative_to(ROOT)}")
        continue
    text = path.read_text()
    for token in tokens:
        if token not in text:
            errors.append(f"missing `{token}` in {path.relative_to(ROOT)}")

if errors:
    print("generic box support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)

print("generic box support audit passed")
