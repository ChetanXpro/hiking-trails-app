import maplibregl from 'maplibre-gl';
import mlcontour from 'maplibre-contour';
import {
	buildColorReliefExpression,
	buildContourTileOptions,
	COLOR_RELIEF_OPACITY,
	CONTOUR_LABEL_SIZE,
	CONTOUR_LABEL_SPACING,
	DEFAULT_TERRAIN_RGB_URL,
	HILLSHADE_EXAGGERATION,
	MIN_TERRAIN_ZOOM,
	normalizeTerrainEncoding,
	normalizeTerrainTileSize,
	type ContourUnit,
	type TerrainBounds,
	type TerrainEncoding,
	type TerrainTileSize,
	isModernMapLibreVersion,
	resolveMapLibreVersion
} from './terrain-config';
import {
	renderSlopeTileImageData,
	SLOPE_MIN_ZOOM,
	type ElevationGrid,
	type ElevationNeighborhood
} from './slope-config';
import { OUTDOOR_THEME } from './outdoor-theme';

const TERRAIN_SOURCE_ID = 'terrain-dem';
const HILLSHADE_SOURCE_ID = 'terrain-dem-hillshade';
const HILLSHADE_LAYER_ID = 'terrain-hillshade';
const COLOR_RELIEF_LAYER_ID = 'terrain-color-relief';
const CONTOUR_SOURCE_ID = 'terrain-contours';
const CONTOUR_MINOR_LAYER_ID = 'terrain-contours-minor';
const CONTOUR_INTERMEDIATE_LAYER_ID = 'terrain-contours-intermediate';
const CONTOUR_INDEX_LAYER_ID = 'terrain-contours-index';
const CONTOUR_LABEL_LAYER_ID = 'terrain-contours-labels';
const SLOPE_SOURCE_ID = 'terrain-slope';
const SLOPE_LAYER_ID = 'terrain-slope-shade';
const DEFAULT_MAX_ZOOM = 13;

let slopeProtocolCounter = 0;

const PALETTE = {
	// A deep, cool slate — dark enough to give valleys and shaded faces real
	// punch without collapsing to the black blobs that would swallow the trails
	// drawn on top of the hillshade.
	shadowBase: '28, 38, 35',
	highlightBase: '255, 250, 232',
	contourMinor: OUTDOOR_THEME.contour.minor,
	contourIntermediate: OUTDOOR_THEME.contour.intermediate,
	contourIndex: OUTDOOR_THEME.contour.index,
	contourLabel: OUTDOOR_THEME.contour.label,
	contourHalo: OUTDOOR_THEME.contour.halo
};

type ContourDemSource = InstanceType<typeof mlcontour.DemSource>;

type TerrainRuntime = {
	demSource: ContourDemSource;
	surfaceAnchorId?: string;
	labelAnchorId?: string;
	contourUnit: ContourUnit;
	contoursVisible: boolean;
	hillshadeVisible: boolean;
	colorReliefVisible: boolean;
	slopeVisible: boolean;
	terrainEnabled: boolean;
	exaggeration: number;
	modern: boolean;
	slopeProtocol?: string;
	slopeMaxzoom: number;
	slopeTileSize: TerrainTileSize;
	slopeOpacity: number;
	bounds?: TerrainBounds;
};

export type TerrainCapabilities = {
	colorRelief: boolean;
	multidirectional: boolean;
	slope: boolean;
};

export type TerrainLayerOptions = {
	demUrl?: string;
	terrainRgbUrl?: string;
	encoding?: TerrainEncoding;
	tileSize?: TerrainTileSize;
	maxzoom?: number;
	bounds?: TerrainBounds;
	attribution?: string;
	surfaceAnchorId?: string;
	lineAnchorId?: string;
	beforeLayerId?: string;
	labelAnchorId?: string;
	contourUnit?: ContourUnit;
	contoursVisible?: boolean;
	hillshadeVisible?: boolean;
	colorReliefVisible?: boolean;
	slopeVisible?: boolean;
	slopeOpacity?: number;
	terrainEnabled?: boolean;
	exaggeration?: number;
	forceModern?: boolean;
};

const runtimes = new WeakMap<maplibregl.Map, TerrainRuntime>();

function detectModern(force?: boolean): boolean {
	if (typeof force === 'boolean') return force;
	return isModernMapLibreVersion(
		resolveMapLibreVersion(maplibregl as { version?: string; getVersion?: () => string })
	);
}

export function addTerrainLayers(
	map: maplibregl.Map,
	options: TerrainLayerOptions = {}
): TerrainCapabilities {
	const demUrl = options.demUrl ?? options.terrainRgbUrl ?? DEFAULT_TERRAIN_RGB_URL;
	const encoding = normalizeTerrainEncoding(options.encoding);
	const tileSize = normalizeTerrainTileSize(options.tileSize);
	const maxzoom = options.maxzoom ?? DEFAULT_MAX_ZOOM;
	const surfaceAnchorId = options.surfaceAnchorId ?? options.lineAnchorId ?? options.beforeLayerId;
	const labelAnchorId = options.labelAnchorId;
	const modern = detectModern(options.forceModern);

	let runtime = runtimes.get(map);
	if (!runtime) {
		const demSource = new mlcontour.DemSource({
			url: demUrl,
			encoding,
			maxzoom,
			worker: true,
			cacheSize: 128,
			timeoutMs: 12000
		});
		demSource.setupMaplibre(maplibregl);

		runtime = {
			demSource,
			surfaceAnchorId,
			labelAnchorId,
			contourUnit: options.contourUnit ?? 'meters',
			contoursVisible: options.contoursVisible ?? false,
			hillshadeVisible: options.hillshadeVisible ?? true,
			colorReliefVisible: options.colorReliefVisible ?? true,
			slopeVisible: options.slopeVisible ?? false,
			terrainEnabled: options.terrainEnabled ?? false,
			exaggeration: options.exaggeration ?? 1.15,
			modern,
			slopeMaxzoom: maxzoom,
			slopeTileSize: tileSize,
			slopeOpacity: clampUnit(options.slopeOpacity ?? 0.7),
			bounds: options.bounds
		};
		runtime.slopeProtocol = registerSlopeProtocol(map, runtime);
		runtimes.set(map, runtime);
	} else {
		runtime.surfaceAnchorId = surfaceAnchorId;
		runtime.labelAnchorId = labelAnchorId;
		runtime.bounds = options.bounds;
		if (options.contourUnit) runtime.contourUnit = options.contourUnit;
		if (options.contoursVisible !== undefined) runtime.contoursVisible = options.contoursVisible;
		if (options.hillshadeVisible !== undefined) runtime.hillshadeVisible = options.hillshadeVisible;
		if (options.colorReliefVisible !== undefined) {
			runtime.colorReliefVisible = options.colorReliefVisible;
		}
		if (options.slopeVisible !== undefined) runtime.slopeVisible = options.slopeVisible;
		if (options.slopeOpacity !== undefined) {
			runtime.slopeOpacity = clampUnit(options.slopeOpacity);
		}
		if (options.terrainEnabled !== undefined) runtime.terrainEnabled = options.terrainEnabled;
		if (options.exaggeration !== undefined) runtime.exaggeration = options.exaggeration;
	}

	ensureTerrainSources(
		map,
		runtime,
		tileSize,
		maxzoom,
		encoding,
		options.bounds,
		options.attribution
	);
	ensureColorReliefLayer(map, runtime);
	ensureHillshadeLayer(map, runtime);
	ensureSlopeLayer(map, runtime);
	rebuildContourLayers(map, runtime);
	applyTerrainState(map, runtime);
	applySky(map, runtime);

	return {
		colorRelief: runtime.modern,
		multidirectional: runtime.modern,
		slope: Boolean(runtime.slopeProtocol)
	};
}

export function setHillshadeVisible(map: maplibregl.Map, visible: boolean): void {
	const runtime = runtimes.get(map);
	if (!runtime) return;
	runtime.hillshadeVisible = visible;
	setLayerVisibility(map, HILLSHADE_LAYER_ID, visible);
}

export function setColorReliefVisible(map: maplibregl.Map, visible: boolean): void {
	const runtime = runtimes.get(map);
	if (!runtime) return;
	runtime.colorReliefVisible = visible;
	setLayerVisibility(map, COLOR_RELIEF_LAYER_ID, visible);
}

export function setSlopeVisible(map: maplibregl.Map, visible: boolean): void {
	const runtime = runtimes.get(map);
	if (!runtime) return;
	runtime.slopeVisible = visible;
	setLayerVisibility(map, SLOPE_LAYER_ID, visible);
}

export function setSlopeOpacity(map: maplibregl.Map, opacity: number): void {
	const runtime = runtimes.get(map);
	if (!runtime) return;
	runtime.slopeOpacity = clampUnit(opacity);
	if (map.getLayer(SLOPE_LAYER_ID)) {
		map.setPaintProperty(
			SLOPE_LAYER_ID,
			'raster-opacity',
			slopeOpacityExpression(runtime.slopeOpacity) as never
		);
	}
}

export function setContoursVisible(map: maplibregl.Map, visible: boolean): void {
	const runtime = runtimes.get(map);
	if (!runtime) return;
	runtime.contoursVisible = visible;
	for (const layerId of contourLayerIds()) {
		setLayerVisibility(map, layerId, visible);
	}
}

export function setContourUnit(map: maplibregl.Map, unit: ContourUnit): void {
	const runtime = runtimes.get(map);
	if (!runtime || runtime.contourUnit === unit) return;
	runtime.contourUnit = unit;
	rebuildContourLayers(map, runtime);
}

export function set3DTerrainEnabled(
	map: maplibregl.Map,
	enabled: boolean,
	exaggeration?: number
): void {
	const runtime = runtimes.get(map);
	if (!runtime) return;
	runtime.terrainEnabled = enabled;
	if (typeof exaggeration === 'number') runtime.exaggeration = exaggeration;
	applyTerrainState(map, runtime);
}

export function setTerrainExaggeration(map: maplibregl.Map, exaggeration: number): void {
	const runtime = runtimes.get(map);
	if (!runtime) return;
	runtime.exaggeration = exaggeration;
	if (runtime.terrainEnabled) {
		map.setTerrain({ source: TERRAIN_SOURCE_ID, exaggeration });
	}
}

export function enableAlpineView(map: maplibregl.Map): void {
	map.setMaxPitch(85);
	map.easeTo({ pitch: 62, duration: 900 });
}

/**
 * Samples the raw merged DEM at a map coordinate. This deliberately bypasses
 * display exaggeration so a location drawer reports the terrain height rather
 * than the dramatized 3D height.
 */
export async function sampleTerrainElevation(
	map: maplibregl.Map,
	coordinates: [number, number],
	signal?: AbortSignal
): Promise<number | null> {
	const runtime = runtimes.get(map);
	if (!runtime || signal?.aborted) return null;
	const [longitude, latitude] = coordinates;
	if (!Number.isFinite(longitude) || !Number.isFinite(latitude) || Math.abs(latitude) >= 85.051129) {
		return null;
	}
	if (
		runtime.bounds &&
		(longitude < runtime.bounds[0] ||
			latitude < runtime.bounds[1] ||
			longitude > runtime.bounds[2] ||
			latitude > runtime.bounds[3])
	) {
		return null;
	}

	const zoom = runtime.slopeMaxzoom;
	const worldSize = 2 ** zoom;
	const normalizedX = ((longitude + 180) / 360) * worldSize;
	const latitudeRad = (latitude * Math.PI) / 180;
	const normalizedY =
		((1 - Math.log(Math.tan(latitudeRad) + 1 / Math.cos(latitudeRad)) / Math.PI) / 2) *
		worldSize;
	const tileX = ((Math.floor(normalizedX) % worldSize) + worldSize) % worldSize;
	const tileY = Math.min(worldSize - 1, Math.max(0, Math.floor(normalizedY)));
	const controller = new AbortController();
	const abort = () => controller.abort();
	signal?.addEventListener('abort', abort, { once: true });

	try {
		const tile = (await runtime.demSource.getDemTile(
			zoom,
			tileX,
			tileY,
			controller
		)) as ElevationGrid;
		if (signal?.aborted) return null;
		const pixelX = (normalizedX - Math.floor(normalizedX)) * Math.max(1, tile.width - 1);
		const pixelY = (normalizedY - Math.floor(normalizedY)) * Math.max(1, tile.height - 1);
		return bilinearElevation(tile, pixelX, pixelY);
	} catch (error) {
		if (controller.signal.aborted || signal?.aborted) return null;
		throw error;
	} finally {
		signal?.removeEventListener('abort', abort);
	}
}

function bilinearElevation(grid: ElevationGrid, x: number, y: number): number | null {
	if (grid.width <= 0 || grid.height <= 0) return null;
	const x0 = Math.min(grid.width - 1, Math.max(0, Math.floor(x)));
	const y0 = Math.min(grid.height - 1, Math.max(0, Math.floor(y)));
	const x1 = Math.min(grid.width - 1, x0 + 1);
	const y1 = Math.min(grid.height - 1, y0 + 1);
	const values = [
		grid.data[y0 * grid.width + x0],
		grid.data[y0 * grid.width + x1],
		grid.data[y1 * grid.width + x0],
		grid.data[y1 * grid.width + x1]
	];
	if (
		values.some(
			(value) => !Number.isFinite(value) || value < -12000 || value > 9000
		)
	) {
		return null;
	}
	const tx = Math.min(1, Math.max(0, x - x0));
	const ty = Math.min(1, Math.max(0, y - y0));
	const top = values[0] + (values[1] - values[0]) * tx;
	const bottom = values[2] + (values[3] - values[2]) * tx;
	return top + (bottom - top) * ty;
}

function ensureTerrainSources(
	map: maplibregl.Map,
	runtime: TerrainRuntime,
	tileSize: TerrainTileSize,
	maxzoom: number,
	encoding: TerrainEncoding,
	bounds?: TerrainBounds,
	attribution?: string
) {
	for (const sourceId of [TERRAIN_SOURCE_ID, HILLSHADE_SOURCE_ID]) {
		if (map.getSource(sourceId)) continue;
		map.addSource(sourceId, {
			type: 'raster-dem',
			tiles: [runtime.demSource.sharedDemProtocolUrl],
			tileSize,
			encoding,
			minzoom: MIN_TERRAIN_ZOOM,
			maxzoom,
			...(bounds ? { bounds } : {}),
			...(attribution ? { attribution } : {})
		});
	}
}

function ensureColorReliefLayer(map: maplibregl.Map, runtime: TerrainRuntime) {
	if (!runtime.modern || map.getLayer(COLOR_RELIEF_LAYER_ID)) return;

	map.addLayer(
		{
			id: COLOR_RELIEF_LAYER_ID,
			type: 'color-relief',
			source: HILLSHADE_SOURCE_ID,
			maxzoom: 18,
			layout: {
				visibility: runtime.colorReliefVisible ? 'visible' : 'none'
			},
			paint: {
				'color-relief-color': buildColorReliefExpression() as never,
				'color-relief-opacity': COLOR_RELIEF_OPACITY as never
			}
		} as never,
		runtime.surfaceAnchorId
	);
}

function ensureHillshadeLayer(map: maplibregl.Map, runtime: TerrainRuntime) {
	if (map.getLayer(HILLSHADE_LAYER_ID)) return;

	const shadow = (alpha: number) => `rgba(${PALETTE.shadowBase}, ${alpha})`;
	const highlight = (alpha: number) => `rgba(${PALETTE.highlightBase}, ${alpha})`;

	// Multidirectional MapLibre shading has no custom near-flat falloff knob.
	// A near-transparent highlight is therefore a restrained approximation that
	// keeps flats clean; the deeper shadow palette supplies contrast on real
	// faces without claiming an x^5-style shader suppression.
	const paint: Record<string, unknown> = runtime.modern
		? {
				'hillshade-method': 'multidirectional',
				'hillshade-illumination-anchor': 'viewport',
				'hillshade-illumination-direction': [270, 315, 0, 45],
				'hillshade-illumination-altitude': [40, 35, 45, 55],
				'hillshade-shadow-color': [
					shadow(0.3),
					shadow(0.6),
					shadow(0.34),
					shadow(0.2)
				],
				'hillshade-highlight-color': [
					highlight(0.04),
					highlight(0.16),
					highlight(0.06),
					highlight(0.03)
				],
				'hillshade-exaggeration': HILLSHADE_EXAGGERATION
			}
		: {
				'hillshade-illumination-anchor': 'viewport',
				'hillshade-illumination-direction': 315,
				'hillshade-shadow-color': shadow(0.66),
				'hillshade-highlight-color': highlight(0.22),
				'hillshade-accent-color': shadow(0.28),
				'hillshade-exaggeration': HILLSHADE_EXAGGERATION
			};

	map.addLayer(
		{
			id: HILLSHADE_LAYER_ID,
			type: 'hillshade',
			source: HILLSHADE_SOURCE_ID,
			maxzoom: 20,
			layout: {
				visibility: runtime.hillshadeVisible ? 'visible' : 'none'
			},
			paint: paint as never
		},
		runtime.surfaceAnchorId
	);
}

function supportsSlopeOverlay(): boolean {
	return (
		typeof OffscreenCanvas !== 'undefined' &&
		typeof ImageData !== 'undefined' &&
		typeof (maplibregl as unknown as { addProtocol?: unknown }).addProtocol === 'function'
	);
}

function parseSlopeTileUrl(url: string): [number, number, number] {
	const match = url.match(/\/(\d+)\/(\d+)\/(\d+)(?:\.[a-z]+)?(?:\?.*)?$/i);
	if (!match) throw new Error(`Invalid slope tile URL: ${url}`);
	return [Number(match[1]), Number(match[2]), Number(match[3])];
}

async function encodeSlopePng(
	rgba: Uint8ClampedArray,
	width: number,
	height: number
): Promise<ArrayBuffer> {
	const canvas = new OffscreenCanvas(width, height);
	const ctx = canvas.getContext('2d');
	if (!ctx) throw new Error('Slope overlay requires a 2D canvas context');
	const image = new ImageData(width, height);
	image.data.set(rgba);
	ctx.putImageData(image, 0, 0);
	const blob = await canvas.convertToBlob({ type: 'image/png' });
	return blob.arrayBuffer();
}

/**
 * Registers a per-DEM maplibre protocol that turns raw elevation tiles into a
 * slope-angle overlay. The heavy DEM decode is reused from maplibre-contour's
 * cache/worker, and the slope math reads unmodified elevations so the angle
 * bands stay honest no matter how the hillshade is exaggerated.
 */
function registerSlopeProtocol(
	map: maplibregl.Map,
	runtime: TerrainRuntime
): string | undefined {
	if (!supportsSlopeOverlay()) return undefined;
	const id = `hiking-slope-${(slopeProtocolCounter += 1)}`;
	const addProtocol = (
		maplibregl as unknown as {
			addProtocol: (
				id: string,
				action: (
					params: { url: string },
					abortController: AbortController
				) => Promise<{ data: ArrayBuffer }>
			) => void;
		}
	).addProtocol;

	addProtocol(id, async (params, abortController) => {
		const [z, x, y] = parseSlopeTileUrl(params.url);
		const neighborhood = await fetchSlopeNeighborhood(runtime, z, x, y, abortController);
		const dem = neighborhood[4];
		if (!dem) throw new Error(`Missing center DEM tile for slope ${z}/${x}/${y}`);
		const rgba = renderSlopeTileImageData(dem, z, y, neighborhood);
		const data = await encodeSlopePng(rgba, dem.width, dem.height);
		return { data };
	});

	const removeProtocol = (maplibregl as unknown as { removeProtocol?: (id: string) => void })
		.removeProtocol;
	if (typeof removeProtocol === 'function') {
		map.once('remove', () => removeProtocol(id));
	}
	return id;
}

async function fetchSlopeNeighborhood(
	runtime: TerrainRuntime,
	z: number,
	x: number,
	y: number,
	abortController: AbortController
): Promise<ElevationNeighborhood> {
	const worldSize = 2 ** z;
	const requests: Array<Promise<ElevationGrid | undefined>> = [];
	for (let dy = -1; dy <= 1; dy++) {
		for (let dx = -1; dx <= 1; dx++) {
			const tileY = y + dy;
			if (tileY < 0 || tileY >= worldSize) {
				requests.push(Promise.resolve(undefined));
				continue;
			}
			const tileX = (x + dx + worldSize) % worldSize;
			const request = runtime.demSource
				.getDemTile(z, tileX, tileY, abortController)
				.then((tile) => tile as ElevationGrid);
			if (dx === 0 && dy === 0) {
				requests.push(request);
			} else {
				requests.push(
					request.catch((error) => {
						if (abortController.signal.aborted) throw error;
						return undefined;
					})
				);
			}
		}
	}
	return Promise.all(requests);
}

function ensureSlopeLayer(map: maplibregl.Map, runtime: TerrainRuntime) {
	if (!runtime.slopeProtocol || map.getLayer(SLOPE_LAYER_ID)) return;

	if (!map.getSource(SLOPE_SOURCE_ID)) {
		map.addSource(SLOPE_SOURCE_ID, {
			type: 'raster',
			tiles: [`${runtime.slopeProtocol}://tile/{z}/{x}/{y}`],
			tileSize: runtime.slopeTileSize,
			minzoom: SLOPE_MIN_ZOOM,
			maxzoom: runtime.slopeMaxzoom,
			...(runtime.bounds ? { bounds: runtime.bounds } : {})
		});
	}

	map.addLayer(
		{
			id: SLOPE_LAYER_ID,
			type: 'raster',
			source: SLOPE_SOURCE_ID,
			minzoom: SLOPE_MIN_ZOOM,
			layout: {
				visibility: runtime.slopeVisible ? 'visible' : 'none'
			},
			paint: {
				// Slope sits above hillshade but below contours/trails, so it can
				// stay legible without ever hiding the linework drawn on top.
				'raster-opacity': slopeOpacityExpression(runtime.slopeOpacity),
				'raster-resampling': 'nearest',
				'raster-fade-duration': 150
			} as never
		},
		runtime.surfaceAnchorId
	);
}

function slopeOpacityExpression(opacity: number): unknown[] {
	return [
		'interpolate',
		['linear'],
		['zoom'],
		SLOPE_MIN_ZOOM,
		0.72 * opacity,
		15,
		0.66 * opacity,
		17,
		0.58 * opacity
	];
}

function clampUnit(value: number): number {
	return Math.min(1, Math.max(0, value));
}

function rebuildContourLayers(map: maplibregl.Map, runtime: TerrainRuntime) {
	for (const layerId of [...contourLayerIds()].reverse()) {
		if (map.getLayer(layerId)) {
			map.removeLayer(layerId);
		}
	}

	if (map.getSource(CONTOUR_SOURCE_ID)) {
		map.removeSource(CONTOUR_SOURCE_ID);
	}

	map.addSource(CONTOUR_SOURCE_ID, {
		type: 'vector',
		tiles: [runtime.demSource.contourProtocolUrl(buildContourTileOptions(runtime.contourUnit))],
		maxzoom: 16,
		...(runtime.bounds ? { bounds: runtime.bounds } : {})
	});

	const visibility = runtime.contoursVisible ? 'visible' : 'none';

	map.addLayer(
		{
			id: CONTOUR_MINOR_LAYER_ID,
			type: 'line',
			source: CONTOUR_SOURCE_ID,
			'source-layer': 'contours',
			minzoom: 9,
			filter: ['==', ['get', 'level'], 0],
			layout: {
				visibility,
				'line-join': 'round'
			},
			paint: {
				'line-color': PALETTE.contourMinor,
				'line-width': ['interpolate', ['linear'], ['zoom'], 9, 0.25, 12, 0.38, 14, 0.48, 16, 0.6],
				'line-opacity': ['interpolate', ['linear'], ['zoom'], 9, 0.16, 11, 0.22, 14, 0.3, 16, 0.38]
			}
		},
		runtime.surfaceAnchorId
	);

	map.addLayer(
		{
			id: CONTOUR_INTERMEDIATE_LAYER_ID,
			type: 'line',
			source: CONTOUR_SOURCE_ID,
			'source-layer': 'contours',
			minzoom: 9,
			filter: ['==', ['get', 'level'], 1],
			layout: {
				visibility,
				'line-join': 'round'
			},
			paint: {
				'line-color': PALETTE.contourIntermediate,
				'line-width': ['interpolate', ['linear'], ['zoom'], 9, 0.36, 12, 0.55, 14, 0.7, 16, 0.85],
				'line-opacity': ['interpolate', ['linear'], ['zoom'], 9, 0.28, 11, 0.36, 14, 0.46, 16, 0.54]
			}
		},
		runtime.surfaceAnchorId
	);

	map.addLayer(
		{
			id: CONTOUR_INDEX_LAYER_ID,
			type: 'line',
			source: CONTOUR_SOURCE_ID,
			'source-layer': 'contours',
			minzoom: 9,
			filter: ['>=', ['get', 'level'], 2],
			layout: {
				visibility,
				'line-join': 'round'
			},
			paint: {
				'line-color': PALETTE.contourIndex,
				'line-width': ['interpolate', ['linear'], ['zoom'], 9, 0.65, 12, 0.85, 14, 1.08, 16, 1.3],
				'line-opacity': ['interpolate', ['linear'], ['zoom'], 9, 0.52, 11, 0.62, 14, 0.72, 16, 0.8]
			}
		},
		runtime.surfaceAnchorId
	);

	map.addLayer(
		{
			id: CONTOUR_LABEL_LAYER_ID,
			type: 'symbol',
			source: CONTOUR_SOURCE_ID,
			'source-layer': 'contours',
			filter: ['>=', ['get', 'level'], 2],
			minzoom: 9,
			layout: {
				visibility,
				'symbol-placement': 'line',
				'symbol-spacing': CONTOUR_LABEL_SPACING as never,
				'text-field': ['to-string', ['round', ['get', 'ele']]],
				'text-size': CONTOUR_LABEL_SIZE as never,
				'text-font': ['Noto Sans Bold'],
				'text-letter-spacing': 0.01,
				'text-max-angle': 45,
				'text-padding': 2,
				'text-pitch-alignment': 'viewport',
				'text-rotation-alignment': 'map',
				'text-keep-upright': true,
				'text-allow-overlap': false,
				'text-ignore-placement': true
			},
			paint: {
				'text-color': PALETTE.contourLabel,
				'text-halo-color': PALETTE.contourHalo,
				'text-halo-width': 2,
				'text-halo-blur': 0.2,
				'text-opacity': ['interpolate', ['linear'], ['zoom'], 9, 0.78, 11, 0.88, 14, 0.96, 16, 1]
			}
		},
		runtime.labelAnchorId
	);
}

function applyTerrainState(map: maplibregl.Map, runtime: TerrainRuntime) {
	if (runtime.terrainEnabled) {
		map.setTerrain({ source: TERRAIN_SOURCE_ID, exaggeration: runtime.exaggeration });
		return;
	}
	map.setTerrain(null);
}

function applySky(map: maplibregl.Map, runtime: TerrainRuntime) {
	const setSky = (map as unknown as { setSky?: (sky: unknown) => void }).setSky;
	if (!runtime.modern || typeof setSky !== 'function') return;
	// Horizon fade for the tilted/3D view: keep fog low toward the ground so near
	// terrain stays crisp, and let the far distance dissolve into a soft warm
	// haze against the sky instead of ending in a noisy hard edge. Atmosphere is
	// heavier when zoomed out (distant peaks on the skyline) and eases in as the
	// hiker zooms to study detail.
	setSky.call(map, {
		'sky-color': '#AFC8E0',
		'horizon-color': '#E7E3D8',
		'fog-color': '#ECE7DC',
		'sky-horizon-blend': 0.6,
		'horizon-fog-blend': 0.75,
		'fog-ground-blend': 0.55,
		'atmosphere-blend': ['interpolate', ['linear'], ['zoom'], 8, 0.8, 13, 0.65, 16, 0.5]
	});
}

function setLayerVisibility(map: maplibregl.Map, layerId: string, visible: boolean) {
	if (!map.getLayer(layerId)) return;
	map.setLayoutProperty(layerId, 'visibility', visible ? 'visible' : 'none');
}

function contourLayerIds() {
	return [
		CONTOUR_MINOR_LAYER_ID,
		CONTOUR_INTERMEDIATE_LAYER_ID,
		CONTOUR_INDEX_LAYER_ID,
		CONTOUR_LABEL_LAYER_ID
	];
}
