<script lang="ts">
	import { compassDirection, weatherCondition, type WeatherForecast } from './weather';

	let {
		forecast,
		status,
		error = null,
		title = 'Trail weather',
		compact = false,
		onrefresh
	}: {
		forecast: WeatherForecast | null;
		status: 'idle' | 'loading' | 'ready' | 'error';
		error?: string | null;
		title?: string;
		compact?: boolean;
		onrefresh?: () => void;
	} = $props();

	function dayLabel(date: string, index: number) {
		if (index === 0) return 'Today';
		return new Intl.DateTimeFormat(undefined, { weekday: 'short' }).format(
			new Date(`${date}T12:00:00`)
		);
	}

	function rounded(value: number | null, suffix = '') {
		return value == null ? '—' : `${Math.round(value)}${suffix}`;
	}

	function decimal(value: number | null, suffix = '') {
		return value == null ? '—' : `${value.toFixed(1)}${suffix}`;
	}

	function windLabel(direction: number | null, speed: number | null) {
		if (direction == null && speed == null) return '—';
		const bearing = compassDirection(direction);
		const velocity = speed == null ? null : `${Math.round(speed)} km/h`;
		return [bearing === '—' ? null : bearing, velocity].filter(Boolean).join(' ');
	}
</script>

<section class:compact class="weather-card" aria-busy={status === 'loading'}>
	<header>
		<div>
			<span class="weather-kicker">Outdoor forecast</span>
			<h3>{title}</h3>
		</div>
		{#if onrefresh}
			<button type="button" onclick={onrefresh} disabled={status === 'loading'} aria-label="Refresh weather">
				↻
			</button>
		{/if}
	</header>

	{#if status === 'loading'}
		<div class="weather-loading">
			<span></span><span></span><span></span>
		</div>
	{:else if status === 'error'}
		<div class="weather-message">
			<strong>Forecast unavailable</strong>
			<p>{error ?? 'Weather data could not be loaded right now.'}</p>
		</div>
	{:else if forecast}
		{@const condition = weatherCondition(forecast.current.weatherCode)}
		<div class="weather-now">
			<span class="weather-icon" aria-hidden="true">{condition.icon}</span>
			<div>
				<strong>{rounded(forecast.current.temperatureC, '°')}</strong>
				<span>{condition.label}</span>
			</div>
			<div class="weather-wind">
				<strong>{windLabel(forecast.current.windDirectionDeg, forecast.current.windSpeedKmh)}</strong>
				<span>{forecast.current.windGustKmh == null ? 'Gust unavailable' : `gust ${Math.round(forecast.current.windGustKmh)} km/h`}</span>
			</div>
		</div>

		<div class="weather-metrics">
			<div><span>Feels</span><strong>{rounded(forecast.current.apparentTemperatureC, '°')}</strong></div>
			<div><span>Rain now</span><strong>{decimal(forecast.current.precipitationMm, ' mm')}</strong></div>
			<div><span>Local time</span><strong>{forecast.timezone.split('/').pop()?.replaceAll('_', ' ')}</strong></div>
		</div>

		<div class="weather-days">
			{#each forecast.days.slice(0, compact ? 3 : 5) as day, index}
				{@const dayCondition = weatherCondition(day.weatherCode)}
				<div class="weather-day">
					<span>{dayLabel(day.date, index)}</span>
					<b aria-label={dayCondition.label}>{dayCondition.icon}</b>
					<strong>{rounded(day.temperatureMaxC, '°')} <small>{rounded(day.temperatureMinC, '°')}</small></strong>
					<em>{rounded(day.precipitationProbabilityPct, '%')} · {rounded(day.windGustMaxKmh, ' km/h')}</em>
				</div>
			{/each}
		</div>
	{:else}
		<div class="weather-message">
			<strong>Choose a location</strong>
			<p>Select a trail or center the map, then refresh.</p>
		</div>
	{/if}

	<footer>
		<span>Forecasts can differ sharply by elevation and slope.</span>
		<a href="https://open-meteo.com/" target="_blank" rel="noreferrer">Open-Meteo</a>
	</footer>
</section>

<style>
	.weather-card {
		display: grid;
		gap: 0.85rem;
		padding: 0.95rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 18px;
		background: linear-gradient(145deg, rgba(42, 59, 51, 0.96), rgba(25, 36, 30, 0.96));
		box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.035);
		color: var(--ui-text, #faf7ee);
	}

	header,
	.weather-now,
	footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
	}

	h3 {
		margin: 0.08rem 0 0;
		font-size: 1rem;
	}

	.weather-kicker {
		color: var(--ui-focus, #79c69b);
		font-size: 0.68rem;
		font-weight: 800;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	header button {
		width: 2rem;
		height: 2rem;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 50%;
		background: rgba(255, 255, 255, 0.06);
		color: inherit;
		cursor: pointer;
	}

	.weather-now {
		justify-content: flex-start;
		padding: 0.2rem 0;
	}

	.weather-icon {
		font-size: 2rem;
		line-height: 1;
	}

	.weather-now > div:nth-child(2) {
		display: grid;
	}

	.weather-now > div:nth-child(2) strong {
		font-size: 1.8rem;
		line-height: 1;
	}

	.weather-now span,
	.weather-wind span,
	.weather-metrics span,
	footer {
		color: var(--ui-muted, #aab3ae);
		font-size: 0.72rem;
	}

	.weather-wind {
		display: grid;
		margin-left: auto;
		text-align: right;
	}

	.weather-metrics {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 0.45rem;
	}

	.weather-metrics > div {
		display: grid;
		gap: 0.15rem;
		padding: 0.55rem;
		border-radius: 11px;
		background: rgba(255, 255, 255, 0.045);
	}

	.weather-metrics strong {
		overflow: hidden;
		font-size: 0.78rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.weather-days {
		display: grid;
		grid-template-columns: repeat(5, minmax(0, 1fr));
		gap: 0.35rem;
	}

	.weather-day {
		display: grid;
		justify-items: center;
		gap: 0.2rem;
		min-width: 0;
		padding: 0.55rem 0.2rem;
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 11px;
		background: rgba(255, 255, 255, 0.035);
		text-align: center;
	}

	.weather-day > span,
	.weather-day em {
		color: var(--ui-muted, #aab3ae);
		font-size: 0.64rem;
		font-style: normal;
	}

	.weather-day b {
		font-size: 1rem;
	}

	.weather-day strong {
		font-size: 0.78rem;
		white-space: nowrap;
	}

	.weather-day small {
		color: var(--ui-muted, #aab3ae);
		font-size: inherit;
		font-weight: 600;
	}

	.weather-day em {
		overflow: hidden;
		max-width: 100%;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.weather-message {
		padding: 0.8rem;
		border-radius: 12px;
		background: rgba(255, 255, 255, 0.04);
	}

	.weather-message p {
		margin: 0.25rem 0 0;
		color: var(--ui-muted, #aab3ae);
		font-size: 0.78rem;
		line-height: 1.4;
	}

	.weather-loading {
		display: grid;
		gap: 0.45rem;
	}

	.weather-loading span {
		height: 2.4rem;
		border-radius: 10px;
		background: linear-gradient(90deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0.1), rgba(255, 255, 255, 0.04));
		background-size: 200% 100%;
		animation: weather-shimmer 1.2s linear infinite;
	}

	footer {
		align-items: flex-start;
		padding-top: 0.1rem;
		line-height: 1.35;
	}

	footer a {
		color: var(--ui-focus, #79c69b);
		white-space: nowrap;
	}

	.compact .weather-days {
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}

	@keyframes weather-shimmer {
		to {
			background-position: -200% 0;
		}
	}

	@media (max-width: 420px) {
		.weather-days {
			grid-template-columns: repeat(3, minmax(0, 1fr));
		}

		.weather-day:nth-child(n + 4) {
			display: none;
		}
	}
</style>
