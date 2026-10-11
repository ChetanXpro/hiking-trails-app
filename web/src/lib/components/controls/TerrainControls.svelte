<script lang="ts">
	import { SLOPE_BANDS, SLOPE_CLEAR_BELOW, SLOPE_PLANNING_NOTE } from '../../slope-config';
	import { contourIntervalsAtZoom } from '../../terrain-config';
	import { OUTDOOR_THEME } from '../../outdoor-theme';

	interface Props {
		mapZoom: number;
		mapPitch: number;
		mapBearing: number;
		terrain3dEnabled: boolean;
		terrainReady: boolean;
		terrainHillshadeVisible: boolean;
		terrainColorReliefVisible: boolean;
		terrainColorReliefSupported: boolean;
		terrainSlopeVisible: boolean;
		terrainSlopeSupported: boolean;
		terrainSlopeOpacity: number;
		terrainContoursVisible: boolean;
		terrainContourUnit: 'meters' | 'feet';
		onHillshadeToggle?: (enabled: boolean) => void;
		onColorReliefToggle?: (enabled: boolean) => void;
		onSlopeToggle?: (enabled: boolean) => void;
		onContoursToggle?: (enabled: boolean) => void;
		onSlopeOpacityChange?: (opacity: number) => void;
		onContourUnitChange?: (unit: 'meters' | 'feet') => void;
		onSet3DViewPreset?: (preset: 'terrain' | 'horizon') => void;
		onMapPitchChange?: (pitch: number) => void;
		onMapBearingChange?: (bearing: number) => void;
		onRotateMap?: (degrees: number) => void;
	}

	let {
		mapZoom,
		mapPitch,
		mapBearing,
		terrain3dEnabled,
		terrainReady,
		terrainHillshadeVisible,
		terrainColorReliefVisible,
		terrainColorReliefSupported,
		terrainSlopeVisible,
		terrainSlopeSupported,
		terrainSlopeOpacity,
		terrainContoursVisible,
		terrainContourUnit,
		onHillshadeToggle,
		onColorReliefToggle,
		onSlopeToggle,
		onContoursToggle,
		onSlopeOpacityChange,
		onContourUnitChange,
		onSet3DViewPreset,
		onMapPitchChange,
		onMapBearingChange,
		onRotateMap
	}: Props = $props();

	const contourInterval = $derived(contourIntervalsAtZoom(terrainContourUnit, mapZoom));
</script>

<div class="terrain-control-list">
	<label class="terrain-switch">
		<div>
			<strong>Hillshade</strong>
			<p>Multidirectional relief for slopes, gullies, and ridgelines.</p>
		</div>
		<input
			type="checkbox"
			checked={terrainHillshadeVisible}
			disabled={!terrainReady}
			onchange={(event) => onHillshadeToggle?.((event.currentTarget as HTMLInputElement).checked)}
		/>
	</label>
	{#if terrainColorReliefSupported}
		<label class="terrain-switch">
			<div>
				<strong>Elevation tint</strong>
				<p>A subtle regional height wash; off by default for clarity.</p>
			</div>
			<input
				type="checkbox"
				checked={terrainColorReliefVisible}
				disabled={!terrainReady}
				onchange={(event) =>
					onColorReliefToggle?.((event.currentTarget as HTMLInputElement).checked)}
			/>
		</label>
	{/if}
	{#if terrainSlopeSupported}
		<label class="terrain-switch">
			<div>
				<strong>Slope angle</strong>
				<p>Optional DEM steepness bands for terrain planning; not an avalanche-danger layer.</p>
			</div>
			<input
				type="checkbox"
				checked={terrainSlopeVisible}
				disabled={!terrainReady}
				onchange={(event) => onSlopeToggle?.((event.currentTarget as HTMLInputElement).checked)}
			/>
		</label>
	{/if}
	<label class="terrain-switch">
		<div>
			<strong>Contours</strong>
			<p>
				{mapZoom < 8
					? contourInterval.index
					: mapZoom < 9
						? contourInterval.intermediate
						: contourInterval.minor}
				{terrainContourUnit === 'feet' ? 'ft' : 'm'} interval at this zoom; bold lines every
				{contourInterval.index}
				{terrainContourUnit === 'feet' ? 'ft' : 'm'}.
			</p>
		</div>
		<input
			type="checkbox"
			checked={terrainContoursVisible}
			disabled={!terrainReady}
			onchange={(event) => onContoursToggle?.((event.currentTarget as HTMLInputElement).checked)}
		/>
	</label>
</div>

{#if terrainSlopeSupported && terrainSlopeVisible}
	<div class="slope-panel" aria-label="Slope angle legend">
		<div class="terrain-slider-row">
			<span class="terrain-fieldset-label">Slope opacity</span>
			<strong>{Math.round(terrainSlopeOpacity * 100)}%</strong>
		</div>
		<input
			class="terrain-slider"
			aria-label="Slope opacity"
			type="range"
			min="0.25"
			max="1"
			step="0.05"
			value={terrainSlopeOpacity}
			oninput={(event) =>
				onSlopeOpacityChange?.(Number((event.currentTarget as HTMLInputElement).value))}
		/>
		<div class="slope-legend">
			<span><i class="slope-swatch clear"></i>&lt;{SLOPE_CLEAR_BELOW}° clear</span>
			{#each SLOPE_BANDS as band}
				<span>
					<i
						class="slope-swatch"
						style={`--slope-color: rgba(${band.color.join(', ')}, ${band.alpha})`}
					></i>
					{band.label}
				</span>
			{/each}
		</div>
		<p class="slope-note">{SLOPE_PLANNING_NOTE}</p>
	</div>
{/if}

<div class="terrain-fieldset">
	<span class="terrain-fieldset-label">Contour unit</span>
	<div class="terrain-segmented">
		<label>
			<input
				type="radio"
				name="terrain-unit"
				checked={terrainContourUnit === 'meters'}
				onchange={() => onContourUnitChange?.('meters')}
			/>
			<span>meters</span>
		</label>
		<label>
			<input
				type="radio"
				name="terrain-unit"
				checked={terrainContourUnit === 'feet'}
				onchange={() => onContourUnitChange?.('feet')}
			/>
			<span>feet</span>
		</label>
	</div>
</div>

{#if terrain3dEnabled}
	<div class="view-presets" aria-label="3D view presets">
		<button class:active={mapPitch < 72} type="button" onclick={() => onSet3DViewPreset?.('terrain')}>
			Terrain view
		</button>
		<button class:active={mapPitch >= 72} type="button" onclick={() => onSet3DViewPreset?.('horizon')}>
			Low horizon
		</button>
	</div>
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
			oninput={(event) =>
				onMapPitchChange?.(Number((event.currentTarget as HTMLInputElement).value))}
		/>
		<div class="terrain-slider-row bearing-row">
			<span class="terrain-fieldset-label">Bearing</span>
			<strong>{mapBearing}°</strong>
		</div>
		<div class="bearing-actions">
			<button type="button" onclick={() => onRotateMap?.(-30)}>↶ 30°</button>
			<button type="button" onclick={() => onMapBearingChange?.(0)}>North</button>
			<button type="button" onclick={() => onRotateMap?.(30)}>30° ↷</button>
		</div>
	</div>
{/if}

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
		border-color: v-bind('OUTDOOR_THEME.brand');
	}

	:global(.terrain-switch div) {
		flex: 1;
		min-width: 0;
	}

	:global(.terrain-switch strong) {
		display: block;
		font-weight: 600;
		margin-bottom: 0.25rem;
		color: v-bind('OUTDOOR_THEME.ink.strong');
	}

	:global(.terrain-switch p) {
		margin: 0;
		font-size: 0.8rem;
		color: v-bind('OUTDOOR_THEME.ink.medium');
		line-height: 1.3;
	}

	:global(.terrain-switch input[type='checkbox']) {
		flex-shrink: 0;
		width: 1.25rem;
		height: 1.25rem;
		cursor: pointer;
		margin-top: 0.125rem;
	}

	:global(.slope-panel) {
		padding: 1rem;
		background: rgba(46, 102, 83, 0.03);
		border-radius: 6px;
		border: 1px solid rgba(46, 102, 83, 0.1);
	}

	:global(.terrain-slider-row) {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	:global(.terrain-slider-row strong) {
		font-variant-numeric: tabular-nums;
		min-width: 3em;
		text-align: right;
	}

	:global(.terrain-slider-row.bearing-row) {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid rgba(0, 0, 0, 0.1);
	}

	:global(.terrain-fieldset-label) {
		font-weight: 600;
		color: v-bind('OUTDOOR_THEME.ink.strong');
		font-size: 0.875rem;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	:global(.terrain-slider) {
		width: 100%;
		height: 6px;
		border-radius: 3px;
		background: linear-gradient(to right, rgba(46, 102, 83, 0.2), rgba(46, 102, 83, 0.5));
		outline: none;
		-webkit-appearance: none;
		appearance: none;
		cursor: pointer;
		margin-bottom: 0.5rem;
	}

	:global(.terrain-slider::-webkit-slider-thumb) {
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

	:global(.terrain-slider::-webkit-slider-thumb:hover) {
		background: v-bind('OUTDOOR_THEME.brandHover');
		box-shadow: 0 3px 6px rgba(0, 0, 0, 0.2);
	}

	:global(.slope-legend) {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		font-size: 0.8rem;
		margin: 0.75rem 0;
	}

	:global(.slope-legend span) {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: v-bind('OUTDOOR_THEME.ink.medium');
	}

	:global(.slope-swatch) {
		display: inline-block;
		width: 1rem;
		height: 1rem;
		border-radius: 2px;
		background-color: var(--slope-color, rgba(46, 102, 83, 0.2));
		flex-shrink: 0;
	}

	:global(.slope-swatch.clear) {
		background-color: transparent;
		border: 1px solid v-bind('OUTDOOR_THEME.border');
	}

	:global(.slope-note) {
		margin: 0.75rem 0 0 0;
		font-size: 0.75rem;
		color: v-bind('OUTDOOR_THEME.ink.light');
		font-style: italic;
	}

	:global(.terrain-fieldset) {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	:global(.terrain-segmented) {
		display: flex;
		gap: 0;
		border-radius: 6px;
		overflow: hidden;
		background: rgba(0, 0, 0, 0.05);
	}

	:global(.terrain-segmented label) {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0.5rem;
		background: white;
		border-right: 1px solid rgba(0, 0, 0, 0.08);
		cursor: pointer;
		font-size: 0.9rem;
		font-weight: 500;
		transition: all 0.2s ease;
	}

	:global(.terrain-segmented label:last-child) {
		border-right: none;
	}

	:global(.terrain-segmented input[type='radio']) {
		margin-right: 0.5rem;
		cursor: pointer;
	}

	:global(.terrain-segmented input[type='radio']:checked + span) {
		color: white;
	}

	:global(.terrain-segmented label:has(input:checked)) {
		background: v-bind('OUTDOOR_THEME.brand');
		color: white;
	}

	:global(.view-presets) {
		display: flex;
		gap: 0.5rem;
		margin: 1rem 0;
	}

	:global(.view-presets button) {
		flex: 1;
		padding: 0.75rem;
		border: 2px solid v-bind('OUTDOOR_THEME.border');
		border-radius: 6px;
		background: white;
		color: v-bind('OUTDOOR_THEME.ink.strong');
		font-weight: 600;
		font-size: 0.9rem;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	:global(.view-presets button:hover) {
		border-color: v-bind('OUTDOOR_THEME.brand');
		background: rgba(46, 102, 83, 0.05);
	}

	:global(.view-presets button.active) {
		background: v-bind('OUTDOOR_THEME.brand');
		color: white;
		border-color: v-bind('OUTDOOR_THEME.brand');
	}

	:global(.camera-controls) {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 1rem;
		background: rgba(250, 247, 238, 0.95);
		border-radius: 6px;
		border: 1px solid rgba(0, 0, 0, 0.1);
	}

	:global(.bearing-actions) {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.5rem;
	}

	:global(.bearing-actions button) {
		flex: 1;
		padding: 0.5rem;
		border: 1px solid rgba(0, 0, 0, 0.1);
		border-radius: 4px;
		background: white;
		font-size: 0.8rem;
		cursor: pointer;
		transition: all 0.2s ease;
		font-weight: 500;
	}

	:global(.bearing-actions button:hover) {
		border-color: v-bind('OUTDOOR_THEME.brand');
		background: rgba(46, 102, 83, 0.05);
	}
</style>
