/**
 * MapConfigService: Configuration loading for the map
 * Extracted from TrailMap.svelte to reduce file size and improve maintainability
 */

import { env } from '$env/dynamic/public';
import {
	DEFAULT_TERRAIN_ATTRIBUTION,
	DEFAULT_TERRAIN_BOUNDS,
	DEFAULT_TERRAIN_RGB_URL,
	normalizeTerrainBounds,
	normalizeTerrainMaxZoom
} from '../terrain-config';

function publicSetting(name: `PUBLIC_${string}`, legacyValue?: string): string | undefined {
	return env[name]?.trim() || legacyValue?.trim() || undefined;
}

export const MAP_CONFIG = {
	INITIAL_CENTER: [77.18, 32.24] as [number, number],
	INITIAL_ZOOM: 9
};

export const loadBasemapConfig = (viteUrl?: string) => {
	const configured = publicSetting('PUBLIC_BASEMAP_STYLE_URL', viteUrl);
	if (!configured) return null;

	// For vector basemaps, return the URL; for raster, return null
	const normalized = configured.trim();
	if (normalized === 'raster' || normalized === 'legacy') return null;
	return normalized;
};

export const loadGlyphsUrl = (viteUrl?: string): string | undefined => {
	const custom = publicSetting('PUBLIC_MAP_GLYPHS_URL', viteUrl);
	if (custom) return custom;

	if (import.meta.env.DEV) {
		return 'https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf';
	}
	return undefined;
};

export const loadTileUrl = (key: string, viteUrl?: string, fallback?: string): string | undefined => {
	const configured = publicSetting(key as `PUBLIC_${string}`, viteUrl);
	if (configured) return configured;

	if (import.meta.env.DEV && fallback) return fallback;
	return undefined;
};

export const TILE_URLS = {
	getBasemapStyleUrl: (viteUrl?: string) => loadBasemapConfig(viteUrl),
	getBasemapLabelAnchor: (viteUrl?: string) => publicSetting('PUBLIC_BASEMAP_LABEL_ANCHOR_ID', viteUrl),
	getGlyphsUrl: (viteUrl?: string) => loadGlyphsUrl(viteUrl),
	getRasterTileUrl: (viteUrl?: string) =>
		loadTileUrl('PUBLIC_RASTER_TILE_URL', viteUrl, 'https://tile.openstreetmap.org/{z}/{x}/{y}.png'),
	getRasterTileAttribution: (viteUrl?: string) =>
		loadTileUrl('PUBLIC_RASTER_TILE_ATTRIBUTION', viteUrl, '&copy; OpenStreetMap contributors'),
	getWeatherApiUrl: (viteUrl?: string) =>
		publicSetting('PUBLIC_WEATHER_API_URL', viteUrl),
	getMartinTrailsUrl: (viteUrl?: string) =>
		publicSetting('PUBLIC_MARTIN_TRAILS_URL', viteUrl) ?? '/trail_lines/{z}/{x}/{y}',
	getMartinWaypointsUrl: (viteUrl?: string) =>
		publicSetting('PUBLIC_MARTIN_OUTDOOR_WAYPOINTS_URL', viteUrl) ?? '/outdoor_waypoints/{z}/{x}/{y}'
};

export const TERRAIN_CONFIG = {
	getRgbUrl: (viteUrl?: string) => publicSetting('PUBLIC_TERRAIN_RGB_URL', viteUrl) ?? DEFAULT_TERRAIN_RGB_URL,
	getRgbEncoding: (viteUrl?: string) => {
		const val = publicSetting('PUBLIC_TERRAIN_RGB_ENCODING', viteUrl)?.toLowerCase();
		return val === 'terrarium' ? 'terrarium' : 'mapbox';
	},
	getRgbTileSize: (viteUrl?: string) => {
		const parsed = Number.parseInt(publicSetting('PUBLIC_TERRAIN_RGB_TILE_SIZE', viteUrl) ?? '512', 10);
		return parsed === 256 ? 256 : 512;
	},
	getRgbMaxZoom: (viteUrl?: string) =>
		normalizeTerrainMaxZoom(publicSetting('PUBLIC_TERRAIN_RGB_MAX_ZOOM', viteUrl) ?? '13'),
	getRgbBounds: (viteUrl?: string) => {
		const rgbUrl = publicSetting('PUBLIC_TERRAIN_RGB_URL', viteUrl) ?? DEFAULT_TERRAIN_RGB_URL;
		return normalizeTerrainBounds(
			publicSetting('PUBLIC_TERRAIN_RGB_BOUNDS', viteUrl),
			rgbUrl === DEFAULT_TERRAIN_RGB_URL ? DEFAULT_TERRAIN_BOUNDS : undefined
		);
	},
	getAttribution: (viteUrl?: string) => {
		const custom = publicSetting('PUBLIC_TERRAIN_ATTRIBUTION', viteUrl);
		if (custom) return custom;

		const rgbUrl = publicSetting('PUBLIC_TERRAIN_RGB_URL', viteUrl) ?? DEFAULT_TERRAIN_RGB_URL;
		return rgbUrl === DEFAULT_TERRAIN_RGB_URL ? DEFAULT_TERRAIN_ATTRIBUTION : undefined;
	}
};

export const LAYER_IDS = {
	// Hovered/selected trails
	HOVERED_ROUTE_CASING: 'hovered-route-casing',
	HOVERED_ROUTE: 'hovered-route-line',

	// Trail networks by tier
	TRAIL_LOCAL_CASING: 'trail-local-casing',
	TRAIL_REGIONAL_CASING: 'trail-regional-casing',
	TRAIL_MAJOR_CASING: 'trail-major-casing',
	TRAIL_LOCAL: 'trail-local-lines',
	TRAIL_REGIONAL: 'trail-regional-lines',
	TRAIL_MAJOR: 'trail-major-lines',
	TRAIL_FALLBACK_CASING: 'trail-fallback-casing',
	TRAIL_FALLBACK: 'trail-fallback-lines',

	TRAIL_NETWORK_IDS: [
		'trail-fallback-lines',
		'trail-local-lines',
		'trail-regional-lines',
		'trail-major-lines'
	] as const,

	TRAIL_LABELS: [
		'trail-major-labels',
		'trail-regional-labels',
		'trail-local-labels',
		'trail-fallback-labels'
	] as const,

	// Peaks and features
	OUTDOOR_ROCK: 'outdoor-rock',
	OUTDOOR_PEAK: 'outdoor-peaks',

	// Selected trail
	SELECTED_TRAIL_SOURCE: 'selected-trail',
	SELECTED_TRAIL_GLOW: 'selected-trail-glow',
	SELECTED_TRAIL_CASING: 'selected-trail-casing',
	SELECTED_TRAIL: 'selected-trail-line',
	SELECTED_TRAIL_ARROW: 'selected-trail-direction',
	SELECTED_TRAIL_MARKERS: 'selected-trail-markers',
	SELECTED_TRAIL_MARKER_LABELS: 'selected-trail-marker-labels',

	// Profile cursor
	SELECTED_TRAIL_CURSOR_SOURCE: 'selected-trail-profile-cursor',
	SELECTED_TRAIL_CURSOR: 'selected-trail-profile-cursor-point',

	// Map feature selection
	SELECTED_MAP_FEATURE_SOURCE: 'selected-map-feature',
	SELECTED_MAP_FEATURE_GLOW: 'selected-map-feature-glow',
	SELECTED_MAP_FEATURE: 'selected-map-feature-point'
};

export const TRAIL_NETWORKS = ['iwn', 'nwn', 'rwn', 'lwn'] as const;
