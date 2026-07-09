#!/usr/bin/env bash
set -euo pipefail

# Mosaics one or more DEM GeoTIFFs (FABDEM / Copernicus tiles) and encodes
# them as Mapbox-standard Terrain-RGB tiles in a single MBTiles file.
#
# Usage:
#   ./scripts/dem_to_terrain_rgb.sh output.mbtiles input1.tif [input2.tif ...]
#
# Environment overrides:
#   MIN_ZOOM (default 6)   MAX_ZOOM (default 14)   WORKERS (default 4)
#
# Requires:
#   - gdalbuildvrt
#   - gdal_translate
#   - rio-rgbify
#
# Example:
#   ./scripts/dem_to_terrain_rgb.sh ./data/terrain_rgb.mbtiles ./data/dem_tiles/*.tif

if [ "$#" -lt 2 ]; then
	echo "Usage: $0 output.mbtiles input1.tif [input2.tif ...]" >&2
	exit 1
fi

for required_cmd in gdalbuildvrt gdal_translate; do
	if ! command -v "$required_cmd" >/dev/null 2>&1; then
		echo "Missing required command: $required_cmd" >&2
		exit 1
	fi
done

if ! command -v rio >/dev/null 2>&1; then
	echo "Missing required command: rio" >&2
	echo "" >&2
	echo "Install rio-rgbify first, then rerun this script:" >&2
	echo "  pip3 install rio-rgbify --break-system-packages" >&2
	echo "" >&2
	echo "If you prefer isolated tooling, you can also use:" >&2
	echo "  uv tool install rio-rgbify" >&2
	exit 1
fi

OUTPUT="$1"
shift
INPUTS=("$@")

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

VRT="$WORKDIR/mosaic.vrt"
MERGED="$WORKDIR/mosaic.tif"

echo "==> Mosaicking ${#INPUTS[@]} input tile(s)..."
gdalbuildvrt "$VRT" "${INPUTS[@]}"

echo "==> Flattening mosaic to a single compressed GeoTIFF..."
gdal_translate \
	-co COMPRESS=DEFLATE \
	-co TILED=YES \
	-co BIGTIFF=IF_SAFER \
	"$VRT" "$MERGED"

echo "==> Encoding as Terrain-RGB (Mapbox encoding: base=-10000, interval=0.1)..."
rio rgbify \
	-b -10000 \
	-i 0.1 \
	--min-z "${MIN_ZOOM:-6}" \
	--max-z "${MAX_ZOOM:-14}" \
	--format png \
	-j "${WORKERS:-4}" \
	-v \
	"$MERGED" "$OUTPUT"

echo ""
echo "==> Done: $OUTPUT"
echo "    Copy this next to your Martin server's data directory. Martin will"
echo "    auto-discover it and serve it at /$(basename "$OUTPUT" .mbtiles)/{z}/{x}/{y}"
