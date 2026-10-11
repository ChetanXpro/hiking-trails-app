<script lang="ts">
	interface Props {
		knownRoutesVisible: boolean;
		routeLabelsVisible: boolean;
		routeLabelsDisabled: boolean;
		ordinaryPathsVisible: boolean;
		outdoorPoisVisible: boolean;
		peakLabelsVisible: boolean;
		regionBoundariesVisible: boolean;
		onKnownRoutesToggle?: (enabled: boolean) => void;
		onRouteLabelsToggle?: (enabled: boolean) => void;
		onOrdinaryPathsToggle?: (enabled: boolean) => void;
		onOutdoorPoisToggle?: (enabled: boolean) => void;
		onPeakLabelsToggle?: (enabled: boolean) => void;
		onRegionBoundariesToggle?: (enabled: boolean) => void;
	}

	let {
		knownRoutesVisible,
		routeLabelsVisible,
		routeLabelsDisabled,
		ordinaryPathsVisible,
		outdoorPoisVisible,
		peakLabelsVisible,
		regionBoundariesVisible,
		onKnownRoutesToggle,
		onRouteLabelsToggle,
		onOrdinaryPathsToggle,
		onOutdoorPoisToggle,
		onPeakLabelsToggle,
		onRegionBoundariesToggle
	}: Props = $props();
</script>

<div class="terrain-control-list">
	<label class="terrain-switch">
		<div>
			<strong>Known hiking routes</strong>
			<p>Mapped hiking routes shown in red.</p>
		</div>
		<input
			type="checkbox"
			bind:checked={knownRoutesVisible}
			onchange={(event) => onKnownRoutesToggle?.((event.currentTarget as HTMLInputElement).checked)}
		/>
	</label>
	<label class="terrain-switch">
		<div>
			<strong>Route names</strong>
			<p>Names of the mapped hiking routes.</p>
		</div>
		<input
			type="checkbox"
			bind:checked={routeLabelsVisible}
			disabled={routeLabelsDisabled}
			onchange={(event) => onRouteLabelsToggle?.((event.currentTarget as HTMLInputElement).checked)}
		/>
	</label>
	<label class="terrain-switch">
		<div>
			<strong>Ordinary paths</strong>
			<p>Other mapped footpaths and tracks.</p>
		</div>
		<input
			type="checkbox"
			bind:checked={ordinaryPathsVisible}
			onchange={(event) => onOrdinaryPathsToggle?.((event.currentTarget as HTMLInputElement).checked)}
		/>
	</label>
	<label class="terrain-switch">
		<div>
			<strong>Outdoor places</strong>
			<p>Rescue, huts, water, and trailheads first; local comforts appear as you zoom in.</p>
		</div>
		<input
			type="checkbox"
			bind:checked={outdoorPoisVisible}
			onchange={(event) => onOutdoorPoisToggle?.((event.currentTarget as HTMLInputElement).checked)}
		/>
	</label>
	<label class="terrain-switch">
		<div>
			<strong>Peaks</strong>
			<p>Highest summits first; nearby peaks appear as you zoom in.</p>
		</div>
		<input
			type="checkbox"
			bind:checked={peakLabelsVisible}
			onchange={(event) => onPeakLabelsToggle?.((event.currentTarget as HTMLInputElement).checked)}
		/>
	</label>
	<label class="terrain-switch">
		<div>
			<strong>Region boundaries</strong>
			<p>Administrative context for trip discovery.</p>
		</div>
		<input
			type="checkbox"
			bind:checked={regionBoundariesVisible}
			onchange={(event) => onRegionBoundariesToggle?.((event.currentTarget as HTMLInputElement).checked)}
		/>
	</label>
</div>

<style>
	:global(.terrain-control-list) {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	:global(.terrain-switch) {
		display: flex;
		gap: 0.75rem;
		padding: 0.75rem;
		border-radius: 6px;
		background: rgba(250, 247, 238, 0.95);
		border: 1px solid rgba(0, 0, 0, 0.08);
		cursor: pointer;
		transition: all 0.2s ease;
		align-items: flex-start;
	}

	:global(.terrain-switch:hover) {
		background: rgba(46, 102, 83, 0.05);
		border-color: #2e6653;
	}

	:global(.terrain-switch div) {
		flex: 1;
		min-width: 0;
	}

	:global(.terrain-switch strong) {
		display: block;
		font-weight: 600;
		margin-bottom: 0.25rem;
		color: #2b1810;
	}

	:global(.terrain-switch p) {
		margin: 0;
		font-size: 0.8rem;
		color: #6b5d4f;
		line-height: 1.3;
	}

	:global(.terrain-switch input[type='checkbox']) {
		flex-shrink: 0;
		width: 1.25rem;
		height: 1.25rem;
		cursor: pointer;
		margin-top: 0.125rem;
	}
</style>
