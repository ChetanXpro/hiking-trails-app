#!/usr/bin/env bash
set -Eeuo pipefail

: "${POSTGRES_DB:?POSTGRES_DB is required}"
: "${POSTGRES_USER:?POSTGRES_USER is required}"
: "${API_DB_PASSWORD:?API_DB_PASSWORD is required}"
: "${IMPORT_DB_PASSWORD:?IMPORT_DB_PASSWORD is required}"
: "${MARTIN_DB_PASSWORD:?MARTIN_DB_PASSWORD is required}"

# This runs only when the Postgres data volume is first initialized. psql reads
# passwords from the container environment inside the script, so secrets never
# appear in process arguments or interpolated shell SQL.
psql \
	--set ON_ERROR_STOP=1 \
	--username "${POSTGRES_USER}" \
	--dbname "${POSTGRES_DB}" <<'SQL'
\getenv db_name POSTGRES_DB
\getenv api_password API_DB_PASSWORD
\getenv import_password IMPORT_DB_PASSWORD
\getenv martin_password MARTIN_DB_PASSWORD

CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS pgcrypto;

SELECT format(
    'CREATE ROLE hiking_api LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION PASSWORD %L',
    :'api_password'
)
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'hiking_api') \gexec

SELECT format('ALTER ROLE hiking_api PASSWORD %L', :'api_password') \gexec

SELECT format(
    'CREATE ROLE hiking_import LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION PASSWORD %L',
    :'import_password'
)
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'hiking_import') \gexec

SELECT format('ALTER ROLE hiking_import PASSWORD %L', :'import_password') \gexec

SELECT format(
    'CREATE ROLE hiking_tiles LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION PASSWORD %L',
    :'martin_password'
)
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'hiking_tiles') \gexec

SELECT format('ALTER ROLE hiking_tiles PASSWORD %L', :'martin_password') \gexec

REVOKE CONNECT, TEMPORARY ON DATABASE :"db_name" FROM PUBLIC;
REVOKE ALL ON SCHEMA public FROM PUBLIC;

GRANT CONNECT ON DATABASE :"db_name" TO hiking_api, hiking_import, hiking_tiles;
GRANT TEMPORARY ON DATABASE :"db_name" TO hiking_import;
GRANT USAGE ON SCHEMA public TO hiking_api, hiking_import, hiking_tiles;
GRANT CREATE ON SCHEMA public TO hiking_import;

GRANT SELECT, INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES, TRIGGER
    ON ALL TABLES IN SCHEMA public TO hiking_import;
GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA public TO hiking_import;

ALTER ROLE hiking_api SET default_transaction_read_only = on;
ALTER ROLE hiking_api SET statement_timeout = '15s';
ALTER ROLE hiking_api SET lock_timeout = '3s';
ALTER ROLE hiking_tiles SET default_transaction_read_only = on;
ALTER ROLE hiking_tiles SET statement_timeout = '15s';
ALTER ROLE hiking_tiles SET lock_timeout = '3s';

ALTER DEFAULT PRIVILEGES FOR ROLE hiking_import IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES, TRIGGER
    ON TABLES TO hiking_import;
ALTER DEFAULT PRIVILEGES FOR ROLE hiking_import IN SCHEMA public
    GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO hiking_import;
SQL
