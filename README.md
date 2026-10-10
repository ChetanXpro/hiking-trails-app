# Hiking trails app

For the low-cost single-VPS production stack, R2 artifact workflow, backups,
and release procedure, see [DEPLOYMENT.md](DEPLOYMENT.md).

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
│   └── 0001..0017           # forward-only PostgreSQL/PostGIS migrations
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
cargo run -p hiking-db --bin hiking-migrate
```

The repository-owned migrator reads `DATABASE_URL` from `.env`, applies every
file in `migrations/`, and avoids requiring a separately installed SQLx CLI.

## 3. Get the data (Himachal Pradesh scope)

Install `osmium-tool` and `osm2pgsql` locally (`brew install
osmium-tool osm2pgsql` on Mac), then:

```bash
curl -O https://download.geofabrik.de/asia/india-latest.osm.pbf
./scripts/build_himachal_trails.sh ./india-latest.osm.pbf ./data
```

This script derives the Himachal Pradesh admin boundary directly from
the local India extract, so you do not need a separately downloaded
`.poly` file. It uses Osmium's smart strategy so ordinary hiking routes and
the polygon relations used for regions/POIs remain reference-complete when
they cross the state boundary. Country-scale `type=superroute` relations are
deliberately excluded from state-level trail staging; those belong in a
future full-country import rather than being published as misleading partial
routes. It writes:

- `./data/himachal_pradesh.geojson`
- `./data/hp.osm.pbf`
- `./data/hp-trails.osm.pbf` (legacy, trails-only filtered extract)

Faster first test with a small bounding box instead of the full HP polygon:

```bash
osmium extract --strategy smart --option types=route,multipolygon,boundary \
  --option tags=route=hiking,boundary=administrative,boundary=protected_area \
  -b 77.05,32.15,77.35,32.35 india-latest.osm.pbf -o test.osm.pbf
```

## 4. Raw import into staging tables

```bash
source .env   # so $DATABASE_URL is set for this shell
# Flex output reads projection from `osm2pgsql/trails.lua`, so do not pass `-l`.
# Import the full Himachal extract here so admin boundaries and named places
# are available for hierarchical search as well as trail stitching.
osm2pgsql -d "$DATABASE_URL" --slim --drop --output=flex \
  --style=osm2pgsql/trails.lua \
  ./data/hp.osm.pbf
```

This pass also stages the outdoor map hierarchy: huts and typed shelters,
campgrounds, verified drinking water (kept distinct from unverified springs),
rescue/emergency points, trailheads, viewpoints, passes, waterfalls, caves,
toilets, parking, guideposts, picnic sites, and aerialway stations. When that
tag list changes, rerun this raw import and then `cargo run -p hiking-import`;
no extra schema migration is required.

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
silently drop legitimate route members and shorten the final trail. The flex
style also uses osm2pgsql's second-stage relation-member selection to retain
supported blank/`forward`/`backward` members that do not have `highway=*`,
without storing every unrelated regional way.

For an authoritative snapshot refresh, set
`IMPORT_PRUNE_MISSING_TRAILS=true` to remove OSM trails that disappeared or
became invalid. This is intentionally off by default: pruning runs only after
the stitching pass has zero reconstruction failures and refuses an empty raw
snapshot. Region/place pruning is not inferred because those rows do not yet
carry enough source-scope provenance to delete safely.

### Optional original OSM mapper attribution

Trail details can show the OpenStreetMap user who created version 1 of the
hiking route relation. This is **original map authorship**, not proof that the
person physically walked or recorded the trail.

The normal importer makes no network requests for contributor metadata. To
opt in, first inspect what the bounded fetcher would request. Dry-run is the
default and performs no network requests or writes:

```bash
./scripts/build_original_mapper_manifest.sh --limit 25
```

After reviewing the relation IDs, run a small identified batch. A contact email
or URL is mandatory and is included in the User-Agent:

```bash
./scripts/build_original_mapper_manifest.sh \
  --fetch \
  --contact maps@example.com \
  --limit 25 \
  --delay 1.25
```

The script requests only immutable relation version-1 records, groups at most
25 relations into each serial multi-fetch request, caches every result, never
refetches cached/unavailable relations, and stops on transient HTTP failures.
Re-run it in modest batches until the manifest is complete. The OSM editing API
is not a bulk download service; for large or worldwide backfills, use an
authorised history extract instead.

The generated file follows
[data/original_mappers.example.json](data/original_mappers.example.json). Set:

```bash
OSM_ORIGINAL_MAPPERS_PATH=/absolute/path/to/original_mappers.json
```

Every entry must explicitly say `osm_relation_version: 1`. The importer rejects
other versions, duplicate relation IDs, non-OSM profile links, unknown fields,
and manifests larger than 5 MiB. Applying the same file repeatedly is
idempotent. Entries absent from the manifest are left unchanged, and relations
not present in the local trail database are reported as unmatched.

Because OSM usernames and user IDs are contributor metadata, acquire and cache
the manifest in accordance with OSM's API, privacy, and attribution policies;
do not infer an original author from the latest public Geofabrik snapshot.

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
pnpm install
pnpm dev
```

Open **http://localhost:5173**.

## Builds do not need a database

The Rust crates use SQLx's runtime query API, so `cargo build`, `cargo test`,
and container builds do not connect to PostgreSQL. A database is required only
when running the migrator, importer, or API. This keeps CI and release image
builds deterministic and prevents build-time access to production credentials.

## Everyday dev loop

- `docker compose up -d` / `docker compose down` for infra, once per session
- `cargo run -p hiking-api`, `pnpm --dir web dev` -- normal native processes, each in their own terminal tab
- Change `.env` once; the migrator, API, importer, and Docker Compose read the same local configuration

## Elevation pipeline

- Elevation profiles are generated during `cargo run -p hiking-import`
  when `DEM_PATH` or `DEM_PATHS` points at one or more local raster DEMs
  (`.tif`, `.vrt`, etc.)
- The importer samples the stitched route, stores the profile in
  `elevation_profiles`, updates `trails.ascent_m` / `trails.descent_m`,
  and recalculates estimated duration using distance plus elevation
- Elevation totals are now computed from a smoothed profile with a
  thresholded gain/loss pass, so small DEM noise does not inflate ascent
- When multiple DEM rasters overlap, the importer now prefers the
  finer-resolution source automatically when `gdalinfo -json` exposes
  enough extent/size metadata to estimate nominal resolution
- If `WORLDCOVER_PATH` is set, the importer also caches coarse terrain
  segments from ESA WorldCover in `trail_landcover_segments`

Example:

```bash
cp .env.example .env
# then set DEM_PATH / DEM_PATHS in .env to your local DEM tile(s)
# overlapping DEMs are okay; the importer will prefer the finer source
# optional: set WORLDCOVER_PATH to a WorldCover GeoTIFF

source .env
osm2pgsql -d "$DATABASE_URL" --slim --output=flex \
  --style=osm2pgsql/trails.lua \
  ./data/hp.osm.pbf

cargo run -p hiking-import
```

- DEM selection is explicit by default. If no DEM path is configured, the
  importer still works and preserves existing elevation enrichment. Set
  `DEM_AUTO_DISCOVER=true` only when you intentionally want it to scan
  `data/dem_tiles`; production keeps this disabled.
- `ELEVATION_MAX_SAMPLES` bounds raster work per trail (20,000 by default),
  preventing malformed or exceptionally long routes from exhausting memory.

## Terrain rendering

The frontend can now render hillshade, client-side contours, and optional
3D terrain from the same DEM source you already use for elevation
profiles.

### 0. Use a vector basemap

Development defaults to a vector basemap style so labels stay crisp above the
relief and trail stack. Production has no hidden public basemap, raster,
glyph, or weather fallback; configure the runtime `PUBLIC_*` variables in
`.env.production` as described in [DEPLOYMENT.md](DEPLOYMENT.md).

```bash
VITE_BASEMAP_STYLE_URL=https://tiles.openfreemap.org/styles/liberty
```

Notes:

- The default URL above uses OpenFreeMap's public `liberty` style, which
  their quick start currently documents for MapLibre integration:
  [OpenFreeMap Quick Start](https://openfreemap.org/quick_start/).
- If you want the old raster fallback for offline testing or side-by-side
  comparison, set `VITE_BASEMAP_STYLE_URL=raster`.
- If you host your own style JSON, point `VITE_BASEMAP_STYLE_URL` at it.
- `VITE_BASEMAP_LABEL_ANCHOR_ID` is optional and only needed if your custom
  style has a nonstandard symbol stack.

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
Copernicus tiles. For the current Himachal build, fuse HMA over Copernicus
first and publish the result as `terrain_rgb_hma.mbtiles`:

```bash
./scripts/fuse_hma_copernicus_dem.sh
MIN_ZOOM=6 MAX_ZOOM=13 WORKERS=6 \
  ./scripts/dem_to_terrain_rgb.sh \
  ./data/terrain_rgb_hma.mbtiles \
  ./data/dem_fusion/fused_himachal_egm2008.tif
```

Or for a larger multi-tile region:

```bash
./scripts/download_copernicus_tiles.sh 28 76 35 82 ./data/dem_tiles

MIN_ZOOM=6 MAX_ZOOM=13 WORKERS=8 \
  ./scripts/dem_to_terrain_rgb.sh ./data/terrain_rgb.mbtiles ./data/dem_tiles/*.tif
```

### 2. Serve it from Martin

This repo's `docker-compose.yml` mounts the local `./data` directory into the
Martin container. Its entrypoint passes only the tile archives named in its
allowlist to Martin, so JSON manifests, database dumps, import files, and
unrelated archives are never mistaken for public tile sources. Placing
`./data/terrain_rgb_hma.mbtiles` there is enough. Restart Martin after the file
exists, and it will serve the tiles at:

```text
/terrain_rgb_hma/{z}/{x}/{y}
```

The web app defaults to that relative URL already, so no extra frontend
config is needed if Martin is reverse-proxied the same way as the trail
vector tiles.

If you switch to a different terrain tileset later, keep these frontend
env vars aligned with the real source:

```bash
VITE_BASEMAP_STYLE_URL=https://tiles.openfreemap.org/styles/liberty
VITE_TERRAIN_RGB_URL=/terrain_rgb_hma/{z}/{x}/{y}
VITE_TERRAIN_RGB_ENCODING=mapbox
VITE_TERRAIN_RGB_TILE_SIZE=512
VITE_TERRAIN_RGB_MAX_ZOOM=13
```

Examples:

- Vector basemap with crisp labels:
  `VITE_BASEMAP_STYLE_URL=https://tiles.openfreemap.org/styles/liberty`
- Local `rio-rgbify` output from this repo:
  `mapbox` encoding, `512` tile size, whichever max zoom you baked
- Mapterhorn / Terrarium-style terrain:
  `terrarium` encoding, `512` tile size, usually `12` as the source max zoom

### 3. Use it in the map

Open the outdoor tool rail on the right side of the map. Its `Terrain`
panel now provides:

- hillshade
- elevation tint
- client-side contour lines
- 3D terrain
- contour units (meters / feet)
- terrain exaggeration
- top-down, terrain, and low-horizon camera presets
- screen-facing contour labels plus pitch and bearing controls

The same rail also includes:

- map-layer controls for known routes, route names, ordinary paths,
  outdoor POIs, peaks/passes, and region boundaries
- coordinate-based current weather and five-day forecasts from the configured
  provider (Open-Meteo is the development default)

Production/commercial deployments should review Open-Meteo's current
usage terms and consider proxy caching.

Notes:

- Contours are generated in-browser from Terrain-RGB via
  `maplibre-contour`, so there is no separate contour-tiling pipeline.
- The build script adds an MBTiles z/x/y index. Without it, a large DEM
  pyramid can take seconds to resolve each requested tile.
- It also builds average-resolution DEM overviews before tiling, which keeps
  broad ridges cleaner and reduces resampling work at regional zooms.
- Copernicus GLO-30 is approximately 30 m source data. Baking far beyond
  z13 only adds resampled pixels and storage, not genuine terrain detail;
  MapLibre can smoothly overzoom the z13 tiles for close trail views.
- `maplibre-gl` 5.x in this repo now supports `color-relief`, so the
  terrain control can blend a subtle hypsometric tint under the trail
  network.
- The cleanest stack is now vector basemap labels on top, trail linework
  below those labels, and terrain shading below the trail casing.

## What's deliberately not here yet

- User accounts / community content -- out of scope for now
- SOI licensed data -- phase 2, pending the licensing/entity-type
  question flagged earlier

The hiking-focused discovery, sharing and data-consistency changes are described in
[App improvements](docs/app-improvements.md), including migrations 0018–0019 and validation.
