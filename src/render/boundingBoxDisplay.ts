/**
 * Bounding-box draw path for meshes the display scheduler put on the
 * 'bounding-box' representation. All boxes are batched into one line-list
 * buffer (position + RGBA, the selection overlay vertex layout) and drawn in
 * one call; the mesh itself is skipped by the frame encoder. Picking falls
 * back to the same world box so a boxed mesh stays selectable.
 */
import type { Vec3 } from '../kernel/geometry/math3d'
import { writeOverlaySlot, releaseOverlaySlot } from './overlayBuilder'
import type { PickHit } from './rendererContracts'
import type { Bounds, GMesh, OverlaySlot } from './types'

/** Corner pairs of the 12 box edges; corner bit 0/1/2 selects max on x/y/z. */
const BOX_EDGES = [0, 1, 2, 3, 4, 5, 6, 7, 0, 2, 1, 3, 4, 6, 5, 7, 0, 4, 1, 5, 2, 6, 3, 7]

export const BOX_VERTICES_PER_MESH = BOX_EDGES.length

const corner = (bounds: Bounds, bits: number): Vec3 => [
  bits & 1 ? bounds.max[0] : bounds.min[0],
  bits & 2 ? bounds.max[1] : bounds.min[1],
  bits & 4 ? bounds.max[2] : bounds.min[2],
]

/** Whether the frame draws this mesh as a box instead of its triangles. */
export const isBoxed = (mesh: GMesh | undefined) => !!mesh?.displayBox && mesh.visible

/** Interleaved line-list vertices (xyz rgba) for every boxed, drawable mesh. */
export function buildBoundingBoxLines(meshes: readonly GMesh[], drawable: (index: number) => boolean): number[] {
  const values: number[] = []
  meshes.forEach((mesh, index) => {
    if (!isBoxed(mesh) || !drawable(index)) return
    const [r, g, b] = mesh.color
    for (const bits of BOX_EDGES) values.push(...corner(mesh.worldBounds, bits), r, g, b, 1)
  })
  return values
}

/** Persistent batched box buffer; rewrites only when the box set or bounds change. */
export class BoundingBoxLayer {
  readonly slot: OverlaySlot = { buffer: null, capacity: 0, count: 0 }
  private last: number[] = []

  update(dev: GPUDevice | null, meshes: readonly GMesh[], drawable: (index: number) => boolean) {
    const values = buildBoundingBoxLines(meshes, drawable)
    if (values.length === this.last.length && values.every((value, i) => value === this.last[i])) return
    this.last = values
    writeOverlaySlot(dev, this.slot, values)
  }

  get boxCount() {
    return this.slot.count / BOX_VERTICES_PER_MESH
  }

  release() {
    releaseOverlaySlot(this.slot)
    this.last = []
  }
}

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
