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

interface SortEntry { index: number; center: readonly [number, number, number]; depth: number }
const compareDepth = (left: SortEntry, right: SortEntry) => right.depth - left.depth || left.index - right.index

/** Reuses sort records across frames; callers consume the result before begin(). */
export class TransparentSortBuffer {
  private readonly pool: SortEntry[] = []
  private readonly ordered: SortEntry[] = []

  private count = 0
  private membershipChanged = false

  begin() { this.count = 0; this.membershipChanged = false }
  clear() { this.begin(); this.pool.length = this.ordered.length = 0 }

  add(index: number, center: readonly [number, number, number]) {
    const slot = this.count++
    let entry = this.pool[slot]
    if (!entry) { entry = { index, center, depth: 0 }; this.pool[slot] = entry; this.membershipChanged = true }
    else {
      if (entry.index !== index || entry.center !== center) this.membershipChanged = true
      entry.index = index; entry.center = center
    }
  }

  sort(eye: readonly [number, number, number], tx: number, ty: number, tz: number): readonly SortEntry[] {
    // Keep the previous sorted order when membership is stable. Small camera
    // movements then give the native adaptive sort an almost sorted sequence.
    if (this.membershipChanged || this.ordered.length !== this.count) {
      this.ordered.length = this.count
      for (let i = 0; i < this.count; i++) this.ordered[i] = this.pool[i]
    }
    let dx = tx - eye[0], dy = ty - eye[1], dz = tz - eye[2]
    const length = Math.hypot(dx, dy, dz)
    if (length > 1e-12 && Number.isFinite(length)) { dx /= length; dy /= length; dz /= length }
    else { dx = 0; dy = 0; dz = -1 }
    for (const entry of this.ordered) {
      entry.depth = (entry.center[0] - eye[0]) * dx + (entry.center[1] - eye[1]) * dy + (entry.center[2] - eye[2]) * dz
    }
    return this.ordered.sort(compareDepth)
  }
}
