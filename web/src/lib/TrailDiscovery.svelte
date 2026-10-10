<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { isAbortError, requestJson } from './request';
	let { bounds, hidden = false, onselect, onhover }: {
		bounds: [number, number, number, number] | null;
		hidden?: boolean;
		onselect: (id: string) => void;
		onhover: (id: string | null) => void;
	} = $props();
	type Trail = { id: string; name: string | null; distance_m: number | null;
		ascent_m: number | null; difficulty: string | null; hike_type: string | null;
		region_name: string | null; incomplete: boolean };
	let open = $state(false);
	let trails: Trail[] = $state([]);
	let maxDistance = $state('');
	let maxAscent = $state('');
	let difficulty = $state('all');
	let loops = $state(false);
	let sort = $state('nearest');
	let loading = $state(false);
	let error = $state('');
	let searchedBounds = $state('');
	let nextOffset: number | null = $state(null);
	let controller: AbortController | null = null;
	const dirty = $derived(bounds != null && bounds.join(',') !== searchedBounds);
	$effect(() => { if (open && bounds && !searchedBounds) untrack(() => void search()); });
	onDestroy(() => controller?.abort());
	async function search(more = false) {
		if (!bounds) return;
		controller?.abort();
		const request = new AbortController();
		controller = request;
		const bbox = more ? searchedBounds : bounds.join(',');
		const params = new URLSearchParams({ bbox, difficulty, loops_only: String(loops), sort });
		if (maxDistance) params.set('max_distance_km', maxDistance);
		if (maxAscent) params.set('max_ascent_m', maxAscent);
		if (more && nextOffset != null) params.set('offset', String(nextOffset));
		loading = true; error = '';
		if (!more) nextOffset = null;
		try {
			const response = await requestJson<{ trails: Trail[]; next_offset: number | null }>(`/api/trails?${params}`, { signal: request.signal, timeoutMs: 12_000 });
			if (request.signal.aborted) return;
			if (!response.ok || !response.data) throw new Error('Couldn’t load hikes. Try again.');
			trails = more ? [...trails, ...response.data.trails] : response.data.trails;
			nextOffset = response.data.next_offset; searchedBounds = bbox;
		} catch (e) { if (!isAbortError(e) && !request.signal.aborted) error = e instanceof Error ? e.message : 'Couldn’t load hikes.'; }
		finally { if (controller === request) loading = false; }
	}
	function rating(value: string | null) {
		return ({ hiking: 'Hiking · T1', mountain_hiking: 'Mountain · T2', demanding_mountain_hiking: 'Mountain · T3', alpine_hiking: 'Alpine · T4', demanding_alpine_hiking: 'Alpine · T5', difficult_alpine_hiking: 'Alpine · T6' } as Record<string,string>)[value ?? ''] ?? 'Difficulty unknown';
	}
</script>

<aside class="discovery" class:expanded={open} class:hidden aria-label="Discover hikes">
	<button class="toggle" type="button" aria-expanded={open} onclick={() => open = !open}>
		<span>⌕ &nbsp; Explore hikes in this area</span><span aria-hidden="true">{open ? '−' : '+'}</span>
	</button>
	{#if open}
		<div class="content">
			<div class="filters">
				<label>Distance<select bind:value={maxDistance} onchange={() => search()}><option value="">Any length</option><option value="5">Up to 5 km</option><option value="10">Up to 10 km</option><option value="20">Up to 20 km</option><option value="40">Up to 40 km</option></select></label>
				<label>Climbing<select bind:value={maxAscent} onchange={() => search()}><option value="">Any ascent</option><option value="300">Up to 300 m</option><option value="600">Up to 600 m</option><option value="1000">Up to 1,000 m</option></select></label>
				<label>Difficulty<select bind:value={difficulty} onchange={() => search()}><option value="all">All difficulties</option><option value="hiking">Hiking (T1)</option><option value="mountain">Mountain (T2–T3)</option><option value="alpine">Alpine (T4–T6)</option><option value="unrated">Unknown</option></select></label>
				<label>Sort by<select bind:value={sort} onchange={() => search()}><option value="nearest">Near map center</option><option value="shortest">Shortest first</option><option value="longest">Longest first</option><option value="ascent">Least climbing</option><option value="name">Name</option></select></label>
			</div>
			<div class="area-actions"><label class="checkbox"><input type="checkbox" bind:checked={loops} onchange={() => search()} />Loops only</label><button class="search-area" type="button" disabled={!bounds || loading} onclick={() => search()}>{loading ? 'Searching…' : dirty ? 'Search this area' : 'Refresh results'}</button></div>
			{#if maxAscent}<p class="note">Hikes with unknown ascent are excluded.</p>{:else if difficulty !== 'all' && difficulty !== 'unrated'}<p class="note">Hikes with unknown difficulty are excluded.</p>{/if}
			<div class="results" aria-busy={loading}>
				{#if error}<p role="alert">{error}</p><button type="button" onclick={() => search()}>Try again</button>
				{:else if trails.length === 0 && !loading && searchedBounds}<p>No mapped hikes match. Widen the map or clear the filters.</p>
				{:else}
					{#each trails as trail (trail.id)}
						<button class="trail" type="button" onclick={() => { onhover(null); onselect(trail.id); }} onpointerenter={() => onhover(trail.id)} onpointerleave={() => onhover(null)} onfocus={() => onhover(trail.id)} onblur={() => onhover(null)}>
							<strong>{trail.name ?? 'Unnamed trail'}</strong>
							<span>{trail.distance_m == null ? 'Length unknown' : `${(trail.distance_m / 1000).toFixed(1)} km`}{trail.ascent_m == null ? ' · Ascent unknown' : ` · ↗ ${Math.round(trail.ascent_m)} m`}{trail.hike_type === 'loop' ? ' · Loop' : ''}</span>
							<small>{rating(trail.difficulty)}{trail.region_name ? ` · ${trail.region_name}` : ''}</small>
							{#if trail.incomplete}<small class="incomplete">Partial route · mapped sections only</small>{/if}
						</button>
					{/each}
				{/if}
				{#if nextOffset != null}<button class="more" type="button" disabled={loading || dirty} onclick={() => search(true)}>Show more hikes</button>{/if}
			</div>
			<p class="note" role="status">{loading ? 'Finding hikes…' : `${trails.length}${nextOffset != null ? '+' : ''} hikes shown`}{dirty && searchedBounds ? ' · Map moved; search again to update.' : ''}</p>
		</div>
	{/if}
</aside>

<style>
	.discovery{position:absolute;top:4.7rem;left:1rem;width:min(370px,calc(100vw - 2rem));z-index:10;background:#f8f6ed;color:#253a31;border:1px solid #d4d9ca;border-radius:16px;box-shadow:0 10px 32px #173d2922;font:inherit;overflow:hidden}
	.hidden{display:none}.toggle{display:flex;justify-content:space-between;width:100%;border:0;background:transparent;padding:14px 16px;font-weight:700;color:inherit;text-align:left;cursor:pointer}.content{padding:0 14px 10px}.filters{display:grid;grid-template-columns:1fr 1fr;gap:10px}label{font-size:11px;font-weight:700;color:#5a6c60}select{display:block;width:100%;margin-top:4px;padding:8px 5px;background:#fffdf6;border:1px solid #cbd4c5;border-radius:7px;font:inherit;font-size:12px;color:#253a31}.area-actions{display:flex;align-items:center;justify-content:space-between;gap:8px;margin:12px 0}.checkbox{display:flex;align-items:center;gap:6px;font-size:12px}.search-area,.more{border:0;border-radius:8px;padding:9px 12px;background:#2e6653;color:#fff;font-weight:650;cursor:pointer}button:disabled{opacity:.5;cursor:default}.results{max-height:calc(100dvh - 380px);min-height:30px;overflow:auto;overscroll-behavior:contain}.trail{display:flex;flex-direction:column;gap:5px;width:100%;padding:13px 3px;border:0;border-top:1px solid #dce2d5;text-align:left;background:transparent;color:inherit;cursor:pointer}.trail:hover,.trail:focus-visible{background:#e8eedf}.trail strong{font-size:14px}.trail span{font-size:12px}.trail small{font-size:11px;color:#647267}.trail .incomplete{color:#875017}.note{font-size:11px;line-height:1.5;color:#677466;margin:9px 0 0}.more{width:100%;margin:8px 0}button:focus-visible,select:focus-visible{outline:2px solid #2e6653;outline-offset:2px}
	@media(max-width:740px){.discovery{top:auto;bottom:.75rem;left:.75rem;width:calc(100vw - 1.5rem)}.discovery:not(.expanded){width:calc(100vw - 11rem)}.results{max-height:23dvh}.content{max-height:55dvh;overflow:auto}}
</style>
