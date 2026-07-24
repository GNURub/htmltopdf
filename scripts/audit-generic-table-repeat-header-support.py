#!/usr/bin/env python3
"""Static guard for repeated table header support across PDF pages."""
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]
LAYOUT = ROOT / "crates/htmlpdf-core/src/layout.rs"
FIXTURE = ROOT / "examples/generic-table-repeat-header.html"
checks = {
    LAYOUT: [
        "struct PreparedCell",
        "fn prepare_table_row",
        "fn paint_table_row",
        "fn is_header_row",
        "fn prepared_row_height",
        "repeats_header",
    ],
    FIXTURE: [
        "<thead><tr><th>Fecha</th>",
        "24/3/26</td><td>23:00",
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
    print("generic repeated table header support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic repeated table header support audit passed")
