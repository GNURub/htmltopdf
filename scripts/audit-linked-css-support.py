#!/usr/bin/env python3
"""Static guard for local linked CSS support."""
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]
checks = {
    ROOT / "crates/htmlpdf-core/src/dom.rs": [
        "pub enum StyleSource",
        "pub fn style_sources",
        "collect_style_sources",
        "element.tag == \"noscript\"",
        "rel.split_whitespace().any(|token| token == \"stylesheet\")",
    ],
    ROOT / "crates/htmlpdf-core/src/lib.rs": [
        "fn load_css_chunks",
        "fn linked_stylesheet_applies_to_print",
        "fn resolve_local_asset",
        "StyleSource::Linked",
        "read_to_string(&path)",
    ],
    ROOT / "crates/htmlpdf-cli/src/main.rs": [
        "options.base_url = Some",
        ".parent()",
    ],
    ROOT / "examples/generic-linked-css.html": [
        "<link rel=\"stylesheet\" href=\"generic-linked-css.css\" media=\"all\">",
        "linked-card",
    ],
    ROOT / "examples/generic-linked-css.css": [
        "--linked-brand: rgb(37 99 235)",
        "color-mix(in srgb, var(--linked-brand), transparent 94%)",
        "box-shadow: 0 1rem 2rem rgb(15 23 42 / 12%)",
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
    print("linked CSS support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("linked CSS support audit passed")
