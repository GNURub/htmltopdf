#!/usr/bin/env python3
"""Static guard for WinAnsi text output used by core PDF fonts."""
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]
PDF = ROOT / "crates/htmlpdf-core/src/pdf.rs"
FIXTURE = ROOT / "examples/generic-unicode.html"
checks = {
    PDF: [
        "/Encoding /WinAnsiEncoding",
        "fn escape_pdf_winansi_literal",
        "fn winansi_byte",
        "'€' => Some(0x80)",
        "'●' => Some(0x95)",
        "'−' => Some(b'-')",
        "'\\u{00a1}'..='\\u{00ff}'",
    ],
    FIXTURE: [
        "Campaña con acentos y símbolo €",
        "Inversión total: 550,00 €",
        "● Estado activo",
        "Niño, acción, información, útil, Málaga",
        "Entidades: &euro; &bull; &ndash; &lt;span&gt; &amp; preservadas.",
        "Unicode fuera de WinAnsi: Ω Привет.",
        "The parser must ignore this comment, including the fake > tag inside it.",
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
    print("generic unicode PDF support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic unicode PDF support audit passed")
