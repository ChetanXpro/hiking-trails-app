import { describe, expect, it } from 'vitest';
import {
	buildCustomPoiFilter,
	buildProviderPoiFilter,
	CUSTOM_OUTDOOR_POI_TIERS,
	isBasemapPoiLayer,
	isCuratedOutdoorPoiLayer,
	OUTDOOR_POI_LAYER_IDS,
	poiLabelExpression,
	PROVIDER_ALL_POIS_ZOOM,
	PROVIDER_OUTDOOR_POI_TIERS,
	providerPoiLayerId
} from './outdoor-poi-config';

describe('outdoor-poi-config', () => {
	it('puts emergency, overnight shelter, and potable water ahead of local comforts', () => {
		const byId = Object.fromEntries(CUSTOM_OUTDOOR_POI_TIERS.map((tier) => [tier.id, tier]));
		expect(byId['outdoor-pois-emergency'].minzoom).toBe(10.5);
		expect(byId['outdoor-pois-overnight'].minzoom).toBe(10.5);
		expect(byId['outdoor-pois-shelters'].minzoom).toBeGreaterThan(
			byId['outdoor-pois-overnight'].minzoom
		);
		expect(byId['outdoor-pois-medical'].minzoom).toBeGreaterThan(
			byId['outdoor-pois-emergency'].minzoom
		);
		expect(byId['outdoor-pois-water'].minzoom).toBeLessThan(
			byId['outdoor-pois-springs'].minzoom
		);
		expect(byId['outdoor-pois-local'].minzoom).toBeGreaterThanOrEqual(14);
	});

	it('reserves the early overnight tier for named mountain huts', () => {
		const huts = CUSTOM_OUTDOOR_POI_TIERS.find((tier) => tier.id === 'outdoor-pois-overnight');
		const shelters = CUSTOM_OUTDOOR_POI_TIERS.find(
			(tier) => tier.id === 'outdoor-pois-shelters'
		);
		expect(huts?.kinds).toEqual(['alpine_hut', 'wilderness_hut', 'basic_hut']);
		expect(shelters?.kinds).toEqual([
			'weather_shelter',
			'lean_to',
			'rock_shelter',
			'shelter'
		]);
	});

	it('does not present an unqualified spring as drinking water', () => {
		const potable = CUSTOM_OUTDOOR_POI_TIERS.find((tier) => tier.id === 'outdoor-pois-water');
		const springs = CUSTOM_OUTDOOR_POI_TIERS.find((tier) => tier.id === 'outdoor-pois-springs');
		expect(potable?.kinds).toEqual(['drinking_water']);
		expect(springs?.kinds).toEqual(['spring']);
		expect(springs!.minzoom).toBeGreaterThan(potable!.minzoom);
	});

	it('shows icons before labels for every locally controlled tier', () => {
		for (const tier of CUSTOM_OUTDOOR_POI_TIERS) {
			expect(tier.labelMinzoom).toBeGreaterThan(tier.minzoom);
			expect(buildCustomPoiFilter(tier)).toEqual([
				'in',
				['get', 'kind'],
				['literal', [...tier.kinds]]
			]);
			expect(poiLabelExpression(tier.labelMinzoom)).toEqual(
				expect.arrayContaining(['step', ['zoom'], '', tier.labelMinzoom])
			);
		}
	});

	it('uses local rank at z14-ish and releases all provider POIs at z17', () => {
		expect(PROVIDER_ALL_POIS_ZOOM).toBe(17);
		for (const tier of PROVIDER_OUTDOOR_POI_TIERS) {
			const ranked = buildProviderPoiFilter(tier, 'ranked');
			const all = buildProviderPoiFilter(tier, 'all');
			expect(ranked[0]).toBe('all');
			expect(JSON.stringify(ranked)).toContain('rank');
			expect(JSON.stringify(all)).not.toContain('rank');
			expect(OUTDOOR_POI_LAYER_IDS).toContain(providerPoiLayerId(tier.key, 'ranked'));
			expect(OUTDOOR_POI_LAYER_IDS).toContain(providerPoiLayerId(tier.key, 'all'));
		}
	});

	it('separates curated layers from generic basemap POI layers', () => {
		expect(isCuratedOutdoorPoiLayer('outdoor-pois-overnight')).toBe(true);
		expect(isBasemapPoiLayer('poi_r7', 'poi')).toBe(true);
		expect(isBasemapPoiLayer('outdoor-pois-provider-trip-ranked', 'poi')).toBe(false);
		expect(isBasemapPoiLayer('road-label', 'transportation_name')).toBe(false);
	});
});
