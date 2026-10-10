# Frontend Map Design Improvements

## Issues Identified

### 1. **Contour Label Problems** (High Priority)
- **Issue**: Numbers show inconsistently, wrong size, poor placement
- **Current**: Labels at 9-13px, spacing 145-900px based on zoom
- **Problem**: Collision detection removes too many labels; text halo is barely visible
- **Impact**: Difficult to quickly read elevation at a glance (vs AllTrails' clear labeling)

### 2. **3D Camera Clipping** (High Priority)
- **Issue**: Camera pitch allows up to 85°, viewer can look "into" mountains
- **Problem**: `maxPitch: 85` in TrailMap.svelte:590 is too permissive
- **Impact**: Disorienting, unrealistic terrain visualization
- **Reference**: AllTrails constrains pitch to ~50-60° max

### 3. **Terrain Shading** (High Priority)
- **Issue**: Current colors too light/washed out (pure Copernicus looks blue/bland)
- **Current**: Using Fused HMA+Copernicus but coloring not optimal
- **Problem**: Hillshade too subtle; shadow/highlight ratio needs adjustment
- **Needs**: Darker, richer terrain with more tonal separation

### 4. **Text Rendering in 3D** (Medium Priority)
- **Issue**: Trail names and place labels distort/stretch when map is pitched
- **Current**: `text-pitch-alignment: 'viewport'` rotates text with camera
- **Problem**: Names become unreadable at high pitch angles
- **Options**: Use `map` alignment (labels rotate with map) or hide text above certain pitch

### 5. **Design Polish** (Medium Priority)
- **Issue**: Overall aesthetic feels raw/incomplete
- **Needs**:
  - Contour lines: darker, sharper (not light tan)
  - Labels: consistent sizing, proper spacing
  - UI chrome: cleaner, less cluttered
  - Color harmony: match AllTrails' sophisticated palette

---

## Implementation Plan

### Phase 1: Contour Labels (Immediate Impact)

**File**: `web/src/lib/terrain-config.ts`

**Changes**:
```typescript
// Increase label spacing to reduce collisions
export const CONTOUR_LABEL_SPACING: unknown[] = [
  'interpolate', ['linear'], ['zoom'],
  7, 1200,    // ← increased from 900
  8, 1000,    // ← increased from 760
  9, 850,     // ← increased from 640
  10, 700,    // ← increased from 520
  11, 580,    // ← increased from 420
  12, 460,    // ← increased from 320
  13, 380,    // ← increased from 255
  14, 320,    // ← increased from 210
  15, 280,    // ← increased from 175
  16, 240     // ← increased from 145
];

// Increase label size for better readability
export const CONTOUR_LABEL_SIZE: unknown[] = [
  'interpolate', ['linear'], ['zoom'],
  7, 11,      // ← increased from 9
  8, 11.5,    // ← increased from 9.5
  9, 12,      // ← increased from 10
  11, 12.5,   // ← increased from 10.5
  12, 13,     // ← increased from 11
  13, 13.5,   // ← increased from 11.5
  14, 14,     // ← increased from 12
  15, 14.5,   // ← increased from 12.5
  16, 15      // ← increased from 13
];

// Increase minimum zoom where labels appear
export const MIN_CONTOUR_LABEL_ZOOM = 6;  // ← decreased from 7 (labels show earlier)
```

**Also update**: `outdoor-theme.ts`
```typescript
contour: {
  minor: '#6B6560',      // ← darker from #7B766C
  intermediate: '#5E5751', // ← darker from #6E675C
  index: '#4C453B',        // ← darker from #5C554B
  label: '#3D3430',        // ← much darker from #51483E (better contrast)
  halo: 'rgba(237, 232, 219, 0.95)' // ← slightly more opaque
}
```

---

### Phase 2: 3D Camera Constraints (Immediate Impact)

**File**: `web/src/lib/TrailMap.svelte`

**Change at line 590**:
```diff
- maxPitch: 85,
+ maxPitch: 60,  // ← limit to 60° (realistic terrain viewing)
```

**Add camera constraint logic** (around line 3110):
```typescript
// Constrain pitch based on zoom level
// At low zoom (zoomed out), limit pitch more to avoid looking into distant mountains
const maxPitchForZoom = Math.max(35, Math.min(60, 50 + (mapZoom - 8) * 1.25));
const pitch = Math.max(0, Math.min(maxPitchForZoom, value));
```

---

### Phase 3: Terrain Shading Enhancement

**File**: `web/src/lib/terrain-layers.ts`

**Update shadow/highlight palette** (lines 51-62):
```typescript
const PALETTE = {
  // Richer, darker slate for more dramatic hillshade
  shadowBase: '18, 24, 22',        // ← much darker from '28, 38, 35'
  highlightBase: '255, 250, 240',  // ← slightly warmer from '255, 250, 232'
  contourMinor: OUTDOOR_THEME.contour.minor,
  contourIntermediate: OUTDOOR_THEME.contour.intermediate,
  contourIndex: OUTDOOR_THEME.contour.index,
  contourLabel: OUTDOOR_THEME.contour.label,
  contourHalo: OUTDOOR_THEME.contour.halo
};
```

**Increase hillshade exaggeration** (terrain-config.ts):
```typescript
export const HILLSHADE_EXAGGERATION = 1.3;  // ← increased from 1.0
```

---

### Phase 4: Text Rendering in 3D

**File**: `web/src/lib/TrailMap.svelte`

**Hide text labels when pitched above threshold** (add to all text-pitch-alignment properties):
```typescript
// For all text layers, add pitch-dependent opacity:
'text-opacity': [
  'interpolate', ['linear'], ['pitch'],
  0, 1,      // ← fully visible at 0° pitch
  45, 0.7,   // ← fade out at 45° pitch
  60, 0      // ← completely hidden at 60° pitch
]
```

**Alternative**: Use `map` alignment for trail labels:
```typescript
'text-pitch-alignment': 'map',  // ← labels rotate with terrain
'text-rotation-alignment': 'map'
```

---

### Phase 5: UI/UX Polish

#### Remove Clutter
- **Search bar**: Simplify styling, reduce border
- **Buttons**: Larger touch targets, consistent spacing
- **Panel transparency**: Slightly more opaque backgrounds for readability

**File**: Update Svelte component styles
```svelte
<style>
  /* Simplify search input */
  .search-input {
    border: none;
    box-shadow: 0 2px 8px rgba(20, 30, 26, 0.15);
    padding: 12px 16px;
    border-radius: 12px;
  }

  /* Increase button clarity */
  .map-button {
    width: 44px;
    height: 44px;
    background: white;
    box-shadow: 0 1px 4px rgba(20, 30, 26, 0.2);
    border-radius: 8px;
  }

  /* Panel backgrounds */
  .panel {
    background: rgba(237, 232, 219, 0.96);  /* ← more opaque */
    border-radius: 12px;
    box-shadow: 0 4px 12px rgba(20, 30, 26, 0.15);
  }
</style>
```

#### Typography
- Trail names: Use `font-weight: 600` (semibold) instead of 400
- Contour labels: Ensure minimum `letter-spacing: 0.5px` for clarity
- Place labels: Use same dark ink color as strong body text

#### Color Refinement
- **Terrain background**: Change from `#EDE8DB` to `#E8E3D7` (slightly warmer, less yellow)
- **Forest green**: Slightly darken from `#A6C095` to `#9DB084`
- **Grass**: Deepen from `#CED9AD` to `#C5D09A`

---

## Testing Checklist

### Visual Quality
- [ ] Contour labels readable at zoom 9-16 (no overlaps)
- [ ] Terrain shading shows clear valley/ridge separation
- [ ] 3D view doesn't allow looking into adjacent peaks
- [ ] Text stays legible when pitched > 40°

### Performance
- [ ] No FPS drops when enabling/disabling contours
- [ ] Label rendering smooth at all zooms
- [ ] Terrain sampling responsive (< 200ms)

### Cross-Device
- [ ] Buttons usable on touch screens (44px minimum)
- [ ] Labels render sharply on 2x/3x displays
- [ ] Performance on mobile (iPad 5th gen baseline)

---

## Optional Enhancements (Phase 6+)

### Advanced
1. **Dynamic contour interval switching** — Show only index contours above zoom 14, minor below zoom 9
2. **Adaptive label density** — Fewer labels in steep terrain (high slope), more in gentle areas
3. **Contour gradient coloring** — Color-code elevations (cool low, warm high) like some online maps
4. **Trail glow effect** — Subtle glow around selected trail (already done, verify visibility)

### Design
1. **Dark mode** — Option to invert colors for night use
2. **Colorblind modes** — Protanopia/Deuteranopia friendly palette
3. **Satellite overlay** — Option to blend satellite data with contours

---

## Priority Order

1. **Contour label spacing + darker colors** (Quick win, huge UX improvement)
2. **Camera pitch limit 60°** (Prevents disorientation)
3. **Hillshade darkening** (More dramatic, professional look)
4. **Text opacity in 3D** (Improves 3D usability)
5. **UI polish** (Refinement pass)

**Estimated time to Phase 4**: 2-3 hours  
**Impact**: Professional, polished appearance competitive with AllTrails

---

## Terrain Data Notes

- **Fused HMA + Copernicus**: Good choice for Himalayas (HMA fills gaps, Copernicus adds detail)
- **Weird dots**: Likely tile boundaries in fused data; minor artifact, acceptable
- **Pure Copernicus**: Too low resolution + bland color relief (keep Fused version)
- **Terrain RGB encoding**: Verify using `mapbox` encoding (not `terrarium`) for consistency

Check current config:
```bash
echo $VITE_TERRAIN_RGB_ENCODING  # Should be empty/mapbox, not terrarium
```
