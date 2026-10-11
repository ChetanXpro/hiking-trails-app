export type ContourUnit = 'meters' | 'feet';
export type TerrainEncoding = 'mapbox' | 'terrarium';
export type TerrainTileSize = 256 | 512;
export type TerrainBounds = [number, number, number, number];

export type TerrainDeviceProfile = {
	deviceMemoryGb?: number;
	mobile?: boolean;
};

export const DEFAULT_TERRAIN_RGB_URL = '/terrain_rgb_hma/{z}/{x}/{y}';
export const DEFAULT_TERRAIN_BOUNDS: TerrainBounds = [75, 30, 79, 34];
export const DEFAULT_TERRAIN_ATTRIBUTION =
	'<a href="https://nsidc.org/data/hma_dem8m_at/versions/1">HMA DEM</a> · <a href="https://dataspace.copernicus.eu/explore-data/data-collections/copernicus-contributing-missions/collections-description/COP-DEM">Copernicus DEM</a>';
export const MIN_TERRAIN_ZOOM = 6;
export const MAX_TERRAIN_ZOOM = 22;
export const DEFAULT_TERRAIN_MAX_ZOOM = 13;
export const MIN_CONTOUR_ZOOM = 7;
export const MIN_CONTOUR_LABEL_ZOOM = 7;
export const MIN_INTERMEDIATE_CONTOUR_ZOOM = 8;
export const MIN_MINOR_CONTOUR_ZOOM = 9;

/**
 * maplibre-contour keeps raw, parsed, and generated-tile caches, so a cache of
 * 128 512px DEM tiles can consume hundreds of megabytes. Keep enough nearby
 * tiles for smooth panning while respecting memory-constrained hiking phones.
 */
export function terrainCacheSizeForDevice({
	deviceMemoryGb,
	mobile = false
}: TerrainDeviceProfile = {}): number {
	if (deviceMemoryGb != null && deviceMemoryGb <= 2) return 16;
	if (mobile || (deviceMemoryGb != null && deviceMemoryGb <= 4)) return 24;
	if (deviceMemoryGb != null && deviceMemoryGb >= 8) return 64;
	return 48;
}

export const HYPSO_STOPS: Array<[number, string]> = [
	[0, '#B8CBAF'],
	[900, '#C8D0AF'],
	[1600, '#D8D4B5'],
	[2400, '#DDD3B8'],
	[3200, '#DED7C8'],
	[4000, '#DDDCD2'],
	[4700, '#D9E5E3'],
	[5400, '#E1ECEE'],
	[6200, '#EFF3F2'],
	[8850, '#FBFCFA']
];

/**
 * Contour intervals [minor, intermediate, index] in meters, by zoom.
 *
 * At regional zooms only the index hierarchy is drawn: 2,000 m at z7 and
 * 1,000 m at z8. Intermediate lines join at z8 and detailed minor lines at z9,
 * preventing steep Himalayan relief from collapsing into solid brown bands.
 * z7 is the safe floor because smooth contour generation samples the z6 DEM;
 * the current Terrain-RGB pyramid does not contain z5. The mid-zoom band
 * (z11–12) carries denser terrain-reading texture.
 */
export const CONTOUR_THRESHOLDS: Record<number, number[]> = {
	7: [500, 1000, 2000],
	8: [250, 500, 1000],
	9: [250, 500, 1000],
	10: [100, 200, 500],
	11: [50, 100, 250],
	12: [25, 50, 100],
	13: [20, 40, 100],
	14: [20, 40, 100],
	15: [20, 40, 100],
	16: [10, 20, 50]
};

/**
 * Repeat index elevations often enough to scan the terrain at a glance.
 * Collision detection still removes labels where trails, places, or other
 * contour labels need the space.
 */
export const CONTOUR_LABEL_SPACING: unknown[] = [
	'interpolate',
	['linear'],
	['zoom'],
	7,
	900,
	8,
	760,
	9,
	640,
	10,
	520,
	11,
	420,
	12,
	320,
	13,
	255,
	14,
	210,
	15,
	175,
	16,
	145
];

export const CONTOUR_LABEL_SIZE: unknown[] = [
	'interpolate',
	['linear'],
	['zoom'],
	7,
	9,
	8,
	9.5,
	9,
	10,
	10,
	10.25,
	11,
	10.5,
	12,
	11,
	13,
	11.5,
	14,
	12,
	15,
	12.5,
	16,
	13
];

/** Imperial mirror of {@link CONTOUR_THRESHOLDS}; rounded to readable intervals. */
export const CONTOUR_THRESHOLDS_FEET: Record<number, number[]> = {
	7: [2500, 5000, 10000],
	8: [1000, 2000, 4000],
	9: [1000, 2000, 4000],
	10: [500, 1000, 2500],
	11: [100, 200, 500],
	12: [50, 100, 250],
	13: [50, 100, 250],
	14: [50, 100, 250],
	15: [50, 100, 250],
	16: [25, 50, 100]
};

export type ContourIntervals = {
	minor: number;
	intermediate: number;
	index: number;
};

export const COLOR_RELIEF_OPACITY: unknown[] = [
	'interpolate',
	['linear'],
	['zoom'],
	6,
	0.22,
	10,
	0.18,
	13,
	0.14,
	15,
	0.1,
	17,
	0.06,
	18,
	0
];

/**
 * Hillshade relief strength by zoom. This exaggerates the *look* of the shading
 * only — it never feeds slope classification (see slope-config), so cranking it
 * cannot skew the avalanche angle bands. Shade for looks, classify for truth.
 *
 * The mid-zoom band (z11–14) is held near its peak instead of tapering early,
 * because that is exactly where hikers study terrain. Very high zoom still eases
 * off so overzoomed DEM pixels don't read as lumpy.
 */
export const HILLSHADE_EXAGGERATION: unknown[] = [
	'interpolate',
	['linear'],
	['zoom'],
	6,
	0.4,
	9,
	0.55,
	11,
	0.73,
	12.5,
	0.78,
	14,
	0.75,
	15,
	0.65,
	16,
	0.53,
	17,
	0.4,
	18,
	0.28,
	19,
	0.15,
	20,
	0.05
];

export function normalizeTerrainEncoding(value?: string | null): TerrainEncoding {
	return value?.trim().toLowerCase() === 'terrarium' ? 'terrarium' : 'mapbox';
}

export function normalizeTerrainTileSize(value?: number | string | null): TerrainTileSize {
	const parsed =
		typeof value === 'number'
			? value
			: typeof value === 'string'
				? Number.parseInt(value, 10)
				: Number.NaN;
	return parsed === 256 ? 256 : 512;
}

export function normalizeTerrainMaxZoom(
	value?: number | string | null,
	fallback = DEFAULT_TERRAIN_MAX_ZOOM
): number {
	const parsed =
		typeof value === 'number'
			? value
			: typeof value === 'string' && value.trim()
				? Number(value)
				: Number.NaN;
	const safeFallback = Number.isFinite(fallback)
		? Math.min(MAX_TERRAIN_ZOOM, Math.max(MIN_TERRAIN_ZOOM, Math.trunc(fallback)))
		: DEFAULT_TERRAIN_MAX_ZOOM;
	if (!Number.isFinite(parsed)) return safeFallback;
	return Math.min(MAX_TERRAIN_ZOOM, Math.max(MIN_TERRAIN_ZOOM, Math.trunc(parsed)));
}

export function normalizeTerrainBounds(
	value?: string | null,
	fallback?: TerrainBounds
): TerrainBounds | undefined {
	const trimmed = value?.trim();
	if (!trimmed) return fallback;
	const parsed = trimmed.split(',').map((part) => Number(part.trim()));
	if (parsed.length !== 4 || parsed.some((part) => !Number.isFinite(part))) return fallback;
	const [west, south, east, north] = parsed;
	if (west < -180 || east > 180 || south < -90 || north > 90) return fallback;
	if (west >= east || south >= north) return fallback;
	return [west, south, east, north];
}

/**
 * Resolve the running MapLibre version string. `getVersion()` is the reliable
 * runtime accessor in MapLibre 5 — the `version` field is not populated on the
 * default export, so preferring the method keeps real users on the modern
 * (multidirectional + color-relief) code path instead of silently falling back
 * to single-direction shading.
 */
export function resolveMapLibreVersion(
	gl: { version?: string; getVersion?: () => string } | undefined
): string | undefined {
	if (!gl) return undefined;
	const fromMethod = typeof gl.getVersion === 'function' ? gl.getVersion() : undefined;
	return fromMethod ?? gl.version;
}

export function isModernMapLibreVersion(version: string | undefined): boolean {
	if (!version) return false;
	const [major = 0, minor = 0] = version
		.split('.')
		.map((part) => Number.parseInt(part, 10) || 0);
	return major > 5 || (major === 5 && minor >= 5);
}

export function buildColorReliefExpression(): unknown[] {
	const expression: unknown[] = ['interpolate', ['linear'], ['elevation']];
	for (const [elevation, color] of HYPSO_STOPS) {
		expression.push(elevation, color);
	}
	return expression;
}

export function buildContourTileOptions(unit: ContourUnit) {
	return {
		multiplier: unit === 'feet' ? 3.28084 : 1,
		thresholds: unit === 'feet' ? CONTOUR_THRESHOLDS_FEET : CONTOUR_THRESHOLDS,
		contourLayer: 'contours',
		elevationKey: 'ele',
		levelKey: 'level',
		buffer: 2,
		extent: 4096,
		overzoom: 1,
		subsampleBelow: 192
	};
}

export function contourIntervalsAtZoom(unit: ContourUnit, zoom: number): ContourIntervals {
	const thresholds = unit === 'feet' ? CONTOUR_THRESHOLDS_FEET : CONTOUR_THRESHOLDS;
	const zoomLevels = Object.keys(thresholds)
		.map(Number)
		.sort((a, b) => a - b);
	const resolvedZoom =
		[...zoomLevels].reverse().find((candidate) => candidate <= Math.floor(zoom)) ?? zoomLevels[0];
	const [minor, intermediate, index] = thresholds[resolvedZoom];
	return { minor, intermediate, index };
}
