import type { Vec3 } from './math3d'

export interface TransparentObject {
  readonly index: number
  readonly center: readonly [number, number, number]
}

/**
 * Return object indices in deterministic back-to-front camera-depth order.
 *
 * Object sorting cannot solve intersecting transparent triangles, but it
 * produces correct source-over blending for separated CAD bodies and is the
 * expected baseline before weighted/order-independent transparency.
 */
export function sortTransparentBackToFront(
  objects: readonly TransparentObject[],
  eye: readonly [number, number, number],
  target: readonly [number, number, number],
): number[] {
  const dx = target[0] - eye[0]
  const dy = target[1] - eye[1]
  const dz = target[2] - eye[2]
  const length = Math.hypot(dx, dy, dz)
  const direction: Vec3 = length > 1e-12 && Number.isFinite(length)
    ? [dx / length, dy / length, dz / length]
    : [0, 0, -1]

  return objects
    .map(object => ({
      index: object.index,
      depth: (object.center[0] - eye[0]) * direction[0]
        + (object.center[1] - eye[1]) * direction[1]
        + (object.center[2] - eye[2]) * direction[2],
    }))
    .sort((left, right) => right.depth - left.depth || left.index - right.index)
    .map(object => object.index)
}
