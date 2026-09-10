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
`vertical-align` variants, baseline selection with positioned descendants,
oversized atomic boxes, nested fixed-position overlays, and full mixed-flow
layout need further validation and implementation.
