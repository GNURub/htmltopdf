# ADR-0001: Generic renderer by default, no silent fixture special-cases

## Status
Accepted

## Context

`htmlpdf` is intended to become an open-source tool that lets users generate a
PDF from arbitrary document HTML (`html + css + js`) through a CLI and API.
During visual-regression work, the repository gained a premium invoice fixture
compositor to prove that the PDF backend can create a polished document without
Chromium.

That fixture is useful as a benchmark, but it is not a product architecture. If
the public renderer silently detects one specific invoice and swaps in hardcoded
coordinates, the project becomes a template renderer disguised as an HTML
renderer. That would be misleading for users and impossible to scale across
open-source contributions.

## Decision

The public rendering path is generic by default.

Demo/fixture compositors may exist only behind an explicit opt-in mode named
`demo-fixture`. They are allowed for visual-regression evidence and PDF backend
stress tests, but they must not be the default CLI/API behavior.

Concretely:

- `RenderOptions::default().render_mode == RenderMode::Generic`
- CLI default is `--render-mode generic`
- fixture rendering requires explicit `--render-mode demo-fixture`
- API service uses default generic rendering unless a future API explicitly adds
  a documented render-mode parameter

## Alternatives considered

### Keep automatic fixture detection

- Positive: prettier demo output for `examples/invoice.html`.
- Negative: dishonest open-source contract; arbitrary HTML would not benefit;
  hardcoded template paths would accumulate architectural debt.

### Remove demo fixture compositor completely

- Positive: pure generic engine only.
- Negative: loses a useful visual stress fixture while the generic layout engine
  is still being built.

### Keep fixture compositor but make it explicit

- Positive: preserves visual-regression value without corrupting the public
  default behavior.
- Negative: two modes must be documented clearly.

Chosen option: explicit fixture mode.

## Consequences

Positive:

- The default renderer behaves like an actual HTML renderer, not a hidden invoice
  template.
- Open-source users can reason about supported features and file issues against
  the generic engine.
- Demo fixtures remain useful for visual regression while the engine matures.

Negative:

- `examples/invoice.html` will not look premium through the default generic
  renderer until the generic layout engine supports the needed CSS/features.
- The project must invest in real generic layout capabilities: boxes, flex/grid,
  tables, borders, radius, backgrounds, assets, pagination, and better text.

## Follow-up work

1. Add conformance fixtures for generic HTML/CSS features instead of single-page
   hardcoded demos.
2. Expand layout primitives into a real box tree.
3. Implement generic painting for borders, backgrounds, radius, and simple SVG.
4. Add fixture-level visual tests that render through `RenderMode::Generic`.
5. Keep demo fixture usage visibly separated from production CLI/API docs.
