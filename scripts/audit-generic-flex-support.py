#!/usr/bin/env python3
"""Static guard for first generic flex/grid row support."""
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]
CSS = ROOT / "crates/htmlpdf-core/src/css.rs"
LAYOUT = ROOT / "crates/htmlpdf-core/src/layout.rs"
FIXTURE = ROOT / "examples/generic-flex.html"
checks = {
    CSS: ["Flex", "Grid", '"flex" => self.display = Display::Flex', '"gap" =>', '"column-gap" =>', "fn parse_gap_shorthand"],
    LAYOUT: ["fn layout_row_container", "fn layout_column_container", "Display::Flex | Display::InlineFlex", "child_width", "style.column_gap", "style.flex_direction == FlexDirection::Column", "resolved_total > available", "let shrink = available / resolved_total"],
    FIXTURE: ["display: flex", "gap: 16px", "flex-direction: column", "class=\"metric\""],
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
    print("generic flex support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic flex support audit passed")
