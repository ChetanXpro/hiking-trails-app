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
		canvas: '#EDE8DB',
		elevated: '#FAF7EE',
		protected: '#B2CBA2',
		forest: '#A6C095',
		grass: '#CED9AD',
		rock: '#D8D2C5',
		wetland: '#B9CDB8',
		residential: '#E7E0D4',
		ice: '#CBE4E9',
		sand: '#DFCFAF',
		water: '#8FC9DE',
		waterLine: '#4A99C0',
		building: '#D1C9BD'
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
		minor: '#7B766C',
		intermediate: '#6E675C',
		index: '#5C554B',
		label: '#51483E',
		halo: 'rgba(250, 247, 238, 0.98)'
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
