import {runGpuSweep} from '../services/photoGpuSweep'
import {PhotogrammetryKernel, type PhotoDensePreset, type PhotoDiagnostics, type PhotoSurface} from '../services/photogrammetryKernel'
import type {PhotoTimings, PhotoWorkerEvent, PhotoWorkerRequest} from '../services/photoWorkerProtocol'

const scope = self as unknown as {postMessage: (event: PhotoWorkerEvent, transfer: Transferable[]) => void}
const post = (event: PhotoWorkerEvent, transfer: Transferable[] = []) => scope.postMessage(event, transfer)
const errorMessage = (error: unknown) => error instanceof Error ? error.message : String(error)

/** Geometry buffers move to the main thread by transfer; a structured clone would copy them again. */
function transferList(result: PhotoSurface): Transferable[] {
  const buffers: Transferable[] = []
  for (const surface of [result, result.documentSurface]) {
    if (!surface) continue
    for (const array of [surface.positions, surface.colors, surface.triangles]) {
      if (ArrayBuffer.isView(array)) buffers.push(array.buffer as ArrayBuffer)
    }
  }
  return buffers
}

/** Compaction is an optional CAD export; its failure must preserve the full surface. */
function publishSurface(kernel: PhotogrammetryKernel, result: PhotoSurface): void {
  try {
    result.documentSurface = kernel.compact()
  } catch (error) {
    post({type: 'warning', message: errorMessage(error)})
  }
  post({type: 'surface', result}, transferList(result))
}

function reconstructSurface(kernel: PhotogrammetryKernel, resolution: number, preset?: PhotoDensePreset): void {
  try {
    publishSurface(kernel, preset === undefined ? kernel.dense(resolution) : kernel.dense(resolution, preset))
  } catch (error) {
    post({type: 'warning', message: errorMessage(error)})
  }
}

/** Dense via the browser WebGPU sweep; any failure falls back to the CPU path. */
async function reconstructSurfaceGpu(kernel: PhotogrammetryKernel, resolution: number): Promise<boolean> {
  const prepared = kernel.densePrepare(resolution)
  if (!prepared) return false
  const scores = await runGpuSweep(prepared.payload, prepared.wgsl)
  publishSurface(kernel, kernel.denseFinish(scores))
  return true
}

// Returns the promise so tests (and careful hosts) can await the async GPU stretch.
self.onmessage = (event: MessageEvent<PhotoWorkerRequest>) => run(event.data)

// The CPU path contains no awaits, so it still runs fully synchronously; the
// WebGPU branch is the only asynchronous stretch.
async function run(data: PhotoWorkerRequest): Promise<void> {
  let kernel: PhotogrammetryKernel | undefined
  const timings: PhotoTimings = {sparseMs: 0, denseMs: 0}
  try {
    kernel = new PhotogrammetryKernel(data.module)
    if (data.images.some(image => image.calibration)) post({type: 'stage', stage: 'calibration'})
    const calibrationStart = performance.now()
    for (const image of data.images) kernel.add(image)
    timings.preparationMs = performance.now() - calibrationStart

    post({type: 'stage', stage: 'cameras'})
    const sparseStart = performance.now()
    const result = kernel.sparse()
    timings.sparseMs = performance.now() - sparseStart
    post({type: 'sparse', result}, transferList(result))

    if (data.dense) {
      post({type: 'stage', stage: 'depth'})
      const denseStart = performance.now()
      const baseline = data.densePreset === undefined || data.densePreset === 'baseline'
      const gpu = data.gpu && baseline
        ? await reconstructSurfaceGpu(kernel, data.resolution).catch(error => {
            post({type: 'warning', message: `WebGPU sweep unavailable: ${errorMessage(error)}`})
            return false
          })
        : false
      if (!gpu) reconstructSurface(kernel, data.resolution, data.densePreset)
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
