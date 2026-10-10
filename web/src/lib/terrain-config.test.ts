import { describe, expect, it } from 'vitest';
import {
	buildColorReliefExpression,
	buildContourTileOptions,
	COLOR_RELIEF_OPACITY,
	CONTOUR_THRESHOLDS,
	CONTOUR_THRESHOLDS_FEET,
	CONTOUR_LABEL_SIZE,
	CONTOUR_LABEL_SPACING,
	contourIntervalsAtZoom,
	DEFAULT_TERRAIN_BOUNDS,
	DEFAULT_TERRAIN_RGB_URL,
	HILLSHADE_EXAGGERATION,
	isModernMapLibreVersion,
	MIN_CONTOUR_LABEL_ZOOM,
	MIN_CONTOUR_ZOOM,
	MIN_INTERMEDIATE_CONTOUR_ZOOM,
	MIN_MINOR_CONTOUR_ZOOM,
	MIN_TERRAIN_ZOOM,
	normalizeTerrainEncoding,
	normalizeTerrainBounds,
	normalizeTerrainMaxZoom,
	normalizeTerrainTileSize,
	resolveMapLibreVersion,
	terrainCacheSizeForDevice
} from './terrain-config';

/** Evaluate a MapLibre `['interpolate', ['linear'], ['zoom'], z, v, ...]` at `zoom`. */
function interpolateZoom(expression: unknown[], zoom: number): number {
	const stops = expression.slice(3) as number[];
	let prevZoom = stops[0];
	let prevValue = stops[1];
	for (let i = 0; i < stops.length; i += 2) {
		const stopZoom = stops[i];
		const stopValue = stops[i + 1];
		if (zoom <= stopZoom) {
			if (zoom === stopZoom || i === 0) return stopValue;
			const t = (zoom - prevZoom) / (stopZoom - prevZoom);
			return prevValue + t * (stopValue - prevValue);
		}
		prevZoom = stopZoom;
		prevValue = stopValue;
	}
	return prevValue;
}

describe('terrain-config', () => {
	it('defaults to the fused HMA terrain pyramid and its real minimum zoom', () => {
		expect(DEFAULT_TERRAIN_RGB_URL).toBe('/terrain_rgb_hma/{z}/{x}/{y}');
		expect(DEFAULT_TERRAIN_BOUNDS).toEqual([75, 30, 79, 34]);
		expect(MIN_TERRAIN_ZOOM).toBe(6);
	});

	it('validates configurable terrain coverage bounds', () => {
		expect(normalizeTerrainBounds('75,30,79,34')).toEqual([75, 30, 79, 34]);
		expect(normalizeTerrainBounds('79,30,75,34', DEFAULT_TERRAIN_BOUNDS)).toEqual(
			DEFAULT_TERRAIN_BOUNDS
		);
		expect(normalizeTerrainBounds('bad', undefined)).toBeUndefined();
		expect(normalizeTerrainBounds('', DEFAULT_TERRAIN_BOUNDS)).toEqual(DEFAULT_TERRAIN_BOUNDS);
	});

	it('normalizes terrain encoding safely', () => {
		expect(normalizeTerrainEncoding('terrarium')).toBe('terrarium');
		expect(normalizeTerrainEncoding('TERRARIUM')).toBe('terrarium');
		expect(normalizeTerrainEncoding('mapbox')).toBe('mapbox');
		expect(normalizeTerrainEncoding('unknown')).toBe('mapbox');
	});

	it('normalizes terrain tile size to supported values', () => {
		expect(normalizeTerrainTileSize(512)).toBe(512);
		expect(normalizeTerrainTileSize('512')).toBe(512);
		expect(normalizeTerrainTileSize(256)).toBe(256);
		expect(normalizeTerrainTileSize('300')).toBe(512);
		expect(normalizeTerrainTileSize(undefined)).toBe(512);
	});

	it('bounds DEM caches for memory-constrained hiking devices', () => {
		expect(terrainCacheSizeForDevice({ deviceMemoryGb: 2 })).toBe(16);
		expect(terrainCacheSizeForDevice({ deviceMemoryGb: 4 })).toBe(24);
		expect(terrainCacheSizeForDevice({ mobile: true, deviceMemoryGb: 8 })).toBe(24);
		expect(terrainCacheSizeForDevice({ deviceMemoryGb: 8 })).toBe(64);
		expect(terrainCacheSizeForDevice()).toBe(48);
	});

	it('clamps configured terrain zooms to MapLibre-safe integer bounds', () => {
		expect(normalizeTerrainMaxZoom(undefined)).toBe(13);
		expect(normalizeTerrainMaxZoom('14')).toBe(14);
		expect(normalizeTerrainMaxZoom('14.9')).toBe(14);
		expect(normalizeTerrainMaxZoom('-20')).toBe(6);
		expect(normalizeTerrainMaxZoom('999')).toBe(22);
		expect(normalizeTerrainMaxZoom('not-a-number', 12)).toBe(12);
	});

	it('builds contour options with feet conversion and smooth overzoom defaults', () => {
		const options = buildContourTileOptions('feet');
		expect(options.multiplier).toBeCloseTo(3.28084, 5);
		expect(options.overzoom).toBe(1);
		expect(options.buffer).toBe(2);
		expect(options.subsampleBelow).toBe(192);
		expect(options.thresholds).toEqual(CONTOUR_THRESHOLDS_FEET);
		expect(options.thresholds[14]).toEqual([50, 100, 250]);
		expect(options.thresholds[14][0] / options.multiplier).toBeGreaterThan(15);
		expect(options.thresholds[14][0] / options.multiplier).toBeLessThan(21);
	});

	it('keeps every contour hierarchy on generated minor-line multiples', () => {
		for (const thresholds of [CONTOUR_THRESHOLDS, CONTOUR_THRESHOLDS_FEET]) {
			for (const [minor, intermediate, index] of Object.values(thresholds)) {
				expect(intermediate % minor).toBe(0);
				expect(index % minor).toBe(0);
			}
		}
	});

	it('reports the intervals hikers are seeing at fractional zooms', () => {
		expect(contourIntervalsAtZoom('meters', 7.8)).toEqual({
			minor: 500,
			intermediate: 1000,
			index: 2000
		});
		expect(contourIntervalsAtZoom('meters', 14.3)).toEqual({
			minor: 20,
			intermediate: 40,
			index: 100
		});
		expect(contourIntervalsAtZoom('feet', 12.8)).toEqual({
			minor: 50,
			intermediate: 100,
			index: 250
		});
	});

	it('adds a sparse regional hierarchy and keeps detailed low zoom guarded', () => {
		// Regional zooms generate a broad hierarchy, while the style shows only
		// its index level until z8 so steep relief does not collapse to bands.
		expect(CONTOUR_THRESHOLDS[7]).toEqual([500, 1000, 2000]);
		expect(CONTOUR_THRESHOLDS[8]).toEqual([250, 500, 1000]);
		expect(CONTOUR_THRESHOLDS[9]).toEqual([250, 500, 1000]);
		expect(CONTOUR_THRESHOLDS[10]).toEqual([100, 200, 500]);
		// z11-12 are the terrain-reading band and are ~2x denser than before.
		expect(CONTOUR_THRESHOLDS[11][0]).toBe(50); // was 100
		expect(CONTOUR_THRESHOLDS[12][0]).toBe(25); // was 50
		expect(CONTOUR_THRESHOLDS_FEET[11][0]).toBe(100); // was 250
		expect(CONTOUR_THRESHOLDS_FEET[12][0]).toBe(50); // was 100
		// Minor interval never grows as you zoom in (no coarser step deeper in).
		for (const thresholds of [CONTOUR_THRESHOLDS, CONTOUR_THRESHOLDS_FEET]) {
			const zooms = Object.keys(thresholds).map(Number).sort((a, b) => a - b);
			for (let i = 1; i < zooms.length; i++) {
				expect(thresholds[zooms[i]][0]).toBeLessThanOrEqual(thresholds[zooms[i - 1]][0]);
			}
		}
		// Index contours repeat every fourth or fifth minor line, the conventional
		// scan-friendly cadence used by detailed topographic maps.
		for (const thresholds of [CONTOUR_THRESHOLDS, CONTOUR_THRESHOLDS_FEET]) {
			for (const [minor, intermediate, index] of Object.values(thresholds)) {
				expect(index / minor).toBeGreaterThanOrEqual(4);
				expect(index / minor).toBeLessThanOrEqual(5);
				expect(index).toBeGreaterThan(intermediate);
			}
		}
	});

	it('holds hillshade exaggeration through the mid-zoom band and eases very high zoom', () => {
		const midZooms = [11, 12, 13, 14];
		for (const zoom of midZooms) {
			// Mid-zoom keeps meaningful relief instead of tapering toward flat.
			expect(interpolateZoom(HILLSHADE_EXAGGERATION, zoom)).toBeGreaterThanOrEqual(0.55);
		}
		// Very high zoom still eases off so overzoomed pixels do not read as lumpy.
		expect(interpolateZoom(HILLSHADE_EXAGGERATION, 20)).toBeLessThan(
			interpolateZoom(HILLSHADE_EXAGGERATION, 14)
		);
		expect(interpolateZoom(HILLSHADE_EXAGGERATION, 18)).toBeLessThan(0.3);
	});

	it('resolves the runtime MapLibre version, preferring getVersion()', () => {
		// The default export leaves `version` undefined at runtime; getVersion() is
		// the reliable accessor that keeps users on the multidirectional path.
		expect(resolveMapLibreVersion({ getVersion: () => '5.24.0' })).toBe('5.24.0');
		expect(resolveMapLibreVersion({ version: '5.8.0' })).toBe('5.8.0');
		expect(resolveMapLibreVersion({ version: '4.0.0', getVersion: () => '5.24.0' })).toBe('5.24.0');
		expect(resolveMapLibreVersion({})).toBeUndefined();
		expect(resolveMapLibreVersion(undefined)).toBeUndefined();
		// End-to-end: the resolved runtime version is detected as modern.
		expect(isModernMapLibreVersion(resolveMapLibreVersion({ getVersion: () => '5.24.0' }))).toBe(
			true
		);
	});

	it('keeps meter contours on the intended physical intervals', () => {
		const options = buildContourTileOptions('meters');
		expect(options.thresholds).toEqual(CONTOUR_THRESHOLDS);
		expect(options.thresholds[7]).toEqual([500, 1000, 2000]);
		expect(options.thresholds[14]).toEqual([20, 40, 100]);
		expect(options.thresholds[16]).toEqual([10, 20, 50]);
		expect(MIN_CONTOUR_ZOOM).toBe(7);
		expect(MIN_CONTOUR_LABEL_ZOOM).toBe(7);
		expect(MIN_INTERMEDIATE_CONTOUR_ZOOM).toBe(8);
		expect(MIN_MINOR_CONTOUR_ZOOM).toBe(9);
	});

	it('uses outdoor-map label density and readable scale progression', () => {
		expect(CONTOUR_THRESHOLDS[10]).toEqual([100, 200, 500]);
		expect(CONTOUR_LABEL_SPACING).toEqual(
			expect.arrayContaining([7, 900, 8, 760, 9, 640, 12, 320, 14, 210, 16, 145])
		);
		expect(CONTOUR_LABEL_SIZE).toEqual(
			expect.arrayContaining([7, 9, 8, 9.5, 9, 10, 12, 11, 14, 12, 16, 13])
		);
	});

	it('uses restrained zoom-dependent tint and relief intensity', () => {
		expect(COLOR_RELIEF_OPACITY).toEqual(
			expect.arrayContaining(['interpolate', expect.arrayContaining(['linear']), expect.arrayContaining(['zoom'])])
		);
		expect(HILLSHADE_EXAGGERATION).toEqual(
			expect.arrayContaining(['interpolate', expect.arrayContaining(['linear']), expect.arrayContaining(['zoom'])])
		);
	});

	it('builds a color relief expression from hypsometric stops', () => {
		const expression = buildColorReliefExpression();
		expect(expression[0]).toBe('interpolate');
		expect(expression[1]).toEqual(['linear']);
		expect(expression[2]).toEqual(['elevation']);
		expect(expression.length).toBeGreaterThan(6);
	});

	it('detects modern maplibre versions correctly', () => {
		expect(isModernMapLibreVersion('5.24.0')).toBe(true);
		expect(isModernMapLibreVersion('5.8.0')).toBe(true);
		expect(isModernMapLibreVersion('5.5.0')).toBe(true);
		expect(isModernMapLibreVersion('5.7.9')).toBe(true);
		expect(isModernMapLibreVersion('5.4.9')).toBe(false);
		expect(isModernMapLibreVersion('4.7.1')).toBe(false);
	});
});
