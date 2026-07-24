#!/usr/bin/env python3
"""Static guard for border-left/right/top/bottom support."""
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]
CSS = ROOT / "crates/htmlpdf-core/src/css.rs"
LAYOUT = ROOT / "crates/htmlpdf-core/src/layout.rs"
FIXTURE = ROOT / "examples/generic-border-sides.html"
checks = {
    CSS: [
        "pub border_left_width: f32",
        "pub border_bottom_width: f32",
        '"border-left" =>',
        '"border-bottom" =>',
        "fn set_all_border_sides",
    ],
    LAYOUT: [
        "fn has_border",
        "border_left_width",
        "border_bottom_width",
        "border_left_color",
        "border_bottom_color",
    ],
    FIXTURE: [
        "border-left: 4px solid #2563eb",
        "border-bottom: 1px solid #cbd5e1",
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
    print("generic border side support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic border side support audit passed")
