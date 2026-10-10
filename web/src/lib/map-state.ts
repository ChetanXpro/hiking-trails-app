export type MapView = { center: [number, number]; zoom: number };
export function readMapState(url: URL): { trail: string | null; view: MapView | null } {
	const trail = url.searchParams.get('trail');
	const raw = url.searchParams.get('map');
	const parts = raw?.split(',').map(Number);
	const view = parts?.length === 3 && parts.every(Number.isFinite) && parts[0] >= 1 && parts[0] <= 22 && Math.abs(parts[1]) <= 85 && Math.abs(parts[2]) <= 180
		? { zoom: parts[0], center: [parts[2], parts[1]] as [number, number] } : null;
	return { trail: trail && /^[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}$/i.test(trail) ? trail : null, view };
}

export function writeMapState(url: URL, trail: string | null, view: MapView | null): URL {
	const result = new URL(url);
	if (trail) result.searchParams.set('trail', trail); else result.searchParams.delete('trail');
	if (view) result.searchParams.set('map', `${view.zoom.toFixed(2)},${view.center[1].toFixed(5)},${wrapLongitude(view.center[0]).toFixed(5)}`);
	return result;
}

export function wrapLongitude(value: number) { return ((value + 180) % 360 + 360) % 360 - 180; }
export function viewportBounds(west: number, south: number, east: number, north: number): [number, number, number, number] {
	return [east - west >= 360 ? -180 : wrapLongitude(west), Math.max(-90, south), east - west >= 360 ? 180 : wrapLongitude(east), Math.min(90, north)];
}
