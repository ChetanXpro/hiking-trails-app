import { afterEach, describe, expect, it, vi } from 'vitest';
import { requestBlob, RequestTimeoutError, requestJson } from './request';

describe('requestJson', () => {
	afterEach(() => {
		vi.useRealTimers();
		vi.unstubAllGlobals();
	});

	it('parses successful JSON responses', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn().mockResolvedValue(new Response(JSON.stringify({ ready: true }), { status: 200 }))
		);

		await expect(requestJson<{ ready: boolean }>('/health')).resolves.toEqual({
			ok: true,
			status: 200,
			data: { ready: true }
		});
	});

	it('does not try to parse non-success error bodies', async () => {
		vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('not json', { status: 503 })));

		await expect(requestJson('/health')).resolves.toEqual({
			ok: false,
			status: 503,
			data: null
		});
	});

	it('aborts requests that exceed their deadline', async () => {
		vi.useFakeTimers();
		vi.stubGlobal(
			'fetch',
			vi.fn((_input: RequestInfo | URL, init?: RequestInit) =>
				new Promise<Response>((_resolve, reject) => {
					init?.signal?.addEventListener('abort', () => {
						reject(new DOMException('Aborted', 'AbortError'));
					});
				})
		)
		);

		const request = requestJson('/slow', { timeoutMs: 250 });
		const rejection = expect(request).rejects.toBeInstanceOf(RequestTimeoutError);
		await vi.advanceTimersByTimeAsync(251);
		await rejection;
	});

	it('forwards caller cancellation', async () => {
		const controller = new AbortController();
		vi.stubGlobal(
			'fetch',
			vi.fn((_input: RequestInfo | URL, init?: RequestInit) =>
				new Promise<Response>((_resolve, reject) => {
					init?.signal?.addEventListener('abort', () => {
						reject(new DOMException('Aborted', 'AbortError'));
					});
				})
			)
		);

		const request = requestJson('/cancel', { signal: controller.signal });
		controller.abort();
		await expect(request).rejects.toMatchObject({ name: 'AbortError' });
	});

	it('reads binary downloads and preserves response headers', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn().mockResolvedValue(
				new Response('gpx-data', {
					status: 200,
					headers: { 'content-disposition': 'attachment; filename="route.gpx"' }
				})
			)
		);

		const response = await requestBlob('/download');
		expect(await response.data?.text()).toBe('gpx-data');
		expect(response.headers.get('content-disposition')).toContain('route.gpx');
	});

	it('cancels an in-flight binary download through the caller signal', async () => {
		const controller = new AbortController();
		vi.stubGlobal(
			'fetch',
			vi.fn((_input: RequestInfo | URL, init?: RequestInit) =>
				new Promise<Response>((_resolve, reject) => {
					init?.signal?.addEventListener('abort', () => {
						reject(new DOMException('Aborted', 'AbortError'));
					});
				})
			)
		);
		const request = requestBlob('/download', { signal: controller.signal });
		controller.abort();
		await expect(request).rejects.toMatchObject({ name: 'AbortError' });
	});
});
