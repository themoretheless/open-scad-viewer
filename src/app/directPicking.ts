import { rayTriangleDistance } from '../kernel/geometry/math3d'
import type { OrbitCamera } from './directModelingTools'
import type { Point2 } from './directModeling'
import type { Vec3 } from '../kernel/geometry/math3d'

export interface PickCandidate {
  id: string
  mesh: { positions: ArrayLike<number>; indices: ArrayLike<number> }
}

/** Orthographic viewport ray against every candidate triangle; nearest hit wins. */
export function pickDirectMesh(
  p: Point2,
  camera: OrbitCamera,
  candidates: Iterable<PickCandidate>,
): { id: string; triangle: number } | null {
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
  let best: { id: string; triangle: number } | null = null,
    bestDistance = Infinity
  for (const body of candidates) {
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
      if (d !== null && d < bestDistance) {
        bestDistance = d
        best = { id: body.id, triangle: t }
      }
    }
  }
  return best
}
