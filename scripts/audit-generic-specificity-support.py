#!/usr/bin/env python3
"""Static guard for generic CSS cascade specificity support."""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
CSS = ROOT / "crates/htmlpdf-core/src/css.rs"
FIXTURE = ROOT / "examples/generic-specificity.html"
checks = {
    CSS: [
        "normal_rule_cascade_key",
        "important_rule_cascade_key",
        "rule.specificity",
        "specificity: (u16, u16, u16)",
        "fn selector_specificity_for_cascade",
        "fn compound_specificity",
        "extract_selector_function(rest, &[\":where\"])",
        "extract_selector_function(rest, &[\":is\", \":not\", \":has\"])",
        "fn max_selector_list_specificity",
    ],
    FIXTURE: [
        "#specific-title { color: #2563eb; }",
        "h1 { color: #dc2626; }",
        ".panel .message { color: #475569; }",
        "p { color: #16a34a; }",
        ":where(.specific-where) { color: #dc2626; }",
        ":is(#missing-specific-id, .specific-is) { color: #2563eb; }",
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
    print("generic specificity support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic specificity support audit passed")
