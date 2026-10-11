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
