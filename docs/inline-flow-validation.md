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
