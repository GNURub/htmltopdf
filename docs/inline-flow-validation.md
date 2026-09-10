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
baselines and rasterization still differ. Atomic `inline-block` layout, mixed
inline/block children, and comprehensive modern CSS compatibility remain open.
Chromium is used only as a test reference, never by the rendering runtime.
