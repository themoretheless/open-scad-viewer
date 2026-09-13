import type { Aabb3, Vec3 } from './math3d'
import { callGeometryRust } from './geometry/kernel'
import type { ProjectionMode } from './viewportModel'

interface OrbitProjectionOptions {
  yaw: number
  pitch: number
  /** Apparent zoom distance at the orbit target, also used by pan gestures. */
  distance: number
  target: Vec3
  aspect: number
  fovY: number
  projection: ProjectionMode
  bounds: Aabb3 | null
  backgroundRadius: number
}

/** Native orbit frame; the host only converts the GPU matrix transport. */
export function computeOrbitCameraFrame(options: OrbitProjectionOptions) {
  const frame = callGeometryRust<{
    eye: Vec3; viewProjection: number[]; eyeDistance: number; near: number; far: number
  }>('viewport', { ...options, action: 'orbit' })
  return { ...frame, viewProjection: new Float32Array(frame.viewProjection) }
}
