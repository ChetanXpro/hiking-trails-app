import { it, expect } from 'vitest';
import { buildRouteIndex, nearestSampleIndex } from './profile-index';
it('finds nearest samples at boundaries and between samples', () => {
	const points = [0, 30, 60, 90].map(pos_m => ({pos_m}));
	expect([-1, 14, 16, 89, 999].map(x => nearestSampleIndex(points,x))).toEqual([0,0,1,3,3]);
});
it('does not add fictional distance across disconnected route parts', () => {
	const index = buildRouteIndex({ type: 'MultiLineString', coordinates: [[[0,0],[10,0]],[[100,0],[110,0]]] }, (a,b) => Math.abs(b[0]-a[0]));
	expect(index.total).toBe(20);
	expect(index.at(.75)).toEqual([105,0]);
	expect(index.at(1)).toEqual([110,0]);
});
