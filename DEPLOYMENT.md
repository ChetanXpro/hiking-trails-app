# Production deployment

This is the low-cost, single-VPS deployment for the prototype. Only Caddy
publishes host ports. PostgreSQL, Martin, the Rust API, and the SvelteKit server
stay on a private Docker network.

```text
browser -> Cloudflare (optional) -> Caddy :443
                                  |-> SvelteKit web
                                  |-> /api/* -> Rust API -> PostgreSQL
                                  `-> tile paths -> Martin -> PostgreSQL + Terrain-RGB MBTiles
```

The conservative defaults fit a 2-vCPU/2-GB VPS with swap and roughly 25 GB of
disk plus the selected terrain file. Do builds and heavy geodata imports on a
workstation or temporary larger machine whenever possible.

Prerequisites are a current Docker Engine with the Compose v2 plugin/BuildKit,
Git, curl, and enough disk for the selected release. Workstation-only geodata
preparation additionally needs the tools listed in the root README.

The stack supports both x86-64 and ARM64 natively. Set `DEPLOY_PLATFORM` to
`linux/amd64` for an Intel/AMD VPS or `linux/arm64` for an Oracle Ampere A1 VPS.
Do not emulate AMD64 on ARM in production. Martin, Caddy, Rust, Node, and the
official PostgreSQL base all publish ARM64 images. Because the upstream
`postgis/postgis` image is AMD64-only, `deploy/Dockerfile.postgis` installs the
official PGDG PostGIS package into the multi-architecture PostgreSQL 16 image.

### Oracle Always Free ARM

For Oracle Cloud, select one `VM.Standard.A1.Flex` instance with Ubuntu or
Oracle Linux and set:

```dotenv
DEPLOY_PLATFORM=linux/arm64
```

Oracle's current Always Free documentation lists a total of **2 OCPUs and 12 GB
RAM**, plus 200 GB of combined boot/block storage. That comfortably fits the
current runtime; keep the large DEM fusion and imports on the workstation. A
50 GB boot volume is enough for the present prototype, or place Docker data and
`DATA_DIR` on an attached block volume for future regional growth. Verify the
actual allowance in the tenancy's Limits, Quotas and Usage page because some
older Oracle pages still show the historical 4-OCPU/24-GB amount. See Oracle's
[current Always Free limits](https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm)
and [tenancy-limit instructions](https://docs.oracle.com/en-us/iaas/Content/General/service-limits/view-tenancy.htm).

Always Free has no SLA, capacity can be unavailable, and Oracle may reclaim
idle free instances. Keep the database dump and immutable data release in R2 so
the server remains replaceable. Route traffic directly through Caddy and
Cloudflare rather than the bandwidth-limited free load balancer.

## What runs in production

- `gateway`: TLS, HTTP/3, compression, security headers, access logs, routing
- `web`: SvelteKit's explicit Node adapter
- `api`: read-only application database role
- `martin`: read-only tile database role and `data/terrain_rgb_hma.mbtiles`
- `db`: Postgres 16 + PostGIS; no host port
- `migrate`, `restore`, `osm-import`, `importer`, and `cleanup-import`: one-shot
  profile services, never always-on runtime services

Every production migration finishes by revoking broad runtime table access and
granting only the API and Martin tables listed in
`deploy/sql/apply_runtime_grants.sql`. New migration-created tables therefore
remain private until they are deliberately added to that allow-list.

Runtime does **not** need the regional PBF, fused analysis DEM, WorldCover, GDAL,
osm2pgsql, or raw import tables. In particular, do not upload or deploy the old
21-GB `terrain_rgb.mbtiles`; the map expects the selected merged
`terrain_rgb_hma.mbtiles` artifact.

Martin's upstream cache default is 512 MB, which is too large for this prototype
container. Production explicitly caps it at 96 MB and uses eight PostgreSQL
connections. Martin handles a small hot set locally; when Cloudflare is enabled,
it uses Caddy's cache headers to absorb repeat public tile traffic.
The API admits at most 32 active handlers, uses ten database connections, and
caps a generated route download at 25,000 emitted points and 2 MiB. Raise those
limits only after raising the API memory budget and measuring real routes.

## 1. Prepare configuration

On the workstation and VPS:

```bash
cp .env.production.example .env.production
chmod 600 .env.production
```

Set the domain, image prefix/version, asset URLs, and four independent database
passwords. Use URL-safe secrets because SQLx/Martin receive libpq URLs:

```bash
openssl rand -hex 32
```

Do that independently for `POSTGRES_PASSWORD`, `API_DB_PASSWORD`,
`IMPORT_DB_PASSWORD`, and `MARTIN_DB_PASSWORD`. Never commit
`.env.production`.

Set `DEPLOY_UID` and `DEPLOY_GID` to the numeric non-root account that owns the
checkout and downloaded data on the VPS (`id -u` and `id -g`). One-shot jobs run
with that identity, so private host artifacts remain readable without making the
containers root. Preflight deliberately rejects UID/GID zero. If the checkout or
data was created by root, change its ownership to the dedicated deployment
account before starting the stack; do not work around the permissions by setting
`DEPLOY_UID=0`.

Set `DEPLOY_PLATFORM=linux/arm64` for Oracle Ampere A1 or `linux/amd64` for a
normal Intel/AMD VPS. Build and deploy every application image with the same
value; preflight rejects other platform strings.

Also keep three independent immutable identifiers:

- `APP_VERSION`: the application image tag, preferably a full Git commit SHA
- `POSTGIS_IMAGE_VERSION`: the deliberately versioned database runtime; do not
  change it during an ordinary application release
- `DATA_RELEASE`: the database/terrain release embedded in public tile cache keys

Leave the three `PUBLIC_MARTIN_*`/`PUBLIC_TERRAIN_RGB_URL` values blank for the
same-origin Compose defaults; they automatically embed `DATA_RELEASE`. If you
override them with a separate HTTPS tile origin, preflight requires the exact
release identifier in each URL.

`PGDATA_VOLUME_NAME` is explicit so a replacement database can be prepared in a
new volume without touching the current rollback volume. Use a versioned name
such as `hiking_pgdata_20260713`.

Run the fail-closed configuration check before building or pulling anything:

```bash
./scripts/production_preflight.sh --env-file .env.production --config-only
```

It rejects the example domains, passwords, image prefix/version, data release,
unsafe secret-file permissions, and unversioned tile URLs without printing any
secret value.

The three non-admin roles are created only when the database volume is first
initialized:

- `hiking_api`: normalized API tables, `SELECT` only
- `hiking_tiles`: Martin tile tables, `SELECT` only
- `hiking_import`: migrations/imports and schema ownership

Changing a password in the env file does not rotate a role in an existing
volume. Rotate the corresponding PostgreSQL role deliberately, then update the
env file.

### Frontend data providers

`PUBLIC_*` values are runtime configuration, so the same web image can run in
staging and production. Production has no hidden OSM raster, demo-glyph, or
weather fallback.

For a self-controlled map, publish the style JSON, sprites, fonts, and any
basemap vector tiles from your R2 custom domain and set:

```dotenv
PUBLIC_BASEMAP_STYLE_URL=https://map-assets.example.com/styles/outdoor.json
PUBLIC_MAP_GLYPHS_URL=https://map-assets.example.com/fonts/{fontstack}/{range}.pbf
```

Check every URL inside the style JSON; otherwise it may still reference a
third-party tile or sprite server. Configure R2 CORS for the application origin.
For the earliest non-commercial prototype you can explicitly choose
OpenFreeMap, but that is an operational dependency rather than a guaranteed
service.

Leave `PUBLIC_WEATHER_API_URL` empty to disable weather. Every `PUBLIC_*` value is
sent to browsers, so never put a secret weather-provider key in it. Use a
domain-restricted public token or a same-origin cached proxy that stores its
credential only on the server.

## 2. Build and publish immutable images

Building Rust on a tiny VPS is slow and can exhaust memory. Build on the
workstation or in CI, then push to GHCR/another registry:

Run the source release gate first:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo audit
pnpm --dir web install --frozen-lockfile
pnpm --dir web check
pnpm --dir web test
pnpm --dir web build
pnpm --dir web audit --prod
docker compose --env-file .env.production -f docker-compose.prod.yml config --quiet
```

Install `cargo-audit` once with `cargo install cargo-audit --locked`. The narrow
RustSec exception in `.cargo/audit.toml` covers only RSA pulled into the lockfile
by SQLx's disabled optional MySQL driver; `cargo tree -i rsa` must remain empty
for the PostgreSQL-only production binaries.

```bash
docker compose --env-file .env.production -f docker-compose.prod.yml \
  build db
docker compose --env-file .env.production -f docker-compose.prod.yml \
  push db
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile tools build api web migrate
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile tools push api web migrate
```

The database image pins PostgreSQL 16.14 and PostGIS 3.6.4 and has its own
`POSTGIS_IMAGE_VERSION`. Build it once for the selected platform and retain that
tag. Do not rebuild or retag it as part of every `APP_VERSION` release. A future
database-image upgrade requires a tested fresh-volume dump/restore (preferred)
or an explicit PostGIS extension upgrade plan.

Set `IMAGE_PREFIX`, for example `ghcr.io/your-org/hiking`, and use an immutable
`APP_VERSION` such as a Git commit SHA. The optional import image is intentionally
separate and large; build/push it only when using the import profile:

```bash
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile import build osm-import importer cleanup-import
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile import push osm-import
```

All three import jobs use the same `tools` image tag, so one push is enough.
For an initial prototype it is also valid to clone the source and build on the
VPS. Set `IMAGE_PREFIX=local/hiking` and an immutable `APP_VERSION`, then run:

```bash
./scripts/production_preflight.sh --env-file .env.production --config-only
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile tools build db api web migrate
```

Temporarily add swap if needed. In this local-build workflow, skip the
corresponding `push` and `pull` commands throughout this guide; Compose will use
the locally tagged images.

## 3. Prepare a small runtime data release

The recommended workflow imports and enriches data on the workstation, then
ships only a normalized data-only database archive and the selected Terrain-RGB
MBTiles file.

After a completely successful import and verification, optionally remove the
regenerable staging tables to keep the archive small:

```bash
psql "$DATABASE_URL" --set ON_ERROR_STOP=1 \
  --file deploy/sql/drop_import_staging.sql
```

This is destructive only to `raw_*`, `planet_osm_*`, and osm2pgsql metadata.
The SQL refuses to run when normalized trails are missing/empty. A future
snapshot import recreates staging.

Build the bootstrap archive without schema ownership or migration rows:

```bash
./scripts/create_bootstrap_dump.sh
```

The script runs `pg_dump` inside the PostgreSQL service, then uses `pg_restore`
to validate both the archive table of contents and every compressed payload
block. This keeps the tools on the server's major version and catches truncated
archives before the atomic rename. Do not replace it with an arbitrary newer
local client: newer dumps can contain settings that PostgreSQL 16 cannot
restore. For a non-default Compose project, pass `--compose-file`, `--env-file`,
and `--db-service` explicitly. PostgreSQL can still print its generic warning
about the self-referencing `regions` table; migration 0017 makes that foreign
key initially deferred, and the production restore runs in one transaction.

Place the production terrain file at:

```text
data/terrain_rgb_hma.mbtiles
```

### Generate a release manifest

Object keys and `DATA_RELEASE` should be versioned and immutable:

```bash
./scripts/generate_data_manifest.sh \
  --source-dir ./data \
  --output ./deploy/data-manifest.tsv \
  --asset runtime terrain_rgb_hma.mbtiles releases/hp-20260713-a1/terrain_rgb_hma.mbtiles \
  --asset runtime bootstrap/hiking-data.dump releases/hp-20260713-a1/hiking-data.dump
```

For a remote import, add the `import` assets shown in
`deploy/data-manifest.example.tsv`: regional PBF, fused analysis DEM, optional
WorldCover, and optional original-mapper manifest.

The analysis source is explicit: set `DEM_PATH` to one fused raster, or clear it
and set comma-separated `DEM_PATHS` for multiple overlapping rasters. Keep
`DEM_AUTO_DISCOVER=false` so an old tile directory cannot silently win. The
Terrain-RGB URL/encoding/tile size/max zoom are separately configurable through
`PUBLIC_TERRAIN_*`; classification/elevation always uses the honest analysis DEM,
not visual terrain exaggeration.

### Upload to R2

Rclone is the supported private uploader. Configure an `s3` remote with
`provider = Cloudflare`, the account R2 endpoint, and credentials stored outside
the repository. Then:

```bash
./scripts/upload_deployment_data.sh \
  --manifest ./deploy/data-manifest.tsv \
  --source-dir ./data \
  --rclone-remote r2:hiking-data \
  --manifest-object manifests/hp-20260713-a1.tsv
```

Assets upload first and the manifest uploads last. Credentials never enter the
manifest. Rclone's immutable mode refuses to change an existing release key; use
a new object key when bytes change. The downloader checks exact byte size and
SHA256, not an S3 ETag.

### Download on the VPS

For a private bucket, configure the same read-only rclone remote on the VPS.
This is recommended for database archives and requires no public data endpoint:

```bash
rclone copyto \
  r2:hiking-data/manifests/hp-20260713-a1.tsv \
  ./deploy/data-manifest.tsv
./scripts/sync_deployment_data.sh \
  --manifest ./deploy/data-manifest.tsv \
  --rclone-remote r2:hiking-data \
  --output-dir ./data \
  --group runtime
```

Alternatively, use an R2 custom domain/read-only HTTPS release origin for
artifacts that are safe to be public. This URL is used by the server during
deployment, not by the map client:

```bash
./scripts/sync_deployment_data.sh \
  --manifest ./deploy/data-manifest.tsv \
  --base-url https://data.example.com \
  --output-dir ./data \
  --group runtime
```

HTTPS downloads resume in `.part` files; rclone transfers retry. Both replace
live files atomically only after SHA256 and size validation. Re-running is safe.
Verify without network writes:

```bash
./scripts/sync_deployment_data.sh \
  --manifest ./deploy/data-manifest.tsv \
  --output-dir ./data --group runtime --check-only
```

## 4. First deployment

Point DNS at the VPS, allow inbound TCP 80/443 and UDP 443, and do not allow
5432, 3000, 3001, or 8080. Then:

```bash
./scripts/production_preflight.sh --env-file .env.production
# Registry-image workflow only; skip these two pulls after a local VPS build.
docker compose --env-file .env.production -f docker-compose.prod.yml pull
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile tools pull migrate
docker compose --env-file .env.production -f docker-compose.prod.yml up -d db
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile tools run --rm migrate
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile tools run --rm restore
docker compose --env-file .env.production -f docker-compose.prod.yml \
  up -d db api martin web gateway
```

`restore` is for a new/empty migrated database. It validates the archive,
restores all data in one transaction, and runs `ANALYZE`; any failure rolls the
restore back. Do not apply the same data-only archive twice.

Check health and logs:

```bash
docker compose --env-file .env.production -f docker-compose.prod.yml ps
docker compose --env-file .env.production -f docker-compose.prod.yml logs --tail=200
curl --fail https://maps.example.com/api/health
curl --fail https://maps.example.com/terrain_rgb_hma/6/45/24 --output /dev/null
```

Caddy obtains and renews TLS automatically. If Cloudflare proxying is enabled,
use SSL/TLS mode **Full (strict)**. Keep the origin certificate valid and,
firewall the origin to Cloudflare IPs after initial validation so traffic cannot
bypass its protections.

Create a required Cloudflare cache-eligibility rule for the explicit Martin tile
prefixes (`/trail_lines/`, `/trail_start_points/`, `/trail_end_points/`,
`/outdoor_waypoints/`, `/terrain_rgb_hma/`). These extensionless URLs are not a
safe place to rely on default file-extension caching. Respect the origin
`Cache-Control` header, retain the complete query string in the cache key, and
never cache errors. Tile URLs contain `DATA_RELEASE`, so new releases get new
cache keys; purge the old prefixes after a successful switch. Do not cache
`/api/search` indiscriminately—the API supplies its own endpoint-specific policy.

## Optional: import on a temporary larger server

Download the import group, then run each one-shot job explicitly:

```bash
./scripts/sync_deployment_data.sh \
  --manifest ./deploy/data-manifest.tsv \
  --base-url https://data.example.com \
  --output-dir ./data --group import

docker compose --env-file .env.production -f docker-compose.prod.yml up -d db
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile import run --rm migrate
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile import run --rm osm-import
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile import run --rm importer
```

`OSM2PGSQL_CACHE_MB` and `IMPORT_MEMORY_LIMIT` cap the one-shot jobs. Keep the
osm2pgsql cache comfortably below the container limit and run this profile on a
temporary larger machine if the selected PBF exceeds the prototype defaults.

Keep `IMPORT_PRUNE_MISSING_TRAILS=false` for regional/partial inputs. Set it true
only when the raw tables are a complete authoritative replacement snapshot. The
importer still prunes only after zero reconstruction failures and refuses to
prune from an empty snapshot.

Only after `importer` exits zero, inspect trail counts, a few details/elevation
profiles, and map tiles. Then opt into staging cleanup:

```bash
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile import run --rm cleanup-import
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile import images osm-import
# After recording the exact tag/ID above, remove that unused tools image only.
```

Remove the PBF, analysis DEM, and WorldCover files from the small runtime VPS
after making sure the R2 release is intact. Martin needs only the runtime
Terrain-RGB MBTiles.

## Application-only upgrade

1. Create and verify a database backup.
2. Publish new immutable image tags.
3. Change `APP_VERSION` and any application-only `PUBLIC_*` settings.
4. Pull, run the one-shot migrator, then recreate runtime services.

```bash
./scripts/production_preflight.sh --env-file .env.production
# Registry workflow: pull. Local-build workflow: use
# `--profile tools build api web migrate` instead.
docker compose --env-file .env.production -f docker-compose.prod.yml pull
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile tools pull migrate
docker compose --env-file .env.production -f docker-compose.prod.yml \
  --profile tools run --rm migrate
docker compose --env-file .env.production -f docker-compose.prod.yml \
  up -d --remove-orphans db api martin web gateway
docker compose --env-file .env.production -f docker-compose.prod.yml ps
```

### Database or terrain data release

Never restore a new data archive over the live database. Prepare it in a fresh,
explicitly named volume while the old application keeps serving:

1. Sync the new manifest and artifacts atomically into a new release-specific
   host directory; never replace the directory mounted by the live Martin.
2. Copy `.env.production` to `.env.production.next`, then set a distinct
   `COMPOSE_PROJECT_NAME`, a new `PGDATA_VOLUME_NAME`, the release-specific
   `DATA_DIR`, the new `RESTORE_DUMP`, `DATA_RELEASE`, and matching versioned
   `PUBLIC_*` tile URLs.
3. Migrate and restore the fresh volume, then start only its private services for
   verification—do not start its gateway.

```bash
cp .env.production .env.production.next
chmod 600 .env.production.next
# Download this manifest with rclone first, then populate an isolated directory:
./scripts/sync_deployment_data.sh \
  --manifest ./deploy/data-manifest.next.tsv \
  --rclone-remote r2:hiking-data \
  --output-dir ./data/releases/hp-20260720 \
  --group runtime
# Edit the next env now; for example:
# COMPOSE_PROJECT_NAME=hiking-data-next
# PGDATA_VOLUME_NAME=hiking_pgdata_20260720
# DATA_DIR=./data/releases/hp-20260720
# DATA_RELEASE=hp-20260720

./scripts/production_preflight.sh --env-file .env.production.next
docker compose --env-file .env.production.next -f docker-compose.prod.yml pull
docker compose --env-file .env.production.next -f docker-compose.prod.yml \
  --profile tools pull migrate
docker compose --env-file .env.production.next -f docker-compose.prod.yml up -d db
docker compose --env-file .env.production.next -f docker-compose.prod.yml \
  --profile tools run --rm migrate
docker compose --env-file .env.production.next -f docker-compose.prod.yml \
  --profile tools run --rm restore
docker compose --env-file .env.production.next -f docker-compose.prod.yml \
  up -d db api martin web
docker compose --env-file .env.production.next -f docker-compose.prod.yml \
  exec -T api curl --fail --silent http://127.0.0.1:8080/ready
docker compose --env-file .env.production.next -f docker-compose.prod.yml \
  exec -T api curl --fail --silent http://martin:3000/health
```

Inspect trail counts, trail detail/elevation/download responses, and representative
tiles. To switch, stop the current stack first, stop the temporary project without
`-v`, update `.env.production` to the verified volume/data release, and start it:

```bash
docker compose --env-file .env.production -f docker-compose.prod.yml \
  stop gateway web api martin db
docker compose --env-file .env.production.next -f docker-compose.prod.yml down
# Update PGDATA_VOLUME_NAME, DATA_DIR, DATA_RELEASE, RESTORE_DUMP, and versioned
# tile URLs in .env.production to the verified values.
./scripts/production_preflight.sh --env-file .env.production
docker compose --env-file .env.production -f docker-compose.prod.yml \
  up -d db api martin web gateway
```

Purge Cloudflare's old tile cache keys after health checks pass. Keep the old
database volume, old manifest, and old images until the release is proven; a
rollback switches `PGDATA_VOLUME_NAME`, `DATA_DIR`, and `DATA_RELEASE` back.

## Backup and restore

Create the same data-only archive contract used for bootstrap. The helper uses
the PostgreSQL 16 tools from the running database container, checks the full
archive payload, publishes it atomically, and leaves no completed file on
failure:

```bash
mkdir -p backups
backup="backups/hiking-$(date -u +%Y%m%dT%H%M%SZ).dump"
./scripts/create_bootstrap_dump.sh \
  --output "$backup" \
  --compose-file docker-compose.prod.yml \
  --env-file .env.production \
  --db-service db
```

Also retain `.env.production` securely, the exact release manifest, and the
Terrain-RGB MBTiles/R2 release. A database-only backup cannot reconstruct those
large immutable artifacts.

Practice restore with the fresh-volume data-release procedure above. The restore
service accepts only the data portion, rejects archives containing migration or
extension-owned table rows, restores with `--single-transaction`, and runs
`ANALYZE`. Never delete the old volume until the restored application has passed
the private API `/ready` check, trail-detail, tile, and download checks. Caddy
intentionally returns 404 for public `/api/ready`; use the internal Compose
healthcheck or `exec -T api curl ... http://127.0.0.1:8080/ready` instead.

## Rollback

For an application-only rollback, set `APP_VERSION` to the previously tested
immutable tag and recreate `api`, `web`, and `gateway`. Keep old image tags until
the new release is proven.

Database migrations are forward-only. If an older binary is incompatible with
the migrated schema, restore the pre-upgrade database backup into a fresh volume
rather than trying to reverse SQL manually. Roll back a data release by selecting
the previous `PGDATA_VOLUME_NAME`, release-specific `DATA_DIR`, and
`DATA_RELEASE`; do not overwrite the live artifact directory in place.

## Routine operations

```bash
# Render and validate the fully interpolated Compose model
docker compose --env-file .env.production -f docker-compose.prod.yml config --quiet

# Service/resource view
docker compose --env-file .env.production -f docker-compose.prod.yml ps
docker stats --no-stream

# Follow bounded JSON logs
docker compose --env-file .env.production -f docker-compose.prod.yml logs -f --tail=200 api martin gateway

# Stop runtime without deleting data
docker compose --env-file .env.production -f docker-compose.prod.yml down
```

Never use `down -v` during routine deployment; it removes the named database
volume. Logs rotate at 10 MB × 3 per service. Caddy's certificate state and the
database remain in named volumes.
