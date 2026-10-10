<script lang="ts">
	import { OUTDOOR_THEME } from '../../outdoor-theme';

	interface Props {
		mapPitch?: number;
		mapBearing?: number;
		onPitchChange?: (pitch: number) => void;
		onBearingChange?: (bearing: number) => void;
	}

	let { mapPitch = 0, mapBearing = 0, onPitchChange, onBearingChange }: Props = $props();

	function handlePitchChange(event: Event) {
		const value = Number((event.target as HTMLInputElement).value);
		onPitchChange?.(value);
	}

	function handleBearingChange(event: Event) {
		const value = Number((event.target as HTMLInputElement).value);
		onBearingChange?.(value);
	}
</script>

<div class="map-3d-controls">
	<div class="terrain-fieldset camera-controls">
		<div class="terrain-slider-row">
			<span class="terrain-fieldset-label">Camera pitch</span>
			<strong>{mapPitch}°</strong>
		</div>
		<input
			class="terrain-slider"
			type="range"
			min="20"
			max="82"
			step="1"
			value={mapPitch}
			oninput={handlePitchChange}
		/>
		<div class="terrain-slider-row bearing-row">
			<span class="terrain-fieldset-label">Bearing</span>
			<strong>{mapBearing}°</strong>
		</div>
		<input
			class="terrain-slider"
			type="range"
			min="0"
			max="360"
			step="1"
			value={mapBearing}
			oninput={handleBearingChange}
		/>
	</div>
</div>

<style>
	.map-3d-controls {
		display: contents;
	}

	.terrain-fieldset {
		border: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.camera-controls {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		padding: 1rem;
		background: rgba(250, 247, 238, 0.95);
		border-radius: 8px;
		border: 1px solid rgba(0, 0, 0, 0.1);
	}

	.terrain-slider-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.terrain-slider-row strong {
		font-variant-numeric: tabular-nums;
		min-width: 3em;
		text-align: right;
	}

	.terrain-slider-row.bearing-row {
		margin-top: 0.5rem;
		padding-top: 0.5rem;
		border-top: 1px solid rgba(0, 0, 0, 0.1);
	}

	.terrain-fieldset-label {
		font-weight: 600;
		color: v-bind('OUTDOOR_THEME.ink.strong');
		font-size: 0.875rem;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.terrain-slider {
		width: 100%;
		height: 6px;
		border-radius: 3px;
		background: linear-gradient(
			to right,
			rgba(46, 102, 83, 0.2),
			rgba(46, 102, 83, 0.5)
		);
		outline: none;
		-webkit-appearance: none;
		appearance: none;
		cursor: pointer;
	}

	.terrain-slider::-webkit-slider-thumb {
		-webkit-appearance: none;
		appearance: none;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background: v-bind('OUTDOOR_THEME.brand');
		cursor: pointer;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
		transition: all 0.2s ease;
	}

	.terrain-slider::-webkit-slider-thumb:hover {
		background: v-bind('OUTDOOR_THEME.brandHover');
		box-shadow: 0 3px 6px rgba(0, 0, 0, 0.2);
	}

	.terrain-slider::-moz-range-thumb {
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background: v-bind('OUTDOOR_THEME.brand');
		cursor: pointer;
		border: none;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
		transition: all 0.2s ease;
	}

	.terrain-slider::-moz-range-thumb:hover {
		background: v-bind('OUTDOOR_THEME.brandHover');
		box-shadow: 0 3px 6px rgba(0, 0, 0, 0.2);
	}
</style>
