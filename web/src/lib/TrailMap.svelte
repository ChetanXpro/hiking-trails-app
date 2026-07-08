<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import maplibregl from 'maplibre-gl';
	import 'maplibre-gl/dist/maplibre-gl.css';

	const INITIAL_CENTER: [number, number] = [77.18, 32.24];
	const INITIAL_ZOOM = 9;

	const MARTIN_TRAILS_URL = 'http://localhost:3000/trails/{z}/{x}/{y}';

	const EMPTY_FEATURE_COLLECTION: GeoJSON.FeatureCollection = {
		type: 'FeatureCollection',
		features: []
	};

	let mapContainer: HTMLDivElement;
	let map: maplibregl.Map | undefined;
	let selectedTrail: Record<string, any> | null = $state(null);
	let selectedWaypoint: { name: string; type: string } | null = $state(null);

	const WAYPOINT_LABELS: Record<string, string> = {
		alpine_hut: 'Alpine hut',
		wilderness_hut: 'Wilderness hut',
		camp_site: 'Campsite',
		peak: 'Peak',
		spring: 'Spring'
	};

	const NETWORK_LABELS: Record<string, string> = {
		lwn: 'Local network',
		rwn: 'Regional network',
		nwn: 'National network',
		iwn: 'International network'
	};

	const WAYPOINT_GROUPS: { key: string; label: string; types: string[] }[] = [
		{ key: 'huts', label: 'Huts', types: ['alpine_hut', 'wilderness_hut'] },
		{ key: 'camp_site', label: 'Campsites', types: ['camp_site'] },
		{ key: 'peak', label: 'Peaks', types: ['peak'] },
		{ key: 'spring', label: 'Springs', types: ['spring'] }
	];

	let enabledGroups = $state<Set<string>>(new Set());

	function applyWaypointFilter() {
		if (!map || !map.getLayer('waypoint-points')) return;
		const enabledTypes = WAYPOINT_GROUPS.filter((g) => enabledGroups.has(g.key)).flatMap(
			(g) => g.types
		);
		map.setFilter('waypoint-points', ['in', ['get', 'type'], ['literal', enabledTypes]]);
	}

	function toggleGroup(key: string) {
		const next = new Set(enabledGroups);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		enabledGroups = next;
		applyWaypointFilter();
	}

	function clearSelection() {
		selectedTrail = null;
		(map?.getSource('selected-trail') as maplibregl.GeoJSONSource | undefined)?.setData(
			EMPTY_FEATURE_COLLECTION
		);
		(map?.getSource('diagnostic-points') as maplibregl.GeoJSONSource | undefined)?.setData(
			EMPTY_FEATURE_COLLECTION
		);
	}

	function createArrowIcon(): ImageData {
		const size = 24;
		const canvas = document.createElement('canvas');
		canvas.width = size;
		canvas.height = size;
		const ctx = canvas.getContext('2d')!;
		ctx.fillStyle = '#F6F5F1';
		ctx.beginPath();
		ctx.moveTo(size * 0.18, size * 0.22);
		ctx.lineTo(size * 0.82, size * 0.5);
		ctx.lineTo(size * 0.18, size * 0.78);
		ctx.closePath();
		ctx.fill();
		return ctx.getImageData(0, 0, size, size);
	}

	type Glyph = 'hut' | 'tent' | 'peak' | 'spring' | 'dot';

	function createWaypointIcon(glyph: Glyph, color: string): ImageData {
		const size = 28;
		const canvas = document.createElement('canvas');
		canvas.width = size;
		canvas.height = size;
		const ctx = canvas.getContext('2d')!;
		const c = size / 2;

		ctx.beginPath();
		ctx.arc(c, c, c - 1.5, 0, Math.PI * 2);
		ctx.fillStyle = '#F6F5F1';
		ctx.fill();
		ctx.lineWidth = 1.5;
		ctx.strokeStyle = color;
		ctx.stroke();

		ctx.fillStyle = color;

		switch (glyph) {
			case 'hut':
				ctx.beginPath();
				ctx.moveTo(c - 7, c + 1);
				ctx.lineTo(c, c - 7);
				ctx.lineTo(c + 7, c + 1);
				ctx.closePath();
				ctx.fill();
				ctx.fillRect(c - 5, c + 1, 10, 6);
				break;
			case 'tent':
				ctx.beginPath();
				ctx.moveTo(c, c - 7);
				ctx.lineTo(c + 8, c + 7);
				ctx.lineTo(c - 8, c + 7);
				ctx.closePath();
				ctx.fill();
				ctx.beginPath();
				ctx.moveTo(c, c - 7);
				ctx.lineTo(c, c + 7);
				ctx.strokeStyle = '#F6F5F1';
				ctx.lineWidth = 1;
				ctx.stroke();
				break;
			case 'peak':
				ctx.beginPath();
				ctx.moveTo(c - 8, c + 7);
				ctx.lineTo(c, c - 8);
				ctx.lineTo(c + 8, c + 7);
				ctx.closePath();
				ctx.fill();
				ctx.beginPath();
				ctx.moveTo(c - 2.5, c - 3);
				ctx.lineTo(c, c - 8);
				ctx.lineTo(c + 2.5, c - 3);
				ctx.lineTo(c + 1, c - 2);
				ctx.lineTo(c, c - 4);
				ctx.lineTo(c - 1, c - 2);
				ctx.closePath();
				ctx.fillStyle = '#F6F5F1';
				ctx.fill();
				break;
			case 'spring':
				ctx.beginPath();
				ctx.moveTo(c, c - 8);
				ctx.quadraticCurveTo(c + 7, c + 3, c, c + 8);
				ctx.quadraticCurveTo(c - 7, c + 3, c, c - 8);
				ctx.closePath();
				ctx.fill();
				break;
			case 'dot':
				ctx.beginPath();
				ctx.arc(c, c, 4, 0, Math.PI * 2);
				ctx.fill();
				break;
		}

		return ctx.getImageData(0, 0, size, size);
	}

	onMount(() => {
		map = new maplibregl.Map({
			container: mapContainer,
			style: {
				version: 8,
				sources: {
					osm: {
						type: 'raster',
						tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
						tileSize: 256,
						attribution: '&copy; OpenStreetMap contributors'
					},
					trails: {
						type: 'vector',
						tiles: [MARTIN_TRAILS_URL],
						minzoom: 0,
						maxzoom: 14
					},
					waypoints: {
						type: 'geojson',
						data: '/api/waypoints'
					}
				},
				layers: [
					{ id: 'osm', type: 'raster', source: 'osm', paint: { 'raster-saturation': -0.5, 'raster-contrast': -0.1 } },
					{
						id: 'trail-lines-casing',
						type: 'line',
						source: 'trails',
						'source-layer': 'trails',
						paint: {
							'line-color': '#F6F5F1',
							'line-width': 5
						}
					},
					{
						id: 'trail-lines',
						type: 'line',
						source: 'trails',
						'source-layer': 'trails',
						paint: {
							'line-color': [
								'match',
								['get', 'network'],
								'lwn', '#E0762C',
								'rwn', '#3B4FA0',
								'nwn', '#C9A227',
								'iwn', '#9C3F8C',
								'#2B2E33'
							],
							'line-width': 2.5
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

			map.addSource('selected-trail', { type: 'geojson', data: EMPTY_FEATURE_COLLECTION });
			map.addSource('diagnostic-points', { type: 'geojson', data: EMPTY_FEATURE_COLLECTION });

			map.addImage('waypoint-hut', createWaypointIcon('hut', '#7A5C3E'));
			map.addImage('waypoint-tent', createWaypointIcon('tent', '#7C8A54'));
			map.addImage('waypoint-peak', createWaypointIcon('peak', '#2B2E33'));
			map.addImage('waypoint-spring', createWaypointIcon('spring', '#5D7E8C'));
			map.addImage('waypoint-default', createWaypointIcon('dot', '#8C9296'));

			map.addLayer({
				id: 'waypoint-points',
				type: 'symbol',
				source: 'waypoints',
				layout: {
					'icon-image': [
						'match',
						['get', 'type'],
						'alpine_hut', 'waypoint-hut',
						'wilderness_hut', 'waypoint-hut',
						'camp_site', 'waypoint-tent',
						'peak', 'waypoint-peak',
						'spring', 'waypoint-spring',
						'waypoint-default'
					],
					'icon-size': 0.85,
					'icon-allow-overlap': true
				}
			});
			applyWaypointFilter();

			map.addLayer(
				{
					id: 'selected-trail-halo',
					type: 'line',
					source: 'selected-trail',
					paint: {
						'line-color': '#B23A32',
						'line-width': 14,
						'line-opacity': 0.22,
						'line-blur': 4
					}
				},
				'waypoint-points'
			);
			map.addLayer(
				{
					id: 'selected-trail-line',
					type: 'line',
					source: 'selected-trail',
					paint: {
						'line-color': '#B23A32',
						'line-width': 4.5
					}
				},
				'waypoint-points'
			);

			map.addImage('trail-arrow', createArrowIcon());

			map.addLayer({
				id: 'selected-trail-arrows',
				type: 'symbol',
				source: 'selected-trail',
				layout: {
					'symbol-placement': 'line',
					'symbol-spacing': 90,
					'icon-image': 'trail-arrow',
					'icon-size': 0.85,
					'icon-rotation-alignment': 'map',
					'icon-allow-overlap': true
				}
			});

			map.addLayer({
				id: 'diagnostic-point-markers',
				type: 'circle',
				source: 'diagnostic-points',
				paint: {
					'circle-radius': 6,
					'circle-color': [
						'match',
						['get', 'kind'],
						'open_end', '#B23A32',
						'branch', '#3B4FA0',
						'disconnected', '#8C9296',
						'#8C9296'
					],
					'circle-stroke-width': 2,
					'circle-stroke-color': '#F6F5F1'
				}
			});
		});

		map.on('click', 'trail-lines', async (e) => {
			const trailId = e.features?.[0]?.properties?.id;
			if (!trailId) return;

			const res = await fetch(`/api/trails/${trailId}`);
			if (!res.ok) return;

			const trail = await res.json();
			selectedTrail = trail;

			if (trail.geometry) {
				(map?.getSource('selected-trail') as maplibregl.GeoJSONSource | undefined)?.setData({
					type: 'Feature',
					geometry: trail.geometry,
					properties: {}
				});
			}

			(map?.getSource('diagnostic-points') as maplibregl.GeoJSONSource | undefined)?.setData(
				trail.diagnostic_points ?? EMPTY_FEATURE_COLLECTION
			);
		});

		map.on('mouseenter', 'trail-lines', () => {
			if (map) map.getCanvas().style.cursor = 'pointer';
		});
		map.on('mouseleave', 'trail-lines', () => {
			if (map) map.getCanvas().style.cursor = '';
		});

		map.on('click', 'waypoint-points', (e) => {
			const props = e.features?.[0]?.properties;
			if (!props) return;
			selectedWaypoint = {
				name: (props.name as string) || WAYPOINT_LABELS[props.type as string] || 'Waypoint',
				type: (props.type as string) || 'unknown'
			};
		});

		map.on('mouseenter', 'waypoint-points', () => {
			if (map) map.getCanvas().style.cursor = 'pointer';
		});
		map.on('mouseleave', 'waypoint-points', () => {
			if (map) map.getCanvas().style.cursor = '';
		});
	});

	onDestroy(() => {
		map?.remove();
	});
</script>

<div class="map-wrapper">
	<div bind:this={mapContainer} class="map"></div>

	{#if selectedTrail}
		<aside class="trail-panel">
			<span class="blaze" data-network={selectedTrail.network ?? 'default'}></span>
			<button class="close" onclick={clearSelection} aria-label="Close">&times;</button>

			<p class="eyebrow">{NETWORK_LABELS[selectedTrail.network] ?? 'Unclassified route'}</p>
			<h2>{selectedTrail.name ?? 'Unnamed trail'}</h2>

			<div class="stat-row">
				<span class="stat-label">Distance</span>
				<span class="stat-value">
					{selectedTrail.distance_m ? (selectedTrail.distance_m / 1000).toFixed(1) : '—'}
					{#if selectedTrail.distance_m}<span class="unit">km</span>{/if}
				</span>
			</div>
			<div class="stat-row">
				<span class="stat-label">Ascent</span>
				{#if selectedTrail.ascent_m}
					<span class="stat-value">{Math.round(selectedTrail.ascent_m)}<span class="unit">m</span></span>
				{:else}
					<span class="stat-pending">not yet measured</span>
				{/if}
			</div>
			<div class="stat-row">
				<span class="stat-label">Difficulty</span>
				{#if selectedTrail.difficulty}
					<span class="stat-value">{selectedTrail.difficulty}</span>
				{:else}
					<span class="stat-pending">unrated</span>
				{/if}
			</div>

			{#if selectedTrail.diagnostic_points?.features?.length > 0}
				<div class="marker-legend">
					<span><i class="dot open-end"></i> open end</span>
					<span><i class="dot branch"></i> junction</span>
					<span><i class="dot disconnected"></i> disconnected</span>
				</div>
			{/if}

			{#if selectedTrail.warnings?.length > 0}
				<details class="diagnostics">
					<summary>Data notes ({selectedTrail.warnings.length})</summary>
					<ul>
						{#each selectedTrail.warnings as warning}
							<li>{warning}</li>
						{/each}
					</ul>
				</details>
			{/if}
		</aside>
	{/if}

	{#if selectedWaypoint}
		<aside class="waypoint-panel">
			<button class="close" onclick={() => (selectedWaypoint = null)} aria-label="Close">&times;</button>
			<p class="waypoint-type">{WAYPOINT_LABELS[selectedWaypoint.type] ?? selectedWaypoint.type}</p>
			<strong>{selectedWaypoint.name}</strong>
		</aside>
	{/if}

	<aside class="layers-panel">
		<p class="eyebrow">Show on map</p>
		{#each WAYPOINT_GROUPS as group (group.key)}
			<label class="layer-toggle">
				<input
					type="checkbox"
					checked={enabledGroups.has(group.key)}
					onchange={() => toggleGroup(group.key)}
				/>
				{group.label}
			</label>
		{/each}
	</aside>
</div>

<style>
	:global(body) {
		font-family: 'IBM Plex Sans', sans-serif;
	}

	.map-wrapper {
		position: relative;
		width: 100%;
		height: 100%;
	}
	.map {
		width: 100%;
		height: 100%;
	}

	.trail-panel {
		position: absolute;
		top: 1.25rem;
		left: 1.25rem;
		width: 260px;
		background: #f6f5f1;
		border-radius: 3px;
		padding: 1.1rem 1.25rem 1.25rem;
		box-shadow: 0 8px 28px rgba(43, 46, 51, 0.18);
		color: #2b2e33;
		overflow: hidden;
	}

	.blaze {
		position: absolute;
		top: 0;
		left: 0;
		width: 6px;
		height: 100%;
		background: #8c9296;
	}
	.blaze[data-network='lwn'] { background: #b5651d; }
	.blaze[data-network='rwn'] { background: #3b4fa0; }
	.blaze[data-network='nwn'] { background: #9c7a1f; }
	.blaze[data-network='iwn'] { background: #79366c; }

	.eyebrow {
		margin: 0 0 0.15rem;
		font-size: 0.68rem;
		letter-spacing: 0.09em;
		text-transform: uppercase;
		color: #6b7074;
		font-weight: 500;
	}

	.trail-panel h2 {
		margin: 0 0 0.9rem;
		font-family: 'Fraunces', serif;
		font-optical-sizing: auto;
		font-weight: 600;
		font-size: 1.35rem;
		line-height: 1.15;
		letter-spacing: -0.01em;
	}

	.stat-row {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		padding: 0.42rem 0;
		border-top: 1px solid rgba(43, 46, 51, 0.1);
	}
	.stat-label {
		font-size: 0.78rem;
		color: #6b7074;
	}
	.stat-value {
		font-family: 'IBM Plex Mono', monospace;
		font-size: 0.95rem;
		font-weight: 500;
	}
	.stat-value .unit {
		font-size: 0.75rem;
		color: #6b7074;
		margin-left: 0.15rem;
	}
	.stat-pending {
		font-family: 'IBM Plex Sans', sans-serif;
		font-size: 0.8rem;
		font-style: italic;
		color: #9a9e9b;
	}

	.diagnostics {
		margin-top: 0.6rem;
		border-top: 1px solid rgba(43, 46, 51, 0.1);
		padding-top: 0.5rem;
	}
	.diagnostics summary {
		font-size: 0.72rem;
		letter-spacing: 0.04em;
		color: #9a9e9b;
		cursor: pointer;
		user-select: none;
	}
	.diagnostics summary:hover {
		color: #6b7074;
	}
	.diagnostics ul {
		margin: 0.5rem 0 0;
		padding-left: 1rem;
		font-size: 0.78rem;
		color: #6b7074;
	}
	.diagnostics li {
		margin-bottom: 0.3rem;
	}

	.marker-legend {
		display: flex;
		flex-wrap: wrap;
		gap: 0.6rem;
		margin-top: 0.6rem;
		padding-top: 0.5rem;
		border-top: 1px solid rgba(43, 46, 51, 0.1);
		font-size: 0.72rem;
		color: #6b7074;
	}
	.marker-legend .dot {
		display: inline-block;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		margin-right: 0.25rem;
	}
	.marker-legend .open-end {
		background: #b23a32;
	}
	.marker-legend .branch {
		background: #3b4fa0;
	}
	.marker-legend .disconnected {
		background: #8c9296;
	}

	.close {
		position: absolute;
		top: 0.5rem;
		right: 0.6rem;
		border: none;
		background: none;
		font-size: 1.15rem;
		line-height: 1;
		cursor: pointer;
		color: #9a9e9b;
		padding: 0.25rem;
	}
	.close:hover {
		color: #2b2e33;
	}

	.waypoint-panel {
		position: absolute;
		bottom: 1.25rem;
		left: 1.25rem;
		background: #f6f5f1;
		border-radius: 3px;
		padding: 0.7rem 1.1rem 0.8rem;
		box-shadow: 0 8px 28px rgba(43, 46, 51, 0.18);
		font-family: 'IBM Plex Sans', sans-serif;
		color: #2b2e33;
	}
	.waypoint-panel .waypoint-type {
		margin: 0 0 0.1rem;
		font-size: 0.68rem;
		letter-spacing: 0.09em;
		text-transform: uppercase;
		color: #6b7074;
		font-weight: 500;
	}
	.waypoint-panel strong {
		font-family: 'Fraunces', serif;
		font-weight: 600;
		font-size: 1.05rem;
	}

	.layers-panel {
		position: absolute;
		bottom: 1.25rem;
		right: 1.25rem;
		background: #f6f5f1;
		border-radius: 3px;
		padding: 0.75rem 1rem 0.85rem;
		box-shadow: 0 8px 28px rgba(43, 46, 51, 0.18);
		font-family: 'IBM Plex Sans', sans-serif;
		color: #2b2e33;
	}
	.layer-toggle {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		font-size: 0.85rem;
		padding: 0.2rem 0;
		cursor: pointer;
		user-select: none;
	}
	.layer-toggle input {
		accent-color: #2b2e33;
		width: 14px;
		height: 14px;
		cursor: pointer;
	}
</style>