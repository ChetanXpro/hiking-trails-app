import type maplibregl from 'maplibre-gl';

export function toggleTerrain3d(
	map: maplibregl.Map | undefined,
	isEnabled: boolean,
	onToggle: (enabled: boolean) => void
) {
	if (!map) return;
	const newState = !isEnabled;
	onToggle(newState);

	map.setMaxPitch(85);
	if (newState) {
		map.easeTo({
			pitch: Math.max(map.getPitch(), 64),
			bearing: Math.abs(map.getBearing()) < 2 ? -24 : map.getBearing(),
			duration: 750
		});
	} else {
		map.easeTo({ pitch: 0, bearing: 0, duration: 650 });
	}
}

export function setMapPitch(
	map: maplibregl.Map | undefined,
	value: number,
	onPitchChange: (value: number) => void,
	on3dToggle?: (enabled: boolean) => void
) {
	if (!map) return;
	const pitch = Math.max(0, Math.min(82, value));
	if (pitch > 0 && on3dToggle) on3dToggle(true);
	map.easeTo({ pitch, duration: 180 });
	onPitchChange(pitch);
}

export function setMapBearing(
	map: maplibregl.Map | undefined,
	value: number,
	onBearingChange: (value: number) => void
) {
	if (!map) return;
	map.easeTo({ bearing: value, duration: 180 });
	onBearingChange(value);
}

export function rotateMap(
	map: maplibregl.Map | undefined,
	delta: number,
	onBearingChange: (value: number) => void
) {
	if (!map) return;
	const newBearing = (map.getBearing() + delta) % 360;
	map.easeTo({ bearing: newBearing, duration: 260 });
	onBearingChange(newBearing);
}

export function set3DViewPreset(
	map: maplibregl.Map | undefined,
	preset: 'terrain' | 'horizon',
	onToggle: (enabled: boolean) => void
) {
	if (!map) return;
	onToggle(true);
	map.setMaxPitch(85);
	map.easeTo({
		pitch: preset === 'horizon' ? 78 : 62,
		bearing: Math.abs(map.getBearing()) < 2 ? -28 : map.getBearing(),
		duration: 750
	});
}
