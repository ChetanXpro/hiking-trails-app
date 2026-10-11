<script lang="ts">
	import { nearestSampleIndex } from './profile-index';
	type Point = { pos_m: number; ele_m: number };
	let { points, onpreview }: { points: Point[]; onpreview: (point: Point | null) => void } = $props();
	let active: number | null = $state(null);
	const width = 360, height = 150, left = 10, right = 10, top = 24, bottom = 24;
	const total = $derived(points.at(-1)?.pos_m ?? 0);
	const extent = $derived.by(() => {
		let min = Infinity, max = -Infinity;
		for (const point of points) { min = Math.min(min, point.ele_m); max = Math.max(max, point.ele_m); }
		return { min, max };
	});
	const min = $derived(extent.min);
	const max = $derived(extent.max);
	const x = (pos: number) => left + pos / Math.max(1, total) * (width - left - right);
	const y = (ele: number) => top + (max - ele) / Math.max(1, max - min) * (height - top - bottom);
	const path = $derived(points.map((p, i) => `${i ? 'L' : 'M'}${x(p.pos_m).toFixed(1)},${y(p.ele_m).toFixed(1)}`).join(' '));
	const focus = $derived(active == null ? null : points[active]);
	function preview(index: number | null) { active = index; onpreview(index == null ? null : points[index]); }
	function move(event: PointerEvent) {
		const rect = (event.currentTarget as SVGElement).getBoundingClientRect();
		const fraction = ((event.clientX - rect.left) / rect.width * width - left) / (width - left - right);
		preview(nearestSampleIndex(points, Math.max(0, Math.min(1, fraction)) * total));
	}
	const distance = (m: number) => m < 1000 ? `${Math.round(m)} m` : `${(m / 1000).toFixed(1)} km`;
</script>

{#if points.length > 1}
	<div class="profile">
		<div class="readout"><strong>{focus ? `${distance(focus.pos_m)} · ${Math.round(focus.ele_m)} m` : 'Elevation along the route'}</strong><span>{Math.round(min)}–{Math.round(max)} m</span></div>
		<svg viewBox={`0 0 ${width} ${height}`} role="img" aria-label={`Elevation profile: ${distance(total)}, from ${Math.round(min)} to ${Math.round(max)} metres`} onpointermove={move} onpointerleave={() => preview(null)}>
			{#each [min, (min + max) / 2, max] as elevation}
				<line x1={left} x2={width - right} y1={y(elevation)} y2={y(elevation)} class="grid" />
			{/each}
			<path d={`${path} L${x(total)},${height - bottom} L${left},${height - bottom} Z`} class="area" />
			<path d={path} class="line" />
			{#if focus}<line x1={x(focus.pos_m)} x2={x(focus.pos_m)} y1={top} y2={height-bottom} class="cursor" /><circle cx={x(focus.pos_m)} cy={y(focus.ele_m)} r="4" />{/if}
			{#each [0, .25, .5, .75, 1] as fraction}<text x={x(total*fraction)} y={height-5} text-anchor={fraction === 0 ? 'start' : fraction === 1 ? 'end' : 'middle'}>{distance(total*fraction)}</text>{/each}
		</svg>
		<label class="slider-label">Explore elevation on the map
			<input type="range" min="0" max={points.length-1} step="1" value={active ?? 0} aria-label="Position along elevation profile" aria-valuetext={focus ? `${distance(focus.pos_m)}, elevation ${Math.round(focus.ele_m)} metres` : `Start, elevation ${Math.round(points[0].ele_m)} metres`} oninput={event => preview(Number(event.currentTarget.value))} onblur={() => preview(null)} />
		</label>
		<div class="endpoints"><span>Start <b>{Math.round(points[0].ele_m)} m</b></span><span>Highest <b>{Math.round(max)} m</b></span><span>Finish <b>{Math.round(points.at(-1)!.ele_m)} m</b></span></div>
	</div>
{/if}

<style>
	.profile{padding:12px;border-radius:14px;background:#ffffff08}.readout{display:flex;justify-content:space-between;gap:10px;font-size:11px;min-height:16px}.readout span{color:#b5c7bc;white-space:nowrap}svg{display:block;width:100%;overflow:visible;touch-action:pan-y}.grid{stroke:#ffffff17;stroke-width:1}.area{fill:#63b48a20}.line{stroke:#90d9ae;stroke-width:2;fill:none;stroke-linejoin:round}.cursor{stroke:#edf7d4;stroke-dasharray:3 3}circle{fill:#edf7d4;stroke:#2e6653;stroke-width:2}text{fill:#a9beb1;font-size:10px}.slider-label{display:block;font-size:10px;color:#b5c7bc}input{width:100%;accent-color:#90d9ae;min-height:25px;margin:0}.endpoints{display:flex;justify-content:space-between;font-size:11px;color:#b5c7bc;margin-top:6px}.endpoints b{display:block;color:#eff5e9;margin-top:3px}
</style>
