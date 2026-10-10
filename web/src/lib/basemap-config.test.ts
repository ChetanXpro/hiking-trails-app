import { describe, expect, it } from 'vitest';
import {
	absolutizeBasemapStyleResources,
	DEFAULT_BASEMAP_STYLE_URL,
	findFirstLabelAnchor,
	findTerrainOverlayAnchor,
	normalizeBasemapStyleUrl,
	prepareOutdoorBasemapStyle,
	resolveTerrainOverlayAnchor,
	shouldHideBasemapLayer,
	terrainBasemapPaintOverrides,
	type MutableBasemapStyle
} from './basemap-config';
import { OUTDOOR_THEME } from './outdoor-theme';

describe('basemap-config', () => {
	it('defaults to the vector basemap when unset', () => {
		expect(normalizeBasemapStyleUrl(undefined)).toBe(DEFAULT_BASEMAP_STYLE_URL);
		expect(normalizeBasemapStyleUrl('')).toBe(DEFAULT_BASEMAP_STYLE_URL);
	});

	it('allows explicit raster fallback tokens', () => {
		expect(normalizeBasemapStyleUrl('raster')).toBeNull();
		expect(normalizeBasemapStyleUrl('legacy')).toBeNull();
	});

	it('finds a useful label anchor from a vector style', () => {
		expect(
			findFirstLabelAnchor([
				{ id: 'background', type: 'background' },
				{ id: 'road-major', type: 'line', sourceLayer: 'transportation' },
				{ id: 'place-city-lg', type: 'symbol', sourceLayer: 'place' },
				{ id: 'poi-level-1', type: 'symbol', sourceLayer: 'poi' }
			])
		).toBe('place-city-lg');
	});

	it('hides embedded terrain layers from the base style', () => {
		expect(shouldHideBasemapLayer({ id: 'hillshade', type: 'hillshade' })).toBe(true);
		expect(
			shouldHideBasemapLayer({ id: 'contours-index', type: 'line', sourceLayer: 'contour' })
		).toBe(true);
		expect(shouldHideBasemapLayer({ id: 'road-major', type: 'line', sourceLayer: 'transportation' })).toBe(false);
	});

	it('places terrain overlays above landcover but below hydrography and roads', () => {
		expect(
			findTerrainOverlayAnchor([
				{ id: 'background', type: 'background' },
				{ id: 'landcover_wood', type: 'fill', sourceLayer: 'landcover' },
				{ id: 'waterway_river', type: 'line', sourceLayer: 'waterway' },
				{ id: 'road_minor', type: 'line', sourceLayer: 'transportation' },
				{ id: 'label_city', type: 'symbol', sourceLayer: 'place' }
			])
		).toBe('waterway_river');
	});

	it('keeps terrain below app trails for the raster fallback style', () => {
		expect(
			resolveTerrainOverlayAnchor(
				[
					{ id: 'osm', type: 'raster' },
					{ id: 'trail-casing', type: 'line', sourceLayer: 'trail_lines' }
				],
				'trail-casing'
			)
		).toBe('trail-casing');
	});

	it('provides a richer outdoor surface palette without treating protected areas as landcover', () => {
		const parkOpacity = terrainBasemapPaintOverrides({ id: 'park' }).find(
			(override) => override.property === 'fill-opacity'
		);
		const forestColor = terrainBasemapPaintOverrides({ id: 'landcover_wood' }).find(
			(override) => override.property === 'fill-color'
		);
		const waterColor = terrainBasemapPaintOverrides({ id: 'water' }).find(
			(override) => override.property === 'fill-color'
		);

		expect(parkOpacity?.value).toEqual(
			expect.arrayContaining(['interpolate', expect.arrayContaining(['linear'])])
		);
		expect(forestColor?.value).toBe(OUTDOOR_THEME.surface.forest);
		expect(waterColor?.value).toBe(OUTDOOR_THEME.surface.water);
		expect(terrainBasemapPaintOverrides({ id: 'unrelated-layer' })).toEqual([]);
	});

	it('restyles Liberty labels with legible outdoor ink and warm halos', () => {
		const cityLabel = terrainBasemapPaintOverrides({ id: 'label_city', type: 'symbol' });
		const pathLabel = terrainBasemapPaintOverrides({ id: 'highway-name-path', type: 'symbol' });
		const waterLabel = terrainBasemapPaintOverrides({
			id: 'water_name_line_label',
			type: 'symbol'
		});

		expect(cityLabel).toEqual(
			expect.arrayContaining([
				{ property: 'text-color', value: OUTDOOR_THEME.ink.strong },
				{ property: 'text-halo-color', value: OUTDOOR_THEME.label.halo },
				{ property: 'text-halo-width', value: 1.7 }
			])
		);
		expect(pathLabel).toContainEqual({
			property: 'text-color',
			value: OUTDOOR_THEME.label.path
		});
		expect(waterLabel).toContainEqual({
			property: 'text-color',
			value: OUTDOOR_THEME.label.water
		});
	});

	it('turns generic Liberty footpaths into readable neutral dashed paths', () => {
		const path = terrainBasemapPaintOverrides({ id: 'road_path_pedestrian', type: 'line' });
		const bridge = terrainBasemapPaintOverrides({
			id: 'bridge_path_pedestrian_casing',
			type: 'line'
		});

		expect(path).toContainEqual({
			property: 'line-color',
			value: OUTDOOR_THEME.trail.fallback
		});
		expect(path).toContainEqual({ property: 'line-dasharray', value: [1.7, 1.35] });
		expect(bridge).toContainEqual({
			property: 'line-color',
			value: OUTDOOR_THEME.route.casing
		});
	});

	it('prepares the final outdoor style before MapLibre renders its first frame', () => {
		const style: MutableBasemapStyle = prepareOutdoorBasemapStyle({
			layers: [
				{ id: 'landcover_wood', type: 'fill', 'source-layer': 'landcover', paint: {} },
				{ id: 'provider-hillshade', type: 'hillshade', paint: {} }
			]
		});

		expect(style.layers[0].paint?.['fill-color']).toBe(OUTDOOR_THEME.surface.forest);
		expect(style.layers[1].layout?.visibility).toBe('none');
	});

	it('does not assign paint properties to an incompatible custom layer type', () => {
		expect(terrainBasemapPaintOverrides({ id: 'water', type: 'line' })).toEqual([]);
		expect(terrainBasemapPaintOverrides({ id: 'label_city', type: 'fill' })).toEqual([]);
	});

	it('preserves relative style assets when the style is passed inline', () => {
		const style: MutableBasemapStyle = absolutizeBasemapStyleResources(
			{
				glyphs: '../fonts/{fontstack}/{range}.pbf',
				sprite: './sprites/outdoor',
				sources: {
					basemap: {
						url: './tiles.json',
						tiles: ['../tiles/{z}/{x}/{y}.pbf']
					}
				},
				layers: []
			},
			'https://maps.example.com/styles/outdoor/style.json',
			'https://app.example.com/map'
		);

		expect(style.glyphs).toBe('https://maps.example.com/styles/fonts/{fontstack}/{range}.pbf');
		expect(style.sprite).toBe('https://maps.example.com/styles/outdoor/sprites/outdoor');
		expect(style.sources?.basemap.url).toBe(
			'https://maps.example.com/styles/outdoor/tiles.json'
		);
		expect(style.sources?.basemap.tiles).toEqual([
			'https://maps.example.com/styles/tiles/{z}/{x}/{y}.pbf'
		]);
	});
});
