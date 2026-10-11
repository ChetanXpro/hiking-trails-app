#!/usr/bin/env bash
set -Eeuo pipefail

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

for value_name in SOUTH WEST NORTH EAST; do
	value=${!value_name}
	if [[ ! "${value}" =~ ^-?[0-9]+$ ]]; then
		lower_name=$(printf '%s' "${value_name}" | tr '[:upper:]' '[:lower:]')
		echo "${lower_name} must be an integer degree, got: ${value}" >&2
		exit 2
	fi
done
if ((SOUTH >= NORTH || WEST >= EAST)); then
	echo "Expected south < north and west < east" >&2
	exit 2
fi
if ((SOUTH < -90 || NORTH > 90 || WEST < -180 || EAST > 180)); then
	echo "Bounds must stay within latitude -90..90 and longitude -180..180" >&2
	exit 2
fi
for command in curl gdalinfo; do
	command -v "${command}" >/dev/null 2>&1 || {
		echo "${command} is required" >&2
		exit 1
	}
done

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
		partial="${dest}.part"

		if [[ -f "${dest}" ]]; then
			if gdalinfo -json "${dest}" >/dev/null 2>&1; then
				echo "  already have valid ${tile_name}, skipping"
				continue
			fi
			echo "  existing ${tile_name} is not a readable raster; replacing it" >&2
			rm -f "${dest}"
		fi

		set +e
		status=$(curl \
			--silent --show-error --location --fail \
			--retry 5 --retry-max-time 180 \
			--connect-timeout 15 --continue-at - \
			--output "${partial}" --write-out '%{http_code}' \
			"${url}")
		curl_status=$?
		set -e

		# A fully downloaded .part may produce HTTP 416 when curl tries to
		# resume it. Validation is authoritative, so accept it if GDAL can
		# open the raster.
		if [[ -f "${partial}" ]] && gdalinfo -json "${partial}" >/dev/null 2>&1; then
			mv -f "${partial}" "${dest}"
			size="$(du -h "${dest}" | cut -f1)"
			echo "  fetched ${tile_name} (${size})"
			FETCHED=$((FETCHED + 1))
		elif [[ "${status}" == "404" ]]; then
			rm -f "${partial}"
			echo "  no public tile for ${tile_name} (HTTP ${status}) -- skipping"
			SKIPPED=$((SKIPPED + 1))
		else
			echo "  failed ${tile_name} (curl ${curl_status}, HTTP ${status:-unknown}); keeping ${partial} for resume" >&2
			exit 1
		fi
	done
done

echo ""
echo "==> Done: ${FETCHED} tile(s) downloaded, ${SKIPPED} cell(s) skipped, into ${OUTDIR}/"
echo "    Next: ./scripts/dem_to_terrain_rgb.sh ./data/terrain_rgb.mbtiles ${OUTDIR}/*.tif"
