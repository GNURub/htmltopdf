#!/usr/bin/env python3
"""Static guard for generic float and clear flow support."""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
CSS = ROOT / "crates/htmlpdf-core/src/css.rs"
LAYOUT = ROOT / "crates/htmlpdf-core/src/layout.rs"
FIXTURE = ROOT / "examples/generic-float.html"
INLINE_FIXTURE = ROOT / "examples/generic-inline-float.html"

checks = {
    CSS: [
        "pub float: FloatSide",
        "pub clear: ClearSide",
        '"float" =>',
        '"clear" =>',
        "pub enum FloatSide",
        "pub enum ClearSide",
    ],
    LAYOUT: [
        "struct ActiveFloat",
        "fn layout_flow_child",
        "fn layout_float",
        "fn clear_floats",
        "fn reflow_floats",
        "fn wrap_inline_segments_with_floats",
        "fn tokenize_inline_segments",
        "fn wrap_one_inline_line",
        "self.layout_flow_child(child, float_base_left, float_base_right)",
        "self.floats.clear()",
    ],
    FIXTURE: [
        "float: left",
        "float: right",
        "clear: both",
        "Este párrafo debe rodear",
        "Este bloque se coloca debajo",
    ],
    INLINE_FIXTURE: [
        "float: left",
        "<strong>en negrita</strong>",
        "<em>énfasis</em>",
        "<a href=\"#details\">enlace</a>",
        "<br>",
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
    print("generic float support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic float support audit passed")
