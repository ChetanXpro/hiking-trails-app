#!/bin/sh
set -eu

: "${RESTORE_DUMP:?RESTORE_DUMP is required}"

archive="/data/${RESTORE_DUMP}"
case "/${RESTORE_DUMP}/" in
	*/../*|*/./*|//*)
		echo "RESTORE_DUMP must be a non-empty relative path without . or .. components" >&2
		exit 2
		;;
esac
[ "${RESTORE_DUMP}" != "${RESTORE_DUMP#/}" ] && {
	echo "RESTORE_DUMP must be relative to /data" >&2
	exit 2
}

parent=${RESTORE_DUMP%/*}
[ "${parent}" != "${RESTORE_DUMP}" ] || parent=
current=/data
while [ -n "${parent}" ]; do
	component=${parent%%/*}
	case "${parent}" in
		*/*) parent=${parent#*/} ;;
		*) parent= ;;
	esac
	current="${current}/${component}"
	if [ -L "${current}" ] || [ ! -d "${current}" ]; then
		echo "Restore path contains a symlink or non-directory component: ${current}" >&2
		exit 1
	fi
done

if [ ! -f "${archive}" ] || [ -L "${archive}" ]; then
	echo "Restore archive must be a regular, non-symlink file: ${archive}" >&2
	exit 1
fi

toc=$(mktemp /tmp/hiking-restore-toc.XXXXXX)
trap 'rm -f "${toc}"' EXIT
pg_restore --list "${archive}" >"${toc}"

# A public-only dump is part of the release contract. Silently filtering a
# broader archive would hide an incorrect backup and can produce an incomplete
# restore, so reject it before changing any rows.
non_public_schema=$(awk '
    /[[:space:]]TABLE DATA[[:space:]]/ {
        line=$0
        sub(/^.*[[:space:]]TABLE DATA[[:space:]]+/, "", line)
        schema=line
        sub(/[[:space:]].*$/, "", schema)
        if (schema != "public") {
            print schema
            exit
        }
    }
' "${toc}")
if [ -n "${non_public_schema}" ]; then
	echo "Restore archive contains non-public table data (${non_public_schema})" >&2
	exit 1
fi

# These rows belong to the freshly migrated schema and must never be present in
# a data-only application archive. Refusing them catches accidental full dumps
# before the first row is restored.
if grep -Eq '[[:space:]]TABLE DATA public (_sqlx_migrations|spatial_ref_sys)[[:space:]]' "${toc}"; then
	echo "Restore archive contains migration or extension-owned table data" >&2
	exit 1
fi

pg_restore \
	--exit-on-error \
	--single-transaction \
	--no-owner \
	--no-acl \
	--data-only \
	--schema=public \
	--dbname="${PGDATABASE}" \
	"${archive}"

psql --set ON_ERROR_STOP=1 <<'SQL'
SELECT format('ANALYZE %I.%I', schemaname, tablename)
FROM pg_tables
WHERE schemaname = 'public'
  AND tableowner = current_user
ORDER BY tablename
\gexec
SQL
echo "Restore and ANALYZE completed successfully"
