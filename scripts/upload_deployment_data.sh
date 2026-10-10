#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
# shellcheck source=scripts/lib/deployment_paths.sh
source "${SCRIPT_DIR}/lib/deployment_paths.sh"

usage() {
	cat <<'EOF'
Upload a generated deployment manifest and its assets to Cloudflare R2.

Usage:
  upload_deployment_data.sh --manifest FILE --source-dir DIR \
    --rclone-remote REMOTE[:BUCKET/PREFIX] [--manifest-object KEY]

Credentials stay in rclone configuration or environment. Assets upload first;
the manifest uploads last, so readers never see a release too early.
Use immutable/versioned object keys in the manifest.
EOF
}

MANIFEST=""
SOURCE_DIR="./data"
RCLONE_REMOTE=""
MANIFEST_OBJECT=""

while (($#)); do
	case "$1" in
		--manifest) MANIFEST=${2:?}; shift 2 ;;
		--source-dir) SOURCE_DIR=${2:?}; shift 2 ;;
		--rclone-remote) RCLONE_REMOTE=${2:?}; shift 2 ;;
		--manifest-object) MANIFEST_OBJECT=${2:?}; shift 2 ;;
		-h|--help) usage; exit 0 ;;
		*) echo "Unknown argument: $1" >&2; usage >&2; exit 2 ;;
	esac
done

[[ -f "${MANIFEST}" && ! -L "${MANIFEST}" ]] || {
	echo "--manifest must name a regular, non-symlink file" >&2
	exit 2
}
[[ -n "${RCLONE_REMOTE}" ]] || { echo "--rclone-remote is required" >&2; exit 2; }
command -v rclone >/dev/null 2>&1 || { echo "rclone is required" >&2; exit 1; }

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

upload_one() {
	local source=$1 object_key=$2
	# --immutable skips an identical object and refuses to modify an existing
	# versioned key whose content differs.
	rclone copyto "${source}" "${RCLONE_REMOTE%/}/${object_key}" \
		--immutable --checksum --retries 5 --low-level-retries 10
}

asset_count=0
while IFS=$'\t' read -r group expected_sha expected_bytes destination object_key extra; do
	[[ -z "${group}" || "${group}" == \#* ]] && continue
	[[ -z "${extra:-}" && -n "${object_key:-}" ]] || { echo "Malformed manifest row" >&2; exit 1; }
	[[ "${group}" == runtime || "${group}" == import ]] || { echo "Invalid group: ${group}" >&2; exit 1; }
	[[ "${expected_sha}" =~ ^[[:xdigit:]]{64}$ ]] || { echo "Invalid SHA256 for ${destination}" >&2; exit 1; }
	[[ "${expected_bytes}" =~ ^[0-9]+$ ]] || { echo "Invalid byte size for ${destination}" >&2; exit 1; }
	validate_deployment_relative_path "${destination}" destination
	validate_deployment_relative_path "${object_key}" object_key
	[[ "${object_key}" =~ ^[A-Za-z0-9._~/-]+$ ]] || { echo "object_key is not URL-safe: ${object_key}" >&2; exit 1; }
	expected_sha=$(printf '%s' "${expected_sha}" | tr '[:upper:]' '[:lower:]')
	source_file=$(deployment_existing_file "${SOURCE_DIR}" "${destination}" asset)
	actual_bytes=$(wc -c <"${source_file}" | tr -d '[:space:]')
	actual_sha=$(sha256_file "${source_file}")
	[[ "${actual_bytes}" == "${expected_bytes}" ]] || {
		echo "Size mismatch for ${source_file}" >&2
		exit 1
	}
	[[ "${actual_sha}" == "${expected_sha}" ]] || {
		echo "SHA256 mismatch for ${source_file}" >&2
		exit 1
	}
	echo "Uploading ${destination} -> ${object_key}"
	upload_one "${source_file}" "${object_key}"
	((asset_count += 1))
done <"${MANIFEST}"

((asset_count > 0)) || { echo "Manifest has no assets" >&2; exit 1; }

if [[ -z "${MANIFEST_OBJECT}" ]]; then
	MANIFEST_OBJECT="manifests/$(basename "${MANIFEST}")"
fi
validate_deployment_relative_path "${MANIFEST_OBJECT}" manifest_object
[[ "${MANIFEST_OBJECT}" =~ ^[A-Za-z0-9._~/-]+$ ]] || {
	echo "manifest object key is not URL-safe: ${MANIFEST_OBJECT}" >&2
	exit 1
}
echo "Publishing manifest last -> ${MANIFEST_OBJECT}"
upload_one "${MANIFEST}" "${MANIFEST_OBJECT}"
echo "Uploaded ${asset_count} assets and manifest"
