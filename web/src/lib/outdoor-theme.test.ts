import { describe, expect, it } from 'vitest';
import { OUTDOOR_THEME } from './outdoor-theme';

function relativeLuminance(hex: string) {
	const channels = hex
		.match(/[0-9a-f]{2}/gi)
		?.map((channel) => Number.parseInt(channel, 16) / 255)
		.map((channel) =>
			channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4
		);
	if (!channels || channels.length !== 3) throw new Error(`Expected a six-digit hex color: ${hex}`);
	return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function contrastRatio(first: string, second: string) {
	const firstLuminance = relativeLuminance(first);
	const secondLuminance = relativeLuminance(second);
	return (
		(Math.max(firstLuminance, secondLuminance) + 0.05) /
		(Math.min(firstLuminance, secondLuminance) + 0.05)
	);
}

describe('outdoor theme', () => {
	it('keeps known-route tiers readable against their casing', () => {
		for (const color of [
			OUTDOOR_THEME.trail.international,
			OUTDOOR_THEME.trail.national,
			OUTDOOR_THEME.trail.regional,
			OUTDOOR_THEME.trail.local
		]) {
			expect(contrastRatio(color, OUTDOOR_THEME.route.casing)).toBeGreaterThanOrEqual(3);
		}
	});

	it('keeps the selected route and direction chevrons visually separate', () => {
		expect(contrastRatio(OUTDOOR_THEME.route.selected, OUTDOOR_THEME.route.casing)).toBeGreaterThanOrEqual(3);
		expect(contrastRatio(OUTDOOR_THEME.route.direction, OUTDOOR_THEME.route.selected)).toBeGreaterThanOrEqual(3);
		expect(OUTDOOR_THEME.route.selected).not.toBe(OUTDOOR_THEME.trail.regional);
	});

	it('keeps both endpoints distinct from the route casing', () => {
		expect(contrastRatio(OUTDOOR_THEME.route.start, OUTDOOR_THEME.route.casing)).toBeGreaterThanOrEqual(3);
		expect(contrastRatio(OUTDOOR_THEME.route.finish, OUTDOOR_THEME.route.casing)).toBeGreaterThanOrEqual(3);
	});
});
