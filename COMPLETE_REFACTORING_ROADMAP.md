# Complete Refactoring Roadmap

**Date**: October 11, 2026  
**Scope**: Full-stack refactoring (Frontend + Backend + Project structure)  
**Timeline**: 4-7 weeks  
**Effort**: ~50-60 hours  
**Risk**: Low (refactoring only, no functional changes)

---

## Executive Summary

Transform your hiking-trails-app from a **startup prototype** into a **production-grade system**:

- **5,374 line TrailMap.svelte** → modular components (< 500 lines)
- **4,325 line hiking-import** → organized services (< 600 lines)
- **2,503 line hiking-api** → clean handlers (< 400 lines)
- **Scattered utilities** → shared hiking-core crate
- **No tests** → > 60% coverage

**Result**: A codebase that's easy to understand, test, extend, and maintain.

---

## Complete Documentation Map

### 📚 Master Plans (Read These First)

1. **PROJECT_ORGANIZATION.md** ← **START HERE**
   - Complete directory structure
   - All phases at a glance
   - How to use this roadmap

2. **REFACTORING_PLAN.md**
   - Frontend vision & structure
   - Benefits analysis
   - High-level roadmap

3. **BACKEND_REFACTORING_PLAN.md**
   - Rust refactoring strategy
   - Module breakdown
   - Implementation roadmap

### 📋 Step-by-Step Guides (Follow These)

4. **REFACTORING_GUIDE.md**
   - Detailed frontend extraction steps
   - Time estimates per component
   - Testing & commit templates

5. **REFACTORING_STARTED.md**
   - Current status
   - Next immediate steps
   - Progress tracking

### ✅ Existing Implementations (Reference)

6. **FIXES_APPLIED.md**
   - 5 critical backend bugs fixed
   - Infrastructure added
   - Performance opportunities identified

7. **CRITICAL_FRONTEND_FIXES.md**
   - 7 frontend issues fixed
   - 3D text distortion solved
   - Contour label improvements

8. **web/src/lib/components/MapConfigService.ts**
   - First extraction (DONE ✅)
   - Reference for future extractions

---

## Phase Breakdown

### 🔴 Phase 1: Frontend (Week 1-2) — 10 hours

Extract 6-8 reusable Svelte components from TrailMap.svelte

| Step | Component | Time | Impact |
|------|-----------|------|--------|
| 1 | MapConfigService | ✅ DONE | -200 lines |
| 2 | TerrainControls | 2h | -250 lines |
| 3 | TrailDetailPanel | 2h | -400 lines |
| 4 | Map3DView | 1.5h | -200 lines |
| 5 | TrailSearchPanel | 2h | -300 lines |
| 6 | MapControls | 1.5h | -150 lines |

**Target**: TrailMap.svelte: 5,374 → ~2,900 lines (46% reduction)

---

### 🔴 Phase 2: Backend - Rust (Week 2-4) — 24 hours

#### 2a: hiking-import (12-14 hours)

Extract 6 modules from main.rs:

| Step | Module | Time | Impact |
|------|--------|------|--------|
| 1 | config/ | 1h | -150 lines |
| 2 | elevation/ | 3-4h | -650 lines |
| 3 | importer/ | 2-3h | -800 lines |
| 4 | data/ | 3-4h | -700 lines |
| 5 | models/ | 1-2h | -300 lines |
| 6 | utils/ | 1h | -200 lines |

**Target**: main.rs: 4,325 → ~600 lines (86% reduction)

#### 2b: hiking-api (8-10 hours)

Extract 5 modules from main.rs:

| Step | Module | Time | Impact |
|------|--------|------|--------|
| 1 | config/ | 1h | -100 lines |
| 2 | export/ | 3-4h | -600 lines |
| 3 | handlers/ | 2-3h | -400 lines |
| 4 | query/ | 1-2h | -300 lines |
| 5 | models/ | 1h | -200 lines |

**Target**: main.rs: 2,503 → ~400 lines (84% reduction)

---

### 🟡 Phase 3: Shared Code (Week 4) — 2-3 hours

Create hiking-core crate:

- Move distance calculations
- Move hashing utilities
- Move shared models
- Both import & api depend on it

**Target**: Eliminate ~200 lines of duplication

---

### 🟡 Phase 4: Testing & Polish (Week 4-5) — 6-8 hours

- Add unit tests to new modules
- Add integration tests
- Verify all systems work
- Update documentation

**Target**: > 60% test coverage

---

## How to Execute

### Week 1: Frontend Components

**Day 1-2: TerrainControls**
1. Read `REFACTORING_GUIDE.md` Step 2
2. Create `web/src/lib/components/controls/TerrainControls.svelte`
3. Test in browser: `npm run dev`
4. Commit: `git commit -m "refactor: extract TerrainControls"`

**Day 3-4: TrailDetailPanel**
1. Read `REFACTORING_GUIDE.md` Step 3
2. Create `web/src/lib/components/sidebar/TrailDetailPanel.svelte`
3. Test & commit

**Day 5: MapControls, Map3DView, TrailSearchPanel**
1. Repeat pattern for each

### Week 2-3: Backend Modules

**Similar incremental approach**:
1. Extract one module at a time
2. Test with `cargo test`
3. Verify functionality still works
4. Commit each extraction

---

## Success Checklist

### Code Quality ✅
- [ ] No file > 500 lines
- [ ] Each module has single responsibility
- [ ] Clear dependencies (no cycles)
- [ ] > 60% unit test coverage
- [ ] All existing tests pass

### Frontend ✅
- [ ] TrailMap.svelte < 500 lines
- [ ] 8+ reusable components
- [ ] Sidebar organized
- [ ] Controls organized
- [ ] Services separated
- [ ] No TypeScript errors
- [ ] Builds successfully

### Backend ✅
- [ ] hiking-import < 600 lines
- [ ] hiking-api < 400 lines
- [ ] hiking-core created
- [ ] All modules tested
- [ ] Cargo builds cleanly
- [ ] No warnings

### Documentation ✅
- [ ] PROJECT_ORGANIZATION.md (this file)
- [ ] ARCHITECTURE.md (updated)
- [ ] Module READMEs in place
- [ ] Component guidelines documented
- [ ] Development setup guide

---

## Key Principles

### During Refactoring

1. **One module at a time** — Don't try to do everything at once
2. **Test after each extraction** — Verify functionality before moving on
3. **Commit frequently** — Small, reviewable commits
4. **No functional changes** — Only reorganization
5. **Use git for safety** — Easy to revert if needed

### Code Organization

1. **Single responsibility** — Each module does one thing
2. **Clear boundaries** — Obvious what each file handles
3. **Minimal dependencies** — Loosely coupled, highly cohesive
4. **Reusable modules** — Components used elsewhere
5. **Testable code** — Easy to write unit tests

---

## Risk Assessment

### ✅ Low Risk Because

- **Refactoring only** — No functional changes
- **Git-backed** — Easy to revert
- **Incremental** — Test after each step
- **Documented** — Clear instructions
- **Parallel path** — Keep old files until new ones work

### 🛡️ Mitigation Strategy

If something breaks:
```bash
git reset --hard HEAD~1
# Try again, more carefully
```

If unsure about a change:
```bash
git stash
# Sleep on it
git stash pop
# Continue or discard
```

---

## Timeline Estimate

```
Week 1 (Mon-Fri):        Frontend Phase 1A (10 hours)
├─ Mon-Tue: 4 hours     TerrainControls + TrailDetailPanel
├─ Wed:     2 hours     Map3DView + TrailSearchPanel
├─ Thu:     2 hours     MapControls + MapLayers
└─ Fri:     2 hours     Testing & verification

Week 2-3 (Mon-Fri):      Backend Phase 2 (14 hours)
├─ Mon-Tue: 4 hours     hiking-import config + elevation
├─ Wed-Thu: 4 hours     hiking-import services
├─ Fri:     2 hours     hiking-import models/utils
│
├─ Mon-Tue: 4 hours     hiking-api handlers + export

Week 4 (Mon-Fri):        Backend finish + Shared code (8 hours)
├─ Mon-Tue: 4 hours     hiking-api queries + models
├─ Wed:     2 hours     Create hiking-core
└─ Thu-Fri: 2 hours     Testing & docs

Week 5 (Mon-Tue):        Polish & documentation (4 hours)
└─ Final testing, docs updates, verification

Total: ~50 hours over 4-5 weeks
```

---

## Document Reading Order

### If You Just Want to Start (15 minutes)
1. This document (overview)
2. `REFACTORING_GUIDE.md` Step 2 (what to do now)
3. Start extracting!

### If You Want Full Context (1-2 hours)
1. **PROJECT_ORGANIZATION.md** (vision & structure)
2. **REFACTORING_PLAN.md** (frontend details)
3. **REFACTORING_GUIDE.md** (step-by-step)
4. **BACKEND_REFACTORING_PLAN.md** (rust details)
5. `REFACTORING_STARTED.md` (status)

### If You Need Deep Context (2-3 hours)
All of the above plus:
- **FIXES_APPLIED.md** (bug fixes)
- **CRITICAL_FRONTEND_FIXES.md** (UI improvements)
- **web/src/lib/components/README.md** (examples)
- **web/src/lib/components/MapConfigService.ts** (reference)

---

## Questions & Answers

**Q: How do I know which component to extract first?**
A: Start with Step 2 in `REFACTORING_GUIDE.md` (TerrainControls). It's high-impact and well-isolated.

**Q: What if I break something?**
A: `git reset --hard HEAD~1` reverts to the last commit. Try again more carefully.

**Q: Can I do this part-time?**
A: Yes! Each extraction is independent. Do one per day if needed.

**Q: Do I have to do all phases?**
A: Start with Phase 1 (frontend). That alone is a huge improvement. Phases 2-4 are optional but recommended.

**Q: How do I test while refactoring?**
A: Frontend: `npm run dev` in browser. Backend: `cargo test` + run the app.

**Q: Will this break production?**
A: No. This is pure refactoring—zero functional changes. The app works exactly the same after.

**Q: How long will this take?**
A: 4-7 weeks of part-time or 2-3 weeks of full-time focused work.

---

## Success Stories (Projected)

### Before Refactoring
- 😞 Single 5,374-line file
- 😞 30+ minute code reviews
- 😞 Difficult to understand flow
- 😞 Hard to add features
- 😞 Low test coverage

### After Refactoring
- 😊 Modular, < 500-line files
- 😊 10-15 minute code reviews
- 😊 Clear separation of concerns
- 😊 Easy to extend
- 😊 > 60% test coverage

---

## Final Notes

This refactoring is:

✅ **Well-documented** — Complete guides provided  
✅ **Achievable** — Detailed steps for each piece  
✅ **Safe** — Git makes it reversible  
✅ **Valuable** — Dramatically improves code quality  

**You've got this! 🚀**

---

## Next Step

1. **Open**: `REFACTORING_GUIDE.md`
2. **Follow**: Step 2 (TerrainControls)
3. **Create**: Component file
4. **Test**: In browser
5. **Commit**: Your first extraction
6. **Repeat**: Next component

**Go build something great!**

---

## Document Index

| Document | Purpose | Read Time |
|----------|---------|-----------|
| **This file** | Overview & roadmap | 15 min |
| PROJECT_ORGANIZATION.md | Full directory structure | 20 min |
| REFACTORING_PLAN.md | Frontend vision | 20 min |
| REFACTORING_GUIDE.md | Step-by-step frontend | 30 min |
| BACKEND_REFACTORING_PLAN.md | Rust refactoring | 30 min |
| REFACTORING_STARTED.md | Current status | 10 min |
| Component READMEs | Examples & guidelines | 15 min |

**Total for full context**: ~2.5 hours  
**Time to start extracting**: 15 minutes

---

**Status**: ✅ Complete plan ready for execution  
**Start date**: Whenever you're ready!  
**Expected completion**: 4-7 weeks  
**Expected outcome**: Professional, maintainable codebase  

**Questions? Check the relevant document above. 📚**

**Ready to start? Go to REFACTORING_GUIDE.md, Step 2! 🚀**
