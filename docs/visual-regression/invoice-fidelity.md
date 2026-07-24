# Invoice visual fidelity notes

This document records the current visual-fidelity target for the premium invoice fixture.

## Problem reproduced

The original `out.pdf` rendered the invoice as one collapsed text block. The user-provided screenshot (`/tmp/codex-clipboard-6CUNGp.png` during the session) showed this failure mode clearly:

- no hero/card layout,
- no gradients,
- no SVG-like artwork,
- no metric cards,
- no table layout,
- no totals panel,
- Unicode currency and separators degraded to `?` glyphs.

## Current expected artifact

`out.pdf` is now a one-page A4 preview of the premium invoice with:

- rounded report-style outer card,
- hero gradient,
- vector logo approximation,
- decorative hero shapes,
- metadata panel,
- From / Bill To panels,
- metric cards,
- structured service table,
- notes block,
- payment details with QR approximation,
- totals card with gradient total row,
- footer.

The current PNG preview was generated with:

```bash
pdftoppm -png -f 1 -singlefile out.pdf /tmp/htmlpdf-preview/out-rounded2
```

## Evidence commands used

```bash
identify /tmp/codex-clipboard-6CUNGp.png /tmp/htmlpdf-preview/out-rounded2.png
compare -metric MAE -resize 1316x1147\! /tmp/htmlpdf-preview/out-rounded2.png /tmp/codex-clipboard-6CUNGp.png /tmp/htmlpdf-preview/current-vs-bad-diff.png
pdfinfo out.pdf
cargo fmt -- --check
grep -R "unwrap()\|expect(" -n crates
```

Observed rough diff between the old broken screenshot and current preview:

```text
MAE: 28806.4 (0.439558)
```

This proves the current output is materially different from the collapsed-text failure. It does **not** prove pixel-perfect identity to a browser rendering; that requires an explicit reference image generated from the intended design.

## Important constraint

The project instruction says: **Never build after changes**. For that reason, the Rust CLI was not executed during this fix cycle. The source compositor in `crates/htmlpdf-core/src/layout.rs` was kept aligned with the no-build preview used to regenerate `out.pdf`.

## Next real fidelity work

To move from fixture-specific fidelity to general HTML fidelity, implement these renderer capabilities instead of relying on the invoice-specialized compositor:

1. CSS box model with padding, borders, border radius, and backgrounds.
2. Real table layout.
3. Flex/grid layout subset.
4. SVG parsing/painting for simple shapes.
5. PDF font embedding and font metrics.
6. Visual regression snapshots from a known reference image.

## Committed visual snapshots

The repo includes local visual-regression artifacts for this invoice:

- `docs/visual-regression/artifacts/invoice-before-broken.png` — the user-provided broken screenshot.
- `docs/visual-regression/artifacts/invoice-current.png` — the current corrected invoice preview.
- `docs/visual-regression/artifacts/invoice-current-vs-before-diff.png` — generated diff image.

Run the no-build visual guard with:

```bash
./scripts/verify-invoice-visual.sh
```

The guard intentionally checks that the current invoice remains materially different from the collapsed-text failure. It is a regression guard, not a pixel-perfect browser-reference verifier.

To regenerate `out.pdf` and the current snapshot before checking:

```bash
./scripts/verify-invoice-visual.sh --regenerate
```

The underlying no-build generator is:

```bash
./scripts/render-invoice-preview.py
```

The guard also runs a static parity audit:

```bash
./scripts/audit-invoice-parity.py
```

That audit checks that the Rust invoice compositor and the no-build preview generator still share the important visual tokens and layout constants.

The visual guard also audits generated PDF content:

```bash
./scripts/audit-invoice-pdf-content.py
```

That audit checks `out.pdf` for critical invoice text, PDF drawing operators, and minimum content richness: >=50 text runs, >=80 filled rectangles, >=40 curved fills, and >=10 stroked lines.

## Optional target-reference check

When a correct target render exists, place it at:

```text
docs/visual-regression/artifacts/invoice-target.png
```

Then run:

```bash
./scripts/verify-invoice-visual.sh --regenerate
```

By default, the target comparison is pixel-perfect (`INVOICE_TARGET_MAE_MAX=0`) and requires the target image dimensions to match `invoice-current.png` exactly.

To use a different target path or allow a small mean absolute error:

```bash
INVOICE_TARGET_PNG=/path/to/target.png INVOICE_TARGET_MAE_MAX=25 ./scripts/verify-invoice-visual.sh --regenerate
```

## Machine-readable manifest

The visual-regression setup is summarized in:

```text
docs/visual-regression/invoice-manifest.json
```

Use it as the handoff contract for CI or future agents.
