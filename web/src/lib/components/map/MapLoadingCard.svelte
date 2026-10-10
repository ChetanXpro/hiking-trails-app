<script lang="ts">
	interface Props {
		isReady?: boolean;
		error?: string | null;
	}

	let { isReady = false, error = null }: Props = $props();
</script>

<div class:ready={isReady} class="map-loading" role="status" aria-hidden={isReady} aria-live="polite">
	<div class="map-loading-card">
		{#if error}
			<span class="map-loading-mark error" aria-hidden="true">!</span>
			<strong>Outdoor map did not finish loading</strong>
			<span>{error}</span>
			<button type="button" onclick={() => window.location.reload()}>Try again</button>
		{:else}
			<span class="map-loading-mark" aria-hidden="true">△</span>
			<strong>Preparing outdoor map</strong>
			<span>Terrain, trails and elevation</span>
		{/if}
	</div>
</div>

<style>
	.map-loading {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(250, 247, 238, 0.96);
		backdrop-filter: blur(1px);
		pointer-events: none;
		opacity: 1;
		transition: opacity 0.4s ease;
		z-index: 5;
	}

	.map-loading.ready {
		opacity: 0;
		pointer-events: none;
	}

	.map-loading-card {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		align-items: center;
		text-align: center;
		padding: 2rem;
		background: white;
		border-radius: 12px;
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.07);
	}

	.map-loading-mark {
		font-size: 2rem;
		font-weight: 700;
		color: rgba(46, 102, 83, 0.5);
	}

	.map-loading-mark.error {
		color: #d32f2f;
	}

	.map-loading-card strong {
		font-size: 1.1rem;
		font-weight: 600;
		color: rgba(0, 0, 0, 0.8);
	}

	.map-loading-card span {
		font-size: 0.9rem;
		color: rgba(0, 0, 0, 0.6);
	}

	.map-loading-card button {
		margin-top: 0.5rem;
		padding: 0.5rem 1.25rem;
		background: rgba(46, 102, 83, 0.1);
		border: 1px solid rgba(46, 102, 83, 0.2);
		border-radius: 6px;
		color: rgba(46, 102, 83, 0.8);
		cursor: pointer;
		font-size: 0.9rem;
		font-weight: 500;
		transition: all 0.2s ease;
	}

	.map-loading-card button:hover {
		background: rgba(46, 102, 83, 0.15);
		border-color: rgba(46, 102, 83, 0.3);
	}
</style>
