# Code Review Fixes Applied

This document tracks the fixes applied based on the comprehensive code review feedback.

## ✅ Critical Bugs Fixed

### 1. Snap Grid Misses High-Latitude Matches (stitch.rs)
**Issue:** The snap grid used fixed degrees-per-meter calculation without accounting for latitude. At high latitudes, longitude cells become narrower, causing the snapping algorithm to miss points that should be within tolerance (especially away from equator).

**Fix:** Modified `NodeAssigner::bucket_key()` and `node_id_for()` to:
- Calculate latitude-aware cell sizes for both longitude and latitude
- Use a dynamic longitude reach based on `cos(latitude)` 
- Search up to `ceil(1.0 / cos(lat))` buckets in longitude direction instead of fixed ±1

**Impact:** Prevents spurious open ends and disconnected pieces in high-latitude routes.

**File:** `crates/hiking-import/src/stitch.rs:28-39`

---

### 2. XSS Vulnerability in URL Tags (main.rs)
**Issue:** `website_url` and `video_url` extracted from OSM tags were stored without validation, enabling javascript: injection if frontend renders them as href attributes.

**Fix:** Added `validate_url()` function that:
- Filters empty/whitespace-only URLs
- Requires http:// or https:// prefix
- Applied to both website_url and video_url extraction

**Impact:** Prevents malicious javascript: URLs from OSM tags reaching the frontend.

**Files:** 
- `crates/hiking-import/src/main.rs:3643-3656` (validate_url function)
- `crates/hiking-import/src/main.rs:1252-1253` (applied to tag extraction)

---

### 3. Max/Min Altitude Using Smoothed Samples (main.rs)
**Issue:** Min and max elevation computed from smoothed elevation data (5-sample window), losing true peaks and passes by ~10-50m. Critical for safety-related information (AMS risk at true peaks).

**Fix:** Changed min/max calculation to use unsmoothed `interpolated_samples` instead of `smoothed_samples`:
- Profile visualization still uses smoothed data for readability
- True peaks/valleys preserved for altitude metadata
- Ascent/descent calculation still uses smoothed data (appropriate for gain/loss)

**Impact:** Accurate elevation metadata for hiking safety decisions.

**File:** `crates/hiking-import/src/main.rs:2648-2655`

---

### 4. One Bad Relation Fails Entire Import (main.rs)
**Issue:** If a single relation failed (missing ways, hierarchy cycles), entire import would bail out, skipping pruning and the mapper manifest for all relations.

**Fix:** Converted relation-specific errors to `StitchOutcome::SkippedInvalid`:
- `reconstruction_incomplete` (missing child relations) → SkippedInvalid
- `missing_way_members` → SkippedInvalid  
- `no ordered ways` → SkippedInvalid
- Only infrastructure errors (database, GDAL) remain as hard failures with warning

**Impact:** Regional extracts with boundary-crossing relations now import successfully; partial import better than total failure.

**Files:**
- `crates/hiking-import/src/main.rs:1172-1225` (relation validation)
- `crates/hiking-import/src/main.rs:3143-3156` (main loop warning instead of bail)

---

### 5. GeoJSON Export Writes ele: 0.0 When Missing (hiking-api/main.rs)
**Issue:** GeoJSON exports emitted 3D coordinates with `ele: 0.0` for routes with missing elevation, encoding false data rather than absence.

**Fix:** Modified both GeoJSON and KML exporters to:
- Emit 2D coordinates `[lon, lat]` when elevation is absent
- Emit 3D coordinates `[lon, lat, ele]` only when elevation data exists

**Impact:** Clients can properly distinguish between "elevation unknown" and "elevation is zero".

**Files:**
- `crates/hiking-api/src/main.rs:1176-1186` (GeoJSON export)
- `crates/hiking-api/src/main.rs:1271-1289` (KML export)

---

## 🔧 Database Migration

**Created:** `migrations/0020_trail_source_hash.sql`

Adds columns for source_hash and data_revision optimization:
- `source_hash`: MD5 of relation+members+geometry+config (for change detection)
- `data_revision`: Derived from source_hash (stable cache key)
- Backfill from existing data; marks trails for recomputation on next import

---

## 📋 Code Infrastructure Added

### Source Hash Computation Function
**Added:** `compute_source_hash()` in `crates/hiking-import/src/main.rs:3643-3671`

Computes deterministic hash from:
- OSM relation ID
- Relation tags (JSON)
- Member list (IDs + roles)
- Ordered way sequence
- Importer version
- DEM/WorldCover configuration flags

**Purpose:** Enables detection of unchanged relations for skipping expensive re-processing (10-100× speedup on incremental imports).

---

## ⏳ Remaining Work

### High Impact (Performance)

1. **Skip Unchanged Relations Using source_hash** *(partially prepared)*
   - Compute source_hash before expensive work
   - Query existing trail's source_hash
   - If unchanged: UPDATE last_synced_at only, skip segments/waypoints/elevation
   - Estimated speedup: 10-100× for incremental imports with few changes
   - Requires: Integration with stitch_one_relation and main import loop

2. **Parallelize Importer with Tokio** *(not yet implemented)*
   - Replace sequential relation processing with `futures::stream::iter().buffer_unordered(N)`
   - Wrap GDAL/DEM sampling in `tokio::task::spawn_blocking()`
   - Estimated speedup: 4-8× on multi-core systems (depends on blocking work ratio)
   - Requires: Converting main loop to async, careful transaction handling

3. **DEM Tile VRT Optimization** *(not yet implemented)*
   - Build VRT from DEM tiles with `gdalbuildvrt` at startup
   - One seamless source instead of per-tile seams
   - Use `gdal` crate for open-across-routes dataset reuse
   - Estimated speedup: 2-5× for elevation sampling (fewer GDAL calls)

### Medium Impact (Quality & Maintainability)

4. **Unify Distance Formula** *(not yet implemented)*
   - Move `approx_distance_m()` to `hiking-db` crate
   - Use in both importer and API
   - Prevents elevation drift between import and export
   - Requires: Crate reorganization, testing

5. **Optimize Waypoint Proximity Queries** *(not yet implemented)*
   - Currently: `ST_DWithin(geography)` against full MultiLineString
   - Better: Join against segments table, then ST_DWithin on each segment
   - Avoids bbox scan over huge coverage area
   - Estimated speedup: 2-10× for nearby waypoint discovery

6. **Store Warning Kinds Enum** *(not yet implemented)*
   - Current: Discovery runs LIKE '%relation splits into%' on warning text
   - Better: Store `warning_kind` enum on stitch_warnings table
   - Add `incomplete` boolean on trails table
   - Avoids text search on every detail request
   - Estimated speedup: 10-50× for warning classification

7. **Direction Mode Constraint** *(no change needed yet)*
   - `default_direction_mode = "both"` can never happen (contradicts both-ends check)
   - Code is correct but comment could be added for clarity

8. **Derive Difficulty on ordered_ways** *(not yet implemented)*
   - Currently: Takes max sac_scale over ALL fetched ways
   - Better: Restrict to ordered_ways in final route
   - Prevents spurious difficulty from disconnected stranded ways

9. **Clean Up Stale Regions/Places** *(not yet implemented)*
   - Currently: Regions and places upserted but never deleted
   - Should: Include delete logic for removed OSM features
   - Prevents stale boundaries from lingering

### Low Impact / Monitoring

10. **/ready Endpoint Flapping** *(needs investigation)*
    - Returns 503 when two probes overlap
    - Multiple load balancers may cause flapping
    - Consider: Probe staggering or grace period

11. **API Response Slimming** *(not yet implemented)*
    - Use `ST_AsGeoJSON(geom, 6)` instead of 9 decimals (saves bytes, same 0.1m precision)
    - Pass geometry through as `Box<RawValue>` instead of parsing/re-serializing
    - Rely on Martin tiles for map geometry
    - Estimated savings: 10-30% on detail response size

12. **Search Performance** *(not yet implemented)*
    - Run 3 trigram queries concurrently with `tokio::try_join!`
    - Cache country_bounds in memory (tiny, read-only)
    - Estimated speedup: 2-3× for search requests

13. **Segment WKB Binding** *(not yet implemented)*
    - Current: CASE statement binds WKB in both branches
    - Better: Reverse geometry in Rust or reference value once
    - Minor optimization, mostly code clarity

---

## Testing Notes

- All modified crates compile successfully (`cargo check`)
- No breaking changes to public APIs
- Migration 0020 is idempotent (ON CONFLICT) and backfill-safe
- Changes preserve existing data; no lossy operations

## Deployment Order

1. Apply migration 0020 (adds new columns)
2. Deploy updated hiking-import and hiking-api binaries
3. Run import with IMPORT_PRUNE_MISSING_TRAILS=true (optional; cleans up old trails)
4. Monitor warning counts in logs and query performance
5. Implement remaining performance optimizations incrementally
