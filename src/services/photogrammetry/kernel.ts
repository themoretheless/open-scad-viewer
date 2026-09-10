import type {PhotoCalibrationGroup, PhotoMeasuredInput} from './calibration'
import {encodeBinary, type BinaryTripleHints} from '../valueBinaryCodec'
import {decodePacked, writeLinear} from '../wasmHost'

/** Geometry fields written by the Rust adapter as numeric triples decode into flat typed arrays. */
const SURFACE_TRIPLES: BinaryTripleHints = {positions: 'f64', colors: 'u8', triangles: 'u32'}

export interface PhotoPixels {
  width: number
  height: number
  focal: number
  rgb: Uint8Array
  calibration?: PhotoMeasuredInput
}

export interface PhotoImageReport {
  image: number
  features: number
  candidateCorrespondences: number
  acceptedObservations: number
  conflictingMatches: number
  poseAttempts: number
  registered: boolean
  reason: string
}

export interface PhotoSeedTrialReport {
  pair: [number, number]
  registeredImages: number
  points: number
  reprojectionRmse: number | null
  error: string | null
}

export interface PhotoBundleReport {
  initialCost: number
  finalCost: number
  iterations: number
  acceptedSteps: number
  observations: number
}

export interface PhotoCalibrationProvenance {
  image: number
  mode: 'focal-hint' | 'measured-brown'
  group?: PhotoCalibrationGroup
  sourceSize?: [number, number]
  inputSize: [number, number]
  output: {width: number; height: number; focal: number; cx: number; cy: number}
  zoom?: number
  outputFovDegrees?: [number, number]
  resampled: boolean
  borderPolicy?: 'fully-valid'
}

export interface PhotoDiagnostics {
  calibrations?: PhotoCalibrationProvenance[]
  images: PhotoImageReport[]
  initialPair: number[] | null
  seedPairsTested: number
  seedTrials?: PhotoSeedTrialReport[]
  matchingRequests: number
  computedPairs: number
  bundleRuns: PhotoBundleReport[]
  warnings: string[]
}

export type PhotoDensePreset = 'baseline' | 'slanted-plane' | 'dual-scale-volume'

export interface PhotoDenseDiagnostics {
  preset?: PhotoDensePreset
  patchRadius?: number
  depthHypotheses?: number
  evaluatedHypotheses?: number
  evaluatedSourcePatches?: number
  sampledSourcePixels?: number
  estimatedMaps: number
  selectedSourcePairs: number
  photometricSamples: number
  consistentSamples: number
  rejectedInconsistentSamples: number
  fusedSamples: number
  vertices: number
  triangles: number
  viewReports: {
    image: number
    sourceImages: number[]
    photometricSamples: number
    consistentSamples: number
  }[]
}

export interface PhotoSurface {
  denseDiagnostics?: PhotoDenseDiagnostics
  positions: Float64Array
  colors: Uint8Array
  triangles: Uint32Array
  documentSurface?: PhotoSurface
}

export interface PhotoCamera {
  image: number
  rotation: number[][]
  translation: number[]
  focal: number
  cx: number
  cy: number
}

export interface PhotoReconstruction extends PhotoSurface {
  diagnostics?: PhotoDiagnostics
  cameras: PhotoCamera[]
  inputImages: number
  reprojectionRmse: number
}

interface Exports extends WebAssembly.Exports {
  memory: WebAssembly.Memory
  photo_alloc(size: number): number
  photo_free(pointer: number, size: number): void
  photo_add(width: number, height: number, focal: number, pointer: number, size: number): bigint
  photo_add_calibrated(width: number, height: number, focal: number, pointer: number, size: number,
    calibrationPointer: number, calibrationSize: number): bigint
  photo_dense(resolution: number, preset: number): bigint
  photo_dense_prepare(resolution: number, preset: number): bigint
  photo_dense_finish(pointer: number, size: number): bigint
  photo_run(action: number, resolution: number): bigint
}

type Response<T> = {ok: true; value: T} | {ok: false; message: string}

/** Owns a single WASM instance. The Worker owns its lifetime and cancellation. */
export class PhotogrammetryKernel {
  private readonly wasm: Exports

  constructor(module: WebAssembly.Module) {
    this.wasm = new WebAssembly.Instance(module).exports as Exports
  }

  private response<T>(packed: bigint): T {
    const result = decodePacked<Response<T>>(
      this.wasm.memory,
      (pointer, size) => this.wasm.photo_free(pointer, size),
      packed,
      SURFACE_TRIPLES,
    )
    if (!result.ok) throw new Error(result.message)
    return result.value
  }

  add(image: PhotoPixels): number {
    const {width, height, focal, rgb} = image
    if (!Number.isInteger(width) || !Number.isInteger(height)
      || width < 48 || height < 48 || width > 2048 || height > 2048
      || rgb.length !== width * height * 3 || !Number.isFinite(focal)) {
      throw new Error('Invalid photo dimensions')
    }
    const metadata = image.calibration ? encodeBinary(image.calibration) : undefined
    if (metadata && metadata.length > 16 * 1024) throw new Error('Calibration metadata exceeds 16 KiB')
    const pointer = this.wasm.photo_alloc(rgb.length)
    if (!pointer) throw new Error('Photo allocation failed')
    let calibrationPointer = 0
    // photo_add/photo_add_calibrated consume the rgb buffer on any outcome
    // (owned and released by Rust); only free it if the call never happened.
    let consumed = false
    try {
      if (metadata) {
        calibrationPointer = this.wasm.photo_alloc(metadata.length)
        if (!calibrationPointer) throw new Error('Calibration allocation failed')
        // Allocation may grow WASM memory; create views only afterwards.
        new Uint8Array(this.wasm.memory.buffer, calibrationPointer, metadata.length).set(metadata)
      }
      new Uint8Array(this.wasm.memory.buffer, pointer, rgb.length).set(rgb)
      const packed = metadata
        ? this.wasm.photo_add_calibrated(width, height, focal, pointer, rgb.length, calibrationPointer, metadata.length)
        : this.wasm.photo_add(width, height, focal, pointer, rgb.length)
      consumed = true
      return this.response<number>(packed)
    } finally {
      if (calibrationPointer && metadata) this.wasm.photo_free(calibrationPointer, metadata.length)
      if (!consumed) this.wasm.photo_free(pointer, rgb.length)
    }
  }

  sparse(): PhotoReconstruction {
    return this.response<PhotoReconstruction>(this.wasm.photo_run(1, 0))
  }

  dense(resolution = 128, preset: PhotoDensePreset = 'baseline'): PhotoSurface {
    if (preset === 'baseline') return this.response<PhotoSurface>(this.wasm.photo_run(2, resolution))
    if (preset === 'slanted-plane') return this.response<PhotoSurface>(this.wasm.photo_dense(resolution, 1))
    if (preset === 'dual-scale-volume') return this.response<PhotoSurface>(this.wasm.photo_dense(resolution, 2))
    throw new Error('Unknown dense reconstruction preset')
  }

  compact(): PhotoSurface {
    return this.response<PhotoSurface>(this.wasm.photo_run(3, 0))
  }

  report(): PhotoDiagnostics | null {
    return this.response<PhotoDiagnostics | null>(this.wasm.photo_run(4, 0))
  }

  /**
   * Browser WebGPU sweep, stage 1: returns the packed shader payload and WGSL
   * text, or null when the request is ineligible for the GPU path.
   */
  densePrepare(resolution: number): {payload: Uint8Array, wgsl: string} | null {
    const decoded = decodePacked<Response<{ptr: number, len: number, wgsl: string} | null>>(
      this.wasm.memory,
      (pointer, size) => this.wasm.photo_free(pointer, size),
      this.wasm.photo_dense_prepare(resolution, 0),
    )
    if (!decoded.ok) throw new Error(decoded.message)
    if (decoded.value === null) return null
    const payloadPointer = decoded.value.ptr
    const payloadSize = decoded.value.len
    // Copy out before releasing the kernel buffer.
    const payload = new Uint8Array(payloadSize)
    payload.set(new Uint8Array(this.wasm.memory.buffer, payloadPointer, payloadSize))
    this.wasm.photo_free(payloadPointer, payloadSize)
    return {payload, wgsl: decoded.value.wgsl}
  }

  /** Stage 2: uploads host-computed scores (ownership moves to the kernel). */
  denseFinish(scores: Float32Array): PhotoSurface {
    const bytes = new Uint8Array(scores.buffer, scores.byteOffset, scores.byteLength)
    const pointer = writeLinear(this.wasm.memory, len => this.wasm.photo_alloc(len), bytes)
    if (!pointer) throw new Error('Score allocation failed')
    // photo_dense_finish consumes the buffer on any outcome; never freed here.
    return this.response<PhotoSurface>(this.wasm.photo_dense_finish(pointer, bytes.length))
  }

  clear(): void {
    this.response(this.wasm.photo_run(0, 0))
  }
}
