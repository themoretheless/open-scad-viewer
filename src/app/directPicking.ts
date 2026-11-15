import { rayTriangleDistance } from '../kernel/geometry/math3d'
import type { OrbitCamera } from './directModelingTools'
import type { Point2 } from './directModeling'
import type { Vec3 } from '../kernel/geometry/math3d'

export interface PickCandidate {
  id: string
  mesh: { positions: ArrayLike<number>; indices: ArrayLike<number> }
}

export interface PickHit {
  id: string
  triangle: number
  distance: number
}

/** Orthographic viewport ray against every candidate triangle; nearest hit wins. */
export function pickDirectMesh(
  p: Point2,
  camera: OrbitCamera,
  candidates: Iterable<PickCandidate>,
): { id: string; triangle: number } | null {
  const hit = pickDirectMeshHits(p, camera, candidates)[0]
  return hit ? { id: hit.id, triangle: hit.triangle } : null
}

/** Every candidate under the ray, one hit per object (its nearest triangle), sorted front to back. */
export function pickDirectMeshHits(p: Point2, camera: OrbitCamera, candidates: Iterable<PickCandidate>): PickHit[] {
  const { yaw, pitch } = camera
  const cy = Math.cos(yaw),
    sy = Math.sin(yaw),
    cp = Math.cos(pitch),
    sp = Math.sin(pitch)
  const right: Vec3 = [cy, -sy, 0],
    up: Vec3 = [sy * sp, cy * sp, -cp],
    toward: Vec3 = [sy * cp, cy * cp, sp]
  const far = 1e6
  const origin = [0, 1, 2].map(k => p[0] * right[k] + p[1] * up[k] + far * toward[k]) as Vec3
  const ray = { origin, direction: [-toward[0], -toward[1], -toward[2]] as Vec3 }
  const hits: PickHit[] = []
  for (const body of candidates) {
    let best: PickHit | null = null
    const pos = body.mesh.positions,
      idx = body.mesh.indices
    for (let t = 0; t < idx.length / 3; t++) {
      const a = idx[t * 3] * 3,
        b = idx[t * 3 + 1] * 3,
        c = idx[t * 3 + 2] * 3
      const d = rayTriangleDistance(
        ray,
        [pos[a], pos[a + 1], pos[a + 2]],
        [pos[b], pos[b + 1], pos[b + 2]],
        [pos[c], pos[c + 1], pos[c + 2]],
      )
      if (d !== null && (!best || d < best.distance)) best = { id: body.id, triangle: t, distance: d }
    }
    if (best) hits.push(best)
  }
  return hits.sort((a, b) => a.distance - b.distance || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
}

/**
 * Cycling among overlapping objects (roadmap C6): a repeated click at the same spot, or an
 * Alt+click, selects the next hit by depth; a click elsewhere starts again at the front.
 */
export class PickCycle {
  private last: { point: Point2; ids: string; index: number } | null = null
  constructor(private readonly tolerance = 4) {}

  choose(point: Point2, hits: readonly PickHit[], options: { cycle?: boolean } = {}): PickHit | null {
    if (!hits.length) {
      this.last = null
      return null
    }
    const ids = hits.map(h => h.id).join('\n')
    const near =
      !!this.last &&
      this.last.ids === ids &&
      Math.hypot(point[0] - this.last.point[0], point[1] - this.last.point[1]) <= this.tolerance
    const index = near || (options.cycle && this.last?.ids === ids) ? (this.last!.index + 1) % hits.length : 0
    this.last = { point, ids, index }
    return hits[index]
  }

  /** Pin an explicit choice (from the candidate list) so the next repeated click continues after it. */
  pin(point: Point2, hits: readonly PickHit[], id: string): void {
    const index = hits.findIndex(h => h.id === id)
    this.last = index < 0 ? null : { point, ids: hits.map(h => h.id).join('\n'), index }
  }

  reset(): void {
    this.last = null
  }
}
