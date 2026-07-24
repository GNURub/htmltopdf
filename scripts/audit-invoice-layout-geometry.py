#!/usr/bin/env python3
"""Guard key premium invoice geometry without building Rust.

The visual snapshot catches regressions after rasterization, but this small
audit protects the specific spacing bug that made the table collide with the
notes card. It mirrors the constants used by the no-build preview and the Rust
fixture compositor.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass


PAGE_HEIGHT = 842.0


@dataclass(frozen=True)
class Band:
    name: str
    bottom: float
    top: float


def require_gap(upper: Band, lower: Band, minimum: float) -> list[str]:
    """Return an error if `upper` does not sit at least minimum pt above lower."""
    gap = upper.bottom - lower.top
    if gap < minimum:
        return [
            f"{upper.name} overlaps {lower.name}: gap={gap:.1f}pt, minimum={minimum:.1f}pt"
        ]
    return []


def main() -> int:
    table_top = PAGE_HEIGHT - 444.0
    table = Band("items table", table_top - 28.0 - (4.0 * 42.0), table_top)

    bottom_y = 44.0
    notes = Band("notes card", bottom_y + 88.0, bottom_y + 88.0 + 60.0)
    payment = Band("payment details", bottom_y + 10.0, bottom_y + 10.0 + 62.0)
    totals = Band("totals card", bottom_y + 20.0, bottom_y + 20.0 + 124.0)
    signature = Band("signature", 34.0, 58.0)
    footer = Band("footer text", 22.0, 30.0)

    errors: list[str] = []
    errors += require_gap(table, notes, 8.0)
    errors += require_gap(table, totals, 8.0)
    errors += require_gap(notes, payment, 14.0)
    errors += require_gap(totals, signature, 4.0)
    errors += require_gap(signature, footer, 3.0)
    errors += require_gap(payment, footer, 20.0)

    if errors:
        print("invoice layout geometry audit failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(
        "invoice layout geometry audit passed "
        f"table_to_notes={table.bottom - notes.top:.1f}pt "
        f"table_to_totals={table.bottom - totals.top:.1f}pt "
        f"totals_to_signature={totals.bottom - signature.top:.1f}pt "
        f"signature_to_footer={signature.bottom - footer.top:.1f}pt"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
