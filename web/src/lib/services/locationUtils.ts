export interface MapViewport {
	lng: number;
	lat: number;
	zoom: number;
	bearing?: number;
	pitch?: number;
}

export interface MapBounds {
	minLng: number;
	minLat: number;
	maxLng: number;
	maxLat: number;
}

export function encodeViewport(view: MapViewport): string {
	const parts = [view.lng.toFixed(5), view.lat.toFixed(5), view.zoom.toFixed(2)];
	if (view.bearing) parts.push(view.bearing.toFixed(1));
	if (view.pitch) parts.push(view.pitch.toFixed(1));
	return parts.join('/');
}

export function decodeViewport(encoded: string): MapViewport | null {
	try {
		const parts = encoded.split('/');
		if (parts.length < 3) return null;
		return {
			lng: parseFloat(parts[0]),
			lat: parseFloat(parts[1]),
			zoom: parseFloat(parts[2]),
			bearing: parts[3] ? parseFloat(parts[3]) : 0,
			pitch: parts[4] ? parseFloat(parts[4]) : 0
		};
	} catch {
		return null;
	}
}

export function boundsToString(bounds: MapBounds): string {
	return `${bounds.minLng.toFixed(5)},${bounds.minLat.toFixed(5)},${bounds.maxLng.toFixed(5)},${bounds.maxLat.toFixed(5)}`;
}
