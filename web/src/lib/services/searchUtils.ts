export interface SearchResult {
	id: string;
	name: string;
	network?: string;
	distance_m?: number;
	kind?: string;
	subtitle?: string;
	breadcrumb?: string[];
	admin_label?: string;
	place_kind?: string;
}

export function parseSearchResults(results: any[]): {
	routes: SearchResult[];
	places: SearchResult[];
} {
	const routes = results.filter((r) => r.kind === 'route').slice(0, 8);
	const places = results.filter((r) => r.kind !== 'route').slice(0, 8);
	return { routes, places };
}

export function findNextSearchResult(
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
