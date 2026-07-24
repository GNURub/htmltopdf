#!/usr/bin/env python3
"""Static guard for generic CSS border support."""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
CSS = ROOT / "crates/htmlpdf-core/src/css.rs"
LAYOUT = ROOT / "crates/htmlpdf-core/src/layout.rs"
FIXTURE = ROOT / "examples/generic-borders.html"
checks = {
    CSS: [
        "pub border_width: f32",
        "pub border_color: Option<Color>",
        '"border" =>',
        '"border-width" =>',
        '"border-style" =>',
        '"border-color" =>',
        "pub enum BorderLineStyle",
        "fn parse_border_line_style",
        "fn parse_border_shorthand_with_rem",
    ],
    LAYOUT: [
        "fn push_container_border",
        "fn border_dash",
        "fn rounded_border_can_use_single_stroke",
        "LayoutItem::StrokeRect(StrokeRect",
        "style.border_width > 0.0",
        "LayoutItem::Line(Line",
    ],
    FIXTURE: [
        "border: 2px solid #2563eb",
        "border-width: 1px",
        "border-style: dashed",
        "border-color: #bfdbfe",
        "border-radius: 16px",
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
    print("generic border support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic border support audit passed")
