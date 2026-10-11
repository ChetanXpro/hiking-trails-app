# Codebase Refactoring Plan

## Problem

Large monolithic files make the codebase difficult to understand and maintain:
- **TrailMap.svelte**: 5,374 lines (1 file)
- **hiking-import/main.rs**: 4,325 lines 
- **hiking-api/main.rs**: 2,503 lines
- **terrain-layers.ts**: 870 lines

## Solution: Modular Architecture

---

## Phase 1: Frontend (Web) - Priority: HIGH

### Current Problem
**TrailMap.svelte** (5,374 lines) handles:
- Map initialization
- Layer management (trails, terrain, peaks, POIs, slopes)
- UI panels (search, trail detail, elevation chart, weather)
- State management
- Event handlers
- 3D mode toggle
- Export functionality
- Etc. (everything!)

### Proposed Structure

```
web/src/lib/
├── components/
│   ├── TrailMap.svelte                 # Main wrapper (200-300 lines)
│   ├── map/
│   │   ├── MapCore.svelte             # Map instance, zoom/pan
│   │   ├── MapLayers.svelte           # Layer management (trails, terrain)
│   │   └── Map3DView.svelte           # 3D terrain & pitch controls
│   ├── sidebar/
│   │   ├── Sidebar.svelte             # Container
│   │   ├── TrailSearchPanel.svelte    # Search & discovery
│   │   ├── TrailDetailPanel.svelte    # Trail info, stats
│   │   └── ElevationChart.svelte      # (move here, already exists)
│   └── controls/
│       ├── MapControls.svelte         # Zoom, pan, fullscreen
│       ├── TerrainControls.svelte     # 3D, contours, hillshade
│       └── ExportPanel.svelte         # GPX, KML, JSON export
├── services/
│   ├── map-state.ts                  # (already exists, keep here)
│   ├── terrain-service.ts            # Terrain layer operations
│   ├── trail-service.ts              # Trail queries & display
│   ├── poi-service.ts                # Peak/POI/waypoint handling
│   └── export-service.ts             # GPX/KML/JSON generation
├── utils/
│   ├── map-selection.ts              # (already exists)
│   ├── trail-utils.ts                # Difficulty, distance calc
│   └── terrain-utils.ts              # Elevation sampling helpers
└── config/
    ├── terrain-config.ts             # (already exists)
    ├── basemap-config.ts             # (already exists)
    ├── map-constants.ts              # Constants (zoom levels, etc)
    └── layer-defaults.ts             # Default layer configs
```

### Files to Create

1. **MapCore.svelte** (200 lines)
   - Maplibre initialization
   - Zoom/pan/bearing management
   - Basemap loading

2. **MapLayers.svelte** (300 lines)
   - Add/remove/toggle layers
   - Trail rendering
   - Terrain layer management

3. **Map3DView.svelte** (250 lines)
   - 3D terrain toggle
   - Pitch/roll controls
   - Exaggeration slider

4. **TrailSearchPanel.svelte** (400 lines)
   - Search input
   - Discovery list
   - Filter/sort UI

5. **TrailDetailPanel.svelte** (500 lines)
   - Trail name/stats
   - Description
   - Difficulty badge
   - Weather widget
   - Share/export buttons

6. **MapControls.svelte** (150 lines)
   - Zoom buttons
   - Fullscreen toggle
   - Center button

7. **TerrainControls.svelte** (200 lines)
   - Contour toggle
   - Hillshade toggle
   - Color relief toggle
   - Slope opacity slider

8. **ExportPanel.svelte** (200 lines)
   - Format selection (GPX/KML/JSON)
   - Download button
   - Copy link

9. **terrain-service.ts** (300 lines)
   - Terrain layer setup
   - DEM source management
   - Contour configuration
   - (Move from terrain-layers.ts functions)

10. **trail-service.ts** (250 lines)
    - Trail layer rendering
    - Trail selection logic
    - Trail filtering

11. **export-service.ts** (200 lines)
    - GPX/KML/JSON generation
    - Download handling

---

## Phase 2: Backend - Rust

### hiking-import (Priority: HIGH)

**Current**: 4,325 lines in main.rs

**Proposed structure**:
```
crates/hiking-import/src/
├── main.rs                      (200 lines - entry point only)
├── lib.rs                       (exports)
├── importer/
│   ├── mod.rs                  (module definition)
│   ├── relation.rs             (relation processing, ~400 lines)
│   ├── elevation.rs            (elevation sampling, DEM, ~500 lines)
│   ├── landcover.rs            (landcover/terrain, ~300 lines)
│   └── waypoint.rs             (waypoint discovery, ~200 lines)
├── models/
│   ├── mod.rs
│   ├── relation.rs             (relation structures)
│   ├── trail.rs                (trail output model)
│   └── geometry.rs             (elevation profile, geometry)
├── validation/
│   ├── mod.rs
│   ├── url_validator.rs        (URL validation - 50 lines)
│   └── relation_validator.rs   (relation checks - 100 lines)
├── service/
│   ├── mod.rs
│   ├── region_sync.rs          (~200 lines)
│   ├── waypoint_sync.rs        (~150 lines)
│   ├── place_sync.rs           (~200 lines)
│   └── manifest.rs             (original mapper manifest - ~200 lines)
└── utils/
    ├── mod.rs
    ├── distance.rs             (distance calculation)
    ├── hash.rs                 (source_hash computation)
    └── logging.rs              (structured logging helpers)
```

### hiking-api (Priority: MEDIUM)

**Current**: 2,503 lines in main.rs

**Proposed structure**:
```
crates/hiking-api/src/
├── main.rs                      (100 lines - server setup)
├── lib.rs                       (exports)
├── handlers/
│   ├── mod.rs
│   ├── trails.rs               (GET /trails, detail, etc)
│   ├── search.rs               (POST /search)
│   ├── export.rs               (GET /trails/:id/export)
│   ├── discovery.rs            (~200 lines, already exists)
│   └── health.rs               (readiness, liveness)
├── models/
│   ├── mod.rs
│   ├── trail.rs                (TrailSummary, TrailDetail)
│   ├── search.rs               (SearchRequest, SearchResult)
│   └── export.rs               (ExportFormat, ExportResult)
├── export/
│   ├── mod.rs
│   ├── geojson.rs              (~300 lines)
│   ├── gpx.rs                  (~300 lines)
│   ├── kml.rs                  (~300 lines)
│   └── bounds.rs               (export format-specific helpers)
├── query/
│   ├── mod.rs
│   ├── trail_detail.rs         (detail query building)
│   ├── search.rs               (search query building)
│   └── filters.rs              (filter application)
└── utils/
    ├── mod.rs
    ├── policy.rs               (request policy, semaphores)
    └── format.rs               (format helpers, JSON, etc)
```

---

## Phase 3: Database/Core Library

### hiking-db (Priority: LOW)

**Current**: 206 lines in models.rs + 117 lines in lib.rs = well-organized  
**Status**: Already good, no major refactoring needed

Could add:
```
crates/hiking-db/src/
├── lib.rs
├── models.rs (keep as is)
└── queries/
    ├── mod.rs
    ├── trails.rs              (SQL builders)
    └── geometry.rs            (WKB, spatial)
```

But not urgent.

---

## Implementation Priority

### Phase 1A (Frontend - Immediate)
**Goal**: Make TrailMap.svelte < 500 lines

1. Extract MapCore.svelte (highest impact)
2. Extract MapLayers.svelte  
3. Extract TerrainControls.svelte
4. Extract TrailDetailPanel.svelte

**Time**: 3-4 hours  
**Impact**: Huge - TrailMap becomes readable

### Phase 1B (Frontend - Follow-up)
Extract remaining components (search, export, etc)

**Time**: 2-3 hours

### Phase 2A (Backend - High Value)
**Goal**: Make hiking-import/main.rs < 800 lines

1. Extract `importer/relation.rs` (relation processing)
2. Extract `importer/elevation.rs` (largest file)
3. Extract `service/region_sync.rs`

**Time**: 4-5 hours  
**Impact**: Code becomes maintainable

### Phase 2B (Backend - Follow-up)
Extract remaining modules

**Time**: 3-4 hours

### Phase 3 (Optional)
hiking-api refactoring (lower priority, still reasonable)

---

## Refactoring Guidelines

### Svelte Components
- Keep parent components < 500 lines
- Extract child components for features > 200 lines
- Use props for data passing
- Use events for communication upward
- Keep logic in `<script context="module">` separate

### Rust Modules
- Separate public API from implementation
- Use `mod.rs` for module organization
- Re-export public types in parent `mod.rs`
- Keep files < 500 lines (exceptions allowed for data-heavy files)
- Use meaningful module names that describe responsibility

### TypeScript/JavaScript
- One exported interface per file (or closely related group)
- Utilities files < 300 lines
- Services < 400 lines
- Keep tests with implementations

---

## Benefits of Refactoring

| Metric | Before | After |
|--------|--------|-------|
| **Largest file** | 5,374 lines | < 500 lines |
| **Average file size** | 650 lines | 200-300 lines |
| **Time to understand a feature** | 30+ minutes | 5-10 minutes |
| **Ease of adding features** | Hard | Easy |
| **Test coverage** | Difficult to test | Testable modules |
| **Reusability** | Low (monolith) | High (components) |

---

## Migration Strategy

### Low Risk
1. Create new file structure alongside old files
2. Extract functionality incrementally
3. Run tests after each extraction
4. Commit after each major change
5. Keep old file until all refs updated
6. Delete old file once verified

### Example: Extract MapCore.svelte

```bash
# Step 1: Create new file with extracted code
cp web/src/lib/TrailMap.svelte web/src/lib/components/MapCore.svelte
# Edit MapCore.svelte to be < 300 lines

# Step 2: Update TrailMap.svelte to import MapCore
# <script>
#   import MapCore from './components/MapCore.svelte';
# </script>

# Step 3: Test in browser
npm run dev

# Step 4: Commit
git add . && git commit -m "refactor: extract MapCore component"

# Repeat for each component
```

---

## Post-Refactoring

Once refactored:
1. Add better tests for extracted modules
2. Document component responsibilities
3. Create ARCHITECTURE.md with module diagram
4. Establish naming conventions
5. Add linting rules for file size

---

## Timeline

- **Phase 1A**: 3-4 hours → Major readability improvement
- **Phase 1B**: 2-3 hours → Complete frontend
- **Phase 2A**: 4-5 hours → Backend maintainability
- **Phase 2B**: 3-4 hours → Polish

**Total**: 12-16 hours (2-3 days of focused work)

**ROI**: Dramatically improved code quality and maintainability
