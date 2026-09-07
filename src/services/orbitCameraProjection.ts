import {
  lookAt, multiply, orthographic, perspective,
  type Aabb3, type Vec3,
} from './math3d'
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

/**
 * Dolly until the eye reaches the model, then continue magnifying optically.
 * Orthographic zoom changes only the image scale. In both projections the
 * whole model stays in front of the eye and the near plane, even when a saved
 * camera, a pinch, a pan, or a rotation would otherwise put the eye inside it.
 */
export function computeOrbitCameraFrame(options: OrbitProjectionOptions) {
  const { yaw, pitch, distance, target, aspect, fovY, projection, bounds } = options
  const cp = Math.cos(pitch)
  const backward: Vec3 = [cp * Math.sin(yaw), -cp * Math.cos(yaw), Math.sin(pitch)]
  let frontDepth = 0
  let eyeDistance = distance
  let extent = options.backgroundRadius
  if (bounds) {
    const center: Vec3 = [0, 1, 2].map(axis => (bounds.min[axis] + bounds.max[axis]) / 2) as Vec3
    const halfSize: Vec3 = [0, 1, 2].map(axis => (bounds.max[axis] - bounds.min[axis]) / 2) as Vec3
    const projectedCenter = backward.reduce((sum, component, axis) => sum + component * (center[axis] - target[axis]), 0)
    const projectedRadius = backward.reduce((sum, component, axis) => sum + Math.abs(component) * halfSize[axis], 0)
    frontDepth = projectedCenter + projectedRadius
    const radius = Math.hypot(...halfSize)
    // A scale-relative gap protects thin models and avoids an unstable depth
    // range at the last surface. Using projected depth keeps panning lateral.
    const surfaceGap = Math.max(0.01, radius * 0.05)
    eyeDistance = Math.max(distance, frontDepth + surfaceGap)
    extent = Math.max(extent, Math.hypot(...center.map((value, axis) => value - target[axis])) + radius)
  }
  const eye: Vec3 = target.map((value, axis) => value + eyeDistance * backward[axis]) as Vec3
  const nearest = eyeDistance - frontDepth
  const near = Math.max(0.001, nearest * 0.05)
  const far = Math.max(near + 1, eyeDistance + extent * 1.1)
  const halfHeight = distance * Math.tan(fovY / 2)
  // This preserves the world units per pixel at the target used by pan and
  // pinch gestures, and is continuous where dolly changes to optical zoom.
  const effectiveFov = 2 * Math.atan(halfHeight / eyeDistance)
  const projectionMatrix = projection === 'perspective'
    ? perspective(effectiveFov, aspect, near, far)
    : orthographic(-halfHeight * aspect, halfHeight * aspect, -halfHeight, halfHeight, near, far)
  const view = lookAt(eye, target, [0, 0, 1])
  return { eye, viewProjection: multiply(projectionMatrix, view), eyeDistance, near, far }
}
