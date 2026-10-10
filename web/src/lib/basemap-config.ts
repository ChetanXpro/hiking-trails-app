import { OUTDOOR_THEME } from './outdoor-theme';

export const DEFAULT_BASEMAP_STYLE_URL = 'https://tiles.openfreemap.org/styles/liberty';

export type BasemapLayerSummary = {
	id: string;
	type?: string;
	sourceLayer?: string;
};

export type BasemapPaintOverride = {
	property: string;
	value: unknown;
};

export type MutableBasemapLayer = BasemapLayerSummary & {
	'source-layer'?: string;
	layout?: Record<string, unknown>;
	paint?: Record<string, unknown>;
};

export type MutableBasemapSource = {
	url?: string;
	tiles?: string[];
	[key: string]: unknown;
};

export type MutableBasemapStyle = {
	layers: MutableBasemapLayer[];
	glyphs?: string;
	sprite?: string | Array<{ id?: string; url: string }>;
	sources?: Record<string, MutableBasemapSource>;
};

function labelPaint(color: string, haloWidth: number): BasemapPaintOverride[] {
	return [
		{ property: 'text-color', value: color },
		{ property: 'text-halo-color', value: OUTDOOR_THEME.label.halo },
		{ property: 'text-halo-width', value: haloWidth },
		{ property: 'text-halo-blur', value: 0.25 }
	];
}

const TERRAIN_PALETTE_OVERRIDES: Record<string, BasemapPaintOverride[]> = {
	background: [{ property: 'background-color', value: OUTDOOR_THEME.surface.canvas }],
	park: [
		{ property: 'fill-color', value: OUTDOOR_THEME.surface.protected },
		{
			property: 'fill-opacity',
			value: ['interpolate', ['linear'], ['zoom'], 5, 0.14, 10, 0.22, 14, 0.32, 17, 0.26]
		},
		{ property: 'fill-outline-color', value: 'rgba(46, 102, 83, 0.2)' }
	],
	park_outline: [
		{ property: 'line-color', value: 'rgba(46, 102, 83, 0.42)' },
		{ property: 'line-opacity', value: 0.54 },
		{ property: 'line-width', value: ['interpolate', ['linear'], ['zoom'], 8, 0.4, 14, 0.7, 17, 0.9] }
	],
	landuse_residential: [
		{ property: 'fill-color', value: OUTDOOR_THEME.surface.residential },
		{ property: 'fill-opacity', value: ['interpolate', ['linear'], ['zoom'], 8, 0.4, 12, 0.66] }
	],
	landcover_wood: [
		{ property: 'fill-color', value: OUTDOOR_THEME.surface.forest },
		{ property: 'fill-opacity', value: 0.82 },
		{ property: 'fill-outline-color', value: 'rgba(0, 0, 0, 0)' }
	],
	landcover_grass: [
		{ property: 'fill-color', value: OUTDOOR_THEME.surface.grass },
		{ property: 'fill-opacity', value: 0.7 },
		{ property: 'fill-outline-color', value: 'rgba(0, 0, 0, 0)' }
	],
	landcover_ice: [
		{ property: 'fill-color', value: OUTDOOR_THEME.surface.ice },
		{ property: 'fill-opacity', value: 0.94 },
		{ property: 'fill-outline-color', value: 'rgba(0, 0, 0, 0)' }
	],
	landcover_wetland: [
		{ property: 'fill-color', value: OUTDOOR_THEME.surface.wetland },
		{ property: 'fill-opacity', value: 0.68 },
		{ property: 'fill-outline-color', value: 'rgba(0, 0, 0, 0)' }
	],
	landcover_sand: [
		{ property: 'fill-color', value: OUTDOOR_THEME.surface.sand },
		{ property: 'fill-opacity', value: 0.9 },
		{ property: 'fill-outline-color', value: 'rgba(0, 0, 0, 0)' }
	],
	water: [
		{ property: 'fill-color', value: OUTDOOR_THEME.surface.water },
		{ property: 'fill-opacity', value: 0.98 }
	],
	waterway_tunnel: [
		{ property: 'line-color', value: OUTDOOR_THEME.surface.waterLine },
		{ property: 'line-opacity', value: 0.58 }
	],
	waterway_river: [
		{ property: 'line-color', value: OUTDOOR_THEME.surface.waterLine },
		{ property: 'line-opacity', value: 0.9 }
	],
	waterway_other: [
		{ property: 'line-color', value: OUTDOOR_THEME.surface.waterLine },
		{ property: 'line-opacity', value: 0.84 }
	],
	building: [
		{ property: 'fill-color', value: OUTDOOR_THEME.surface.building },
		{ property: 'fill-outline-color', value: 'rgba(142, 132, 117, 0.58)' }
	],
	road_path_pedestrian: [
		{ property: 'line-color', value: OUTDOOR_THEME.trail.fallback },
		{
			property: 'line-width',
			value: ['interpolate', ['linear'], ['zoom'], 14, 0.9, 17, 1.75, 20, 3.8]
		},
		{
			property: 'line-opacity',
			value: ['interpolate', ['linear'], ['zoom'], 14, 0.56, 16, 0.72, 19, 0.84]
		},
		{ property: 'line-dasharray', value: [1.7, 1.35] }
	],
	tunnel_path_pedestrian: [
		{ property: 'line-color', value: OUTDOOR_THEME.trail.fallback },
		{
			property: 'line-width',
			value: ['interpolate', ['linear'], ['zoom'], 14, 0.8, 17, 1.6, 20, 3.4]
		},
		{ property: 'line-opacity', value: 0.44 },
		{ property: 'line-dasharray', value: [1.1, 1.8] }
	],
	bridge_path_pedestrian_casing: [
		{ property: 'line-color', value: OUTDOOR_THEME.route.casing },
		{
			property: 'line-width',
			value: ['interpolate', ['linear'], ['zoom'], 14, 2.2, 17, 3.4, 20, 6.2]
		},
		{ property: 'line-opacity', value: 0.9 }
	],
	bridge_path_pedestrian: [
		{ property: 'line-color', value: OUTDOOR_THEME.trail.fallback },
		{
			property: 'line-width',
			value: ['interpolate', ['linear'], ['zoom'], 14, 1, 17, 1.85, 20, 4]
		},
		{ property: 'line-opacity', value: 0.82 },
		{ property: 'line-dasharray', value: [1.7, 1.15] }
	],
	waterway_line_label: labelPaint(OUTDOOR_THEME.label.water, 1.4),
	water_name_point_label: labelPaint(OUTDOOR_THEME.label.water, 1.5),
	water_name_line_label: labelPaint(OUTDOOR_THEME.label.water, 1.5),
	'highway-name-path': labelPaint(OUTDOOR_THEME.label.path, 1.35),
	'highway-name-minor': labelPaint(OUTDOOR_THEME.label.secondary, 1.4),
	'highway-name-major': labelPaint(OUTDOOR_THEME.ink.strong, 1.55),
	poi_r20: labelPaint(OUTDOOR_THEME.label.secondary, 1.25),
	poi_r7: labelPaint(OUTDOOR_THEME.label.secondary, 1.25),
	poi_r1: labelPaint(OUTDOOR_THEME.ink.strong, 1.4),
	label_other: labelPaint(OUTDOOR_THEME.label.secondary, 1.35),
	label_village: labelPaint(OUTDOOR_THEME.ink.strong, 1.45),
	label_town: labelPaint(OUTDOOR_THEME.ink.strong, 1.55),
	label_state: labelPaint(OUTDOOR_THEME.label.secondary, 1.5),
	label_city: labelPaint(OUTDOOR_THEME.ink.strong, 1.7),
	label_capital: labelPaint(OUTDOOR_THEME.ink.strong, 1.8),
	label_country: labelPaint(OUTDOOR_THEME.ink.strong, 1.8)
};

export function normalizeBasemapStyleUrl(value?: string | null): string | null {
	const trimmed = value?.trim();
	if (!trimmed) return DEFAULT_BASEMAP_STYLE_URL;
	if (trimmed === 'raster' || trimmed === 'legacy') return null;
	return trimmed;
}

export function shouldHideBasemapLayer(layer: BasemapLayerSummary): boolean {
	const id = layer.id.toLowerCase();
	const sourceLayer = layer.sourceLayer?.toLowerCase() ?? '';
	return /hillshade|contour|terrain|dem/.test(id) || /contour|hillshade/.test(sourceLayer);
}

export function terrainBasemapPaintOverrides(layer: BasemapLayerSummary): BasemapPaintOverride[] {
	const overrides = TERRAIN_PALETTE_OVERRIDES[layer.id.toLowerCase()] ?? [];
	const layerType = layer.type;
	if (!layerType) return overrides;
	return overrides.filter((override) => paintPropertyMatchesLayerType(layerType, override.property));
}

function paintPropertyMatchesLayerType(type: string, property: string): boolean {
	if (type === 'symbol') return property.startsWith('text-') || property.startsWith('icon-');
	return property.startsWith(`${type}-`);
}

function resolveStyleResourceUrl(value: string, styleUrl: string, documentUrl: string): string {
	try {
		const absoluteStyleUrl = new URL(styleUrl, documentUrl);
		return new URL(value, absoluteStyleUrl)
			.toString()
			.replaceAll('%7B', '{')
			.replaceAll('%7D', '}');
	} catch {
		return value;
	}
}

/**
 * Inline style objects lose the response URL MapLibre normally uses to resolve
 * relative glyph, sprite, TileJSON and tile-template URLs. Resolve those URLs
 * before passing the precomposed object to the renderer.
 */
export function absolutizeBasemapStyleResources<T extends MutableBasemapStyle>(
	style: T,
	styleUrl: string,
	documentUrl = 'http://localhost/'
): T {
	if (style.glyphs) {
		style.glyphs = resolveStyleResourceUrl(style.glyphs, styleUrl, documentUrl);
	}

	if (typeof style.sprite === 'string') {
		style.sprite = resolveStyleResourceUrl(style.sprite, styleUrl, documentUrl);
	} else if (Array.isArray(style.sprite)) {
		style.sprite = style.sprite.map((sprite) => ({
			...sprite,
			url: resolveStyleResourceUrl(sprite.url, styleUrl, documentUrl)
		}));
	}

	for (const source of Object.values(style.sources ?? {})) {
		if (source.url) {
			source.url = resolveStyleResourceUrl(source.url, styleUrl, documentUrl);
		}
		if (source.tiles) {
			source.tiles = source.tiles.map((tile) =>
				resolveStyleResourceUrl(tile, styleUrl, documentUrl)
			);
		}
	}

	return style;
}

/**
 * Applies the outdoor palette before MapLibre sees the style. This avoids a
 * first complete frame in the provider's original colors followed by a second
 * frame with runtime overrides.
 */
export function prepareOutdoorBasemapStyle<T extends MutableBasemapStyle>(style: T): T {
	for (const layer of style.layers) {
		const sourceLayer = layer.sourceLayer ?? layer['source-layer'];
		if (shouldHideBasemapLayer({ id: layer.id, type: layer.type, sourceLayer })) {
			layer.layout = { ...layer.layout, visibility: 'none' };
		}

		const overrides = terrainBasemapPaintOverrides({
			id: layer.id,
			type: layer.type,
			sourceLayer
		});
		if (overrides.length === 0) continue;

		layer.paint = { ...layer.paint };
		for (const override of overrides) {
			layer.paint[override.property] = override.value;
		}
	}
	return style;
}

export function findTerrainOverlayAnchor(layers: BasemapLayerSummary[]): string | undefined {
	return layers.find((layer) => {
		const sourceLayer = layer.sourceLayer?.toLowerCase() ?? '';
		return (
			layer.type !== 'symbol' &&
			/water|waterway|transportation|aeroway|building/.test(sourceLayer)
		);
	})?.id;
}

export function resolveTerrainOverlayAnchor(
	layers: BasemapLayerSummary[],
	fallbackLayerId: string
): string {
	return findTerrainOverlayAnchor(layers) ?? fallbackLayerId;
}

export function findFirstLabelAnchor(layers: BasemapLayerSummary[]): string | undefined {
	const preferred = layers.find(
		(layer) =>
			layer.type === 'symbol' &&
			/place|settlement|city|town|village|mountain|peak|water_name|transportation_name/i.test(
				layer.id
			)
	);
	if (preferred) return preferred.id;
	return layers.find((layer) => layer.type === 'symbol')?.id;
}
