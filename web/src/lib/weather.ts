export type LngLatTuple = [number, number];

export type WeatherCurrent = {
	temperatureC: number;
	apparentTemperatureC: number;
	precipitationMm: number;
	weatherCode: number;
	windSpeedKmh: number;
	windDirectionDeg: number;
	windGustKmh: number;
};

export type WeatherDay = {
	date: string;
	weatherCode: number;
	temperatureMaxC: number;
	temperatureMinC: number;
	precipitationProbabilityPct: number;
	precipitationMm: number;
	windSpeedMaxKmh: number;
	windGustMaxKmh: number;
};

export type WeatherForecast = {
	latitude: number;
	longitude: number;
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

export function weatherForecastUrl(coordinates: LngLatTuple): string {
	const [longitude, latitude] = coordinates;
	const url = new URL('https://api.open-meteo.com/v1/forecast');
	url.searchParams.set('latitude', latitude.toFixed(5));
	url.searchParams.set('longitude', longitude.toFixed(5));
	url.searchParams.set('current', CURRENT_FIELDS.join(','));
	url.searchParams.set('daily', DAILY_FIELDS.join(','));
	url.searchParams.set('timezone', 'auto');
	url.searchParams.set('forecast_days', '5');
	return url.toString();
}

function numberAt(value: unknown, fallback = 0): number {
	return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

function arrayAt(value: unknown): unknown[] {
	return Array.isArray(value) ? value : [];
}

export function parseWeatherForecast(payload: unknown): WeatherForecast {
	if (!payload || typeof payload !== 'object') throw new Error('Weather service returned invalid data');
	const root = payload as Record<string, unknown>;
	const current = (root.current ?? {}) as Record<string, unknown>;
	const daily = (root.daily ?? {}) as Record<string, unknown>;
	const dates = arrayAt(daily.time);
	if (dates.length === 0 || typeof current.time !== 'string') {
		throw new Error('Weather forecast is unavailable for this location');
	}

	const dayValue = (field: string, index: number) => arrayAt(daily[field])[index];
	return {
		latitude: numberAt(root.latitude),
		longitude: numberAt(root.longitude),
		timezone: typeof root.timezone === 'string' ? root.timezone : 'Local time',
		updatedAt: current.time,
		current: {
			temperatureC: numberAt(current.temperature_2m),
			apparentTemperatureC: numberAt(current.apparent_temperature),
			precipitationMm: numberAt(current.precipitation),
			weatherCode: numberAt(current.weather_code),
			windSpeedKmh: numberAt(current.wind_speed_10m),
			windDirectionDeg: numberAt(current.wind_direction_10m),
			windGustKmh: numberAt(current.wind_gusts_10m)
		},
		days: dates.map((date, index) => ({
			date: String(date),
			weatherCode: numberAt(dayValue('weather_code', index)),
			temperatureMaxC: numberAt(dayValue('temperature_2m_max', index)),
			temperatureMinC: numberAt(dayValue('temperature_2m_min', index)),
			precipitationProbabilityPct: numberAt(
				dayValue('precipitation_probability_max', index)
			),
			precipitationMm: numberAt(dayValue('precipitation_sum', index)),
			windSpeedMaxKmh: numberAt(dayValue('wind_speed_10m_max', index)),
			windGustMaxKmh: numberAt(dayValue('wind_gusts_10m_max', index))
		}))
	};
}

export async function fetchWeatherForecast(
	coordinates: LngLatTuple,
	signal?: AbortSignal
): Promise<WeatherForecast> {
	const response = await fetch(weatherForecastUrl(coordinates), { signal });
	if (!response.ok) throw new Error(`Weather request failed with ${response.status}`);
	return parseWeatherForecast(await response.json());
}

export function weatherCondition(code: number): { label: string; icon: string } {
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

export function compassDirection(degrees: number): string {
	const directions = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW'];
	const normalized = ((degrees % 360) + 360) % 360;
	return directions[Math.round(normalized / 45) % directions.length];
}
