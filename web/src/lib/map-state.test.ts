import { describe, it, expect } from 'vitest';
import { readMapState, writeMapState, viewportBounds } from './map-state';
describe('shareable map state', () => {
	it('round trips a trail and view while retaining unrelated query parameters', () => {
		const id = '5685d17f-f1cd-4ad9-9f95-a51cd35af24b';
		const url = writeMapState(new URL('https://example.com/?other=yes'), id, { center: [76.3, 32.2], zoom: 12 });
		expect(readMapState(url)).toEqual({ trail: id, view: { center: [76.3, 32.2], zoom: 12 } });
		expect(url.searchParams.get('other')).toBe('yes');
		expect(readMapState(writeMapState(url, null, null)).trail).toBeNull();
	});
	it('ignores malformed links', () => {
		for (const map of ['NaN,32,76', '12,91,76', '99,32,76', '1,2', '12,32,181']) expect(readMapState(new URL(`https://x/?trail=bad&map=${map}`))).toEqual({ trail: null, view: null });
	});
	it('handles the antimeridian and a world-sized view', () => {
		expect(viewportBounds(170, -10, 190, 10)).toEqual([170, -10, -170, 10]);
		expect(viewportBounds(-200, -91, 200, 91)).toEqual([-180, -90, 180, 90]);
	});
});
