import adapter from '@sveltejs/adapter-node';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		// Production runs as a small Node service behind Caddy. Using the
		// explicit adapter keeps builds reproducible instead of relying on
		// adapter-auto to guess the deployment platform.
		adapter: adapter()
	}
};

export default config;
