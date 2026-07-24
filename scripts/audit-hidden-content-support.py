#!/usr/bin/env python3
"""Static guard for browser-like hidden/non-painted content support."""
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]
checks = {
    ROOT / "crates/htmlpdf-core/src/css.rs": [
        "element.attr(\"hidden\").is_some()",
        "Display::Contents",
        '"contents" => self.display = Display::Contents',
        '"inline-block"',
        '"flow-root"',
        '"visibility" | "content-visibility"',
        "pub enum Visibility",
        '"hidden" | "collapse" => self.visibility = Visibility::Hidden',
    ],
    ROOT / "crates/htmlpdf-core/src/layout.rs": [
        "style.display == Display::Contents",
        "fn paint_snapshot",
        "fn restore_paint_snapshot",
        "Visibility::Hidden",
        '"template"',
        '"iframe"',
        '"canvas"',
    ],
    ROOT / "examples/generic-hidden-content.html": [
        "display: contents",
        "display: inline-block",
        "display: flow-root",
        "hidden>This hidden attribute text must not render",
        "aria-hidden text must still render visually",
        "visibility: hidden",
        "<template>",
        "<iframe",
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
    print("hidden content support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("hidden content support audit passed")
