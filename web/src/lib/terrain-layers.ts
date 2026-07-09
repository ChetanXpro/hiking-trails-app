import maplibregl from 'maplibre-gl';
import mlcontour from 'maplibre-contour';

const TERRAIN_SOURCE_ID = 'terrain-dem';
const HILLSHADE_SOURCE_ID = 'terrain-dem-hillshade';
const HILLSHADE_LAYER_ID = 'terrain-hillshade';
const CONTOUR_SOURCE_ID = 'terrain-contours';
const CONTOUR_MINOR_LAYER_ID = 'terrain-contours-minor';
const CONTOUR_MAJOR_LAYER_ID = 'terrain-contours-major';
const CONTOUR_LABEL_LAYER_ID = 'terrain-contours-labels';
const DEFAULT_MAX_ZOOM = 14;

type ContourUnit = 'meters' | 'feet';
type ContourDemSource = InstanceType<typeof mlcontour.DemSource>;

type TerrainRuntime = {
	demSource: ContourDemSource;
	beforeLayerId?: string;
	contourUnit: ContourUnit;
	contoursVisible: boolean;
	hillshadeVisible: boolean;
	terrainEnabled: boolean;
	exaggeration: number;
	colorReliefVisible: boolean;
};

export type TerrainCapabilities = {
	colorRelief: boolean;
};

export type TerrainLayerOptions = {
	terrainRgbUrl?: string;
	beforeLayerId?: string;
	contourUnit?: ContourUnit;
	contoursVisible?: boolean;
	hillshadeVisible?: boolean;
	terrainEnabled?: boolean;
	exaggeration?: number;
	maxzoom?: number;
};

const runtimes = new WeakMap<maplibregl.Map, TerrainRuntime>();

export function addTerrainLayers(
	map: maplibregl.Map,
	options: TerrainLayerOptions = {}
): TerrainCapabilities {
	const terrainRgbUrl = options.terrainRgbUrl ?? '/terrain_rgb/{z}/{x}/{y}';
	const beforeLayerId = options.beforeLayerId;
	const maxzoom = options.maxzoom ?? DEFAULT_MAX_ZOOM;
	const contourUnit = options.contourUnit ?? 'meters';
	const contoursVisible = options.contoursVisible ?? false;
	const hillshadeVisible = options.hillshadeVisible ?? true;
	const terrainEnabled = options.terrainEnabled ?? false;
	const exaggeration = options.exaggeration ?? 1.2;

	let runtime = runtimes.get(map);
	if (!runtime) {
		const demSource = new mlcontour.DemSource({
			url: terrainRgbUrl,
			encoding: 'mapbox',
			maxzoom,
			worker: true,
			cacheSize: 96,
			timeoutMs: 12000
		});
		demSource.setupMaplibre(maplibregl);

		runtime = {
			demSource,
			beforeLayerId,
			contourUnit,
			contoursVisible,
			hillshadeVisible,
			terrainEnabled,
			exaggeration,
			colorReliefVisible: false
		};
		runtimes.set(map, runtime);
	} else {
		runtime.beforeLayerId = beforeLayerId;
		runtime.contourUnit = contourUnit;
		runtime.contoursVisible = contoursVisible;
		runtime.hillshadeVisible = hillshadeVisible;
		runtime.terrainEnabled = terrainEnabled;
		runtime.exaggeration = exaggeration;
	}

	ensureTerrainSources(map, runtime, maxzoom);
	ensureHillshadeLayer(map, runtime);
	rebuildContourLayers(map, runtime);
	applyTerrainState(map, runtime);

	return { colorRelief: false };
}

export function setHillshadeVisible(map: maplibregl.Map, visible: boolean): void {
	const runtime = requireRuntime(map);
	if (!runtime) return;
	runtime.hillshadeVisible = visible;
	setLayerVisibility(map, HILLSHADE_LAYER_ID, visible);
}

export function setContoursVisible(map: maplibregl.Map, visible: boolean): void {
	const runtime = requireRuntime(map);
	if (!runtime) return;
	runtime.contoursVisible = visible;
	const visibility = visible ? 'visible' : 'none';
	for (const layerId of contourLayerIds()) {
		setLayerVisibility(map, layerId, visibility === 'visible');
	}
}

export function setContourUnit(map: maplibregl.Map, unit: ContourUnit): void {
	const runtime = requireRuntime(map);
	if (!runtime || runtime.contourUnit === unit) return;
	runtime.contourUnit = unit;
	rebuildContourLayers(map, runtime);
}

export function set3DTerrainEnabled(
	map: maplibregl.Map,
	enabled: boolean,
	exaggeration?: number
): void {
	const runtime = requireRuntime(map);
	if (!runtime) return;
	runtime.terrainEnabled = enabled;
	if (typeof exaggeration === 'number') {
		runtime.exaggeration = exaggeration;
	}
	applyTerrainState(map, runtime);
}

export function setTerrainExaggeration(map: maplibregl.Map, exaggeration: number): void {
	const runtime = requireRuntime(map);
	if (!runtime) return;
	runtime.exaggeration = exaggeration;
	if (runtime.terrainEnabled) {
		map.setTerrain({ source: TERRAIN_SOURCE_ID, exaggeration });
	}
}

export function setColorReliefVisible(_map: maplibregl.Map, _visible: boolean): void {
	// MapLibre GL 4.1 in this repo does not support a `color-relief` layer yet.
}

function requireRuntime(map: maplibregl.Map) {
	return runtimes.get(map);
}

function ensureTerrainSources(map: maplibregl.Map, runtime: TerrainRuntime, maxzoom: number) {
	if (!map.getSource(TERRAIN_SOURCE_ID)) {
		map.addSource(TERRAIN_SOURCE_ID, {
			type: 'raster-dem',
			tiles: [runtime.demSource.sharedDemProtocolUrl],
			tileSize: 256,
			encoding: 'mapbox',
			maxzoom
		});
	}

	if (!map.getSource(HILLSHADE_SOURCE_ID)) {
		map.addSource(HILLSHADE_SOURCE_ID, {
			type: 'raster-dem',
			tiles: [runtime.demSource.sharedDemProtocolUrl],
			tileSize: 256,
			encoding: 'mapbox',
			maxzoom
		});
	}
}

function ensureHillshadeLayer(map: maplibregl.Map, runtime: TerrainRuntime) {
	if (map.getLayer(HILLSHADE_LAYER_ID)) return;
	map.addLayer(
		{
			id: HILLSHADE_LAYER_ID,
			type: 'hillshade',
			source: HILLSHADE_SOURCE_ID,
			layout: {
				visibility: runtime.hillshadeVisible ? 'visible' : 'none'
			},
			paint: {
				'hillshade-shadow-color': 'rgba(47, 39, 27, 0.58)',
				'hillshade-highlight-color': 'rgba(250, 244, 225, 0.76)',
				'hillshade-accent-color': 'rgba(120, 90, 58, 0.42)',
				'hillshade-exaggeration': 0.52
			}
		},
		runtime.beforeLayerId
	);
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
		tiles: [runtime.demSource.contourProtocolUrl(contourTileOptions(runtime.contourUnit))],
		maxzoom: 15
	});

	const visibility = runtime.contoursVisible ? 'visible' : 'none';

	map.addLayer(
		{
			id: CONTOUR_MINOR_LAYER_ID,
			type: 'line',
			source: CONTOUR_SOURCE_ID,
			'source-layer': 'contours',
			filter: ['==', ['get', 'level'], 0],
			layout: {
				visibility
			},
			paint: {
				'line-color': 'rgba(74, 62, 46, 0.42)',
				'line-width': [
					'interpolate',
					['linear'],
					['zoom'],
					9,
					0.45,
					12,
					0.7,
					15,
					1.05
				]
			}
		},
		runtime.beforeLayerId
	);

	map.addLayer(
		{
			id: CONTOUR_MAJOR_LAYER_ID,
			type: 'line',
			source: CONTOUR_SOURCE_ID,
			'source-layer': 'contours',
			filter: ['>', ['get', 'level'], 0],
			layout: {
				visibility
			},
			paint: {
				'line-color': 'rgba(58, 47, 35, 0.72)',
				'line-width': [
					'interpolate',
					['linear'],
					['zoom'],
					9,
					0.7,
					12,
					1.1,
					15,
					1.5
				]
			}
		},
		runtime.beforeLayerId
	);

	map.addLayer(
		{
			id: CONTOUR_LABEL_LAYER_ID,
			type: 'symbol',
			source: CONTOUR_SOURCE_ID,
			'source-layer': 'contours',
			filter: ['>', ['get', 'level'], 0],
			minzoom: 12,
			layout: {
				visibility,
				'symbol-placement': 'line',
				'text-field': [
					'concat',
					['to-string', ['round', ['get', 'ele']]],
					runtime.contourUnit === 'feet' ? ' ft' : ' m'
				],
				'text-size': [
					'interpolate',
					['linear'],
					['zoom'],
					12,
					10,
					15,
					12
				],
				'text-font': ['Noto Sans Regular'],
				'text-allow-overlap': false,
				'text-ignore-placement': false,
				'symbol-spacing': 300
			},
			paint: {
				'text-color': 'rgba(52, 43, 31, 0.82)',
				'text-halo-color': 'rgba(255, 250, 242, 0.88)',
				'text-halo-width': 1.15
			}
		},
		runtime.beforeLayerId
	);

}

function applyTerrainState(map: maplibregl.Map, runtime: TerrainRuntime) {
	if (runtime.terrainEnabled) {
		map.setTerrain({ source: TERRAIN_SOURCE_ID, exaggeration: runtime.exaggeration });
		return;
	}
	map.setTerrain(null);
}

function setLayerVisibility(map: maplibregl.Map, layerId: string, visible: boolean) {
	if (!map.getLayer(layerId)) return;
	map.setLayoutProperty(layerId, 'visibility', visible ? 'visible' : 'none');
}

function contourLayerIds() {
	return [CONTOUR_MINOR_LAYER_ID, CONTOUR_MAJOR_LAYER_ID, CONTOUR_LABEL_LAYER_ID];
}

function contourTileOptions(unit: ContourUnit) {
	return {
		multiplier: unit === 'feet' ? 3.28084 : 1,
		thresholds: {
			9: [200, 1000],
			11: [100, 500],
			13: [50, 200],
			15: [20, 100]
		},
		contourLayer: 'contours',
		elevationKey: 'ele',
		levelKey: 'level',
		buffer: 1,
		extent: 4096,
		overzoom: 1,
		subsampleBelow: 80
	};
}
