#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
# shellcheck source=scripts/lib/deployment_paths.sh
source "${SCRIPT_DIR}/lib/deployment_paths.sh"

usage() {
	cat <<'EOF'
Atomically download and verify deployment data from an HTTPS R2 custom domain.

Usage:
  sync_deployment_data.sh --manifest FILE \
    (--base-url HTTPS_URL | --rclone-remote REMOTE[:BUCKET/PREFIX]) \
    [--output-dir DIR] [--group runtime|import|all] [--check-only]

HTTPS downloads resume into .part files; private rclone downloads retry safely.
A destination is replaced only after both its byte size and SHA256 match the
manifest. No browser/client depends on the artifact source: the server downloads
data before startup and Martin serves it locally.
EOF
}

MANIFEST=""
BASE_URL=""
RCLONE_REMOTE=""
OUTPUT_DIR="./data"
GROUP="runtime"
CHECK_ONLY=false

while (($#)); do
	case "$1" in
		--manifest) MANIFEST=${2:?}; shift 2 ;;
		--base-url) BASE_URL=${2:?}; shift 2 ;;
		--rclone-remote) RCLONE_REMOTE=${2:?}; shift 2 ;;
		--output-dir) OUTPUT_DIR=${2:?}; shift 2 ;;
		--group) GROUP=${2:?}; shift 2 ;;
		--check-only) CHECK_ONLY=true; shift ;;
		-h|--help) usage; exit 0 ;;
		*) echo "Unknown argument: $1" >&2; usage >&2; exit 2 ;;
	esac
done

[[ -f "${MANIFEST}" && ! -L "${MANIFEST}" ]] || {
	echo "--manifest must name a regular, non-symlink file" >&2
	exit 2
}
[[ "${GROUP}" == runtime || "${GROUP}" == import || "${GROUP}" == all ]] || {
	echo "--group must be runtime, import, or all" >&2
	exit 2
}
if ! ${CHECK_ONLY}; then
	if [[ -n "${BASE_URL}" && -n "${RCLONE_REMOTE}" ]] || [[ -z "${BASE_URL}" && -z "${RCLONE_REMOTE}" ]]; then
		echo "Choose exactly one of --base-url or --rclone-remote" >&2
		exit 2
	fi
	if [[ -n "${BASE_URL}" ]]; then
		[[ "${BASE_URL}" == https://* ]] || { echo "--base-url must use HTTPS" >&2; exit 2; }
		command -v curl >/dev/null 2>&1 || { echo "curl is required" >&2; exit 1; }
	else
		command -v rclone >/dev/null 2>&1 || { echo "rclone is required" >&2; exit 1; }
	fi
fi

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

verify_file() {
	local file=$1 expected_bytes=$2 expected_sha=$3
	[[ -f "${file}" ]] || return 1
	local actual_bytes
	actual_bytes=$(wc -c <"${file}" | tr -d '[:space:]')
	[[ "${actual_bytes}" == "${expected_bytes}" ]] || return 1
	[[ "$(sha256_file "${file}")" == "${expected_sha}" ]]
}

OUTPUT_ROOT=$(deployment_root "${OUTPUT_DIR}" create)
LOCK_DIR="${OUTPUT_ROOT}/.deployment-data-sync.lock"
if ! mkdir "${LOCK_DIR}" 2>/dev/null; then
	echo "Another data sync appears to be running: ${LOCK_DIR}" >&2
	exit 1
fi
trap 'rmdir "${LOCK_DIR}" 2>/dev/null || true' EXIT

selected=0
downloaded=0
reused=0

while IFS=$'\t' read -r row_group expected_sha expected_bytes destination object_key extra; do
	[[ -z "${row_group}" || "${row_group}" == \#* ]] && continue
	[[ -z "${extra:-}" && -n "${object_key:-}" ]] || { echo "Malformed manifest row" >&2; exit 1; }
	[[ "${row_group}" == runtime || "${row_group}" == import ]] || {
		echo "Invalid manifest group: ${row_group}" >&2
		exit 1
	}
	[[ "${expected_sha}" =~ ^[[:xdigit:]]{64}$ ]] || {
		echo "Invalid SHA256 for ${destination}" >&2
		exit 1
	}
	[[ "${expected_bytes}" =~ ^[0-9]+$ ]] || {
		echo "Invalid byte size for ${destination}" >&2
		exit 1
	}
	validate_deployment_relative_path "${destination}" destination
	validate_deployment_relative_path "${object_key}" object_key
	[[ "${object_key}" =~ ^[A-Za-z0-9._~/-]+$ ]] || { echo "object_key is not URL-safe: ${object_key}" >&2; exit 1; }
	expected_sha_lower=$(printf '%s' "${expected_sha}" | tr '[:upper:]' '[:lower:]')

	if [[ "${GROUP}" != all && "${row_group}" != "${GROUP}" ]]; then
		continue
	fi
	((selected += 1))
	destination_file=$(deployment_destination_file "${OUTPUT_ROOT}" "${destination}" destination)

	if verify_file "${destination_file}" "${expected_bytes}" "${expected_sha_lower}"; then
		echo "Verified ${destination}"
		((reused += 1))
		continue
	fi
	if ${CHECK_ONLY}; then
		echo "Missing or invalid: ${destination}" >&2
		exit 1
	fi

	partial_file="${destination_file}.part"
	assert_deployment_regular_or_absent "${partial_file}" "partial download"
	set +e
	if [[ -n "${BASE_URL}" ]]; then
		url="${BASE_URL%/}/${object_key}"
		echo "Downloading ${url}"
		curl \
			--fail --location --show-error --silent \
			--proto '=https' --proto-redir '=https' \
			--retry 5 --retry-delay 2 \
			--connect-timeout 15 --continue-at - \
			--output "${partial_file}" "${url}"
		download_status=$?
	else
		remote_object="${RCLONE_REMOTE%/}/${object_key}"
		echo "Downloading private R2 object ${remote_object}"
		rclone copyto "${remote_object}" "${partial_file}" \
			--ignore-times --retries 5 --low-level-retries 10
		download_status=$?
	fi
	set -e

	if verify_file "${partial_file}" "${expected_bytes}" "${expected_sha_lower}"; then
		mv -f "${partial_file}" "${destination_file}"
		((downloaded += 1))
		continue
	fi
	if ((download_status != 0)); then
		echo "Download failed for ${destination}; keeping ${partial_file} for resume" >&2
		exit 1
	fi
	echo "Downloaded asset failed size/SHA256 validation: ${destination}" >&2
	rm -f "${partial_file}"
	exit 1
done <"${MANIFEST}"

((selected > 0)) || { echo "Manifest has no '${GROUP}' assets" >&2; exit 1; }
echo "Data sync complete: ${downloaded} downloaded, ${reused} already valid"
