#!/bin/sh
set -eu

# Martin treats a directory argument as a general geodata discovery root. The
# shared data directory also contains JSON manifests, database dumps, and raw
# import inputs, none of which should become public tile sources. Pass only the
# explicitly allowed tile archives that actually exist; DATABASE_URL is
# discovered from the environment by Martin itself.
archive_count=0
for archive_name in ${MARTIN_TILE_ARCHIVES:-terrain_rgb_hma.mbtiles}; do
	case "${archive_name}" in
	*.mbtiles | *.pmtiles)
		case "${archive_name}" in
		*/* | .* | *[!A-Za-z0-9._-]*)
			echo "Invalid MARTIN_TILE_ARCHIVES entry: ${archive_name}" >&2
			exit 2
			;;
		esac
		;;
	*)
		echo "MARTIN_TILE_ARCHIVES entries must end in .mbtiles or .pmtiles: ${archive_name}" >&2
		exit 2
		;;
	esac

	archive="/data/${archive_name}"
	[ -f "${archive}" ] || continue
	set -- "$@" "${archive}"
	archive_count=$((archive_count + 1))
done

echo "Martin: selected ${archive_count} allowlisted tile archive(s) from /data"
exec /usr/local/bin/martin "$@"
