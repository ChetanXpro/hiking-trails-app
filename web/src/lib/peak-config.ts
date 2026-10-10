export type PeakLabelTier = {
	id: string;
	minzoom: number;
	minElevationM?: number;
	maxElevationM?: number;
	unknownElevation?: boolean;
	textPadding: number;
	textSize: unknown[];
	textOpacity: unknown[];
};

/**
 * Reveal summit labels progressively so the regional map stays readable.
 * Elevation is an imperfect proxy for prominence, but it is the strongest
 * ranking signal currently available in the imported waypoint tiles.
 */
export const PEAK_LABEL_TIERS: PeakLabelTier[] = [
	{
		id: 'outdoor-peaks-major',
		minzoom: 8,
		minElevationM: 6400,
		maxElevationM: 9000,
		textPadding: 14,
		textSize: ['interpolate', ['linear'], ['zoom'], 8, 10.2, 11, 11, 14, 12],
		textOpacity: ['interpolate', ['linear'], ['zoom'], 8, 0.84, 10, 0.96]
	},
	{
		id: 'outdoor-peaks-regional',
		minzoom: 10,
		minElevationM: 5500,
		maxElevationM: 6400,
		textPadding: 10,
		textSize: ['interpolate', ['linear'], ['zoom'], 10, 9.8, 13, 11, 16, 12],
		textOpacity: ['interpolate', ['linear'], ['zoom'], 10, 0.76, 12, 0.92]
	},
	{
		id: 'outdoor-peaks-local',
		minzoom: 12,
		minElevationM: -500,
		maxElevationM: 5500,
		textPadding: 7,
		textSize: ['interpolate', ['linear'], ['zoom'], 12, 9.5, 15, 10.8, 18, 11.8],
		textOpacity: ['interpolate', ['linear'], ['zoom'], 12, 0.72, 14, 0.9]
	},
	{
		id: 'outdoor-peaks-unranked',
		minzoom: 14,
		unknownElevation: true,
		textPadding: 5,
		textSize: ['interpolate', ['linear'], ['zoom'], 14, 9.3, 16, 10.5, 18, 11.5],
		textOpacity: ['interpolate', ['linear'], ['zoom'], 14, 0.68, 16, 0.86]
	}
];

export const OUTDOOR_PEAK_LAYER_IDS = PEAK_LABEL_TIERS.map((tier) => tier.id);

export function buildPeakTierFilter(tier: PeakLabelTier): unknown[] {
	const filters: unknown[] = [
		['==', ['get', 'kind'], 'peak'],
		['has', 'name'],
		['!=', ['get', 'name'], '']
	];

	if (tier.unknownElevation) {
		filters.push(['!', ['has', 'elevation_m']]);
	} else {
		const elevation = ['to-number', ['get', 'elevation_m']];
		filters.push(['has', 'elevation_m']);
		if (tier.minElevationM !== undefined) {
			filters.push(['>=', elevation, tier.minElevationM]);
		}
		if (tier.maxElevationM !== undefined) {
			filters.push(['<', elevation, tier.maxElevationM]);
		}
	}

	return ['all', ...filters];
}
