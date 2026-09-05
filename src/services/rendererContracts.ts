import type { MeshSourceReference } from '../core/mesh'
import type { CameraState } from './cameraHistory'
import type { Vec3 } from './math3d'
export type { ProjectionMode, StandardView } from './viewportModel'

export type DisplayMode = 'shaded' | 'edges' | 'xray'
export type SelectionMode = 'object' | 'face' | 'point'

export interface PickHit {
  meshIndex: number
  triangleIndex: number
  faceId: number | null
  point: Vec3
  normal: Vec3
  barycentric: Vec3
  source: MeshSourceReference | null
  backside: boolean
  cycleIndex?: number
  cycleCount?: number
}

export interface DistanceMeasurement {
  points: Vec3[]
  distance: number | null
}

export type SelectionChangeHandler = (selectedIndex: number | null, isIsolated: boolean, hit: PickHit | null) => void
export type HoverChangeHandler = (hit: PickHit | null) => void
export type MeasurementChangeHandler = (measurement: DistanceMeasurement | null, active: boolean) => void
export type CameraHistoryChangeHandler = (canGoBack: boolean) => void
export type CameraChangeHandler = (state: CameraState) => void

export type RendererLifecycleEvent =
  | { readonly status: 'idle' }
  | { readonly status: 'initializing' }
  | { readonly status: 'ready' }
  | { readonly status: 'unavailable'; readonly reason: 'webgpu' | 'adapter' | 'context'; readonly message: string }
  | { readonly status: 'device-lost'; readonly reason: 'destroyed' | 'unknown'; readonly message: string }
  | { readonly status: 'error'; readonly phase: 'initialization' | 'frame'; readonly error: Error }
  | { readonly status: 'destroyed' }

export type RendererStatusChangeHandler = (event: RendererLifecycleEvent) => void

export interface SetMeshesOptions {
  preserveMeasurement?: boolean
  /** Opaque publication token, echoed only after its first GPU submission. */
  frameToken?: number
}

/** CPU-side publication counters, not GPU execution time. */
export interface SceneUploadMetrics {
  geometryUploadBytes: number
  geometryBuffersCreated: number
  reusedEntities: number
}
