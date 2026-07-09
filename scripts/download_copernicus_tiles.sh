#!/usr/bin/env bash
set -euo pipefail

# Bulk-download Copernicus GLO-30 DEM tiles (30m, public, no registration)
# covering a lat/lon bounding box.
#
# Usage:
#   ./scripts/download_copernicus_tiles.sh <south> <west> <north> <east> <output_dir>
#
# Example:
#   ./scripts/download_copernicus_tiles.sh 28 76 35 82 ./data/dem_tiles
#
# Each tile is a 1x1 degree cell named by its SW corner, for example:
#   Copernicus_DSM_COG_10_N32_00_E077_00_DEM
#
# Not every cell has a public tile. Missing cells are skipped with a warning.

if [ "$#" -ne 5 ]; then
	echo "Usage: $0 <south> <west> <north> <east> <output_dir>" >&2
	echo "Example: $0 28 76 35 82 ./data/dem_tiles" >&2
	exit 1
fi

SOUTH="$1"
WEST="$2"
NORTH="$3"
EAST="$4"
OUTDIR="$5"

mkdir -p "$OUTDIR"

BASE_URL="https://copernicus-dem-30m.s3.amazonaws.com"
FETCHED=0
SKIPPED=0

for (( lat=SOUTH; lat<NORTH; lat++ )); do
	for (( lon=WEST; lon<EAST; lon++ )); do
		if [ "$lat" -ge 0 ]; then
			lat_code="$(printf 'N%02d_00' "$lat")"
		else
			lat_code="$(printf 'S%02d_00' "$((-lat))")"
		fi

		if [ "$lon" -ge 0 ]; then
			lon_code="$(printf 'E%03d_00' "$lon")"
		else
			lon_code="$(printf 'W%03d_00' "$((-lon))")"
		fi

		tile_name="Copernicus_DSM_COG_10_${lat_code}_${lon_code}_DEM"
		url="${BASE_URL}/${tile_name}/${tile_name}.tif"
		dest="${OUTDIR}/${tile_name}.tif"

		if [ -f "$dest" ]; then
			echo "  already have ${tile_name}, skipping"
			continue
		fi

		status="$(curl -sS -L -o "$dest" -w '%{http_code}' "$url")"
		if [ "$status" = "200" ]; then
			size="$(du -h "$dest" | cut -f1)"
			echo "  fetched ${tile_name} (${size})"
			FETCHED=$((FETCHED + 1))
		else
			rm -f "$dest"
			echo "  no public tile for ${tile_name} (HTTP ${status}) -- skipping"
			SKIPPED=$((SKIPPED + 1))
		fi
	done
done

echo ""
echo "==> Done: ${FETCHED} tile(s) downloaded, ${SKIPPED} cell(s) skipped, into ${OUTDIR}/"
echo "    Next: ./scripts/dem_to_terrain_rgb.sh ./data/terrain_rgb.mbtiles ${OUTDIR}/*.tif"
