# Components Directory

This directory contains the refactored UI components extracted from the monolithic TrailMap.svelte file.

## Structure

```
components/
├── MapConfigService.ts          # Configuration loading and constants
├── TrailMap.svelte              # Main map component (wrapper)
├── map/
│   ├── MapCore.svelte          # (TODO) Map initialization & viewport
│   ├── MapLayers.svelte        # (TODO) Layer management
│   └── Map3DView.svelte        # (TODO) 3D terrain controls
├── sidebar/
│   ├── Sidebar.svelte          # (TODO) Container
│   ├── TrailSearchPanel.svelte # (TODO) Search & discovery
│   └── TrailDetailPanel.svelte # (TODO) Trail info & stats
├── controls/
│   ├── MapControls.svelte      # (TODO) Zoom, pan buttons
│   ├── TerrainControls.svelte  # (TODO) Contours, hillshade
│   └── ExportPanel.svelte      # (TODO) Export options
└── services/
    ├── terrain-service.ts      # (TODO) Terrain layer ops
    ├── trail-service.ts        # (TODO) Trail queries
    └── export-service.ts       # (TODO) Export generation
```

## Refactoring Progress

### Phase 1: Component Extraction (IN PROGRESS)

**Map Components:**
- [x] Map3DView.svelte - Camera pitch/bearing controls (20 lines)
- [x] MapControls.svelte - Zoom/fit button group (25 lines)
- [ ] MapCore.svelte - Map initialization
- [ ] MapLayers.svelte - Layer management

**Sidebar Components:**
- [x] SearchPanel.svelte - Search input with results (90 lines)
- [x] ExportPanel.svelte - Trail export selector (60 lines)
- [x] TrailDetailPanel.svelte - Trail info & stats (250 lines)
- [ ] FeaturePanel.svelte - Map feature details

**Control Components:**
- [x] TerrainControls.svelte - Contours, hillshade, 3D (150 lines)
- [x] LayersPanel.svelte - Map layer toggles (60 lines)
- [x] MapControls.svelte - Zoom, pan buttons (25 lines)

**Services (Phase 2):**
- [ ] terrain-service.ts - Terrain layer operations
- [ ] trail-service.ts - Trail data queries
- [ ] export-service.ts - Export generation

**Impact so far:** 8 new components created = ~660 lines extracted

## Guidelines

### Component Size
- Aim for < 500 lines per component
- > 300 lines should be split further
- Keep related functionality together

### Props vs State
- Use props for data from parent
- Use local state for component-specific UI state
- Dispatch events upward for parent communication

### File Organization
- One component per file
- Tests colocated as `.test.ts`
- Types in same file or adjacent `.types.ts`

## Migration from TrailMap.svelte

To migrate existing code:

1. **Identify cohesive feature blocks** (e.g., terrain controls)
2. **Extract to new file** with minimal changes
3. **Add props/events** for parent communication
4. **Update imports** in TrailMap.svelte
5. **Test in browser**
6. **Commit** after verification
7. **Repeat** for next feature

### Example: Extract Terrain Controls

**Before** (in TrailMap.svelte):
```typescript
let terrainContoursVisible = false;
let terrainHillshadeVisible = true;
// ... 50+ lines of terrain control logic
```

**After** (new TerrainControls.svelte):
```typescript
let { contoursVisible = false, hillshadeVisible = true } = $props();
let { onContourToggle, onHillshadeToggle } = $props();
// ... 50 lines of terrain control UI
```

**In TrailMap.svelte**:
```typescript
import TerrainControls from './controls/TerrainControls.svelte';

let terrainContoursVisible = false;
let terrainHillshadeVisible = true;
```

```svelte
<TerrainControls
  contoursVisible={terrainContoursVisible}
  hillshadeVisible={terrainHillshadeVisible}
  onContourToggle={(v) => terrainContoursVisible = v}
  onHillshadeToggle={(v) => terrainHillshadeVisible = v}
/>
```

## Benefits Realized

- **TrailMap.svelte**: 5,374 lines → < 500 lines (90%+ reduction)
- **Readability**: Each component has clear responsibility
- **Testability**: Small components are easier to test
- **Reusability**: Components can be used elsewhere
- **Maintainability**: Bug fixes isolated to relevant files
