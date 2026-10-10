/// <reference lib="webworker" />

import { renderPaddedSlopeTileImageData } from './slope-config';
import type { SlopeWorkerRequest, SlopeWorkerResponse } from './slope-worker-client';

const workerScope = self as unknown as DedicatedWorkerGlobalScope;

async function encodePng(
	rgba: Uint8ClampedArray,
	width: number,
	height: number
): Promise<ArrayBuffer> {
	const canvas = new OffscreenCanvas(width, height);
	const context = canvas.getContext('2d');
	if (!context) throw new Error('Slope overlay requires a 2D OffscreenCanvas context');
	const image = new ImageData(width, height);
	image.data.set(rgba);
	context.putImageData(image, 0, 0);
	const blob = await canvas.convertToBlob({ type: 'image/png' });
	return blob.arrayBuffer();
}

workerScope.onmessage = async (event: MessageEvent<SlopeWorkerRequest>) => {
	const request = event.data;
	try {
		const rgba = renderPaddedSlopeTileImageData(
			{
				width: request.width,
				height: request.height,
				data: new Float32Array(request.paddedBuffer)
			},
			request.zoom,
			request.tileY
		);
		const data = await encodePng(rgba, request.width, request.height);
		const response: SlopeWorkerResponse = { id: request.id, data };
		workerScope.postMessage(response, [data]);
	} catch (error) {
		const response: SlopeWorkerResponse = {
			id: request.id,
			error: error instanceof Error ? error.message : 'Slope tile classification failed'
		};
		workerScope.postMessage(response);
	}
};

