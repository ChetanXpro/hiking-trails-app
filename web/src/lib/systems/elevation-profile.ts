export interface ElevationProfile {
	elevationProfile: Array<{ distance_m: number; elevation_m: number }>;
	elevationGain: number;
	elevationLoss: number;
	minElevation: number;
	maxElevation: number;
}

const DEFAULT_ELEVATION_SMOOTHING_WINDOW = 5;
const DEFAULT_GAIN_LOSS_THRESHOLD_M = 10.0;

export function smoothElevationData(
	data: Array<{ distance_m: number; elevation_m: number }>,
	windowSize: number = DEFAULT_ELEVATION_SMOOTHING_WINDOW
): Array<{ distance_m: number; elevation_m: number }> {
	if (data.length < 2) return data;

	const smoothed = [];
	for (let i = 0; i < data.length; i++) {
		const start = Math.max(0, i - windowSize);
		const end = Math.min(data.length, i + windowSize + 1);
		const window = data.slice(start, end);
		const avg = window.reduce((sum, p) => sum + p.elevation_m, 0) / window.length;
		smoothed.push({ distance_m: data[i].distance_m, elevation_m: avg });
	}
	return smoothed;
}

export function calculateGainLoss(
	elevationData: Array<{ distance_m: number; elevation_m: number }>,
	threshold: number = DEFAULT_GAIN_LOSS_THRESHOLD_M
): { gain: number; loss: number } {
	let gain = 0;
	let loss = 0;

	for (let i = 1; i < elevationData.length; i++) {
		const delta = elevationData[i].elevation_m - elevationData[i - 1].elevation_m;
		if (delta > threshold) gain += delta;
		else if (delta < -threshold) loss += -delta;
	}

	return { gain, loss };
}

export function getElevationStats(
	data: Array<{ distance_m: number; elevation_m: number }>
): Pick<ElevationProfile, 'minElevation' | 'maxElevation'> {
	if (data.length === 0) {
		return { minElevation: 0, maxElevation: 0 };
	}

	const elevations = data.map((p) => p.elevation_m);
	return {
		minElevation: Math.min(...elevations),
		maxElevation: Math.max(...elevations)
	};
}

export function findElevationAtDistance(
	data: Array<{ distance_m: number; elevation_m: number }>,
	distanceM: number
): number | null {
	if (data.length === 0) return null;

	// Find exact or interpolate
	for (let i = 0; i < data.length - 1; i++) {
		const current = data[i];
		const next = data[i + 1];
		if (distanceM >= current.distance_m && distanceM <= next.distance_m) {
			const ratio = (distanceM - current.distance_m) / (next.distance_m - current.distance_m);
			return (
				current.elevation_m + (next.elevation_m - current.elevation_m) * ratio
			);
		}
	}

	return null;
}
