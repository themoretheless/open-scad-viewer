import type { Aabb3, Mat4 } from '../kernel/geometry/math3d'
import { sameTypedArray } from './drawSorting'
import type { SlicerResult } from './gpuSlicer'
import type { Bounds, GMesh } from './types'

type Vec3 = [number, number, number]

/** Scene bounds measurement and per-mesh scene queries extracted from the renderer. */

/**
 * Keyed by geometryAssetId (content) so republished equal meshes hit; per
 * content id a small list of transform snapshots covers instance edits.
 */
export type MeshBoundsCache = Map<
  string,
  {
    transformSnapshot: Float32Array
    result: { local: Aabb3; world: Bounds } | null
  }[]
>

/**
 * Scalar min/max scan over the interleaved vertex stream (no per-vertex
 * allocations or closures). Cache each immutable geometry/instance pair so
 * shared assets do not evict each other's transformed bounds. Weak keys do
 * not keep previous scene instances alive; snapshots detect transform edits.
 */
export function measureMeshBounds(
  cache: MeshBoundsCache,
  assetId: string | null,
  vertices: Float32Array,
  transform: Mat4,
): { local: Aabb3; world: Bounds } | null {
  // Meshes without a content identity compute bounds without caching.
  const entries = assetId === null ? undefined : cache.get(assetId)
  if (entries) {
    for (const entry of entries) {
      if (sameTypedArray(entry.transformSnapshot, transform)) return entry.result
    }
  }

  const m0 = transform[0]!,
    m1 = transform[1]!,
    m2 = transform[2]!,
    m3 = transform[3]!
  const m4 = transform[4]!,
    m5 = transform[5]!,
    m6 = transform[6]!,
    m7 = transform[7]!
  const m8 = transform[8]!,
    m9 = transform[9]!,
    m10 = transform[10]!,
    m11 = transform[11]!
  const m12 = transform[12]!,
    m13 = transform[13]!,
    m14 = transform[14]!,
    m15 = transform[15]!

  let lMinX = Infinity,
    lMinY = Infinity,
    lMinZ = Infinity
  let lMaxX = -Infinity,
    lMaxY = -Infinity,
    lMaxZ = -Infinity
  let wMinX = Infinity,
    wMinY = Infinity,
    wMinZ = Infinity
  let wMaxX = -Infinity,
    wMaxY = -Infinity,
    wMaxZ = -Infinity
  for (let i = 0; i + 2 < vertices.length; i += 6) {
    const x = vertices[i]!,
      y = vertices[i + 1]!,
      z = vertices[i + 2]!
    if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) continue
    const w = m12 * x + m13 * y + m14 * z + m15
    const iw = Number.isFinite(w) && Math.abs(w) > 1e-12 ? 1 / w : 1
    const wx = (m0 * x + m1 * y + m2 * z + m3) * iw
    const wy = (m4 * x + m5 * y + m6 * z + m7) * iw
    const wz = (m8 * x + m9 * y + m10 * z + m11) * iw
    if (!Number.isFinite(wx) || !Number.isFinite(wy) || !Number.isFinite(wz)) continue
    if (x < lMinX) lMinX = x
    if (y < lMinY) lMinY = y
    if (z < lMinZ) lMinZ = z
    if (x > lMaxX) lMaxX = x
    if (y > lMaxY) lMaxY = y
    if (z > lMaxZ) lMaxZ = z
    if (wx < wMinX) wMinX = wx
    if (wy < wMinY) wMinY = wy
    if (wz < wMinZ) wMinZ = wz
    if (wx > wMaxX) wMaxX = wx
    if (wy > wMaxY) wMaxY = wy
    if (wz > wMaxZ) wMaxZ = wz
  }
  if (!Number.isFinite(lMinX)) return null
  const result = {
    local: { min: [lMinX, lMinY, lMinZ] as Vec3, max: [lMaxX, lMaxY, lMaxZ] as Vec3 },
    world: {
      center: [(wMinX + wMaxX) / 2, (wMinY + wMaxY) / 2, (wMinZ + wMaxZ) / 2] as [number, number, number],
      radius: Math.hypot(wMaxX - wMinX, wMaxY - wMinY, wMaxZ - wMinZ) / 2,
      min: [wMinX, wMinY, wMinZ] as [number, number, number],
      max: [wMaxX, wMaxY, wMaxZ] as [number, number, number],
    },
  }
  if (assetId === null) return result
  let list = cache.get(assetId)
  if (!list) {
    list = []
    cache.set(assetId, list)
  }
  if (list.length >= 8) list.shift()
  list.push({ transformSnapshot: new Float32Array(transform), result })
  return result
}

export function combineBounds(bounds: Bounds[]): Bounds | null {
  if (!bounds.length) return null
  const min: Vec3 = [Infinity, Infinity, Infinity]
  const max: Vec3 = [-Infinity, -Infinity, -Infinity]
  for (const bound of bounds) {
    for (let axis = 0; axis < 3; axis++) {
      min[axis] = Math.min(min[axis]!, bound.min[axis]!)
      max[axis] = Math.max(max[axis]!, bound.max[axis]!)
    }
  }
  const center: Vec3 = [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2]
  return { center, radius: Math.hypot(max[0] - min[0], max[1] - min[1], max[2] - min[2]) / 2, min, max }
}

export interface SceneSliceOptions {
  planeNormal: [number, number, number]
  firstPlaneOffset: number
  layerThickness: number
  numLayers: number
}

/** Slice every visible mesh on the GPU, in scene order. */
export async function sliceVisibleMeshes(
  meshes: readonly GMesh[],
  options: SceneSliceOptions,
): Promise<Array<{ meshIndex: number; slice: SlicerResult }>> {
  // The slicer and its compute runner load on the first slice request, not with the renderer.
  const { runGpuMeshSlicer } = await import('./gpuSlicer')
  const results: Array<{ meshIndex: number; slice: SlicerResult }> = []
  for (let i = 0; i < meshes.length; i++) {
    const m = meshes[i]!
    if (!m.visible) continue
    const slice = await runGpuMeshSlicer(m.vertices, m.indices, {
      strideFloats: 6,
      planeNormal: options.planeNormal,
      firstLayerDistance: options.firstPlaneOffset,
      layerStep: options.layerThickness,
      numLayers: options.numLayers,
    })
    results.push({ meshIndex: i, slice })
  }
  return results
}
