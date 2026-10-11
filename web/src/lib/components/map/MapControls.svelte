<script lang="ts">
	import { OUTDOOR_THEME } from '../../outdoor-theme';

	interface Props {
		onZoomIn?: () => void;
		onZoomOut?: () => void;
		onFitBounds?: () => void;
		canZoomIn?: boolean;
		canZoomOut?: boolean;
	}

	let { onZoomIn, onZoomOut, onFitBounds, canZoomIn = true, canZoomOut = true }: Props = $props();
</script>

<div class="map-controls">
	<button
		class="map-control-button"
		disabled={!canZoomIn}
		onclick={onZoomIn}
		title="Zoom in (⌘ +)"
		aria-label="Zoom in"
	>
		<svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
			<path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z" />
		</svg>
	</button>

	<button
		class="map-control-button"
		disabled={!canZoomOut}
		onclick={onZoomOut}
		title="Zoom out (⌘ −)"
		aria-label="Zoom out"
	>
		<svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
			<path d="M19 13H5v-2h14v2z" />
		</svg>
	</button>

	<div class="map-control-divider"></div>

	<button
		class="map-control-button"
		onclick={onFitBounds}
		title="Fit trail in view"
		aria-label="Fit trail bounds"
	>
		<svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
			<path
				d="M3 5v4h4V7H5v2H3zm18 0v2h-2v2h4V5h-4zm-2 14h2v-2h2v4h-4v-2zM3 15h4v-2H5v2H3v2h4v-2H3v-2zm12-8h2V5h-2v2zm0 8h2v-2h-2v2z"
			/>
		</svg>
	</button>
</div>

<style>
	.map-controls {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.5rem;
		background: rgba(250, 247, 238, 0.95);
		border-radius: 8px;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
		border: 1px solid rgba(0, 0, 0, 0.1);
	}

	.map-control-button {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 44px;
		height: 44px;
		padding: 0;
		border: 1px solid rgba(0, 0, 0, 0.1);
		border-radius: 6px;
		background: white;
		color: v-bind('OUTDOOR_THEME.ink.strong');
		cursor: pointer;
		transition: all 0.2s ease;
		flex-shrink: 0;
	}

	.map-control-button:hover:not(:disabled) {
		background: rgba(46, 102, 83, 0.05);
		border-color: v-bind('OUTDOOR_THEME.brand');
		box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);
	}

	.map-control-button:active:not(:disabled) {
		transform: scale(0.95);
	}

	.map-control-button:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.map-control-divider {
		height: 1px;
		background: rgba(0, 0, 0, 0.1);
		margin: 0.25rem 0;
	}
</style>
