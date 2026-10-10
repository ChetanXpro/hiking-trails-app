	<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { pushState, replaceState } from '$app/navigation';
	import { env } from '$env/dynamic/public';
	import maplibregl from 'maplibre-gl';
	import 'maplibre-gl/dist/maplibre-gl.css';
	import {
		absolutizeBasemapStyleResources,
		buildOsmDerivedVectorSource,
		findFirstLabelAnchor,
		normalizeBasemapStyleUrl,
		prepareOutdoorBasemapStyle,
		resolveTerrainOverlayAnchor,
		shouldHideBasemapLayer,
		terrainBasemapPaintOverrides,
		type MutableBasemapStyle
	} from './basemap-config';
	import WeatherSummary from './WeatherSummary.svelte';
	import TrailDiscovery from './TrailDiscovery.svelte';
	import ElevationChart from './ElevationChart.svelte';
	import { readMapState, writeMapState, viewportBounds } from './map-state';
	import { buildRouteIndex } from './profile-index';
	let { qaMode = false }: { qaMode?: boolean } = $props();
	import { installAppOutdoorPoiImages } from './outdoor-poi-icons';
	import { OUTDOOR_THEME } from './outdoor-theme';
	import {
		buildCustomPoiFilter,
		buildProviderPoiFilter,
		CUSTOM_OUTDOOR_POI_TIERS,
		isBasemapPoiLayer,
		OUTDOOR_POI_LAYER_IDS,
		poiLabelExpression,
		PROVIDER_ALL_POIS_ZOOM,
		PROVIDER_OUTDOOR_POI_TIERS,
		PROVIDER_POI_ICON_EXPRESSION,
		providerPoiLayerId,
		type CustomOutdoorPoiTier,
		type ProviderOutdoorPoiTier
	} from './outdoor-poi-config';
	import {
		buildPeakTierFilter,
		OUTDOOR_PEAK_LAYER_IDS,
		PEAK_LABEL_TIERS
	} from './peak-config';
	import {
		DEFAULT_TERRAIN_ATTRIBUTION,
		contourIntervalsAtZoom,
		DEFAULT_TERRAIN_BOUNDS,
		DEFAULT_TERRAIN_RGB_URL,
		normalizeTerrainBounds,
		normalizeTerrainMaxZoom
	} from './terrain-config';
	import { SLOPE_BANDS, SLOPE_CLEAR_BELOW, SLOPE_PLANNING_NOTE } from './slope-config';
	import {
		DEFAULT_WEATHER_API_URL,
		fetchWeatherForecast,
		type LngLatTuple,
		type WeatherForecast
	} from './weather';
	import { isAbortError, requestBlob, requestJson } from './request';
	import {
		dedupeMapSelections,
		markedLocationSelection,
		openStreetMapUrl,
		selectionFromRenderedFeature,
		type MapFeatureSelection
	} from './map-selection';
	import {
		addTerrainLayers,
		sampleTerrainElevation,
		set3DTerrainEnabled,
		setColorReliefVisible,
		setContourUnit,
		setContoursVisible,
		setHillshadeVisible,
		setSlopeOpacity,
		setSlopeVisible
	} from './terrain-layers';
	import { TerrainControls, LayersPanel } from './components/controls';
	import { TrailDetailPanel, FeaturePanel } from './components/sidebar';
	import { MapControls, Map3DView } from './components/map';

	const INITIAL_CENTER: [number, number] = [77.18, 32.24];
	const INITIAL_ZOOM = 9;
	function publicSetting(name: `PUBLIC_${string}`, legacyValue?: string) {
		return env[name]?.trim() || legacyValue?.trim() || undefined;
	}

	const CONFIGURED_BASEMAP_STYLE_URL = publicSetting(
		'PUBLIC_BASEMAP_STYLE_URL',
		import.meta.env.VITE_BASEMAP_STYLE_URL
	);
	const BASEMAP_STYLE_URL = CONFIGURED_BASEMAP_STYLE_URL
		? normalizeBasemapStyleUrl(CONFIGURED_BASEMAP_STYLE_URL)
		: import.meta.env.DEV
			? normalizeBasemapStyleUrl(undefined)
			: null;
	const BASEMAP_LABEL_ANCHOR_ID = publicSetting(
		'PUBLIC_BASEMAP_LABEL_ANCHOR_ID',
		import.meta.env.VITE_BASEMAP_LABEL_ANCHOR_ID
	);
	const CUSTOM_GLYPHS_URL = publicSetting(
		'PUBLIC_MAP_GLYPHS_URL',
		import.meta.env.VITE_MAP_GLYPHS_URL
	);
	const GLYPHS_URL =
		CUSTOM_GLYPHS_URL ??
		(import.meta.env.DEV
			? 'https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf'
			: undefined);
	const RASTER_TILE_URL_TEMPLATE =
		publicSetting('PUBLIC_RASTER_TILE_URL', import.meta.env.VITE_RASTER_TILE_URL) ??
		(import.meta.env.DEV ? 'https://tile.openstreetmap.org/{z}/{x}/{y}.png' : undefined);
	const RASTER_TILE_ATTRIBUTION =
		publicSetting(
			'PUBLIC_RASTER_TILE_ATTRIBUTION',
			import.meta.env.VITE_RASTER_TILE_ATTRIBUTION
		) ?? (import.meta.env.DEV ? '&copy; OpenStreetMap contributors' : undefined);
	const WEATHER_API_URL =
		publicSetting('PUBLIC_WEATHER_API_URL', import.meta.env.VITE_WEATHER_API_URL) ??
		(import.meta.env.DEV ? DEFAULT_WEATHER_API_URL : undefined);
	const MARTIN_TRAILS_URL_TEMPLATE =
		publicSetting('PUBLIC_MARTIN_TRAILS_URL', import.meta.env.VITE_MARTIN_TRAILS_URL) ??
		'/trail_lines/{z}/{x}/{y}';
	const MARTIN_OUTDOOR_WAYPOINTS_URL_TEMPLATE =
		publicSetting(
			'PUBLIC_MARTIN_OUTDOOR_WAYPOINTS_URL',
			import.meta.env.VITE_MARTIN_OUTDOOR_WAYPOINTS_URL
		) ?? '/outdoor_waypoints/{z}/{x}/{y}';
	const TERRAIN_RGB_URL_TEMPLATE =
		publicSetting('PUBLIC_TERRAIN_RGB_URL', import.meta.env.VITE_TERRAIN_RGB_URL) ??
		DEFAULT_TERRAIN_RGB_URL;
	const TERRAIN_RGB_ENCODING =
		publicSetting('PUBLIC_TERRAIN_RGB_ENCODING', import.meta.env.VITE_TERRAIN_RGB_ENCODING)?.toLowerCase() === 'terrarium'
			? 'terrarium'
			: 'mapbox';
	const TERRAIN_RGB_TILE_SIZE =
		Number.parseInt(
			publicSetting('PUBLIC_TERRAIN_RGB_TILE_SIZE', import.meta.env.VITE_TERRAIN_RGB_TILE_SIZE) ??
				'512',
			10
		) === 256
			? 256
			: 512;
	const TERRAIN_RGB_MAX_ZOOM = normalizeTerrainMaxZoom(
		publicSetting('PUBLIC_TERRAIN_RGB_MAX_ZOOM', import.meta.env.VITE_TERRAIN_RGB_MAX_ZOOM) ??
			'13'
	);
	const TERRAIN_RGB_BOUNDS = normalizeTerrainBounds(
		publicSetting('PUBLIC_TERRAIN_RGB_BOUNDS', import.meta.env.VITE_TERRAIN_RGB_BOUNDS),
		TERRAIN_RGB_URL_TEMPLATE === DEFAULT_TERRAIN_RGB_URL ? DEFAULT_TERRAIN_BOUNDS : undefined
	);
	const TERRAIN_ATTRIBUTION =
		publicSetting('PUBLIC_TERRAIN_ATTRIBUTION', import.meta.env.VITE_TERRAIN_ATTRIBUTION) ||
		(TERRAIN_RGB_URL_TEMPLATE === DEFAULT_TERRAIN_RGB_URL
			? DEFAULT_TERRAIN_ATTRIBUTION
			: undefined);
	const HOVERED_ROUTE_CASING_LAYER = 'hovered-route-casing';
	const HOVERED_ROUTE_LAYER = 'hovered-route-line';
	const TRAIL_LOCAL_CASING_LAYER = 'trail-local-casing';
	const TRAIL_REGIONAL_CASING_LAYER = 'trail-regional-casing';
	const TRAIL_MAJOR_CASING_LAYER = 'trail-major-casing';
	const TRAIL_LOCAL_LAYER = 'trail-local-lines';
	const TRAIL_REGIONAL_LAYER = 'trail-regional-lines';
	const TRAIL_MAJOR_LAYER = 'trail-major-lines';
	const TRAIL_FALLBACK_CASING_LAYER = 'trail-fallback-casing';
	const TRAIL_FALLBACK_LAYER = 'trail-fallback-lines';
	const TRAIL_NETWORK_LAYER_IDS = [
		TRAIL_FALLBACK_LAYER,
		TRAIL_LOCAL_LAYER,
		TRAIL_REGIONAL_LAYER,
		TRAIL_MAJOR_LAYER
	] as const;
	const TRAIL_LABEL_LAYER_IDS = [
		'trail-major-labels',
		'trail-regional-labels',
		'trail-local-labels',
		'trail-fallback-labels'
	] as const;
	const OUTDOOR_ROCK_LAYER_ID = 'outdoor-rock';
	const OUTDOOR_PEAK_LAYER_ID = OUTDOOR_PEAK_LAYER_IDS[0];
	const OUTDOOR_PEAK_LAYER_ID_SET = new Set(OUTDOOR_PEAK_LAYER_IDS);
	const SELECTED_TRAIL_SOURCE = 'selected-trail';
	const SELECTED_TRAIL_GLOW_LAYER = 'selected-trail-glow';
	const SELECTED_TRAIL_CASING_LAYER = 'selected-trail-casing';
	const SELECTED_TRAIL_LAYER = 'selected-trail-line';
	const SELECTED_TRAIL_ARROW_LAYER = 'selected-trail-direction';
	const SELECTED_TRAIL_ARROW_IMAGE = 'selected-trail-direction-chevron';
	const SELECTED_TRAIL_MARKER_LAYER = 'selected-trail-markers';
	const SELECTED_TRAIL_MARKER_LABEL_LAYER = 'selected-trail-marker-labels';
	const SELECTED_TRAIL_CURSOR_SOURCE = 'selected-trail-profile-cursor';
	const SELECTED_TRAIL_CURSOR_LAYER = 'selected-trail-profile-cursor-point';
	const SELECTED_MAP_FEATURE_SOURCE = 'selected-map-feature';
	const SELECTED_MAP_FEATURE_GLOW_LAYER = 'selected-map-feature-glow';
	const SELECTED_MAP_FEATURE_LAYER = 'selected-map-feature-point';
	const PROFILE_CURSOR_KIND = 'profile_cursor';
	const KNOWN_NETWORKS = ['iwn', 'nwn', 'rwn', 'lwn'] as const;

	type TrailGeometry = {
		type: string;
		coordinates?: unknown;
		geometries?: TrailGeometry[];
	};

	type PointGeometry = {
		type: 'Point';
		coordinates: [number, number];
	};

	type TrailDiagnostic = {
		kind: string;
		geometry: PointGeometry;
	};

	type GeoJsonFeature = {
		type: 'Feature';
		properties: Record<string, unknown>;
		geometry: TrailGeometry | PointGeometry;
	};

	type GeoJsonFeatureCollection = {
		type: 'FeatureCollection';
		features: GeoJsonFeature[];
	};

	type WarningSummary = {
		key: string;
		title: string;
		description: string;
		count: number;
	};

	type TrailWarning = {
		kind: string;
		message: string;
	};

	type DirectionMode = 'forward' | 'backward' | 'both';
	type DownloadFormat = 'gpx' | 'kml' | 'geojson';
	type MapPanel = 'layers' | 'terrain' | null;

	type RouteChoice = {
		id: string;
		name: string;
		network: string | null;
		tag: string | null;
	};

	type ElevationPoint = {
		pos_m: number;
		ele_m: number;
	};

	type ElevationProfile = {
		min_elevation_m: number | null;
		max_elevation_m: number | null;
		profile_points: ElevationPoint[];
	};

	type SearchRouteResult = {
		kind: 'trail';
		id: string;
		name: string;
		subtitle: string | null;
		network: string | null;
		distance_m: number | null;
		breadcrumb: string[];
	};

	type SearchPlaceResult = {
		kind: 'region' | 'place';
		id: string;
		name: string;
		place_kind: string | null;
		admin_label: string | null;
		breadcrumb: string[];
		bbox: [number, number, number, number] | null;
		center: [number, number] | null;
	};

	type SearchResult = SearchRouteResult | SearchPlaceResult;

	type SearchPayload = {
		routes: SearchRouteResult[];
		places: SearchPlaceResult[];
	};

	type RuntimeStyleLayer = {
		id: string;
		type?: string;
		source?: string;
		'source-layer'?: string;
	};

	type TrailDetail = {
		id: string;
		osm_relation_id: number;
		data_revision: string | null;
		last_synced_at: string | null;
		cautions?: Array<{kind: string; message: string; source_url: string}>;
		name: string | null;
		network: string | null;
		difficulty: string | null;
		distance_m: number | null;
		ascent_m: number | null;
		descent_m: number | null;
		summary: string | null;
		hike_type: string | null;
		estimated_duration_min: number | null;
		default_direction_mode?: DirectionMode | null;
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
		surfaces: Array<{
			surface_type: string;
			distance_m: number;
			pct: number;
		}>;
		features: string[];
		waypoints: Array<{
			longitude: number | null;
			latitude: number | null;
			kind: string;
			name: string | null;
			elevation_m: number | null;
		}>;
		stitch_diagnostics: TrailDiagnostic[];
		stitch_warnings: TrailWarning[];
	};

	let mapContainer: HTMLDivElement;
	let searchInput: HTMLInputElement | undefined;
	let map: maplibregl.Map | undefined;
	let scaleControl: maplibregl.ScaleControl | undefined;
	let waypointMarker: maplibregl.Marker | undefined;
	let selectedTrail: TrailDetail | null = $state(null);
	let selectedMapFeature: MapFeatureSelection | null = $state(null);
	let nearbyMapFeatures: MapFeatureSelection[] = $state([]);
	let nearbyTrailChoices: RouteChoice[] = $state([]);
	let selectedFeatureElevationStatus: 'idle' | 'loading' | 'ready' | 'missing' = $state('idle');
	let coordinateCopyStatus: 'idle' | 'copied' | 'error' = $state('idle');
	let mapFeatureRequestVersion = 0;
	let mapFeatureElevationAbortController: AbortController | null = null;
	let detailError: string | null = $state(null);
	let directionMode: DirectionMode = $state('forward');
	let downloadFormat: DownloadFormat = $state('gpx');
	let downloadSectionOpen = $state(false);
	let downloadBusy = $state(false);
	let downloadError: string | null = $state(null);
	let downloadAbortController: AbortController | null = null;
	let downloadRequestVersion = 0;
	let trailRequestVersion = 0;
	let trailDetailAbortController: AbortController | null = null;
	let elevationAbortController: AbortController | null = null;
	let routeChoices: RouteChoice[] = $state([]);
	let routeChooserPosition: { x: number; y: number } | null = $state(null);
	let hoveredRouteChoiceId: string | null = $state(null);
	let elevationProfile: ElevationProfile | null = $state(null);
	let elevationStatus: 'idle' | 'loading' | 'ready' | 'missing' | 'error' = $state('idle');
	let elevationError: string | null = $state(null);
	let elevationHoverPosM: number | null = $state(null);
	let elevationHoverEleM: number | null = $state(null);
	let terrainPanelOpen = $state(false);
	let terrainReady = $state(false);
	let mapVisualReady = $state(false);
	let mapLoadError: string | null = $state(null);
	let terrain3dEnabled = $state(false);
	let terrainHillshadeVisible = $state(true);
	let terrainContoursVisible = $state(true);
	let terrainColorReliefVisible = $state(false);
	let terrainSlopeVisible = $state(false);
	let terrainSlopeOpacity = $state(0.7);
	let terrainContourUnit: 'meters' | 'feet' = $state('meters');
	const TERRAIN_EXAGGERATION = 1.15;
	let terrainColorReliefSupported = $state(false);
	let terrainSlopeSupported = $state(false);
	let mapPitch = $state(0);
	let mapBearing = $state(0);
	let activeMapPanel: MapPanel = $state(null);
	let knownRoutesVisible = $state(true);
	let routeLabelsVisible = $state(true);
	let ordinaryPathsVisible = $state(true);
	let outdoorPoisVisible = $state(true);
	let peakLabelsVisible = $state(true);
	let regionBoundariesVisible = $state(true);
	let weatherForecast: WeatherForecast | null = $state(null);
	let weatherStatus: 'idle' | 'loading' | 'ready' | 'error' = $state('idle');
	let weatherError: string | null = $state(null);
	let weatherAbortController: AbortController | null = null;
	const weatherCache = new Map<string, { forecast: WeatherForecast; expiresAt: number }>();
	let searchQuery = $state('');
	let searchRoutes: SearchRouteResult[] = $state([]);
	let searchPlaces: SearchPlaceResult[] = $state([]);
	let searchResults: SearchResult[] = $state([]);
	let searchPanelOpen = $state(false);
	let searchLoading = $state(false);
	let searchError: string | null = $state(null);
	let activeSearchIndex = $state(-1);
	let searchRequestVersion = 0;
	let searchAbortController: AbortController | null = null;
	let searchDebounceHandle: ReturnType<typeof setTimeout> | null = null;
	let mapLabelAnchorId: string | undefined;
	let mapTerrainAnchorId: string | undefined;
	let mapRevealHandle: ReturnType<typeof setTimeout> | null = null;
	let componentDestroyed = false;
	const reportedMapErrors = new Set<string>();
	let detailPanelOpen = $derived(Boolean(selectedTrail || selectedMapFeature || detailError));

	let discoveryBounds: [number, number, number, number] | null = $state(null);
	let mapZoom = $state(INITIAL_ZOOM);
	let shareMessage = $state('');
	let shareFallback = $state('');
	let restoringLocation = false;
	let restorationVersion = 0;
	let navigationTrailId: string | null = null;
	const routeIndex = $derived.by(() => selectedTrail?.geometry ? buildRouteIndex(selectedTrail.geometry, coordinateDistanceM) : null);
	const incompleteRoute = $derived.by(() => selectedTrail?.stitch_warnings.some(w => ['disconnected_pieces', 'stranded_segments', 'missing_members'].includes(w.kind)) ?? false);
	const contourInterval = $derived(contourIntervalsAtZoom(terrainContourUnit, mapZoom));

	function showWaypoint(waypoint: TrailDetail['waypoints'][number]) {
		if (!map || waypoint.longitude == null || waypoint.latitude == null) return;
		const center: [number, number] = [waypoint.longitude, waypoint.latitude];
		waypointMarker?.remove();
		waypointMarker = new maplibregl.Marker({ color: '#d3a642' }).setLngLat(center).addTo(map);
		map.fitBounds([center, center], { maxZoom: Math.max(14, map.getZoom()), duration: 500,
			padding: window.innerWidth < 740 ? { top: 50, bottom: window.innerHeight * .6, left: 20, right: 20 } : { top: 70, bottom: 40, left: 500, right: 80 } });
	}

	function currentView() {
		return map ? { center: [map.getCenter().lng, map.getCenter().lat] as [number, number], zoom: map.getZoom() } : null;
	}
	function persistLocation(push = false) {
		if (restoringLocation || !map) return;
		const url = writeMapState(new URL(window.location.href), navigationTrailId, currentView());
		if (url.href !== window.location.href) (push ? pushState : replaceState)(url, {});
	}
	function updateViewport() {
		if (!map) return;
		const b = map.getBounds();
		discoveryBounds = viewportBounds(b.getWest(), b.getSouth(), b.getEast(), b.getNorth()).map(v => Number(v.toFixed(5))) as [number, number, number, number];
		mapZoom = map.getZoom();
		persistLocation();
	}
	async function restoreLocation() {
		const version = ++restorationVersion;
		const state = readMapState(new URL(window.location.href));
		restoringLocation = true;
		try {
			navigationTrailId = state.trail;
			if (state.view) map?.jumpTo({ ...state.view, padding: { top: 0, bottom: 0, left: 0, right: 0 } });
			if (state.trail) await loadTrail(state.trail, false, !state.view);
			else closePanel();
		} finally { if (version === restorationVersion) restoringLocation = false; }
	}
	async function shareTrail() {
		if (!selectedTrail) return;
		const url = writeMapState(new URL(window.location.href), selectedTrail.id, currentView()).href;
		shareMessage = 'Share this hike'; shareFallback = url;
		try {
			if (navigator.share) { await navigator.share({ title: selectedTrail.name ?? 'Hiking route', url }); shareMessage = 'Shared'; }
			else { await navigator.clipboard.writeText(url); shareMessage = 'Link copied'; shareFallback = ''; }
		} catch (error) {
			if (error instanceof Error && error.name === 'AbortError') return;
			shareFallback = url; shareMessage = 'Copy this link to share the hike.';
		}
	}

	// PostgreSQL's trigram index needs three characters to avoid turning every
	// short public search into a full scan as the place catalogue grows.
	const SEARCH_MIN_CHARS = 3;

	$effect(() => {
		if (!selectedTrail) return;
		directionMode;
		downloadSectionOpen;
		syncSelectedTrailLayer();
	});

	$effect(() => {
		selectedTrail;
		elevationHoverPosM;
		elevationHoverEleM;
		syncProfileCursorLayer();
	});

	$effect(() => {
		hoveredRouteChoiceId;
		syncHoveredRouteLayer();
	});

	$effect(() => {
		map;
		terrainReady;
		terrain3dEnabled;
		terrainHillshadeVisible;
		terrainContoursVisible;
		terrainColorReliefVisible;
		terrainSlopeVisible;
		terrainSlopeOpacity;
		terrainContourUnit;

		if (!map || !terrainReady) return;

		setHillshadeVisible(map, terrainHillshadeVisible);
		setContoursVisible(map, terrainContoursVisible);
		setColorReliefVisible(map, terrainColorReliefVisible);
		setSlopeVisible(map, terrainSlopeVisible);
		setSlopeOpacity(map, terrainSlopeOpacity);
		setContourUnit(map, terrainContourUnit);
		setOutdoorElevationUnit(terrainContourUnit);
		scaleControl?.setUnit(terrainContourUnit === 'feet' ? 'imperial' : 'metric');
		set3DTerrainEnabled(map, terrain3dEnabled, TERRAIN_EXAGGERATION);
	});

	$effect(() => {
		map;
		terrainReady;
		knownRoutesVisible;
		routeLabelsVisible;
		ordinaryPathsVisible;
		outdoorPoisVisible;
		peakLabelsVisible;
		regionBoundariesVisible;
		if (!map || !terrainReady) return;
		applyWorkspaceLayerVisibility();
	});

	onMount(() => {
		window.addEventListener('popstate', restoreLocation);
		void initializeMap().catch((error) => {
			mapLoadError = error instanceof Error ? error.message : 'The outdoor map could not be prepared.';
		});
	});

	async function loadPreparedMapStyle(): Promise<string | maplibregl.StyleSpecification> {
		if (!BASEMAP_STYLE_URL) return buildLegacyRasterStyle();

		const controller = new AbortController();
		const timeout = setTimeout(() => controller.abort(), 4500);
		try {
			const response = await fetch(BASEMAP_STYLE_URL, { signal: controller.signal });
			if (!response.ok) throw new Error(`Basemap style request failed with ${response.status}`);
			const style = (await response.json()) as maplibregl.StyleSpecification;
			const mutableStyle = style as unknown as MutableBasemapStyle;
			absolutizeBasemapStyleResources(
				mutableStyle,
				BASEMAP_STYLE_URL,
				typeof window === 'undefined' ? 'http://localhost/' : window.location.href
			);
			if (CUSTOM_GLYPHS_URL) mutableStyle.glyphs = CUSTOM_GLYPHS_URL;
			return prepareOutdoorBasemapStyle(mutableStyle) as unknown as maplibregl.StyleSpecification;
		} catch {
			// The loading veil still prevents a provider-style flash while MapLibre
			// falls back to loading the remote style directly.
			return BASEMAP_STYLE_URL;
		} finally {
			clearTimeout(timeout);
		}
	}

	async function initializeMap() {
		const martinTrailsUrl = absolutizeTileTemplate(MARTIN_TRAILS_URL_TEMPLATE);
		const martinOutdoorWaypointsUrl = absolutizeTileTemplate(
			MARTIN_OUTDOOR_WAYPOINTS_URL_TEMPLATE
		);
		const terrainRgbUrl = absolutizeTileTemplate(TERRAIN_RGB_URL_TEMPLATE);
		const initialState = readMapState(new URL(window.location.href));
		navigationTrailId = initialState.trail;
		const preparedStyle = await loadPreparedMapStyle();
		if (componentDestroyed) return;

		map = new maplibregl.Map({
			container: mapContainer,
			style: preparedStyle,
			center: initialState.view?.center ?? INITIAL_CENTER,
			zoom: initialState.view?.zoom ?? INITIAL_ZOOM,
			maxPitch: 60,
			centerClampedToGround: true,
			attributionControl: false,
			fadeDuration: 100
		});
		map.on('error', (event) => handleMapError(event.error));

		map.addControl(new maplibregl.AttributionControl({ compact: true }), 'bottom-right');
		map.addControl(new maplibregl.FullscreenControl(), 'top-right');
		map.addControl(new maplibregl.NavigationControl(), 'top-right');
		map.addControl(
			new maplibregl.GeolocateControl({
				positionOptions: { enableHighAccuracy: true },
				trackUserLocation: true,
				showAccuracyCircle: true
			}),
			'top-right'
		);
		scaleControl = new maplibregl.ScaleControl({ maxWidth: 140, unit: 'metric' });
		map.addControl(scaleControl, 'bottom-left');
		scheduleMapRevealFallback(10000);
		map.on('load', () => {
			if (!map) return;
			tuneBasemapForTerrain();
			mapLabelAnchorId = resolveLabelAnchorId();
			mapTerrainAnchorId = resolveTerrainAnchorId();
			addOutdoorContextLayers(
				martinOutdoorWaypointsUrl,
				mapTerrainAnchorId,
				mapLabelAnchorId
			);
			addTrailNetworkLayers(
				martinTrailsUrl,
				resolveOutdoorPoiAnchorId()
			);
			const capabilities = addTerrainLayers(map, {
				terrainRgbUrl,
				encoding: TERRAIN_RGB_ENCODING,
				tileSize: TERRAIN_RGB_TILE_SIZE,
				maxzoom: TERRAIN_RGB_MAX_ZOOM,
				bounds: TERRAIN_RGB_BOUNDS,
				attribution: TERRAIN_ATTRIBUTION,
				surfaceAnchorId: mapTerrainAnchorId,
				labelAnchorId: map.getLayer(OUTDOOR_PEAK_LAYER_ID)
					? OUTDOOR_PEAK_LAYER_ID
					: TRAIL_LOCAL_CASING_LAYER,
				terrainEnabled: terrain3dEnabled,
				hillshadeVisible: terrainHillshadeVisible,
				contoursVisible: terrainContoursVisible,
				colorReliefVisible: terrainColorReliefVisible,
				slopeVisible: terrainSlopeVisible,
				slopeOpacity: terrainSlopeOpacity,
				contourUnit: terrainContourUnit,
				exaggeration: TERRAIN_EXAGGERATION
			});
			terrainColorReliefSupported = capabilities.colorRelief;
			terrainSlopeSupported = capabilities.slope;
			terrainReady = true;
			applyWorkspaceLayerVisibility();
			if (selectedTrail) {
				syncSelectedTrailLayer();
				fitToSelectedTrail();
			}
			if (selectedMapFeature) syncSelectedMapFeatureLayer();

			for (const layerId of TRAIL_NETWORK_LAYER_IDS) {
				map.on('mouseenter', layerId, () => {
					map?.getCanvas().style.setProperty('cursor', 'pointer');
				});
				map.on('mouseleave', layerId, () => {
					map?.getCanvas().style.setProperty('cursor', '');
				});
			}
			for (const layerId of interactiveMapFeatureLayerIds()) {
				map.on('mouseenter', layerId, () => {
					map?.getCanvas().style.setProperty('cursor', 'pointer');
				});
				map.on('mouseleave', layerId, () => {
					map?.getCanvas().style.setProperty('cursor', '');
				});
			}

			void restoreLocation();
			updateViewport();
			revealMapAfterInitialRender();
		});

		map.on('pitch', () => {
			if (map) mapPitch = Math.round(map.getPitch());
		});
		map.on('rotate', () => {
			if (map) mapBearing = Math.round(map.getBearing());
		});

		map.on('moveend', updateViewport);
		map.on('click', (event) => void handleMapClick(event));
	}

	function handleMapError(error: unknown) {
		if (componentDestroyed || isAbortError(error)) return;
		const message =
			error instanceof Error
				? error.message
				: typeof error === 'string'
					? error
					: 'Unknown map rendering error';
		if (!reportedMapErrors.has(message) && reportedMapErrors.size < 5) {
			reportedMapErrors.add(message);
			console.warn('MapLibre:', message);
		}

		// Individual missing tiles are recoverable and should not cover a usable
		// map. Surface only initialization/rendering failures before first paint.
		if (
			!mapVisualReady &&
			/(style|webgl|failed to initialize|source .* not found|parse)/i.test(message)
		) {
			mapLoadError = 'The map could not finish loading its style or renderer. Check the configured map services and try again.';
		}
	}

	async function handleMapClick(event: maplibregl.MapMouseEvent) {
		closeSearchPanel();
		const pointFeatures = mapFeatureSelectionsAtPoint(event.point, [
			event.lngLat.lng,
			event.lngLat.lat
		]);
		const choices = routeChoicesAtPoint(event.point);
		if (pointFeatures.length > 0) {
			clearRouteChoices();
			await selectMapFeature(pointFeatures[0], pointFeatures.slice(1), choices);
			return;
		}

		if (choices.length === 0) {
			clearRouteChoices();
			await selectMapFeature(
				markedLocationSelection([event.lngLat.lng, event.lngLat.lat]),
				[],
				[]
			);
			return;
		}

		if (choices.length === 1) {
			clearRouteChoices();
			await loadTrail(choices[0].id);
			return;
		}

		routeChoices = choices;
		routeChooserPosition = routeChooserPositionFor(event.point, choices.length);
	}

	function interactivePointLayerIds() {
		if (!map) return [];
		return [...OUTDOOR_POI_LAYER_IDS, ...OUTDOOR_PEAK_LAYER_IDS].filter((layerId) =>
			Boolean(map?.getLayer(layerId))
		);
	}

	function interactiveNamedFeatureLayerIds() {
		return styleLayers()
			.filter((layer) => {
				if (layer.source !== 'openmaptiles') return false;
				const sourceLayer = layer['source-layer']?.toLowerCase() ?? '';
				if (layer.type === 'line' && sourceLayer === 'waterway') return true;
				return (
					layer.type === 'symbol' &&
					/^(place|water_name|waterway|transportation_name|park)$/.test(sourceLayer)
				);
			})
			.map((layer) => layer.id);
	}

	function interactiveMapFeatureLayerIds() {
		return [...new Set([...interactivePointLayerIds(), ...interactiveNamedFeatureLayerIds()])];
	}

	function mapFeatureSelectionsAtPoint(
		point: maplibregl.Point,
		clickedCoordinates: [number, number]
	) {
		if (!map) return [];
		const pointLayers = new Set(interactivePointLayerIds());
		const layers = interactiveMapFeatureLayerIds();
		if (layers.length === 0) return [];
		const rendered = map.queryRenderedFeatures(
			[
				[point.x - 9, point.y - 9],
				[point.x + 9, point.y + 9]
			],
			{ layers }
		);
		const ordered = [...rendered].sort((left, right) => {
			const leftPriority =
				left.source === 'outdoor-waypoints' ? 0 : pointLayers.has(left.layer.id) ? 1 : 2;
			const rightPriority =
				right.source === 'outdoor-waypoints' ? 0 : pointLayers.has(right.layer.id) ? 1 : 2;
			return leftPriority - rightPriority;
		});
		const selections = ordered.flatMap((feature) => {
			let coordinates = clickedCoordinates;
			if (pointLayers.has(feature.layer.id) && feature.geometry?.type === 'Point') {
				const pointCoordinates = feature.geometry.coordinates;
				if (
					Array.isArray(pointCoordinates) &&
					pointCoordinates.length >= 2 &&
					typeof pointCoordinates[0] === 'number' &&
					typeof pointCoordinates[1] === 'number'
				) {
					coordinates = [pointCoordinates[0], pointCoordinates[1]];
				}
			}
			const selection = selectionFromRenderedFeature({
				layerId: feature.layer.id,
				source: feature.source,
				sourceLayer: (feature.layer as RuntimeStyleLayer)['source-layer'],
				properties: feature.properties,
				coordinates
			});
			return selection ? [selection] : [];
		});
		return dedupeMapSelections(selections);
	}

	async function selectMapFeature(
		feature: MapFeatureSelection,
		nearby: MapFeatureSelection[],
		nearbyTrails: RouteChoice[]
	) {
		abortTrailRequests();
		mapFeatureElevationAbortController?.abort();
		const controller = new AbortController();
		mapFeatureElevationAbortController = controller;
		const requestVersion = ++mapFeatureRequestVersion;
		selectedTrail = null;
		navigationTrailId = null;
		persistLocation(true);
		waypointMarker?.remove();
		detailError = null;
		clearSelectedTrailLayer();
		selectedMapFeature = feature;
		nearbyMapFeatures = nearby;
		nearbyTrailChoices = nearbyTrails;
		coordinateCopyStatus = 'idle';
		selectedFeatureElevationStatus = feature.elevationM == null ? 'loading' : 'ready';
		syncSelectedMapFeatureLayer();
		keepSelectedMapFeatureVisible();
		void refreshWeather(feature.coordinates);

		if (feature.elevationM != null || !map) return;
		try {
			const elevationM = await sampleTerrainElevation(map, feature.coordinates, controller.signal);
			if (
				controller.signal.aborted ||
				requestVersion !== mapFeatureRequestVersion ||
				selectedMapFeature?.id !== feature.id
			) {
				return;
			}
			if (elevationM == null) {
				selectedFeatureElevationStatus = 'missing';
				return;
			}
			selectedMapFeature = {
				...selectedMapFeature,
				elevationM,
				elevationSource: 'terrain'
			};
			selectedFeatureElevationStatus = 'ready';
		} catch {
			if (requestVersion === mapFeatureRequestVersion) {
				selectedFeatureElevationStatus = 'missing';
			}
		}
	}

	function syncSelectedMapFeatureLayer() {
		if (!map) return;
		const features: GeoJsonFeature[] = selectedMapFeature
			? [
					{
						type: 'Feature',
						properties: { kind: selectedMapFeature.kind },
						geometry: {
							type: 'Point',
							coordinates: selectedMapFeature.coordinates
						}
					}
				]
			: [];
		const data: GeoJsonFeatureCollection = { type: 'FeatureCollection', features };
		const source = map.getSource(SELECTED_MAP_FEATURE_SOURCE) as
			| maplibregl.GeoJSONSource
			| undefined;
		if (source) {
			source.setData(data as never);
			return;
		}
		map.addSource(SELECTED_MAP_FEATURE_SOURCE, { type: 'geojson', data: data as never });
		addLayerIfMissing({
			id: SELECTED_MAP_FEATURE_GLOW_LAYER,
			type: 'circle',
			source: SELECTED_MAP_FEATURE_SOURCE,
			paint: {
				'circle-radius': ['interpolate', ['linear'], ['zoom'], 8, 10, 14, 14, 18, 17],
				'circle-color': 'rgba(255, 249, 236, 0.72)',
				'circle-blur': 0.35
			}
		});
		addLayerIfMissing({
			id: SELECTED_MAP_FEATURE_LAYER,
			type: 'circle',
			source: SELECTED_MAP_FEATURE_SOURCE,
			paint: {
				'circle-radius': ['interpolate', ['linear'], ['zoom'], 8, 5, 14, 7, 18, 8],
				'circle-color': '#2e6653',
				'circle-stroke-color': '#fff9ec',
				'circle-stroke-width': 2.5
			}
		});
	}

	function clearSelectedMapFeatureLayer() {
		selectedMapFeature = null;
		nearbyMapFeatures = [];
		nearbyTrailChoices = [];
		selectedFeatureElevationStatus = 'idle';
		coordinateCopyStatus = 'idle';
		mapFeatureElevationAbortController?.abort();
		mapFeatureElevationAbortController = null;
		mapFeatureRequestVersion += 1;
		if (!map) return;
		const source = map.getSource(SELECTED_MAP_FEATURE_SOURCE) as
			| maplibregl.GeoJSONSource
			| undefined;
		source?.setData({ type: 'FeatureCollection', features: [] });
	}

	function centerSelectedMapFeature() {
		if (!map || !selectedMapFeature) return;
		const mobile = (mapContainer?.clientWidth ?? window.innerWidth) <= 740;
		map.easeTo({
			center: selectedMapFeature.coordinates,
			zoom: Math.max(map.getZoom(), 14),
			offset: mobile ? [0, -110] : [220, 0],
			duration: 650
		});
	}

	function keepSelectedMapFeatureVisible() {
		if (!map || !selectedMapFeature) return;
		const projected = map.project(selectedMapFeature.coordinates);
		const width = mapContainer?.clientWidth ?? window.innerWidth;
		const height = mapContainer?.clientHeight ?? window.innerHeight;
		const mobile = width <= 740;
		const obscured = mobile ? projected.y > height * 0.48 : projected.x < Math.min(490, width * 0.48);
		if (!obscured) return;
		map.easeTo({
			center: selectedMapFeature.coordinates,
			offset: mobile ? [0, -110] : [220, 0],
			duration: 420
		});
	}

	async function copySelectedCoordinates() {
		if (!selectedMapFeature) return;
		const [longitude, latitude] = selectedMapFeature.coordinates;
		try {
			await navigator.clipboard.writeText(`${latitude.toFixed(6)}, ${longitude.toFixed(6)}`);
			coordinateCopyStatus = 'copied';
		} catch {
			coordinateCopyStatus = 'error';
		}
	}

	function featureCoordinateLabel(feature: MapFeatureSelection) {
		const [longitude, latitude] = feature.coordinates;
		return `${latitude.toFixed(5)}, ${longitude.toFixed(5)}`;
	}

	function selectNearbyMapFeature(feature: MapFeatureSelection) {
		const current = selectedMapFeature;
		const remaining = nearbyMapFeatures.filter((item) => item.id !== feature.id);
		void selectMapFeature(
			feature,
			dedupeMapSelections(current ? [current, ...remaining] : remaining),
			nearbyTrailChoices
		);
	}

	function toggleMapPanel(panel: Exclude<MapPanel, null>) {
		const opening = activeMapPanel !== panel;
		activeMapPanel = opening ? panel : null;
		terrainPanelOpen = activeMapPanel === 'terrain';
	}

	function preferredWeatherCoordinates(): LngLatTuple | null {
		const trailCoordinates = selectedTrail?.start_point?.coordinates;
		if (trailCoordinates) return trailCoordinates;
		if (!map) return null;
		const center = map.getCenter();
		return [center.lng, center.lat];
	}

	async function refreshWeather(
		coordinates = preferredWeatherCoordinates(),
		force = false
	) {
		if (!coordinates) return;
		if (!WEATHER_API_URL) {
			weatherForecast = null;
			weatherStatus = 'error';
			weatherError = 'Weather service is not configured for this deployment.';
			return;
		}
		weatherAbortController?.abort();
		weatherAbortController = null;
		const cacheKey = `${coordinates[0].toFixed(3)},${coordinates[1].toFixed(3)}`;
		const cached = weatherCache.get(cacheKey);
		if (!force && cached && cached.expiresAt > Date.now()) {
			weatherForecast = cached.forecast;
			weatherStatus = 'ready';
			weatherError = null;
			return;
		}
		const controller = new AbortController();
		weatherAbortController = controller;
		weatherStatus = 'loading';
		weatherError = null;
		try {
			const forecast = await fetchWeatherForecast(
				coordinates,
				controller.signal,
				10_000,
				WEATHER_API_URL
			);
			if (weatherAbortController !== controller) return;
			weatherForecast = forecast;
			weatherCache.delete(cacheKey);
			weatherCache.set(cacheKey, { forecast, expiresAt: Date.now() + 10 * 60 * 1000 });
			while (weatherCache.size > 24) {
				const oldestKey = weatherCache.keys().next().value;
				if (typeof oldestKey !== 'string') break;
				weatherCache.delete(oldestKey);
			}
			weatherStatus = 'ready';
		} catch (error) {
			if (controller.signal.aborted || weatherAbortController !== controller) return;
			weatherStatus = 'error';
			weatherError = error instanceof Error ? error.message : 'Weather forecast failed';
		} finally {
			if (weatherAbortController === controller) weatherAbortController = null;
		}
	}

	function revealMapAfterInitialRender() {
		if (!map) return;
		map.once('idle', markMapVisualReady);
		scheduleMapRevealFallback(3500);
	}

	function markMapVisualReady() {
		if (mapVisualReady || componentDestroyed) return;
		if (!map || !terrainReady || !map.isStyleLoaded()) return;
		mapVisualReady = true;
		mapLoadError = null;
		if (mapRevealHandle) {
			clearTimeout(mapRevealHandle);
			mapRevealHandle = null;
		}
	}

	function scheduleMapRevealFallback(delayMs: number) {
		if (mapRevealHandle) clearTimeout(mapRevealHandle);
		mapRevealHandle = setTimeout(() => {
			markMapVisualReady();
			if (!mapVisualReady && !componentDestroyed) {
				mapLoadError = 'Map layers are taking too long to load. Check the basemap or terrain service and try again.';
			}
		}, delayMs);
	}

	function buildLegacyRasterStyle() {
		if (!RASTER_TILE_URL_TEMPLATE) {
			throw new Error(
				'No production basemap configured. Set PUBLIC_BASEMAP_STYLE_URL or PUBLIC_RASTER_TILE_URL.'
			);
		}
		if (!GLYPHS_URL) {
			throw new Error(
				'Raster basemaps require PUBLIC_MAP_GLYPHS_URL for trail, contour, and place labels.'
			);
		}
		return {
			version: 8,
			glyphs: GLYPHS_URL,
			sources: {
				osm: {
					type: 'raster',
					tiles: [RASTER_TILE_URL_TEMPLATE],
					tileSize: 256,
					...(RASTER_TILE_ATTRIBUTION ? { attribution: RASTER_TILE_ATTRIBUTION } : {})
				}
			},
			layers: [{ id: 'osm', type: 'raster', source: 'osm' }]
		} satisfies maplibregl.StyleSpecification;
	}

	function addOutdoorContextLayers(
		outdoorWaypointsUrl: string,
		surfaceAnchorId?: string,
		labelAnchorId?: string
	) {
		if (!map) return;
		installAppOutdoorPoiImages(map);

		if (!map.getSource('outdoor-waypoints')) {
			map.addSource('outdoor-waypoints', buildOsmDerivedVectorSource(outdoorWaypointsUrl));
		}

		if (map.getSource('openmaptiles')) {
			addLayerIfMissing(
				{
					id: OUTDOOR_ROCK_LAYER_ID,
					type: 'fill',
					source: 'openmaptiles',
					'source-layer': 'landcover',
					filter: ['==', ['get', 'class'], 'rock'],
					paint: {
						'fill-color': OUTDOOR_THEME.surface.rock,
						'fill-opacity': 0.86,
						'fill-outline-color': 'rgba(0, 0, 0, 0)'
					}
				},
				surfaceAnchorId
			);

			for (const tier of [...PROVIDER_OUTDOOR_POI_TIERS].reverse()) {
				addProviderOutdoorPoiTier(tier, labelAnchorId);
			}
		}

		for (const [tierIndex, tier] of PEAK_LABEL_TIERS.entries()) {
			addLayerIfMissing(
				{
					id: tier.id,
					type: 'symbol',
					source: 'outdoor-waypoints',
					'source-layer': 'outdoor_waypoints',
					minzoom: tier.minzoom,
					filter: buildPeakTierFilter(tier) as maplibregl.FilterSpecification,
					layout: {
						'text-field': outdoorPeakTextField(terrainContourUnit),
						'text-font': ['Noto Sans Bold'],
						'text-size': tier.textSize as maplibregl.ExpressionSpecification,
						'text-line-height': 1.08,
						'text-max-width': 12,
						'text-padding': tier.textPadding,
						'text-pitch-alignment': 'map',
						'text-rotation-alignment': 'viewport',
						'text-allow-overlap': false,
						'text-ignore-placement': false,
						'symbol-avoid-edges': true,
						'symbol-sort-key': [
							'-',
							10000,
							['coalesce', ['to-number', ['get', 'elevation_m']], 0]
						]
					},
					paint: {
						'text-color': tierIndex === 0 ? '#49382B' : '#5A4939',
						'text-halo-color': OUTDOOR_THEME.label.halo,
						'text-halo-width': tierIndex === 0 ? 1.8 : 1.55,
						'text-halo-blur': 0.35,
						'text-opacity': tier.textOpacity as maplibregl.ExpressionSpecification
					}
				},
				labelAnchorId
			);
		}

		// Later symbol layers receive collision priority. Add lower-value local
		// amenities first and emergency/overnight anchors last.
		for (const tier of [...CUSTOM_OUTDOOR_POI_TIERS].reverse()) {
			addCustomOutdoorPoiTier(tier, labelAnchorId);
		}
	}

	function resolveOutdoorPoiAnchorId(): string | undefined {
		if (!map) return mapLabelAnchorId;
		return OUTDOOR_POI_LAYER_IDS.find((layerId) => map?.getLayer(layerId)) ?? mapLabelAnchorId;
	}

	function addCustomOutdoorPoiTier(tier: CustomOutdoorPoiTier, labelAnchorId?: string) {
		addLayerIfMissing(
			{
				id: tier.id,
				type: 'symbol',
				source: 'outdoor-waypoints',
				'source-layer': 'outdoor_waypoints',
				minzoom: tier.minzoom,
				filter: buildCustomPoiFilter(tier) as maplibregl.FilterSpecification,
				layout: {
					'icon-image': tier.icon,
					'icon-size': [
						'interpolate',
						['linear'],
						['zoom'],
						tier.minzoom,
						0.62,
						15,
						0.8,
						17,
						0.9
					],
					'icon-padding': 3,
					'icon-allow-overlap': false,
					'icon-ignore-placement': false,
					'icon-optional': false,
					'text-optional': true,
					'text-anchor': 'top',
					'text-field': poiLabelExpression(tier.labelMinzoom) as maplibregl.ExpressionSpecification,
					'text-font': ['Noto Sans Bold'],
					'text-size': ['interpolate', ['linear'], ['zoom'], 11, 9.5, 14, 10.5, 17, 11.5],
					'text-offset': [0, 0.8],
					'text-max-width': 11,
					'text-padding': 4,
					'text-pitch-alignment': 'map',
					'text-rotation-alignment': 'viewport',
					'text-allow-overlap': false,
					'text-ignore-placement': false,
					'symbol-avoid-edges': true,
					'symbol-sort-key': ['case', ['has', 'name'], tier.priority, tier.priority + 1]
				},
				paint: outdoorPoiTextPaint(tier.kinds)
			},
			labelAnchorId
		);
	}

	function addProviderOutdoorPoiTier(tier: ProviderOutdoorPoiTier, labelAnchorId?: string) {
		for (const density of ['ranked', 'all'] as const) {
			addLayerIfMissing(
				{
					id: providerPoiLayerId(tier.key, density),
					type: 'symbol',
					source: 'openmaptiles',
					'source-layer': 'poi',
					minzoom: density === 'ranked' ? tier.minzoom : PROVIDER_ALL_POIS_ZOOM,
					maxzoom: density === 'ranked' ? PROVIDER_ALL_POIS_ZOOM : 24,
					filter: buildProviderPoiFilter(tier, density) as maplibregl.FilterSpecification,
					layout: {
						'icon-image': PROVIDER_POI_ICON_EXPRESSION as maplibregl.ExpressionSpecification,
						'icon-size': ['interpolate', ['linear'], ['zoom'], 14, 0.6, 17, 0.8, 19, 0.92],
						'icon-padding': 3,
						'icon-allow-overlap': false,
						'icon-ignore-placement': false,
						'icon-optional': false,
						'text-optional': true,
						'text-anchor': 'top',
						'text-field': poiLabelExpression(tier.minzoom + 1) as maplibregl.ExpressionSpecification,
						'text-font': ['Noto Sans Bold'],
						'text-size': ['interpolate', ['linear'], ['zoom'], 14, 9.5, 17, 11.5, 19, 12],
						'text-offset': [0, 0.82],
						'text-max-width': 11,
						'text-padding': 4,
						'text-pitch-alignment': 'map',
						'text-rotation-alignment': 'viewport',
						'text-allow-overlap': false,
						'text-ignore-placement': false,
						'symbol-avoid-edges': true,
						'symbol-sort-key': ['coalesce', ['to-number', ['get', 'rank']], 99]
					},
					paint: outdoorProviderPoiTextPaint() as never
				},
				labelAnchorId
			);
		}
	}

	function outdoorPoiTextPaint(kinds: readonly string[]) {
		const water = kinds.includes('drinking_water') || kinds.includes('spring');
		const emergency = kinds.some((kind) =>
			['mountain_rescue', 'emergency_phone', 'first_aid_kit', 'defibrillator', 'hospital', 'clinic'].includes(kind)
		);
		return {
			'text-color': emergency ? '#A63B32' : water ? OUTDOOR_THEME.label.water : OUTDOOR_THEME.label.secondary,
			'text-halo-color': OUTDOOR_THEME.label.halo,
			'text-halo-width': 1.6,
			'text-halo-blur': 0.3
		};
	}

	function outdoorProviderPoiTextPaint() {
		return {
			'text-color': [
				'match',
				['get', 'subclass'],
				['drinking_water', 'waterfall'],
				OUTDOOR_THEME.label.water,
				['emergency_phone', 'first_aid_kit', 'defibrillator'],
				'#A63B32',
				OUTDOOR_THEME.label.secondary
			],
			'text-halo-color': OUTDOOR_THEME.label.halo,
			'text-halo-width': 1.5,
			'text-halo-blur': 0.3
		};
	}

	function outdoorPeakTextField(unit: 'meters' | 'feet'): maplibregl.ExpressionSpecification {
		const suffix = unit === 'feet' ? ' ft' : ' m';
		const rawName = ['coalesce', ['get', 'name'], ''];
		const displayName = [
			'case',
			['>', ['length', rawName], 34],
			['concat', ['slice', rawName, 0, 34], '…'],
			rawName
		];
		const elevation =
			unit === 'feet'
				? ['*', ['to-number', ['get', 'elevation_m']], 3.28084]
				: ['to-number', ['get', 'elevation_m']];
		return [
			'concat',
			'▲ ',
			displayName,
			[
				'case',
				['has', 'elevation_m'],
				['concat', '\n', ['to-string', ['round', elevation]], suffix],
				''
			]
		] as unknown as maplibregl.ExpressionSpecification;
	}

	function setOutdoorElevationUnit(unit: 'meters' | 'feet') {
		if (!map) return;
		for (const layerId of OUTDOOR_PEAK_LAYER_IDS) {
			if (!map.getLayer(layerId)) continue;
			map.setLayoutProperty(layerId, 'text-field', outdoorPeakTextField(unit) as never);
		}
	}

	function addTrailNetworkLayers(trailTilesUrl: string, beforeLayerId?: string) {
		if (!map) return;

		if (!map.getSource('trails')) {
			map.addSource('trails', buildOsmDerivedVectorSource(trailTilesUrl));
		}

		addKnownRouteLineLayer(
			TRAIL_FALLBACK_CASING_LAYER,
			null,
			11.5,
			[11.5, 2.4, 12, 2.8, 14, 3.7, 16, 4.5, 18, 5],
			OUTDOOR_THEME.route.casing,
			[11.5, 0.38, 12, 0.62, 13, 0.78, 14, 0.88],
			beforeLayerId
		);

		addKnownRouteLineLayer(
			TRAIL_LOCAL_CASING_LAYER,
			['lwn'],
			11,
			[11, 2.4, 12, 2.8, 14, 3.7, 16, 4.5, 18, 5],
			OUTDOOR_THEME.route.casing,
			[11, 0.42, 12, 0.68, 13, 0.82, 14, 0.9],
			beforeLayerId
		);
		addKnownRouteLineLayer(
			TRAIL_REGIONAL_CASING_LAYER,
			['rwn'],
			9.5,
			[9.5, 2, 10, 2.3, 12, 3.4, 14, 4.2, 16, 5, 18, 5.6],
			OUTDOOR_THEME.route.casing,
			[9.5, 0.34, 10, 0.62, 12, 0.84, 14, 0.92],
			beforeLayerId
		);
		addKnownRouteLineLayer(
			TRAIL_MAJOR_CASING_LAYER,
			['iwn', 'nwn'],
			8,
			[8, 2.1, 10, 2.8, 12, 3.8, 14, 4.8, 16, 5.8, 18, 6.5],
			OUTDOOR_THEME.route.casing,
			[8, 0.46, 10, 0.72, 12, 0.88, 14, 0.94],
			beforeLayerId
		);

		addKnownRouteLineLayer(
			TRAIL_FALLBACK_LAYER,
			null,
			11.5,
			[11.5, 0.85, 12, 1.25, 14, 1.8, 16, 2.4, 18, 2.9],
			OUTDOOR_THEME.trail.local,
			[11.5, 0.44, 12, 0.7, 13, 0.88, 14, 0.96],
			beforeLayerId
		);
		addKnownRouteLineLayer(
			TRAIL_LOCAL_LAYER,
			['lwn'],
			11,
			[11, 0.9, 12, 1.35, 14, 1.9, 16, 2.5, 18, 3],
			OUTDOOR_THEME.trail.local,
			[11, 0.5, 12, 0.76, 13, 0.92, 14, 0.98],
			beforeLayerId
		);
		addKnownRouteLineLayer(
			TRAIL_REGIONAL_LAYER,
			['rwn'],
			9.5,
			[9.5, 0.8, 10, 1.2, 12, 1.8, 14, 2.3, 16, 3, 18, 3.5],
			OUTDOOR_THEME.trail.regional,
			[9.5, 0.4, 10, 0.72, 12, 0.94, 14, 0.99],
			beforeLayerId
		);
		addKnownRouteLineLayer(
			TRAIL_MAJOR_LAYER,
			['iwn', 'nwn'],
			8,
			[8, 1.2, 10, 1.7, 12, 2.2, 14, 2.8, 16, 3.6, 18, 4.2],
			[
				'match',
				['get', 'network'],
				'iwn',
				OUTDOOR_THEME.trail.international,
				OUTDOOR_THEME.trail.national
			],
			[8, 0.65, 10, 0.84, 12, 0.95, 14, 0.99],
			beforeLayerId
		);

		addLayerIfMissing(
			{
				id: HOVERED_ROUTE_CASING_LAYER,
				type: 'line',
				source: 'trails',
				'source-layer': 'trail_lines',
				filter: ['==', ['get', 'id'], ''],
				layout: {
					'line-cap': 'round',
					'line-join': 'round'
				},
				paint: {
					'line-color': OUTDOOR_THEME.route.casing,
					'line-width': ['interpolate', ['linear'], ['zoom'], 9, 5, 12, 6, 14, 7, 16, 8.5],
					'line-opacity': 0.98,
					'line-blur': 0.2
				}
			},
			beforeLayerId
		);
		addLayerIfMissing(
			{
				id: HOVERED_ROUTE_LAYER,
				type: 'line',
				source: 'trails',
				'source-layer': 'trail_lines',
				filter: ['==', ['get', 'id'], ''],
				layout: {
					'line-cap': 'round',
					'line-join': 'round'
				},
				paint: {
					'line-color': OUTDOOR_THEME.route.selected,
					'line-width': ['interpolate', ['linear'], ['zoom'], 9, 2.8, 12, 3.6, 14, 4.4, 16, 5.4],
					'line-opacity': 0.98
				}
			},
			beforeLayerId
		);
		addKnownRouteLabelLayer('trail-major-labels', ['iwn', 'nwn'], 10.5, true, beforeLayerId);
		addKnownRouteLabelLayer('trail-regional-labels', ['rwn'], 11.5, false, beforeLayerId);
		addKnownRouteLabelLayer('trail-local-labels', ['lwn'], 13, false, beforeLayerId);
		addKnownRouteLabelLayer('trail-fallback-labels', null, 14, false, beforeLayerId);
	}

	function addKnownRouteLineLayer(
		id: string,
		networks: string[] | null,
		minzoom: number,
		widthStops: number[],
		color: unknown,
		opacityStops: number[],
		beforeLayerId?: string
	) {
		addLayerIfMissing(
			{
				id,
				type: 'line',
				source: 'trails',
				'source-layer': 'trail_lines',
				minzoom,
				filter: knownRouteNetworkFilter(networks),
				layout: {
					'line-cap': 'round',
					'line-join': 'round'
				},
				paint: {
					'line-color': color as never,
					'line-width': ['interpolate', ['linear'], ['zoom'], ...widthStops] as never,
					'line-opacity': ['interpolate', ['linear'], ['zoom'], ...opacityStops] as never
				}
			},
			beforeLayerId
		);
	}

	function addKnownRouteLabelLayer(
		id: string,
		networks: string[] | null,
		minzoom: number,
		uppercase: boolean,
		beforeLayerId?: string
	) {
		addLayerIfMissing(
			{
				id,
				type: 'symbol',
				source: 'trails',
				'source-layer': 'trail_lines',
				minzoom,
				filter: [
					'all',
					knownRouteNetworkFilter(networks),
					['has', 'name'],
					['!=', ['get', 'name'], '']
				] as never,
				layout: {
					'symbol-placement': 'line',
					'symbol-spacing': [
						'interpolate',
						['linear'],
						['zoom'],
						10,
						900,
						11,
						700,
						12,
						560,
						14,
						450,
						16,
						340
					],
					'text-field': ['get', 'name'],
					'text-font': ['Noto Sans Bold'],
					'text-size': ['interpolate', ['linear'], ['zoom'], 10, 10.3, 12, 11, 14, 11.7, 16, 12.4],
					'text-transform': uppercase ? 'uppercase' : 'none',
					'text-letter-spacing': 0.01,
					'text-max-angle': 30,
					'text-padding': 6,
					'text-pitch-alignment': 'map',
					'text-rotation-alignment': 'viewport-glyph',
					'text-allow-overlap': false,
					'text-ignore-placement': false
				},
				paint: {
					'text-color': OUTDOOR_THEME.trail.label,
					'text-halo-color': OUTDOOR_THEME.route.casing,
					'text-halo-width': 1.7,
					'text-halo-blur': 0.3
				}
			},
			beforeLayerId
		);
	}

	function knownRouteNetworkFilter(networks: string[] | null): maplibregl.FilterSpecification {
		if (networks) return ['in', ['get', 'network'], ['literal', networks]] as never;
		return [
			'!',
			['in', ['coalesce', ['get', 'network'], ''], ['literal', [...KNOWN_NETWORKS]]]
		] as never;
	}

	function applyWorkspaceLayerVisibility() {
		if (!map) return;
		for (const layerId of [
			TRAIL_FALLBACK_CASING_LAYER,
			TRAIL_FALLBACK_LAYER,
			TRAIL_LOCAL_CASING_LAYER,
			TRAIL_REGIONAL_CASING_LAYER,
			TRAIL_MAJOR_CASING_LAYER,
			HOVERED_ROUTE_CASING_LAYER,
			HOVERED_ROUTE_LAYER,
			...TRAIL_NETWORK_LAYER_IDS
		]) {
			setLayerVisibilitySafe(layerId, knownRoutesVisible);
		}
		for (const layerId of TRAIL_LABEL_LAYER_IDS) {
			setLayerVisibilitySafe(layerId, knownRoutesVisible && routeLabelsVisible);
		}
		for (const layerId of OUTDOOR_POI_LAYER_IDS) {
			setLayerVisibilitySafe(layerId, outdoorPoisVisible);
		}
		for (const layerId of OUTDOOR_PEAK_LAYER_IDS) {
			setLayerVisibilitySafe(layerId, peakLabelsVisible);
		}

		for (const layer of styleLayers()) {
			const id = layer.id.toLowerCase();
			const sourceLayer = layer['source-layer']?.toLowerCase() ?? '';
			if (/path_pedestrian/.test(id)) setLayerVisibilitySafe(layer.id, ordinaryPathsVisible);
			if (layer.type === 'symbol' && isBasemapPoiLayer(layer.id, sourceLayer)) {
				// The provider's general-purpose POI layers are replaced by the
				// ranked outdoor hierarchy above; leaving both on causes duplicates
				// and lets shops/cafes crowd out huts, rescue, and water.
				setLayerVisibilitySafe(layer.id, false);
			}
			if (
				layer.type === 'symbol' &&
				(/mountain_peak/.test(sourceLayer) || /(^|[-_])(mountain|peak)([-_]|$)/.test(id))
			) {
				// Peaks are rendered from our imported OSM waypoint source. Keep the
				// basemap's independent peak layer off so names/elevations cannot disagree.
				setLayerVisibilitySafe(
					layer.id,
					OUTDOOR_PEAK_LAYER_ID_SET.has(layer.id) && peakLabelsVisible
				);
			}
			if (
				layer.type !== 'symbol' &&
				(/boundary/.test(sourceLayer) || /(^|[-_])boundary|admin[-_]/.test(id))
			) {
				setLayerVisibilitySafe(layer.id, regionBoundariesVisible);
			}
		}
	}

	function setLayerVisibilitySafe(layerId: string, visible: boolean) {
		if (!map?.getLayer(layerId)) return;
		try {
			map.setLayoutProperty(layerId, 'visibility', visible ? 'visible' : 'none');
		} catch {
			// Third-party styles may make a layer immutable during a style transition.
		}
	}

	function addLayerIfMissing(layer: maplibregl.LayerSpecification, beforeLayerId?: string) {
		if (!map || map.getLayer(layer.id)) return;
		const anchor = beforeLayerId && map.getLayer(beforeLayerId) ? beforeLayerId : undefined;
		if (anchor) {
			map.addLayer(layer, anchor);
			return;
		}
		map.addLayer(layer);
	}

	function styleLayers() {
		return ((map?.getStyle().layers ?? []) as RuntimeStyleLayer[]) ?? [];
	}

	function resolveLabelAnchorId() {
		if (!map) return undefined;
		if (BASEMAP_LABEL_ANCHOR_ID && map.getLayer(BASEMAP_LABEL_ANCHOR_ID)) {
			return BASEMAP_LABEL_ANCHOR_ID;
		}
		return findFirstLabelAnchor(
			styleLayers().map((layer) => ({
				id: layer.id,
				type: layer.type,
				sourceLayer: layer['source-layer']
			}))
		);
	}

	function resolveTerrainAnchorId() {
		return resolveTerrainOverlayAnchor(
			styleLayers().map((layer) => ({
				id: layer.id,
				type: layer.type,
				sourceLayer: layer['source-layer']
			})),
			TRAIL_LOCAL_CASING_LAYER
		);
	}

	function tuneBasemapForTerrain() {
		if (!map) return;

		for (const layer of styleLayers()) {
			const sourceLayer = layer['source-layer'] ?? '';

			if (
				shouldHideBasemapLayer({
					id: layer.id,
					type: layer.type,
					sourceLayer
				})
			) {
				safeSetLayoutProperty(layer.id, 'visibility', 'none');
			}

			for (const override of terrainBasemapPaintOverrides({
				id: layer.id,
				type: layer.type,
				sourceLayer
			})) {
				safeSetPaintProperty(layer.id, override.property, override.value);
			}
		}
	}

	function safeSetPaintProperty(layerId: string, property: string, value: unknown) {
		if (!map || !map.getLayer(layerId)) return;
		try {
			map.setPaintProperty(layerId, property, value as never);
		} catch {
			// Ignore style-specific property mismatches across third-party basemaps.
		}
	}

	function safeSetLayoutProperty(layerId: string, property: string, value: unknown) {
		if (!map || !map.getLayer(layerId)) return;
		try {
			map.setLayoutProperty(layerId, property, value as never);
		} catch {
			// Ignore style-specific property mismatches across third-party basemaps.
		}
	}

	function absolutizeTileTemplate(template: string) {
		if (typeof window === 'undefined') return template;
		if (/^[a-z]+:\/\//i.test(template)) return template;
		if (template.startsWith('//')) {
			return `${window.location.protocol}${template}`;
		}
		if (template.startsWith('/')) {
			return `${window.location.origin}${template}`;
		}
		return `${window.location.origin}/${template.replace(/^\.?\//, '')}`;
	}

	function sanitizeSearchResults(payload: unknown): SearchPayload {
		const empty = { routes: [], places: [] } satisfies SearchPayload;
		if (!payload || typeof payload !== 'object') return empty;

		const value = payload as Record<string, unknown>;
		const routes = Array.isArray(value.routes) ? value.routes : [];
		const places = Array.isArray(value.places) ? value.places : [];

		return {
			routes: routes
				.map((item) => {
					const result = item as Record<string, unknown>;
					const kind = result.kind === 'trail' ? 'trail' : null;
					const id = typeof result.id === 'string' ? result.id : null;
					const name = typeof result.name === 'string' ? result.name : null;
					const subtitle = typeof result.subtitle === 'string' ? result.subtitle : null;
					const network = typeof result.network === 'string' ? result.network : null;
					const distance_m = typeof result.distance_m === 'number' ? result.distance_m : null;
					const breadcrumb = Array.isArray(result.breadcrumb)
						? result.breadcrumb.filter((entry): entry is string => typeof entry === 'string')
						: [];

					if (!kind || !id || !name) return null;

					return {
						kind,
						id,
						name,
						subtitle,
						network,
						distance_m,
						breadcrumb
					} satisfies SearchRouteResult;
				})
				.filter((item): item is SearchRouteResult => item !== null),
			places: places
				.map((item) => {
					const result = item as Record<string, unknown>;
					const kind =
						result.kind === 'region' ? 'region' : result.kind === 'place' ? 'place' : null;
					const id = typeof result.id === 'string' ? result.id : null;
					const name = typeof result.name === 'string' ? result.name : null;
					const place_kind = typeof result.place_kind === 'string' ? result.place_kind : null;
					const admin_label = typeof result.admin_label === 'string' ? result.admin_label : null;
					const breadcrumb = Array.isArray(result.breadcrumb)
						? result.breadcrumb.filter((entry): entry is string => typeof entry === 'string')
						: [];
					const bbox =
						Array.isArray(result.bbox) &&
						result.bbox.length === 4 &&
						result.bbox.every((entry) => typeof entry === 'number')
							? (result.bbox as [number, number, number, number])
							: null;
					const center =
						Array.isArray(result.center) &&
						result.center.length === 2 &&
						result.center.every((entry) => typeof entry === 'number')
							? (result.center as [number, number])
							: null;

					if (!kind || !id || !name) return null;

					return {
						kind,
						id,
						name,
						place_kind,
						admin_label,
						breadcrumb,
						bbox,
						center
					} satisfies SearchPlaceResult;
				})
				.filter((item): item is SearchPlaceResult => item !== null)
		};
	}

	function closeSearchPanel() {
		searchPanelOpen = false;
		activeSearchIndex = -1;
		searchLoading = false;
		searchRequestVersion += 1;
		if (searchDebounceHandle) {
			clearTimeout(searchDebounceHandle);
			searchDebounceHandle = null;
		}
		searchAbortController?.abort();
		searchAbortController = null;
	}

	function clearSearchResults() {
		searchRoutes = [];
		searchPlaces = [];
		searchResults = [];
		searchLoading = false;
		searchError = null;
		activeSearchIndex = -1;
	}

	function resetSearch() {
		searchQuery = '';
		if (searchDebounceHandle) {
			clearTimeout(searchDebounceHandle);
			searchDebounceHandle = null;
		}
		clearSearchResults();
		closeSearchPanel();
		searchInput?.focus();
	}

	async function runSearch(term: string, requestVersion: number) {
		const controller = new AbortController();
		searchAbortController?.abort();
		searchAbortController = controller;
		try {
			const params = new URLSearchParams({
				q: term.trim(),
				limit: '10'
			});
			const response = await requestJson<unknown>(`/api/search?${params.toString()}`, {
				signal: controller.signal,
				timeoutMs: 8_000
			});
			if (!response.ok) {
				throw new Error(`Search request failed with ${response.status}`);
			}

			if (requestVersion !== searchRequestVersion) return;

			const parsed = sanitizeSearchResults(response.data);
			searchRoutes = parsed.routes;
			searchPlaces = parsed.places;
			searchResults = [...parsed.routes, ...parsed.places];
			searchLoading = false;
			searchError = null;
			activeSearchIndex = searchResults.length > 0 ? 0 : -1;
		} catch (error) {
			if (controller.signal.aborted || isAbortError(error)) return;
			if (requestVersion !== searchRequestVersion) return;
			searchRoutes = [];
			searchPlaces = [];
			searchResults = [];
			searchLoading = false;
			searchError = error instanceof Error ? error.message : 'Search failed';
			activeSearchIndex = -1;
		} finally {
			if (searchAbortController === controller) searchAbortController = null;
		}
	}

	function queueSearch(term: string) {
		const trimmed = term.trim();
		searchPanelOpen = true;

		if (searchDebounceHandle) {
			clearTimeout(searchDebounceHandle);
			searchDebounceHandle = null;
		}
		searchAbortController?.abort();
		searchAbortController = null;

		if (trimmed.length < SEARCH_MIN_CHARS) {
			searchRequestVersion += 1;
			searchLoading = false;
			searchRoutes = [];
			searchPlaces = [];
			searchResults = [];
			searchError = null;
			activeSearchIndex = -1;
			return;
		}

		searchLoading = true;
		searchError = null;
		const requestVersion = ++searchRequestVersion;
		searchDebounceHandle = setTimeout(() => {
			searchDebounceHandle = null;
			void runSearch(trimmed, requestVersion);
		}, 150);
	}

	function handleSearchInput(event: Event) {
		searchQuery = (event.currentTarget as HTMLInputElement).value;
		queueSearch(searchQuery);
	}

	function openSearchPanel() {
		if (!mapVisualReady) return;
		searchPanelOpen = true;
		if (searchQuery.trim().length >= SEARCH_MIN_CHARS && !searchLoading && searchResults.length === 0) {
			queueSearch(searchQuery);
		}
	}

	function fitToSearchBounds(bbox: [number, number, number, number]) {
		if (!map) return;
		const [minLon, minLat, maxLon, maxLat] = bbox;
		const mobile = window.innerWidth < 740;
		map.fitBounds(
			[
				[minLon, minLat],
				[maxLon, maxLat]
			],
			{
				padding: {
					top: 96,
					right: mobile ? 24 : activeMapPanel ? 440 : 80,
					bottom: 80,
					left: mobile ? 24 : detailPanelOpen ? 460 : 80
				},
				duration: 850
			}
		);
	}

	function flyToSearchCenter(center: [number, number]) {
		if (!map) return;
		map.flyTo({
			center,
			zoom: Math.max(map.getZoom(), 11.5),
			duration: 850
		});
	}

	async function selectSearchResult(result: SearchResult) {
		if (!mapVisualReady || !map?.isStyleLoaded()) return;
		searchQuery = result.name;
		closeSearchPanel();

		if (result.kind === 'trail') {
			await loadTrail(result.id);
			return;
		}

		closePanel();
		if (result.bbox) {
			fitToSearchBounds(result.bbox);
			return;
		}

		if (result.center) {
			flyToSearchCenter(result.center);
		}
	}

	async function confirmActiveSearchResult() {
		const fallback = searchResults[0];
		const result =
			activeSearchIndex >= 0 && activeSearchIndex < searchResults.length
				? searchResults[activeSearchIndex]
				: fallback;
		if (!result) return;
		await selectSearchResult(result);
	}

	function handleSearchKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			closeSearchPanel();
			return;
		}

		if (event.key === 'ArrowDown' && searchResults.length > 0) {
			event.preventDefault();
			activeSearchIndex = (activeSearchIndex + 1 + searchResults.length) % searchResults.length;
			return;
		}

		if (event.key === 'ArrowUp' && searchResults.length > 0) {
			event.preventDefault();
			activeSearchIndex =
				(activeSearchIndex - 1 + searchResults.length) % searchResults.length;
			return;
		}

		if (event.key === 'Enter') {
			event.preventDefault();
			void confirmActiveSearchResult();
		}
	}

	function abortTrailRequests() {
		trailRequestVersion += 1;
		trailDetailAbortController?.abort();
		trailDetailAbortController = null;
		elevationAbortController?.abort();
		elevationAbortController = null;
		abortDownloadRequest();
	}

	function abortDownloadRequest() {
		downloadRequestVersion += 1;
		downloadAbortController?.abort();
		downloadAbortController = null;
		downloadBusy = false;
	}

	onDestroy(() => {
		componentDestroyed = true;
		if (typeof window !== 'undefined') window.removeEventListener('popstate', restoreLocation);
		if (searchDebounceHandle) {
			clearTimeout(searchDebounceHandle);
		}
		if (mapRevealHandle) {
			clearTimeout(mapRevealHandle);
		}
		searchAbortController?.abort();
		abortTrailRequests();
		weatherAbortController?.abort();
		mapFeatureElevationAbortController?.abort();
		waypointMarker?.remove();
		map?.remove();
	});

	async function loadTrail(trailId: string, updateHistory = true, fit = true) {
		waypointMarker?.remove();
		shareMessage = ''; shareFallback = '';
		searchPanelOpen = false;
		abortTrailRequests();
		const requestVersion = trailRequestVersion;
		const controller = new AbortController();
		trailDetailAbortController = controller;
		detailError = null;
		clearSelectedMapFeatureLayer();
		weatherAbortController?.abort();
		weatherAbortController = null;
		weatherForecast = null;
		weatherStatus = 'idle';
		weatherError = null;

		try {
			clearRouteChoices();
			const response = await requestJson<TrailDetail>(`/api/trails/${trailId}?v=2`, {
				cache: 'no-cache',
				signal: controller.signal,
				timeoutMs: 12_000
			});
			if (!response.ok) {
				throw new Error(`Trail request failed with ${response.status}`);
			}

			const payload = response.data;
			if (!payload || typeof payload !== 'object') {
				throw new Error('Trail service returned invalid data');
			}
			if (requestVersion !== trailRequestVersion) return;

			const trailDetail: TrailDetail = {
				...payload,
				stitch_warnings: sanitizeTrailWarnings(payload?.stitch_warnings)
			};
			selectedTrail = trailDetail;
			navigationTrailId = trailId;
			if (updateHistory) persistLocation(true);
			if (trailDetail.start_point?.coordinates) {
				void refreshWeather(trailDetail.start_point.coordinates);
			}
			elevationProfile = null;
			elevationStatus = 'loading';
			elevationError = null;
			elevationHoverPosM = null;
			elevationHoverEleM = null;
			directionMode = defaultDirectionMode(trailDetail);
			downloadFormat = 'gpx';
			downloadSectionOpen = false;
			downloadBusy = false;
			downloadError = null;
			void loadElevation(trailId, requestVersion);
			if (fit) fitToSelectedTrail();
		} catch (error) {
			if (controller.signal.aborted || isAbortError(error)) return;
			if (requestVersion !== trailRequestVersion) return;
			selectedTrail = null;
			clearSelectedTrailLayer();
			detailError = error instanceof Error ? error.message : 'Failed to load trail details';
		} finally {
			if (trailDetailAbortController === controller) trailDetailAbortController = null;
		}
	}

	async function loadElevation(trailId: string, requestVersion: number) {
		elevationAbortController?.abort();
		const controller = new AbortController();
		elevationAbortController = controller;
		elevationStatus = 'loading';
		elevationError = null;
		elevationHoverPosM = null;
		elevationHoverEleM = null;

		try {
			const response = await requestJson<Record<string, unknown>>(
				`/api/trails/${trailId}/elevation?revision=${encodeURIComponent(selectedTrail?.data_revision ?? '')}`,
				{ signal: controller.signal, timeoutMs: 12_000 }
			);
			if (controller.signal.aborted || requestVersion !== trailRequestVersion) return;
			if (response.status === 404) {
				if (selectedTrail?.id === trailId) {
					elevationProfile = null;
					elevationStatus = 'missing';
				}
				return;
			}
			if (!response.ok) {
				throw new Error(`Elevation request failed with ${response.status}`);
			}

			const payload = response.data;
			if (!payload) throw new Error('Elevation service returned invalid data');
			const profilePoints = sanitizeElevationPoints(payload?.profile_points);
			const profile: ElevationProfile = {
				min_elevation_m:
					typeof payload?.min_elevation_m === 'number' ? payload.min_elevation_m : null,
				max_elevation_m:
					typeof payload?.max_elevation_m === 'number' ? payload.max_elevation_m : null,
				profile_points: profilePoints
			};

			if (selectedTrail?.id === trailId) {
				elevationProfile = profilePoints.length > 0 ? profile : null;
				elevationStatus = profilePoints.length > 0 ? 'ready' : 'missing';
			}
		} catch (error) {
			if (controller.signal.aborted || isAbortError(error)) return;
			if (requestVersion !== trailRequestVersion) return;
			if (selectedTrail?.id === trailId) {
				elevationProfile = null;
				elevationStatus = 'error';
				elevationError =
					error instanceof Error ? error.message : 'Failed to load elevation profile';
			}
		} finally {
			if (elevationAbortController === controller) elevationAbortController = null;
		}
	}

	function syncSelectedTrailLayer() {
		if (!map) return;

		const featureCollection = buildSelectedTrailFeatureCollection();
		ensureDirectionArrowImage();
		const overlayAnchorId = resolveOutdoorPoiAnchorId();

		const source = map.getSource(SELECTED_TRAIL_SOURCE) as maplibregl.GeoJSONSource | undefined;
		if (source) {
			source.setData(featureCollection as never);
		} else {
			map.addSource(SELECTED_TRAIL_SOURCE, {
				type: 'geojson',
				data: featureCollection as never
			});
			addLayerIfMissing(
				{
					id: SELECTED_TRAIL_GLOW_LAYER,
					type: 'line',
					source: SELECTED_TRAIL_SOURCE,
					filter: ['==', ['get', 'role'], 'route'],
					layout: {
						'line-cap': 'round',
						'line-join': 'round'
					},
					paint: {
						'line-color': OUTDOOR_THEME.route.shadow,
						'line-width': [
							'interpolate',
							['linear'],
							['zoom'],
							9,
							7.5,
							12,
							8.5,
							14,
							10,
							16,
							12,
							18,
							14
						],
						'line-opacity': 0.92,
						'line-blur': 0.65
					}
				},
				overlayAnchorId
			);
			addLayerIfMissing(
				{
					id: SELECTED_TRAIL_CASING_LAYER,
					type: 'line',
					source: SELECTED_TRAIL_SOURCE,
					filter: ['==', ['get', 'role'], 'route'],
					layout: {
						'line-cap': 'round',
						'line-join': 'round'
					},
					paint: {
						'line-color': OUTDOOR_THEME.route.casing,
						'line-width': [
							'interpolate',
							['linear'],
							['zoom'],
							9,
							5.5,
							12,
							6.5,
							14,
							7.5,
							16,
							9,
							18,
							10.5
						],
						'line-opacity': 0.98
					}
				},
				overlayAnchorId
			);
			addLayerIfMissing(
				{
					id: SELECTED_TRAIL_LAYER,
					type: 'line',
					source: SELECTED_TRAIL_SOURCE,
					filter: ['==', ['get', 'role'], 'route'],
					layout: {
						'line-cap': 'round',
						'line-join': 'round'
					},
					paint: {
						'line-color': OUTDOOR_THEME.route.selected,
						'line-width': [
							'interpolate',
							['linear'],
							['zoom'],
							9,
							3,
							12,
							4,
							14,
							4.8,
							16,
							5.8,
							18,
							7
						],
						'line-opacity': 0.98
					}
				},
				overlayAnchorId
			);
			addLayerIfMissing(
				{
					id: SELECTED_TRAIL_ARROW_LAYER,
					type: 'symbol',
					source: SELECTED_TRAIL_SOURCE,
					minzoom: 11,
					filter: ['==', ['get', 'role'], 'direction-chevron'],
					layout: {
						'icon-image': SELECTED_TRAIL_ARROW_IMAGE,
						'icon-size': [
							'interpolate',
							['linear'],
							['zoom'],
							11,
							0.9,
							14,
							1.1,
							16,
							1.25,
							18,
							1.35
						],
						'icon-rotation-alignment': 'map',
						'icon-keep-upright': false,
						'icon-allow-overlap': true,
						'icon-ignore-placement': true,
						'symbol-placement': 'line',
						'symbol-spacing': [
							'interpolate',
							['linear'],
							['zoom'],
							11,
							160,
							14,
							110,
							16,
							72,
							18,
							56
						]
					},
					paint: {
						'icon-opacity': 1
					}
				},
				overlayAnchorId
			);
			addLayerIfMissing(
				{
					id: SELECTED_TRAIL_MARKER_LAYER,
					type: 'circle',
					source: SELECTED_TRAIL_SOURCE,
					filter: ['==', ['get', 'role'], 'marker'],
					paint: {
					'circle-radius': [
						'match',
						['get', 'kind'],
						'start',
							8,
							'end',
							8,
							'open_end',
							7,
							'branch',
							6,
							'disconnected',
							5,
							6
						],
					'circle-color': [
						'match',
						['get', 'kind'],
						'start',
							OUTDOOR_THEME.route.start,
							'end',
							OUTDOOR_THEME.route.finish,
							'open_end',
							OUTDOOR_THEME.route.offRoute,
							'branch',
							'#67e8f9',
							'disconnected',
							'#9ca3af',
							'#f8fafc'
						],
					'circle-stroke-color': '#ffffff',
					'circle-stroke-width': 2
					}
				},
				overlayAnchorId
			);
			addLayerIfMissing(
				{
					id: SELECTED_TRAIL_MARKER_LABEL_LAYER,
					type: 'symbol',
					source: SELECTED_TRAIL_SOURCE,
					filter: [
						'all',
						['==', ['get', 'role'], 'marker'],
						['in', ['get', 'kind'], ['literal', ['start', 'end']]]
					],
					layout: {
						'text-field': ['get', 'label'],
						'text-font': ['Noto Sans Bold'],
						'text-size': [
							'interpolate',
							['linear'],
							['zoom'],
							9,
							10.5,
							12,
							11.5,
							14,
							12
						],
						'text-anchor': 'top',
						'text-offset': [0, 1.35],
						'text-pitch-alignment': 'map',
						'text-rotation-alignment': 'viewport',
						'text-allow-overlap': true,
						'text-ignore-placement': true
					},
					paint: {
						'text-color': [
							'match',
							['get', 'kind'],
							'start',
							OUTDOOR_THEME.route.start,
							OUTDOOR_THEME.route.finish
						],
						'text-halo-color': OUTDOOR_THEME.route.casing,
						'text-halo-width': 2
					}
				},
				overlayAnchorId
			);
		}
	}

	function syncProfileCursorLayer() {
		if (!map || !map.isStyleLoaded()) return;
		const source = map.getSource(SELECTED_TRAIL_CURSOR_SOURCE) as
			| maplibregl.GeoJSONSource
			| undefined;
		if (!selectedTrail && !source) return;
		const features: GeoJsonFeature[] = [];
		if (
			selectedTrail?.geometry &&
			elevationHoverPosM != null &&
			elevationHoverEleM != null
		) {
			const profileLength = elevationProfile?.profile_points.at(-1)?.pos_m ?? 0;
			const hoverPoint = routeIndex?.at(profileLength > 0 ? elevationHoverPosM / profileLength : 0);
			if (hoverPoint) {
				features.push({
					type: 'Feature',
					properties: { kind: PROFILE_CURSOR_KIND },
					geometry: { type: 'Point', coordinates: hoverPoint }
				});
			}
		}

		const data: GeoJsonFeatureCollection = { type: 'FeatureCollection', features };
		if (source) {
			source.setData(data as never);
			return;
		}

		map.addSource(SELECTED_TRAIL_CURSOR_SOURCE, { type: 'geojson', data: data as never });
		addLayerIfMissing(
			{
				id: SELECTED_TRAIL_CURSOR_LAYER,
				type: 'circle',
				source: SELECTED_TRAIL_CURSOR_SOURCE,
				paint: {
					'circle-radius': 9,
					'circle-color': OUTDOOR_THEME.route.profileCursor,
					'circle-stroke-color': 'rgba(255, 255, 255, 0.98)',
					'circle-stroke-width': 3
				}
			},
			resolveOutdoorPoiAnchorId()
		);
	}

	function ensureDirectionArrowImage() {
		if (!map || map.hasImage(SELECTED_TRAIL_ARROW_IMAGE)) return;

		const size = 28;
		const canvas = document.createElement('canvas');
		canvas.width = size;
		canvas.height = size;

		const context = canvas.getContext('2d');
		if (!context) return;

		context.clearRect(0, 0, size, size);
		context.translate(size / 2, size / 2);

		context.beginPath();
		context.moveTo(-5.5, -6);
		context.lineTo(4.8, 0);
		context.lineTo(-5.5, 6);
		context.lineJoin = 'round';
		context.lineCap = 'round';
		context.strokeStyle = OUTDOOR_THEME.route.directionOutline;
		context.lineWidth = 5.5;
		context.stroke();

		context.beginPath();
		context.moveTo(-5.5, -6);
		context.lineTo(4.8, 0);
		context.lineTo(-5.5, 6);
		context.strokeStyle = OUTDOOR_THEME.route.direction;
		context.lineWidth = 2.8;
		context.stroke();

		map.addImage(SELECTED_TRAIL_ARROW_IMAGE, context.getImageData(0, 0, size, size), {
			pixelRatio: 2
		});
	}

	function buildSelectedTrailFeatureCollection(): GeoJsonFeatureCollection {
		const features: GeoJsonFeature[] = [];

		if (selectedTrail?.geometry) {
			features.push({
				type: 'Feature',
				properties: {
					role: 'route'
				},
				geometry: selectedTrail.geometry
			});

			if (downloadSectionOpen) {
				for (const directionFeature of directionFeatures(selectedTrail.geometry, directionMode)) {
					features.push({
						type: 'Feature',
						properties: {
							role: directionFeature.role
						},
						geometry: directionFeature.geometry
					});
				}
			}
		}

		if (selectedTrail?.start_point) {
			features.push({
				type: 'Feature',
				properties: {
					role: 'marker',
					kind: 'start',
					label: 'Start'
				},
				geometry: selectedTrail.start_point
			});
		}

		if (selectedTrail?.end_point) {
			features.push({
				type: 'Feature',
				properties: {
					role: 'marker',
					kind: 'end',
					label: 'Finish'
				},
				geometry: selectedTrail.end_point
			});
		}

		for (const diagnostic of qaMode ? selectedTrail?.stitch_diagnostics ?? [] : []) {
			if (
				diagnostic.kind === 'open_end' &&
				(pointMatches(diagnostic.geometry, selectedTrail?.start_point) ||
					pointMatches(diagnostic.geometry, selectedTrail?.end_point))
			) {
				continue;
			}

			features.push({
				type: 'Feature',
				properties: {
					role: 'marker',
					kind: diagnostic.kind,
					label: diagnosticLabel(diagnostic.kind)
				},
				geometry: diagnostic.geometry
			});
		}

		return {
			type: 'FeatureCollection',
			features
		};
	}

	function clearSelectedTrailLayer() {
		const source = map?.getSource(SELECTED_TRAIL_SOURCE) as maplibregl.GeoJSONSource | undefined;
		source?.setData(emptyFeatureCollection() as never);
		const cursorSource = map?.getSource(SELECTED_TRAIL_CURSOR_SOURCE) as
			| maplibregl.GeoJSONSource
			| undefined;
		cursorSource?.setData(emptyFeatureCollection() as never);
	}

	function emptyFeatureCollection(): GeoJsonFeatureCollection {
		return {
			type: 'FeatureCollection' as const,
			features: []
		};
	}

	function fitToSelectedTrail() {
		if (!map || !selectedTrail?.geometry) return;

		const bounds = boundsForGeometry(selectedTrail.geometry);
		if (!bounds) return;

		const leftPadding = window.innerWidth < 740 ? 24 : 500;
		const rightPadding = window.innerWidth < 740 ? 24 : activeMapPanel ? 440 : 72;

		map.fitBounds(bounds, {
			padding: { top: 72, right: rightPadding, bottom: window.innerWidth < 740 ? window.innerHeight * 0.6 : 72, left: leftPadding },
			maxZoom: 14,
			duration: 700
		});
	}

	function boundsForGeometry(geometry: TrailGeometry) {
		const coordinates = collectCoordinates(geometry);
		if (coordinates.length === 0) return null;

		const bounds = new maplibregl.LngLatBounds(coordinates[0], coordinates[0]);
		for (const coordinate of coordinates.slice(1)) {
			bounds.extend(coordinate);
		}
		return bounds;
	}

	function collectCoordinates(geometry: TrailGeometry): [number, number][] {
		switch (geometry.type) {
			case 'LineString':
				return (geometry.coordinates as [number, number][]) ?? [];
			case 'MultiLineString':
				return ((geometry.coordinates as [number, number][][]) ?? []).flat();
			case 'GeometryCollection':
				return ((geometry.geometries as TrailGeometry[] | undefined) ?? []).flatMap(collectCoordinates);
			default:
				return [];
		}
	}

	function coordinateDistanceM(a: [number, number], b: [number, number]) {
		const from = new maplibregl.LngLat(a[0], a[1]);
		const to = new maplibregl.LngLat(b[0], b[1]);
		return from.distanceTo(to);
	}

	function closePanel() {
		waypointMarker?.remove();
		navigationTrailId = null;
		persistLocation(true);
		shareMessage = ''; shareFallback = '';
		abortTrailRequests();
		selectedTrail = null;
		clearSelectedMapFeatureLayer();
		detailError = null;
		directionMode = 'forward';
		downloadFormat = 'gpx';
		downloadSectionOpen = false;
		downloadBusy = false;
		downloadError = null;
		elevationProfile = null;
		elevationStatus = 'idle';
		elevationError = null;
		elevationHoverPosM = null;
		elevationHoverEleM = null;
		weatherAbortController?.abort();
		weatherAbortController = null;
		weatherForecast = null;
		weatherStatus = 'idle';
		weatherError = null;
		clearRouteChoices();
		clearSelectedTrailLayer();
	}

	function clearRouteChoices() {
		routeChoices = [];
		routeChooserPosition = null;
		hoveredRouteChoiceId = null;
	}

	function previewRouteChoice(trailId: string | null) {
		if (hoveredRouteChoiceId === trailId) return;
		hoveredRouteChoiceId = trailId;
		if (!trailId) return;
		fitHoveredRouteChoice(trailId);
	}

	function syncHoveredRouteLayer() {
		if (!map) return;

		const filter = hoveredRouteChoiceId
			? (['==', ['get', 'id'], hoveredRouteChoiceId] as maplibregl.FilterSpecification)
			: (['==', ['get', 'id'], ''] as maplibregl.FilterSpecification);

		if (map.getLayer(HOVERED_ROUTE_CASING_LAYER)) {
			map.setFilter(HOVERED_ROUTE_CASING_LAYER, filter);
		}

		if (map.getLayer(HOVERED_ROUTE_LAYER)) {
			map.setFilter(HOVERED_ROUTE_LAYER, filter);
		}
	}

	function fitHoveredRouteChoice(trailId: string) {
		if (!map) return;
		const bounds = boundsForRouteChoice(trailId);
		if (!bounds) return;

		map.fitBounds(bounds, {
			padding: previewPaddingForChooser(),
			maxZoom: 14,
			duration: 280
		});
	}

	function boundsForRouteChoice(trailId: string) {
		if (!map) return null;

		const features = map.querySourceFeatures('trails', {
			sourceLayer: 'trail_lines',
			filter: ['==', ['get', 'id'], trailId]
		});

		let bounds: maplibregl.LngLatBounds | null = null;
		for (const feature of features) {
			for (const coordinate of collectUnknownGeometryCoordinates(feature.geometry)) {
				if (!bounds) {
					bounds = new maplibregl.LngLatBounds(coordinate, coordinate);
				} else {
					bounds.extend(coordinate);
				}
			}
		}

		return bounds;
	}

	function collectUnknownGeometryCoordinates(geometry: unknown): [number, number][] {
		if (!geometry || typeof geometry !== 'object') return [];
		const candidate = geometry as TrailGeometry;
		return collectCoordinates(candidate);
	}

	function previewPaddingForChooser() {
		const viewportWidth = mapContainer?.clientWidth ?? window.innerWidth;
		const chooserWidth = routeChooserPosition ? 360 : 72;
		const toolWidth = activeMapPanel ? 420 : 72;
		const panelWidth = detailPanelOpen ? Math.min(460, viewportWidth - 24) : 72;
		const chooserOnLeft =
			routeChooserPosition != null &&
			routeChooserPosition.x + Math.min(320, viewportWidth - 32) / 2 < viewportWidth / 2;
		const left = chooserOnLeft ? Math.max(panelWidth, chooserWidth) : panelWidth;
		const right = Math.max(toolWidth, chooserOnLeft ? 72 : chooserWidth);

		return {
			top: 72,
			right,
			bottom: 72,
			left
		};
	}

	function routeChoicesAtPoint(point: maplibregl.Point) {
		if (!map) return [];
		const layers = [...TRAIL_NETWORK_LAYER_IDS, ...TRAIL_LABEL_LAYER_IDS].filter((layerId) =>
			Boolean(map?.getLayer(layerId))
		);
		if (layers.length === 0) return [];

		const rendered = map.queryRenderedFeatures(
			[
				[point.x - 6, point.y - 6],
				[point.x + 6, point.y + 6]
			],
			{ layers }
		);

		const seen = new Set<string>();
		const choices: RouteChoice[] = [];

		for (const feature of rendered) {
			const properties = feature.properties ?? {};
			// Martin's feature id can be the numeric relation primary key; the
			// explicit `id` property is the UUID accepted by the trail API.
			const id = properties.id ?? feature.id;
			if (id == null) continue;

			const trailId = String(id);
			if (seen.has(trailId)) continue;
			seen.add(trailId);

			choices.push({
				id: trailId,
				name: String(properties.name ?? 'Unnamed trail'),
				network: properties.network ? String(properties.network) : null,
				tag: properties.tag ? String(properties.tag) : null
			});
		}

		return choices.sort((a, b) => a.name.localeCompare(b.name));
	}

	function routeChooserPositionFor(point: maplibregl.Point, choiceCount: number) {
		const padding = 18;
		const viewportWidth = mapContainer?.clientWidth ?? window.innerWidth;
		const viewportHeight = mapContainer?.clientHeight ?? window.innerHeight;
		const panelWidth = Math.max(0, Math.min(320, viewportWidth - padding * 2));
		const panelHeight = Math.max(0, Math.min(440, 134 + choiceCount * 88, viewportHeight - padding * 2));
		const leftPanelInset =
			detailPanelOpen ? Math.min(460, viewportWidth - 32) + 28 : padding;
		const maxX = Math.max(padding, viewportWidth - panelWidth - padding);
		const minX = Math.min(leftPanelInset, maxX);
		const minY = padding;
		const maxY = Math.max(padding, viewportHeight - panelHeight - padding);
		const prefersLeftColumn = !detailPanelOpen && point.x > viewportWidth / 2;
		const preferredX = prefersLeftColumn ? minX : maxX;
		const preferredY = Math.min(maxY, Math.max(minY, point.y - panelHeight / 2));

		return {
			x: preferredX,
			y: preferredY
		};
	}

	function formatDistance(distanceM: number | null) {
		return distanceM == null ? '—' : `${(distanceM / 1000).toFixed(1)} km`;
	}

	function formatElevation(elevationM: number | null) {
		return elevationM == null ? '—' : `${Math.round(elevationM)} m`;
	}

	function formatDuration(minutes: number | null) {
		if (!minutes || minutes <= 0) return '—';
		const hours = Math.floor(minutes / 60);
		const mins = minutes % 60;
		if (hours === 0) return `${mins} min`;
		if (mins === 0) return `${hours} hr`;
		return `${hours} hr ${mins} min`;
	}

	function originalMapperProfileUrl(trail: TrailDetail) {
		const explicit = normalizeExternalUrl(trail.original_mapper_profile_url);
		if (explicit) return explicit;
		if (trail.original_mapper_provider !== 'osm' || !trail.original_mapper_name) return null;
		return `https://www.openstreetmap.org/user/${encodeURIComponent(trail.original_mapper_name)}`;
	}

	function sanitizeTrailWarnings(raw: unknown): TrailWarning[] {
		if (!Array.isArray(raw)) return [];
		return raw
			.map((item) => {
				if (!item || typeof item !== 'object') return null;
				const kind = (item as Record<string, unknown>).kind;
				const message = (item as Record<string, unknown>).message;
				if (typeof kind !== 'string' || typeof message !== 'string') return null;
				return { kind, message };
			})
			.filter((item): item is TrailWarning => item !== null);
	}

	function difficultyLabel(value: string | null) {
		if (!value) return 'Difficulty unknown';

		switch (value) {
			case 'hiking':
				return 'Hiking · T1';
			case 'mountain_hiking':
				return 'Mountain · T2';
			case 'demanding_mountain_hiking':
				return 'Mountain · T3';
			case 'alpine_hiking':
				return 'Alpine · T4';
			case 'demanding_alpine_hiking':
				return 'Alpine · T5';
			case 'difficult_alpine_hiking':
				return 'Alpine · T6';
			default:
				return value.replaceAll('_', ' ');
		}
	}

	function hikeTypeLabel(value: string | null) {
		switch (value) {
			case 'loop':
				return 'Loop';
			case 'point_to_point':
				return 'Point to point';
			case 'complex':
				return 'Complex';
			default:
				return '—';
		}
	}

	function surfaceLabel(value: string) {
		return value
			.split('_')
			.map((part) => part[0]?.toUpperCase() + part.slice(1))
			.join(' ');
	}

	function featureLabel(value: string) {
		switch (value) {
			case 'camping':
				return 'camping';
			case 'huts':
				return 'huts';
			case 'summit':
				return 'summit';
			case 'spring':
				return 'springs';
			case 'lakes':
				return 'lakes';
			default:
				return value.replaceAll('_', ' ');
		}
	}

	function sanitizeElevationPoints(raw: unknown): ElevationPoint[] {
		if (!Array.isArray(raw)) return [];
		return raw
			.map((item) => {
				if (!item || typeof item !== 'object') return null;
				const pos = (item as Record<string, unknown>).pos_m;
				const ele = (item as Record<string, unknown>).ele_m;
				if (typeof pos !== 'number' || typeof ele !== 'number') return null;
				return { pos_m: pos, ele_m: ele };
			})
			.filter((item): item is ElevationPoint => item !== null)
			.sort((a, b) => a.pos_m - b.pos_m);
	}

	function directionSourceLabel(value: string | null | undefined) {
		switch (value) {
			case 'member_role':
				return 'Explicit route direction';
			case 'member_order':
				return 'Relation member order';
			case 'topology_fallback':
				return 'Topology fallback';
			default:
				return 'Unspecified';
		}
	}

	function normalizeExternalUrl(value: string | null | undefined) {
		if (!value) return null;
		const trimmed = value.trim();
		if (!trimmed) return null;
		if (/^https?:\/\//i.test(trimmed)) return trimmed;
		return `https://${trimmed}`;
	}

	function surfaceColor(surfaceType: string) {
		switch (surfaceType) {
			case 'ground':
				return '#a77245';
			case 'gravel':
				return '#c6c2bb';
			case 'rock':
				return '#8b8f99';
			case 'paved':
				return '#4ba3d9';
			case 'snow_ice':
				return '#dceeff';
			default:
				return '#8d8d98';
		}
	}

	function diagnosticLabel(kind: string) {
		switch (kind) {
			case 'open_end':
				return 'Open end';
			case 'branch':
				return 'Branch';
			case 'disconnected':
				return 'Disconnected';
			default:
				return kind.replaceAll('_', ' ');
		}
	}

	function pointMatches(a: PointGeometry | null | undefined, b: PointGeometry | null | undefined) {
		if (!a || !b) return false;
		return a.coordinates[0] === b.coordinates[0] && a.coordinates[1] === b.coordinates[1];
	}

	function diagnosticCount(kind: string) {
		return selectedTrail?.stitch_diagnostics.filter((item) => item.kind === kind).length ?? 0;
	}

	function warningSummaries(warnings: TrailWarning[]): WarningSummary[] {
		const summaries = new Map<string, WarningSummary>();

		for (const warning of warnings) {
			if (warning.kind === 'branch_ambiguity') {
				incrementSummary(
					summaries,
					'branch_ambiguity',
					'Direction ambiguity',
					'This route passes through junctions, so the stitcher followed the first available continuation at some branch points.'
				);
				continue;
			}

			if (warning.kind === 'missing_members') {
				incrementSummary(
					summaries,
					'missing_members',
					'Missing relation members',
					'Some OSM relation members were missing from the staged raw ways, so the route geometry or distance may be incomplete.'
				);
				continue;
			}

			if (warning.kind === 'disconnected_pieces') {
				incrementSummary(
					summaries,
					'disconnected_pieces',
					'Disconnected pieces',
					'The OSM relation breaks into multiple pieces, and only the main connected path was kept as the stitched route.'
				);
				continue;
			}

			if (warning.kind === 'stranded_segments') {
				incrementSummary(
					summaries,
					'stranded_segments',
					'Unreached segments',
					'Some route members looked like spurs or out-and-back branches and could not be covered by one simple walk.'
				);
				continue;
			}

			incrementSummary(summaries, warning.message, 'Data warning', warning.message);
		}

		return Array.from(summaries.values());
	}

	function incrementSummary(
		summaries: Map<string, WarningSummary>,
		key: string,
		title: string,
		description: string
	) {
		const existing = summaries.get(key);
		if (existing) {
			existing.count += 1;
			return;
		}

		summaries.set(key, {
			key,
			title,
			description,
			count: 1
		});
	}

	function defaultDirectionMode(trail: TrailDetail): DirectionMode {
		return trail.default_direction_mode ?? (trail.hike_type === 'loop' ? 'both' : 'forward');
	}

	function directionFeatures(
		geometry: TrailGeometry,
		mode: DirectionMode
	): Array<{ role: 'direction-chevron'; geometry: TrailGeometry }> {
		switch (mode) {
			case 'forward':
				return [{ role: 'direction-chevron', geometry }];
			case 'backward':
				return [{ role: 'direction-chevron', geometry: reverseGeometry(geometry) }];
			case 'both':
				// A two-way trail has no prescribed travel direction. Repeated dots read
				// like navigation instructions, so the route remains deliberately clean.
				return [];
		}
	}

	function reverseGeometry(geometry: TrailGeometry): TrailGeometry {
		switch (geometry.type) {
			case 'LineString':
				return {
					...geometry,
					coordinates: [...((geometry.coordinates as [number, number][]) ?? [])].reverse()
				};
			case 'MultiLineString':
				return {
					...geometry,
					coordinates: [...((geometry.coordinates as [number, number][][]) ?? [])]
						.reverse()
						.map((line) => [...line].reverse())
				};
			case 'GeometryCollection':
				return {
					...geometry,
					geometries: ((geometry.geometries as TrailGeometry[] | undefined) ?? []).map(reverseGeometry)
				};
			default:
				return geometry;
		}
	}

	function toggleTerrainPanel() {
		toggleMapPanel('terrain');
	}

	function toggleTerrain3d() {
		terrain3dEnabled = !terrain3dEnabled;
		if (!map) return;
		map.setMaxPitch(85);
		if (terrain3dEnabled) {
			map.easeTo({
				pitch: Math.max(map.getPitch(), 64),
				bearing: Math.abs(map.getBearing()) < 2 ? -24 : map.getBearing(),
				duration: 750
			});
		} else {
			map.easeTo({ pitch: 0, bearing: 0, duration: 650 });
		}
	}

	function setMapPitchValue(value: number) {
		if (!map) return;
		const pitch = Math.max(0, Math.min(82, value));
		if (pitch > 0 && !terrain3dEnabled) terrain3dEnabled = true;
		map.easeTo({ pitch, duration: 180 });
	}

	function setMapBearingValue(value: number) {
		map?.easeTo({ bearing: value, duration: 180 });
	}

	function rotateMap(delta: number) {
		map?.easeTo({ bearing: (map.getBearing() + delta) % 360, duration: 260 });
	}

	function set3DViewPreset(preset: 'terrain' | 'horizon') {
		if (!map) return;
		terrain3dEnabled = true;
		map.setMaxPitch(85);
		map.easeTo({
			pitch: preset === 'horizon' ? 78 : 62,
			bearing: Math.abs(map.getBearing()) < 2 ? -28 : map.getBearing(),
			duration: 750
		});
	}

	function setTerrainContoursEnabled(enabled: boolean) {
		terrainContoursVisible = enabled;
	}

		function setTerrainHillshadeEnabled(enabled: boolean) {
			terrainHillshadeVisible = enabled;
		}

		function setTerrainColorReliefEnabled(enabled: boolean) {
			terrainColorReliefVisible = enabled;
		}

		function setTerrainSlopeEnabled(enabled: boolean) {
			terrainSlopeVisible = enabled;
		}

		function setTerrainSlopeOpacity(value: number) {
			terrainSlopeOpacity = Math.min(1, Math.max(0.25, value));
		}

		function setTerrainContourUnit(unit: 'meters' | 'feet') {
			terrainContourUnit = unit;
		}

	function toggleDownloadSection() {
		downloadSectionOpen = !downloadSectionOpen;
		downloadError = null;
	}

	function setDirection(mode: DirectionMode) {
		directionMode = mode;
		downloadError = null;
	}

	function directionLabel(mode: DirectionMode, trail: TrailDetail) {
		const from = trail.route_from?.trim();
		const to = trail.route_to?.trim();
		const via = trail.route_via?.trim();

		if (mode === 'both') return 'both ways';
		if (!from && !to) return mode;

		if (mode === 'forward') {
			if (from && via && to) return `${from} → ${via} → ${to}`;
			if (from && to) return `${from} → ${to}`;
			return from ? `from ${from}` : `to ${to}`;
		}

		if (from && via && to) return `${to} → ${via} → ${from}`;
		if (from && to) return `${to} → ${from}`;
		return to ? `from ${to}` : `to ${from}`;
	}

	function downloadFilenameFromHeader(headerValue: string | null) {
		if (!headerValue) return null;
		const match = headerValue.match(/filename="([^"]+)"/i);
		return match?.[1] ?? null;
	}

	async function downloadRoute(formatOverride?: 'gpx' | 'kml' | 'geojson') {
		const trail = selectedTrail;
		if (!trail) return;
		const format = formatOverride ?? downloadFormat;
		const direction = directionMode;
		abortDownloadRequest();
		const requestVersion = downloadRequestVersion;
		const controller = new AbortController();
		downloadAbortController = controller;

		downloadBusy = true;
		downloadError = null;

		try {
			const params = new URLSearchParams({
				format,
				direction,
				revision: trail.data_revision ?? ''
			});
			const response = await requestBlob(`/api/trails/${trail.id}/download?${params.toString()}`, {
				signal: controller.signal,
				timeoutMs: 30_000
			});
			if (controller.signal.aborted || requestVersion !== downloadRequestVersion) return;
			if (!response.ok) {
				throw new Error(response.status === 409 ? 'This route has changed. Reopen it before downloading.' : `Download request failed with ${response.status}`);
			}

			if (!response.data || response.data.size === 0) {
				throw new Error('Download service returned an empty file');
			}
			const blob = response.data;
			const objectUrl = URL.createObjectURL(blob);
			const link = document.createElement('a');
			link.href = objectUrl;
			link.download =
				downloadFilenameFromHeader(response.headers.get('content-disposition')) ??
				`${trail.id}.${format}`;
			document.body.appendChild(link);
			link.click();
			link.remove();
			setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
		} catch (error) {
			if (controller.signal.aborted || requestVersion !== downloadRequestVersion || isAbortError(error)) {
				return;
			}
			downloadError = error instanceof Error ? error.message : 'Failed to download route';
		} finally {
			if (downloadAbortController === controller && requestVersion === downloadRequestVersion) {
				downloadAbortController = null;
				downloadBusy = false;
			}
		}
	}
</script>

<div
	class:terrain-open={terrainPanelOpen}
	class:tools-open={activeMapPanel !== null}
	class:trail-open={detailPanelOpen}
	class="map-shell"
>
	<div
		bind:this={mapContainer}
		class:ready={mapVisualReady}
		class="map"
		aria-busy={!mapVisualReady}
	></div>
	<div
		class:ready={mapVisualReady}
		class="map-loading"
		role="status"
		aria-hidden={mapVisualReady}
		aria-live="polite"
	>
		<div class="map-loading-card">
			{#if mapLoadError}
				<span class="map-loading-mark error" aria-hidden="true">!</span>
				<strong>Outdoor map did not finish loading</strong>
				<span>{mapLoadError}</span>
				<button type="button" onclick={() => window.location.reload()}>Try again</button>
			{:else}
				<span class="map-loading-mark" aria-hidden="true">△</span>
				<strong>Preparing outdoor map</strong>
				<span>Terrain, trails and elevation</span>
			{/if}
		</div>
	</div>

		<div class:panel-open={detailPanelOpen} class="search-dock">
		<div class:open={searchPanelOpen} class="search-card">
			<div class="search-input-wrap">
				<span class="search-icon" aria-hidden="true">⌕</span>
				<input
					bind:this={searchInput}
					class="search-input"
					aria-label="Search trails or places"
					type="search"
					placeholder={mapVisualReady ? 'Search trails or places…' : 'Preparing map search…'}
					disabled={!mapVisualReady}
					value={searchQuery}
					oninput={handleSearchInput}
					onfocus={openSearchPanel}
					onkeydown={handleSearchKeydown}
				/>
				{#if searchQuery}
					<button class="search-clear" type="button" aria-label="Clear search" onclick={resetSearch}>
						×
					</button>
				{/if}
			</div>

			{#if searchPanelOpen}
				<div class="search-results">
					{#if searchQuery.trim().length < SEARCH_MIN_CHARS}
						<p class="search-hint">Type at least {SEARCH_MIN_CHARS} characters to search trails and areas.</p>
					{:else if searchLoading}
						<p class="search-hint">Searching the trail index…</p>
					{:else if searchError}
						<p class="search-error">{searchError}</p>
					{:else if searchResults.length === 0}
						<p class="search-hint">No matches yet. Try a trail name or a place like Manali.</p>
					{:else}
						<div class="search-result-list">
							{#if searchRoutes.length > 0}
								<div class="search-group-label">Trails</div>
								{#each searchRoutes as result}
									<button
										class:active={searchResults[activeSearchIndex]?.id === result.id}
										class="search-result"
										type="button"
										onmouseenter={() => (activeSearchIndex = searchResults.findIndex((item) => item.id === result.id))}
										onfocus={() => (activeSearchIndex = searchResults.findIndex((item) => item.id === result.id))}
										onclick={() => selectSearchResult(result)}
									>
										<div class="search-result-topline">
											<span class="search-kind-pill">Trail</span>
											{#if result.network}
												<span class="search-network">{result.network}</span>
											{/if}
											{#if result.distance_m != null}
												<span class="search-distance">{formatDistance(result.distance_m)}</span>
											{/if}
										</div>
										<strong>{result.name}</strong>
										{#if result.subtitle}
											<p>{result.subtitle}</p>
										{/if}
									</button>
								{/each}
							{/if}

							{#if searchPlaces.length > 0}
								<div class="search-group-label">Places</div>
								{#each searchPlaces as result}
									<button
										class:active={searchResults[activeSearchIndex]?.id === result.id}
										class="search-result"
										type="button"
										onmouseenter={() => (activeSearchIndex = searchResults.findIndex((item) => item.id === result.id))}
										onfocus={() => (activeSearchIndex = searchResults.findIndex((item) => item.id === result.id))}
										onclick={() => selectSearchResult(result)}
									>
										<div class="search-result-topline">
											<span class:region-pill={result.kind === 'region'} class="search-kind-pill">
												{result.kind === 'region' ? 'Area' : 'Place'}
											</span>
											{#if result.admin_label}
												<span class="search-network">{result.admin_label}</span>
											{:else if result.place_kind}
												<span class="search-network">{result.place_kind}</span>
											{/if}
										</div>
										<strong>{result.name}</strong>
										{#if result.breadcrumb.length > 0}
											<p>{result.breadcrumb.join(', ')}</p>
										{/if}
									</button>
								{/each}
							{/if}
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>

	<TrailDiscovery bounds={discoveryBounds} hidden={detailPanelOpen || searchPanelOpen || !mapVisualReady || activeMapPanel !== null} onselect={(id) => void loadTrail(id)} onhover={(id) => hoveredRouteChoiceId = id} />

	<div class="terrain-dock">
		<div class="map-tool-rail" role="toolbar" aria-label="Outdoor map tools">
			<button class:active={activeMapPanel === 'layers'} type="button" onclick={() => toggleMapPanel('layers')} aria-label="Map layers" aria-pressed={activeMapPanel === 'layers'} title="Map layers">
				<svg aria-hidden="true" viewBox="0 0 24 24"><path d="m4 8 8-4 8 4-8 4-8-4Zm0 4 8 4 8-4M4 16l8 4 8-4" /></svg>
			</button>
			<button class:active={activeMapPanel === 'terrain'} type="button" onclick={toggleTerrainPanel} aria-label="Terrain settings" aria-pressed={activeMapPanel === 'terrain'} title="Terrain settings">
				<svg aria-hidden="true" viewBox="0 0 24 24"><path d="M3.5 17.5 8.5 9l3.2 4.2 2.4-3.1 6.4 7.4h-17Z" /><path d="m6.2 17.5 4-5.2" /></svg>
			</button>
			<button class:active={terrain3dEnabled} type="button" onclick={toggleTerrain3d} aria-label={terrain3dEnabled ? 'Turn off 3D terrain' : 'Turn on 3D terrain'} aria-pressed={terrain3dEnabled} disabled={!terrainReady} title={terrain3dEnabled ? 'Turn off 3D terrain' : 'Turn on 3D terrain'}>
				<span class="tool-3d" aria-hidden="true">3D</span>
			</button>
		</div>

		{#if activeMapPanel}
			<aside class="terrain-card map-tool-panel" aria-label={`${activeMapPanel} tools`}>
				<div class="terrain-card-header">
					<div>
						<h3>{activeMapPanel === 'layers' ? 'Map layers' : 'Terrain'}</h3>
						<p>{activeMapPanel === 'layers' ? 'Choose the outdoor information shown on the map.' : 'Adjust relief and contours. Use the 3D button on the map for perspective.'}</p>
					</div>
					<button class="tool-panel-close" type="button" onclick={() => toggleMapPanel(activeMapPanel as Exclude<MapPanel, null>)} aria-label="Close tools">×</button>
				</div>

				{#if activeMapPanel === 'layers'}
					<LayersPanel
						knownRoutesVisible={knownRoutesVisible}
						routeLabelsVisible={routeLabelsVisible}
						routeLabelsDisabled={!knownRoutesVisible}
						ordinaryPathsVisible={ordinaryPathsVisible}
						outdoorPoisVisible={outdoorPoisVisible}
						peakLabelsVisible={peakLabelsVisible}
						regionBoundariesVisible={regionBoundariesVisible}
						onKnownRoutesToggle={(v) => (knownRoutesVisible = v)}
						onRouteLabelsToggle={(v) => (routeLabelsVisible = v)}
						onOrdinaryPathsToggle={(v) => (ordinaryPathsVisible = v)}
						onOutdoorPoisToggle={(v) => (outdoorPoisVisible = v)}
						onPeakLabelsToggle={(v) => (peakLabelsVisible = v)}
						onRegionBoundariesToggle={(v) => (regionBoundariesVisible = v)}
					/>
				{:else if activeMapPanel === 'terrain'}
					<TerrainControls
						mapZoom={mapZoom}
						mapPitch={mapPitch}
						mapBearing={mapBearing}
						terrain3dEnabled={terrain3dEnabled}
						terrainReady={terrainReady}
						terrainHillshadeVisible={terrainHillshadeVisible}
						terrainColorReliefVisible={terrainColorReliefVisible}
						terrainColorReliefSupported={terrainColorReliefSupported}
						terrainSlopeVisible={terrainSlopeVisible}
						terrainSlopeSupported={terrainSlopeSupported}
						terrainSlopeOpacity={terrainSlopeOpacity}
						terrainContoursVisible={terrainContoursVisible}
						terrainContourUnit={terrainContourUnit}
						onHillshadeToggle={setTerrainHillshadeEnabled}
						onColorReliefToggle={setTerrainColorReliefEnabled}
						onSlopeToggle={setTerrainSlopeEnabled}
						onContoursToggle={setTerrainContoursEnabled}
						onSlopeOpacityChange={setTerrainSlopeOpacity}
						onContourUnitChange={setTerrainContourUnit}
						onSet3DViewPreset={set3DViewPreset}
						onMapPitchChange={setMapPitchValue}
						onMapBearingChange={setMapBearingValue}
						onRotateMap={rotateMap}
					/>
				{/if}
			</aside>
		{/if}
	</div>

	{#if routeChoices.length > 1 && routeChooserPosition}
		<div
			class="route-chooser"
			style={`left:${routeChooserPosition.x}px;top:${routeChooserPosition.y}px;`}
		>
			<div class="route-chooser-header">
				<h3>Routes here</h3>
				<button class="route-chooser-close" onclick={clearRouteChoices} aria-label="Close route choices">×</button>
			</div>
			<p class="route-chooser-note">
				Multiple trails share this section. Pick the specific route you want to inspect.
			</p>
			<div class="route-choice-list">
				{#each routeChoices as choice}
					<button
						class:preview={hoveredRouteChoiceId === choice.id}
						class="route-choice"
						onmouseenter={() => previewRouteChoice(choice.id)}
						onmouseleave={() => previewRouteChoice(null)}
						onfocus={() => previewRouteChoice(choice.id)}
						onblur={() => previewRouteChoice(null)}
						onclick={() => loadTrail(choice.id)}
					>
						{#if choice.tag}
							<span class="route-tag">{choice.tag}</span>
						{/if}
						<div>
							<strong>{choice.name}</strong>
							{#if choice.network}
								<p>{choice.network}</p>
							{/if}
						</div>
					</button>
				{/each}
			</div>
		</div>
	{/if}

	<FeaturePanel
		selectedFeature={selectedMapFeature}
		weatherForecast={weatherForecast}
		weatherStatus={weatherStatus}
		weatherError={weatherError}
		nearbyFeatures={nearbyMapFeatures}
		nearbyTrails={nearbyTrailChoices}
		coordinateCopyStatus={coordinateCopyStatus}
		onClose={closePanel}
		onCenter={centerSelectedMapFeature}
		onCopyCoordinates={copySelectedCoordinates}
		onSelectNearby={selectNearbyMapFeature}
		onLoadTrail={(id) => loadTrail(id)}
		onRefreshWeather={(coords) => refreshWeather(coords ?? undefined, true)}
	/>

	{#if selectedTrail || detailError}
		<TrailDetailPanel
			selectedTrail={selectedTrail}
			incompleteRoute={incompleteRoute}
			downloadBusy={downloadBusy}
			shareMessage={shareMessage}
			shareFallback={shareFallback}
			downloadError={downloadError}
			elevationStatus={elevationStatus}
			elevationProfile={elevationProfile}
			weatherForecast={weatherForecast}
			weatherStatus={weatherStatus}
			weatherError={weatherError}
			detailError={detailError}
			onClose={closePanel}
			onCenter={fitToSelectedTrail}
			onShare={shareTrail}
			onExportGPX={() => downloadRoute("gpx")}
			onReloadRoute={() => selectedTrail && loadTrail(selectedTrail.id, false, false)}
			onShowWaypoint={showWaypoint}
			onRefreshWeather={(coords) => refreshWeather(coords ?? undefined, true)}
			onElevationHover={(point) => { elevationHoverPosM = point?.pos_m ?? null; elevationHoverEleM = point?.ele_m ?? null; }}
		/>
	{/if}
</div>

<style>
	:global(.maplibregl-popup-content) {
		font-family: inherit;
	}

	.map-shell {
		--ui-spruce: #202824;
		--ui-spruce-deep: #17201b;
		--ui-spruce-raised: #29342e;
		--ui-text: #faf7ee;
		--ui-muted: #aab3ae;
		--ui-soft: #c9d0cc;
		--ui-border: rgba(238, 242, 233, 0.14);
		--ui-brand: #2e6653;
		--ui-brand-hover: #245443;
		--ui-focus: #79c69b;
		--ui-route: #245ee7;
		--ui-danger: #d95b32;
		--ui-paper: rgba(250, 247, 238, 0.96);
		--ui-paper-ink: #25302a;
		--ui-shadow: rgba(22, 31, 26, 0.2);
		position: relative;
		width: 100%;
		height: 100%;
		overflow: hidden;
		overflow: clip;
		font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
		background:
			radial-gradient(circle at top left, rgba(46, 102, 83, 0.24), transparent 28%),
			radial-gradient(circle at bottom right, rgba(36, 94, 231, 0.12), transparent 30%),
			var(--ui-spruce-deep);
	}

	.map {
		width: 100%;
		height: 100%;
		opacity: 0;
		transition: opacity 220ms ease;
	}

	.map.ready {
		opacity: 1;
	}

	.map-loading {
		position: absolute;
		inset: 0;
		z-index: 2;
		display: grid;
		place-items: center;
		background:
			radial-gradient(circle at 22% 18%, rgba(143, 201, 222, 0.22), transparent 28%),
			radial-gradient(circle at 76% 72%, rgba(166, 192, 149, 0.3), transparent 34%),
			linear-gradient(145deg, #e9e8dc 0%, #d9e3d0 48%, #c9d8c0 100%);
		opacity: 1;
		visibility: visible;
		transition:
			opacity 220ms ease,
			visibility 0s linear 0s;
	}

	.map-loading::before,
	.map-loading::after {
		position: absolute;
		inset: -20%;
		content: '';
		pointer-events: none;
	}

	.map-loading::before {
		background: repeating-radial-gradient(
			ellipse at 38% 56%,
			transparent 0 26px,
			rgba(60, 84, 67, 0.075) 27px 28px
		);
		transform: rotate(-8deg) scale(1.15);
	}

	.map-loading::after {
		background: linear-gradient(108deg, transparent 42%, rgba(255, 255, 255, 0.28) 50%, transparent 58%);
		animation: map-loading-sweep 1.7s ease-in-out infinite;
	}

	.map-loading.ready {
		opacity: 0;
		visibility: hidden;
		transition:
			opacity 220ms ease,
			visibility 0s linear 220ms;
	}

	.map-loading-card {
		position: relative;
		z-index: 1;
		display: grid;
		justify-items: center;
		gap: 0.25rem;
		padding: 1rem 1.25rem;
		border: 1px solid rgba(37, 48, 42, 0.12);
		border-radius: 18px;
		background: rgba(250, 247, 238, 0.88);
		backdrop-filter: blur(12px);
		box-shadow: 0 18px 40px rgba(37, 48, 42, 0.14);
		color: #25302a;
	}

	.map-loading-card strong {
		font-size: 0.92rem;
		letter-spacing: -0.01em;
	}

	.map-loading-card > span:not(.map-loading-mark) {
		max-width: 28rem;
		color: #5d6961;
		font-size: 0.75rem;
		line-height: 1.4;
		text-align: center;
	}

	.map-loading-mark {
		display: grid;
		place-items: center;
		width: 2rem;
		height: 2rem;
		margin-bottom: 0.15rem;
		border-radius: 50%;
		background: #2e6653;
		color: #fff9ec;
		font-size: 1.2rem;
		font-weight: 800;
		animation: map-loading-pulse 1.2s ease-in-out infinite;
	}

	.map-loading-mark.error {
		background: #d95b32;
		animation: none;
	}

	.map-loading-card button {
		margin-top: 0.45rem;
		padding: 0.55rem 0.85rem;
		border: 0;
		border-radius: 10px;
		background: #2e6653;
		color: #fff9ec;
		font-weight: 750;
		cursor: pointer;
	}

	@keyframes map-loading-sweep {
		0%,
		20% {
			transform: translateX(-30%);
		}
		80%,
		100% {
			transform: translateX(30%);
		}
	}

	@keyframes map-loading-pulse {
		0%,
		100% {
			transform: scale(0.92);
			opacity: 0.72;
		}
		50% {
			transform: scale(1);
			opacity: 1;
		}
	}

	.map-shell.trail-open :global(.maplibregl-ctrl-bottom-left) {
		left: calc(min(460px, calc(100vw - 2rem)) + 2rem);
	}

	:global(.maplibregl-ctrl-top-right) {
		top: 0.65rem;
		right: 0.65rem;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 0.42rem;
	}

	:global(.maplibregl-ctrl-top-right .maplibregl-ctrl) {
		margin: 0;
	}

	:global(.maplibregl-ctrl-group) {
		overflow: hidden;
		border: 1px solid rgba(37, 48, 42, 0.14);
		border-radius: 14px;
		background: var(--ui-paper);
		box-shadow: 0 12px 28px var(--ui-shadow);
	}

	:global(.maplibregl-ctrl-group button) {
		width: 46px;
		height: 46px;
		background-color: transparent;
	}

	:global(.maplibregl-user-location-dot) {
		width: 18px;
		height: 18px;
		border: 3px solid #fff9ec;
		background-color: #17355f;
		box-shadow: 0 2px 8px rgba(23, 53, 95, 0.42);
	}

	:global(.maplibregl-user-location-accuracy-circle) {
		background-color: rgba(36, 94, 231, 0.13);
	}

	:global(.maplibregl-ctrl-group button + button) {
		border-top-color: rgba(37, 48, 42, 0.12);
	}

	:global(.maplibregl-ctrl-group button:hover) {
		background-color: rgba(46, 102, 83, 0.1);
	}

	:global(.maplibregl-ctrl-bottom-left) {
		left: 1rem;
		bottom: 0.9rem;
	}

	:global(.maplibregl-ctrl-bottom-right) {
		right: 0.75rem;
		bottom: 0.55rem;
		max-width: calc(100% - 1.5rem);
	}

	:global(.maplibregl-ctrl-attrib) {
		background: rgba(250, 247, 238, 0.9);
		color: #25302a;
	}

	:global(.maplibregl-ctrl-scale) {
		margin: 0;
		padding: 0.28rem 0.5rem;
		border: 1px solid rgba(37, 48, 42, 0.14);
		border-radius: 999px;
		background: var(--ui-paper);
		backdrop-filter: blur(12px);
		box-shadow: 0 10px 24px var(--ui-shadow);
		color: var(--ui-paper-ink);
		font-size: 0.76rem;
		line-height: 1.1;
	}

		.search-dock {
			position: absolute;
			top: 1rem;
			left: 1rem;
			z-index: 5;
			width: min(460px, calc(100vw - 2rem));
			transition:
				width 160ms ease,
				transform 160ms ease;
		}

		.search-dock.panel-open {
			width: min(460px, calc(100vw - 2rem));
		}

		.search-card {
			border: 1px solid var(--ui-border);
			border-radius: 20px;
			background:
				linear-gradient(180deg, rgba(32, 40, 36, 0.97), rgba(23, 32, 27, 0.94));
			backdrop-filter: blur(18px);
			box-shadow:
				0 18px 38px rgba(22, 31, 26, 0.3),
				inset 0 1px 0 rgba(255, 255, 255, 0.04);
			color: var(--ui-text);
			overflow: hidden;
		}

		.search-card.open {
			border-color: rgba(121, 198, 155, 0.48);
			box-shadow:
				0 18px 38px rgba(22, 31, 26, 0.3),
				0 0 0 3px rgba(121, 198, 155, 0.12);
		}

		.search-input-wrap {
			display: flex;
			align-items: center;
			gap: 0.7rem;
			padding: 0.82rem 0.95rem;
		}

	.search-icon {
		color: var(--ui-muted);
		font-size: 1.05rem;
		line-height: 1;
	}

		.search-input {
			flex: 1;
			border: none;
			outline: none;
			background: transparent;
			color: var(--ui-text);
			font-size: 1.02rem;
			font-weight: 650;
		}

	.search-input::placeholder {
		color: var(--ui-muted);
		font-weight: 500;
	}

		.search-clear {
			width: 2.05rem;
			height: 2.05rem;
			border: 1px solid rgba(255, 255, 255, 0.08);
			border-radius: 999px;
			background: rgba(255, 255, 255, 0.06);
			color: var(--ui-text);
			font-size: 1.1rem;
			cursor: pointer;
			transition:
				background 120ms ease,
				border-color 120ms ease,
				transform 120ms ease;
		}

		.search-clear:hover {
			border-color: rgba(121, 198, 155, 0.42);
			background: rgba(121, 198, 155, 0.12);
			transform: translateY(-1px);
		}

		.search-results {
			border-top: 1px solid rgba(255, 255, 255, 0.08);
			padding: 0.3rem 0.35rem 0.4rem;
			max-height: min(30rem, 42vh);
			overflow: auto;
		}

	.search-hint,
	.search-error {
		margin: 0;
		padding: 0.85rem 0.85rem 0.95rem;
		color: var(--ui-muted);
		font-size: 0.9rem;
		line-height: 1.45;
	}

	.search-error {
		color: #ff9f9f;
	}

		.search-result-list {
			display: grid;
			gap: 0.5rem;
		}

		.search-group-label {
			padding: 0.6rem 0.45rem 0.12rem;
			color: var(--ui-focus);
			font-size: 0.72rem;
			font-weight: 800;
			letter-spacing: 0.12em;
			text-transform: uppercase;
		}

		.search-result {
			width: 100%;
			padding: 0.82rem 0.88rem 0.9rem;
			border: 1px solid rgba(255, 255, 255, 0.07);
			border-radius: 15px;
			background: linear-gradient(180deg, rgba(255, 255, 255, 0.035), rgba(255, 255, 255, 0.02));
			color: inherit;
			text-align: left;
			cursor: pointer;
		transition:
			border-color 120ms ease,
			background 120ms ease,
			transform 120ms ease;
	}

		.search-result:hover,
		.search-result.active {
			border-color: rgba(121, 198, 155, 0.48);
			background: linear-gradient(180deg, rgba(121, 198, 155, 0.13), rgba(121, 198, 155, 0.07));
			transform: translateY(-1px);
		}

	.search-result-topline {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		flex-wrap: wrap;
		margin-bottom: 0.32rem;
	}

		.search-kind-pill {
			display: inline-flex;
			align-items: center;
			padding: 0.18rem 0.5rem;
			border-radius: 999px;
			background: rgba(121, 198, 155, 0.15);
			color: #aee2c3;
			font-size: 0.72rem;
			font-weight: 700;
		letter-spacing: 0.04em;
		text-transform: uppercase;
	}

		.search-kind-pill.region-pill {
			background: rgba(140, 186, 221, 0.16);
			color: #bad8ef;
		}

	.search-network,
	.search-distance {
		color: var(--ui-muted);
		font-size: 0.8rem;
		font-weight: 600;
	}

	.search-result strong {
		display: block;
		font-size: 0.98rem;
	}

	.search-result p {
		margin: 0.28rem 0 0;
		color: var(--ui-muted);
		font-size: 0.86rem;
		line-height: 1.4;
	}

	.terrain-dock {
		position: absolute;
		top: 16.65rem;
		right: 0.75rem;
		z-index: 3;
		display: flex;
		flex-direction: row-reverse;
		align-items: flex-start;
		gap: 0.7rem;
	}

	.map-tool-rail {
		display: grid;
		gap: 0;
		padding: 0;
		overflow: hidden;
		border: 1px solid rgba(37, 48, 42, 0.14);
		border-radius: 14px;
		background: var(--ui-paper);
		box-shadow: 0 12px 28px var(--ui-shadow);
	}

	.map-tool-rail button {
		display: grid;
		place-items: center;
		width: 46px;
		height: 46px;
		min-height: 46px;
		padding: 0;
		border: 0;
		border-radius: 0;
		background: transparent;
		color: var(--ui-paper-ink);
		cursor: pointer;
		transition: background 120ms ease, color 120ms ease;
	}

	.map-tool-rail button + button {
		border-top: 1px solid rgba(37, 48, 42, 0.12);
	}

	.map-tool-rail button:hover {
		background: rgba(46, 102, 83, 0.1);
	}

	.map-tool-rail button.active {
		background: var(--ui-brand);
		color: #fff9ec;
	}

	.map-tool-rail button > span {
		font-size: 1rem;
		font-weight: 800;
		line-height: 1;
	}

	.map-tool-rail button svg {
		width: 21px;
		height: 21px;
		fill: none;
		stroke: currentColor;
		stroke-width: 1.9;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	.map-tool-rail button:disabled {
		cursor: not-allowed;
		opacity: 0.45;
	}

	.map-tool-rail .tool-3d {
		font-size: 0.9rem;
		letter-spacing: -0.03em;
	}

	.terrain-card {
		width: min(370px, calc(100vw - 5.5rem));
		max-height: calc(100vh - 18rem);
		overflow: auto;
		padding: 1rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 20px;
		background: rgba(32, 40, 36, 0.96);
		backdrop-filter: blur(14px);
		box-shadow: 0 22px 44px rgba(22, 31, 26, 0.36);
		color: var(--ui-text);
	}

	.map-tool-panel {
		scrollbar-color: rgba(121, 198, 155, 0.45) transparent;
	}

	.terrain-card-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 0.85rem;
	}

	.tool-panel-close {
		display: grid;
		place-items: center;
		flex: 0 0 auto;
		width: 2rem;
		height: 2rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 50%;
		background: rgba(255, 255, 255, 0.05);
		color: var(--ui-text);
		font-size: 1.15rem;
		cursor: pointer;
	}

	.view-presets {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.45rem;
		margin-top: 0.9rem;
	}

	.bearing-actions {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 0.45rem;
		margin-top: 0.65rem;
	}

	.view-presets button,
	.bearing-actions button {
		min-height: 2.55rem;
		padding: 0.55rem 0.5rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 11px;
		background: rgba(255, 255, 255, 0.055);
		color: var(--ui-text);
		font-size: 0.74rem;
		font-weight: 750;
		cursor: pointer;
	}

	.view-presets button.active,
	.view-presets button:hover,
	.bearing-actions button:hover {
		border-color: rgba(121, 198, 155, 0.52);
		background: rgba(121, 198, 155, 0.12);
	}

	.bearing-row {
		margin-top: 0.75rem;
	}

	.terrain-card-header h3 {
		margin: 0;
		font-size: 1rem;
	}

	.terrain-card-header p {
		margin: 0.35rem 0 0;
		color: var(--ui-muted);
		font-size: 0.88rem;
		line-height: 1.45;
	}

	.terrain-control-list {
		display: grid;
		gap: 0.75rem;
		margin-top: 0.95rem;
	}

	.terrain-switch {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 0.9rem;
		padding: 0.85rem 0.9rem;
		border-radius: 16px;
		background: rgba(255, 255, 255, 0.04);
	}

	.terrain-switch strong {
		display: block;
		font-size: 0.95rem;
	}

	.terrain-switch p {
		margin: 0.24rem 0 0;
		color: var(--ui-muted);
		font-size: 0.84rem;
		line-height: 1.4;
	}

	.terrain-switch input[type='checkbox'] {
		margin-top: 0.2rem;
		width: 1rem;
		height: 1rem;
		accent-color: var(--ui-focus);
	}

	.slope-panel {
		margin-top: 0.9rem;
		padding: 0.9rem;
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 16px;
		background: rgba(255, 255, 255, 0.045);
	}

	.slope-legend {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.48rem 0.7rem;
		margin-top: 0.75rem;
	}

	.slope-legend span {
		display: flex;
		align-items: center;
		gap: 0.42rem;
		color: var(--ui-muted);
		font-size: 0.78rem;
	}

	.slope-swatch {
		flex: 0 0 auto;
		width: 0.85rem;
		height: 0.85rem;
		border: 1px solid rgba(255, 255, 255, 0.32);
		border-radius: 3px;
		background: var(--slope-color);
	}

	.slope-swatch.clear {
		background: repeating-linear-gradient(
			135deg,
			rgba(255, 255, 255, 0.05) 0 3px,
			rgba(255, 255, 255, 0.16) 3px 5px
		);
	}

	.slope-note {
		margin: 0.75rem 0 0;
		color: var(--ui-muted);
		font-size: 0.76rem;
		line-height: 1.42;
	}

	.terrain-fieldset {
		margin-top: 0.9rem;
		padding: 0.9rem;
		border-radius: 16px;
		background: rgba(255, 255, 255, 0.05);
	}

	.terrain-fieldset-label {
		display: block;
		font-size: 0.78rem;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--ui-muted);
	}

	.terrain-slider-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
	}

	.terrain-slider-row strong {
		color: var(--ui-focus);
		font-size: 0.95rem;
	}

	.terrain-segmented {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.65rem;
		margin-top: 0.7rem;
	}

	.terrain-segmented label {
		display: flex;
		align-items: center;
		gap: 0.55rem;
		padding: 0.7rem 0.8rem;
		border-radius: 14px;
		background: rgba(255, 255, 255, 0.04);
	}

	.terrain-segmented input[type='radio'] {
		margin: 0;
		accent-color: var(--ui-focus);
	}

	.terrain-slider {
		width: 100%;
		margin-top: 0.8rem;
		accent-color: var(--ui-focus);
	}

	.route-chooser {
		position: absolute;
		z-index: 4;
		width: min(320px, calc(100vw - 2rem));
		padding: 0.9rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 18px;
		background: rgba(32, 40, 36, 0.96);
		backdrop-filter: blur(12px);
		box-shadow: 0 20px 42px rgba(22, 31, 26, 0.36);
		color: var(--ui-text);
	}

	.route-chooser-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.8rem;
	}

	.route-chooser-header h3 {
		margin: 0;
		font-size: 1rem;
	}

	.route-chooser-close {
		width: 1.9rem;
		height: 1.9rem;
		border: none;
		border-radius: 999px;
		background: rgba(255, 255, 255, 0.08);
		color: var(--ui-text);
		font-size: 1.1rem;
		cursor: pointer;
	}

	.route-chooser-note {
		margin: 0.55rem 0 0.85rem;
		color: var(--ui-muted);
		font-size: 0.9rem;
		line-height: 1.4;
	}

	.route-choice-list {
		display: grid;
		gap: 0.6rem;
	}

	.route-choice {
		display: flex;
		gap: 0.8rem;
		align-items: flex-start;
		width: 100%;
		padding: 0.8rem 0.85rem;
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 14px;
		background: rgba(255, 255, 255, 0.04);
		color: inherit;
		text-align: left;
		cursor: pointer;
		transition:
			border-color 120ms ease,
			background 120ms ease,
			transform 120ms ease;
	}

	.route-choice:hover {
		border-color: rgba(121, 198, 155, 0.55);
		background: rgba(121, 198, 155, 0.1);
		transform: translateY(-1px);
	}

	.route-choice.preview {
		border-color: rgba(121, 198, 155, 0.55);
		background: rgba(121, 198, 155, 0.1);
		transform: translateY(-1px);
	}

	.route-choice p {
		margin: 0.18rem 0 0;
		color: var(--ui-muted);
		font-size: 0.86rem;
	}

	.route-tag {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 2.25rem;
		padding: 0.15rem 0.45rem;
		border: 2px solid var(--ui-route);
		background: var(--ui-paper);
		color: var(--ui-paper-ink);
		font-size: 0.78rem;
		font-weight: 700;
		line-height: 1.1;
	}

		.trail-panel {
			position: absolute;
			top: 5.6rem;
			left: 1rem;
			bottom: 1rem;
			width: min(460px, calc(100vw - 2rem));
			box-sizing: border-box;
			overflow: auto;
			padding: 1rem 1rem 1.25rem;
			border: 1px solid var(--ui-border);
			border-radius: 24px;
			background:
				linear-gradient(180deg, rgba(32, 40, 36, 0.97), rgba(23, 32, 27, 0.95));
			backdrop-filter: blur(16px);
			box-shadow:
				0 22px 54px rgba(22, 31, 26, 0.34),
				inset 0 1px 0 rgba(255, 255, 255, 0.03);
			color: var(--ui-text);
			font-family:
				'ui-sans-serif',
			'system-ui',
			sans-serif;
	}

		.close {
			position: absolute;
			top: 0.95rem;
			right: 0.95rem;
			width: 2.1rem;
			height: 2.1rem;
			border: 1px solid rgba(255, 255, 255, 0.08);
			border-radius: 999px;
			background: rgba(255, 255, 255, 0.06);
			color: var(--ui-text);
			font-size: 1.05rem;
			cursor: pointer;
			z-index: 2;
			transition:
				background 120ms ease,
				border-color 120ms ease,
				transform 120ms ease;
		}

		.close:hover {
			border-color: rgba(121, 198, 155, 0.42);
			background: rgba(121, 198, 155, 0.12);
			transform: translateY(-1px);
		}

		.error {
			margin-top: 2.8rem;
			color: #ffb4ab;
		}

		.breadcrumb {
			margin: 0.2rem 2.6rem 0.65rem 0;
			font-size: 0.76rem;
			letter-spacing: 0.08em;
			text-transform: uppercase;
			color: #8f9a87;
		}

		.hero {
			padding-right: 3rem;
		}

	.hero h2 {
		margin: 0;
		font-size: clamp(1.5rem, 3vw, 1.9rem);
		line-height: 1.12;
	}

	.eyebrow {
		margin: 0 0 0.6rem;
		font-size: 0.82rem;
		font-weight: 700;
		color: #ffd54f;
	}

		.summary {
			margin: 0.72rem 0 0;
			color: #c7d0d8;
			line-height: 1.5;
			font-size: 0.95rem;
		}

	.caution-list {padding-left:1.1rem;font-size:.83rem;line-height:1.55;color:var(--ui-muted)}
	.caution-list li {margin-bottom:.55rem}.caution-list a {color:var(--ui-focus);white-space:nowrap}
	.waypoint-section summary,.weather-section summary {cursor:pointer;font-size:.88rem;font-weight:650;padding:.2rem 0}
	.waypoint-list {max-height:270px;overflow:auto;overscroll-behavior:contain}.waypoint-list button {display:flex;justify-content:space-between;align-items:center;text-align:left;width:100%;padding:.7rem .2rem;background:transparent;border:0;border-bottom:1px solid var(--ui-border);color:var(--ui-text);cursor:pointer}.waypoint-list small {display:block;color:var(--ui-muted);font-size:.73rem;margin-top:.25rem}

	.source-credit,.share-status { font-size:.75rem; line-height:1.5; color:var(--ui-muted); }
	.source-credit a { color:var(--ui-focus); }
	.share-link { width:100%;box-sizing:border-box;padding:.7rem;border:1px solid var(--ui-border);border-radius:8px; }
	.route-quality { margin-top:.85rem;padding:.65rem .8rem;background:#a4752026;border:1px solid #d9ad5a55;border-radius:12px;color:#eedab1;font-size:.8rem; }
	.route-quality p { margin:.3rem 0 0;line-height:1.45; }

		.stats {
			display: grid;
			grid-template-columns: repeat(4, minmax(0, 1fr));
			gap: 0.5rem;
			margin-top: 0.8rem;
		}

		.stats div {
			padding: 0.7rem 0.6rem;
			border-radius: 16px;
			background: linear-gradient(180deg, rgba(255, 255, 255, 0.045), rgba(255, 255, 255, 0.03));
		}

	.stats span {
		display: block;
		margin-bottom: 0.35rem;
		font-size: 0.68rem;
		color: var(--ui-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

		.stats strong {
			font-size: 0.9rem;
		}

	.feature-stats {
		grid-template-columns: repeat(2, minmax(0, 1fr));
	}

	.feature-stats small {
		display: block;
		margin-top: 0.32rem;
		color: var(--ui-muted);
		font-size: 0.74rem;
		line-height: 1.35;
	}

	.trail-quick-actions {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 0.55rem;
		margin: 0.8rem 0;
	}

	.trail-quick-actions button,
	.trail-quick-actions a {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		min-height: 2.7rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 13px;
		background: rgba(255, 255, 255, 0.045);
		color: var(--ui-text);
		font-size: 0.78rem;
		font-weight: 750;
		cursor: pointer;
		text-decoration: none;
	}

	.trail-quick-actions button:hover,
	.trail-quick-actions a:hover {
		border-color: rgba(121, 198, 155, 0.45);
		background: rgba(121, 198, 155, 0.1);
	}

	.feature-actions {
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}

	.nearby-feature-list {
		display: grid;
		gap: 0.55rem;
	}

	.nearby-feature {
		display: grid;
		gap: 0.2rem;
		width: 100%;
		padding: 0.8rem 0.9rem;
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 14px;
		background: rgba(255, 255, 255, 0.04);
		color: var(--ui-text);
		text-align: left;
		cursor: pointer;
	}

	.nearby-feature:hover {
		border-color: rgba(121, 198, 155, 0.45);
		background: rgba(121, 198, 155, 0.1);
	}

	.nearby-feature span {
		color: var(--ui-focus);
		font-size: 0.72rem;
		font-weight: 800;
		letter-spacing: 0.06em;
		text-transform: uppercase;
	}

	.nearby-feature small {
		color: var(--ui-muted);
		font-size: 0.74rem;
	}

		.section {
			margin-top: 1.2rem;
		}

		.section h3 {
			margin: 0 0 0.68rem;
			font-size: 0.98rem;
		}

	.meta-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.75rem;
	}

		.meta-card {
			padding: 0.85rem 0.9rem;
			border-radius: 16px;
			background: linear-gradient(180deg, rgba(255, 255, 255, 0.045), rgba(255, 255, 255, 0.03));
		}

	.meta-card-wide {
		grid-column: 1 / -1;
	}

	.meta-card span {
		display: block;
		margin-bottom: 0.38rem;
		font-size: 0.8rem;
		color: var(--ui-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.meta-card strong {
		display: block;
		font-size: 1rem;
		line-height: 1.3;
	}

	.meta-card p {
		margin: 0.18rem 0 0;
		color: var(--ui-muted);
		font-size: 0.88rem;
		line-height: 1.45;
	}

	.meta-link {
		color: #9bd9b6;
		word-break: break-word;
		text-decoration: none;
	}

	.meta-link:hover {
		text-decoration: underline;
	}

	.download-section {
		padding: 0.2rem 0 0;
	}

	.section-toggle {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding: 0;
		border: none;
		background: transparent;
		color: inherit;
		cursor: pointer;
		text-align: left;
	}

	.section-title {
		margin-bottom: 0;
		font-size: 1rem;
		font-weight: 700;
	}

	.section-toggle span {
		color: var(--ui-focus);
		font-size: 1.1rem;
		transition: transform 120ms ease;
	}

	.section-toggle span.open {
		transform: rotate(180deg);
	}

	.download-fieldset {
		margin-top: 0.9rem;
		padding: 0.95rem 1rem;
		border-radius: 16px;
		background: rgba(255, 255, 255, 0.05);
	}

	.fieldset-label {
		display: block;
		margin-bottom: 0.7rem;
		font-size: 0.82rem;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		color: var(--ui-muted);
	}

	.radio-list {
		display: grid;
		gap: 0.65rem;
	}

	.format-list {
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}

	.format-list label,
	.direction-list label {
		display: flex;
		gap: 0.7rem;
		align-items: flex-start;
		padding: 0.75rem 0.8rem;
		border-radius: 14px;
		background: rgba(255, 255, 255, 0.04);
	}

	.format-list label {
		align-items: center;
	}

	.radio-list input {
		margin: 0.15rem 0 0;
		accent-color: var(--ui-focus);
	}

	.direction-list strong {
		display: block;
		font-size: 0.96rem;
	}

	.direction-list p {
		margin: 0.2rem 0 0;
		color: var(--ui-muted);
		font-size: 0.86rem;
		line-height: 1.35;
	}

	.download-button {
		width: 100%;
		margin-top: 0.95rem;
		padding: 0.92rem 1rem;
		border: none;
		border-radius: 16px;
		background: var(--ui-brand);
		color: var(--ui-text);
		font-size: 1rem;
		font-weight: 800;
		cursor: pointer;
		transition:
			transform 120ms ease,
			filter 120ms ease;
	}

	.download-button:hover:not(:disabled) {
		transform: translateY(-1px);
		background: var(--ui-brand-hover);
	}

	.download-button:disabled {
		cursor: progress;
		opacity: 0.72;
	}

	.download-error {
		margin: 0.7rem 0 0;
		color: #ffb4ab;
		font-size: 0.9rem;
	}

	.analysis-note {
		margin: 0 0 0.8rem;
		color: var(--ui-muted);
		font-size: 0.9rem;
		line-height: 1.45;
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.chip {
		padding: 0.45rem 0.8rem;
		border-radius: 999px;
		background: rgba(255, 255, 255, 0.08);
		color: #dfe5eb;
		font-size: 0.9rem;
	}

	.surface-bar {
		display: flex;
		height: 0.9rem;
		overflow: hidden;
		border-radius: 999px;
		background: rgba(255, 255, 255, 0.08);
	}

	.surface-segment:first-child {
		border-top-left-radius: 999px;
		border-bottom-left-radius: 999px;
	}

	.surface-segment:last-child {
		border-top-right-radius: 999px;
		border-bottom-right-radius: 999px;
	}

	.surface-list {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.75rem;
		margin-top: 0.9rem;
	}

	.surface-item,
	.analysis-item {
		display: flex;
		gap: 0.75rem;
		align-items: flex-start;
		padding: 0.9rem 0.95rem;
		border-radius: 14px;
		background: rgba(255, 255, 255, 0.05);
	}

	.surface-item p,
	.analysis-item p {
		margin: 0.16rem 0 0;
		color: var(--ui-muted);
		font-size: 0.88rem;
	}

	.dot {
		display: inline-block;
		width: 0.9rem;
		height: 0.9rem;
		margin-top: 0.18rem;
		border-radius: 999px;
		flex: 0 0 auto;
	}

	.analysis-list {
		display: grid;
		gap: 0.7rem;
	}

	.count-pill {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 1.3rem;
		height: 1.3rem;
		margin-left: 0.45rem;
		padding: 0 0.38rem;
		border-radius: 999px;
		background: rgba(239, 68, 68, 0.16);
		color: #ffd3d3;
		font-size: 0.78rem;
		font-weight: 700;
		vertical-align: middle;
	}

	.start-dot {
		background: #16875d;
	}

	.end-dot {
		background: #2a3138;
	}

	.warning-dot {
		background: var(--ui-danger);
	}

	.branch-dot {
		background: #67e8f9;
	}

	.disconnected-dot {
		background: #9ca3af;
	}

	@media (prefers-reduced-motion: reduce) {
		.map,
		.map-loading,
		.map-loading::after,
		.map-loading-mark {
			animation: none;
			transition: none;
		}
	}

		@media (max-width: 740px) {
			.map-shell.terrain-open :global(.maplibregl-ctrl-bottom-left),
			.map-shell.tools-open :global(.maplibregl-ctrl-bottom-left),
			.map-shell.trail-open :global(.maplibregl-ctrl-bottom-left) {
				display: none;
			}
			:global(.maplibregl-ctrl-top-right) {
				top: 4.55rem;
				right: 0.75rem;
			}

			:global(.maplibregl-ctrl-top-right .maplibregl-ctrl-group) {
				display: flex;
			}

			:global(.maplibregl-ctrl-group button + button) {
				border-top: 0;
				border-left: 1px solid rgba(37, 48, 42, 0.12);
			}

			:global(.maplibregl-ctrl-bottom-left) {
				left: 0.75rem;
				bottom: 2.8rem;
			}

			:global(.maplibregl-ctrl-bottom-right) {
				bottom: 4.15rem;
			}

			.map-shell.terrain-open :global(.maplibregl-ctrl-bottom-right),
			.map-shell.tools-open :global(.maplibregl-ctrl-bottom-right),
			.map-shell.trail-open :global(.maplibregl-ctrl-bottom-right) {
				top: 4.6rem;
				right: 4.5rem;
				max-width: calc(100% - 5.25rem);
				bottom: auto;
			}

			.search-dock {
				top: 0.75rem;
				left: 0.75rem;
				width: calc(100vw - 1.5rem);
			}

			.search-dock.panel-open {
				width: calc(100vw - 1.5rem);
			}

			.terrain-dock {
				top: auto;
				right: 0.75rem;
				bottom: 0.75rem;
				left: auto;
				flex-direction: column-reverse;
				align-items: flex-end;
			}

			.map-tool-rail {
				grid-template-columns: repeat(3, minmax(0, 1fr));
			}

			.map-tool-rail button {
				width: 46px;
			}

			.map-tool-rail button + button {
				border-top: 0;
				border-left: 1px solid rgba(37, 48, 42, 0.12);
			}

			.terrain-card {
				width: calc(100vw - 1.5rem);
				max-height: calc(100vh - 11rem);
				overflow: auto;
			}

			.map-shell.trail-open .terrain-dock {
				display: none;
			}

			.trail-panel {
				top: auto;
				right: 0.75rem;
				left: 0.75rem;
				bottom: 0.75rem;
				width: auto;
				max-height: 58vh;
				max-height: 58dvh;
			}

		.meta-grid,
		.surface-list,
		.format-list {
			grid-template-columns: 1fr;
		}

			.stats { grid-template-columns: repeat(4, minmax(0, 1fr)); }
			.stats div { padding: .65rem .4rem; }
			.stats strong { font-size: .78rem; }
			.stats span { font-size: .6rem; }

		.feature-actions {
			grid-template-columns: 1fr;
		}
	}
</style>
