#!/usr/bin/env python3
"""Static guard for generic CSS selector support."""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
CSS = ROOT / "crates/htmlpdf-core/src/css.rs"
DOM = ROOT / "crates/htmlpdf-core/src/dom.rs"
FIXTURE = ROOT / "examples/generic-selectors.html"
checks = {
    DOM: ["pub fn parent_of(&self, child: NodeId) -> Option<NodeId>"],
    CSS: [
        "struct SelectorPart",
        "fn parse(raw: &str) -> Option<Self>",
        "'*' =>",
        "fn split_selector_parts_for_parse",
        "Combinator::Child",
        "document.parent_of(id)",
    ],
    FIXTURE: [
        "* { box-sizing: border-box; }",
        ".shell .card.primary",
        "section.card h1",
        "#selector-target strong",
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
    print("generic selector support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic selector support audit passed")
