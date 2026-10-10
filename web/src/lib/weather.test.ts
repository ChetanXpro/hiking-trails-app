import { describe, expect, it } from 'vitest';
import {
	compassDirection,
	parseWeatherForecast,
	weatherCondition,
	weatherForecastUrl
} from './weather';

describe('weather helpers', () => {
	it('requests an outdoor-focused five-day forecast in local time', () => {
		const url = new URL(weatherForecastUrl([77.18, 32.24]));
		expect(url.hostname).toBe('api.open-meteo.com');
		expect(url.searchParams.get('timezone')).toBe('auto');
		expect(url.searchParams.get('forecast_days')).toBe('5');
		expect(url.searchParams.get('current')).toContain('wind_gusts_10m');
		expect(url.searchParams.get('daily')).toContain('precipitation_probability_max');
	});

	it('supports a deployment-owned weather endpoint', () => {
		const url = new URL(weatherForecastUrl([77.18, 32.24], 'https://weather.example.com/v1'));
		expect(url.origin).toBe('https://weather.example.com');
		expect(url.searchParams.get('latitude')).toBe('32.24000');
	});

	it('parses current conditions and daily mountain-relevant values', () => {
		const forecast = parseWeatherForecast({
			latitude: 32.24,
			longitude: 77.18,
			timezone: 'Asia/Kolkata',
			current: {
				time: '2026-07-11T23:30',
				temperature_2m: 12,
				apparent_temperature: 10,
				precipitation: 0.2,
				weather_code: 61,
				wind_speed_10m: 14,
				wind_direction_10m: 45,
				wind_gusts_10m: 27
			},
			daily: {
				time: ['2026-07-11'],
				weather_code: [61],
				temperature_2m_max: [15],
				temperature_2m_min: [7],
				precipitation_probability_max: [70],
				precipitation_sum: [5.2],
				wind_speed_10m_max: [19],
				wind_gusts_10m_max: [35]
			}
		});

		expect(forecast.current.windGustKmh).toBe(27);
		expect(forecast.days[0].precipitationProbabilityPct).toBe(70);
	});

	it('translates weather and wind codes into concise map labels', () => {
		expect(weatherCondition(0).label).toBe('Clear');
		expect(weatherCondition(95).label).toBe('Thunderstorm');
		expect(weatherCondition(null).label).toBe('Unavailable');
		expect(compassDirection(225)).toBe('SW');
		expect(compassDirection(null)).toBe('—');
	});

	it('preserves missing values as unavailable rather than false zero readings', () => {
		const forecast = parseWeatherForecast({
			current: { time: '2026-07-11T23:30', weather_code: 0 },
			daily: { time: ['2026-07-11'], weather_code: [0] }
		});

		expect(forecast.current.temperatureC).toBeNull();
		expect(forecast.current.windSpeedKmh).toBeNull();
		expect(forecast.days[0].precipitationProbabilityPct).toBeNull();
		expect(forecast.current.weatherCode).toBe(0);
	});
});
