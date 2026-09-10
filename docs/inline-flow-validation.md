# Inline flow validation

The `examples/generic-inline-flow.html` fixture exercises adjacent styled inline
text, source whitespace, wrapping inside padded backgrounds, and relative
positioning without moving subsequent normal-flow content.

Changes covered by regression tests:

- Text nodes inherit text properties, not the parent's box dimensions or offsets.
- Common phrasing elements have inline user-agent display defaults.
- Text-only inline children of containers share an inline formatting context;
  the container remains responsible for its background, padding and positioning.
- Source whitespace between elements survives parsing and collapses during layout.
- Whitespace between blocks does not interrupt adjacent-margin collapse.
- Justification writes the same word spacing used to position subsequent runs.
- Unembedded PDF fallback fonts use standard font names, including bold variants.
- A word spanning multiple styled text runs is measured and wrapped as a unit;
  styling boundaries do not introduce soft wrap opportunities.
- U+00A0 and U+202F survive whitespace normalization in inline and plain-text
  layout. End-to-end HTML tests cover `&nbsp;` with and without nested emphasis
  under `overflow-wrap: normal`.

Whitespace normalization follows the document-whitespace distinction in
[CSS Text](https://www.w3.org/TR/css-text-3/#white-space-processing), not Rust's
broader Unicode `is_whitespace` classification. This does not yet implement the
full Unicode line-breaking algorithm or all CSS whitespace modes.

## Visual evidence (2026-09-10)

Compared the complete fixture PDF with the locally installed Chromium headless
shell (Playwright revision 1243), rasterizing both through Poppler. Both PDFs
have one page and identical 667 x 500 raster dimensions; no resizing was used.
After the inline-flow changes, normalized RMSE was 0.126381 with the old font
fallback and 0.085043 with standard PDF fallback names. Visual inspection
confirmed restored bold text and one background per box. This comparison does
not measure the entire patch against its parent commit.

Reproduce using a locally installed test browser:

```sh
python3 scripts/audit-browser-parity.py examples/generic-inline-flow.html \
  --browser /path/to/chrome-headless-shell --pages all \
  --out-dir /tmp/htmlpdf-inline-validation
```

This is not a production-readiness or Chromium-parity claim. Font metrics,
baselines and rasterization still differ. Mixed inline/block children and
comprehensive modern CSS compatibility remain open.
Chromium is used only as a test reference, never by the rendering runtime.

## Atomic inline boxes

`display: inline-block` is now a distinct computed display value. Inline layout
measures its independent contents into a reusable paint list, retains box width,
height and baseline, then moves the complete box when wrapping. Line height
accounts for atomic boxes instead of always using the parent's text line height.
The measurement context does not clone previously rendered document pages.

Regression tests cover consecutive boxes, complete-box line and page breaks,
empty boxes, fixed dimensions with content-box padding, minimum dimensions,
hidden boxes reserving space, and floats contributing to auto height. Related
fixes correct content-box height constraints and min/max width constraints.

The complete `examples/generic-inline-block.html` fixture was compared with the
same Chromium headless shell and Poppler process. Both outputs have one page and
667 x 500 raster dimensions without resizing. Normalized RMSE is 0.0666196.
Visual review exposed a height/padding defect during development; correcting it
reduced the metric from 0.0888212. These numbers compare two development versions,
not the complete patch against its parent commit. The visual fixture covers
badges, adjacent tiles wrapping to another line, and an empty inline box.

This is initial support, not complete inline formatting conformance. In
particular, intrinsic sizing still uses approximate text measurements;
`vertical-align` variants beyond the top/bottom support below, baseline selection with positioned descendants,
oversized atomic boxes, nested fixed-position overlays, and full mixed-flow
layout need further validation and implementation.

## Sizing constraints

The general and normal-flow width resolvers now share the same calculation,
including auto widths with min/max constraints. A minimum width takes priority
over a conflicting maximum; an oversized specified or minimum width is not
silently clamped to its container. Border-box dimensions are floored at padding
plus borders, so the implied content size cannot become negative. Aspect-ratio
height calculations use content-box or border-box dimensions according to
`box-sizing`.

Four regression tests cover these rules, including document-level placement of
the text after a zero-sized border box with padding. These additions were
verified through layout geometry tests; no new visual-parity score is claimed
for them. Normative references: [CSS Sizing 3](https://www.w3.org/TR/css-sizing-3/)
and [CSS Sizing 4](https://www.w3.org/TR/css-sizing-4/).

## Top and bottom inline alignment

Atomic inline boxes now apply `vertical-align: top` and `bottom` when measuring
and painting a line. Their height constrains the line edges rather than their
internal text baseline. Tests cover exact document coordinates beside a taller
baseline-aligned box, isolated edge alignment, and combined top/bottom boxes
without excess line height. These are geometry tests, not a new browser-parity
measurement. Reference: [CSS line height calculations](https://www.w3.org/TR/CSS22/visudet.html#line-height).

`middle`, independent `text-top`/`text-bottom` semantics, super/subscript offsets,
and arbitrary length/percentage vertical alignment still require work. The
current parser aliases `text-top`/`text-bottom` to `top`/`bottom`; that is not a
claim that those CSS values are equivalent.

## Visual acceptance gate

When `--threshold-rmse-normalized` is supplied, the browser comparator now
requires every page from both PDFs to be compared, matching raster dimensions
without reference resizing, and all compared pages within the error threshold.
`--pages first` cannot approve a multipage document; neither can a restrictive
`--max-pages` cap. A resized reference remains available for diagnostic images
and metrics, but cannot produce an accepted gate result.

Reports include `all_pages_compared`, `raster_dimensions_match`, and explicit
`failure_reasons`. Eleven browser-independent tests exercise these gates,
invalid page counts, and valid/invalid ImageMagick metric output. Running
without a threshold remains diagnostic only. These stricter checks do not
establish renderer conformance by themselves, and raster dimensions do not
verify every PDF page-box property.

## Transforms after a page break

Form controls now capture their paint range after space reservation has selected
the destination page. A regression test initially reproduced a control moving
to page two while losing its 20pt translation, then passed after correcting the
range. The test covers inputs, textarea text and its clip, plus unchanged
surrounding flow.

Images now apply the existing element-transform path to their paint range,
including object-fit clipping, after selecting the destination page. A second
regression test checks translated image/clip coordinates on page two and
unmodified surrounding text. These are translation/geometry checks, not proof
of full rotation, skew or multipage transform compatibility.

## Borders and ordinary container content

Ordinary block and measured inline-block containers now reserve border widths
as well as padding around their children. Previously a 2pt border did not inset
text and was absent from the natural content-driven height. The regression
test failed before the correction and passes for plain text, inline spans and
nested blocks. Additional checks compare right-aligned and wrapped text against
equivalent padding for both box-sizing modes, and verify that first-line page
reservation includes vertical borders.

This follows the [CSS box model](https://www.w3.org/TR/css-box-3/). These are
layout-coordinate checks, not a new raster parity score. Independent flex,
grid, table and control layout paths, border-side zero overrides, positioned
containing-block edges and fragmented border decoration still need validation
and corrections; this change does not establish complete box-model parity.

Follow-up: per-side width longhands now control ordinary container sizing,
content insets, background geometry and border painting without taking the
maximum with the earlier shorthand. A regression first reproduced an explicitly
zero left border being restored to 4pt. It now checks zero top/left edges,
smaller right/bottom strokes, content coordinates and following-block height.
The existing box-size floor test now constructs all four computed longhands,
as the CSS shorthand parser does, while retaining its numeric assertions.
Per-side border styles and shorthand resets, synthetic table borders and native
control defaults remain separate incomplete paths. No visual parity score is
claimed for this correction.

`border-width` now expands one through four components in CSS side order,
including functional lengths and thin/medium/thick (1/3/5 CSS pixels).
A computed-style regression checks all four output sides and verifies that
negative, percentage, non-finite and overlong lists leave the preceding valid
declaration intact. This does not yet unify keyword handling across individual
longhands or implement per-side styles and their computed zero-width rules.
Reference: [CSS border widths](https://www.w3.org/TR/CSS2/box.html#border-width-properties).

The individual physical/logical width longhands and the two logical axis width
properties now share that component parser. Regression cases cover all ten
properties with keywords, zero, functional lengths and invalid negative,
non-finite and percentage values. Axis pairs reject an invalid component as a
whole. Logical-to-physical mapping for nondefault writing modes remains
incomplete, as do border-style-dependent used widths and general `border`
shorthand resets. These tests validate computed values, not raster parity.

Border colors now expand one through four values, preserving function arguments
and supporting transparent/currentColor components. Invalid lists and invalid
physical/logical color longhands preserve previous valid colors. The regression
also exposed nonstandard basic named colors: red, blue, green and gray/grey now
match their sRGB hexadecimal values. Separate exact-value tests cover those
names. This changes named-color output across CSS and SVG consumers; no new
visual comparison score is asserted. Deferred currentColor resolution and
per-side border styles still need further work.

The named-color catalog was subsequently checked against the complete
[W3C CSS Color 4 table](https://www.w3.org/TR/css-color-4/#named-colors).
Twenty-three absent names were added. An offline reference fixture contains
all 148 name/hex pairs, with source and retrieval date; a unit test checks every
RGB value, uppercase spelling and unique entry count through the runtime color
parser. This proves the standard named-color lookup, not general color-space,
compositing or PDF color-management fidelity. No dependency was added.

Malformed hexadecimal color validation now precedes byte-pair slicing. A
regression reproduced a panic when a multibyte UTF-8 character crossed a pair
boundary. The parser rejects non-ASCII hexadecimal digits and signs before
slicing. Tests cover 567 combinations of Unicode/control/invalid characters
and positions, valid short/long RGB and RGBA equivalents, uppercase digits and
invalid lengths. This fixes that specific panic; it is not an exhaustive
malformed-input or resource-exhaustion audit of the renderer.

Absolute RGB/RGBA parsing now distinguishes comma-separated legacy syntax from
space-separated modern syntax. It rejects extra channels, empty components,
mixed separator forms, repeated slashes and invalid alpha instead of silently
using opacity one. Legacy channel units must be consistent; modern syntax
accepts mixed number/percentage channels and resolves `none` to zero for direct
painting. Regression cases include malformed Unicode alpha and valid alpha
percentages. Relative RGB, other functional color grammars, missing-component
interpolation and full CSS numeric-token validation remain incomplete.
Reference: [CSS RGB functions](https://www.w3.org/TR/css-color-4/#rgb-functions).

RGB and shared alpha channel parsing now validate numeric token structure and
finiteness before clamping. NaN/infinity spellings, malformed exponents,
trailing decimal points and whitespace inside percentage tokens are rejected;
signed fractions, scientific notation, percentages and finite out-of-range
clamping are tested. Numbers exceeding f32 capacity are currently rejected,
rather than implementing full CSS numeric range handling. Other color models
may still substitute their alpha default when their shared parser rejects a
value; their complete grammar and fallback behavior remain to be corrected.

## Solid border raster comparison

`examples/generic-border-box-geometry.html` isolates uniform and unequal solid
borders without fonts. Visual review exposed strokes extending outside their
border boxes and overlapping corner colors. Non-rounded solid borders now use
an inset single stroke for uniform sides or four joined side polygons for
unequal widths/colors. A coordinate test checks the inset stroke, while the
side-width regression now measures polygon thicknesses instead of line widths.

Full-page comparison against the installed Chromium headless shell produced
one 500 x 375 raster from each PDF without resizing. Normalized RMSE fell from
0.207669 to 0.0527316. Both final rasters were inspected; remaining differences
include subpixel edge placement/rasterization. No acceptance threshold was
asserted. Rounded borders, dashed/dotted corners, extreme overlapping border
widths and fragmented decorations are not validated by this fixture. The
existing inline-block fixture was also rechecked at 0.0666196 before this
border-paint change; it is not a post-change suite-wide parity result.
