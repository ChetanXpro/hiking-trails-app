#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
# shellcheck source=scripts/lib/deployment_paths.sh
source "${SCRIPT_DIR}/lib/deployment_paths.sh"

usage() {
	cat <<'EOF'
Validate production configuration without printing secrets.

Usage:
  production_preflight.sh [--env-file FILE] [--compose-file FILE] [--config-only]

By default the runtime database dump and Terrain-RGB file are also required.
Use --config-only before those artifacts have been downloaded.
EOF
}

ENV_FILE=.env.production
COMPOSE_FILE=docker-compose.prod.yml
CONFIG_ONLY=false

while (($#)); do
	case "$1" in
		--env-file) ENV_FILE=${2:?--env-file requires a value}; shift 2 ;;
		--compose-file) COMPOSE_FILE=${2:?--compose-file requires a value}; shift 2 ;;
		--config-only) CONFIG_ONLY=true; shift ;;
		-h|--help) usage; exit 0 ;;
		*) echo "Unknown argument: $1" >&2; usage >&2; exit 2 ;;
	esac
done

[[ -f "${ENV_FILE}" && ! -L "${ENV_FILE}" ]] || {
	echo "Environment file must be a regular, non-symlink file: ${ENV_FILE}" >&2
	exit 1
}
[[ -f "${COMPOSE_FILE}" && ! -L "${COMPOSE_FILE}" ]] || {
	echo "Compose file must be a regular, non-symlink file: ${COMPOSE_FILE}" >&2
	exit 1
}

dotenv_get() {
	local key=$1 value
	value=$(awk -v wanted="${key}" '
		$0 ~ "^[[:space:]]*" wanted "=" {
			line=$0
			sub("^[[:space:]]*" wanted "=", "", line)
			gsub(/\r$/, "", line)
			print line
			exit
		}
	' "${ENV_FILE}")
	if [[ "${value}" == \"*\" && "${value}" == *\" ]]; then
		value=${value:1:${#value}-2}
	elif [[ "${value}" == \'*\' && "${value}" == *\' ]]; then
		value=${value:1:${#value}-2}
	fi
	printf '%s' "${value}"
}

duplicate_keys=$(awk '
	/^[[:space:]]*[A-Za-z_][A-Za-z0-9_]*=/ {
		key=$0
		sub(/^[[:space:]]*/, "", key)
		sub(/=.*/, "", key)
		seen[key]++
	}
	END {
		for (key in seen) {
			if (seen[key] > 1) print key
		}
	}
' "${ENV_FILE}" | sort)
[[ -z "${duplicate_keys}" ]] || {
	echo "Environment file contains duplicate keys:" >&2
	printf '%s\n' "${duplicate_keys}" >&2
	exit 1
}

require_value() {
	local key=$1 value
	value=$(dotenv_get "${key}")
	[[ -n "${value}" ]] || { echo "${key} must be set" >&2; exit 1; }
	printf '%s' "${value}"
}

require_uint_between() {
	local key=$1 minimum=$2 maximum=$3 value number
	value=$(require_value "${key}")
	[[ "${value}" =~ ^[0-9]+$ ]] || {
		echo "${key} must be an unsigned integer" >&2
		exit 1
	}
	number=$((10#${value}))
	((number >= minimum && number <= maximum)) || {
		echo "${key} must be between ${minimum} and ${maximum}" >&2
		exit 1
	}
	printf '%s' "${number}"
}

is_https_public_url() {
	local value=$1
	[[ "${value}" =~ ^https://[^/?#[:space:]]+([/?#][^[:space:]]*)?$ ]]
}

is_secure_public_url() {
	local value=$1
	is_https_public_url "${value}" ||
		[[ "${value}" == /* && "${value}" != //* && "${value}" != *[[:space:]]* ]]
}

permissions=""
if stat -c '%a' "${ENV_FILE}" >/dev/null 2>&1; then
	permissions=$(stat -c '%a' "${ENV_FILE}")
elif stat -f '%Lp' "${ENV_FILE}" >/dev/null 2>&1; then
	permissions=$(stat -f '%Lp' "${ENV_FILE}")
fi
if [[ -n "${permissions}" && "${permissions: -2}" != "00" ]]; then
	echo "${ENV_FILE} must not be readable or writable by group/other (run chmod 600)" >&2
	exit 1
fi

domain=$(require_value DOMAIN)
origin=$(require_value APP_ORIGIN)
image_prefix=$(require_value IMAGE_PREFIX)
app_version=$(require_value APP_VERSION)
postgis_image_version=$(require_value POSTGIS_IMAGE_VERSION)
data_release=$(require_value DATA_RELEASE)
pgdata_volume=$(require_value PGDATA_VOLUME_NAME)
deploy_platform=$(require_value DEPLOY_PLATFORM)

[[ "${domain}" != *example.com* && "${domain}" != localhost* ]] || {
	echo "DOMAIN still contains an example/local placeholder" >&2
	exit 1
}
[[ "${domain}" =~ ^[A-Za-z0-9]([A-Za-z0-9.-]*[A-Za-z0-9])?$ && "${domain}" != *..* ]] || {
	echo "DOMAIN must be a plain hostname without a scheme, path, or port" >&2
	exit 1
}
[[ "${origin}" == "https://${domain}" ]] || {
	echo "APP_ORIGIN must be exactly https://DOMAIN" >&2
	exit 1
}
[[ "${image_prefix}" == */* && "${image_prefix}" != hiking ]] || {
	echo "IMAGE_PREFIX must be an explicit registry/namespace prefix" >&2
	exit 1
}
[[ "${image_prefix}" =~ ^[A-Za-z0-9._:-]+(/[A-Za-z0-9._-]+)+$ ]] || {
	echo "IMAGE_PREFIX contains characters that are not valid in an image repository" >&2
	exit 1
}
case "${app_version}" in local|latest|prototype-1|*REPLACE*|*replace*)
	echo "APP_VERSION must be an immutable release identifier, preferably a Git SHA" >&2
	exit 1
	;;
esac
[[ "${app_version}" =~ ^[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}$ ]] || {
	echo "APP_VERSION must be a valid immutable container tag" >&2
	exit 1
}
case "${postgis_image_version}" in local|latest|*REPLACE*|*replace*)
	echo "POSTGIS_IMAGE_VERSION must be an immutable database image identifier" >&2
	exit 1
	;;
esac
[[ "${postgis_image_version}" =~ ^[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}$ ]] || {
	echo "POSTGIS_IMAGE_VERSION must be a valid immutable container tag" >&2
	exit 1
}
case "${data_release}" in local|latest|prototype-1|*REPLACE*|*replace*)
	echo "DATA_RELEASE must be an immutable data release identifier" >&2
	exit 1
	;;
esac
[[ "${data_release}" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$ ]] || {
	echo "DATA_RELEASE must be a URL-safe release identifier" >&2
	exit 1
}
[[ "${pgdata_volume}" != *example* && "${pgdata_volume}" != *REPLACE* &&
	"${pgdata_volume}" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]+$ ]] || {
	echo "PGDATA_VOLUME_NAME still contains a placeholder" >&2
	exit 1
}
[[ "${deploy_platform}" == linux/amd64 || "${deploy_platform}" == linux/arm64 ]] || {
	echo "DEPLOY_PLATFORM must be linux/amd64 or linux/arm64" >&2
	exit 1
}

for key in POSTGRES_PASSWORD API_DB_PASSWORD IMPORT_DB_PASSWORD MARTIN_DB_PASSWORD; do
	secret=$(require_value "${key}")
	[[ ${#secret} -ge 32 && "${secret}" =~ ^[A-Za-z0-9_-]+$ && "${secret}" != *replace* ]] || {
		echo "${key} must be at least 32 URL-safe random characters and not a placeholder" >&2
		exit 1
	}
done

for key in DEPLOY_UID DEPLOY_GID; do
	identity=$(require_value "${key}")
	[[ "${identity}" =~ ^[0-9]+$ && "${identity}" != 0 ]] || {
		echo "${key} must be a non-zero numeric host identity" >&2
		exit 1
	}
done

prune_missing=$(dotenv_get IMPORT_PRUNE_MISSING_TRAILS)
[[ -z "${prune_missing}" || "${prune_missing}" == true || "${prune_missing}" == false ]] || {
	echo "IMPORT_PRUNE_MISSING_TRAILS must be true or false" >&2
	exit 1
}

require_uint_between API_WORKERS 1 64 >/dev/null
require_uint_between API_MAX_CONNECTIONS 1 4096 >/dev/null
require_uint_between API_MAX_IN_FLIGHT_REQUESTS 1 4096 >/dev/null
require_uint_between API_READINESS_TIMEOUT_MS 1 5000 >/dev/null
require_uint_between API_EXPORT_MAX_POINTS 1 100000 >/dev/null
require_uint_between API_EXPORT_MAX_BODY_BYTES 1 8388608 >/dev/null
shutdown_seconds=$(require_uint_between API_SHUTDOWN_TIMEOUT_SECONDS 1 3600)
stop_grace_seconds=$(require_uint_between API_STOP_GRACE_PERIOD_SECONDS 1 3600)
((stop_grace_seconds > shutdown_seconds)) || {
	echo "API_STOP_GRACE_PERIOD_SECONDS must exceed API_SHUTDOWN_TIMEOUT_SECONDS" >&2
	exit 1
}

for key in PUBLIC_BASEMAP_STYLE_URL PUBLIC_MAP_GLYPHS_URL; do
	url=$(require_value "${key}")
	[[ "${url}" != *example.com* ]] && is_https_public_url "${url}" || {
		echo "${key} must be a non-placeholder HTTPS URL" >&2
		exit 1
	}
done

for key in PUBLIC_MARTIN_TRAILS_URL PUBLIC_MARTIN_OUTDOOR_WAYPOINTS_URL PUBLIC_TERRAIN_RGB_URL; do
	url=$(dotenv_get "${key}")
	# Blank selects Compose's same-origin default, which embeds DATA_RELEASE.
	[[ -n "${url}" ]] || continue
	is_secure_public_url "${url}" || {
		echo "${key} must be HTTPS or a same-origin absolute path" >&2
		exit 1
	}
	[[ "${url}" == *"${data_release}"* ]] || {
		echo "${key} must contain DATA_RELEASE so cache keys change with data" >&2
		exit 1
	}
done

for key in PUBLIC_RASTER_TILE_URL PUBLIC_WEATHER_API_URL; do
	url=$(dotenv_get "${key}")
	if [[ -n "${url}" ]] && ! is_secure_public_url "${url}"; then
		echo "${key} must be empty, HTTPS, or a same-origin absolute path" >&2
		exit 1
	fi
done

weather_url=$(dotenv_get PUBLIC_WEATHER_API_URL)
if [[ -n "${weather_url}" && "${weather_url}" =~ ([?&](api_?key|token|secret)=) ]]; then
	echo "Warning: PUBLIC_WEATHER_API_URL appears to contain a browser-visible credential" >&2
fi

command -v docker >/dev/null 2>&1 || { echo "docker is required" >&2; exit 1; }
docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" config --quiet

if ! ${CONFIG_ONLY}; then
	data_dir=$(require_value DATA_DIR)
	restore_dump=$(require_value RESTORE_DUMP)
	deployment_existing_file "${data_dir}" "${restore_dump}" \
		"RESTORE_DUMP" >/dev/null
	deployment_existing_file "${data_dir}" "terrain_rgb_hma.mbtiles" \
		"Terrain-RGB artifact" >/dev/null
fi

echo "Production preflight passed"
