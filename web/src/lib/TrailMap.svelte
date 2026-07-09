	<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import maplibregl from 'maplibre-gl';
	import 'maplibre-gl/dist/maplibre-gl.css';
	import {
		addTerrainLayers,
		set3DTerrainEnabled,
		setContourUnit,
		setContoursVisible,
		setHillshadeVisible,
		setTerrainExaggeration
	} from './terrain-layers';

	const INITIAL_CENTER: [number, number] = [77.18, 32.24];
	const INITIAL_ZOOM = 9;
	const GLYPHS_URL =
		import.meta.env.VITE_MAP_GLYPHS_URL ??
		'https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf';
	const MARTIN_TRAILS_URL_TEMPLATE =
		import.meta.env.VITE_MARTIN_TRAILS_URL ?? '/trail_lines/{z}/{x}/{y}';
	const TERRAIN_RGB_URL_TEMPLATE =
		import.meta.env.VITE_TERRAIN_RGB_URL ?? '/terrain_rgb/{z}/{x}/{y}';
	const HOVERED_ROUTE_CASING_LAYER = 'hovered-route-casing';
	const HOVERED_ROUTE_LAYER = 'hovered-route-line';
	const SELECTED_TRAIL_SOURCE = 'selected-trail';
	const SELECTED_TRAIL_GLOW_LAYER = 'selected-trail-glow';
	const SELECTED_TRAIL_CASING_LAYER = 'selected-trail-casing';
	const SELECTED_TRAIL_LAYER = 'selected-trail-line';
	const SELECTED_TRAIL_ARROW_LAYER = 'selected-trail-direction';
	const SELECTED_TRAIL_ARROW_IMAGE = 'selected-trail-direction-chevron';
	const SELECTED_TRAIL_BOTH_LAYER = 'selected-trail-both-direction';
	const SELECTED_TRAIL_BOTH_IMAGE = 'selected-trail-direction-dot';
	const SELECTED_TRAIL_MARKER_LAYER = 'selected-trail-markers';

	type TrailGeometry = {
		type: string;
		coordinates?: unknown;
		geometries?: unknown[];
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

	type TrailDetail = {
		id: string;
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
			kind: string;
			name: string | null;
			elevation_m: number | null;
		}>;
		stitch_diagnostics: TrailDiagnostic[];
		stitch_warnings: TrailWarning[];
	};

	let mapContainer: HTMLDivElement;
	let map: maplibregl.Map | undefined;
	let selectedTrail: TrailDetail | null = $state(null);
	let detailError: string | null = $state(null);
	let directionMode: DirectionMode = $state('forward');
	let downloadFormat: DownloadFormat = $state('gpx');
	let downloadSectionOpen = $state(false);
	let downloadBusy = $state(false);
	let downloadError: string | null = $state(null);
	let trailRequestVersion = 0;
	let routeChoices: RouteChoice[] = $state([]);
	let routeChooserPosition: { x: number; y: number } | null = $state(null);
	let hoveredRouteChoiceId: string | null = $state(null);
	let elevationProfile: ElevationProfile | null = $state(null);
	let elevationStatus: 'idle' | 'loading' | 'ready' | 'missing' | 'error' = $state('idle');
	let elevationError: string | null = $state(null);
	let terrainPanelOpen = $state(false);
	let terrainReady = $state(false);
	let terrain3dEnabled = $state(false);
	let terrainHillshadeVisible = $state(true);
	let terrainContoursVisible = $state(false);
	let terrainContourUnit: 'meters' | 'feet' = $state('meters');
	let terrainExaggeration = $state(1.2);
	let terrainColorReliefSupported = $state(false);

	$effect(() => {
		if (!selectedTrail) return;
		directionMode;
		downloadSectionOpen;
		syncSelectedTrailLayer();
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
		terrainContourUnit;
		terrainExaggeration;

		if (!map || !terrainReady) return;

		setHillshadeVisible(map, terrainHillshadeVisible);
		setContoursVisible(map, terrainContoursVisible);
		setContourUnit(map, terrainContourUnit);
		setTerrainExaggeration(map, terrainExaggeration);
		set3DTerrainEnabled(map, terrain3dEnabled, terrainExaggeration);
	});

	onMount(() => {
		const martinTrailsUrl = absolutizeTileTemplate(MARTIN_TRAILS_URL_TEMPLATE);
		const terrainRgbUrl = absolutizeTileTemplate(TERRAIN_RGB_URL_TEMPLATE);

		map = new maplibregl.Map({
			container: mapContainer,
			style: {
				version: 8,
				glyphs: GLYPHS_URL,
				sources: {
					osm: {
						type: 'raster',
						tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
						tileSize: 256,
						attribution: '&copy; OpenStreetMap contributors'
					},
					trails: {
						type: 'vector',
						tiles: [martinTrailsUrl],
						minzoom: 0,
						maxzoom: 14
					}
				},
				layers: [
					{ id: 'osm', type: 'raster', source: 'osm' },
					{
						id: 'trail-lines',
						type: 'line',
						source: 'trails',
						'source-layer': 'trail_lines',
						layout: {
							'line-cap': 'round',
							'line-join': 'round'
						},
						paint: {
							'line-color': [
								'match',
								['get', 'difficulty'],
								'hiking', '#5f9f4f',
								'mountain_hiking', '#c9a227',
								'demanding_mountain_hiking', '#d27b2c',
								'alpine_hiking', '#c45843',
								'demanding_alpine_hiking', '#8b5aa8',
								'difficult_alpine_hiking', '#57457d',
								'#4d82b8'
							],
							'line-width': 2.6,
							'line-opacity': 0.72
						}
					},
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
							'line-color': 'rgba(65, 74, 20, 0.55)',
							'line-width': 10,
							'line-opacity': 0.88,
							'line-blur': 0.6
						}
					},
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
							'line-color': '#dfff2a',
							'line-width': 6.4,
							'line-opacity': 0.98
						}
					}
				]
			},
			center: INITIAL_CENTER,
			zoom: INITIAL_ZOOM
		});

		map.addControl(new maplibregl.NavigationControl(), 'top-right');
		map.on('load', () => {
			if (!map) return;
			const capabilities = addTerrainLayers(map, {
				terrainRgbUrl,
				beforeLayerId: 'trail-lines',
				terrainEnabled: terrain3dEnabled,
				hillshadeVisible: terrainHillshadeVisible,
				contoursVisible: terrainContoursVisible,
				contourUnit: terrainContourUnit,
				exaggeration: terrainExaggeration
			});
			terrainColorReliefSupported = capabilities.colorRelief;
			terrainReady = true;
		});

		map.on('click', 'trail-lines', async (event) => {
			const choices = routeChoicesAtPoint(event.point);
			if (choices.length === 0) return;

			if (choices.length === 1) {
				clearRouteChoices();
				await loadTrail(choices[0].id);
				return;
			}

			routeChoices = choices;
			routeChooserPosition = routeChooserPositionFor(event.point, choices.length);
		});

		map.on('click', (event) => {
			if (routeChoicesAtPoint(event.point).length === 0) {
				clearRouteChoices();
			}
		});

		map.on('mouseenter', 'trail-lines', () => {
			map?.getCanvas().style.setProperty('cursor', 'pointer');
		});

		map.on('mouseleave', 'trail-lines', () => {
			map?.getCanvas().style.setProperty('cursor', '');
		});
	});

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

	onDestroy(() => {
		map?.remove();
	});

	async function loadTrail(trailId: string) {
		const requestVersion = ++trailRequestVersion;
		detailError = null;

		try {
			clearRouteChoices();
			const response = await fetch(`/api/trails/${trailId}`);
			if (!response.ok) {
				throw new Error(`Trail request failed with ${response.status}`);
			}

			const payload = await response.json();
			if (requestVersion !== trailRequestVersion) return;

			const trailDetail: TrailDetail = {
				...payload,
				stitch_warnings: sanitizeTrailWarnings(payload?.stitch_warnings)
			};
			selectedTrail = trailDetail;
			elevationProfile = null;
			elevationStatus = 'loading';
			elevationError = null;
			directionMode = defaultDirectionMode(trailDetail);
			downloadFormat = 'gpx';
			downloadSectionOpen = false;
			downloadBusy = false;
			downloadError = null;
			void loadElevation(trailId);
			syncSelectedTrailLayer();
			fitToSelectedTrail();
		} catch (error) {
			if (requestVersion !== trailRequestVersion) return;
			selectedTrail = null;
			clearSelectedTrailLayer();
			detailError = error instanceof Error ? error.message : 'Failed to load trail details';
		}
	}

	async function loadElevation(trailId: string) {
		elevationStatus = 'loading';
		elevationError = null;

		try {
			const response = await fetch(`/api/trails/${trailId}/elevation`);
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

			const payload = await response.json();
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
			if (selectedTrail?.id === trailId) {
				elevationProfile = null;
				elevationStatus = 'error';
				elevationError =
					error instanceof Error ? error.message : 'Failed to load elevation profile';
			}
		}
	}

	function syncSelectedTrailLayer() {
		if (!map) return;

		const featureCollection = buildSelectedTrailFeatureCollection();
		ensureDirectionArrowImage();
		ensureBothDirectionImage();

		const source = map.getSource(SELECTED_TRAIL_SOURCE) as maplibregl.GeoJSONSource | undefined;
		if (source) {
			source.setData(featureCollection as never);
		} else {
			map.addSource(SELECTED_TRAIL_SOURCE, {
				type: 'geojson',
				data: featureCollection as never
			});
			map.addLayer({
				id: SELECTED_TRAIL_GLOW_LAYER,
				type: 'line',
				source: SELECTED_TRAIL_SOURCE,
				filter: ['==', ['get', 'role'], 'route'],
				layout: {
					'line-cap': 'round',
					'line-join': 'round'
				},
				paint: {
					'line-color': '#55620e',
					'line-width': 16,
					'line-opacity': 0.32,
					'line-blur': 1.2
				}
			});
			map.addLayer({
				id: SELECTED_TRAIL_CASING_LAYER,
				type: 'line',
				source: SELECTED_TRAIL_SOURCE,
				filter: ['==', ['get', 'role'], 'route'],
				layout: {
					'line-cap': 'round',
					'line-join': 'round'
				},
				paint: {
					'line-color': '#78880f',
					'line-width': 11,
					'line-opacity': 0.98
				}
			});
			map.addLayer({
				id: SELECTED_TRAIL_LAYER,
				type: 'line',
				source: SELECTED_TRAIL_SOURCE,
				filter: ['==', ['get', 'role'], 'route'],
				layout: {
					'line-cap': 'round',
					'line-join': 'round'
				},
				paint: {
					'line-color': '#dfff2a',
					'line-width': 7.2,
					'line-opacity': 0.98
				}
			});
				map.addLayer({
					id: SELECTED_TRAIL_ARROW_LAYER,
					type: 'symbol',
					source: SELECTED_TRAIL_SOURCE,
					filter: ['==', ['get', 'role'], 'direction-chevron'],
					layout: {
						'icon-image': SELECTED_TRAIL_ARROW_IMAGE,
						'icon-size': [
							'interpolate',
							['linear'],
							['zoom'],
							9,
							0.62,
							11,
							0.75,
							13,
							0.88,
							15,
							1
						],
						'icon-rotation-alignment': 'map',
						'icon-keep-upright': false,
						'icon-allow-overlap': true,
						'icon-ignore-placement': true,
						'symbol-placement': 'line',
						'symbol-spacing': 28
					},
					paint: {
						'icon-opacity': 0.92
					}
				});
				map.addLayer({
					id: SELECTED_TRAIL_BOTH_LAYER,
					type: 'symbol',
					source: SELECTED_TRAIL_SOURCE,
					filter: ['==', ['get', 'role'], 'direction-dot'],
					layout: {
						'icon-image': SELECTED_TRAIL_BOTH_IMAGE,
						'icon-size': [
							'interpolate',
							['linear'],
							['zoom'],
							9,
							0.9,
							11,
							1.02,
							13,
							1.16,
							15,
							1.28
						],
						'icon-allow-overlap': true,
						'icon-ignore-placement': true,
						'symbol-placement': 'line',
						'symbol-spacing': 30
					},
					paint: {
						'icon-opacity': 0.9
					}
				});
				map.addLayer({
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
						'#22c55e',
						'end',
						'#ef4444',
						'open_end',
						'#ef4444',
						'branch',
						'#67e8f9',
						'disconnected',
						'#9ca3af',
						'#f8fafc'
					],
					'circle-stroke-color': '#ffffff',
					'circle-stroke-width': 2
				}
			});
		}
	}

	function ensureDirectionArrowImage() {
		if (!map || map.hasImage(SELECTED_TRAIL_ARROW_IMAGE)) return;

		const size = 24;
		const canvas = document.createElement('canvas');
		canvas.width = size;
		canvas.height = size;

		const context = canvas.getContext('2d');
		if (!context) return;

		context.clearRect(0, 0, size, size);
		context.translate(size / 2, size / 2);

		context.beginPath();
		context.moveTo(-4.8, -5.2);
		context.lineTo(3.8, 0);
		context.lineTo(-4.8, 5.2);
		context.lineJoin = 'round';
		context.lineCap = 'round';
		context.strokeStyle = 'rgba(128, 31, 31, 0.38)';
		context.lineWidth = 5.2;
		context.stroke();

		context.beginPath();
		context.moveTo(-4.8, -5.2);
		context.lineTo(3.8, 0);
		context.lineTo(-4.8, 5.2);
		context.strokeStyle = 'rgba(239, 68, 68, 0.96)';
		context.lineWidth = 2.7;
		context.stroke();

		map.addImage(SELECTED_TRAIL_ARROW_IMAGE, context.getImageData(0, 0, size, size), {
			pixelRatio: 2
		});
	}

	function ensureBothDirectionImage() {
		if (!map || map.hasImage(SELECTED_TRAIL_BOTH_IMAGE)) return;

		const size = 20;
		const canvas = document.createElement('canvas');
		canvas.width = size;
		canvas.height = size;

		const context = canvas.getContext('2d');
		if (!context) return;

		context.clearRect(0, 0, size, size);
		context.beginPath();
		context.arc(size / 2, size / 2, 3.8, 0, Math.PI * 2);
		context.fillStyle = 'rgba(239, 68, 68, 0.96)';
		context.fill();
		context.lineWidth = 1.8;
		context.strokeStyle = 'rgba(255, 245, 245, 0.95)';
		context.stroke();

		map.addImage(SELECTED_TRAIL_BOTH_IMAGE, context.getImageData(0, 0, size, size), {
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

		for (const diagnostic of selectedTrail?.stitch_diagnostics ?? []) {
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

		const leftPadding = window.innerWidth < 740 ? 24 : 420;

		map.fitBounds(bounds, {
			padding: { top: 72, right: 72, bottom: 72, left: leftPadding },
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

	function closePanel() {
		selectedTrail = null;
		detailError = null;
		directionMode = 'forward';
		downloadFormat = 'gpx';
		downloadSectionOpen = false;
		downloadBusy = false;
		downloadError = null;
		elevationProfile = null;
		elevationStatus = 'idle';
		elevationError = null;
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
		const panelWidth = selectedTrail || detailError ? Math.min(440, viewportWidth - 24) : 72;
		const chooserOnLeft =
			routeChooserPosition != null &&
			routeChooserPosition.x + Math.min(320, viewportWidth - 32) / 2 < viewportWidth / 2;
		const left = chooserOnLeft ? Math.max(panelWidth, chooserWidth) : panelWidth;
		const right = chooserOnLeft ? 72 : chooserWidth;

		return {
			top: 72,
			right,
			bottom: 72,
			left
		};
	}

	function routeChoicesAtPoint(point: maplibregl.Point) {
		if (!map) return [];

		const rendered = map.queryRenderedFeatures(
			[
				[point.x - 6, point.y - 6],
				[point.x + 6, point.y + 6]
			],
			{ layers: ['trail-lines'] }
		);

		const seen = new Set<string>();
		const choices: RouteChoice[] = [];

		for (const feature of rendered) {
			const properties = feature.properties ?? {};
			const id = feature.id ?? properties.id;
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
		const panelWidth = 320;
		const panelHeight = Math.min(440, 134 + choiceCount * 88);
		const padding = 18;
		const viewportWidth = mapContainer?.clientWidth ?? window.innerWidth;
		const viewportHeight = mapContainer?.clientHeight ?? window.innerHeight;
		const leftPanelInset =
			selectedTrail || detailError ? Math.min(420, viewportWidth - 32) + 28 : padding;
		const minX = Math.min(leftPanelInset, Math.max(padding, viewportWidth - panelWidth - padding));
		const maxX = viewportWidth - panelWidth - padding;
		const minY = padding;
		const maxY = viewportHeight - panelHeight - padding;
		const prefersLeftColumn = !selectedTrail && !detailError && point.x > viewportWidth / 2;
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

	function formatDistanceCompact(distanceM: number | null) {
		if (distanceM == null || distanceM < 0) return '—';
		if (distanceM === 0) return '0 m';
		const km = distanceM / 1000;
		if (km >= 10) return `${km.toFixed(0)} km`;
		if (km >= 1) return `${km.toFixed(1)} km`;
		return `${Math.round(distanceM)} m`;
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
		if (!value) return 'Unrated';

		switch (value) {
			case 'hiking':
				return 'Easy';
			case 'mountain_hiking':
				return 'Moderate';
			case 'demanding_mountain_hiking':
				return 'Challenging';
			case 'alpine_hiking':
				return 'Hard';
			case 'demanding_alpine_hiking':
				return 'Severe';
			case 'difficult_alpine_hiking':
				return 'Expert';
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

	function elevationChartMetrics(profile: ElevationProfile | null) {
		const points = profile?.profile_points ?? [];
		if (points.length === 0) return null;

		const width = 320;
		const height = 180;
		const left = 8;
		const right = 8;
		const top = 10;
		const bottom = 24;
		const maxPos = points[points.length - 1]?.pos_m ?? 0;
		const minEle =
			profile?.min_elevation_m ?? Math.min(...points.map((point) => point.ele_m));
		const maxEle =
			profile?.max_elevation_m ?? Math.max(...points.map((point) => point.ele_m));
		const eleRange = Math.max(1, maxEle - minEle);
		const xScale = (pos: number) =>
			left + (maxPos <= 0 ? 0 : (pos / maxPos) * (width - left - right));
		const yScale = (ele: number) =>
			top + ((maxEle - ele) / eleRange) * (height - top - bottom);
		const line = points
			.map((point, index) => `${index === 0 ? 'M' : 'L'} ${xScale(point.pos_m)} ${yScale(point.ele_m)}`)
			.join(' ');
		const area = `${line} L ${xScale(maxPos)} ${height - bottom} L ${xScale(0)} ${height - bottom} Z`;

		return {
			width,
			height,
			top,
			bottom,
			left,
			right,
			maxPos,
			minEle,
			maxEle,
			line,
			area,
			startEle: points[0]?.ele_m ?? null,
			endEle: points[points.length - 1]?.ele_m ?? null
		};
	}

	function profileDistanceLabels(metrics: ReturnType<typeof elevationChartMetrics>) {
		if (!metrics) return [];
		const ticks = [0, 0.5, 1];
		return ticks.map((fraction) => {
			const pos = metrics.left + fraction * (metrics.width - metrics.left - metrics.right);
			const distance = metrics.maxPos * fraction;
			return { pos, label: formatDistanceCompact(distance) };
		});
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
	): Array<{ role: 'direction-chevron' | 'direction-dot'; geometry: TrailGeometry }> {
		switch (mode) {
			case 'forward':
				return [{ role: 'direction-chevron', geometry }];
			case 'backward':
				return [{ role: 'direction-chevron', geometry: reverseGeometry(geometry) }];
			case 'both':
				return [{ role: 'direction-dot', geometry }];
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
		terrainPanelOpen = !terrainPanelOpen;
	}

	function toggleTerrain3d() {
		terrain3dEnabled = !terrain3dEnabled;
		if (terrain3dEnabled && map && map.getPitch() < 45) {
			map.easeTo({ pitch: 55, duration: 700 });
		}
	}

	function setTerrainContoursEnabled(enabled: boolean) {
		terrainContoursVisible = enabled;
	}

	function setTerrainHillshadeEnabled(enabled: boolean) {
		terrainHillshadeVisible = enabled;
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

	async function downloadRoute() {
		if (!selectedTrail) return;

		downloadBusy = true;
		downloadError = null;

		try {
			const params = new URLSearchParams({
				format: downloadFormat,
				direction: directionMode
			});
			const response = await fetch(`/api/trails/${selectedTrail.id}/download?${params.toString()}`);
			if (!response.ok) {
				throw new Error(`Download request failed with ${response.status}`);
			}

			const blob = await response.blob();
			const objectUrl = URL.createObjectURL(blob);
			const link = document.createElement('a');
			link.href = objectUrl;
			link.download =
				downloadFilenameFromHeader(response.headers.get('content-disposition')) ??
				`${selectedTrail.id}.${downloadFormat}`;
			document.body.appendChild(link);
			link.click();
			link.remove();
			URL.revokeObjectURL(objectUrl);
		} catch (error) {
			downloadError = error instanceof Error ? error.message : 'Failed to download route';
		} finally {
			downloadBusy = false;
		}
	}
</script>

<div class="map-shell">
	<div bind:this={mapContainer} class="map"></div>

	<div class="terrain-dock">
		<button
			class:open={terrainPanelOpen}
			class="terrain-dock-toggle"
			type="button"
			aria-expanded={terrainPanelOpen}
			onclick={toggleTerrainPanel}
		>
			Terrain
		</button>

		{#if terrainPanelOpen}
			<div class="terrain-card">
				<div class="terrain-card-header">
					<div>
						<h3>Terrain view</h3>
						<p>Hillshade, 3D relief, and client-side contours from one DEM source.</p>
					</div>
				</div>

				<div class="terrain-control-list">
					<label class="terrain-switch">
						<div>
							<strong>3D terrain</strong>
							<p>Lift the map surface from the DEM. Best seen when the camera tilts.</p>
						</div>
						<input
							type="checkbox"
							checked={terrain3dEnabled}
							disabled={!terrainReady}
							onchange={toggleTerrain3d}
						/>
					</label>

					<label class="terrain-switch">
						<div>
							<strong>Hillshade</strong>
							<p>Add shaded relief under the trails for clearer slopes and ridgelines.</p>
						</div>
						<input
							type="checkbox"
							checked={terrainHillshadeVisible}
							disabled={!terrainReady}
							onchange={(event) =>
								setTerrainHillshadeEnabled((event.currentTarget as HTMLInputElement).checked)}
						/>
					</label>

					<label class="terrain-switch">
						<div>
							<strong>Contours</strong>
							<p>Generate contour lines in the browser from the same Terrain-RGB tiles.</p>
						</div>
						<input
							type="checkbox"
							checked={terrainContoursVisible}
							disabled={!terrainReady}
							onchange={(event) =>
								setTerrainContoursEnabled((event.currentTarget as HTMLInputElement).checked)}
						/>
					</label>
				</div>

				<div class="terrain-fieldset">
					<span class="terrain-fieldset-label">Contour unit</span>
					<div class="terrain-segmented">
						<label>
							<input
								type="radio"
								name="terrain-unit"
								checked={terrainContourUnit === 'meters'}
								disabled={!terrainReady}
								onchange={() => setTerrainContourUnit('meters')}
							/>
							<span>meters</span>
						</label>
						<label>
							<input
								type="radio"
								name="terrain-unit"
								checked={terrainContourUnit === 'feet'}
								disabled={!terrainReady}
								onchange={() => setTerrainContourUnit('feet')}
							/>
							<span>feet</span>
						</label>
					</div>
				</div>

				<div class="terrain-fieldset">
					<div class="terrain-slider-row">
						<span class="terrain-fieldset-label">Terrain exaggeration</span>
						<strong>{terrainExaggeration.toFixed(1)}×</strong>
					</div>
					<input
						class="terrain-slider"
						type="range"
						min="1"
						max="2.4"
						step="0.1"
						bind:value={terrainExaggeration}
						disabled={!terrainReady}
					/>
				</div>

				{#if !terrainReady}
					<p class="terrain-status">Loading terrain controls…</p>
				{:else if !terrainColorReliefSupported}
					<p class="terrain-status">
						Elevation tint is not enabled yet because the installed MapLibre version does not
						support `color-relief` layers.
					</p>
				{/if}
			</div>
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

	{#if selectedTrail || detailError}
		<aside class="trail-panel">
			<button class="close" onclick={closePanel} aria-label="Close trail panel">×</button>

			{#if detailError}
				<p class="error">{detailError}</p>
			{:else if selectedTrail}
				{#if selectedTrail.region_breadcrumb.length > 0}
					<p class="breadcrumb">{selectedTrail.region_breadcrumb.join(' › ')}</p>
				{/if}

				<div class="hero">
					<p class="eyebrow">
						{difficultyLabel(selectedTrail.difficulty)}
						{#if selectedTrail.network}
							<span>• {selectedTrail.network}</span>
						{/if}
					</p>
					<h2>{selectedTrail.name ?? 'Unnamed trail'}</h2>
					{#if selectedTrail.summary}
						<p class="summary">{selectedTrail.summary}</p>
					{/if}
				</div>

				<div class="stats">
					<div>
						<span>Length</span>
						<strong>{formatDistance(selectedTrail.distance_m)}</strong>
					</div>
					<div>
						<span>Estimated time</span>
						<strong>{formatDuration(selectedTrail.estimated_duration_min)}</strong>
					</div>
					<div>
						<span>Elevation gain</span>
						<strong>{formatElevation(selectedTrail.ascent_m)}</strong>
					</div>
					<div>
						<span>Hike type</span>
						<strong>{hikeTypeLabel(selectedTrail.hike_type)}</strong>
					</div>
				</div>

				<section class="section">
					<h3>Elevation profile</h3>
					{#if elevationStatus === 'loading'}
						<div class="meta-card">
							<strong>Loading profile…</strong>
							<p>Fetching elevation samples for this route.</p>
						</div>
					{:else if elevationStatus === 'ready' && elevationProfile}
						{@const metrics = elevationChartMetrics(elevationProfile)}
						{#if metrics}
							<div class="profile-shell">
								<div class="profile-chart">
									<svg viewBox={`0 0 ${metrics.width} ${metrics.height}`} aria-label="Elevation profile chart">
										<defs>
											<linearGradient id="elevation-fill" x1="0" x2="0" y1="0" y2="1">
												<stop offset="0%" stop-color="rgba(223, 255, 42, 0.38)" />
												<stop offset="100%" stop-color="rgba(223, 255, 42, 0.04)" />
											</linearGradient>
										</defs>
										<line x1={metrics.left} y1={metrics.top} x2={metrics.left} y2={metrics.height - metrics.bottom} class="profile-axis" />
										<line x1={metrics.left} y1={metrics.height - metrics.bottom} x2={metrics.width - metrics.right} y2={metrics.height - metrics.bottom} class="profile-axis" />
										<line x1={metrics.left} y1={metrics.top} x2={metrics.width - metrics.right} y2={metrics.top} class="profile-grid" />
										<line x1={metrics.left} y1={(metrics.top + (metrics.height - metrics.bottom)) / 2} x2={metrics.width - metrics.right} y2={(metrics.top + (metrics.height - metrics.bottom)) / 2} class="profile-grid" />
										<line x1={metrics.left} y1={metrics.height - metrics.bottom} x2={metrics.width - metrics.right} y2={metrics.height - metrics.bottom} class="profile-grid" />
										<path d={metrics.area} class="profile-area" />
										<path d={metrics.line} class="profile-line" />
										{#each profileDistanceLabels(metrics) as tick}
											<text x={tick.pos} y={metrics.height - 6} class="profile-label" text-anchor="middle">
												{tick.label}
											</text>
										{/each}
										<text x={metrics.left} y={metrics.top - 1} class="profile-label" text-anchor="start">
											{formatElevation(metrics.maxEle)}
										</text>
										<text x={metrics.left} y={metrics.height - metrics.bottom - 4} class="profile-label" text-anchor="start">
											{formatElevation(metrics.minEle)}
										</text>
									</svg>
								</div>
								<div class="meta-grid profile-meta-grid">
									<div class="meta-card">
										<span>Start</span>
										<strong>{formatElevation(metrics.startEle)}</strong>
									</div>
									<div class="meta-card">
										<span>Peak</span>
										<strong>{formatElevation(metrics.maxEle)}</strong>
									</div>
									<div class="meta-card">
										<span>Finish</span>
										<strong>{formatElevation(metrics.endEle)}</strong>
									</div>
									<div class="meta-card">
										<span>Profile length</span>
										<strong>{formatDistanceCompact(metrics.maxPos)}</strong>
									</div>
								</div>
							</div>
						{/if}
					{:else if elevationStatus === 'missing'}
						<div class="meta-card">
							<strong>No elevation profile yet</strong>
							<p>This trail does not have generated elevation samples yet. Re-run the importer with `DEM_PATH` configured to populate this route.</p>
						</div>
					{:else if elevationStatus === 'error'}
						<div class="meta-card">
							<strong>Couldn’t load profile</strong>
							<p>{elevationError ?? 'Something went wrong while loading the elevation profile.'}</p>
						</div>
					{/if}
				</section>

				{#if selectedTrail.features.length > 0}
					<section class="section">
						<h3>What to expect</h3>
						<div class="chips">
							{#each selectedTrail.features as feature}
								<span class="chip">{featureLabel(feature)}</span>
							{/each}
						</div>
					</section>
				{/if}

				{#if selectedTrail.route_from || selectedTrail.route_to || selectedTrail.route_via || selectedTrail.route_tag || selectedTrail.network_type || selectedTrail.website_url || selectedTrail.video_url || selectedTrail.direction_source}
					<section class="section">
						<h3>Route details</h3>
						<div class="meta-grid">
							{#if selectedTrail.route_from || selectedTrail.route_to}
								<div class="meta-card meta-card-wide">
									<span>Route path</span>
									<strong>{directionLabel('forward', selectedTrail)}</strong>
								</div>
							{/if}
							{#if selectedTrail.route_tag}
								<div class="meta-card">
									<span>Route tag</span>
									<strong>{selectedTrail.route_tag}</strong>
								</div>
							{/if}
							{#if selectedTrail.network_type}
								<div class="meta-card">
									<span>Network type</span>
									<strong>{selectedTrail.network_type}</strong>
								</div>
							{/if}
							{#if selectedTrail.direction_source}
								<div class="meta-card">
									<span>Direction basis</span>
									<strong>{directionSourceLabel(selectedTrail.direction_source)}</strong>
								</div>
							{/if}
							{#if normalizeExternalUrl(selectedTrail.website_url)}
								<div class="meta-card meta-card-wide">
									<span>Website</span>
									<a class="meta-link" href={normalizeExternalUrl(selectedTrail.website_url)} target="_blank" rel="noreferrer">
										{normalizeExternalUrl(selectedTrail.website_url)}
									</a>
								</div>
							{/if}
							{#if normalizeExternalUrl(selectedTrail.video_url)}
								<div class="meta-card meta-card-wide">
									<span>Video</span>
									<a class="meta-link" href={normalizeExternalUrl(selectedTrail.video_url)} target="_blank" rel="noreferrer">
										{normalizeExternalUrl(selectedTrail.video_url)}
									</a>
								</div>
							{/if}
						</div>
					</section>
				{/if}

				<section class="section download-section">
					<button
						class="section-toggle"
						type="button"
						aria-expanded={downloadSectionOpen}
						onclick={toggleDownloadSection}
					>
						<span class="section-title">Download route</span>
						<span class:open={downloadSectionOpen}>⌄</span>
					</button>

					{#if downloadSectionOpen}
						<p class="analysis-note">
							Choose an export format and travel direction. The red arrows on the map update to preview the selected direction.
						</p>

						<div class="download-fieldset">
							<span class="fieldset-label">Format</span>
							<div class="radio-list format-list">
								<label>
									<input type="radio" bind:group={downloadFormat} value="gpx" />
									<span>GPX</span>
								</label>
								<label>
									<input type="radio" bind:group={downloadFormat} value="kml" />
									<span>KML</span>
								</label>
								<label>
									<input type="radio" bind:group={downloadFormat} value="geojson" />
									<span>GeoJSON</span>
								</label>
							</div>
						</div>

						<div class="download-fieldset">
							<span class="fieldset-label">Include ways in direction</span>
							<div class="radio-list direction-list">
								<label>
									<input
										type="radio"
										name="direction"
										checked={directionMode === 'forward'}
										onchange={() => setDirection('forward')}
									/>
									<div>
										<strong>forwards</strong>
										<p>{directionLabel('forward', selectedTrail)}</p>
									</div>
								</label>
								<label>
									<input
										type="radio"
										name="direction"
										checked={directionMode === 'backward'}
										onchange={() => setDirection('backward')}
									/>
									<div>
										<strong>backwards</strong>
										<p>{directionLabel('backward', selectedTrail)}</p>
									</div>
								</label>
								<label>
									<input
										type="radio"
										name="direction"
										checked={directionMode === 'both'}
										onchange={() => setDirection('both')}
									/>
									<div>
										<strong>both</strong>
										<p>{directionLabel('both', selectedTrail)}</p>
									</div>
								</label>
							</div>
						</div>

						<button class="download-button" type="button" disabled={downloadBusy} onclick={downloadRoute}>
							{downloadBusy ? 'Preparing file…' : 'Get file'}
						</button>

						{#if downloadError}
							<p class="download-error">{downloadError}</p>
						{/if}
					{/if}
				</section>

				{#if selectedTrail.start_point || selectedTrail.end_point || selectedTrail.stitch_warnings.length > 0}
					<section class="section">
						<h3>Analyze relation</h3>
						<p class="analysis-note">
							This route was reconstructed from OSM relation members. Markers and warnings highlight places where the computed path may need review.
						</p>
						<div class="analysis-list">
							{#if selectedTrail.start_point}
								<div class="analysis-item">
									<span class="dot start-dot"></span>
									<div>
										<strong>Start point</strong>
										<p>Green marker on the map. Open Download route to preview red direction arrows from here.</p>
									</div>
								</div>
							{/if}
							{#if selectedTrail.end_point}
								<div class="analysis-item">
									<span class="dot end-dot"></span>
									<div>
										<strong>Finish point</strong>
										<p>Red marker on the map marks the end of the currently stitched route.</p>
									</div>
								</div>
							{/if}
							{#if diagnosticCount('branch') > 0}
								<div class="analysis-item">
									<span class="dot branch-dot"></span>
									<div>
										<strong>{diagnosticCount('branch')} branch point{diagnosticCount('branch') === 1 ? '' : 's'}</strong>
										<p>Blue markers show junctions where more than two route segments meet.</p>
									</div>
								</div>
							{/if}
							{#if diagnosticCount('open_end') > 0}
								<div class="analysis-item">
									<span class="dot warning-dot"></span>
									<div>
										<strong>{diagnosticCount('open_end')} open end{diagnosticCount('open_end') === 1 ? '' : 's'}</strong>
										<p>These points mark route ends where the computed path does not continue.</p>
									</div>
								</div>
							{/if}
							{#if diagnosticCount('disconnected') > 0}
								<div class="analysis-item">
									<span class="dot disconnected-dot"></span>
									<div>
										<strong>{diagnosticCount('disconnected')} disconnected point{diagnosticCount('disconnected') === 1 ? '' : 's'}</strong>
										<p>Gray markers show broken-off pieces that were not part of the main stitched path.</p>
									</div>
								</div>
							{/if}
							{#each warningSummaries(selectedTrail.stitch_warnings) as summary}
								<div class="analysis-item warning-item">
									<span class="dot warning-dot"></span>
									<div>
										<strong>
											{summary.title}
											{#if summary.count > 1}
												<span class="count-pill">{summary.count}</span>
											{/if}
										</strong>
										<p>{summary.description}</p>
									</div>
								</div>
							{/each}
						</div>
					</section>
				{/if}

				{#if selectedTrail.surfaces.length > 0}
					<section class="section">
						<h3>Surfaces</h3>
						<div class="surface-bar">
							{#each selectedTrail.surfaces as surface}
								<div
									class="surface-segment"
									style={`width:${surface.pct}%;background:${surfaceColor(surface.surface_type)}`}
								></div>
							{/each}
						</div>
						<div class="surface-list">
							{#each selectedTrail.surfaces as surface}
								<div class="surface-item">
									<span class="dot" style={`background:${surfaceColor(surface.surface_type)}`}></span>
									<div>
										<strong>{surfaceLabel(surface.surface_type)}</strong>
										<p>{surface.pct.toFixed(0)}% · {formatDistance(surface.distance_m)}</p>
									</div>
								</div>
							{/each}
						</div>
					</section>
				{/if}
			{/if}
		</aside>
	{/if}
</div>

<style>
	:global(.maplibregl-popup-content) {
		font-family: inherit;
	}

	.map-shell {
		position: relative;
		width: 100%;
		height: 100%;
		overflow: hidden;
		background:
			radial-gradient(circle at top left, rgba(49, 80, 84, 0.22), transparent 28%),
			radial-gradient(circle at bottom right, rgba(177, 122, 69, 0.18), transparent 30%),
			#10141a;
	}

	.map {
		width: 100%;
		height: 100%;
	}

	.terrain-dock {
		position: absolute;
		top: 4.5rem;
		right: 1rem;
		z-index: 3;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 0.7rem;
	}

	.terrain-dock-toggle {
		padding: 0.72rem 1rem;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 999px;
		background: rgba(17, 22, 30, 0.88);
		backdrop-filter: blur(12px);
		color: #eef1f3;
		font-size: 0.92rem;
		font-weight: 700;
		cursor: pointer;
		box-shadow: 0 16px 30px rgba(0, 0, 0, 0.28);
		transition:
			transform 120ms ease,
			border-color 120ms ease,
			background 120ms ease;
	}

	.terrain-dock-toggle:hover,
	.terrain-dock-toggle.open {
		transform: translateY(-1px);
		border-color: rgba(223, 255, 42, 0.45);
		background: rgba(24, 31, 40, 0.94);
	}

	.terrain-card {
		width: min(340px, calc(100vw - 2rem));
		padding: 1rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 20px;
		background: rgba(17, 22, 30, 0.92);
		backdrop-filter: blur(14px);
		box-shadow: 0 22px 44px rgba(0, 0, 0, 0.34);
		color: #eef1f3;
	}

	.terrain-card-header h3 {
		margin: 0;
		font-size: 1rem;
	}

	.terrain-card-header p {
		margin: 0.35rem 0 0;
		color: #aeb6c1;
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
		color: #9ca8b6;
		font-size: 0.84rem;
		line-height: 1.4;
	}

	.terrain-switch input[type='checkbox'] {
		margin-top: 0.2rem;
		width: 1rem;
		height: 1rem;
		accent-color: #dfff2a;
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
		color: #9ca8b6;
	}

	.terrain-slider-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
	}

	.terrain-slider-row strong {
		color: #dfff2a;
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
		accent-color: #dfff2a;
	}

	.terrain-slider {
		width: 100%;
		margin-top: 0.8rem;
		accent-color: #dfff2a;
	}

	.terrain-status {
		margin: 0.85rem 0 0;
		color: #9ca8b6;
		font-size: 0.82rem;
		line-height: 1.45;
	}

	.route-chooser {
		position: absolute;
		z-index: 4;
		width: min(320px, calc(100vw - 2rem));
		padding: 0.9rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 18px;
		background: rgba(18, 24, 32, 0.94);
		backdrop-filter: blur(12px);
		box-shadow: 0 20px 42px rgba(0, 0, 0, 0.34);
		color: #eef1f3;
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
		color: #eef1f3;
		font-size: 1.1rem;
		cursor: pointer;
	}

	.route-chooser-note {
		margin: 0.55rem 0 0.85rem;
		color: #aeb6c1;
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
		border-color: rgba(223, 255, 42, 0.55);
		background: rgba(223, 255, 42, 0.08);
		transform: translateY(-1px);
	}

	.route-choice.preview {
		border-color: rgba(223, 255, 42, 0.55);
		background: rgba(223, 255, 42, 0.08);
		transform: translateY(-1px);
	}

	.route-choice p {
		margin: 0.18rem 0 0;
		color: #9ca8b6;
		font-size: 0.86rem;
	}

	.route-tag {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 2.25rem;
		padding: 0.15rem 0.45rem;
		border: 2px solid #ffb000;
		background: #fffaf0;
		color: #262626;
		font-size: 0.78rem;
		font-weight: 700;
		line-height: 1.1;
	}

	.trail-panel {
		position: absolute;
		top: 1rem;
		left: 1rem;
		bottom: 1rem;
		width: min(420px, calc(100vw - 2rem));
		overflow: auto;
		padding: 1.25rem 1.25rem 1.5rem;
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 22px;
		background: rgba(17, 22, 30, 0.9);
		backdrop-filter: blur(12px);
		box-shadow: 0 24px 60px rgba(0, 0, 0, 0.35);
		color: #eef1f3;
		font-family:
			'ui-sans-serif',
			'system-ui',
			sans-serif;
	}

	.close {
		position: sticky;
		top: 0;
		float: right;
		width: 2rem;
		height: 2rem;
		border: none;
		border-radius: 999px;
		background: rgba(255, 255, 255, 0.08);
		color: #eef1f3;
		font-size: 1.2rem;
		cursor: pointer;
	}

	.error {
		margin-top: 2.75rem;
		color: #ffb4ab;
	}

	.breadcrumb {
		margin: 2.75rem 0 0.75rem;
		font-size: 0.82rem;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		color: #97a2b0;
	}

	.hero h2 {
		margin: 0;
		font-size: clamp(1.7rem, 4vw, 2.5rem);
		line-height: 1.05;
	}

	.eyebrow {
		margin: 0 0 0.6rem;
		font-size: 0.96rem;
		font-weight: 700;
		color: #ffd54f;
	}

	.eyebrow span {
		color: #aeb6c1;
		font-weight: 500;
	}

	.summary {
		margin: 0.8rem 0 0;
		color: #cfd6de;
		line-height: 1.55;
	}

	.stats {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.85rem;
		margin-top: 1.35rem;
	}

	.stats div {
		padding: 0.95rem 1rem;
		border-radius: 16px;
		background: rgba(255, 255, 255, 0.05);
	}

	.stats span {
		display: block;
		margin-bottom: 0.35rem;
		font-size: 0.82rem;
		color: #9ca8b6;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.stats strong {
		font-size: 1.05rem;
	}

	.section {
		margin-top: 1.45rem;
	}

	.section h3 {
		margin: 0 0 0.75rem;
		font-size: 1rem;
	}

	.meta-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.75rem;
	}

	.meta-card {
		padding: 0.95rem 1rem;
		border-radius: 16px;
		background: rgba(255, 255, 255, 0.05);
	}

	.meta-card-wide {
		grid-column: 1 / -1;
	}

	.meta-card span {
		display: block;
		margin-bottom: 0.38rem;
		font-size: 0.8rem;
		color: #9ca8b6;
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
		color: #9ca8b6;
		font-size: 0.88rem;
		line-height: 1.45;
	}

	.meta-link {
		color: #dfff2a;
		word-break: break-word;
		text-decoration: none;
	}

	.meta-link:hover {
		text-decoration: underline;
	}

	.profile-shell {
		display: grid;
		gap: 0.9rem;
	}

	.profile-chart {
		padding: 0.7rem 0.7rem 0.35rem;
		border-radius: 18px;
		background:
			linear-gradient(180deg, rgba(223, 255, 42, 0.05), rgba(255, 255, 255, 0.02)),
			rgba(255, 255, 255, 0.05);
	}

	.profile-chart svg {
		display: block;
		width: 100%;
		height: auto;
		overflow: visible;
	}

	.profile-axis {
		stroke: rgba(255, 255, 255, 0.14);
		stroke-width: 1;
	}

	.profile-grid {
		stroke: rgba(255, 255, 255, 0.08);
		stroke-width: 1;
		stroke-dasharray: 3 4;
	}

	.profile-area {
		fill: url(#elevation-fill);
	}

	.profile-line {
		fill: none;
		stroke: #dfff2a;
		stroke-width: 2.5;
		stroke-linejoin: round;
		stroke-linecap: round;
	}

	.profile-label {
		fill: #9ca8b6;
		font-size: 10px;
		font-family: ui-sans-serif, system-ui, sans-serif;
	}

	.profile-meta-grid {
		grid-template-columns: repeat(4, minmax(0, 1fr));
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
		color: #dfff2a;
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
		color: #9ca8b6;
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
		accent-color: #dfff2a;
	}

	.direction-list strong {
		display: block;
		font-size: 0.96rem;
	}

	.direction-list p {
		margin: 0.2rem 0 0;
		color: #9ca8b6;
		font-size: 0.86rem;
		line-height: 1.35;
	}

	.download-button {
		width: 100%;
		margin-top: 0.95rem;
		padding: 0.92rem 1rem;
		border: none;
		border-radius: 16px;
		background: #dfff2a;
		color: #11161e;
		font-size: 1rem;
		font-weight: 800;
		cursor: pointer;
		transition:
			transform 120ms ease,
			filter 120ms ease;
	}

	.download-button:hover:not(:disabled) {
		transform: translateY(-1px);
		filter: brightness(1.03);
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
		color: #aeb6c1;
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
		color: #9ca8b6;
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
		background: #22c55e;
	}

	.end-dot,
	.warning-dot {
		background: #ef4444;
	}

	.branch-dot {
		background: #67e8f9;
	}

	.disconnected-dot {
		background: #9ca3af;
	}

	@media (max-width: 740px) {
		.terrain-dock {
			top: 0.75rem;
			right: 0.75rem;
			left: 0.75rem;
			align-items: stretch;
		}

		.terrain-card {
			width: 100%;
		}

		.trail-panel {
			top: auto;
			right: 0.75rem;
			left: 0.75rem;
			bottom: 0.75rem;
			width: auto;
			max-height: 58vh;
		}

		.stats,
		.meta-grid,
		.profile-meta-grid,
		.surface-list,
		.format-list {
			grid-template-columns: 1fr;
		}
	}
</style>
