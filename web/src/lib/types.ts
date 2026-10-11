export type { WeatherForecast } from './weather';
export type { MapFeatureSelection } from './map-selection';

export type TrailWaypoint = {
	longitude: number | null;
	latitude: number | null;
	kind: string;
	name: string | null;
	elevation_m: number | null;
};

export type TrailSurface = {
	surface_type: string;
	distance_m: number;
	pct: number;
};

export type TrailGeometry = {
	type: string;
	coordinates?: unknown;
	geometries?: TrailGeometry[];
};

export type PointGeometry = {
	type: 'Point';
	coordinates: [number, number];
};

export type TrailDiagnostic = { kind: string; geometry: PointGeometry };
export type TrailWarning = { kind: string; message: string };

export type TrailDetail = {
	id: string;
	osm_relation_id: number;
	data_revision: string | null;
	last_synced_at: string | null;
	cautions?: Array<{ kind: string; message: string; source_url: string }>;
	name: string | null;
	network: string | null;
	difficulty: string | null;
	distance_m: number | null;
	ascent_m: number | null;
	descent_m: number | null;
	summary: string | null;
	hike_type: string | null;
	estimated_duration_min: number | null;
	default_direction_mode?: 'forward' | 'backward' | 'both' | null;
	direction_source?: string | null;
	source_kind: string;
	original_mapper_name?: string | null;
	original_mapper_provider?: string | null;
	original_mapper_user_id?: string | null;
	original_mapper_profile_url?: string | null;
	original_mapped_at?: string | null;
	original_mapper_source?: string | null;
	network_type?: string | null;
	route_tag?: string | null;
	route_from?: string | null;
	route_to?: string | null;
	route_via?: string | null;
	website_url?: string | null;
	video_url?: string | null;
	geometry: TrailGeometry | null;
	start_point: PointGeometry | null;
	end_point: PointGeometry | null;
	region_breadcrumb: string[];
	surfaces: TrailSurface[];
	features: string[];
	waypoints: TrailWaypoint[];
	stitch_diagnostics: TrailDiagnostic[];
	stitch_warnings: TrailWarning[];
};
