import type {PhotoDensePreset, PhotoDiagnostics, PhotoPixels, PhotoReconstruction, PhotoSurface} from './photogrammetryKernel'

export interface PhotoWorkerRequest {
  /** Kernel module compiled once on the main thread; the Worker only instantiates it. */
  module: WebAssembly.Module
  /** Try the WebGPU depth sweep first (baseline dense preset only); the CPU path
   * remains the fallback and the reference. */
  gpu?: boolean
  images: PhotoPixels[]
  dense: boolean
  resolution: number
  densePreset?: PhotoDensePreset
}

export interface PhotoTimings {
  preparationMs?: number
  sparseMs: number
  denseMs: number
}

export type PhotoWorkerEvent =
  | {type: 'stage'; stage: 'calibration' | 'cameras' | 'depth'}
  | {type: 'sparse'; result: PhotoReconstruction}
  | {type: 'surface'; result: PhotoSurface}
  | {type: 'warning'; message: string}
  | {type: 'error'; message: string; diagnostics?: PhotoDiagnostics}
  | {type: 'done'; timings: PhotoTimings}
