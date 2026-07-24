#!/usr/bin/env python3
"""Static guard for @page and print media support."""
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]
CSS = ROOT / "crates/htmlpdf-core/src/css.rs"
LAYOUT = ROOT / "crates/htmlpdf-core/src/layout.rs"
LIB = ROOT / "crates/htmlpdf-core/src/lib.rs"
FIXTURE = ROOT / "examples/generic-print-page.html"
checks = {
    CSS: [
        "pub fn page_options",
        "struct PageStyle",
        "fn parse_page_style",
        "fn parse_page_declarations",
        "selector.starts_with(\"@page\")",
        "mm_to_pt",
        "strip_suffix(\"mm\")",
        "\"page-break-inside\"",
        "\"break-inside\"",
        "\"page-break-before\"",
        "\"page-break-after\"",
        "page_break_after_avoid",
        "page_break_inside_avoid",
        "struct MediaContext",
        "fn media_query_matches",
        "fn parse_media_width_constraint",
    ],
    LAYOUT: [
        "fn should_start_avoid_block_on_next_page",
        "fn should_keep_with_next_on_next_page",
        "remaining_height < printable_height * 0.40",
    ],
    LIB: [
        "effective_options.page = stylesheet.page_options(options.page)",
        "pub fn write_pages_to_pdf",
        "pdf::write_pdf(pages, page)",
    ],
    FIXTURE: [
        "@page { size: A4; margin: 25mm 20mm; }",
        "@media print",
        "display: none !important",
        "@media (max-width: 768px)",
        "responsive-print",
        "page-break-inside: avoid",
        "page-break-after: avoid",
        "page-break-before: always",
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
    print("generic print page support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic print page support audit passed")
