export type MapFeatureSource =
	| 'outdoor_waypoint'
	| 'basemap_poi'
	| 'basemap_feature'
	| 'marked_location';

export type MapFeatureSelection = {
	id: string;
	source: MapFeatureSource;
	title: string;
	kind: string;
	kindLabel: string;
	coordinates: [number, number];
	elevationM: number | null;
	elevationSource: 'mapped' | 'terrain' | null;
	osmType: 'N' | 'W' | 'R' | null;
	osmId: number | null;
	sourceLabel: string;
	layerId: string | null;
};

export type RenderedMapFeature = {
	layerId: string;
	source: string;
	sourceLayer?: string | null;
	properties?: Record<string, unknown> | null;
	coordinates: [number, number];
};

const KIND_LABELS: Record<string, string> = {
	aerialway_station: 'Aerialway station',
	alpine_hut: 'Alpine hut',
	basic_hut: 'Basic hut',
	camp_site: 'Campground',
	cave_entrance: 'Cave entrance',
	clinic: 'Clinic',
	defibrillator: 'Defibrillator',
	drinking_water: 'Drinking water',
	emergency_phone: 'Emergency phone',
	first_aid_kit: 'First aid',
	guidepost: 'Guidepost',
	hospital: 'Hospital',
	information: 'Trail information',
	information_board: 'Information board',
	information_map: 'Trail map',
	lean_to: 'Lean-to shelter',
	mountain_pass: 'Mountain pass',
	mountain_rescue: 'Mountain rescue',
	parking: 'Parking',
	peak: 'Peak',
	picnic_site: 'Picnic site',
	ranger_station: 'Ranger station',
	rock_shelter: 'Rock shelter',
	saddle: 'Saddle',
	shelter: 'Shelter',
	spring: 'Spring',
	toilets: 'Toilets',
	trailhead: 'Trailhead',
	viewpoint: 'Viewpoint',
	waterfall: 'Waterfall',
	waterway: 'Waterway',
	weather_shelter: 'Weather shelter',
	wilderness_hut: 'Wilderness hut'
};

function stringProperty(value: unknown): string | null {
	if (typeof value !== 'string') return null;
	const normalized = value.trim();
	return normalized.length > 0 ? normalized : null;
}

function numberProperty(value: unknown): number | null {
	const parsed = typeof value === 'number' ? value : Number.parseFloat(String(value ?? ''));
	return Number.isFinite(parsed) ? parsed : null;
}

function normalizeOsmType(value: unknown): 'N' | 'W' | 'R' | null {
	const normalized = stringProperty(value)?.charAt(0).toUpperCase();
	return normalized === 'N' || normalized === 'W' || normalized === 'R' ? normalized : null;
}

export function outdoorKindLabel(kind: string): string {
	const normalized = kind.trim().toLowerCase();
	if (KIND_LABELS[normalized]) return KIND_LABELS[normalized];
	if (!normalized) return 'Point of interest';
	return normalized
		.replace(/[_-]+/g, ' ')
		.replace(/\b\w/g, (letter) => letter.toUpperCase());
}

export function selectionFromRenderedFeature(
	feature: RenderedMapFeature
): MapFeatureSelection | null {
	const [longitude, latitude] = feature.coordinates;
	if (!Number.isFinite(longitude) || !Number.isFinite(latitude)) return null;
	const properties = feature.properties ?? {};
	const isLocal = feature.source === 'outdoor-waypoints';
	const sourceLayer = stringProperty(feature.sourceLayer)?.toLowerCase() ?? '';
	const isPoi =
		isLocal || sourceLayer === 'poi' || /(^|[-_])pois?([-_]|$)/.test(feature.layerId);
	const mappedName =
		stringProperty(properties.name_en) ??
		stringProperty(properties['name:en']) ??
		stringProperty(properties.name);
	const rawKind = isLocal
		? stringProperty(properties.kind)
		: stringProperty(properties.subclass) ??
			stringProperty(properties.class) ??
			stringProperty(properties.type) ??
			sourceLayer;
	const kind = (rawKind ?? 'point_of_interest').toLowerCase();
	const kindLabel = outdoorKindLabel(kind);
	if (!isPoi && !mappedName) return null;
	const title = mappedName ?? kindLabel;
	const osmType = isLocal ? normalizeOsmType(properties.osm_type) : null;
	const osmId = isLocal ? numberProperty(properties.osm_id) : null;
	const elevationM = isLocal ? numberProperty(properties.elevation_m) : null;
	const identity =
		osmType && osmId != null
			? `osm:${osmType}:${osmId}`
			: `${feature.source}:${feature.layerId}:${kind}:${title}:${longitude.toFixed(5)},${latitude.toFixed(5)}`;

	return {
		id: identity,
		source: isLocal ? 'outdoor_waypoint' : isPoi ? 'basemap_poi' : 'basemap_feature',
		title,
		kind,
		kindLabel,
		coordinates: [longitude, latitude],
		elevationM,
		elevationSource: elevationM == null ? null : 'mapped',
		osmType,
		osmId,
		sourceLabel: isLocal ? 'OpenStreetMap outdoor data' : 'OpenStreetMap basemap',
		layerId: feature.layerId
	};
}

export function markedLocationSelection(
	coordinates: [number, number]
): MapFeatureSelection {
	return {
		id: `marked:${coordinates[0].toFixed(6)},${coordinates[1].toFixed(6)}`,
		source: 'marked_location',
		title: 'Marked location',
		kind: 'marked_location',
		kindLabel: 'Map location',
		coordinates,
		elevationM: null,
		elevationSource: null,
		osmType: null,
		osmId: null,
		sourceLabel: 'Selected on the map',
		layerId: null
	};
}

export function dedupeMapSelections(
	selections: readonly MapFeatureSelection[]
): MapFeatureSelection[] {
	const seen = new Set<string>();
	const unique: MapFeatureSelection[] = [];
	for (const selection of selections) {
		const coordinateKey = `${selection.coordinates[0].toFixed(5)},${selection.coordinates[1].toFixed(5)}`;
		const semanticKey = `${selection.kind}:${selection.title.toLowerCase()}:${coordinateKey}`;
		const keys = [selection.id, semanticKey];
		if (keys.some((key) => seen.has(key))) continue;
		for (const key of keys) seen.add(key);
		unique.push(selection);
	}
	return unique;
}

export function openStreetMapUrl(selection: MapFeatureSelection, zoom = 16): string {
	if (selection.osmType && selection.osmId != null) {
		const objectType =
			selection.osmType === 'N' ? 'node' : selection.osmType === 'W' ? 'way' : 'relation';
		return `https://www.openstreetmap.org/${objectType}/${selection.osmId}`;
	}
	const [longitude, latitude] = selection.coordinates;
	return `https://www.openstreetmap.org/?mlat=${latitude.toFixed(6)}&mlon=${longitude.toFixed(6)}#map=${zoom}/${latitude.toFixed(6)}/${longitude.toFixed(6)}`;
}
