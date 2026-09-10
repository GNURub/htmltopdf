#!/usr/bin/env python3
"""Static guard for CSS margin/padding shorthand support."""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
CSS = ROOT / "crates/htmlpdf-core/src/css.rs"
FIXTURE = ROOT / "examples/generic-spacing.html"
checks = {
    CSS: [
        "fn parse_box_shorthand(value: &str) -> Option<[f32; 4]>",
        "[vertical, horizontal]",
        "[top, horizontal, bottom]",
        "[top, right, bottom, left]",
        "parse_box_shorthand_with_rem(&value, length_context)",
    ],
    FIXTURE: [
        "margin: 24px 48px 12px 96px",
        "padding: 24px 36px 18px 48px",
        "margin: 12px 24px",
        "padding: 10px 20px 16px",
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
    print("generic spacing support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic spacing support audit passed")
