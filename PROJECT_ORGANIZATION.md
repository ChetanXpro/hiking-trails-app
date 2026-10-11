# Complete Project Organization Blueprint

**Status**: Comprehensive refactoring plan ready for implementation  
**Scope**: Frontend (Svelte) + Backend (Rust) + Overall structure  
**Estimated time**: 4-7 weeks of focused work  
**Payoff**: Enterprise-grade, professional codebase

---

## Vision

Transform the hiking-trails-app from a startup prototype into a **production-ready, maintainable system** with:

- ✅ Clear separation of concerns
- ✅ Modular, reusable components
- ✅ Easy to test and verify
- ✅ Fast to onboard new developers
- ✅ Safe to extend and refactor
- ✅ Consistent architecture across full stack

---

## Directory Structure (Target)

```
hiking-trails-app/
│
├── 📁 crates/                          (Rust binaries & libraries)
│   ├── hiking-core/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── distance.rs             (shared distance calculations)
│   │   │   ├── hash.rs                 (shared hashing)
│   │   │   ├── geometry.rs             (shared geometry ops)
│   │   │   └── models/
│   │   │       ├── trail.rs
│   │   │       └── geometry.rs
│   │   ├── tests/
│   │   │   ├── distance_test.rs
│   │   │   └── hash_test.rs
│   │   ├── Cargo.toml
│   │   └── README.md
│   │
│   ├── hiking-db/
│   │   ├── src/
│   │   │   ├── lib.rs                  (pool creation, exports)
│   │   │   ├── models.rs               (database models)
│   │   │   └── queries/                (optional: SQL builders)
│   │   ├── Cargo.toml
│   │   └── README.md
│   │
│   ├── hiking-import/
│   │   ├── src/
│   │   │   ├── main.rs                 (~200 lines - entry point)
│   │   │   ├── lib.rs                  (re-exports)
│   │   │   ├── config/
│   │   │   │   ├── mod.rs
│   │   │   │   └── environment.rs
│   │   │   ├── importer/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── relation.rs         (relation processing)
│   │   │   │   ├── waypoint.rs         (waypoint discovery)
│   │   │   │   └── source_hash.rs      (change detection)
│   │   │   ├── elevation/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── sampler.rs          (DEM sampling)
│   │   │   │   ├── profile.rs          (profile generation)
│   │   │   │   └── interpolation.rs
│   │   │   ├── terrain/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── landcover.rs        (WorldCover)
│   │   │   │   └── slope.rs
│   │   │   ├── data/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── region.rs           (region sync)
│   │   │   │   ├── place.rs            (place sync)
│   │   │   │   ├── waypoint.rs         (waypoint sync)
│   │   │   │   └── manifest.rs         (mapper attribution)
│   │   │   ├── models/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── trail.rs
│   │   │   │   ├── relation.rs
│   │   │   │   ├── elevation.rs
│   │   │   │   └── geometry.rs
│   │   │   ├── validation/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── url.rs              (URL validation)
│   │   │   │   └── relation.rs         (relation validation)
│   │   │   └── utils/
│   │   │       ├── mod.rs
│   │   │       ├── distance.rs         (uses hiking-core)
│   │   │       ├── hash.rs             (uses hiking-core)
│   │   │       └── logging.rs
│   │   ├── tests/
│   │   │   ├── integration_test.rs
│   │   │   └── fixtures/
│   │   ├── Cargo.toml
│   │   └── README.md
│   │
│   └── hiking-api/
│       ├── src/
│       │   ├── main.rs                 (~100 lines - server setup)
│       │   ├── lib.rs                  (re-exports)
│       │   ├── config/
│       │   │   ├── mod.rs
│       │   │   └── environment.rs
│       │   ├── handlers/
│       │   │   ├── mod.rs
│       │   │   ├── trails.rs           (GET /trails/*)
│       │   │   ├── search.rs           (POST /search)
│       │   │   ├── export.rs           (GET /trails/:id/export)
│       │   │   ├── discovery.rs        (GET /discovery/*)
│       │   │   └── health.rs           (readiness/liveness)
│       │   ├── export/
│       │   │   ├── mod.rs
│       │   │   ├── geojson.rs          (GeoJSON export)
│       │   │   ├── gpx.rs              (GPX export)
│       │   │   ├── kml.rs              (KML export)
│       │   │   └── format.rs           (common logic)
│       │   ├── query/
│       │   │   ├── mod.rs
│       │   │   ├── builder.rs          (SQL builders)
│       │   │   └── filter.rs           (filter logic)
│       │   ├── models/
│       │   │   ├── mod.rs
│       │   │   ├── trail.rs            (response models)
│       │   │   ├── search.rs           (search models)
│       │   │   └── export.rs           (export models)
│       │   ├── middleware/
│       │   │   ├── mod.rs
│       │   │   ├── policy.rs           (request policy)
│       │   │   └── cors.rs             (CORS)
│       │   └── utils/
│       │       ├── mod.rs
│       │       ├── distance.rs         (uses hiking-core)
│       │       └── response.rs         (response formatting)
│       ├── tests/
│       │   ├── integration_test.rs
│       │   └── fixtures/
│       ├── Cargo.toml
│       └── README.md
│
├── 📁 web/                             (Frontend - Svelte)
│   ├── src/
│   │   ├── lib/
│   │   │   ├── index.ts                (main exports)
│   │   │   ├── components/
│   │   │   │   ├── TrailMap.svelte     (~400 lines - main wrapper)
│   │   │   │   ├── map/
│   │   │   │   │   ├── MapCore.svelte  (map initialization)
│   │   │   │   │   ├── MapLayers.svelte (layer management)
│   │   │   │   │   └── Map3DView.svelte (3D controls)
│   │   │   │   ├── sidebar/
│   │   │   │   │   ├── Sidebar.svelte
│   │   │   │   │   ├── TrailSearchPanel.svelte
│   │   │   │   │   ├── TrailDetailPanel.svelte
│   │   │   │   │   └── WeatherSummary.svelte
│   │   │   │   ├── controls/
│   │   │   │   │   ├── MapControls.svelte
│   │   │   │   │   ├── TerrainControls.svelte
│   │   │   │   │   └── ExportPanel.svelte
│   │   │   │   ├── charts/
│   │   │   │   │   └── ElevationChart.svelte
│   │   │   │   ├── discovery/
│   │   │   │   │   └── TrailDiscovery.svelte
│   │   │   │   ├── README.md            (component guidelines)
│   │   │   │   └── MapConfigService.ts (✅ DONE)
│   │   │   ├── config/                 (configuration)
│   │   │   │   ├── terrain-config.ts
│   │   │   │   ├── basemap-config.ts
│   │   │   │   ├── peak-config.ts
│   │   │   │   ├── slope-config.ts
│   │   │   │   └── outdoor-poi-config.ts
│   │   │   ├── services/               (business logic)
│   │   │   │   ├── map-state.ts        (map state management)
│   │   │   │   ├── terrain-service.ts  (terrain operations)
│   │   │   │   ├── trail-service.ts    (trail queries)
│   │   │   │   ├── export-service.ts   (export generation)
│   │   │   │   ├── request.ts          (HTTP helpers)
│   │   │   │   └── weather.ts          (weather API)
│   │   │   ├── utils/                  (utilities)
│   │   │   │   ├── map-selection.ts
│   │   │   │   ├── profile-index.ts
│   │   │   │   ├── outdoor-poi-icons.ts
│   │   │   │   └── slope-worker-client.ts
│   │   │   ├── theme/                  (styling)
│   │   │   │   ├── outdoor-theme.ts
│   │   │   │   ├── outdoor-poi-config.ts
│   │   │   │   └── slope-config.ts
│   │   │   └── workers/                (web workers)
│   │   │       └── slope-worker.ts
│   │   ├── routes/
│   │   │   ├── +layout.svelte
│   │   │   ├── +page.svelte
│   │   │   └── qa/
│   │   │       └── +page.svelte
│   │   └── app.css
│   ├── tests/
│   │   ├── component.test.ts
│   │   ├── service.test.ts
│   │   └── fixtures/
│   ├── .svelte-kit/                    (build output)
│   ├── package.json
│   ├── package-lock.json
│   ├── vite.config.ts
│   ├── svelte.config.js
│   └── README.md
│
├── 📁 migrations/                      (Database migrations)
│   ├── 0001_*.sql
│   ├── 0002_*.sql
│   └── ...
│   └── 0020_trail_source_hash.sql      (✅ DONE)
│
├── 📁 scripts/                         (Utility scripts)
│   ├── build_himachal_trails.sh
│   ├── download_copernicus_tiles.sh
│   ├── dem_to_terrain_rgb.sh
│   └── README.md
│
├── 📁 osm2pgsql/                       (OSM data config)
│   └── trails.lua
│
├── 📁 hma_dem/                         (HMA elevation data)
│   ├── download.sh
│   └── hma_list.txt
│
├── 📝 Cargo.toml                       (workspace root)
├── 📝 Cargo.lock
├── 📝 docker-compose.yml               (dev environment)
├── 📝 Dockerfile
├── 📝 .dockerignore
├── 📝 .env.example
├── 📝 .gitignore
│
├── 📚 Documentation (DONE ✅)
│   ├── README.md                       (project overview)
│   ├── ARCHITECTURE.md                 (system design)
│   ├── DEPLOYMENT.md                   (production guide)
│   ├── DEVELOPMENT.md                  (dev guide)
│   │
│   ├── Refactoring Plans
│   ├── REFACTORING_PLAN.md             (frontend + backend overview)
│   ├── REFACTORING_GUIDE.md            (step-by-step frontend)
│   ├── BACKEND_REFACTORING_PLAN.md     (step-by-step backend)
│   ├── REFACTORING_STARTED.md          (status & progress)
│   │
│   ├── Code Reviews & Fixes
│   ├── FIXES_APPLIED.md                (backend bug fixes ✅)
│   ├── CRITICAL_FRONTEND_FIXES.md      (UI fixes ✅)
│   ├── FRONTEND_CHANGES_APPLIED.md     (styling changes ✅)
│   │
│   └── Feature Docs
│       ├── FRONTEND_IMPROVEMENTS.md    (design plan)
│       └── WORK_COMPLETED.md           (summary of all work)
│
└── .claude/                            (Claude Code config)
    └── launch.json
```

---

## Implementation Phases

### Phase 1️⃣: Frontend Refactoring (8-10 days)

**Status**: 🚀 Ready to start  
**Docs**: `REFACTORING_GUIDE.md`

```
Week 1:
├─ Mon-Tue: Extract TerrainControls & TrailDetailPanel
├─ Wed:     Extract Map3DView & TrailSearchPanel
├─ Thu:     Extract MapControls & MapLayers
└─ Fri:     Polish & test

Result: TrailMap.svelte 5,374 → ~400 lines (93% reduction)
```

**Key deliverables**:
- ✅ 8+ reusable components
- ✅ Clear prop interfaces
- ✅ Event-based communication
- ✅ All tests passing
- ✅ Browser functionality verified

---

### Phase 2️⃣: Backend Refactoring (12-15 days)

**Status**: 📋 Plan created, ready to start  
**Docs**: `BACKEND_REFACTORING_PLAN.md`

```
Week 2-3:
├─ Mon-Tue:  Extract hiking-import config & elevation
├─ Wed-Thu:  Extract hiking-import relation & data services
├─ Fri:      Extract hiking-import models & utils
│
├─ Mon-Tue:  Extract hiking-api handlers & export
├─ Wed-Thu:  Extract hiking-api queries & models
├─ Fri:      Create hiking-core shared crate
│
└─ Mon-Tue:  Testing & integration
```

**Key deliverables**:
- ✅ hiking-import: 4,325 → ~600 lines
- ✅ hiking-api: 2,503 → ~400 lines
- ✅ hiking-core: new shared library
- ✅ > 60% test coverage
- ✅ All integration tests passing

---

### Phase 3️⃣: Integration & Documentation (3-5 days)

**Status**: 📋 Ready after phases 1-2

```
├─ ARCHITECTURE.md   (update for new structure)
├─ README.md         (contribution guidelines)
├─ DEVELOPMENT.md    (setup & development)
└─ Module READMEs    (each crate)
```

**Key deliverables**:
- ✅ Complete API documentation
- ✅ Development setup guide
- ✅ Architecture overview
- ✅ Contributing guidelines

---

## Quality Metrics

### Code Quality

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Largest file | 5,374 lines | < 500 lines | 🚀 |
| Avg module size | 650 lines | 150 lines | 🚀 |
| Cyclomatic complexity | Very high | Low | 🚀 |
| Test coverage | ~10% | > 60% | 🚀 |

### Developer Experience

| Aspect | Current | Target | Status |
|--------|---------|--------|--------|
| Onboarding | 2+ weeks | 2-3 days | 🚀 |
| Bug fix time | 30+ min | 5-10 min | 🚀 |
| Feature add time | Days | Hours | 🚀 |
| Code review time | 30+ min | 10-15 min | 🚀 |

---

## Next Actions

### Immediate (This Week)
- [x] Create comprehensive refactoring plans
- [x] Set up component structure
- [ ] **Start Phase 1, Step 1**: Extract TerrainControls component

### Short-term (Next 2 Weeks)
- [ ] Complete Phase 1 (frontend)
- [ ] Complete Phase 2a (hiking-import)
- [ ] Complete Phase 2b (hiking-api)

### Medium-term (Next 3-4 Weeks)
- [ ] Create hiking-core shared library
- [ ] Finish testing & integration
- [ ] Update all documentation

---

## Success Criteria

✅ **Code Quality**
- No file > 500 lines
- Each module has single responsibility
- > 60% unit test coverage
- All tests pass

✅ **Functionality**
- Import still works correctly
- API still works correctly
- Web app still works correctly
- No performance regression

✅ **Maintainability**
- New developers can understand code quickly
- Bug fixes are localized
- Adding features is straightforward
- Code reviews are efficient

✅ **Documentation**
- Architecture documented
- Setup/development guide complete
- Module READMEs in place
- Contributing guidelines clear

---

## How to Use This Document

1. **Overview understanding**: Read this section (you're here ✓)
2. **Frontend details**: See `REFACTORING_GUIDE.md`
3. **Backend details**: See `BACKEND_REFACTORING_PLAN.md`
4. **Current status**: See `REFACTORING_STARTED.md`
5. **Execute step by step**: Follow the guides

---

## Final Notes

This refactoring is:

✅ **Safe** — No functional changes  
✅ **Incremental** — Do piece by piece  
✅ **Reversible** — Git makes it easy  
✅ **Documented** — Complete guides provided  
✅ **Achievable** — 4-7 weeks of focused work  

**The result**: A professional, production-ready codebase that's a joy to work with.

---

## Questions?

| Question | Answer Location |
|----------|-----------------|
| How do I refactor frontend? | `REFACTORING_GUIDE.md` |
| How do I refactor backend? | `BACKEND_REFACTORING_PLAN.md` |
| What's the current status? | `REFACTORING_STARTED.md` |
| What files should exist? | This document |
| How should I organize code? | `web/src/lib/components/README.md` |

---

**Status**: ✅ Complete plan, ready for execution  
**Next**: Start Phase 1, Step 1 (TerrainControls component)  
**Goal**: Professional, maintainable codebase  

**Let's build something great! 🚀**
