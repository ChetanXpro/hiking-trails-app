import type { Map } from 'maplibre-gl';

export const APP_POI_ICON_NAMES = {
	emergency: 'hiking-poi-emergency',
	overnight: 'hiking-poi-overnight',
	medical: 'hiking-poi-medical',
	water: 'hiking-poi-water',
	trailhead: 'hiking-poi-trailhead',
	camp: 'hiking-poi-camp',
	spring: 'hiking-poi-spring',
	shelter: 'hiking-poi-shelter',
	local: 'hiking-poi-local'
} as const;

type IconGlyph = 'cross' | 'hut' | 'drop' | 'flag' | 'tent' | 'shelter' | 'view';

type IconSpec = {
	color: [number, number, number];
	glyph: IconGlyph;
};

export const APP_POI_ICON_SPECS: Record<string, IconSpec> = {
	[APP_POI_ICON_NAMES.emergency]: { color: [171, 55, 50], glyph: 'cross' },
	[APP_POI_ICON_NAMES.overnight]: { color: [63, 82, 65], glyph: 'hut' },
	[APP_POI_ICON_NAMES.medical]: { color: [183, 62, 57], glyph: 'cross' },
	[APP_POI_ICON_NAMES.water]: { color: [39, 112, 159], glyph: 'drop' },
	[APP_POI_ICON_NAMES.trailhead]: { color: [53, 105, 75], glyph: 'flag' },
	[APP_POI_ICON_NAMES.camp]: { color: [93, 83, 55], glyph: 'tent' },
	[APP_POI_ICON_NAMES.spring]: { color: [54, 126, 165], glyph: 'drop' },
	[APP_POI_ICON_NAMES.shelter]: { color: [91, 81, 58], glyph: 'shelter' },
	[APP_POI_ICON_NAMES.local]: { color: [94, 76, 57], glyph: 'view' }
};

const ICON_SIZE = 48;
const GLYPH_COLOR: [number, number, number] = [255, 252, 242];
const BORDER_COLOR: [number, number, number] = [255, 252, 242];

function paintPixel(
	data: Uint8Array,
	x: number,
	y: number,
	color: [number, number, number],
	alpha = 255
): void {
	if (x < 0 || y < 0 || x >= ICON_SIZE || y >= ICON_SIZE) return;
	const index = (y * ICON_SIZE + x) * 4;
	data[index] = color[0];
	data[index + 1] = color[1];
	data[index + 2] = color[2];
	data[index + 3] = alpha;
}

function glyphPixel(glyph: IconGlyph, x: number, y: number): boolean {
	const dx = x - 24;
	const dy = y - 23;
	switch (glyph) {
		case 'cross':
			return (Math.abs(dx) <= 2 && Math.abs(dy) <= 9) || (Math.abs(dy) <= 2 && Math.abs(dx) <= 9);
		case 'hut': {
			const roof = y >= 13 && y <= 22 && Math.abs(dx) <= y - 12;
			const body = y >= 22 && y <= 32 && Math.abs(dx) <= 9;
			return roof || body;
		}
		case 'drop': {
			const top = y >= 12 && y <= 23 && Math.abs(dx) <= Math.max(1, (y - 10) * 0.55);
			const bowl = y > 23 && y <= 33 && dx * dx + (y - 24) * (y - 24) <= 9 * 9;
			return top || bowl;
		}
		case 'flag':
			return (x >= 17 && x <= 20 && y >= 13 && y <= 34) || (x > 20 && x <= 32 && y >= 14 && y <= 22);
		case 'tent': {
			const leftEdge = Math.abs(x - (24 - (y - 14) * 0.62)) <= 1.25;
			const rightEdge = Math.abs(x - (24 + (y - 14) * 0.62)) <= 1.25;
			return y >= 14 && y <= 32 && (leftEdge || rightEdge || (y >= 30 && Math.abs(dx) <= 12));
		}
		case 'shelter': {
			const roof = y >= 14 && y <= 20 && Math.abs(dx) <= 12 - Math.abs(y - 19);
			const posts = y >= 20 && y <= 33 && (Math.abs(dx + 8) <= 1 || Math.abs(dx - 8) <= 1);
			return roof || posts || (y >= 31 && y <= 33 && Math.abs(dx) <= 9);
		}
		case 'view': {
			const distance = Math.sqrt((dx / 11) ** 2 + (dy / 6) ** 2);
			return (distance >= 0.78 && distance <= 1.18) || dx * dx + dy * dy <= 3.5 ** 2;
		}
	}
}

/** Produces an app-owned RGBA icon, independent of any basemap sprite. */
export function renderAppPoiIcon(name: string): { width: number; height: number; data: Uint8Array } {
	const spec = APP_POI_ICON_SPECS[name];
	if (!spec) throw new Error(`Unknown app POI icon: ${name}`);
	const data = new Uint8Array(ICON_SIZE * ICON_SIZE * 4);
	const centerX = 24;
	const centerY = 23;
	const radius = 18;
	for (let y = 0; y < ICON_SIZE; y += 1) {
		for (let x = 0; x < ICON_SIZE; x += 1) {
			const distance = Math.hypot(x - centerX, y - centerY);
			if (distance > radius + 0.5) continue;
			const alpha = Math.round(255 * Math.min(1, radius + 0.5 - distance));
			paintPixel(data, x, y, distance >= radius - 2.5 ? BORDER_COLOR : spec.color, alpha);
		}
	}
	for (let y = 8; y <= 37; y += 1) {
		for (let x = 8; x <= 40; x += 1) {
			if (glyphPixel(spec.glyph, x, y)) paintPixel(data, x, y, GLYPH_COLOR);
		}
	}
	return { width: ICON_SIZE, height: ICON_SIZE, data };
}

export function installAppOutdoorPoiImages(map: Map): void {
	for (const name of Object.keys(APP_POI_ICON_SPECS)) {
		if (map.hasImage(name)) continue;
		map.addImage(name, renderAppPoiIcon(name), { pixelRatio: 2 });
	}
}

