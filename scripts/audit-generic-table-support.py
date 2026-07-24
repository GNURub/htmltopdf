#!/usr/bin/env python3
"""Static guard for generic table layout support."""
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]
LAYOUT = ROOT / "crates/htmlpdf-core/src/layout.rs"
CSS = ROOT / "crates/htmlpdf-core/src/css.rs"
FIXTURE = ROOT / "examples/generic-table.html"
checks = {
    LAYOUT: [
        "fn layout_table",
        "fn table_rows",
        "fn table_cells",
        "fn table_column_widths",
        "struct PreparedLine",
        "fn prepare_table_cell_lines",
        "fn push_table_text_lines",
        "fn estimated_inline_block_height",
        "unified_header_background",
        "table_style.border_radius",
        "let column_widths = table_column_widths",
        "long_tables_keep_content_based_column_widths",
        "push_table_cell_border",
        "has_border(style)",
        "element.tag == \"table\"",
        "BorderCollapse",
        "table_row_height",
        "collapsed_border_overlap",
    ],
    CSS: [
        '"th" =>',
        '"td" =>',
        "pub enum BorderCollapse",
        '"border-collapse"',
    ],
    FIXTURE: [
        "<table>",
        "<th>Service</th>",
        "class=\"amount\"",
        "border-collapse: collapse",
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
    print("generic table support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic table support audit passed")
