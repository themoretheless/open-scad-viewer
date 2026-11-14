/**
 * Per-mesh tessellation levels for the viewer. A mesh published with `lods`
 * keeps its published geometry as the base and switches its GPU buffers and
 * CPU views between the certified levels chosen by the native LOD selector.
 * Selection, visibility and transforms stay on the same GMesh; only the
 * geometry views, picking snapshot and edge buffer change.
 */
import type { MeshLodLevel } from '../core/mesh'
import type { NativePickingCache } from './nativePickingCache'
import type { GMesh } from './types'
import { pixelsPerUnit, selectViewerLod, type ViewerLodCamera } from './viewerLod'

interface LevelBuffers {
  vb: GPUBuffer
  ib: GPUBuffer
}

export interface GMeshLodState {
  readonly levels: readonly MeshLodLevel[]
  /** Level currently bound to the mesh, or null while the base geometry is shown. */
  current: number | null
  readonly buffers: (LevelBuffers | null)[]
  readonly base: Pick<
    GMesh,
    'vb' | 'ib' | 'ic' | 'vertices' | 'indices' | 'edgeIndices' | 'faceIds' | 'bvh' | 'provenance'
  >
}

export interface MeshLodPolicy {
  /** Allowed projected geometric error in screen pixels. */
  readonly pixelBudget: number
  readonly triangleBudget: number
  readonly hysteresis: number
}

export const DEFAULT_MESH_LOD_POLICY: MeshLodPolicy = { pixelBudget: 0.75, triangleBudget: 200_000, hysteresis: 1.5 }

export function createMeshLodState(
  mesh: GMesh,
  levels: readonly MeshLodLevel[] | undefined,
): GMeshLodState | undefined {
  if (!levels || levels.length < 2 || levels.length > 3) return undefined
  const { vb, ib, ic, vertices, indices, edgeIndices, faceIds, bvh, provenance } = mesh
  return {
    levels,
    current: null,
    buffers: levels.map(() => null),
    base: { vb, ib, ic, vertices, indices, edgeIndices, faceIds, bvh, provenance },
  }
}

/**
 * Bind `level` to `mesh`, uploading its buffers on first use. Returns false
 * when the mesh is already showing that level. The caller clears draw caches
 * and any triangle-level hit state, which no longer indexes this geometry.
 */
export function bindMeshLod(dev: GPUDevice, mesh: GMesh, level: number, nativePicking: NativePickingCache): boolean {
  const lod = mesh.lod
  if (!lod || lod.current === level || !lod.levels[level]) return false
  const data = lod.levels[level]
  let buffers = lod.buffers[level]
  if (!buffers) {
    const vb = dev.createBuffer({
      size: data.vertices.byteLength,
      usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
    })
    const ib = dev.createBuffer({
      size: data.indices.byteLength,
      usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST,
    })
    dev.queue.writeBuffer(vb, 0, data.vertices)
    dev.queue.writeBuffer(ib, 0, data.indices)
    buffers = lod.buffers[level] = { vb, ib }
  }
  if (mesh.vb !== lod.base.vb) nativePicking.release(mesh.vb)
  // The edge buffer indexes the previous vertices; it is rebuilt on demand.
  if (mesh.edgeIB && !isBaseEdgeBuffer(mesh)) mesh.edgeIB.destroy()
  mesh.edgeIB = null
  mesh.edgeIC = 0
  mesh.vb = buffers.vb
  mesh.ib = buffers.ib
  mesh.ic = data.indices.length
  mesh.vertices = data.vertices
  mesh.indices = data.indices
  mesh.edgeIndices = data.edgeIndices
  mesh.faceIds = data.faceIds
  mesh.bvh = data.bvh
  mesh.provenance = data.provenance
  mesh.faceIndex = undefined
  lod.current = level
  return true
}

/** The base edge buffer may be shared with other instances of the same asset; leave it alive. */
function isBaseEdgeBuffer(mesh: GMesh): boolean {
  return mesh.lod?.current === null
}

/** Restore base views before destruction so shared-asset bookkeeping sees the published buffers. */
export function releaseMeshLod(mesh: GMesh, nativePicking: NativePickingCache): void {
  const lod = mesh.lod
  if (!lod) return
  if (lod.current !== null) {
    nativePicking.release(mesh.vb)
    mesh.edgeIB?.destroy()
    mesh.edgeIB = null
    mesh.edgeIC = 0
    Object.assign(mesh, lod.base)
    mesh.faceIndex = undefined
    lod.current = null
  }
  for (const buffers of lod.buffers) {
    buffers?.vb.destroy()
    buffers?.ib.destroy()
  }
  lod.buffers.fill(null)
}

/**
 * Evaluate every LOD mesh against the camera and bind the chosen levels.
 * Returns the meshes whose geometry changed.
 */
export function updateMeshLods(
  dev: GPUDevice,
  meshes: readonly GMesh[],
  camera: ViewerLodCamera,
  nativePicking: NativePickingCache,
  policy: MeshLodPolicy = DEFAULT_MESH_LOD_POLICY,
): GMesh[] {
  const changed: GMesh[] = []
  for (const mesh of meshes) {
    const lod = mesh.lod
    if (!lod || !mesh.visible) continue
    const { center, radius } = mesh.worldBounds
    const selection = selectViewerLod(
      lod.levels.map(level => ({
        triangleCount: level.triangleCount,
        errorUpper: level.errorUpper * worldScale(mesh.transform),
      })),
      lod.current,
      { pixelsPerUnit: pixelsPerUnit(camera, center, radius), ...policy },
    )
    if (bindMeshLod(dev, mesh, selection.level, nativePicking)) changed.push(mesh)
  }
  return changed
}

/** Largest axis scale of a row-major affine transform; model-space bounds grow by it in world space. */
export function worldScale(transform: Float32Array): number {
  let scale = 0
  for (let column = 0; column < 3; column++) {
    scale = Math.max(scale, Math.hypot(transform[column], transform[4 + column], transform[8 + column]))
  }
  return scale > 0 && Number.isFinite(scale) ? scale : 1
}
