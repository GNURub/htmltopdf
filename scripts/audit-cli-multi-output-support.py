#!/usr/bin/env python3
"""Static guard for CLI multi-input rendering modes."""
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "crates/htmlpdf-cli/src/main.rs"
LIB = ROOT / "crates/htmlpdf-core/src/lib.rs"
README = ROOT / "README.md"
DOC = ROOT / "docs/architecture/generic-rendering-strategy.md"
checks = {
    CLI: [
        "fn render_many_to_one_pdf",
        "fn render_pages_for_input",
        "write_pages_to_pdf",
        "render_html_to_pages",
        "htmlpdf render <files...> -o <combined.pdf>",
        "output_dir.extension().is_some()",
        "same_media_box",
    ],
    LIB: [
        "pub fn render_html_to_pages",
        "pub fn write_pages_to_pdf",
    ],
    README: [
        "cargo run -p htmlpdf-cli -- render examples/*.html -o out-examples",
        "cargo run -p htmlpdf-cli -- render examples/*.html -o out.pdf",
    ],
    DOC: [
        "cargo run -p htmlpdf-cli -- render examples/*.html -o out-examples",
        "cargo run -p htmlpdf-cli -- render examples/*.html -o out.pdf",
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
    print("CLI multi-output support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("CLI multi-output support audit passed")
