import { describe, expect, it } from 'vitest';
import { buildPeakTierFilter, OUTDOOR_PEAK_LAYER_IDS, PEAK_LABEL_TIERS } from './peak-config';

describe('peak-config', () => {
	it('reveals progressively denser peak tiers as users zoom in', () => {
		expect(PEAK_LABEL_TIERS.map((tier) => tier.minzoom)).toEqual([8, 10, 12, 14]);
		expect(OUTDOOR_PEAK_LAYER_IDS).toEqual([
			'outdoor-peaks-major',
			'outdoor-peaks-regional',
			'outdoor-peaks-local',
			'outdoor-peaks-unranked'
		]);
	});

	it('keeps the known elevation bands disjoint and physically plausible', () => {
		const [major, regional, local] = PEAK_LABEL_TIERS;
		expect(major.minElevationM).toBe(6400);
		expect(major.maxElevationM).toBe(9000);
		expect(regional.minElevationM).toBe(5500);
		expect(regional.maxElevationM).toBe(6400);
		expect(local.minElevationM).toBe(-500);
		expect(local.maxElevationM).toBe(5500);
	});

	it('requires named peaks and separates missing elevations', () => {
		const majorFilter = buildPeakTierFilter(PEAK_LABEL_TIERS[0]);
		const unrankedFilter = buildPeakTierFilter(PEAK_LABEL_TIERS[3]);

		expect(majorFilter).toEqual(
			expect.arrayContaining([
				'all',
				['==', ['get', 'kind'], 'peak'],
				['has', 'name'],
				['has', 'elevation_m']
			])
		);
		expect(unrankedFilter).toEqual(
			expect.arrayContaining([
				'all',
				['==', ['get', 'kind'], 'peak'],
				['!', ['has', 'elevation_m']]
			])
		);
	});
});
