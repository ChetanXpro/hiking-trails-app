import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	// The repository intentionally shares one root .env across Docker, Rust,
	// and the frontend. Vite otherwise looks only inside web/.
	envDir: '..',
	plugins: [sveltekit()],
	resolve: {
		alias: {
			// `maplibre-contour@0.1.0` ships a malformed `exports` map.
			// Point Vite at its real ESM entry until the package is fixed upstream.
			'maplibre-contour': new URL(
				'./node_modules/maplibre-contour/dist/index.mjs',
				import.meta.url
			).pathname
		}
	},
	preview: {
		proxy: {
			'/api': {
				target: 'http://localhost:8080',
				rewrite: (path) => path.replace(/^\/api/, '')
			},
			'/trail_lines': {
				target: 'http://localhost:3000'
			},
			'/outdoor_waypoints': {
				target: 'http://localhost:3000'
			},
			'/terrain_rgb': {
				target: 'http://localhost:3000'
			}
		}
	},
	server: {
		proxy: {
			'/api': {
				target: 'http://localhost:8080',
				rewrite: (path) => path.replace(/^\/api/, '')
			},
			'/trail_lines': {
				target: 'http://localhost:3000'
			},
			'/outdoor_waypoints': {
				target: 'http://localhost:3000'
			},
			'/terrain_rgb': {
				target: 'http://localhost:3000'
			}
		}
	}
});
