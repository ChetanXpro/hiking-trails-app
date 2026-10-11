#!/bin/sh
set -eu

# The importer distinguishes an absent optional WorldCover variable from an
# empty-but-present one. Compose represents an optional value as an empty env
# entry, so normalize that case before starting the binary.
if [ -z "${WORLDCOVER_PATH:-}" ]; then
	unset WORLDCOVER_PATH
fi

exec hiking-import
