import { describe, expect, it } from 'vitest';
import {
	dedupeMapSelections,
	markedLocationSelection,
	openStreetMapUrl,
	outdoorKindLabel,
	selectionFromRenderedFeature
} from './map-selection';

describe('map selection', () => {
	it('normalizes an imported outdoor waypoint with mapped elevation and OSM identity', () => {
		const selection = selectionFromRenderedFeature({
			layerId: 'outdoor-pois-local',
			source: 'outdoor-waypoints',
			coordinates: [77.16697, 32.25406],
			properties: {
				osm_type: 'N',
				osm_id: '1234',
				kind: 'viewpoint',
				name: 'Manalsu Nala Viewpoint',
				elevation_m: '2055.4'
			}
		});
		expect(selection).toMatchObject({
			id: 'osm:N:1234',
			title: 'Manalsu Nala Viewpoint',
			kindLabel: 'Viewpoint',
			elevationM: 2055.4,
			elevationSource: 'mapped'
		});
		expect(openStreetMapUrl(selection!)).toBe('https://www.openstreetmap.org/node/1234');
	});

	it('uses provider class data without inventing an OSM identity', () => {
		const selection = selectionFromRenderedFeature({
			layerId: 'outdoor-pois-provider-trip-ranked',
			source: 'openmaptiles',
			coordinates: [77.2, 32.2],
			properties: { class: 'aerialway', name_en: 'Solang lift' }
		});
		expect(selection).toMatchObject({
			source: 'basemap_poi',
			title: 'Solang lift',
			kindLabel: 'Aerialway',
			osmType: null,
			osmId: null
		});
	});

	it('identifies a named basemap river at the clicked coordinate', () => {
		const selection = selectionFromRenderedFeature({
			layerId: 'waterway_line_label',
			source: 'openmaptiles',
			sourceLayer: 'waterway',
			coordinates: [77.16697, 32.25406],
			properties: { class: 'river', name: 'Manalsu Nala' }
		});
		expect(selection).toMatchObject({
			source: 'basemap_feature',
			title: 'Manalsu Nala',
			kindLabel: 'River',
			coordinates: [77.16697, 32.25406]
		});
	});

	it('does not invent a selectable name for anonymous basemap linework', () => {
		expect(
			selectionFromRenderedFeature({
				layerId: 'waterway_river',
				source: 'openmaptiles',
				sourceLayer: 'waterway',
				coordinates: [77.16697, 32.25406],
				properties: { class: 'river' }
			})
		).toBeNull();
	});

	it('falls back to a marked location and a coordinate OSM link', () => {
		const selection = markedLocationSelection([77.16697, 32.25406]);
		expect(selection.title).toBe('Marked location');
		expect(openStreetMapUrl(selection)).toContain('mlat=32.254060');
	});

	it('deduplicates the same semantic POI across local and provider layers', () => {
		const local = selectionFromRenderedFeature({
			layerId: 'outdoor-pois-local',
			source: 'outdoor-waypoints',
			coordinates: [77.16697, 32.25406],
			properties: { osm_type: 'N', osm_id: 1, kind: 'viewpoint', name: 'Lookout' }
		})!;
		const provider = selectionFromRenderedFeature({
			layerId: 'outdoor-pois-provider-trip-ranked',
			source: 'openmaptiles',
			coordinates: [77.16697, 32.25406],
			properties: { subclass: 'viewpoint', name: 'Lookout' }
		})!;
		expect(dedupeMapSelections([local, provider])).toEqual([local]);
	});

	it('humanizes unknown outdoor kinds', () => {
		expect(outdoorKindLabel('fire_watch_tower')).toBe('Fire Watch Tower');
	});
});
