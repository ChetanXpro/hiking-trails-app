<script lang="ts">
	import type { MapFeatureSelection, WeatherForecast } from '../../types';
	import { openStreetMapUrl } from '../../map-selection';
	import WeatherSummary from '../../WeatherSummary.svelte';

	interface Props {
		selectedFeature: MapFeatureSelection | null;
		weatherForecast: WeatherForecast | null;
		weatherStatus: 'idle' | 'loading' | 'ready' | 'error';
		weatherError: string | null;
		nearbyFeatures?: any[];
		nearbyTrails?: any[];
		onClose?: () => void;
		onCenter?: () => void;
		onCopyCoordinates?: () => void;
		onSelectNearby?: (feature: any) => void;
		onLoadTrail?: (id: string) => void;
		onRefreshWeather?: (coordinates?: any) => void;
		coordinateCopyStatus?: 'idle' | 'copied' | 'error';
	}

	let {
		selectedFeature = null,
		weatherForecast = null,
		weatherStatus = 'idle',
		weatherError = null,
		nearbyFeatures = [],
		nearbyTrails = [],
		onClose,
		onCenter,
		onCopyCoordinates,
		onSelectNearby,
		onLoadTrail,
		onRefreshWeather,
		coordinateCopyStatus = 'idle'
	}: Props = $props();
</script>

<aside class="trail-panel feature-panel" hidden={!selectedFeature} aria-label={selectedFeature ? `${selectedFeature.title} details` : ''}>
	<button class="close" onclick={onClose} aria-label="Close location panel">×</button>
	{#if selectedFeature}
		<p class="breadcrumb">{selectedFeature.sourceLabel}</p>

		<div class="hero">
			<p class="eyebrow">{selectedFeature.kindLabel}</p>
			<h2>{selectedFeature.title}</h2>
			<p class="summary">
				{selectedFeature.source === 'marked_location'
					? 'Terrain and current conditions for the point you selected.'
					: 'Mapped outdoor information and current conditions at this location.'}
			</p>
		</div>

		<div class="stats feature-stats" aria-live="polite">
			<div>
				<span>Elevation</span>
				<strong>
					{selectedFeature.elevationM == null ? '—' : selectedFeature.elevationM.toFixed(0) + ' m'}
				</strong>
				{#if selectedFeature.elevationSource}
					<small>{selectedFeature.elevationSource === 'mapped' ? 'Mapped elevation' : 'Merged terrain DEM'}</small>
				{/if}
			</div>
			<div>
				<span>Coordinates</span>
				<strong>{selectedFeature.coordinates[1].toFixed(5)}, {selectedFeature.coordinates[0].toFixed(5)}</strong>
				<small>Latitude, longitude</small>
			</div>
		</div>

		<div class="trail-quick-actions feature-actions" aria-label="Location actions">
			<button type="button" onclick={onCenter}><span>◎</span>Center</button>
			<button type="button" onclick={onCopyCoordinates}>
				<span>⧉</span>{coordinateCopyStatus === 'copied'
					? 'Copied'
					: coordinateCopyStatus === 'error'
						? 'Copy failed'
						: 'Copy coordinates'}
			</button>
			<a href={openStreetMapUrl(selectedFeature)} target="_blank" rel="noreferrer">
				<span>↗</span>Open in OSM
			</a>
		</div>

		<WeatherSummary
			forecast={weatherForecast}
			status={weatherStatus}
			error={weatherError}
			title="Current weather"
			compact={true}
			onrefresh={() => onRefreshWeather?.(selectedFeature?.coordinates)}
		/>

		<section class="section">
			<h3>Location details</h3>
			<div class="meta-grid">
				<div class="meta-card">
					<span>Feature</span>
					<strong>{selectedFeature.kindLabel}</strong>
				</div>
				<div class="meta-card">
					<span>Source</span>
					<strong>{selectedFeature.source === 'marked_location' ? 'Map selection' : 'OpenStreetMap'}</strong>
				</div>
				{#if selectedFeature.osmType && selectedFeature.osmId != null}
					<div class="meta-card meta-card-wide">
						<span>OSM object</span>
						<strong>{selectedFeature.osmType === 'N' ? 'Node' : selectedFeature.osmType === 'W' ? 'Way' : 'Relation'}
							{selectedFeature.osmId}</strong>
						<p>OpenStreetMap contributors maintain this public feature.</p>
					</div>
				{/if}
			</div>
		</section>

		{#if nearbyFeatures.length > 0 || nearbyTrails.length > 0}
			<section class="section">
				<h3>Also here</h3>
				<div class="nearby-feature-list">
					{#each nearbyFeatures as feature}
						<button type="button" class="nearby-feature" onclick={() => onSelectNearby?.(feature)}>
							<span>{feature.kindLabel}</span>
							<strong>{feature.title}</strong>
						</button>
					{/each}
					{#each nearbyTrails as trail}
						<button type="button" class="nearby-feature" onclick={() => onLoadTrail?.(trail.id)}>
							<span>Trail</span>
							<strong>{trail.name}</strong>
							{#if trail.network}<small>{trail.network}</small>{/if}
						</button>
					{/each}
				</div>
			</section>
		{/if}
	{/if}
</aside>
