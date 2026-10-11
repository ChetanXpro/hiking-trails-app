#!/bin/sh
set -eu

hiking-migrate

# psql uses PGHOST/PGPORT/PGDATABASE/PGUSER/PGPASSWORD from the environment so
# the database password never appears in a process argument.
exec psql --set ON_ERROR_STOP=1 --file /app/deploy/sql/apply_runtime_grants.sql
