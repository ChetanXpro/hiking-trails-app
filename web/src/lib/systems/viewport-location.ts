import type maplibregl from 'maplibre-gl';

export interface MapViewport {
	lng: number;
	lat: number;
	zoom: number;
	bearing?: number;
	pitch?: number;
}

export function getCurrentViewport(map: maplibregl.Map | undefined): MapViewport | null {
	if (!map) return null;

	const center = map.getCenter();
	return {
		lng: center.lng,
		lat: center.lat,
		zoom: map.getZoom(),
		bearing: map.getBearing(),
		pitch: map.getPitch()
	};
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

export function restoreViewport(
	map: maplibregl.Map | undefined,
	viewport: MapViewport | null
) {
	if (!map || !viewport) return;

	map.flyTo({
		center: [viewport.lng, viewport.lat],
		zoom: viewport.zoom,
		bearing: viewport.bearing ?? 0,
		pitch: viewport.pitch ?? 0,
		duration: 600
	});
}

export function buildShareUrl(
	baseUrl: string,
	viewport: MapViewport,
	trailId?: string
): string {
	const viewStr = encodeViewport(viewport);
	const params = new URLSearchParams();
	params.set('map', viewStr);
	if (trailId) params.set('trail', trailId);
	return `${baseUrl}?${params.toString()}`;
}

export function parseShareUrl(
	url: string
): { viewport: MapViewport | null; trailId?: string } {
	try {
		const u = new URL(url);
		const viewStr = u.searchParams.get('map');
		const viewport = viewStr ? decodeViewport(viewStr) : null;
		const trailId = u.searchParams.get('trail') ?? undefined;
		return { viewport, trailId };
	} catch {
		return { viewport: null };
	}
}
