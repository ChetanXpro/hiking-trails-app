# Step-by-Step Refactoring Guide

## Quick Start

This guide provides concrete, actionable steps to refactor the codebase incrementally.

---

## Phase 1: Frontend (Web) - Immediate Impact

### Why This First?
- TrailMap.svelte is 5,374 lines (impossible to maintain)
- Each extraction is independent
- Can test immediately in browser
- Low risk of breaking things

### Step 1: Extract MapConfigService (DONE ✅)

**Status**: Complete - `web/src/lib/components/MapConfigService.ts` created

This extracts all configuration loading and constants from TrailMap.svelte.

**Impact**: Reduces TrailMap.svelte by ~200 lines

**Next**: Use it in TrailMap.svelte

```typescript
// In TrailMap.svelte, replace:
const CONFIGURED_BASEMAP_STYLE_URL = publicSetting(
  'PUBLIC_BASEMAP_STYLE_URL',
  import.meta.env.VITE_BASEMAP_STYLE_URL
);
// ... 100 lines of config loading

// With:
import { TILE_URLS, TERRAIN_CONFIG, LAYER_IDS, MAP_CONFIG } from './components/MapConfigService';
const basemapUrl = TILE_URLS.getBasemapStyleUrl(import.meta.env.VITE_BASEMAP_STYLE_URL);
```

---

### Step 2: Extract TerrainControls Component (HIGH VALUE)

**Estimated size**: 200-250 lines  
**Current location in TrailMap.svelte**: Lines 3420-3450 (UI) + related state

**What to extract**:
```typescript
// State
let terrainContoursVisible: boolean;
let terrainHillshadeVisible: boolean;
let terrainColorReliefVisible: boolean;
let terrainSlopeVisible: boolean;
let terrainSlopeOpacity: number;
let terrainContourUnit: ContourUnit;

// Event handlers
const setTerrainContoursEnabled = (enabled: boolean) => { ... };
const setTerrainHillshadeEnabled = (enabled: boolean) => { ... };
const setTerrainColorReliefEnabled = (enabled: boolean) => { ... };
// ... more handlers

// UI
<label class="terrain-switch"><input ... /></label>
// ... more terrain UI elements
```

**Create file**: `web/src/lib/components/controls/TerrainControls.svelte`

**Make it a component**:
```svelte
<script lang="ts">
  let {
    contoursVisible = false,
    hillshadeVisible = false,
    colorReliefVisible = false,
    slopeVisible = false,
    slopeOpacity = 1,
    contourUnit = 'meters',
    onContourToggle,
    onHillshadeToggle,
    onColorReliefToggle,
    onSlopeToggle,
    onSlopeOpacityChange,
    onContourUnitChange,
    terrainReady = false,
    mapZoom = 9
  }: Props = $props();

  // Component-specific state
  let expandedSection = 'none';
</script>

<div class="terrain-controls">
  {/* existing terrain UI from TrailMap.svelte */}
</div>

<style>
  {/* existing terrain styles */}
</style>
```

**Update TrailMap.svelte**:
```svelte
<script>
  import TerrainControls from './components/controls/TerrainControls.svelte';
  
  let terrainContoursVisible = false;
  // ... other terrain state
  
  const handleContourToggle = (enabled: boolean) => {
    terrainContoursVisible = enabled;
    setContoursVisible(map, enabled);
  };
  // ... other handlers
</script>

<TerrainControls
  contoursVisible={terrainContoursVisible}
  hillshadeVisible={terrainHillshadeVisible}
  onContourToggle={handleContourToggle}
  onHillshadeToggle={handleHillshadeToggle}
  terrainReady={terrainReady}
  mapZoom={mapZoom}
/>
```

**Time**: 1-2 hours  
**Impact**: -250 lines from TrailMap.svelte

---

### Step 3: Extract TrailDetailPanel Component (HIGH VALUE)

**Estimated size**: 500-600 lines  
**Current location**: Lines ~1500-2100 and ~3500+

**What to extract**:
- Trail name, stats (distance, elevation, time)
- Description
- Difficulty badge
- Weather widget
- Share/export buttons
- Navigation breadcrumbs

**Create file**: `web/src/lib/components/sidebar/TrailDetailPanel.svelte`

**Status**: Selected trail info + actions

**Make it a component**:
```svelte
<script lang="ts">
  let { trail, selectedTrailProfile }: Props = $props();
  
  let { onExport, onShare, onCenter }: Events = $props();
</script>

{/* existing trail detail UI */}
```

**Time**: 2-3 hours  
**Impact**: -400 lines from TrailMap.svelte

---

### Step 4: Extract Map Controls Component (MEDIUM VALUE)

**Estimated size**: 150-200 lines  
**Current location**: Lines 3300-3350 (right sidebar buttons)

**What to extract**:
- Zoom in/out buttons
- Fullscreen toggle
- Center button
- 3D toggle
- Layer selector

**Create file**: `web/src/lib/components/controls/MapControls.svelte`

**Time**: 1-1.5 hours  
**Impact**: -150 lines from TrailMap.svelte

---

### Step 5: Extract TrailSearchPanel Component (MEDIUM-HIGH VALUE)

**Estimated size**: 400-500 lines  
**Current location**: Lines 1000-1200 and scattered handlers

**What to extract**:
- Search input
- Discovery list
- Filter/sort options
- Trail filtering logic

**Create file**: `web/src/lib/components/sidebar/TrailSearchPanel.svelte`

**Time**: 2-3 hours  
**Impact**: -300 lines from TrailMap.svelte

---

### Step 6: Extract Map3DView Component (MEDIUM VALUE)

**Estimated size**: 200-250 lines  
**Current location**: Lines 3100-3150 (3D controls)

**What to extract**:
- Pitch slider
- Bearing controls
- Camera preset buttons (horizon, alpine)
- Exaggeration slider

**Create file**: `web/src/lib/components/map/Map3DView.svelte`

**Time**: 1-2 hours  
**Impact**: -200 lines from TrailMap.svelte

---

## Running Total: Frontend Refactoring

| Step | Component | Size | Removed | TrailMap Left |
|------|-----------|------|---------|---------------|
| 1 | MapConfigService | +200 | 200 | ~5,174 |
| 2 | TerrainControls | +250 | 250 | ~4,924 |
| 3 | TrailDetailPanel | +500 | 400 | ~4,524 |
| 4 | MapControls | +150 | 150 | ~4,374 |
| 5 | TrailSearchPanel | +400 | 300 | ~4,074 |
| 6 | Map3DView | +200 | 200 | ~3,874 |

**After Phase 1A**: TrailMap.svelte down to ~3,900 lines (28% reduction)

**Further reductions**: Continue with MapCore, MapLayers, ExportPanel, etc.

**Goal**: Get TrailMap.svelte < 500 lines (91% reduction)

---

## Phase 2: Backend - Rust

### Refactoring hiking-import (Priority: HIGH)

**Current**: 4,325 lines in main.rs  
**Problem**: Mixed concerns (config, relation processing, elevation, regions, places, etc.)

#### Step 1: Extract Region Sync Service

**Create**: `crates/hiking-import/src/service/region_sync.rs`

Extract from main.rs:
```rust
async fn sync_regions(pool: &sqlx::PgPool) -> Result<()> { ... }
async fn sync_country_bounds(pool: &sqlx::PgPool) -> Result<()> { ... }
```

**Impact**: -200 lines from main.rs

#### Step 2: Extract Place Sync Service

**Create**: `crates/hiking-import/src/service/place_sync.rs`

Extract:
```rust
async fn sync_places(pool: &sqlx::PgPool) -> Result<()> { ... }
```

**Impact**: -150 lines from main.rs

#### Step 3: Extract Waypoint Sync Service

**Create**: `crates/hiking-import/src/service/waypoint_sync.rs`

Extract:
```rust
async fn sync_outdoor_waypoints(pool: &sqlx::PgPool) -> Result<()> { ... }
```

**Impact**: -100 lines from main.rs

#### Step 4: Extract Elevation Module

**Create**: `crates/hiking-import/src/importer/elevation.rs`

Extract:
```rust
struct ElevationSampler { ... }
impl ElevationSampler {
    fn sample_route(...) { ... }
    // 500+ lines of elevation logic
}
```

**Impact**: -500 lines from main.rs

#### Step 5: Extract Relation Processing

**Create**: `crates/hiking-import/src/importer/relation.rs`

Extract:
```rust
async fn stitch_one_relation(...) -> Result<StitchOutcome> { ... }
// Related helpers
```

**Impact**: -400 lines from main.rs

#### Step 6: Extract Models

**Create**: `crates/hiking-import/src/models/` directory

Extract:
```rust
// models/relation.rs
struct RawRelationRow { ... }
struct RawWayRow { ... }
// etc.

// models/elevation.rs
struct ElevationPoint { ... }
struct ComputedElevationProfile { ... }
// etc.
```

**Impact**: -200 lines from main.rs

---

## Time Estimates

| Phase | Component | Time | Difficulty |
|-------|-----------|------|------------|
| 1A | Frontend (6 components) | 8-10 hours | Medium |
| 1B | Frontend (remaining) | 4-5 hours | Medium |
| 2A | Backend (3 services) | 4-5 hours | Hard |
| 2B | Backend (importer modules) | 5-6 hours | Hard |

**Total**: 21-26 hours  
**Estimated**: 3-4 days of focused work

---

## Refactoring Checklist

For each extraction:

- [ ] Create new file with extracted code
- [ ] Add proper imports/exports
- [ ] Define component props/events (for Svelte)
- [ ] Update source file with imports
- [ ] **Test in browser** (if frontend)
- [ ] **Run tests** (if backend)
- [ ] Commit with message: `refactor: extract <name> <type>`
- [ ] Verify no regressions

---

## Commit Messages

```bash
# Good
git commit -m "refactor: extract TerrainControls component"
git commit -m "refactor: extract region_sync service module"

# Bad
git commit -m "refactor"
git commit -m "split files"
```

---

## Rollback Plan

If something breaks:

```bash
# Before extraction
git stash

# Return to working state
git reset --hard HEAD~1

# Try again with more conservative changes
```

---

## Next: Start Extracting

Ready to begin?

1. **Pick Step 2** (TerrainControls) - high impact, low complexity
2. Create the file
3. Copy code from TrailMap.svelte
4. Test in browser
5. Commit
6. Move to Step 3

**Expected outcome after Phase 1A**: Much more readable, maintainable codebase with logical separation of concerns.
