#!/usr/bin/env bash
set -Eeuo pipefail

usage() {
	cat <<'EOF'
Create and validate the normalized production bootstrap archive.

Usage:
  create_bootstrap_dump.sh [--output FILE] [--compose-file FILE]
                           [--env-file FILE] [--db-service NAME]

The dump and validation run inside the database service, guaranteeing that
pg_dump/pg_restore match the PostgreSQL server major version. Raw import
staging, extension-owned data, and SQLx migration rows are excluded.
EOF
}

OUTPUT="data/bootstrap/hiking-data.dump"
COMPOSE_FILE="docker-compose.yml"
ENV_FILE=".env"
DB_SERVICE="db"

while (($#)); do
	case "$1" in
	--output) OUTPUT=${2:?--output requires a value}; shift 2 ;;
	--compose-file) COMPOSE_FILE=${2:?--compose-file requires a value}; shift 2 ;;
	--env-file) ENV_FILE=${2:?--env-file requires a value}; shift 2 ;;
	--db-service) DB_SERVICE=${2:?--db-service requires a value}; shift 2 ;;
	-h|--help) usage; exit 0 ;;
	*) echo "Unknown argument: $1" >&2; usage >&2; exit 2 ;;
	esac
done

[[ -f "${COMPOSE_FILE}" && ! -L "${COMPOSE_FILE}" ]] || {
	echo "Compose file must be a regular, non-symlink file: ${COMPOSE_FILE}" >&2
	exit 1
}
[[ -f "${ENV_FILE}" && ! -L "${ENV_FILE}" ]] || {
	echo "Environment file must be a regular, non-symlink file: ${ENV_FILE}" >&2
	exit 1
}
[[ -n "${OUTPUT}" && "${OUTPUT}" != */ ]] || {
	echo "--output must name a file" >&2
	exit 2
}
[[ "$(basename "${OUTPUT}")" != -* ]] || {
	echo "--output basename must not begin with '-'" >&2
	exit 2
}
[[ ! -L "${OUTPUT}" ]] || {
	echo "Refusing to replace symlink output: ${OUTPUT}" >&2
	exit 1
}
[[ ! -e "${OUTPUT}" || -f "${OUTPUT}" ]] || {
	echo "Refusing to replace non-regular output: ${OUTPUT}" >&2
	exit 1
}
[[ "${DB_SERVICE}" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]*$ ]] || {
	echo "--db-service contains invalid characters" >&2
	exit 2
}

command -v docker >/dev/null 2>&1 || { echo "docker is required" >&2; exit 1; }
docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" config --quiet

mkdir -p "$(dirname "${OUTPUT}")"
umask 077
PARTIAL=$(mktemp "${OUTPUT}.partial.XXXXXX")
trap 'rm -f -- "${PARTIAL}"' EXIT

echo "Creating PostgreSQL-version-compatible bootstrap archive"
docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" \
	exec -T "${DB_SERVICE}" sh -c '
		exec pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
			--format=custom --data-only --schema=public --no-owner --no-acl \
			--exclude-table=_sqlx_migrations --exclude-table=spatial_ref_sys \
			--exclude-table="raw_*" --exclude-table="planet_osm_*" \
			--exclude-table=osm2pgsql_properties
	' >"${PARTIAL}"

[[ -s "${PARTIAL}" ]] || {
	echo "pg_dump produced an empty archive" >&2
	exit 1
}

docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" \
	exec -T "${DB_SERVICE}" pg_restore --list <"${PARTIAL}" >/dev/null
# `--list` checks only the archive header/TOC. Rendering the complete SQL
# stream to /dev/null forces pg_restore to decompress and checksum every data
# block, catching truncation before the archive receives its final name.
docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" \
	exec -T "${DB_SERVICE}" pg_restore --exit-on-error --file=/dev/null \
	<"${PARTIAL}"

mv -f -- "${PARTIAL}" "${OUTPUT}"
trap - EXIT
echo "Verified bootstrap archive: ${OUTPUT}"
