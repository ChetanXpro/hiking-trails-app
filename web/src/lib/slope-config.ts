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

/**
 * A DEM tile plus the one-pixel border needed by Horn's 3x3 kernel. Keeping
 * only the border avoids cloning nine complete DEM tiles into the classifier
 * worker for every requested slope tile.
 */
export type PaddedElevationGrid = {
	width: number;
	height: number;
	data: Float32Array;
};

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

function matchingGrid(
	grid: ElevationGrid | undefined,
	width: number,
	height: number
): ElevationGrid | undefined {
	return grid?.width === width && grid.height === height ? grid : undefined;
}

/**
 * Copies the center tile and only the edge pixels required from its eight
 * neighbors into a transferable `(width + 2) x (height + 2)` grid.
 */
export function buildPaddedElevationGrid(
	dem: ElevationGrid,
	neighbors?: ElevationNeighborhood
): PaddedElevationGrid {
	const { width, height } = dem;
	const paddedWidth = width + 2;
	const paddedHeight = height + 2;
	const data = new Float32Array(paddedWidth * paddedHeight);
	data.fill(Number.NaN);

	for (let y = 0; y < height; y += 1) {
		data.set(dem.data.subarray(y * width, (y + 1) * width), (y + 1) * paddedWidth + 1);
	}

	const neighborhood = neighbors ?? [];
	const north = matchingGrid(neighborhood[1], width, height);
	const west = matchingGrid(neighborhood[3], width, height);
	const east = matchingGrid(neighborhood[5], width, height);
	const south = matchingGrid(neighborhood[7], width, height);

	if (north) {
		data.set(north.data.subarray((height - 1) * width, height * width), 1);
	}
	if (south) {
		data.set(south.data.subarray(0, width), (height + 1) * paddedWidth + 1);
	}
	if (west) {
		for (let y = 0; y < height; y += 1) {
			data[(y + 1) * paddedWidth] = west.data[y * width + width - 1];
		}
	}
	if (east) {
		for (let y = 0; y < height; y += 1) {
			data[(y + 1) * paddedWidth + width + 1] = east.data[y * width];
		}
	}

	const northwest = matchingGrid(neighborhood[0], width, height);
	const northeast = matchingGrid(neighborhood[2], width, height);
	const southwest = matchingGrid(neighborhood[6], width, height);
	const southeast = matchingGrid(neighborhood[8], width, height);
	if (northwest) data[0] = northwest.data[height * width - 1];
	if (northeast) data[width + 1] = northeast.data[(height - 1) * width];
	if (southwest) data[(height + 1) * paddedWidth] = southwest.data[width - 1];
	if (southeast) data[paddedHeight * paddedWidth - 1] = southeast.data[0];

	return { width, height, data };
}

/** Classifies a transferable padded DEM grid without accessing map state. */
export function renderPaddedSlopeTileImageData(
	padded: PaddedElevationGrid,
	zoom: number,
	tileY: number
): Uint8ClampedArray {
	const { width, height, data } = padded;
	const paddedWidth = width + 2;
	if (width <= 0 || height <= 0 || data.length !== paddedWidth * (height + 2)) {
		throw new Error('Invalid padded elevation grid');
	}

	const res = metersPerPixel(zoom, tileCenterLat(tileY, zoom), width);
	const rgba = new Uint8ClampedArray(width * height * 4);
	for (let y = 0; y < height; y += 1) {
		for (let x = 0; x < width; x += 1) {
			const centerIndex = (y + 1) * paddedWidth + x + 1;
			const z1 = data[centerIndex - paddedWidth - 1];
			const z2 = data[centerIndex - paddedWidth];
			const z3 = data[centerIndex - paddedWidth + 1];
			const z4 = data[centerIndex - 1];
			const center = data[centerIndex];
			const z6 = data[centerIndex + 1];
			const z7 = data[centerIndex + paddedWidth - 1];
			const z8 = data[centerIndex + paddedWidth];
			const z9 = data[centerIndex + paddedWidth + 1];
			if (![center, z1, z2, z3, z4, z6, z7, z8, z9].every(validElevation)) continue;

			const dzdx = (z3 + 2 * z6 + z9 - (z1 + 2 * z4 + z7)) / (8 * res);
			const dzdy = (z7 + 2 * z8 + z9 - (z1 + 2 * z2 + z3)) / (8 * res);
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
	const neighborhood = neighbors ?? [undefined, undefined, undefined, undefined, dem];
	if (neighborhood[4] !== dem) {
		const mutable = [...neighborhood];
		mutable[4] = dem;
		return renderPaddedSlopeTileImageData(buildPaddedElevationGrid(dem, mutable), zoom, tileY);
	}
	return renderPaddedSlopeTileImageData(buildPaddedElevationGrid(dem, neighborhood), zoom, tileY);
}
