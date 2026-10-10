type Coordinate = [number, number];
export type RouteGeometry = { type: string; coordinates?: unknown; geometries?: RouteGeometry[] };
export function nearestSampleIndex(points: Array<{ pos_m: number }>, target: number): number {
	let low = 0, high = points.length;
	while (low < high) { const mid = (low + high) >>> 1; if (points[mid].pos_m < target) low = mid + 1; else high = mid; }
	if (low === 0) return 0;
	if (low === points.length) return Math.max(0, low - 1);
	return target - points[low - 1].pos_m <= points[low].pos_m - target ? low - 1 : low;
}

function lines(geometry: RouteGeometry): Coordinate[][] {
	if (geometry.type === 'LineString') return [geometry.coordinates as Coordinate[]];
	if (geometry.type === 'MultiLineString') return geometry.coordinates as Coordinate[][];
	return (geometry.geometries ?? []).flatMap(lines);
}

/** One cumulative distance index per route; never draw a bridge across parts. */
export function buildRouteIndex(geometry: RouteGeometry, distance: (a: Coordinate, b: Coordinate) => number) {
	let total = 0;
	const segments: { start: Coordinate; end: Coordinate; from: number; to: number }[] = [];
	for (const line of lines(geometry)) for (let i = 1; i < line.length; i++) {
		const length = distance(line[i - 1], line[i]);
		if (length > 0) { segments.push({ start: line[i - 1], end: line[i], from: total, to: total + length }); total += length; }
	}
	return { total, at(fraction: number): Coordinate | null {
		if (!segments.length) return null;
		const target = Math.min(1, Math.max(0, fraction)) * total;
		let low = 0, high = segments.length - 1;
		while (low < high) { const mid = (low + high) >>> 1; if (segments[mid].to < target) low = mid + 1; else high = mid; }
		const s = segments[low], t = (target - s.from) / (s.to - s.from);
		return [s.start[0] + (s.end[0] - s.start[0]) * t, s.start[1] + (s.end[1] - s.start[1]) * t];
	} };
}
