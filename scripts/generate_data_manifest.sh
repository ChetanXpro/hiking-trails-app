#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
# shellcheck source=scripts/lib/deployment_paths.sh
source "${SCRIPT_DIR}/lib/deployment_paths.sh"

usage() {
	cat <<'EOF'
Generate a checksummed deployment-data manifest from an existing data layout.

Usage:
  generate_data_manifest.sh --output FILE [--source-dir DIR] \
    --asset GROUP DESTINATION OBJECT_KEY [--asset ...]

Example:
  ./scripts/generate_data_manifest.sh --source-dir ./data \
    --output ./deploy/data-manifest.tsv \
    --asset runtime terrain_rgb_hma.mbtiles releases/v1/terrain_rgb_hma.mbtiles \
    --asset runtime bootstrap/hiking-data.dump releases/v1/hiking-data.dump

GROUP must be runtime or import. DESTINATION is both the path below SOURCE_DIR
and the path the downloader recreates. OBJECT_KEY is the immutable R2 key.
EOF
}

SOURCE_DIR="./data"
OUTPUT=""
ASSETS=()

while (($#)); do
	case "$1" in
		--source-dir)
			SOURCE_DIR=${2:?--source-dir requires a value}
			shift 2
			;;
		--output)
			OUTPUT=${2:?--output requires a value}
			shift 2
			;;
		--asset)
			(($# >= 4)) || { echo "--asset requires GROUP DESTINATION OBJECT_KEY" >&2; exit 2; }
			ASSETS+=("$2" "$3" "$4")
			shift 4
			;;
		-h|--help)
			usage
			exit 0
			;;
		*)
			echo "Unknown argument: $1" >&2
			usage >&2
			exit 2
			;;
	esac
done

[[ -n "${OUTPUT}" ]] || { echo "--output is required" >&2; exit 2; }
((${#ASSETS[@]} > 0)) || { echo "At least one --asset is required" >&2; exit 2; }

sha256_file() {
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum "$1" | awk '{print $1}'
	elif command -v shasum >/dev/null 2>&1; then
		shasum -a 256 "$1" | awk '{print $1}'
	else
		echo "sha256sum or shasum is required" >&2
		return 1
	fi
}

mkdir -p "$(dirname "${OUTPUT}")"
[[ ! -L "${OUTPUT}" ]] || { echo "Manifest output may not be a symlink: ${OUTPUT}" >&2; exit 1; }
TEMP_OUTPUT=$(mktemp "${OUTPUT}.tmp.XXXXXX")
trap 'rm -f "${TEMP_OUTPUT}"' EXIT

{
	printf '# hiking-data-manifest-v1\n'
	printf '# group\tsha256\tbytes\tdestination\tobject_key\n'

	for ((i = 0; i < ${#ASSETS[@]}; i += 3)); do
		group=${ASSETS[i]}
		destination=${ASSETS[i + 1]}
		object_key=${ASSETS[i + 2]}

		[[ "${group}" == runtime || "${group}" == import ]] || {
			echo "Invalid group '${group}'; expected runtime or import" >&2
			exit 2
		}
		validate_deployment_relative_path "${destination}" destination
		validate_deployment_relative_path "${object_key}" object_key
		[[ "${object_key}" =~ ^[A-Za-z0-9._~/-]+$ ]] || {
			echo "object_key must contain only URL-safe path characters: ${object_key}" >&2
			exit 2
		}

		source_file=$(deployment_existing_file "${SOURCE_DIR}" "${destination}" asset)
		bytes=$(wc -c <"${source_file}" | tr -d '[:space:]')
		sha256=$(sha256_file "${source_file}")
		printf '%s\t%s\t%s\t%s\t%s\n' \
			"${group}" "${sha256}" "${bytes}" "${destination}" "${object_key}"
	done
} >"${TEMP_OUTPUT}"

mv -f "${TEMP_OUTPUT}" "${OUTPUT}"
trap - EXIT
echo "Wrote ${OUTPUT} ($((${#ASSETS[@]} / 3)) assets)"
