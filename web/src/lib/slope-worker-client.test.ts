import { describe, expect, it } from 'vitest';
import {
	SlopeWorkerPool,
	slopeWorkerConcurrency,
	type SlopeWorkerLike,
	type SlopeWorkerRequest,
	type SlopeWorkerResponse
} from './slope-worker-client';

class FakeWorker implements SlopeWorkerLike {
	onmessage: ((event: MessageEvent<SlopeWorkerResponse>) => void) | null = null;
	onerror: ((event: ErrorEvent) => void) | null = null;
	requests: SlopeWorkerRequest[] = [];
	terminated = false;

	postMessage(message: SlopeWorkerRequest): void {
		this.requests.push(message);
	}

	terminate(): void {
		this.terminated = true;
	}

	complete(index = 0): void {
		const request = this.requests[index];
		this.onmessage?.({ data: { id: request.id, data: new ArrayBuffer(8) } } as MessageEvent);
	}

	fail(message = 'worker failed'): void {
		this.onerror?.({ message } as ErrorEvent);
	}
}

function input(): Omit<SlopeWorkerRequest, 'id'> {
	return {
		zoom: 13,
		tileY: 1600,
		width: 2,
		height: 2,
		paddedBuffer: new Float32Array(16).buffer
	};
}

describe('SlopeWorkerPool', () => {
	it('bounds concurrency and rejects work beyond the queue limit', async () => {
		const workers: FakeWorker[] = [];
		const pool = new SlopeWorkerPool({
			concurrency: 1,
			maxQueue: 1,
			workerFactory: () => {
				const worker = new FakeWorker();
				workers.push(worker);
				return worker;
			}
		});
		const first = pool.classify(input());
		const second = pool.classify(input());
		await expect(pool.classify(input())).rejects.toThrow('queue is full');
		expect(pool.activeCount).toBe(1);
		expect(pool.queuedCount).toBe(1);

		workers[0].complete();
		await expect(first).resolves.toHaveProperty('byteLength', 8);
		expect(workers[0].requests).toHaveLength(2);
		workers[0].complete(1);
		await expect(second).resolves.toHaveProperty('byteLength', 8);
		pool.dispose();
	});

	it('removes queued aborts without consuming worker capacity', async () => {
		const worker = new FakeWorker();
		const pool = new SlopeWorkerPool({
			concurrency: 1,
			maxQueue: 2,
			workerFactory: () => worker
		});
		const first = pool.classify(input());
		const controller = new AbortController();
		const queued = pool.classify(input(), controller.signal);
		controller.abort();
		await expect(queued).rejects.toMatchObject({ name: 'AbortError' });
		expect(pool.queuedCount).toBe(0);
		worker.complete();
		await first;
		pool.dispose();
	});

	it('terminates active work on abort and dispatches queued work to a replacement', async () => {
		const workers: FakeWorker[] = [];
		const pool = new SlopeWorkerPool({
			concurrency: 1,
			maxQueue: 2,
			workerFactory: () => {
				const worker = new FakeWorker();
				workers.push(worker);
				return worker;
			}
		});
		const controller = new AbortController();
		const active = pool.classify(input(), controller.signal);
		const next = pool.classify(input());
		controller.abort();
		await expect(active).rejects.toMatchObject({ name: 'AbortError' });
		expect(workers[0].terminated).toBe(true);
		expect(workers[1].requests).toHaveLength(1);
		workers[1].complete();
		await next;
		pool.dispose();
	});

	it('uses at most two workers and one on lower-core devices', () => {
		expect(slopeWorkerConcurrency(2)).toBe(1);
		expect(slopeWorkerConcurrency(4)).toBe(1);
		expect(slopeWorkerConcurrency(8)).toBe(2);
		expect(slopeWorkerConcurrency(undefined)).toBe(2);
	});

	it('disposes all workers after a module/runtime error instead of respawning forever', async () => {
		const workers: FakeWorker[] = [];
		const pool = new SlopeWorkerPool({
			concurrency: 2,
			workerFactory: () => {
				const worker = new FakeWorker();
				workers.push(worker);
				return worker;
			}
		});
		const first = pool.classify(input());
		const second = pool.classify(input());
		const firstResult = expect(first).rejects.toThrow('blocked by worker-src');
		const secondResult = expect(second).rejects.toMatchObject({ name: 'AbortError' });
		workers[0].fail('blocked by worker-src');
		await firstResult;
		await secondResult;
		expect(workers).toHaveLength(2);
		expect(workers.every((worker) => worker.terminated)).toBe(true);
	});
});
