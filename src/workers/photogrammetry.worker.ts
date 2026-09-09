import {PhotogrammetryKernel, type PhotoDensePreset, type PhotoDiagnostics} from '../services/photogrammetryKernel'
import type {PhotoTimings, PhotoWorkerEvent, PhotoWorkerRequest} from '../services/photoWorkerProtocol'

const post = (event: PhotoWorkerEvent) => self.postMessage(event)
const errorMessage = (error: unknown) => error instanceof Error ? error.message : String(error)

/** Compaction is an optional CAD export; its failure must preserve the full surface. */
function reconstructSurface(kernel: PhotogrammetryKernel, resolution: number, preset?: PhotoDensePreset): void {
  try {
    const result = preset === undefined ? kernel.dense(resolution) : kernel.dense(resolution, preset)
    try {
      result.documentSurface = kernel.compact()
    } catch (error) {
      post({type: 'warning', message: errorMessage(error)})
    }
    post({type: 'surface', result})
  } catch (error) {
    post({type: 'warning', message: errorMessage(error)})
  }
}

self.onmessage = (event: MessageEvent<PhotoWorkerRequest>) => {
  let kernel: PhotogrammetryKernel | undefined
  const timings: PhotoTimings = {sparseMs: 0, denseMs: 0}
  try {
    kernel = new PhotogrammetryKernel()
    if (event.data.images.some(image => image.calibration)) post({type: 'stage', stage: 'calibration'})
    const calibrationStart = performance.now()
    for (const image of event.data.images) kernel.add(image)
    timings.preparationMs = performance.now() - calibrationStart

    post({type: 'stage', stage: 'cameras'})
    const sparseStart = performance.now()
    const result = kernel.sparse()
    timings.sparseMs = performance.now() - sparseStart
    post({type: 'sparse', result})

    if (event.data.dense) {
      post({type: 'stage', stage: 'depth'})
      const denseStart = performance.now()
      reconstructSurface(kernel, event.data.resolution, event.data.densePreset)
      timings.denseMs = performance.now() - denseStart
    }
    post({type: 'done', timings})
  } catch (error) {
    let diagnostics: PhotoDiagnostics | undefined
    try {
      diagnostics = kernel?.report() ?? undefined
    } catch {
      // Preserve the original error if report transport also fails.
    }
    post({type: 'error', message: errorMessage(error), diagnostics})
  } finally {
    try {
      kernel?.clear()
    } catch {
      // The UI terminates this disposable Worker; cleanup cannot replace its outcome.
    }
  }
}
