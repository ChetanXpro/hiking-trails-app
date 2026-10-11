# Hiking-focused app changes

The main view now focuses on discovering a hike, understanding its mapped route,
and sharing or exporting it. The existing `/qa` view keeps reconstruction
diagnostics for maintainers.

## Interface

- **Explore hikes in this area** searches the current map viewport. Filters cover
  distance, ascent, SAC difficulty and loops; results default to the start points
  nearest the map center. Results are paginated, highlight their route on hover
  or keyboard focus, and explicitly identify partial routes.
- The route overview puts distance, ascent, descent, estimated time and the
  elevation profile first. Weather and nearby places can be expanded. Mapper
  attribution is a simple line with a source link; there are no account or
  contributor-profile screens. The older contributor database was already
  removed by migration 0013.
- Shared links retain the route and map view. Reload and browser Back/Forward
  restore that state. Sharing uses native sharing when available, clipboard
  otherwise, with a selectable link available if either is blocked.
- The quick GPX button always requests GPX, even after another format was chosen
  in the advanced export controls.
- The elevation chart supports mouse, touch and keyboard input. A cumulative
  distance index and binary search locate the matching map position without
  rescanning every vertex or bridging disconnected geometry parts.
- Fine contour lines are lighter; index contours remain prominent. Labels can
  follow tighter curves and respect label collisions. Terrain controls report
  the currently visible contour interval and unit.
- Nearby waypoints are existing mapped places within 150 m, with map focus
  controls. Their presence does not imply that they lie directly on the path.

## Data consistency

Migration **0018** invalidates elevation profiles, ascent, descent, duration and
land cover when the ordered route geometry changes, including route reversal.
It also removes legacy elevation profiles whose final distance differs from
the route by more than 100 m or 5%, whichever is greater. That repair identifies
incompatible lengths; it cannot prove that every historical same-length profile
used the right geometry.

The importer now recomputes durations from retained or newly sampled elevation,
does not assume a flat route when elevation is absent, and rejects DEM profiles
with missing endpoints, more than 5% missing samples, or an interior gap spanning
more than 120 m. Small interior gaps retain the existing interpolation and
smoothing. OSM mapper `note` tags no longer become public route descriptions.
Equal-size disconnected route components are selected deterministically.

Migration **0019** adds a published `cautions` projection. Imports derive access,
conditional access, seasonal, poor trail visibility and ford notes from the
actual included OSM ways, with source links. These are mapped facts, not live
closure reports. Absence of a note is not an assessment of current conditions.
The serving API remains independent of raw staging tables.

Trail details carry a `data_revision`. Elevation and download requests include
that revision; the API returns an uncached **409** if the route has changed.
Revision-specific URLs keep cached profiles and exports paired with the
corresponding overview. Region listings now include descendant regions.

## Running the update

Apply migrations before starting the new API or importer:

```sh
cargo run --locked -p hiking-db --bin hiking-migrate
# Configure DEM_PATH or DEM_PATHS to the existing local elevation rasters.
cargo run --locked -p hiking-import
```

Regenerate the deployment bootstrap dump after rebuilding data; the old dump
will still contain the previous profiles and won't contain the new hiking notes.
This change does not download a newer OSM extract or WorldCover dataset.

## Validation on the local dataset

- 318 trails rebuilt from existing staged OSM and the fused HMA/Copernicus DEM;
  318 profiles, zero distance/profile mismatches (previously 30), 16 routes with
  hiking notes. This corrects derived data, not gaps in the source route mapping.
- Recursive listing for Himachal Pradesh returned 306 trails (previously zero
  when querying the parent region directly).
- Rust workspace tests, frontend tests, Svelte checking and production build.
- Read-only API checks covered pagination, ordering, filters, invalid inputs,
  antimeridian bounds, revision conflicts and all nine export format/direction
  combinations. GPX/KML were parsed as XML and GeoJSON as JSON.
- `tests/sql/route_enrichment.sql` verifies the actual database trigger using a
  fixture inside a rolled-back transaction, including same-length reversal.
- Browser checks cover route links, sharing fallback, history, chart keyboard
  input, discovery and desktop/phone layouts.
- The preview browser did not report a download event for the GPX button, so
  saving a file through that browser remains unverified; export responses were
  checked independently as described above.

## Implementation references

- [maplibre-contour](https://github.com/onthegomap/maplibre-contour): existing
  worker-based DEM contour generation and zoom thresholds remain appropriate.
- [MapLibre symbol layout](https://maplibre.org/maplibre-style-spec/layers/):
  contour curvature and collision behavior.
- [SvelteKit shallow routing](https://svelte.dev/docs/kit/shallow-routing): use
  the installed SvelteKit 2 navigation helpers for history updates.
- OSM definitions for [pedestrian access](https://wiki.openstreetmap.org/wiki/Key:foot)
  and [trail visibility](https://wiki.openstreetmap.org/wiki/Key:trail_visibility).
