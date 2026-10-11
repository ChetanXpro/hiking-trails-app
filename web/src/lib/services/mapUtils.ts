import type maplibregl from 'maplibre-gl';

export function interactivePointLayerIds(
	trailRoutes: Array<{ id: string }>,
	peakLayersPresent: boolean,
	terrainLayersPresent: boolean
): string[] {
	const ids: string[] = [];
	if (trailRoutes.length) ids.push('trail-casing', 'trail-line', 'trail-click');
	if (peakLayersPresent) ids.push('peak-symbol');
	if (terrainLayersPresent) ids.push('contour-label');
	return ids;
}

export function interactiveNamedFeatureLayerIds(): string[] {
	return [
		'osm-poi-amenity-point',
		'osm-poi-tourism-point',
		'osm-poi-natural-point',
		'osm-named-places',
		'osm-named-peaks',
		'osm-waterway-label'
	];
}

export function interactiveMapFeatureLayerIds(
	pointIds: string[],
	namedIds: string[]
): string[] {
	return [...pointIds, ...namedIds];
}

export function findFirstLabelAnchor(
	style: maplibregl.StyleSpecification,
	layersBefore?: string[]
): string | undefined {
	if (!style.layers) return undefined;
	const textLayers = style.layers.filter((l) => l.type === 'symbol' && l.layout && 'text-field' in l.layout);
	if (layersBefore?.length) {
		return textLayers.find((l) => layersBefore.includes(l.id))?.id;
	}
	return textLayers[0]?.id;
}
