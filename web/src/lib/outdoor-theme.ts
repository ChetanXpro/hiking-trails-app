/**
 * Shared color language for the outdoor map and its surrounding controls.
 *
 * The palette deliberately separates three jobs:
 * - evergreen identifies the product and interactive state;
 * - signal red identifies known hiking-route relations (not a maintenance guarantee);
 * - cobalt identifies the one route a hiker is actively following.
 */
export const OUTDOOR_THEME = {
	brand: '#2E6653',
	brandHover: '#245443',
	focus: '#79C69B',
	surface: {
		canvas: '#E8E3D7',
		elevated: '#FAF7EE',
		protected: '#A8C09A',
		forest: '#9DB084',
		grass: '#C5D09A',
		rock: '#D0C9BC',
		wetland: '#B0C3AF',
		residential: '#DFD8CC',
		ice: '#C0D9DE',
		sand: '#D5C5A5',
		water: '#7FBFD3',
		waterLine: '#3D8EB5',
		building: '#C9C1B5'
	},
	ink: {
		strong: '#25302A',
		muted: '#4F5C54',
		soft: '#68736B',
		inverse: '#FAF7EE'
	},
	route: {
		selected: '#245EE7',
		casing: '#FFF9EC',
		shadow: 'rgba(20, 42, 91, 0.58)',
		direction: '#FFFFFF',
		directionOutline: '#17355F',
		start: '#16875D',
		finish: '#2A3138',
		profileCursor: '#F2B544',
		offRoute: '#D95B32'
	},
	trail: {
		international: '#B91F24',
		national: '#C52225',
		regional: '#CF2D28',
		local: '#D43A32',
		fallback: '#626A63',
		label: '#9E241F'
	},
	contour: {
		minor: '#6B6560',
		intermediate: '#5E5751',
		index: '#4C453B',
		label: '#3D3430',
		halo: 'rgba(237, 232, 219, 0.95)'
	},
	label: {
		primary: '#25302A',
		secondary: '#46534B',
		path: '#72584B',
		water: '#347EAA',
		halo: '#FAF7EE'
	}
} as const;

export type OutdoorTheme = typeof OUTDOOR_THEME;
