<script lang="ts">
	import { OUTDOOR_THEME } from '../../outdoor-theme';

	interface Props {
		query?: string;
		panelOpen?: boolean;
		isLoading?: boolean;
		isError?: boolean;
		errorMessage?: string | null;
		searchResults?: any[];
		searchRoutes?: any[];
		searchPlaces?: any[];
		activeSearchIndex?: number;
		mapReady?: boolean;
		minChars?: number;
		detailPanelOpen?: boolean;
		onInput?: (event: Event) => void;
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
				oninput={onInput}
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
									{#if result.breadcrumb?.length > 0}
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
