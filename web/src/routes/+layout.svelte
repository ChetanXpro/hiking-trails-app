<script lang="ts">
	let { children } = $props();
</script>

<div class="app-shell">
	{@render children()}
</div>

<style>
	:global(html), :global(body) {
		margin: 0;
		height: 100%;
	}
	.app-shell {
		height: 100vh;
		width: 100vw;
	}
</style>
