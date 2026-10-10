export type SlopeWorkerRequest = {
	id: number;
	zoom: number;
	tileY: number;
	width: number;
	height: number;
	paddedBuffer: ArrayBuffer;
};

export type SlopeWorkerResponse = {
	id: number;
	data?: ArrayBuffer;
	error?: string;
};

export type SlopeClassificationInput = Omit<SlopeWorkerRequest, 'id'>;

export interface SlopeWorkerLike {
	onmessage: ((event: MessageEvent<SlopeWorkerResponse>) => void) | null;
	onerror: ((event: ErrorEvent) => void) | null;
	postMessage(message: SlopeWorkerRequest, transfer: Transferable[]): void;
	terminate(): void;
}

type SlopeTask = {
	id: number;
	input: SlopeClassificationInput;
	signal?: AbortSignal;
	abortListener?: () => void;
	resolve: (data: ArrayBuffer) => void;
	reject: (error: Error) => void;
	state: 'queued' | 'active' | 'settled';
};

type WorkerSlot = {
	worker: SlopeWorkerLike;
	task?: SlopeTask;
};

export type SlopeWorkerPoolOptions = {
	concurrency?: number;
	maxQueue?: number;
	workerFactory?: () => SlopeWorkerLike;
};

function abortError(): DOMException {
	return new DOMException('Slope tile request aborted', 'AbortError');
}

export function slopeWorkerConcurrency(hardwareConcurrency?: number): number {
	if (hardwareConcurrency != null && hardwareConcurrency <= 4) return 1;
	return 2;
}

/**
 * Small cancellation-aware pool for CPU-heavy slope classification. Active
 * aborts replace the worker because synchronous pixel classification cannot
 * observe a cancel message until after it finishes.
 */
export class SlopeWorkerPool {
	readonly maxQueue: number;
	readonly concurrency: number;
	private readonly workerFactory: () => SlopeWorkerLike;
	private readonly slots: WorkerSlot[] = [];
	private readonly queue: SlopeTask[] = [];
	private disposed = false;
	private nextId = 1;

	constructor(options: SlopeWorkerPoolOptions = {}) {
		this.concurrency = Math.min(2, Math.max(1, Math.floor(options.concurrency ?? 1)));
		this.maxQueue = Math.min(32, Math.max(1, Math.floor(options.maxQueue ?? 12)));
		this.workerFactory =
			options.workerFactory ??
			(() =>
				new Worker(new URL('./slope-worker.ts', import.meta.url), {
					type: 'module',
					name: 'hiking-slope-classifier'
				}) as SlopeWorkerLike);
		try {
			for (let index = 0; index < this.concurrency; index += 1) {
				this.slots.push(this.spawnSlot());
			}
		} catch (error) {
			for (const slot of this.slots) slot.worker.terminate();
			throw error;
		}
	}

	get queuedCount(): number {
		return this.queue.length;
	}

	get activeCount(): number {
		return this.slots.filter((slot) => slot.task).length;
	}

	classify(input: SlopeClassificationInput, signal?: AbortSignal): Promise<ArrayBuffer> {
		if (this.disposed) return Promise.reject(new Error('Slope worker pool is disposed'));
		if (signal?.aborted) return Promise.reject(abortError());
		if (this.queue.length >= this.maxQueue && this.activeCount >= this.concurrency) {
			return Promise.reject(new Error('Slope worker queue is full'));
		}

		return new Promise<ArrayBuffer>((resolve, reject) => {
			const task: SlopeTask = {
				id: this.nextId++,
				input,
				signal,
				resolve,
				reject,
				state: 'queued'
			};
			task.abortListener = () => this.abortTask(task);
			signal?.addEventListener('abort', task.abortListener, { once: true });
			this.queue.push(task);
			this.dispatch();
		});
	}

	dispose(): void {
		if (this.disposed) return;
		this.disposed = true;
		for (const task of this.queue.splice(0)) this.settle(task, undefined, abortError());
		for (const slot of this.slots) {
			slot.worker.terminate();
			if (slot.task) this.settle(slot.task, undefined, abortError());
			slot.task = undefined;
		}
	}

	private spawnSlot(): WorkerSlot {
		const slot = {} as WorkerSlot;
		this.configureWorker(slot);
		return slot;
	}

	private configureWorker(slot: WorkerSlot): void {
		slot.worker = this.workerFactory();
		slot.worker.onmessage = (event) => this.completeSlot(slot, event.data);
		slot.worker.onerror = (event) => {
			this.failSlot(slot, new Error(event.message || 'Slope classification worker failed'));
		};
	}

	private replaceWorker(slot: WorkerSlot): void {
		slot.worker.terminate();
		if (this.disposed) return;
		try {
			this.configureWorker(slot);
		} catch {
			this.dispose();
		}
	}

	private dispatch(): void {
		if (this.disposed) return;
		for (const slot of this.slots) {
			if (slot.task) continue;
			let task = this.queue.shift();
			while (task?.state === 'settled') task = this.queue.shift();
			if (!task) return;
			task.state = 'active';
			slot.task = task;
			const request: SlopeWorkerRequest = { id: task.id, ...task.input };
			try {
				slot.worker.postMessage(request, [request.paddedBuffer]);
			} catch (error) {
				slot.task = undefined;
				this.settle(
					task,
					undefined,
					error instanceof Error ? error : new Error('Could not queue slope classification')
				);
				this.dispatch();
				return;
			}
		}
	}

	private completeSlot(slot: WorkerSlot, response: SlopeWorkerResponse): void {
		const task = slot.task;
		if (!task || task.id !== response.id) return;
		slot.task = undefined;
		if (response.data) {
			this.settle(task, response.data);
		} else {
			this.settle(task, undefined, new Error(response.error ?? 'Slope classification failed'));
		}
		this.dispatch();
	}

	private failSlot(slot: WorkerSlot, error: Error): void {
		const task = slot.task;
		slot.task = undefined;
		if (task) this.settle(task, undefined, error);
		// A Worker error usually means its module could not load (for example a
		// worker-src CSP violation). Disable the pool instead of respawning forever.
		this.dispose();
	}

	private abortTask(task: SlopeTask): void {
		if (task.state === 'settled') return;
		if (task.state === 'queued') {
			const index = this.queue.indexOf(task);
			if (index >= 0) this.queue.splice(index, 1);
			this.settle(task, undefined, abortError());
			return;
		}

		const slot = this.slots.find((candidate) => candidate.task === task);
		if (!slot) return;
		slot.task = undefined;
		this.settle(task, undefined, abortError());
		this.replaceWorker(slot);
		this.dispatch();
	}

	private settle(task: SlopeTask, data?: ArrayBuffer, error?: Error): void {
		if (task.state === 'settled') return;
		task.state = 'settled';
		if (task.abortListener) task.signal?.removeEventListener('abort', task.abortListener);
		if (error) task.reject(error);
		else task.resolve(data ?? new ArrayBuffer(0));
	}
}

export function createSlopeWorkerPool(): SlopeWorkerPool {
	const hardwareConcurrency =
		typeof navigator === 'undefined' ? undefined : navigator.hardwareConcurrency;
	return new SlopeWorkerPool({ concurrency: slopeWorkerConcurrency(hardwareConcurrency) });
}
