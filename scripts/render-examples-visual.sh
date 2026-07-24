#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="${1:-$ROOT_DIR/out-examples}"
RASTER_DIR="${2:-$ROOT_DIR/out-raster}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "missing dependency: cargo" >&2
  exit 1
fi
if ! command -v pdftoppm >/dev/null 2>&1; then
  echo "missing dependency: pdftoppm" >&2
  exit 1
fi
if ! command -v identify >/dev/null 2>&1; then
  echo "missing dependency: ImageMagick identify" >&2
  exit 1
fi

mkdir -p "$OUT_DIR" "$RASTER_DIR"

echo "== Render examples with real CLI =="
cargo run -p htmlpdf-cli -- render "$ROOT_DIR"/examples/*.html -o "$OUT_DIR"

echo "== Rasterize first page of each PDF =="
for pdf in "$OUT_DIR"/*.pdf; do
  name=$(basename "$pdf" .pdf)
  pdftoppm -png -f 1 -singlefile "$pdf" "$RASTER_DIR/$name"
done

echo "== Raster outputs =="
identify "$RASTER_DIR"/*.png

echo "visual smoke complete: pdfs=$OUT_DIR rasters=$RASTER_DIR"
