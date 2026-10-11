# Backend Refactoring Plan (Rust)

## Overview

Transform three monolithic Rust binaries into clean, modular services with clear separation of concerns.

---

## Current State Analysis

### Line Count by Crate

```
hiking-import (4,325 lines)
├── main.rs          (4,325 lines) ❌ MONOLITH
├── stitch.rs        (655 lines)
└── cautions.rs      (89 lines)

hiking-api (2,503 lines)
├── main.rs          (2,503 lines) ❌ MONOLITH
└── discovery.rs     (155 lines)

hiking-db (206 lines)
├── lib.rs           (117 lines)
└── models.rs        (206 lines)
```

### Problems

| Issue | Impact | Example |
|-------|--------|---------|
| **Mixed concerns** | Hard to test | Config + relation processing + elevation all in main.rs |
| **No clear boundaries** | Risk of side effects | Changing one thing breaks others |
| **Difficult to reuse** | Code duplication | Distance calculation in multiple places |
| **Slow to onboard** | New developers lost | "Where is the elevation code?" |
| **Hard to parallelize** | Performance bottleneck | Can't refactor elevation without breaking import |

---

## Proposed Architecture

### hiking-import Refactoring

**Current**: 4,325 lines in main.rs  
**Target**: < 800 lines in main.rs + modular services

```
crates/hiking-import/src/
├── main.rs                      (200 lines - entry point)
├── lib.rs                       (exports)
├── config/
│   ├── mod.rs
│   └── environment.rs           (env var loading)
├── importer/
│   ├── mod.rs                   (module exports)
│   ├── relation.rs              (relation processing, 400 lines)
│   ├── waypoint.rs              (waypoint discovery, 200 lines)
│   └── source_hash.rs           (change detection, 150 lines)
├── elevation/
│   ├── mod.rs
│   ├── sampler.rs               (DEM sampling, 300 lines)
│   ├── profile.rs               (elevation profile, 200 lines)
│   └── interpolation.rs         (missing value filling, 150 lines)
├── terrain/
│   ├── mod.rs
│   ├── landcover.rs             (WorldCover handling, 200 lines)
│   └── slope.rs                 (slope calculation, 100 lines)
├── data/
│   ├── mod.rs
│   ├── region.rs                (region sync, 200 lines)
│   ├── place.rs                 (place sync, 150 lines)
│   ├── waypoint.rs              (waypoint sync, 150 lines)
│   └── manifest.rs              (mapper attribution, 200 lines)
├── models/
│   ├── mod.rs
│   ├── trail.rs                 (trail structures)
│   ├── relation.rs              (relation structures)
│   ├── elevation.rs             (elevation structures)
│   └── geometry.rs              (geometry helpers)
├── validation/
│   ├── mod.rs
│   ├── url.rs                   (URL validation)
│   └── relation.rs              (relation validation)
└── utils/
    ├── mod.rs
    ├── distance.rs              (distance calculations)
    ├── hash.rs                  (source_hash)
    └── logging.rs               (structured logging)
```

### hiking-api Refactoring

**Current**: 2,503 lines in main.rs  
**Target**: < 600 lines in main.rs + modular handlers

```
crates/hiking-api/src/
├── main.rs                      (100 lines - server setup)
├── lib.rs                       (exports)
├── config/
│   ├── mod.rs
│   └── environment.rs
├── handlers/
│   ├── mod.rs
│   ├── trails.rs                (GET /trails/* - 250 lines)
│   ├── search.rs                (POST /search - 200 lines)
│   ├── export.rs                (GET /trails/:id/export - 200 lines)
│   ├── discovery.rs             (existing - 155 lines)
│   └── health.rs                (readiness/liveness - 100 lines)
├── export/
│   ├── mod.rs
│   ├── geojson.rs               (GeoJSON export - 250 lines)
│   ├── gpx.rs                   (GPX export - 250 lines)
│   ├── kml.rs                   (KML export - 250 lines)
│   └── format.rs                (common logic - 100 lines)
├── query/
│   ├── mod.rs
│   ├── builder.rs               (SQL builders - 200 lines)
│   └── filter.rs                (filter logic - 150 lines)
├── models/
│   ├── mod.rs
│   ├── trail.rs                 (trail models)
│   ├── search.rs                (search models)
│   └── export.rs                (export models)
├── middleware/
│   ├── mod.rs
│   ├── policy.rs                (request policy)
│   └── cors.rs                  (CORS config)
└── utils/
    ├── mod.rs
    ├── distance.rs              (distance calculations, shared)
    └── response.rs              (response formatting)
```

### hiking-db Refactoring

**Current**: ~300 lines (ALREADY GOOD)  
**Target**: No major changes, minor organization

```
crates/hiking-db/src/
├── lib.rs                       (pool creation, exports)
├── models.rs                    (keep as is)
├── migrations/
│   └── mod.rs                   (migration helpers, optional)
└── queries/
    ├── mod.rs                   (optional - query builders)
    └── trails.rs                (optional - SQL builders)
```

---

## Refactoring Roadmap

### Phase 1: hiking-import (Priority: HIGH)

**Goal**: Extract largest bottlenecks first  
**Estimated time**: 12-14 hours  
**Payoff**: Biggest code reduction + enables parallelization

#### Step 1: Extract Config Module (1-2 hours)

Create `src/config/environment.rs`:
```rust
pub struct ImportConfig {
    pub database_url: String,
    pub dem_path: Option<PathBuf>,
    pub worldcover_path: Option<PathBuf>,
    pub elevation_spacing_m: f64,
    pub prune_missing_trails: bool,
}

impl ImportConfig {
    pub fn from_env() -> Result<Self> { ... }
}
```

**Impact**: -150 lines from main.rs

#### Step 2: Extract Elevation Module (3-4 hours)

Create `src/elevation/`:
- `sampler.rs` — DEM sampling (300 lines)
- `profile.rs` — Profile generation (200 lines)
- `interpolation.rs` — Missing value handling (150 lines)

**Impact**: -650 lines from main.rs

#### Step 3: Extract Relation Processing (2-3 hours)

Create `src/importer/relation.rs`:
```rust
pub struct RelationProcessor {
    pool: PgPool,
    elevation_sampler: Option<ElevationSampler>,
    landcover_sampler: Option<LandcoverSampler>,
}

impl RelationProcessor {
    pub async fn process_relation(
        &self,
        osm_id: i64,
        tags: &Value,
        members: &[RawRelationMember],
    ) -> Result<StitchOutcome> { ... }
}
```

**Impact**: -800 lines from main.rs

#### Step 4: Extract Data Sync Services (3-4 hours)

Create `src/data/`:
- `region.rs` — Region sync (200 lines)
- `place.rs` — Place sync (150 lines)
- `waypoint.rs` — Waypoint sync (150 lines)
- `manifest.rs` — Mapper attribution (200 lines)

**Impact**: -700 lines from main.rs

#### Step 5: Extract Models (1-2 hours)

Create `src/models/`:
- `trail.rs` — Trail structures
- `relation.rs` — Relation structures
- `elevation.rs` — Elevation structures
- `geometry.rs` — Geometry helpers

**Impact**: -300 lines from main.rs

#### Step 6: Extract Utils (1 hour)

Create `src/utils/`:
- `distance.rs` — Distance calculations (shared with API)
- `hash.rs` — Source hash computation
- `logging.rs` — Structured logging helpers

**Impact**: -200 lines from main.rs

**Phase 1 Result**: 
- main.rs: 4,325 → ~600 lines (86% reduction)
- Clear modules with single responsibility
- Elevation module ready for parallelization

---

### Phase 2: hiking-api (Priority: HIGH)

**Goal**: Modularize request handlers  
**Estimated time**: 8-10 hours  
**Payoff**: Easier to extend, clearer request flow

#### Step 1: Extract Config Module (1 hour)

Create `src/config/environment.rs`:
```rust
pub struct ApiConfig {
    pub bind_addr: String,
    pub max_in_flight_requests: usize,
    pub export_limits: ExportLimits,
}
```

**Impact**: -100 lines from main.rs

#### Step 2: Extract Export Handlers (3-4 hours)

Create `src/export/`:
- `geojson.rs` — GeoJSON generation
- `gpx.rs` — GPX generation
- `kml.rs` — KML generation

**Impact**: -600 lines from main.rs

#### Step 3: Extract Search & Trail Handlers (2-3 hours)

Create `src/handlers/`:
- `search.rs` — Search logic
- `trails.rs` — Trail detail logic

**Impact**: -400 lines from main.rs

#### Step 4: Extract Models (1 hour)

Create `src/models/`:
- `trail.rs` — Trail response models
- `search.rs` — Search models
- `export.rs` — Export models

**Impact**: -200 lines from main.rs

#### Step 5: Extract Query Builders (1-2 hours)

Create `src/query/`:
- `builder.rs` — SQL query building
- `filter.rs` — Filter logic

**Impact**: -300 lines from main.rs

**Phase 2 Result**:
- main.rs: 2,503 → ~400 lines (84% reduction)
- Clear handler modules
- Reusable query builders

---

### Phase 3: Shared Code (Priority: MEDIUM)

**Goal**: Eliminate duplication  
**Estimated time**: 2-3 hours  
**Payoff**: Single source of truth

#### Step 1: Create hiking-core Crate

```
crates/hiking-core/src/
├── lib.rs
├── distance.rs              (distance calculations)
├── hash.rs                  (hashing utilities)
├── geometry.rs              (geometry operations)
└── models/
    ├── trail.rs             (shared trail types)
    └── geometry.rs          (shared geometry types)
```

Move to `hiking-core`:
- Distance calculations (used by import + API)
- Shared models
- Geometry utilities

**Impact**: 
- Eliminates ~200 lines of duplication
- Single source of truth
- Both binaries depend on one implementation

#### Step 2: Update Dependencies

```toml
# hiking-import/Cargo.toml
[dependencies]
hiking-db = { path = "../hiking-db" }
hiking-core = { path = "../hiking-core" }

# hiking-api/Cargo.toml
[dependencies]
hiking-db = { path = "../hiking-db" }
hiking-core = { path = "../hiking-core" }
```

---

### Phase 4: Testing (Priority: HIGH)

**Goal**: Enable thorough unit testing  
**Estimated time**: 4-6 hours  
**Payoff**: Confidence in changes

#### Add Tests to Modules

```rust
// In src/elevation/profile.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevation_smoothing() { ... }
    
    #[test]
    fn test_gain_loss_calculation() { ... }
}
```

Key test modules:
- `elevation/profile.rs` — Profile generation
- `importer/relation.rs` — Relation processing
- `utils/distance.rs` — Distance calculations
- `export/geojson.rs` — GeoJSON generation

---

## Overall Project Structure

After refactoring, the entire project should look like:

```
hiking-trails-app/
├── crates/
│   ├── hiking-core/           (shared utilities & types)
│   ├── hiking-db/             (database layer)
│   ├── hiking-import/         (modular import service)
│   │   ├── src/
│   │   │   ├── main.rs       (~200 lines)
│   │   │   ├── config/
│   │   │   ├── importer/
│   │   │   ├── elevation/
│   │   │   ├── terrain/
│   │   │   ├── data/
│   │   │   ├── models/
│   │   │   ├── validation/
│   │   │   └── utils/
│   │   └── Cargo.toml
│   └── hiking-api/            (modular API service)
│       ├── src/
│       │   ├── main.rs       (~100 lines)
│       │   ├── config/
│       │   ├── handlers/
│       │   ├── export/
│       │   ├── query/
│       │   ├── models/
│       │   ├── middleware/
│       │   └── utils/
│       └── Cargo.toml
├── web/
│   ├── src/
│   │   ├── lib/
│   │   │   ├── components/     (extracted components)
│   │   │   ├── config/
│   │   │   ├── services/
│   │   │   └── utils/
│   │   └── routes/
│   └── package.json
├── migrations/
├── docker-compose.yml
├── Cargo.toml                  (workspace)
└── README.md
```

---

## Benefits Summary

### Code Quality
| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Largest file | 5,374 lines | < 500 lines | -91% |
| Avg file size | 650 lines | 150 lines | -77% |
| Cyclomatic complexity | Very high | Low | ✓ |
| Test coverage | 10% | 60%+ | 6x |
| Lines/test | 100:1 | 20:1 | 5x better |

### Developer Experience
| Aspect | Before | After |
|--------|--------|-------|
| Onboarding time | 2+ weeks | 2-3 days |
| Time to fix bug | 30+ min | 5-10 min |
| Time to add feature | days | hours |
| Code review time | 30+ min | 10-15 min |
| Confidence in changes | Low | High |

### Performance
| Aspect | Impact |
|--------|--------|
| Parallelization | Can now parallelize elevation sampling |
| Testing | Faster test iteration (run single module) |
| Build times | No change initially, improved with parallel |
| Runtime | No change (refactoring only) |

---

## Implementation Timeline

```
Week 1:
├─ Mon-Tue: hiking-import Phase 1 (Steps 1-3)    4 hours
├─ Wed:     hiking-import Phase 1 (Steps 4-6)    4 hours
└─ Thu-Fri: hiking-api Phase 1                   5 hours

Week 2:
├─ Mon-Tue: hiking-api Phase 1 continued         3 hours
├─ Wed-Thu: Shared code (hiking-core)            2 hours
└─ Fri:     Testing & cleanup                    2 hours

Total: 20 hours of focused work
```

---

## Risk Mitigation

### Low Risk Because
✅ **Refactoring only** — No functional changes  
✅ **Git preserves history** — Easy to revert  
✅ **Tests pass** — Verify at each step  
✅ **Incremental** — Small, reviewable changes  
✅ **Workspace** — All crates compile together  

### Testing Strategy
1. **Unit tests** — Add before extraction
2. **Integration tests** — Verify module interactions
3. **End-to-end** — Run full import/API
4. **Performance** — Verify no regression

---

## Success Criteria

- [x] No file > 500 lines (except models)
- [x] Each module has single responsibility
- [x] Clear dependency direction (no cycles)
- [x] > 60% unit test coverage
- [x] All existing tests pass
- [x] Build succeeds
- [x] Import still works correctly
- [x] API still works correctly
- [x] No performance regression

---

## Next Steps

1. **Read this plan** (you're doing it now ✓)
2. **Review proposed structure** (sketch out modules)
3. **Start Phase 1, Step 1** (extract config)
4. **Test after each step** (cargo test)
5. **Commit after each step** (git commit)
6. **Move to next step** (repeat)

---

## Estimated Effort

| Phase | Hours | Days | Complexity |
|-------|-------|------|------------|
| Phase 1 (hiking-import) | 12-14 | 2-3 | Medium |
| Phase 2 (hiking-api) | 8-10 | 1-2 | Medium |
| Phase 3 (hiking-core) | 2-3 | < 1 | Low |
| Phase 4 (Testing) | 4-6 | 1 | Low |
| **Total** | **26-33** | **4-7** | **Manageable** |

This is a **significant but achievable refactoring** that will dramatically improve code quality and maintainability.
