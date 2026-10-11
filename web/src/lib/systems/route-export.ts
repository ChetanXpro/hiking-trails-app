import type { TrailDetail } from '../types';

export type ExportFormat = 'gpx' | 'geojson' | 'kml';
export type DirectionMode = 'forward' | 'backward' | 'both';

export function buildExportFilename(
	trail: TrailDetail | null,
	format: ExportFormat
): string {
	if (!trail) return `trail.${format}`;

	const name = (trail.name ?? 'trail')
		.toLowerCase()
		.replace(/\s+/g, '-')
		.replace(/[^a-z0-9\-]/g, '')
		.substring(0, 50);

	const timestamp = new Date().toISOString().split('T')[0];
	return `${name}-${timestamp}.${format}`;
}

export function getExportFormatDescription(format: ExportFormat): string {
	const descriptions: Record<ExportFormat, string> = {
		gpx: 'GPX (GPS Exchange Format) - Compatible with all GPS devices and hiking apps',
		geojson: 'GeoJSON - Standard web GIS format, works with mapping libraries',
		kml: 'KML (Google Earth) - Import into Google Earth, Google Maps'
	};
	return descriptions[format];
}

export function directionLabel(mode: DirectionMode, trail: TrailDetail | null): string {
	if (!trail) return mode;

	const from = trail.route_from?.trim();
	const to = trail.route_to?.trim();
	const via = trail.route_via?.trim();

	if (mode === 'both') return 'both ways';
	if (!from && !to) return mode;

	if (mode === 'forward') {
		if (from && via && to) return `${from} → ${via} → ${to}`;
		if (from && to) return `${from} → ${to}`;
		return from ? `from ${from}` : `to ${to}`;
	}

	if (from && via && to) return `${to} → ${via} → ${from}`;
	if (from && to) return `${to} → ${from}`;
	return to ? `from ${to}` : `to ${from}`;
}

export function extractFilenameFromHeader(headerValue: string | null): string | null {
	if (!headerValue) return null;

	// Extract from Content-Disposition header like: attachment; filename="file.gpx"
	const match = headerValue.match(/filename="?([^"]+)"?/);
	return match ? match[1] : null;
}

export function formatBytes(bytes: number): string {
	if (bytes === 0) return '0 B';
	const k = 1024;
	const sizes = ['B', 'KB', 'MB'];
	const i = Math.floor(Math.log(bytes) / Math.log(k));
	return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
}

export function validateExportFormat(format: unknown): format is ExportFormat {
	return ['gpx', 'geojson', 'kml'].includes(String(format));
}
