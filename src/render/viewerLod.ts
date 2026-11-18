/**
 * Screen-size inputs for the native viewer LOD selector (`nurbs_viewer_lod_select`).
 * Levels and their error bounds belong to the caller; this module only measures
 * how many screen pixels one model unit covers and forwards the choice to Rust.
 */
import { callGeometryRust } from '../kernel/geometry/kernel'
import type { Vec3 } from '../kernel/geometry/math3d'
import type { ProjectionMode } from './viewportModel'

export interface ViewerLodLevel {
  readonly triangleCount: number
  /** Geometric error upper bound in model units, owned by the level producer. */
  readonly errorUpper: number
}

export interface ViewerLodRequest {
  readonly pixelsPerUnit: number
  readonly pixelBudget: number
  readonly triangleBudget: number
  readonly hysteresis: number
}

export interface ViewerLodSelection {
  readonly level: number
  readonly projectedError: number
  readonly withinPixelBudget: boolean
  readonly reason: 'initial' | 'pixel-budget' | 'hysteresis-hold' | 'triangle-budget'
}

export interface ViewerLodCamera {
  readonly eye: Vec3
  /** Apparent zoom distance at the orbit target, as in the orbit camera. */
  readonly distance: number
  /** Eye to orbit target distance used by the perspective projection. */
  readonly eyeDistance: number
  readonly fovY: number
  readonly projection: ProjectionMode
  readonly viewportHeightPx: number
}

const positive = (value: number) => Number.isFinite(value) && value > 0

/**
 * Pixels per model unit at the nearest point of a bounding sphere. Perspective
 * uses the orbit projection's focal length, so a body closer to the eye gets a
 * larger scale; orthographic scale is independent of depth. The nearest depth
 * is clamped to a small positive value when the eye is inside the sphere.
 */
export function pixelsPerUnit(camera: ViewerLodCamera, center: Vec3, radius: number): number {
  if (
    !positive(camera.distance) ||
    !positive(camera.eyeDistance) ||
    !positive(camera.viewportHeightPx) ||
    !(camera.fovY > 0 && camera.fovY < Math.PI) ||
    !(radius >= 0) ||
    !Number.isFinite(radius)
  )
    throw new RangeError('Viewer LOD needs a positive camera, viewport and finite radius')
  const halfHeight = camera.distance * Math.tan(camera.fovY / 2)
  if (camera.projection === 'orthographic') return camera.viewportHeightPx / (2 * halfHeight)
  const depth = Math.max(
    Math.hypot(center[0] - camera.eye[0], center[1] - camera.eye[1], center[2] - camera.eye[2]) - radius,
    camera.eyeDistance * 1e-3,
  )
  return (camera.viewportHeightPx * camera.eyeDistance) / (2 * depth * halfHeight)
}

/** Picks one of two or three existing levels; hysteresis and budgets are enforced natively. */
export function selectViewerLod(
  levels: readonly ViewerLodLevel[],
  current: number | null,
  request: ViewerLodRequest,
): ViewerLodSelection {
  return callGeometryRust<ViewerLodSelection>('nurbs_viewer_lod_select', { levels, current, request })
}
