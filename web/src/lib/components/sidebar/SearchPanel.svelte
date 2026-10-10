<script lang="ts">
	import { OUTDOOR_THEME } from '../../outdoor-theme';

	interface SearchResult {
		id: string;
		name: string;
		region?: string;
		elevation?: number;
		distance?: number;
	}

	interface Props {
		query?: string;
		results?: SearchResult[];
		isLoading?: boolean;
		onSearch?: (query: string) => void;
		onSelectResult?: (result: SearchResult) => void;
		placeholder?: string;
	}

	let {
		query = '',
		results = [],
		isLoading = false,
		onSearch,
		onSelectResult,
		placeholder = 'Search trails...'
	}: Props = $props();

	let searchInput: HTMLInputElement;

	function handleInput(e: Event) {
		const value = (e.target as HTMLInputElement).value;
		onSearch?.(value);
	}

	function handleResultClick(result: SearchResult) {
		onSelectResult?.(result);
	}
</script>

<div class="search-panel">
	<div class="search-input-wrapper">
		<input
			bind:this={searchInput}
			type="text"
			class="search-input"
			{placeholder}
			value={query}
			oninput={handleInput}
			disabled={isLoading}
		/>
		{#if isLoading}
			<span class="search-spinner"></span>
		{:else if query}
			<span class="search-icon">🔍</span>
		{/if}
	</div>

	{#if results.length > 0}
		<div class="search-results">
			<div class="results-header">
				<span class="results-count">{results.length} result{results.length === 1 ? '' : 's'}</span>
			</div>
			<ul class="results-list">
				{#each results as result (result.id)}
					<li class="result-item">
						<button
							class="result-button"
							onclick={() => handleResultClick(result)}
							type="button"
						>
							<div class="result-name">{result.name}</div>
							<div class="result-details">
								{#if result.region}
									<span class="result-region">{result.region}</span>
								{/if}
								{#if result.elevation}
									<span class="result-elevation">{result.elevation}m</span>
								{/if}
								{#if result.distance}
									<span class="result-distance">{result.distance.toFixed(1)} km</span>
								{/if}
							</div>
						</button>
					</li>
				{/each}
			</ul>
		</div>
	{:else if query && !isLoading}
		<div class="search-empty">
			<p>No trails found matching "{query}"</p>
		</div>
	{/if}
</div>

<style>
	.search-panel {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.search-input-wrapper {
		position: relative;
		display: flex;
		align-items: center;
	}

	.search-input {
		width: 100%;
		padding: 0.75rem 1rem 0.75rem 2.75rem;
		border: 2px solid v-bind('OUTDOOR_THEME.border');
		border-radius: 6px;
		background: white;
		font-size: 0.95rem;
		color: v-bind('OUTDOOR_THEME.ink.strong');
		transition: all 0.2s ease;
	}

	.search-input::placeholder {
		color: v-bind('OUTDOOR_THEME.ink.light');
	}

	.search-input:focus {
		outline: none;
		border-color: v-bind('OUTDOOR_THEME.brand');
		box-shadow: 0 0 0 3px rgba(46, 102, 83, 0.1);
	}

	.search-input:disabled {
		background: rgba(250, 247, 238, 0.6);
		cursor: not-allowed;
	}

	.search-icon,
	.search-spinner {
		position: absolute;
		left: 0.75rem;
		font-size: 1rem;
	}

	.search-spinner {
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

	.search-results {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-height: 400px;
		overflow-y: auto;
	}

	.results-header {
		padding: 0 0.5rem;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.results-count {
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: v-bind('OUTDOOR_THEME.ink.medium');
	}

	.results-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.result-item {
		margin: 0;
	}

	.result-button {
		width: 100%;
		padding: 0.75rem;
		border: 1px solid rgba(0, 0, 0, 0.08);
		border-radius: 6px;
		background: rgba(250, 247, 238, 0.95);
		color: inherit;
		text-align: left;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.result-button:hover {
		background: rgba(46, 102, 83, 0.08);
		border-color: v-bind('OUTDOOR_THEME.brand');
		transform: translateX(2px);
	}

	.result-name {
		font-weight: 600;
		color: v-bind('OUTDOOR_THEME.ink.strong');
		margin-bottom: 0.25rem;
		font-size: 0.95rem;
	}

	.result-details {
		display: flex;
		gap: 1rem;
		font-size: 0.8rem;
		color: v-bind('OUTDOOR_THEME.ink.medium');
	}

	.result-region,
	.result-elevation,
	.result-distance {
		display: inline-block;
	}

	.search-empty {
		padding: 2rem 1rem;
		text-align: center;
		color: v-bind('OUTDOOR_THEME.ink.light');
		font-size: 0.875rem;
	}
</style>
