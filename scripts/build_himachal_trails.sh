#!/usr/bin/env bash
set -euo pipefail

# Build a Himachal Pradesh trail extract directly from india-latest.osm.pbf.
#
# This avoids needing a separately downloaded .poly file. The script:
# 1. pulls the Himachal administrative boundary relation from the India extract
# 2. exports that boundary as GeoJSON
# 3. extracts the Himachal region
# 4. filters the result down to the OSM objects needed by this app
#
# Usage:
#   ./scripts/build_himachal_trails.sh [india_pbf] [output_dir]
#
# Defaults:
#   india_pbf  = ./india-latest.osm.pbf
#   output_dir = ./data

INPUT_PBF="${1:-./india-latest.osm.pbf}"
OUTPUT_DIR="${2:-./data}"

HIMACHAL_RELATION_ID="364186"
BOUNDARY_GEOJSON="${OUTPUT_DIR}/himachal_pradesh.geojson"
REGION_PBF="${OUTPUT_DIR}/hp.osm.pbf"
TRAILS_PBF="${OUTPUT_DIR}/hp-trails.osm.pbf"

if ! command -v osmium >/dev/null 2>&1; then
	echo "osmium is required. Install it first: brew install osmium-tool" >&2
	exit 1
fi

if [ ! -f "${INPUT_PBF}" ]; then
	echo "Input extract not found: ${INPUT_PBF}" >&2
	exit 1
fi

mkdir -p "${OUTPUT_DIR}"

WORKDIR=$(mktemp -d)
trap 'rm -rf "${WORKDIR}"' EXIT

BOUNDARY_OSM="${WORKDIR}/himachal-boundary.osm.pbf"

echo "==> Deriving Himachal Pradesh boundary from ${INPUT_PBF}"
osmium getid -r -t "${INPUT_PBF}" "r${HIMACHAL_RELATION_ID}" -o "${BOUNDARY_OSM}" --overwrite
osmium export "${BOUNDARY_OSM}" -o "${BOUNDARY_GEOJSON}" --overwrite

echo "==> Extracting Himachal Pradesh region"
osmium extract -p "${BOUNDARY_GEOJSON}" "${INPUT_PBF}" -o "${REGION_PBF}" --overwrite

echo "==> Filtering hiking-relevant OSM objects"
osmium tags-filter "${REGION_PBF}" \
	w/highway \
	r/route=hiking \
	nwr/tourism=alpine_hut,wilderness_hut,camp_site,viewpoint,picnic_site,information \
	nwr/amenity=shelter,drinking_water,toilets,parking,ranger_station,hospital,clinic \
	nwr/emergency=phone,first_aid_kit,defibrillator,mountain_rescue \
	nwr/highway=trailhead \
	nwr/natural=peak,spring,waterfall,cave_entrance,saddle \
	nwr/mountain_pass=yes \
	nwr/aerialway=station \
	nwr/place=city,town,village,hamlet,suburb,neighbourhood,locality,isolated_dwelling \
	r/boundary=administrative,protected_area \
	-o "${TRAILS_PBF}" --overwrite

echo ""
echo "==> Done"
echo "Boundary: ${BOUNDARY_GEOJSON}"
echo "Region:   ${REGION_PBF}"
echo "Trails:   ${TRAILS_PBF}"
echo ""
echo "Next:"
echo "  source .env"
echo "  # Use the full regional extract so hierarchy/place search can be built too."
echo "  osm2pgsql -d \"\$DATABASE_URL\" --slim --output=flex --style=osm2pgsql/trails.lua ${REGION_PBF}"
echo "  cargo run -p hiking-import"
