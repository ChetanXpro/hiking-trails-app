import type maplibregl from 'maplibre-gl';

export interface MapFeatureSelection {
	id: string;
	title: string;
	kindLabel: string;
	source: string;
	sourceLabel: string;
	coordinates: [number, number];
	elevationM?: number;
	elevationSource?: string;
	osmType?: string;
	osmId?: number;
}

export function queryFeaturesAtPoint(
	map: maplibregl.Map | undefined,
	point: maplibregl.Point,
	layerIds: string[]
): maplibregl.GeoJSONFeature[] {
	if (!map) return [];

	const features = map.querySourceFeatures('openmaptiles', {
		sourceLayer: ['poi'],
		filter: ['in', ['get', 'type'], ['literal', ['peak', 'alpine_hut']]]
	});

	return features;
}

export function centerOnFeature(
	map: maplibregl.Map | undefined,
	feature: MapFeatureSelection
) {
	if (!map) return;

	map.flyTo({
		center: feature.coordinates,
		zoom: Math.max(14, map.getZoom()),
		duration: 500
	});
}

export function getFeatureFromLayer(
	feature: any
): MapFeatureSelection | null {
	if (!feature) return null;

	const props = feature.properties || {};
	const [lng, lat] = feature.geometry?.coordinates ?? [0, 0];

	return {
		id: feature.id || props.id || '',
		title: props.name || 'Unknown',
		kindLabel: props.type || 'Feature',
		source: props.source || 'map',
		sourceLabel: props.source_label || 'Map Feature',
		coordinates: [lng, lat],
		elevationM: props.elevation_m,
		elevationSource: props.elevation_source,
		osmType: props.osm_type,
		osmId: props.osm_id
	};
}

export function buildNearbyFeaturesQuery(
	center: [number, number],
	radiusKm: number = 5
): maplibregl.GeoJSONFeature[] {
	// This would typically query the backend API
	// Stub for now
	return [];
}

export function calculateDistance(
	from: [number, number],
	to: [number, number]
): number {
	// Haversine formula for distance between two coordinates
	const R = 6371; // Earth's radius in km
	const dLat = ((to[1] - from[1]) * Math.PI) / 180;
	const dLng = ((to[0] - from[0]) * Math.PI) / 180;
	const a =
		Math.sin(dLat / 2) * Math.sin(dLat / 2) +
		Math.cos((from[1] * Math.PI) / 180) *
			Math.cos((to[1] * Math.PI) / 180) *
			Math.sin(dLng / 2) *
			Math.sin(dLng / 2);
	const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
	return R * c;
}
