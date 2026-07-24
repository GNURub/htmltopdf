#!/usr/bin/env python3
"""Static guard for framework-generated selector tolerance."""
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]
CSS = ROOT / "crates/htmlpdf-core/src/css.rs"
LAYOUT = ROOT / "crates/htmlpdf-core/src/layout.rs"
FIXTURE = ROOT / "examples/generic-framework-selectors.html"
checks = {
    CSS: [
        "fn normalize_selector_part",
        "struct AttributeSelector",
        "enum AttributeOperator",
        "fn parse_attribute_selectors",
        "fn parse_attribute_selector",
        "fn parse_negation_selectors",
        "fn strip_selector_functions",
        "enum PseudoClass",
        "fn parse_pseudo_classes",
        "fn parse_nth_child_expression",
        "fn element_sibling_position",
        "fn element_sibling_position_of_type",
        "NthChild",
        "fn expand_selector_functions",
        "fn extract_selector_function",
        "fn parse_unitless_line_height",
        "fn parse_unitless_calc",
        "fn parse_unitless_product",
        "pub enum FlexDirection",
        "\"flex-direction\"",
        "\"grid-template-columns\"",
        "fn parse_grid_template_columns",
        "fn parse_gap_shorthand",
        "fn flatten_supported_css",
        "fn find_matching_brace",
        "fn strip_important_flag",
        "eq_ignore_ascii_case(\"!important\")",
        "'[' => bracket_depth += 1",
        "':' if bracket_depth == 0",
        'raw == ":root" || raw == ":host"',
        "\":where\"",
        "\":is\"",
        "\":not\"",
        "fn split_selector_parts_for_parse",
        "enum Combinator",
        "AdjacentSibling",
        "GeneralSibling",
        "struct RelativeSelector",
        "fn normalize_relative_has_selector",
        "fn next_element_sibling",
        "fn previous_element_sibling",
    ],
    LAYOUT: [
        "fn layout_grid_container",
        "fn split_long_word",
        "fn grid_auto_placement_rows",
        "fn grid_spanned_track_width",
        "grid_min_column_width",
        "column_gap",
        "row_gap",
    ],
    FIXTURE: [
        ".framework-card[data-v-demo]",
        "flex-direction: column",
        "grid-template-columns: repeat(3, 1fr)",
        "line-height: 1.15",
        "line-height: calc(1.25 / .875)",
        "h1:first-child",
        ":last-child",
        ":not(:last-child)",
        "[data-state=\"active\"]",
        ":nth-child(2)",
        ":nth-child(2n + 1)",
        ":nth-last-child(2)",
        ":first-of-type",
        ":nth-of-type(2n)",
        ":last-of-type",
        ":where(.framework-card) :is(.item)",
        "> .item[data-v-demo]",
        ".anchor + .adjacent",
        ".anchor ~ .later",
        ".anchor:has(+ .adjacent)",
        ".combinator-row:has(> .later)",
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
    print("generic framework selector support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic framework selector support audit passed")
