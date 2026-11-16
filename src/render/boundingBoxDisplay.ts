/**
 * Picking for meshes drawn as bounding-box proxies (`GMeshLodState.boxed`,
 * outlined by `boundingBoxLineVertices`): a boxed mesh is hit by its world box
 * so it stays selectable while its triangles are not drawn.
 */
import type { Vec3 } from '../kernel/geometry/math3d'
import type { PickHit } from './rendererContracts'
import type { Bounds, GMesh } from './types'

/** Whether the frame draws this mesh as its box outline instead of its triangles. */
export const isBoxed = (mesh: GMesh | undefined) => !!mesh?.lod?.boxed && !!mesh?.visible

/** Nearest entry of a ray into an axis-aligned box (origin inside → exit is not reported; t = 0). */
export function rayBoxHit(
  ray: { origin: Vec3; direction: Vec3 },
  bounds: Pick<Bounds, 'min' | 'max'>,
): { t: number; point: Vec3; normal: Vec3 } | null {
  let near = -Infinity
  let far = Infinity
  let axis = 0
  let sign = 1
  for (let i = 0; i < 3; i++) {
    const o = ray.origin[i]
    const d = ray.direction[i]
    if (Math.abs(d) < 1e-15) {
      if (o < bounds.min[i] || o > bounds.max[i]) return null
      continue
    }
    let t0 = (bounds.min[i] - o) / d
    let t1 = (bounds.max[i] - o) / d
    let entrySign = -1
    if (t0 > t1) {
      ;[t0, t1] = [t1, t0]
      entrySign = 1
    }
    if (t0 > near) {
      near = t0
      axis = i
      sign = entrySign
    }
    far = Math.min(far, t1)
    if (near > far) return null
  }
  if (far < 0) return null
  const t = Math.max(0, near)
  const point: Vec3 = [
    ray.origin[0] + ray.direction[0] * t,
    ray.origin[1] + ray.direction[1] * t,
    ray.origin[2] + ray.direction[2] * t,
  ]
  const normal: Vec3 = [0, 0, 0]
  normal[axis] = sign
  return { t, point, normal }
}

/** Object-level pick hit on a boxed mesh; there is no triangle under a box. */
export function boxPickHit(meshIndex: number, mesh: GMesh, point: Vec3, normal: Vec3): PickHit {
  return {
    meshIndex,
    triangleIndex: -1,
    faceId: null,
    point,
    normal,
    barycentric: [1, 0, 0],
    source: mesh.provenance[0]?.source ?? null,
    backside: false,
  }
}
