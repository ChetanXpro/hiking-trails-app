import { describe, expect, it } from 'vitest';
import {
	buildPaddedElevationGrid,
	metersPerPixel,
	renderPaddedSlopeTileImageData,
	renderSlopeTileImageData,
	slopeColorForAngle,
	SLOPE_BANDS,
	SLOPE_CLEAR_BELOW,
	SLOPE_MIN_ZOOM,
	tileCenterLat,
	type ElevationGrid
} from './slope-config';

const [thirty, thirtyFive, forty, fortyFive] = SLOPE_BANDS;

/** Build a tile whose surface is a uniform plane of `angleDeg` at a real zoom. */
function uniformSlopeTile(angleDeg: number, zoom = 13, tileY = 1600, size = 8): ElevationGrid {
	const res = metersPerPixel(zoom, tileCenterLat(tileY, zoom), size);
	const slopePerPixel = res * Math.tan((angleDeg * Math.PI) / 180);
	const data = new Float32Array(size * size);
	for (let y = 0; y < size; y++) {
		for (let x = 0; x < size; x++) {
			data[y * size + x] = x * slopePerPixel; // rises along +x only
		}
	}
	return { width: size, height: size, data };
}

function uniformSlopeNeighborhood(
	angleDeg: number,
	zoom = 13,
	tileY = 1600,
	size = 8
): ElevationGrid[] {
	const res = metersPerPixel(zoom, tileCenterLat(tileY, zoom), size);
	const slopePerPixel = res * Math.tan((angleDeg * Math.PI) / 180);
	const tiles: ElevationGrid[] = [];
	for (let tileRow = -1; tileRow <= 1; tileRow++) {
		for (let tileColumn = -1; tileColumn <= 1; tileColumn++) {
			const data = new Float32Array(size * size);
			for (let y = 0; y < size; y++) {
				for (let x = 0; x < size; x++) {
					data[y * size + x] = (tileColumn * size + x) * slopePerPixel;
				}
			}
			tiles.push({ width: size, height: size, data });
		}
	}
	return tiles;
}

function pixel(rgba: Uint8ClampedArray, x: number, y: number, width: number) {
	const i = (y * width + x) * 4;
	return [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]];
}

describe('slope-config', () => {
	it('leaves gentle terrain clear and bands the classic steepness thresholds', () => {
		expect(slopeColorForAngle(0)).toEqual([0, 0, 0, 0]);
		expect(slopeColorForAngle(SLOPE_CLEAR_BELOW - 0.1)).toEqual([0, 0, 0, 0]);
		expect(slopeColorForAngle(30)).toEqual([...thirty.color, thirty.alpha]);
		expect(slopeColorForAngle(34.9)).toEqual([...thirty.color, thirty.alpha]);
		expect(slopeColorForAngle(35)).toEqual([...thirtyFive.color, thirtyFive.alpha]);
		expect(slopeColorForAngle(39.9)).toEqual([...thirtyFive.color, thirtyFive.alpha]);
		expect(slopeColorForAngle(40)).toEqual([...forty.color, forty.alpha]);
		expect(slopeColorForAngle(44.9)).toEqual([...forty.color, forty.alpha]);
		expect(slopeColorForAngle(45)).toEqual([...fortyFive.color, fortyFive.alpha]);
		expect(slopeColorForAngle(75)).toEqual([...fortyFive.color, fortyFive.alpha]);
		expect(SLOPE_MIN_ZOOM).toBe(11);
	});

	it('uses neutral gray above 45 degrees instead of implying more avalanche danger', () => {
		expect(Math.max(...fortyFive.color) - Math.min(...fortyFive.color)).toBeLessThan(12);
		expect(fortyFive.alpha).toBeLessThanOrEqual(forty.alpha);
	});

	it('computes Web-Mercator ground resolution that halves each zoom and thins with latitude', () => {
		const equatorZ0 = metersPerPixel(0, 0, 512);
		expect(equatorZ0).toBeCloseTo(78271.5, 0);
		// Each zoom in halves the ground distance per pixel.
		expect(metersPerPixel(1, 0, 512)).toBeCloseTo(equatorZ0 / 2, 5);
		// A 512-pixel tile samples half as far on the ground as a 256-pixel tile.
		expect(metersPerPixel(5, 0, 256)).toBeCloseTo(metersPerPixel(5, 0, 512) * 2, 5);
		// cos(lat) shrinks resolution toward the poles.
		expect(metersPerPixel(5, 60, 512)).toBeCloseTo(metersPerPixel(5, 0, 512) * 0.5, 3);
	});

	it('places tile-row center latitudes symmetrically about the equator', () => {
		expect(tileCenterLat(0, 1)).toBeGreaterThan(0);
		expect(tileCenterLat(1, 1)).toBeLessThan(0);
		expect(tileCenterLat(0, 1)).toBeCloseTo(-tileCenterLat(1, 1), 6);
	});

	it('classifies a real slope by its true angle, independent of exaggeration', () => {
		// A physically 42deg plane must land in the 40-45 band at interior pixels...
		const tile = uniformSlopeTile(42);
		const rgba = renderSlopeTileImageData(tile, 13, 1600);
		const expected = [...forty.color, Math.round(forty.alpha * 255)];
		expect(pixel(rgba, 1, 1, tile.width)).toEqual(expected); // interior
		// Without neighbor tiles the border is deliberately clear, avoiding a
		// fabricated one-sided slope and making missing buffers testable.
		expect(pixel(rgba, 0, 3, tile.width)).toEqual([0, 0, 0, 0]);
		expect(pixel(rgba, 7, 5, tile.width)).toEqual([0, 0, 0, 0]);

		// ...and the same construction at a different zoom yields the same class,
		// proving the angle is true geometry, not a zoom/tile artifact.
		const deep = uniformSlopeTile(42, 16, 30000);
		const deepRgba = renderSlopeTileImageData(deep, 16, 30000);
		expect(pixel(deepRgba, 2, 2, deep.width)).toEqual(expected);
	});

	it('keeps flats and sub-threshold slopes completely clear', () => {
		const flat: ElevationGrid = { width: 6, height: 6, data: new Float32Array(36).fill(2400) };
		const flatRgba = renderSlopeTileImageData(flat, 13, 1600);
		expect(Array.from(flatRgba).every((v) => v === 0)).toBe(true);

		const mellow = renderSlopeTileImageData(uniformSlopeTile(26), 13, 1600);
		const alphas = [];
		for (let i = 3; i < mellow.length; i += 4) alphas.push(mellow[i]);
		expect(alphas.every((a) => a === 0)).toBe(true);
	});

	it('keeps invalid or void DEM samples transparent', () => {
		const tile = uniformSlopeTile(40, 13, 1600, 7);
		tile.data[3 * tile.width + 3] = Number.NaN;
		const rgba = renderSlopeTileImageData(tile, 13, 1600);
		expect(pixel(rgba, 3, 3, tile.width)).toEqual([0, 0, 0, 0]);
		// Horn's 3x3 neighborhood also suppresses neighbors influenced by the void.
		expect(pixel(rgba, 2, 3, tile.width)).toEqual([0, 0, 0, 0]);
	});

	it('uses adjacent DEM tiles to classify edge pixels without seams', () => {
		const neighborhood = uniformSlopeNeighborhood(42);
		const center = neighborhood[4];
		const rgba = renderSlopeTileImageData(center, 13, 1600, neighborhood);
		const padded = buildPaddedElevationGrid(center, neighborhood);
		const workerRgba = renderPaddedSlopeTileImageData(padded, 13, 1600);
		const expected = [...forty.color, Math.round(forty.alpha * 255)];
		expect(pixel(rgba, 0, 3, center.width)).toEqual(expected);
		expect(pixel(rgba, center.width - 1, 4, center.width)).toEqual(expected);
		expect(workerRgba).toEqual(rgba);
		expect(padded.data.length).toBe((center.width + 2) * (center.height + 2));
		expect(padded.data.length).toBeLessThan(center.data.length * neighborhood.length);
	});
});
