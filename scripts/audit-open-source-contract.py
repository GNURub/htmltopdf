#!/usr/bin/env python3
"""Static guard for the open-source renderer contract.

The project may keep demo fixture compositors for visual regression, but the
public default must remain a generic renderer. This audit prevents accidentally
returning to silent per-template hardcoding.
"""

from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
LIB = ROOT / "crates/htmlpdf-core/src/lib.rs"
LAYOUT = ROOT / "crates/htmlpdf-core/src/layout.rs"
CLI = ROOT / "crates/htmlpdf-cli/src/main.rs"
ADR = ROOT / "docs/adr/0001-generic-renderer-no-silent-fixtures.md"

checks = {
    LIB: [
        "pub render_mode: RenderMode,",
        "render_mode: RenderMode::Generic,",
        "pub enum RenderMode",
        "Generic",
        "DemoFixture",
    ],
    LAYOUT: [
        "options.render_mode == RenderMode::DemoFixture && is_premium_invoice_fixture(document)",
    ],
    CLI: [
        "--render-mode",
        '"generic" => RenderMode::Generic',
        '"demo-fixture" => RenderMode::DemoFixture',
        "The default render mode is generic",
    ],
    ADR: [
        "Generic renderer by default",
        "no silent fixture special-cases",
        "fixture rendering requires explicit `--render-mode demo-fixture`",
    ],
}

errors: list[str] = []
for path, tokens in checks.items():
    if not path.exists():
        errors.append(f"missing required file: {path.relative_to(ROOT)}")
        continue
    text = path.read_text()
    for token in tokens:
        if token not in text:
            errors.append(f"missing `{token}` in {path.relative_to(ROOT)}")

layout = LAYOUT.read_text()
silent_pattern = "if is_premium_invoice_fixture(document) {\n        return premium_invoice_pages(options);"
if silent_pattern in layout:
    errors.append("premium invoice fixture is still silently enabled without RenderMode::DemoFixture")

if errors:
    print("open-source contract audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)

print("open-source contract audit passed")
