import type maplibregl from 'maplibre-gl';

export type MapLayer = 'routes' | 'paths' | 'pois' | 'peaks' | 'regions';

export interface LayerVisibility {
	routes: boolean;
	paths: boolean;
	pois: boolean;
	peaks: boolean;
	regions: boolean;
}

export const DEFAULT_LAYER_VISIBILITY: LayerVisibility = {
	routes: true,
	paths: true,
	pois: true,
	peaks: true,
	regions: false
};

export function toggleLayerVisibility(
	map: maplibregl.Map | undefined,
	layerId: MapLayer,
	visible: boolean
) {
	if (!map) return;

	const layerMapping: Record<MapLayer, string[]> = {
		routes: ['trail-casing', 'trail-line', 'trail-click'],
		paths: ['path-line', 'path-click'],
		pois: ['osm-poi-amenity-point', 'osm-poi-tourism-point'],
		peaks: ['peak-symbol', 'osm-named-peaks'],
		regions: ['region-fill', 'region-line']
	};

	const layers = layerMapping[layerId] || [];
	for (const layer of layers) {
		try {
			map.setLayoutProperty(layer, 'visibility', visible ? 'visible' : 'none');
		} catch {
			// Layer doesn't exist, skip
		}
	}
}

export function ensureLayerExists(
	map: maplibregl.Map | undefined,
	layerId: string
): boolean {
	if (!map) return false;
	try {
		return !!map.getLayer(layerId);
	} catch {
		return false;
	}
}

export function getInteractiveLayerIds(
	map: maplibregl.Map | undefined,
	includeTrails: boolean,
	includePeaks: boolean,
	includeTerrainLabels: boolean
): string[] {
	if (!map) return [];

	const ids: string[] = [];

	if (includeTrails) {
		ids.push('trail-casing', 'trail-line', 'trail-click');
	}
	if (includePeaks) {
		ids.push('peak-symbol', 'osm-named-peaks');
	}
	if (includeTerrainLabels) {
		ids.push('contour-label');
	}

	return ids.filter((id) => {
		try {
			return !!map.getLayer(id);
		} catch {
			return false;
		}
	});
}

export function getMapStyle(
	map: maplibregl.Map | undefined
): maplibregl.StyleSpecification | null {
	if (!map) return null;
	return map.getStyle();
}

export function findLayerInsertionPoint(
	style: maplibregl.StyleSpecification | null,
	beforeLayerType: 'symbol' | 'text'
): string | undefined {
	if (!style || !style.layers) return undefined;

	const matchingLayers = style.layers.filter((l) => {
		if (beforeLayerType === 'symbol') return l.type === 'symbol';
		if (beforeLayerType === 'text') return 'text-field' in (l.paint || {});
		return false;
	});

	return matchingLayers[0]?.id;
}
