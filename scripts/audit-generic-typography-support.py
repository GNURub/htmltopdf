#!/usr/bin/env python3
"""Static guard for generic typography cascade support."""
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]
CSS = ROOT / "crates/htmlpdf-core/src/css.rs"
LAYOUT = ROOT / "crates/htmlpdf-core/src/layout.rs"
PDF = ROOT / "crates/htmlpdf-core/src/pdf.rs"
FIXTURE = ROOT / "examples/generic-typography.html"
checks = {
    CSS: [
        "pub enum TextTransform",
        "line_height_multiplier",
        '"normal" | "lighter" => FontWeight::Normal',
        '"text-transform" =>',
        "TextTransform::Uppercase",
        "TextTransform::Capitalize",
        '"letter-spacing" =>',
        "parse_letter_spacing_with_rem",
        '"transform" | "-webkit-transform" =>',
        "parse_css_transform",
        "parse_rotate_value",
        "transform_rotate_deg",
        "pub enum FontFace",
        '"font-family" =>',
        "parse_font_face",
        "rem_base_pt",
        "is_html_root",
    ],
    LAYOUT: [
        "fn transform_text",
        "fn capitalize_text",
        "text.to_uppercase()",
        "style.text_transform",
        "letter_spacing",
        "rotation_deg",
        "style.transform_rotate_deg",
        "estimate_text_width_with_spacing",
        "text.split('\\n')",
        "standard_pdf_glyph_width",
        "helvetica_glyph_width",
        "times_glyph_width",
        "fn split_long_word_at_soft_breaks",
        "fn soft_break_segments",
        "FontFace::Mono",
        "FontFace::Serif",
        "font_weight",
    ],
    PDF: [
        'base_font: "NotoSans-Regular"',
        'base_font: "NotoSans-Bold"',
        "fallback_font_object",
        "pdf_font_name",
    ],
    FIXTURE: [
        "html { font-size: 14px; }",
        "font-weight: 400",
        "font-weight: 500",
        "font-weight: 700",
        "letter-spacing: .08em",
        "letter-spacing: -0.03em",
        "text-transform: uppercase",
        "text-transform: capitalize",
        "transform: rotate(-12deg)",
        "Rotated watermark text",
        "font-family: ui-monospace",
        "font-family: Georgia",
        "Hard line one<br>Hard line two<br>Hard line three",
        "Width probe: III lll www MMM 000 € punctuation .,;:",
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
    print("generic typography support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic typography support audit passed")
