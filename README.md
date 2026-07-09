# Hiking trails app

A from-scratch hiking trail platform built on free OpenStreetMap data,
scoped to Himachal Pradesh first but designed to scale to all of India
or the world without schema changes.

## Layout

```
hiking-trails-app/
├── Cargo.toml               # workspace root
├── crates/
│   ├── hiking-db/           # schema-matching models + WKB geometry helpers
│   ├── hiking-import/       # OSM extraction -> stitching -> trails/segments
│   └── hiking-api/          # Actix-web API server
├── migrations/
│   └── 0001_init_schema.sql # regions, trails, segments, waypoints, elevation_profiles, sync_state
├── osm2pgsql/
│   └── trails.lua           # flex-output config: raw staging import from .osm.pbf
└── web/                     # SvelteKit + MapLibre frontend
```

## Setup

Copy the env template once (this is your local config; `.env` is
gitignored, `.env.example` is the committed template):

```bash
cp .env.example .env
```

Docker is used for infra only -- Postgres and Martin. The Rust
binaries and the SvelteKit dev server run natively on your machine for
a fast, normal dev loop (no rebuild-on-save anywhere).

## 1. Start infra

```bash
docker compose up -d
```

Starts `db` (Postgres + PostGIS) and `martin` (vector tile server),
both configured from `.env`.

**Checkpoint:** `docker compose ps` -- both should show `running`/`healthy`.

## 2. Apply the schema

```bash
sqlx migrate run
```

`sqlx-cli` reads `DATABASE_URL` straight from `.env` automatically --
no manual `export` needed. (Install it once: `cargo install sqlx-cli
--no-default-features --features postgres`.)

## 3. Get the data (Himachal Pradesh scope)

Install `osmium-tool` and `osm2pgsql` locally (`brew install
osmium-tool osm2pgsql` on Mac), then:

```bash
curl -O https://download.geofabrik.de/asia/india-latest.osm.pbf
osmium extract -p himachal_pradesh.poly india-latest.osm.pbf -o hp.osm.pbf
osmium tags-filter hp.osm.pbf \
  w/highway \
  r/route=hiking \
  n/tourism=alpine_hut,wilderness_hut,camp_site \
  n/natural=peak,spring \
  -o hp-trails.osm.pbf
```

Faster first test with a small bounding box instead of the full HP polygon:

```bash
osmium extract -b 77.05,32.15,77.35,32.35 india-latest.osm.pbf -o test.osm.pbf
```

## 4. Raw import into staging tables

```bash
source .env   # so $DATABASE_URL is set for this shell
# Flex output reads projection from `osm2pgsql/trails.lua`, so do not pass `-l`.
osm2pgsql -d "$DATABASE_URL" --slim --output=flex \
  --style=osm2pgsql/trails.lua \
  hp-trails.osm.pbf
```

## 5. Stitch raw data into real trails

```bash
cargo run -p hiking-import
```

`hiking-import` loads `.env` itself at startup (via the `dotenvy`
crate) -- you don't need `DATABASE_URL` exported in this shell for
this step. Reads `raw_relations` + `raw_ways`, reconstructs each
relation's real walking order via graph/degree analysis (see
`crates/hiking-import/src/stitch.rs` for the full reasoning -- this is
the piece that fixes what Waymarked Trails' UI surfaces as a raw
"route is not sorted" warning, permanently, instead of just displaying
it), and writes ordered rows into `trails` + `segments`.

The broad `w/highway` extract is intentional. Hiking relations often
reuse ordinary road segments for short connectors, village approaches,
or bridge crossings. Filtering to only `path`/`footway`/`track` can
silently drop legitimate route members and shorten the final trail.

## 6. Run the API

```bash
cargo run -p hiking-api
```

Also loads `.env` itself at startup -- just run it, no export needed.
Serves on `:8080`:
- `GET /health`
- `GET /regions/:region_id/trails` -- light list view, no geometry
- `GET /trails/:trail_id` -- full detail, geometry as GeoJSON
- `GET /trails/:trail_id/elevation` -- elevation profile, fetched lazily

## 7. Run the frontend

```bash
cd web
npm install
npm run dev
```

Open **http://localhost:5173**.

## A build-time requirement worth knowing

`hiking-api` and `hiking-import` use `sqlx::query_as!`/`sqlx::query!`
macros, not the runtime-only query API -- these connect to a real,
migrated Postgres database *while you're compiling*, to verify the SQL
and Rust types match the actual schema. Practically this means:

- `DATABASE_URL` (from `.env`) must point at a live, migrated database
  whenever you run `cargo build` / `cargo check`, not just when you run
  the binaries.
- For CI, or anywhere a live DB isn't reachable at build time, run
  `cargo sqlx prepare --workspace` once locally -- this caches the
  verified query metadata into a `.sqlx/` folder that you commit to
  git, and sqlx will use that cache instead of a live connection.

## Everyday dev loop

- `docker compose up -d` / `docker compose down` for infra, once per session
- `cargo run -p hiking-api`, `npm run dev` -- normal native processes, each in their own terminal tab
- Change `.env` once, every tool (`sqlx-cli`, `hiking-api`, `hiking-import`, `docker compose`) picks it up the same way -- no more hunting down every place a credential is hardcoded

## Elevation pipeline

- Elevation profiles are generated during `cargo run -p hiking-import`
  when `DEM_PATH` or `DEM_PATHS` points at one or more local raster DEMs
  (`.tif`, `.vrt`, etc.)
- The importer samples the stitched route, stores the profile in
  `elevation_profiles`, updates `trails.ascent_m` / `trails.descent_m`,
  and recalculates estimated duration using distance plus elevation
- Elevation totals are now computed from a smoothed profile with a
  thresholded gain/loss pass, so small DEM noise does not inflate ascent
- If `WORLDCOVER_PATH` is set, the importer also caches coarse terrain
  segments from ESA WorldCover in `trail_landcover_segments`

Example:

```bash
cp .env.example .env
# then set DEM_PATH / DEM_PATHS in .env to your local DEM tile(s)
# optional: set WORLDCOVER_PATH to a WorldCover GeoTIFF

source .env
osm2pgsql -d "$DATABASE_URL" --slim --output=flex \
  --style=osm2pgsql/trails.lua \
  hp-trails.osm.pbf

cargo run -p hiking-import
```

- If no DEM path is configured, the importer still works, but elevation
  profiles and ascent/descent stats are skipped

## Terrain rendering

The frontend can now render hillshade, client-side contours, and optional
3D terrain from the same DEM source you already use for elevation
profiles.

### 1. Build Terrain-RGB tiles from your DEM

Install `rio-rgbify` once:

```bash
pip install rio-rgbify --break-system-packages
```

If you want public DEM coverage without registration, download the
Copernicus GLO-30 tiles for your bbox first:

```bash
./scripts/download_copernicus_tiles.sh 31 76 33 78 ./data/dem_tiles
```

Then generate a Terrain-RGB MBTiles file from one or more FABDEM /
Copernicus tiles:

```bash
./scripts/dem_to_terrain_rgb.sh ./data/terrain_rgb.mbtiles ./data/dem_tiles/*.tif
```

Or for a larger multi-tile region:

```bash
./scripts/download_copernicus_tiles.sh 28 76 35 82 ./data/dem_tiles

MIN_ZOOM=6 MAX_ZOOM=14 WORKERS=8 \
  ./scripts/dem_to_terrain_rgb.sh ./data/terrain_rgb.mbtiles ./data/dem_tiles/*.tif
```

### 2. Serve it from Martin

This repo's `docker-compose.yml` now mounts the local `./data`
directory into the Martin container, so placing
`./data/terrain_rgb.mbtiles` there is enough. Restart Martin after the
file exists, and it will auto-discover the MBTiles and serve the tiles
at:

```text
/terrain_rgb/{z}/{x}/{y}
```

The web app defaults to that relative URL already, so no extra frontend
config is needed if Martin is reverse-proxied the same way as the trail
vector tiles.

### 3. Use it in the map

Open the new `Terrain` control in the top-right of the map to toggle:

- hillshade
- client-side contour lines
- 3D terrain
- contour units (meters / feet)
- terrain exaggeration

Notes:

- Contours are generated in-browser from Terrain-RGB via
  `maplibre-contour`, so there is no separate contour-tiling pipeline.
- The currently installed `maplibre-gl` version in this repo supports
  hillshade + raster-dem terrain, but not `color-relief` layers yet, so
  elevation tinting is intentionally left disabled for now instead of
  half-working.

## What's deliberately not here yet

- User accounts / community content -- out of scope for now
- SOI licensed data -- phase 2, pending the licensing/entity-type
  question flagged earlier
