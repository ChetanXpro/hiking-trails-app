<script lang="ts">
	import type { TrailDetail, TrailWaypoint, WeatherForecast } from '../../types';
	import ElevationChart from '../../ElevationChart.svelte';
	import WeatherSummary from '../../WeatherSummary.svelte';

	interface Props {
		selectedTrail: TrailDetail | null;
		detailError?: string | null;
		incompleteRoute: boolean;
		downloadBusy: boolean;
		shareMessage: string | null;
		shareFallback: string | null;
		downloadError: string | null;
		elevationStatus: 'idle' | 'loading' | 'ready' | 'error' | 'missing';
		elevationProfile: any | null;
		weatherForecast: WeatherForecast | null;
		weatherStatus: 'idle' | 'loading' | 'ready' | 'error';
		weatherError: string | null;
		qaMode?: boolean;
		downloadSectionOpen: boolean;
		downloadFormat: 'gpx' | 'kml' | 'geojson';
		directionMode: 'forward' | 'backward' | 'both';
		mapperProfileUrl: string | null;
		directionLabel: (mode: 'forward' | 'backward' | 'both') => string;
		formatFeatureLabel: (feature: string) => string;
		formatSurfaceLabel: (surface: string) => string;
		surfaceColor: (surface: string) => string;
		formatDistance: (distance: number | null) => string;
		warningSummaries: Array<{ key: string; title: string; description: string; count: number }>;
		// Pre-formatted display strings from parent
		difficultyLabel?: string;
		hikeTypeLabel?: string;
		distanceLabel?: string;
		ascentLabel?: string;
		descentLabel?: string;
		durationLabel?: string;
		onClose?: () => void;
		onCenter?: () => void;
		onShare?: () => void;
		onExportGPX?: () => void;
		onExport?: () => void;
		onToggleDownload?: () => void;
		onFormatChange?: (format: 'gpx' | 'kml' | 'geojson') => void;
		onDirectionChange?: (direction: 'forward' | 'backward' | 'both') => void;
		onReloadRoute?: () => void;
		onShowWaypoint?: (waypoint: TrailWaypoint) => void;
		onRefreshWeather?: (coordinates?: any) => void;
		onElevationHover?: (point: any) => void;
	}

	let {
		selectedTrail = null,
		detailError = null,
		incompleteRoute = false,
		downloadBusy = false,
		shareMessage = null,
		shareFallback = null,
		downloadError = null,
		elevationStatus = 'idle',
		elevationProfile = null,
		weatherForecast = null,
		weatherStatus = 'idle',
		weatherError = null,
		qaMode = false,
		downloadSectionOpen,
		downloadFormat,
		directionMode,
		mapperProfileUrl,
		directionLabel,
		formatFeatureLabel,
		formatSurfaceLabel,
		surfaceColor,
		formatDistance,
		warningSummaries,
		difficultyLabel = 'Unrated',
		hikeTypeLabel = 'Hike',
		distanceLabel = '—',
		ascentLabel = '—',
		descentLabel = '—',
		durationLabel = '—',
		onClose,
		onCenter,
		onShare,
		onExportGPX,
		onExport,
		onToggleDownload,
		onFormatChange,
		onDirectionChange,
		onReloadRoute,
		onShowWaypoint,
		onRefreshWeather,
		onElevationHover
	}: Props = $props();
</script>

<aside class="trail-panel" hidden={!selectedTrail && !detailError}>
	<button class="close" onclick={onClose} aria-label="Close trail panel">×</button>

	{#if detailError}
		<p class="error">{detailError}</p>
	{:else if selectedTrail}
		{#if selectedTrail.region_breadcrumb.length > 0}
			<p class="breadcrumb">{selectedTrail.region_breadcrumb.join(' › ')}</p>
		{/if}

		<div class="hero">
			<p class="eyebrow">
				{difficultyLabel} · {hikeTypeLabel}
			</p>
			<h2>{selectedTrail.name ?? 'Unnamed trail'}</h2>
			{#if selectedTrail.summary}
				<p class="summary">{selectedTrail.summary}</p>
			{/if}
		</div>

		<div class="stats">
			<div>
				<span>Length</span>
				<strong>{distanceLabel}</strong>
			</div>
			<div>
				<span>Ascent</span>
				<strong>{ascentLabel}</strong>
			</div>
			<div>
				<span>Descent</span>
				<strong>{descentLabel}</strong>
			</div>
			<div>
				<span>Est. time</span>
				<strong>{durationLabel}</strong>
			</div>
		</div>

		{#if incompleteRoute}
			<div class="route-quality" role="note">
				<strong>Partial route</strong>
				<p>
					The source has disconnected or missing sections. The line, distance, profile and download
					cover only the reconstructed portion.
				</p>
			</div>
		{/if}

		<div class="trail-quick-actions" aria-label="Trail actions">
			<button type="button" onclick={onCenter}>
				<span>◎</span>Center
			</button>
			<button type="button" onclick={onShare}><span>↗</span>Share</button>
			<button type="button" onclick={onExportGPX} disabled={downloadBusy}>
				<span>⇩</span>{downloadBusy ? 'Preparing…' : 'Export GPX'}
			</button>
		</div>

		{#if shareMessage}
			<p class="share-status" role="status">{shareMessage}</p>
		{/if}
		{#if shareFallback}
			<input
				class="share-link"
				aria-label="Share link"
				readonly
				value={shareFallback}
				onfocus={(event) => event.currentTarget.select()}
			/>
		{/if}
		{#if downloadError}
			<p class="download-error" role="alert">{downloadError}</p>
		{/if}

		<section class="section elevation-section">
			{#if elevationStatus === 'ready' && elevationProfile}
				<ElevationChart
					points={elevationProfile.profile_points}
					onpreview={(point) => onElevationHover?.(point)}
				/>
			{:else if elevationStatus === 'loading'}
				<p class="analysis-note">Loading elevation…</p>
			{:else if elevationStatus === 'error'}
				<p class="analysis-note">Couldn't load elevation. Reload the route to try again.</p>
				<button type="button" onclick={onReloadRoute}>Reload route</button>
			{:else}
				<p class="analysis-note">Elevation is unavailable for this route.</p>
			{/if}
			<p class="analysis-note">
				Distances follow the mapped route. Time estimates exclude breaks and changing trail conditions.
			</p>
		</section>
		<p class="source-credit">Map data © OpenStreetMap contributors{#if selectedTrail.original_mapper_name}. Route first added by {#if mapperProfileUrl}<a href={mapperProfileUrl} target="_blank" rel="noreferrer">{selectedTrail.original_mapper_name}</a>{:else}{selectedTrail.original_mapper_name}{/if}{/if}. <a href={`https://www.openstreetmap.org/relation/${selectedTrail.osm_relation_id}`} target="_blank" rel="noreferrer">View source route ↗</a></p>

		{#if selectedTrail.cautions?.length}
			<section class="section">
				<h3>Access & trail notes</h3>
				<ul class="caution-list">
					{#each selectedTrail.cautions as caution}
						<li>
							{caution.message}
							<a href={caution.source_url} target="_blank" rel="noreferrer">Source ↗</a>
						</li>
					{/each}
				</ul>
			</section>
		{/if}

		{#if selectedTrail.waypoints.length}
			<details class="section waypoint-section">
				<summary>Near the route · {selectedTrail.waypoints.length} mapped places</summary>
				<p class="analysis-note">
					Within 150 m of the mapped line; a place may require a detour. A mapped water source does not
					confirm drinking-water quality.
				</p>
				<div class="waypoint-list">
					{#each selectedTrail.waypoints as waypoint}
						<button
							type="button"
							disabled={waypoint.longitude == null || waypoint.latitude == null}
							onclick={() => onShowWaypoint?.(waypoint)}
						>
							<span>
								<strong>{waypoint.name ?? waypoint.kind}</strong>
								<small>
									{waypoint.kind}{waypoint.elevation_m != null
										? ` · ${Math.round(waypoint.elevation_m)} m`
										: ''}
								</small>
							</span>
							<span aria-hidden="true">◎</span>
						</button>
					{/each}
				</div>
			</details>
		{/if}

		<details class="section weather-section">
			<summary>Trailhead weather</summary>
			<WeatherSummary
				forecast={weatherForecast}
				status={weatherStatus}
				error={weatherError}
				title="Trailhead weather"
				compact={true}
				onrefresh={() => onRefreshWeather?.(selectedTrail?.start_point?.coordinates)}
			/>
		</details>

		{#if selectedTrail.features.length > 0}
			<section class="section">
				<h3>What to expect</h3>
				<div class="chips">
					{#each selectedTrail.features as feature}
						<span class="chip">{formatFeatureLabel(feature)}</span>
					{/each}
				</div>
			</section>
		{/if}

		{#if selectedTrail.surfaces.length > 0}
			<section class="section">
				<h3>Surfaces</h3>
				<div class="surface-bar">
					{#each selectedTrail.surfaces as surface}
						<div class="surface-segment" style={`width:${surface.pct}%;background:${surfaceColor(surface.surface_type)}`}></div>
					{/each}
				</div>
				<div class="surface-list">
					{#each selectedTrail.surfaces as surface}
						<div class="surface-item">
							<span class="dot" style={`background:${surfaceColor(surface.surface_type)}`}></span>
							<div>
								<strong>{formatSurfaceLabel(surface.surface_type)}</strong>
								<p>{surface.pct.toFixed(0)}% · {formatDistance(surface.distance_m)}</p>
							</div>
						</div>
					{/each}
				</div>
			</section>
		{/if}

		<section class="section download-section">
			<button class="section-toggle" type="button" aria-expanded={downloadSectionOpen} onclick={onToggleDownload}>
				<span class="section-title">Download route</span><span class:open={downloadSectionOpen}>⌄</span>
			</button>
			{#if downloadSectionOpen}
				<p class="analysis-note">Choose a format and travel direction. The map previews the selected direction.</p>
				<div class="download-fieldset">
					<span class="fieldset-label">Format</span>
					<div class="radio-list format-list">
						{#each ['gpx', 'kml', 'geojson'] as format}
							<label><input type="radio" name="route-format" checked={downloadFormat === format} onchange={() => onFormatChange?.(format as 'gpx' | 'kml' | 'geojson')} /><span>{format === 'geojson' ? 'GeoJSON' : format.toUpperCase()}</span></label>
						{/each}
					</div>
				</div>
				<div class="download-fieldset">
					<span class="fieldset-label">Travel direction</span>
					<div class="radio-list direction-list">
						{#each ['forward', 'backward', 'both'] as direction}
							<label><input type="radio" name="route-direction" checked={directionMode === direction} onchange={() => onDirectionChange?.(direction as 'forward' | 'backward' | 'both')} /><div><strong>{direction === 'both' ? 'Both ways' : direction === 'forward' ? 'Forward' : 'Backward'}</strong><p>{directionLabel(direction as 'forward' | 'backward' | 'both')}</p></div></label>
						{/each}
					</div>
				</div>
				<button class="download-button" type="button" disabled={downloadBusy} onclick={onExport}>{downloadBusy ? 'Preparing file…' : 'Get file'}</button>
			{/if}
		</section>

		{#if qaMode && (selectedTrail.stitch_diagnostics.length || warningSummaries.length)}
			<section class="section">
				<h3>Analyze relation</h3>
				<p class="analysis-note">Review reconstruction warnings against the route markers on the map.</p>
				<div class="analysis-list">
					{#each warningSummaries as summary (summary.key)}
						<div class="analysis-item warning-item"><span class="dot warning-dot"></span><div><strong>{summary.title}{summary.count > 1 ? ` (${summary.count})` : ''}</strong><p>{summary.description}</p></div></div>
					{/each}
				</div>
			</section>
		{/if}
	{/if}
</aside>
