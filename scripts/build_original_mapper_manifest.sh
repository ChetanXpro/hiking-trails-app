#!/usr/bin/env bash
set -euo pipefail

# Build the optional original-mapper manifest from immutable version 1 of each
# OSM hiking route relation. This script is deliberately separate from the
# importer: normal data imports never contact OSM or collect user metadata.
#
# The public OSM editing API is not a bulk download service. This tool therefore
# runs as a dry-run by default, fetches small multi-object batches serially,
# requires an identifying contact, waits between requests, and caches version-1
# responses forever. For a large/global backfill, use an authorised regional
# history extract instead of this script.

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "${ROOT_DIR}"

if [[ -f .env ]]; then
	set -a
	# shellcheck disable=SC1091
	source .env
	set +a
fi

CACHE_DIR="${ROOT_DIR}/data/osm_relation_v1_cache"
OUTPUT_PATH="${ROOT_DIR}/data/original_mappers.json"
LIMIT=25
BATCH_SIZE=25
DELAY_SECONDS=1.25
CONTACT="${OSM_ATTRIBUTION_CONTACT:-}"
FETCH=false

usage() {
	cat <<'EOF'
Usage: ./scripts/build_original_mapper_manifest.sh [options]

Default mode is a read-only dry run. It reports cache hits and the relation IDs
that the next bounded fetch would request.

Options:
  --fetch              Perform network requests and write the manifest
  --contact VALUE      Contact email or URL included in the User-Agent
  --limit N            Maximum uncached relations fetched this run (default 25)
  --batch-size N       Relations per serial API request, 1-100 (default 25)
  --delay SECONDS      Delay after each request; minimum 1.0 (default 1.25)
  --cache-dir PATH     Immutable relation-version-1 response cache
  --output PATH        Generated JSON manifest path
  -h, --help           Show this help

Environment:
  DATABASE_URL             Required; loaded from .env when present
  OSM_ATTRIBUTION_CONTACT  Alternative to --contact
EOF
}

while [[ $# -gt 0 ]]; do
	case "$1" in
	--fetch)
		FETCH=true
		shift
		;;
	--contact)
		CONTACT="${2:?--contact requires a value}"
		shift 2
		;;
	--limit)
		LIMIT="${2:?--limit requires a value}"
		shift 2
		;;
	--batch-size)
		BATCH_SIZE="${2:?--batch-size requires a value}"
		shift 2
		;;
	--delay)
		DELAY_SECONDS="${2:?--delay requires a value}"
		shift 2
		;;
	--cache-dir)
		CACHE_DIR="${2:?--cache-dir requires a value}"
		shift 2
		;;
	--output)
		OUTPUT_PATH="${2:?--output requires a value}"
		shift 2
		;;
	-h | --help)
		usage
		exit 0
		;;
	*)
		echo "Unknown option: $1" >&2
		usage >&2
		exit 2
		;;
	esac
done

for command in psql curl jq; do
	if ! command -v "${command}" >/dev/null 2>&1; then
		echo "${command} is required" >&2
		exit 1
	fi
done

: "${DATABASE_URL:?DATABASE_URL must be set in .env or the environment}"

if [[ ! "${LIMIT}" =~ ^[0-9]+$ ]] || [[ "${LIMIT}" -lt 1 ]]; then
	echo "--limit must be a positive integer" >&2
	exit 2
fi

if [[ ! "${BATCH_SIZE}" =~ ^[0-9]+$ ]] ||
	[[ "${BATCH_SIZE}" -lt 1 ]] ||
	[[ "${BATCH_SIZE}" -gt 100 ]]; then
	echo "--batch-size must be an integer from 1 to 100" >&2
	exit 2
fi

if ! awk -v delay="${DELAY_SECONDS}" 'BEGIN { exit !(delay + 0 >= 1.0) }'; then
	echo "--delay must be a number greater than or equal to 1.0 seconds" >&2
	exit 2
fi

RELATION_IDS=()
relation_ids_output=$(PGDATABASE="${DATABASE_URL}" psql -Atc \
	"SELECT osm_relation_id FROM trails ORDER BY osm_relation_id")
while IFS= read -r relation_id; do
	[[ -n "${relation_id}" ]] && RELATION_IDS+=("${relation_id}")
done <<<"${relation_ids_output}"

if [[ ${#RELATION_IDS[@]} -eq 0 ]]; then
	echo "No trails with OSM relation IDs were found."
	exit 0
fi

CACHE_HITS=0
MISSING_IDS=()
for relation_id in "${RELATION_IDS[@]}"; do
	if [[ -s "${CACHE_DIR}/${relation_id}.json" ]] ||
		[[ -s "${CACHE_DIR}/${relation_id}.unavailable" ]]; then
		CACHE_HITS=$((CACHE_HITS + 1))
	else
		MISSING_IDS+=("${relation_id}")
	fi
done

echo "Relations in database: ${#RELATION_IDS[@]}"
echo "Cached or known unavailable: ${CACHE_HITS}"
echo "Not cached: ${#MISSING_IDS[@]}"

if [[ "${FETCH}" != true ]]; then
	echo "Dry run only; no network requests or files were written."
	if [[ ${#MISSING_IDS[@]} -gt 0 ]]; then
		echo "Next batch (up to ${LIMIT}):"
		for relation_id in "${MISSING_IDS[@]:0:${LIMIT}}"; do
			echo "  relation ${relation_id} version 1"
		done
	fi
	exit 0
fi

if [[ -z "${CONTACT// }" ]]; then
	echo "--contact or OSM_ATTRIBUTION_CONTACT is required with --fetch" >&2
	exit 2
fi

mkdir -p "${CACHE_DIR}" "$(dirname "${OUTPUT_PATH}")"
USER_AGENT="HikingTrailsOriginalMapperFetcher/1.0 (contact: ${CONTACT})"
SELECTED_IDS=()
for relation_id in "${MISSING_IDS[@]}"; do
	[[ ${#SELECTED_IDS[@]} -ge ${LIMIT} ]] && break
	SELECTED_IDS+=("${relation_id}")
done

NETWORK_REQUESTS=0
FETCHED=0
FETCH_FAILED=false
offset=0
while [[ ${offset} -lt ${#SELECTED_IDS[@]} ]]; do
	batch_ids=()
	index=${offset}
	while [[ ${index} -lt ${#SELECTED_IDS[@]} && ${#batch_ids[@]} -lt ${BATCH_SIZE} ]]; do
		batch_ids+=("${SELECTED_IDS[${index}]}")
		index=$((index + 1))
	done

	relations_param=""
	for relation_id in "${batch_ids[@]}"; do
		[[ -n "${relations_param}" ]] && relations_param+=","
		relations_param+="${relation_id}v1"
	done

	url="https://api.openstreetmap.org/api/0.6/relations.json?relations=${relations_param}"
	batch_tmp="${CACHE_DIR}/batch-$$-${offset}.json.tmp"
	echo "Fetching ${#batch_ids[@]} relation version-1 records (serial batch)"

	set +e
	http_code=$(curl -sS \
		--connect-timeout 10 \
		--max-time 60 \
		--retry 4 \
		--retry-max-time 180 \
		--fail-with-body \
		--user-agent "${USER_AGENT}" \
		--header "Accept: application/json" \
		--output "${batch_tmp}" \
		--write-out '%{http_code}' \
		"${url}")
	curl_status=$?
	set -e

	NETWORK_REQUESTS=$((NETWORK_REQUESTS + 1))

	if [[ "${curl_status}" -eq 0 && "${http_code}" == "200" ]]; then
		FETCHED=$((FETCHED + ${#batch_ids[@]}))
		for relation_id in "${batch_ids[@]}"; do
			tmp_path="${CACHE_DIR}/${relation_id}.json.tmp"
			if jq -e --argjson relation_id "${relation_id}" '
				[
					.elements[]
					| select(.type == "relation" and .id == $relation_id and .version == 1)
				] as $relations
				| ($relations | length) == 1
				and ($relations[0].user | type == "string" and length > 0)
				and ($relations[0].uid | type == "number")
				and ($relations[0].timestamp | type == "string" and length > 0)
			' "${batch_tmp}" >/dev/null; then
				jq --argjson relation_id "${relation_id}" '
					{
						version,
						generator,
						elements: [
							.elements[]
							| select(.type == "relation" and .id == $relation_id and .version == 1)
						]
					}
				' "${batch_tmp}" >"${tmp_path}"
				mv "${tmp_path}" "${CACHE_DIR}/${relation_id}.json"
			else
				rm -f "${tmp_path}"
				echo "OSM returned no usable version-1 author metadata for relation ${relation_id}" >&2
				printf 'author metadata unavailable\n' >"${CACHE_DIR}/${relation_id}.unavailable"
			fi
		done
		rm -f "${batch_tmp}"
	else
		rm -f "${batch_tmp}"
		echo "OSM batch request failed (curl ${curl_status}, HTTP ${http_code})" >&2
		if [[ "${http_code}" == "403" || "${http_code}" == "404" ]]; then
			echo "A requested version may be unavailable; retry this batch with --batch-size 1." >&2
		fi
		echo "Stopping this batch; cached successful responses are preserved." >&2
		FETCH_FAILED=true
		break
	fi

	sleep "${DELAY_SECONDS}"
	offset=${index}
done

if [[ "${FETCH_FAILED}" == true ]]; then
	echo "Manifest was not changed because the requested fetch was incomplete. Re-run to resume from the immutable cache." >&2
	exit 1
fi

entries_path=$(mktemp "${TMPDIR:-/tmp}/original-mappers.XXXXXX")
output_tmp="${OUTPUT_PATH}.tmp"
trap 'rm -f "${entries_path}" "${output_tmp}"' EXIT

for relation_id in "${RELATION_IDS[@]}"; do
	cache_path="${CACHE_DIR}/${relation_id}.json"
	[[ -s "${cache_path}" ]] || continue

	jq -c '
		.elements[0] as $relation
		| {
			osm_relation_id: $relation.id,
			osm_relation_version: 1,
			display_name: $relation.user,
			provider_user_id: ($relation.uid | tostring),
			profile_url: (
				"https://www.openstreetmap.org/user/" + ($relation.user | @uri)
			),
			mapped_at: $relation.timestamp
		}
	' "${cache_path}" >>"${entries_path}"
done

jq -s '{schema_version: 1, relations: .}' "${entries_path}" >"${output_tmp}"
mv "${output_tmp}" "${OUTPUT_PATH}"

echo "Relation records requested this run: ${FETCHED}"
echo "Network requests this run: ${NETWORK_REQUESTS}"
echo "Manifest written: ${OUTPUT_PATH}"
echo "Set OSM_ORIGINAL_MAPPERS_PATH=${OUTPUT_PATH} before running hiking-import."
