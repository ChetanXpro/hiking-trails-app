/**
 * Optional terrain-steepness shading derived from the raw DEM. This stays
 * independent of hillshade and 3D exaggeration, but it remains a planning aid:
 * DEM resolution, voids, vegetation, and terrain change limit local accuracy.
 */

export type SlopeBand = {
	/** Inclusive lower bound of the band, in degrees. */
	minAngle: number;
	/** sRGB fill color for the band. */
	color: [number, number, number];
	/** Fill alpha (0..1) applied where the band is active. */
	alpha: number;
	/** Short label for legends. */
	label: string;
};

/** Below this angle the overlay stays clear; this is not a statement of safety. */
export const SLOPE_CLEAR_BELOW = 30;
export const SLOPE_MIN_ZOOM = 11;

/**
 * Operational slope-reading breakpoints. The 45°+ band changes to neutral dark
 * gray rather than implying endlessly increasing avalanche likelihood. Slope
 * angle alone does not show snow stability, connected terrain, or runout.
 */
export const SLOPE_BANDS: SlopeBand[] = [
	{ minAngle: 30, color: [244, 202, 65], alpha: 0.24, label: '30–35°' },
	{ minAngle: 35, color: [236, 139, 48], alpha: 0.28, label: '35–40°' },
	{ minAngle: 40, color: [204, 65, 55], alpha: 0.32, label: '40–45°' },
	{ minAngle: 45, color: [76, 82, 86], alpha: 0.24, label: '45°+' }
];

export const SLOPE_PLANNING_NOTE =
	'DEM planning aid only — it does not show snow stability, avalanche runout, or overhead hazard. Verify terrain in the field.';

/** Returns the RGBA a slope of `angleDeg` should be painted, or fully clear. */
export function slopeColorForAngle(angleDeg: number): [number, number, number, number] {
	let active: SlopeBand | undefined;
	for (const band of SLOPE_BANDS) {
		if (angleDeg >= band.minAngle) active = band;
	}
	if (!active) return [0, 0, 0, 0];
	return [active.color[0], active.color[1], active.color[2], active.alpha];
}

const EARTH_CIRCUMFERENCE_M = 2 * Math.PI * 6378137;

/**
 * Web-Mercator ground resolution (meters per DEM pixel) at a tile's center
 * latitude. Uses the DEM grid width, not the display tile size, because slope
 * must be computed against the real sample spacing.
 */
export function metersPerPixel(zoom: number, latDeg: number, tileWidth: number): number {
	const latRad = (latDeg * Math.PI) / 180;
	return (EARTH_CIRCUMFERENCE_M * Math.cos(latRad)) / (tileWidth * Math.pow(2, zoom));
}

/** Latitude (deg) of the top edge of tile row `y` at `zoom`. */
export function tileYToLat(y: number, zoom: number): number {
	const n = Math.PI - (2 * Math.PI * y) / Math.pow(2, zoom);
	return (180 / Math.PI) * Math.atan(0.5 * (Math.exp(n) - Math.exp(-n)));
}

/** Center latitude (deg) of tile row `y` at `zoom`. */
export function tileCenterLat(y: number, zoom: number): number {
	return tileYToLat(y + 0.5, zoom);
}

/** Minimal DEM tile shape (matches maplibre-contour's decoded `DemTile`). */
export type ElevationGrid = {
	width: number;
	height: number;
	/** Elevation values in meters, row-major order. */
	data: Float32Array;
};

/** Neighbor order: NW, N, NE, W, center, E, SW, S, SE. */
export type ElevationNeighborhood = readonly (ElevationGrid | undefined)[];

const MIN_VALID_ELEVATION_M = -12000;
const MAX_VALID_ELEVATION_M = 9000;

function validElevation(value: number | undefined): value is number {
	return (
		typeof value === 'number' &&
		Number.isFinite(value) &&
		value >= MIN_VALID_ELEVATION_M &&
		value <= MAX_VALID_ELEVATION_M
	);
}

function sampleNeighborhood(
	neighborhood: ElevationNeighborhood,
	center: ElevationGrid,
	x: number,
	y: number
): number | undefined {
	const { width, height } = center;
	let tileX = 0;
	let tileY = 0;
	let localX = x;
	let localY = y;
	if (x < 0) {
		tileX = -1;
		localX += width;
	} else if (x >= width) {
		tileX = 1;
		localX -= width;
	}
	if (y < 0) {
		tileY = -1;
		localY += height;
	} else if (y >= height) {
		tileY = 1;
		localY -= height;
	}

	const grid = neighborhood[(tileY + 1) * 3 + (tileX + 1)];
	if (!grid || grid.width !== width || grid.height !== height) return undefined;
	const value = grid.data[localY * width + localX];
	return validElevation(value) ? value : undefined;
}

/**
 * Classify a raw DEM tile into an RGBA slope overlay with Horn's weighted 3×3
 * gradient. Supplying neighboring tiles prevents seams at tile edges. Invalid
 * or unbuffered samples remain transparent instead of inventing a slope.
 */
export function renderSlopeTileImageData(
	dem: ElevationGrid,
	zoom: number,
	tileY: number,
	neighbors?: ElevationNeighborhood
): Uint8ClampedArray {
	const { width, height } = dem;
	const res = metersPerPixel(zoom, tileCenterLat(tileY, zoom), width);
	const rgba = new Uint8ClampedArray(width * height * 4);
	const neighborhood = neighbors ?? [undefined, undefined, undefined, undefined, dem];
	if (neighborhood[4] !== dem) {
		const mutable = [...neighborhood];
		mutable[4] = dem;
		return renderSlopeTileImageData(dem, zoom, tileY, mutable);
	}

	for (let y = 0; y < height; y++) {
		for (let x = 0; x < width; x++) {
			const center = sampleNeighborhood(neighborhood, dem, x, y);
			if (center === undefined) continue;
			const z1 = sampleNeighborhood(neighborhood, dem, x - 1, y - 1);
			const z2 = sampleNeighborhood(neighborhood, dem, x, y - 1);
			const z3 = sampleNeighborhood(neighborhood, dem, x + 1, y - 1);
			const z4 = sampleNeighborhood(neighborhood, dem, x - 1, y);
			const z6 = sampleNeighborhood(neighborhood, dem, x + 1, y);
			const z7 = sampleNeighborhood(neighborhood, dem, x - 1, y + 1);
			const z8 = sampleNeighborhood(neighborhood, dem, x, y + 1);
			const z9 = sampleNeighborhood(neighborhood, dem, x + 1, y + 1);
			if ([z1, z2, z3, z4, z6, z7, z8, z9].some((value) => value === undefined)) {
				continue;
			}

			const dzdx =
				(z3! + 2 * z6! + z9! - (z1! + 2 * z4! + z7!)) / (8 * res);
			const dzdy =
				(z7! + 2 * z8! + z9! - (z1! + 2 * z2! + z3!)) / (8 * res);
			const angle = (Math.atan(Math.hypot(dzdx, dzdy)) * 180) / Math.PI;
			const [r, g, b, a] = slopeColorForAngle(angle);
			const i = (y * width + x) * 4;
			rgba[i] = r;
			rgba[i + 1] = g;
			rgba[i + 2] = b;
			rgba[i + 3] = Math.round(a * 255);
		}
	}
	return rgba;
}
