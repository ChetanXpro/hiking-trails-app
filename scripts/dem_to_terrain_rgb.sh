#!/usr/bin/env bash
set -euo pipefail

# Mosaics one or more DEM GeoTIFFs (FABDEM / Copernicus tiles) and encodes
# them as Mapbox-standard Terrain-RGB tiles in a single MBTiles file.
#
# Usage:
#   ./scripts/dem_to_terrain_rgb.sh output.mbtiles input1.tif [input2.tif ...]
#
# Environment overrides:
#   MIN_ZOOM (default 6)   MAX_ZOOM (default 13)   WORKERS (default 4)
#
# Requires:
#   - gdalbuildvrt
#   - gdal_translate
#   - gdaladdo
#   - rio-rgbify
#   - sqlite3
#
# Example:
#   ./scripts/dem_to_terrain_rgb.sh ./data/terrain_rgb.mbtiles ./data/dem_tiles/*.tif

if [ "$#" -lt 2 ]; then
	echo "Usage: $0 output.mbtiles input1.tif [input2.tif ...]" >&2
	exit 1
fi

for required_cmd in gdalbuildvrt gdal_translate gdaladdo sqlite3; do
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
MIN_ZOOM_VALUE="${MIN_ZOOM:-6}"
MAX_ZOOM_VALUE="${MAX_ZOOM:-13}"

if ! [[ "$MIN_ZOOM_VALUE" =~ ^[0-9]+$ && "$MAX_ZOOM_VALUE" =~ ^[0-9]+$ ]]; then
	echo "MIN_ZOOM and MAX_ZOOM must be non-negative integers" >&2
	exit 1
fi

if (( MAX_ZOOM_VALUE < MIN_ZOOM_VALUE )); then
	echo "MAX_ZOOM must be greater than or equal to MIN_ZOOM" >&2
	exit 1
fi

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

echo "==> Building average DEM overviews for cleaner small-scale relief..."
gdaladdo \
	-r average \
	--config COMPRESS_OVERVIEW DEFLATE \
	"$MERGED" 2 4 8 16 32 64 128 256

echo "==> Encoding as Terrain-RGB (Mapbox encoding: base=-10000, interval=0.1)..."
python3 "$(dirname "$0")/rio_rgbify_compat.py" \
	-b -10000 \
	-i 0.1 \
	--min-z "$MIN_ZOOM_VALUE" \
	--max-z "$MAX_ZOOM_VALUE" \
	--format png \
	-j "${WORKERS:-4}" \
	-v \
	"$MERGED" "$OUTPUT"

echo "==> Indexing Terrain-RGB tiles for fast z/x/y lookup..."
sqlite3 "$OUTPUT" "
	CREATE UNIQUE INDEX IF NOT EXISTS tile_index ON tiles (zoom_level, tile_column, tile_row);
	DELETE FROM metadata WHERE name IN ('minzoom', 'maxzoom');
	INSERT INTO metadata (name, value) VALUES ('minzoom', '$MIN_ZOOM_VALUE');
	INSERT INTO metadata (name, value) VALUES ('maxzoom', '$MAX_ZOOM_VALUE');
	ANALYZE;
"

echo ""
echo "==> Done: $OUTPUT"
echo "    rio-rgbify writes 512px Mapbox Terrain-RGB tiles. Configure the"
echo "    frontend with VITE_TERRAIN_RGB_TILE_SIZE=512 and the same max zoom."
echo "    Copy this next to your Martin server's data directory. Martin will"
echo "    auto-discover it and serve it at /$(basename "$OUTPUT" .mbtiles)/{z}/{x}/{y}"
