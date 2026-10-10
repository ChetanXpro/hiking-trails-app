/**
 * Scale-aware outdoor POI hierarchy.
 *
 * The local Martin source carries the trip anchors we control and can expose
 * before generic OpenMapTiles POIs arrive at z14. Provider POIs are then
 * released in ranked tiers, with the full local inventory reserved for z17+.
 */

import { APP_POI_ICON_NAMES } from './outdoor-poi-icons';

export type CustomOutdoorPoiTier = {
	id: string;
	minzoom: number;
	labelMinzoom: number;
	kinds: readonly string[];
	icon: string;
	priority: number;
};

export type ProviderOutdoorPoiTier = {
	key: string;
	minzoom: number;
	classValues: readonly string[];
	subclassValues: readonly string[];
};

export const PROVIDER_ALL_POIS_ZOOM = 17;
export const PROVIDER_RANK_LIMIT = 10;

export const CUSTOM_OUTDOOR_POI_TIERS: readonly CustomOutdoorPoiTier[] = [
	{
		id: 'outdoor-pois-emergency',
		minzoom: 10.5,
		labelMinzoom: 11.5,
		kinds: [
			'mountain_rescue',
			'ranger_station',
			'emergency_phone',
			'first_aid_kit',
			'defibrillator'
		],
		icon: APP_POI_ICON_NAMES.emergency,
		priority: 0
	},
	{
		id: 'outdoor-pois-overnight',
		minzoom: 10.5,
		labelMinzoom: 11.5,
		kinds: ['alpine_hut', 'wilderness_hut', 'basic_hut'],
		icon: APP_POI_ICON_NAMES.overnight,
		priority: 10
	},
	{
		id: 'outdoor-pois-medical',
		minzoom: 12.5,
		labelMinzoom: 13.5,
		kinds: ['hospital', 'clinic'],
		icon: APP_POI_ICON_NAMES.medical,
		priority: 15
	},
	{
		id: 'outdoor-pois-water',
		minzoom: 11.5,
		labelMinzoom: 12.5,
		kinds: ['drinking_water'],
		icon: APP_POI_ICON_NAMES.water,
		priority: 20
	},
	{
		id: 'outdoor-pois-trailheads',
		minzoom: 11.5,
		labelMinzoom: 12.5,
		kinds: ['trailhead'],
		icon: APP_POI_ICON_NAMES.trailhead,
		priority: 30
	},
	{
		id: 'outdoor-pois-camps',
		minzoom: 11.5,
		labelMinzoom: 13,
		kinds: ['camp_site'],
		icon: APP_POI_ICON_NAMES.camp,
		priority: 40
	},
	{
		id: 'outdoor-pois-springs',
		minzoom: 13.5,
		labelMinzoom: 15,
		kinds: ['spring'],
		icon: APP_POI_ICON_NAMES.spring,
		priority: 60
	},
	{
		id: 'outdoor-pois-shelters',
		minzoom: 13,
		labelMinzoom: 14.5,
		kinds: ['weather_shelter', 'lean_to', 'rock_shelter', 'shelter'],
		icon: APP_POI_ICON_NAMES.shelter,
		priority: 65
	},
	{
		id: 'outdoor-pois-local',
		minzoom: 14,
		labelMinzoom: 15,
		kinds: [
			'viewpoint',
			'waterfall',
			'cave_entrance',
			'saddle',
			'mountain_pass',
			'parking',
			'toilets',
			'guidepost',
			'information_map',
			'information_board',
			'information',
			'picnic_site',
			'aerialway_station'
		],
		icon: APP_POI_ICON_NAMES.local,
		priority: 70
	}
] as const;

export const PROVIDER_OUTDOOR_POI_TIERS: readonly ProviderOutdoorPoiTier[] = [
	{
		key: 'essentials',
		minzoom: 13.5,
		classValues: ['hospital'],
		subclassValues: [
			'shelter',
			'drinking_water',
			'trailhead',
			'ranger_station',
			'emergency_phone',
			'first_aid_kit',
			'defibrillator',
			'clinic',
			'pharmacy'
		]
	},
	{
		key: 'trip',
		minzoom: 14,
		classValues: ['aerialway'],
		subclassValues: [
			'viewpoint',
			'waterfall',
			'parking',
			'toilets',
			'cave_entrance',
			'picnic_site',
			'information',
			'guidepost'
		]
	},
	{
		key: 'resupply',
		minzoom: 15,
		classValues: ['grocery', 'cafe', 'fast_food', 'fuel', 'lodging'],
		subclassValues: ['restaurant', 'supermarket', 'convenience', 'bakery']
	},
	{
		key: 'local',
		minzoom: 16,
		classValues: [],
		subclassValues: [
			'bench',
			'fireplace',
			'bbq',
			'shower',
			'waste_basket',
			'drinking_water_tap'
		]
	}
] as const;

export function providerPoiLayerId(key: string, density: 'ranked' | 'all'): string {
	return `outdoor-pois-provider-${key}-${density}`;
}

export const OUTDOOR_POI_LAYER_IDS = [
	...CUSTOM_OUTDOOR_POI_TIERS.map((tier) => tier.id),
	...PROVIDER_OUTDOOR_POI_TIERS.flatMap((tier) => [
		providerPoiLayerId(tier.key, 'ranked'),
		providerPoiLayerId(tier.key, 'all')
	])
] as const;

const OUTDOOR_POI_LAYER_ID_SET = new Set<string>(OUTDOOR_POI_LAYER_IDS);

export function buildCustomPoiFilter(tier: CustomOutdoorPoiTier): unknown[] {
	return ['in', ['get', 'kind'], ['literal', [...tier.kinds]]];
}

export function buildProviderPoiFilter(
	tier: ProviderOutdoorPoiTier,
	density: 'ranked' | 'all'
): unknown[] {
	const categoryFilters: unknown[] = [];
	if (tier.classValues.length > 0) {
		categoryFilters.push(['in', ['get', 'class'], ['literal', [...tier.classValues]]]);
	}
	if (tier.subclassValues.length > 0) {
		categoryFilters.push(['in', ['get', 'subclass'], ['literal', [...tier.subclassValues]]]);
	}

	const categoryFilter =
		categoryFilters.length === 1 ? categoryFilters[0] : ['any', ...categoryFilters];
	if (density === 'all') return categoryFilter as unknown[];

	return [
		'all',
		categoryFilter,
		[
			'any',
			['!', ['has', 'rank']],
			['<', ['to-number', ['get', 'rank']], PROVIDER_RANK_LIMIT]
		]
	];
}

export function poiLabelExpression(labelMinzoom: number): unknown[] {
	return [
		'step',
		['zoom'],
		'',
		labelMinzoom,
		['coalesce', ['get', 'name_en'], ['get', 'name'], '']
	];
}

export const PROVIDER_POI_ICON_EXPRESSION: unknown[] = [
	'match',
	['get', 'class'],
	'campsite',
	'campsite',
	'hospital',
	'hospital',
	'aerialway',
	'aerialway',
	'lodging',
	'lodging',
	'grocery',
	'grocery',
	'fuel',
	'fuel',
	[
		'match',
		['get', 'subclass'],
		['shelter', 'alpine_hut', 'wilderness_hut', 'ranger_station'],
		'lodging',
		['drinking_water', 'waterfall', 'cave_entrance'],
		'attraction',
		['viewpoint', 'trailhead', 'picnic_site', 'information', 'guidepost'],
		'park',
		['parking'],
		'car',
		['emergency_phone', 'first_aid_kit', 'defibrillator', 'clinic', 'pharmacy'],
		'hospital',
		'attraction'
	]
];

export function isCuratedOutdoorPoiLayer(layerId: string): boolean {
	return OUTDOOR_POI_LAYER_ID_SET.has(layerId);
}

export function isBasemapPoiLayer(layerId: string, sourceLayer?: string): boolean {
	if (isCuratedOutdoorPoiLayer(layerId)) return false;
	const normalizedId = layerId.toLowerCase();
	return (
		sourceLayer?.toLowerCase() === 'poi' || /(^|[-_])poi([-_]|$)/.test(normalizedId)
	);
}
