<script lang="ts">
	import type { TrailDetail, TrailSurface, TrailWaypoint, WeatherForecast } from '../../types';
	import ElevationChart from '../../ElevationChart.svelte';
	import WeatherSummary from '../../WeatherSummary.svelte';

	interface Props {
		selectedTrail: TrailDetail | null;
		incompleteRoute: boolean;
		downloadBusy: boolean;
		shareMessage: string | null;
		shareFallback: string | null;
		downloadError: string | null;
		elevationStatus: 'loading' | 'ready' | 'error' | 'unavailable';
		elevationProfile: any | null;
		weatherForecast: WeatherForecast | null;
		weatherStatus: 'loading' | 'ready' | 'error';
		weatherError: string | null;
		qaMode?: boolean;
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
		onReloadRoute?: () => void;
		onShowWaypoint?: (waypoint: TrailWaypoint) => void;
		onRefreshWeather?: (coordinates?: any) => void;
		onElevationHover?: (point: any) => void;
	}

	let {
		selectedTrail = null,
		incompleteRoute = false,
		downloadBusy = false,
		shareMessage = null,
		shareFallback = null,
		downloadError = null,
		elevationStatus = 'unavailable',
		elevationProfile = null,
		weatherForecast = null,
		weatherStatus = 'loading',
		weatherError = null,
		qaMode = false,
		onClose,
		onCenter,
		onShare,
		onExportGPX,
		onReloadRoute,
		onShowWaypoint,
		onRefreshWeather,
		onElevationHover
	}: Props = $props();
</script>

<aside class="trail-panel" hidden={!selectedTrail}>
	<button class="close" onclick={onClose} aria-label="Close trail panel">×</button>

	{#if selectedTrail}
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
						<span class="chip">{feature}</span>
					{/each}
				</div>
			</section>
		{/if}

		{#if selectedTrail.surfaces.length > 0}
			<section class="section">
				<h3>Surfaces</h3>
				<div class="surface-bar">
					{#each selectedTrail.surfaces as surface}
						<div class="surface-segment" style={`width:${surface.pct}%;background: #888`}></div>
					{/each}
				</div>
				<div class="surface-list">
					{#each selectedTrail.surfaces as surface}
						<div class="surface-item">
							<span class="dot" style="background: #888"></span>
							<div>
								<strong>{surface.surface_type}</strong>
								<p>{surface.pct.toFixed(0)}%</p>
							</div>
						</div>
					{/each}
				</div>
			</section>
		{/if}
	{/if}
</aside>

<style>
	.trail-panel {
		position: absolute;
		right: 0;
		top: 0;
		bottom: 0;
		z-index: 10;
		width: min(460px, 100vw);
		background: rgba(250, 247, 238, 0.96);
		border-left: 1px solid rgba(0, 0, 0, 0.08);
		overflow: auto;
		padding: 1rem;
		display: flex;
		flex-direction: column;
		transform: translateX(100%);
		transition: transform 200ms ease;
	}

	.trail-panel:not([hidden]) {
		transform: translateX(0);
	}

	.close {
		position: absolute;
		top: 1rem;
		right: 1rem;
		width: 2rem;
		height: 2rem;
		border: none;
		border-radius: 50%;
		background: rgba(0, 0, 0, 0.05);
		cursor: pointer;
		font-size: 1.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.2s ease;
		color: #25302a;
	}

	.close:hover {
		background: rgba(0, 0, 0, 0.1);
	}

	.breadcrumb {
		margin: 0;
		font-size: 0.8rem;
		color: #6b5d4f;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		font-weight: 500;
	}

	.hero {
		margin: 1rem 0;
	}

	.hero h2 {
		margin: 0.5rem 0 0.25rem;
		font-size: 1.5rem;
		color: #25302a;
	}

	.hero .eyebrow {
		margin: 0;
		font-size: 0.85rem;
		color: #2e6653;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.hero .summary {
		margin: 0.5rem 0 0;
		font-size: 0.95rem;
		color: #5d6961;
		line-height: 1.5;
	}

	.stats {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
		margin: 1rem 0;
	}

	.stats div {
		padding: 0.75rem;
		background: rgba(46, 102, 83, 0.05);
		border-radius: 6px;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.stats span {
		font-size: 0.75rem;
		font-weight: 600;
		color: #6b5d4f;
		text-transform: uppercase;
	}

	.stats strong {
		font-size: 1.1rem;
		color: #25302a;
	}

	.trail-quick-actions {
		display: flex;
		gap: 0.5rem;
		margin: 1rem 0;
	}

	.trail-quick-actions button {
		flex: 1;
		padding: 0.75rem;
		border: 1px solid rgba(0, 0, 0, 0.1);
		border-radius: 6px;
		background: white;
		cursor: pointer;
		font-weight: 600;
		font-size: 0.85rem;
		transition: all 0.2s ease;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		color: #25302a;
	}

	.trail-quick-actions button:hover:not(:disabled) {
		background: rgba(46, 102, 83, 0.1);
		border-color: #2e6653;
	}

	.trail-quick-actions button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.section {
		margin: 1.5rem 0;
		padding: 0.75rem;
		border-radius: 6px;
		background: rgba(46, 102, 83, 0.03);
		border: 1px solid rgba(46, 102, 83, 0.1);
	}

	.section h3 {
		margin: 0 0 0.75rem;
		font-size: 0.95rem;
		font-weight: 700;
		color: #25302a;
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.chip {
		display: inline-block;
		padding: 0.35rem 0.65rem;
		background: rgba(46, 102, 83, 0.15);
		border-radius: 999px;
		font-size: 0.8rem;
		color: #2e6653;
		font-weight: 500;
	}

	.waypoint-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.waypoint-list button {
		padding: 0.75rem;
		border: 1px solid rgba(0, 0, 0, 0.08);
		border-radius: 6px;
		background: white;
		text-align: left;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: space-between;
		transition: all 0.2s ease;
	}

	.waypoint-list button:not(:disabled):hover {
		background: rgba(46, 102, 83, 0.05);
		border-color: #2e6653;
	}

	.surface-bar {
		display: flex;
		height: 1.5rem;
		border-radius: 6px;
		overflow: hidden;
		margin-bottom: 0.75rem;
	}

	.surface-segment {
		flex: 1;
		min-width: 4px;
	}

	.surface-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.surface-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: 0.85rem;
		padding: 0.5rem;
	}

	.dot {
		width: 1rem;
		height: 1rem;
		border-radius: 3px;
		flex-shrink: 0;
	}

	.analysis-note {
		margin: 0.5rem 0;
		font-size: 0.8rem;
		color: #6b5d4f;
		line-height: 1.4;
	}
</style>
