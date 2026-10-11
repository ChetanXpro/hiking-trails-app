#!/usr/bin/env bash
set -euo pipefail

# Fuse the local NASA HMA 8 m mosaic over Copernicus GLO-30 for Himachal.
#
# HMA is the higher-resolution, higher-priority source. Its elevations are
# WGS84 ellipsoid heights, so an EGM2008 geoid grid is subtracted before the
# data is combined with Copernicus GLO-30 (EGM2008 orthometric heights).
#
# Usage:
#   ./scripts/fuse_hma_copernicus_dem.sh
#
# Optional overrides:
#   HMA=/path/to/hma_3857.tif
#   DEMDIR=/path/to/copernicus_tiles
#   OUTDIR=/path/to/output_directory
#   OUTPUT=/path/to/fused.tif
#   GEOID_GRID=/path/to/us_nga_egm08_25.tif

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HMA="${HMA:-$ROOT/hma_dem/hma_3857.tif}"
DEMDIR="${DEMDIR:-$ROOT/data/dem_tiles}"
OUTDIR="${OUTDIR:-$ROOT/data/dem_fusion}"
OUTPUT="${OUTPUT:-$OUTDIR/fused_himachal_egm2008.tif}"

# The 16 downloaded GLO-30 tiles span this rectangle. Keeping the processing
# extent here avoids expanding the result to the much larger HMA-wide mosaic.
WEST="${WEST:-75}"
SOUTH="${SOUTH:-30}"
EAST="${EAST:-79}"
NORTH="${NORTH:-34}"

for command in gdalbuildvrt gdalwarp gdal_calc.py gdalinfo gdaldem gdal_translate jq projinfo; do
	if ! command -v "$command" >/dev/null 2>&1; then
		echo "Missing required command: $command" >&2
		exit 1
	fi
done

if [[ ! -f "$HMA" ]]; then
	echo "HMA mosaic not found: $HMA" >&2
	exit 1
fi

COPERNICUS_TILES=("$DEMDIR"/Copernicus_DSM_COG_10_*_DEM.tif)
if [[ ! -e "${COPERNICUS_TILES[0]}" ]]; then
	echo "No Copernicus tiles found in: $DEMDIR" >&2
	exit 1
fi

if [[ -e "$OUTPUT" ]]; then
	echo "Output already exists; move it or choose OUTPUT=... before rerunning:" >&2
	echo "  $OUTPUT" >&2
	exit 1
fi

GEOID_GRID="${GEOID_GRID:-}"
if [[ -z "$GEOID_GRID" ]]; then
	while IFS= read -r search_path; do
		if [[ -f "$search_path/us_nga_egm08_25.tif" ]]; then
			GEOID_GRID="$search_path/us_nga_egm08_25.tif"
			break
		fi
	done < <(projinfo --searchpaths)
fi

if [[ -z "$GEOID_GRID" || ! -f "$GEOID_GRID" ]]; then
	echo "The EGM2008 grid us_nga_egm08_25.tif was not found in the PROJ search paths." >&2
	echo "Set GEOID_GRID=/absolute/path/to/us_nga_egm08_25.tif and rerun." >&2
	exit 1
fi

HMA_INFO="$(gdalinfo -json "$HMA")"
HMA_NODATA="$(jq -r '.bands[0].noDataValue // ""' <<<"$HMA_INFO")"
if ! jq -e '.bands[0].noDataValue == -9999' >/dev/null <<<"$HMA_INFO"; then
	echo "Expected HMA NoData=-9999, found: ${HMA_NODATA:-unset}" >&2
	exit 1
fi

RESOLUTION="$(jq -r '.geoTransform[1]' <<<"$HMA_INFO")"
mkdir -p "$OUTDIR"

GLO_VRT="$OUTDIR/glo30_himachal.vrt"
HMA_ALIGNED="$OUTDIR/hma_aligned.vrt"
GLO_ALIGNED="$OUTDIR/glo30_aligned.vrt"
GEOID_ALIGNED="$OUTDIR/egm2008_geoid_aligned.vrt"

echo "==> Building Copernicus GLO-30 mosaic from ${#COPERNICUS_TILES[@]} tiles..."
gdalbuildvrt -overwrite "$GLO_VRT" "${COPERNICUS_TILES[@]}"

COMMON_WARP_OPTIONS=(
	-overwrite
	-of VRT
	-t_srs EPSG:3857
	-te_srs EPSG:4326
	-te "$WEST" "$SOUTH" "$EAST" "$NORTH"
	-tr "$RESOLUTION" "$RESOLUTION"
	-tap
	-r bilinear
	-dstnodata -9999
)

echo "==> Aligning HMA, Copernicus, and EGM2008 to one ${RESOLUTION} m grid..."
gdalwarp "${COMMON_WARP_OPTIONS[@]}" -srcnodata -9999 "$HMA" "$HMA_ALIGNED"
gdalwarp "${COMMON_WARP_OPTIONS[@]}" "$GLO_VRT" "$GLO_ALIGNED"
gdalwarp "${COMMON_WARP_OPTIONS[@]}" "$GEOID_GRID" "$GEOID_ALIGNED"

echo "==> Verifying exact grid agreement..."
REFERENCE_GRID="$(gdalinfo -json "$HMA_ALIGNED" | jq -c '{size, geoTransform}')"
for aligned in "$GLO_ALIGNED" "$GEOID_ALIGNED"; do
	GRID="$(gdalinfo -json "$aligned" | jq -c '{size, geoTransform}')"
	if [[ "$GRID" != "$REFERENCE_GRID" ]]; then
		echo "Grid mismatch: $aligned" >&2
		exit 1
	fi
done

echo "==> Correcting HMA to EGM2008 and filling its voids from Copernicus..."
gdal_calc.py \
	-A "$HMA_ALIGNED" \
	-B "$GEOID_ALIGNED" \
	-C "$GLO_ALIGNED" \
	--calc='numpy.where(A <= -9990, C, A - B)' \
	--hideNoData \
	--projectionCheck \
	--type=Float32 \
	--NoDataValue=-9999 \
	--format=GTiff \
	--co COMPRESS=DEFLATE \
	--co PREDICTOR=3 \
	--co TILED=YES \
	--co BLOCKXSIZE=512 \
	--co BLOCKYSIZE=512 \
	--co BIGTIFF=YES \
	--co NUM_THREADS=ALL_CPUS \
	--outfile="$OUTPUT"

PREVIEW_DEM="$OUTDIR/fused_preview_dem.tif"
PREVIEW_HILLSHADE="$OUTDIR/fused_preview_hillshade.tif"
PREVIEW_PNG="$OUTDIR/fused_preview.png"

echo "==> Creating a lightweight multidirectional hillshade preview..."
gdal_translate \
	-r average \
	-outsize 2000 0 \
	-co COMPRESS=DEFLATE \
	-co TILED=YES \
	"$OUTPUT" "$PREVIEW_DEM"
gdaldem hillshade \
	-multidirectional \
	-compute_edges \
	-z 1.0 \
	-co COMPRESS=DEFLATE \
	-co TILED=YES \
	"$PREVIEW_DEM" "$PREVIEW_HILLSHADE"
gdal_translate -of PNG "$PREVIEW_HILLSHADE" "$PREVIEW_PNG"

echo ""
echo "==> Fusion complete"
echo "    DEM:     $OUTPUT"
echo "    Preview: $PREVIEW_PNG"
echo ""
echo "Next, generate a new Terrain-RGB file without overwriting the current map:"
echo "  MIN_ZOOM=6 MAX_ZOOM=13 WORKERS=6 $ROOT/scripts/dem_to_terrain_rgb.sh \\"
echo "    $ROOT/data/terrain_rgb_hma.mbtiles $OUTPUT"
