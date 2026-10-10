export class RequestTimeoutError extends Error {
	constructor(timeoutMs: number) {
		super(`Request timed out after ${Math.max(1, Math.ceil(timeoutMs / 1000))} seconds`);
		this.name = 'RequestTimeoutError';
	}
}

export type JsonResponse<T> = {
	ok: boolean;
	status: number;
	data: T | null;
};

export type BlobResponse = {
	ok: boolean;
	status: number;
	data: Blob | null;
	headers: Headers;
};

/**
 * Fetches and parses a JSON response while keeping the timeout active until the
 * body is consumed. The caller's signal is forwarded without losing the local
 * timeout, and all listeners/timers are removed when the request settles.
 */
async function requestData<T>(
	input: RequestInfo | URL,
	init: RequestInit & { timeoutMs?: number },
	readBody: (response: Response) => Promise<T>
): Promise<{ ok: boolean; status: number; data: T | null; headers: Headers }> {
	const { timeoutMs = 12_000, signal: callerSignal, ...requestInit } = init;
	const controller = new AbortController();
	let timedOut = false;
	const abortFromCaller = () => controller.abort();

	if (callerSignal?.aborted) {
		controller.abort();
	} else {
		callerSignal?.addEventListener('abort', abortFromCaller, { once: true });
	}

	const timeout = setTimeout(() => {
		timedOut = true;
		controller.abort();
	}, Math.max(1, timeoutMs));

	try {
		const response = await fetch(input, { ...requestInit, signal: controller.signal });
		let data: T | null = null;
		if (response.ok) {
			data = await readBody(response);
		} else if (response.body) {
			await response.body.cancel().catch(() => undefined);
		}
		return { ok: response.ok, status: response.status, data, headers: response.headers };
	} catch (error) {
		if (timedOut) throw new RequestTimeoutError(timeoutMs);
		throw error;
	} finally {
		clearTimeout(timeout);
		callerSignal?.removeEventListener('abort', abortFromCaller);
	}
}

export async function requestJson<T>(
	input: RequestInfo | URL,
	init: RequestInit & { timeoutMs?: number } = {}
): Promise<JsonResponse<T>> {
	const response = await requestData(input, init, async (value) => (await value.json()) as T);
	return { ok: response.ok, status: response.status, data: response.data };
}

export async function requestBlob(
	input: RequestInfo | URL,
	init: RequestInit & { timeoutMs?: number } = {}
): Promise<BlobResponse> {
	return requestData(input, init, (response) => response.blob());
}

export function isAbortError(error: unknown): boolean {
	return error instanceof DOMException
		? error.name === 'AbortError'
		: error instanceof Error && error.name === 'AbortError';
}
