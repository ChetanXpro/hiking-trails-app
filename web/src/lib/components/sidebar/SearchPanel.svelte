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

<style>
	.search-dock {
		position: absolute;
		top: 20px;
		left: 20px;
		z-index: 15;
		pointer-events: none;
	}

	.search-dock.panel-open {
		/* Don't block map - keep search card position */
	}

	.search-card {
		position: relative;
		width: 360px;
		background: rgba(250, 247, 238, 0.96);
		border: 1px solid rgba(0, 0, 0, 0.08);
		border-radius: 8px;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
		pointer-events: auto;
		transition: all 0.2s ease;
	}

	.search-card.open {
		border-radius: 8px 8px 0 0;
		box-shadow: 0 8px 16px rgba(0, 0, 0, 0.12);
	}

	.search-input-wrap {
		position: relative;
		display: flex;
		align-items: center;
		padding: 12px 16px;
		border-bottom: 1px solid rgba(0, 0, 0, 0.06);
	}

	.search-icon {
		position: absolute;
		left: 16px;
		font-size: 1.2rem;
		color: rgba(0, 0, 0, 0.4);
		pointer-events: none;
	}

	.search-input {
		flex: 1;
		padding: 8px 12px 8px 36px;
		border: none;
		background: transparent;
		font-size: 0.95rem;
		color: rgba(0, 0, 0, 0.8);
		outline: none;
		font-family: inherit;
	}

	.search-input::placeholder {
		color: rgba(0, 0, 0, 0.4);
	}

	.search-input:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.search-clear {
		background: none;
		border: none;
		padding: 4px 8px;
		font-size: 1.5rem;
		color: rgba(0, 0, 0, 0.4);
		cursor: pointer;
		transition: color 0.2s;
	}

	.search-clear:hover {
		color: rgba(0, 0, 0, 0.7);
	}

	.search-results {
		max-height: 480px;
		overflow-y: auto;
		border-top: 1px solid rgba(0, 0, 0, 0.06);
	}

	.search-hint,
	.search-error {
		padding: 16px;
		margin: 0;
		text-align: center;
		font-size: 0.9rem;
		color: rgba(0, 0, 0, 0.5);
	}

	.search-error {
		color: #c62828;
		background: rgba(198, 40, 40, 0.05);
	}

	.search-result-list {
		display: flex;
		flex-direction: column;
	}

	.search-group-label {
		padding: 8px 16px;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: rgba(0, 0, 0, 0.4);
		border-top: 1px solid rgba(0, 0, 0, 0.06);
		margin-top: 8px;
	}

	.search-result {
		padding: 12px 16px;
		border: none;
		background: transparent;
		text-align: left;
		cursor: pointer;
		transition: background 0.2s;
		border-bottom: 1px solid rgba(0, 0, 0, 0.04);
		color: inherit;
		font-family: inherit;
	}

	.search-result:hover,
	.search-result.active {
		background: rgba(46, 102, 83, 0.08);
	}

	.search-result-topline {
		display: flex;
		gap: 8px;
		align-items: center;
		margin-bottom: 4px;
		font-size: 0.8rem;
	}

	.search-kind-pill {
		display: inline-block;
		padding: 2px 6px;
		background: rgba(46, 102, 83, 0.15);
		color: rgba(46, 102, 83, 0.8);
		border-radius: 3px;
		font-weight: 500;
		font-size: 0.7rem;
		text-transform: uppercase;
	}

	.search-kind-pill.region-pill {
		background: rgba(66, 133, 244, 0.15);
		color: rgba(66, 133, 244, 0.8);
	}

	.search-network,
	.search-distance {
		font-size: 0.8rem;
		color: rgba(0, 0, 0, 0.5);
	}

	.search-result strong {
		display: block;
		font-weight: 600;
		color: rgba(0, 0, 0, 0.8);
		margin-bottom: 2px;
	}

	.search-result p {
		margin: 0;
		font-size: 0.8rem;
		color: rgba(0, 0, 0, 0.5);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
