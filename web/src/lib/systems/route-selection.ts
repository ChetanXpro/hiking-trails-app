import type maplibregl from 'maplibre-gl';
import type { TrailDetail } from '../types';

export interface RouteChoice {
	id: string;
	name: string;
	tag?: string;
	network?: string;
}

export function boundsForGeometry(
	coordinates: Array<[number, number]>
): { minLng: number; minLat: number; maxLng: number; maxLat: number } | null {
	if (coordinates.length === 0) return null;

	let minLng = coordinates[0][0];
	let maxLng = coordinates[0][0];
	let minLat = coordinates[0][1];
	let maxLat = coordinates[0][1];

	for (const [lng, lat] of coordinates) {
		minLng = Math.min(minLng, lng);
		maxLng = Math.max(maxLng, lng);
		minLat = Math.min(minLat, lat);
		maxLat = Math.max(maxLat, lat);
	}

	return { minLng, maxLng, minLat, maxLat };
}

export function fitToTrailBounds(
	map: maplibregl.Map | undefined,
	trail: TrailDetail | null
) {
	if (!map || !trail || !Array.isArray(trail.geometry?.coordinates)) return;

	const bounds = boundsForGeometry(trail.geometry.coordinates as Array<[number, number]>);
	if (!bounds) return;

	const padding = { top: 60, bottom: 300, left: 20, right: 20 };
	map.fitBounds(
		[
			[bounds.minLng, bounds.minLat],
			[bounds.maxLng, bounds.maxLat]
		],
		{ padding, maxZoom: 16, duration: 750 }
	);
}

export function centerOnTrail(
	map: maplibregl.Map | undefined,
	trail: TrailDetail | null,
	zoom?: number
) {
	if (!map || !trail || !Array.isArray(trail.geometry?.coordinates)) return;

	const bounds = boundsForGeometry(trail.geometry.coordinates as Array<[number, number]>);
	if (!bounds) return;

	const centerLng = (bounds.minLng + bounds.maxLng) / 2;
	const centerLat = (bounds.minLat + bounds.maxLat) / 2;

	map.flyTo({
		center: [centerLng, centerLat],
		zoom: zoom ?? map.getZoom(),
		duration: 750
	});
}

export function boundsForRouteChoice(
	routeChoice: RouteChoice,
	routeGeometry?: Array<[number, number]>
): { minLng: number; minLat: number; maxLng: number; maxLat: number } | null {
	if (!routeGeometry) return null;
	return boundsForGeometry(routeGeometry);
}

export function fitToRouteChoice(
	map: maplibregl.Map | undefined,
	routeGeometry?: Array<[number, number]>
) {
	if (!map || !routeGeometry) return;

	const bounds = boundsForGeometry(routeGeometry);
	if (!bounds) return;

	const padding = { top: 60, bottom: 300, left: 20, right: 20 };
	map.fitBounds(
		[
			[bounds.minLng, bounds.minLat],
			[bounds.maxLng, bounds.maxLat]
		],
		{ padding, maxZoom: 16, duration: 500 }
	);
}
