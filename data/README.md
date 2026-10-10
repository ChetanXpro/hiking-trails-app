This directory is for downloaded and generated local geodata.

Typical contents:
- `dem_tiles/` from `./scripts/download_copernicus_tiles.sh`
- `terrain_rgb_hma.mbtiles` from the fused HMA + Copernicus terrain workflow
- `hp.osm.pbf` and `hp-trails.osm.pbf` from `./scripts/build_himachal_trails.sh`

For the current importer, use `hp.osm.pbf` with `osm2pgsql`. The smaller
`hp-trails.osm.pbf` is only a legacy trails-only extract and does not
contain enough admin/place data for hierarchical search.

`build_himachal_trails.sh` creates `hp.osm.pbf` with reference-complete
ordinary hiking routes and feature relations. State-level imports intentionally
do not publish country-scale OSM `superroute` relations.

These files are intentionally gitignored because they are large and
machine-local.

The terrain build script emits 512px Mapbox Terrain-RGB tiles and adds a
`tile_index` for fast Martin lookups. Keep the frontend tile-size and max-zoom
settings aligned with the values used when the MBTiles file was generated.
