import { requestJson } from './request';

export type LngLatTuple = [number, number];

export type WeatherCurrent = {
	temperatureC: number | null;
	apparentTemperatureC: number | null;
	precipitationMm: number | null;
	weatherCode: number | null;
	windSpeedKmh: number | null;
	windDirectionDeg: number | null;
	windGustKmh: number | null;
};

export type WeatherDay = {
	date: string;
	weatherCode: number | null;
	temperatureMaxC: number | null;
	temperatureMinC: number | null;
	precipitationProbabilityPct: number | null;
	precipitationMm: number | null;
	windSpeedMaxKmh: number | null;
	windGustMaxKmh: number | null;
};

export type WeatherForecast = {
	latitude: number | null;
	longitude: number | null;
	timezone: string;
	updatedAt: string;
	current: WeatherCurrent;
	days: WeatherDay[];
};

const CURRENT_FIELDS = [
	'temperature_2m',
	'apparent_temperature',
	'precipitation',
	'weather_code',
	'wind_speed_10m',
	'wind_direction_10m',
	'wind_gusts_10m'
];

const DAILY_FIELDS = [
	'weather_code',
	'temperature_2m_max',
	'temperature_2m_min',
	'precipitation_probability_max',
	'precipitation_sum',
	'wind_speed_10m_max',
	'wind_gusts_10m_max'
];

export const DEFAULT_WEATHER_API_URL = 'https://api.open-meteo.com/v1/forecast';

export function weatherForecastUrl(
	coordinates: LngLatTuple,
	endpoint = DEFAULT_WEATHER_API_URL
): string {
	const [longitude, latitude] = coordinates;
	const url = new URL(
		endpoint,
		typeof window === 'undefined' ? 'http://localhost/' : window.location.href
	);
	url.searchParams.set('latitude', latitude.toFixed(5));
	url.searchParams.set('longitude', longitude.toFixed(5));
	url.searchParams.set('current', CURRENT_FIELDS.join(','));
	url.searchParams.set('daily', DAILY_FIELDS.join(','));
	url.searchParams.set('timezone', 'auto');
	url.searchParams.set('forecast_days', '5');
	return url.toString();
}

function numberOrNull(value: unknown): number | null {
	return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function arrayAt(value: unknown): unknown[] {
	return Array.isArray(value) ? value : [];
}

export function parseWeatherForecast(payload: unknown): WeatherForecast {
	if (!payload || typeof payload !== 'object') throw new Error('Weather service returned invalid data');
	const root = payload as Record<string, unknown>;
	const current = (root.current ?? {}) as Record<string, unknown>;
	const daily = (root.daily ?? {}) as Record<string, unknown>;
	const dates = arrayAt(daily.time)
		.map((date, index) => ({ date, index }))
		.filter(
			(entry): entry is { date: string; index: number } =>
				typeof entry.date === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(entry.date)
		);
	if (dates.length === 0 || typeof current.time !== 'string') {
		throw new Error('Weather forecast is unavailable for this location');
	}

	const dayValue = (field: string, index: number) => arrayAt(daily[field])[index];
	return {
		latitude: numberOrNull(root.latitude),
		longitude: numberOrNull(root.longitude),
		timezone:
			typeof root.timezone === 'string' && root.timezone.trim()
				? root.timezone
				: 'Local time',
		updatedAt: current.time,
		current: {
			temperatureC: numberOrNull(current.temperature_2m),
			apparentTemperatureC: numberOrNull(current.apparent_temperature),
			precipitationMm: numberOrNull(current.precipitation),
			weatherCode: numberOrNull(current.weather_code),
			windSpeedKmh: numberOrNull(current.wind_speed_10m),
			windDirectionDeg: numberOrNull(current.wind_direction_10m),
			windGustKmh: numberOrNull(current.wind_gusts_10m)
		},
		days: dates.map(({ date, index }) => ({
			date,
			weatherCode: numberOrNull(dayValue('weather_code', index)),
			temperatureMaxC: numberOrNull(dayValue('temperature_2m_max', index)),
			temperatureMinC: numberOrNull(dayValue('temperature_2m_min', index)),
			precipitationProbabilityPct: numberOrNull(
				dayValue('precipitation_probability_max', index)
			),
			precipitationMm: numberOrNull(dayValue('precipitation_sum', index)),
			windSpeedMaxKmh: numberOrNull(dayValue('wind_speed_10m_max', index)),
			windGustMaxKmh: numberOrNull(dayValue('wind_gusts_10m_max', index))
		}))
	};
}

export async function fetchWeatherForecast(
	coordinates: LngLatTuple,
	signal?: AbortSignal,
	timeoutMs = 10_000,
	endpoint = DEFAULT_WEATHER_API_URL
): Promise<WeatherForecast> {
	const response = await requestJson<unknown>(weatherForecastUrl(coordinates, endpoint), {
		signal,
		timeoutMs
	});
	if (!response.ok) throw new Error(`Weather request failed with ${response.status}`);
	return parseWeatherForecast(response.data);
}

export function weatherCondition(code: number | null): { label: string; icon: string } {
	if (code == null || !Number.isFinite(code) || code < 0) {
		return { label: 'Unavailable', icon: '—' };
	}
	if (code === 0) return { label: 'Clear', icon: '☀' };
	if (code <= 2) return { label: 'Partly cloudy', icon: '◒' };
	if (code === 3) return { label: 'Overcast', icon: '☁' };
	if (code === 45 || code === 48) return { label: 'Fog', icon: '≋' };
	if (code >= 51 && code <= 57) return { label: 'Drizzle', icon: '☂' };
	if (code >= 61 && code <= 67) return { label: 'Rain', icon: '☂' };
	if (code >= 71 && code <= 77) return { label: 'Snow', icon: '❄' };
	if (code >= 80 && code <= 82) return { label: 'Showers', icon: '☔' };
	if (code >= 85 && code <= 86) return { label: 'Snow showers', icon: '❄' };
	if (code >= 95) return { label: 'Thunderstorm', icon: 'ϟ' };
	return { label: 'Variable', icon: '◌' };
}

export function compassDirection(degrees: number | null): string {
	if (degrees == null || !Number.isFinite(degrees)) return '—';
	const directions = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW'];
	const normalized = ((degrees % 360) + 360) % 360;
	return directions[Math.round(normalized / 45) % directions.length];
}
