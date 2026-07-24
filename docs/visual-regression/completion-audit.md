# Completion audit — premium invoice render fidelity

## Objective restated

User objective: "no renderiza nada bien nada [Image #1] dejalo perfecto hasta que sea identico".

Concrete deliverables derived from that objective:

1. The bad invoice output shown in `Image #1` must no longer be the generated result.
2. `out.pdf` must render as a structured premium invoice, not a collapsed text block.
3. The generated PDF must contain the critical invoice business content.
4. The no-build preview artifact must remain aligned with the Rust invoice compositor.
5. There must be repeatable evidence for visual regression checks.
6. Pixel-perfect identity can only be claimed if there is a correct target reference image. The available `Image #1` is the broken baseline, not the desired target.

## Prompt-to-artifact checklist

| Requirement / gate | Evidence inspected | Status |
| --- | --- | --- |
| `Image #1` investigated | `docs/visual-regression/artifacts/invoice-before-broken.png` copied from `/tmp/codex-clipboard-6CUNGp.png` | Complete: this is the broken collapsed-text render |
| Current output is not the broken render | `./scripts/verify-invoice-visual.sh --regenerate` reports `MAE 28814 (0.439674)` against the broken baseline | Complete for regression-away-from-broken |
| Current PDF exists and is page-shaped | `pdfinfo out.pdf` inside the guard reports PDF 1.4, 1 page, A4 `595 x 842 pts` | Complete |
| Current output has premium invoice structure | `docs/visual-regression/artifacts/invoice-current.png` shows hero, cards, table, notes, QR, totals, footer; `scripts/audit-invoice-image-quality.py` reports `colors=7032 global_std=0.299 hero_mean=0.383 hero_colors=3241 total_std=0.195` | Complete by visual inspection plus raster-quality guard |
| Generated PDF includes invoice business content | `scripts/audit-invoice-pdf-content.py` checks critical strings and enforces minimum richness: >=50 text runs, >=80 filled rectangles, >=40 curves, >=10 stroked lines; latest output reports `text_runs=76 rect_ops=108 curve_ops=55 line_ops=24 pages=1` | Complete |
| Preview generator and Rust compositor are aligned | `scripts/audit-invoice-parity.py` is run by `verify-invoice-visual.sh` and passes | Complete |
| Table and bottom cards do not collide | `scripts/audit-invoice-layout-geometry.py` is run by `verify-invoice-visual.sh` and reports `table_to_notes=10.0pt table_to_totals=14.0pt totals_to_signature=6.0pt signature_to_footer=4.0pt` | Complete |
| Signature SVG content is represented | `scripts/audit-invoice-pdf-content.py` requires `AUTHORIZED SIGNATURE`; `scripts/audit-invoice-parity.py` requires `draw_compact_signature` and preview `signature_points`; latest output reports `text_runs=76` and `line_ops=24` | Complete |
| Verification is reproducible without Rust build | `scripts/render-invoice-preview.py` and `scripts/verify-invoice-visual.sh --regenerate` regenerate and verify `out.pdf` | Complete |
| Rust build/test/run verification | Not run because project instruction says **Never build after changes** | Intentionally not covered |
| Pixel-perfect identity to intended target | `scripts/verify-invoice-visual.sh --require-target` fails unless `invoice-target.png` / `INVOICE_TARGET_PNG` exists and matches within `INVOICE_TARGET_MAE_MAX`; no correct target image exists; `Image #1` is the broken baseline | Not complete / unverifiable |

## Latest verification output

Command:

```bash
./scripts/verify-invoice-visual.sh --regenerate
```

Observed output:

```text
visual regression guard passed
invoice image quality audit passed colors=7032 global_std=0.299 hero_mean=0.383 hero_colors=3241 total_std=0.195
invoice parity audit passed
invoice layout geometry audit passed table_to_notes=10.0pt table_to_totals=14.0pt totals_to_signature=6.0pt signature_to_footer=4.0pt
invoice PDF content audit passed
text_runs=76 rect_ops=108 curve_ops=55 line_ops=24 pages=1
```

Strict target gate:

```bash
./scripts/verify-invoice-visual.sh --regenerate --require-target
```

Expected current result while no correct target exists:

```text
missing required target reference: docs/visual-regression/artifacts/invoice-target.png
```

## Completion decision

Do **not** mark the goal complete yet.

The implementation has fixed the concrete failure represented by `Image #1`—the collapsed text render—and added repeatable no-build verification. However, the phrase "perfecto" / "idéntico" requires a correct target reference image. The only available image is the broken baseline, so pixel-perfect identity to the intended design is not objectively verifiable.

## What remains to truly close the goal

One of these must happen:

1. Provide a correct target screenshot/reference render for the intended invoice design at `docs/visual-regression/artifacts/invoice-target.png` or via `INVOICE_TARGET_PNG`, then run `./scripts/verify-invoice-visual.sh --regenerate --require-target`.
2. Permit running the Rust CLI/build path so the real renderer output can be compared against the current no-build preview.
3. Re-scope the objective from "identical" to "no longer collapsed; premium invoice render verified by current guard".
