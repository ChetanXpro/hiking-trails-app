<script lang="ts">
	import { OUTDOOR_THEME } from '../../outdoor-theme';

	interface Props {
		query?: string;
		panelOpen?: boolean;
		isLoading?: boolean;
		isError?: boolean;
		errorMessage?: string;
		searchResults?: any[];
		searchRoutes?: any[];
		searchPlaces?: any[];
		activeSearchIndex?: number;
		mapReady?: boolean;
		minChars?: number;
		detailPanelOpen?: boolean;
		onInput?: (value: string) => void;
		onFocus?: () => void;
		onKeydown?: (event: KeyboardEvent) => void;
		onSelectResult?: (result: any) => void;
		onReset?: () => void;
	}

	let {
		query = '',
		panelOpen = false,
		isLoading = false,
		isError = false,
		errorMessage = '',
		searchResults = [],
		searchRoutes = [],
		searchPlaces = [],
		activeSearchIndex = 0,
		mapReady = false,
		minChars = 3,
		detailPanelOpen = false,
		onInput,
		onFocus,
		onKeydown,
		onSelectResult,
		onReset
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

<div class:panel-open={detailPanelOpen} class="search-dock">
	<div class:open={panelOpen} class="search-card">
		<div class="search-input-wrap">
			<span class="search-icon" aria-hidden="true">⌕</span>
			<input
				class="search-input"
				aria-label="Search trails or places"
				type="search"
				placeholder={mapReady ? 'Search trails or places…' : 'Preparing map search…'}
				disabled={!mapReady}
				value={query}
				oninput={(e) => onInput?.((e.target as HTMLInputElement).value)}
				onfocus={onFocus}
				onkeydown={onKeydown}
			/>
			{#if query}
				<button class="search-clear" type="button" aria-label="Clear search" onclick={onReset}>
					×
				</button>
			{/if}
		</div>

		{#if panelOpen}
			<div class="search-results">
				{#if query.trim().length < minChars}
					<p class="search-hint">Type at least {minChars} characters to search trails and areas.</p>
				{:else if isLoading}
					<p class="search-hint">Searching the trail index…</p>
				{:else if isError}
					<p class="search-error">{errorMessage}</p>
				{:else if searchResults.length === 0}
					<p class="search-hint">No matches yet. Try a trail name or a place like Manali.</p>
				{:else}
					<div class="search-result-list">
						{#if searchRoutes.length > 0}
							<div class="search-group-label">Trails</div>
							{#each searchRoutes as result}
								<button
									class:active={searchResults[activeSearchIndex]?.id === result.id}
									class="search-result"
									type="button"
									onmouseenter={() => { /* handled by parent */ }}
									onfocus={() => { /* handled by parent */ }}
									onclick={() => onSelectResult?.(result)}
								>
									<div class="search-result-topline">
										<span class="search-kind-pill">Trail</span>
										{#if result.network}
											<span class="search-network">{result.network}</span>
										{/if}
										{#if result.distance_m != null}
											<span class="search-distance">{(result.distance_m / 1000).toFixed(1)} km</span>
										{/if}
									</div>
									<strong>{result.name}</strong>
									{#if result.subtitle}
										<p>{result.subtitle}</p>
									{/if}
								</button>
							{/each}
						{/if}

						{#if searchPlaces.length > 0}
							<div class="search-group-label">Places</div>
							{#each searchPlaces as result}
								<button
									class:active={searchResults[activeSearchIndex]?.id === result.id}
									class="search-result"
									type="button"
									onmouseenter={() => { /* handled by parent */ }}
									onfocus={() => { /* handled by parent */ }}
									onclick={() => onSelectResult?.(result)}
								>
									<div class="search-result-topline">
										<span class:region-pill={result.kind === 'region'} class="search-kind-pill">
											{result.kind === 'region' ? 'Area' : 'Place'}
										</span>
										{#if result.admin_label}
											<span class="search-network">{result.admin_label}</span>
										{:else if result.place_kind}
											<span class="search-network">{result.place_kind}</span>
										{/if}
									</div>
									<strong>{result.name}</strong>
									{#if result.breadcrumb.length > 0}
										<p>{result.breadcrumb.join(', ')}</p>
									{/if}
								</button>
							{/each}
						{/if}
					</div>
				{/if}
			</div>
		{/if}
	</div>
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
