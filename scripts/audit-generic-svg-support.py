#!/usr/bin/env python3
"""Static guard for first-pass inline SVG shape support."""
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]
LAYOUT = ROOT / "crates/htmlpdf-core/src/layout.rs"
CSS = ROOT / "crates/htmlpdf-core/src/css.rs"
PDF = ROOT / "crates/htmlpdf-core/src/pdf.rs"
FIXTURE = ROOT / "examples/generic-svg.html"
checks = {
    LAYOUT: [
        "fn layout_svg",
        "background_items(style, self.options, x, bottom, width, height)",
        "self.push_container_decoration(x, bottom, width, height, style)",
        "fn paint_svg_child",
        "parse_view_box",
        "fn svg_aspect_height",
        "parse_svg_points",
        "parse_svg_path",
        "tokenize_svg_path",
        "PathCommand::CubicTo",
        "line_cap_round",
        "parse_svg_alpha",
        "LayoutItem::CircleStroke",
        "LayoutItem::StrokeRect",
        "svg_paint_color",
        "fn svg_url_paint_color",
        "fn svg_url_linear_gradient",
        "fn svg_gradient_stops",
        "parse_svg_gradient_coord",
        "parse_svg_gradient_offset",
        "LayoutItem::LinearGradient",
        "LayoutItem::BeginClip",
        "struct SvgTransform",
        "fn parse_svg_transform",
        "fn parse_svg_transform_functions",
        "fn matching_svg_function_end",
        "inherited_transform: SvgTransform",
        "fn find_svg_paint_server",
        "fn first_svg_stop_color",
        "fn averaged_svg_stop_color",
        "fn svg_stop_colors",
        "fn svg_stop_color",
        "stop-opacity",
        'element.tag == "svg"',
        '"rect" =>',
        '"circle" =>',
        '"line" =>',
        '"polygon" =>',
        '"path" =>',
        '"text" =>',
        "LayoutItem::Circle",
        "CircleStroke",
        "StrokeRect",
        "LayoutItem::Polygon",
        "LayoutItem::Line",
        "LayoutItem::SvgPath",
        "svg_without_explicit_height_uses_viewbox_aspect_ratio",
        "svg_elements_paint_css_filter_drop_shadow_and_decoration",
    ],
    CSS: [
        "pub(crate) fn from_css",
    ],
    FIXTURE: [
        "<svg viewBox=\"0 0 160 96\"",
        ".ratio-svg",
        "<svg class=\"ratio-svg\" viewBox=\"0 0 260 72\"",
        ".decorated-svg",
        "filter: drop-shadow(0 10px 16px rgb(15 23 42 / 18%))",
        "<svg class=\"decorated-svg\" viewBox=\"0 0 120 72\"",
        "<linearGradient id=\"blueWash\">",
        "stop-opacity=\"0.94\"",
        "fill=\"url(#blueWash)\"",
        "<rect x=\"4\"",
        "stroke=\"#1e40af\"",
        "<circle cx=\"46\"",
        "fill=\"none\" stroke=\"#1d4ed8\"",
        "stroke-opacity=\"0.45\"",
        "<polygon points=\"92,26 132,48 92,70\"",
        "<g transform=\"translate(8 4) scale(0.9)\">",
        "<line x1=\"16\"",
        "stroke-linecap=\"round\"",
        "<path d=\"M18 20 H54 V34 H18 Z",
        "<text x=\"18\" y=\"76\"",
        "fill=\"currentColor\"",
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
    print("generic SVG support audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)
print("generic SVG support audit passed")
