import type maplibregl from 'maplibre-gl';

export interface SearchResult {
	id: string;
	name: string;
	kind: string;
	network?: string;
	distance_m?: number;
	subtitle?: string;
	breadcrumb?: string[];
	coordinates?: [number, number];
}

const MIN_SEARCH_CHARS = 3;

export function validateSearchQuery(query: string): boolean {
	return query.trim().length >= MIN_SEARCH_CHARS;
}

export function parseSearchResults(results: SearchResult[]): {
	routes: SearchResult[];
	places: SearchResult[];
} {
	const routes = results.filter((r) => r.kind === 'route').slice(0, 8);
	const places = results.filter((r) => r.kind !== 'route').slice(0, 8);
	return { routes, places };
}

export function findNextSearchIndex(
	current: number,
	total: number,
	direction: 'up' | 'down'
): number {
	if (total === 0) return 0;
	if (direction === 'down') {
		return current + 1 >= total ? 0 : current + 1;
	}
	return current - 1 < 0 ? total - 1 : current - 1;
}

export function fitToSearchResults(
	map: maplibregl.Map | undefined,
	results: SearchResult[]
) {
	if (!map || results.length === 0) return;

	const coordinates = results
		.map((r) => r.coordinates)
		.filter((c): c is [number, number] => c !== undefined);

	if (coordinates.length === 0) return;

	if (coordinates.length === 1) {
		map.flyTo({
			center: coordinates[0],
			zoom: 14,
			duration: 500
		});
		return;
	}

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

	const padding = { top: 60, bottom: 300, left: 20, right: 20 };
	map.fitBounds(
		[
			[minLng, minLat],
			[maxLng, maxLat]
		],
		{ padding, maxZoom: 14, duration: 500 }
	);
}

export function centerOnSearchResult(
	map: maplibregl.Map | undefined,
	result: SearchResult
) {
	if (!map || !result.coordinates) return;

	map.flyTo({
		center: result.coordinates,
		zoom: 13,
		duration: 600
	});
}
