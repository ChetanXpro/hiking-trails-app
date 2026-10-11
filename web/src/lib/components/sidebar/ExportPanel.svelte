<script lang="ts">
	import { OUTDOOR_THEME } from '../../outdoor-theme';

	interface Props {
		trailId?: string;
		trailName?: string;
		onExport?: (format: string) => void;
		isLoading?: boolean;
		availableFormats?: string[];
	}

	let {
		trailId,
		trailName,
		onExport,
		isLoading = false,
		availableFormats = ['GPX', 'GeoJSON', 'KML']
	}: Props = $props();

	function handleExport(format: string) {
		onExport?.(format);
	}
</script>

<div class="export-panel">
	{#if trailId && trailName}
		<div class="export-header">
			<h3>Export Trail</h3>
			<p class="export-trail-name">{trailName}</p>
		</div>

		<div class="export-formats">
			{#each availableFormats as format}
				<button
					class="export-button"
					disabled={isLoading}
					onclick={() => handleExport(format)}
					title="Export as {format}"
				>
					{#if isLoading}
						<span class="export-spinner"></span>
					{:else}
						<span class="export-icon">📥</span>
					{/if}
					{format}
				</button>
			{/each}
		</div>

		<div class="export-note">
			<p>Available formats:</p>
			<ul>
				<li><strong>GPX</strong> — GPS Exchange Format, compatible with most devices</li>
				<li><strong>GeoJSON</strong> — Standard web format with full trail data</li>
				<li><strong>KML</strong> — Google Earth format</li>
			</ul>
		</div>
	{:else}
		<div class="export-empty">
			<p>Select a trail to export</p>
		</div>
	{/if}
</div>

<style>
	.export-panel {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.export-header {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.export-header h3 {
		margin: 0;
		font-size: 1rem;
		font-weight: 700;
		color: v-bind('OUTDOOR_THEME.ink.strong');
	}

	.export-trail-name {
		margin: 0;
		font-size: 0.875rem;
		color: v-bind('OUTDOOR_THEME.ink.medium');
		font-weight: 500;
	}

	.export-formats {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
		gap: 0.5rem;
	}

	.export-button {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 0.75rem;
		border: 2px solid v-bind('OUTDOOR_THEME.border');
		border-radius: 6px;
		background: rgba(250, 247, 238, 0.95);
		color: v-bind('OUTDOOR_THEME.ink.strong');
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.export-button:hover:not(:disabled) {
		border-color: v-bind('OUTDOOR_THEME.brand');
		background: rgba(46, 102, 83, 0.05);
		transform: translateY(-2px);
		box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
	}

	.export-button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.export-icon {
		font-size: 1.1rem;
	}

	.export-spinner {
		display: inline-block;
		width: 1rem;
		height: 1rem;
		border: 2px solid rgba(46, 102, 83, 0.2);
		border-top-color: v-bind('OUTDOOR_THEME.brand');
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.export-note {
		padding: 0.75rem;
		background: rgba(46, 102, 83, 0.05);
		border-radius: 6px;
		border-left: 4px solid v-bind('OUTDOOR_THEME.brand');
	}

	.export-note p {
		margin: 0 0 0.5rem 0;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: v-bind('OUTDOOR_THEME.ink.medium');
	}

	.export-note ul {
		margin: 0;
		padding-left: 1.25rem;
		font-size: 0.8rem;
		color: v-bind('OUTDOOR_THEME.ink.medium');
		line-height: 1.4;
	}

	.export-note li {
		margin: 0.25rem 0;
	}

	.export-note strong {
		color: v-bind('OUTDOOR_THEME.ink.strong');
	}

	.export-empty {
		padding: 2rem 1rem;
		text-align: center;
		color: v-bind('OUTDOOR_THEME.ink.light');
		font-size: 0.875rem;
	}
</style>
