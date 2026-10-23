import { geometryTransformTransition } from '../services/geometryTransformTransition'
import { invert, type Aabb3, type Mat4 } from '../services/math3d'
import type { NativePickingCache } from '../services/nativePickingCache'
import type { MeshData } from '../core/mesh'
import type { GeometryAssetId } from '../core/scene'
import type {
  DisplayMode,
  SceneUploadMetrics,
  ShadingModel,
} from '../services/rendererContracts'
import { buildSceneAabbIndex, type SceneAabbIndex } from '../services/sceneAabbIndex'
import { effectiveDisplayAlpha } from '../services/backendQuality'
import { OBJECT_UNIFORM_LAYOUT } from '../services/shaders'
import { fillObjectUniform } from '../services/uniformFill'
import {
  MAX_EDGE_BUFFER_BYTES,
  type Bounds,
  type GMesh,
} from './types'
import {
  blendMorphPositions,
  extractPositions,
  sameTypedArray,
} from './drawSorting'

export interface StageSceneMeshesContext {
  readonly dev: GPUDevice
  readonly previous: readonly GMesh[]
  readonly animate: boolean
  readonly displayMode: DisplayMode
  readonly defaultShadingModel: ShadingModel
  readonly defaultMaterial: {
    baseColor: [number, number, number]
    metallic: number
    roughness: number
    emissive: [number, number, number]
    alpha: number
  }
  readonly objBGL: GPUBindGroupLayout
  readonly morphDummyVB: GPUBuffer | null
  readonly objectUniformScratch: Float32Array
  readonly nativePicking: NativePickingCache
  readonly measureMeshBounds: (
    assetId: string | null,
    vertices: Float32Array,
    transform: Mat4,
  ) => { local: Aabb3; world: Bounds } | null
}

export interface StagedSceneMeshesResult {
  readonly next: GMesh[]
  readonly nextSceneIndex: SceneAabbIndex
  readonly retainedGeometryBuffers: Set<GPUBuffer>
  readonly metrics: SceneUploadMetrics
}

export function stageSceneMeshes(
  meshes: readonly MeshData[],
  ctx: StageSceneMeshesContext,
): StagedSceneMeshesResult {
  const { dev, previous, animate } = ctx
  const metrics: SceneUploadMetrics = { geometryUploadBytes: 0, geometryBuffersCreated: 0, reusedEntities: 0 }
  const previousSet = new Set(previous)
  const previousById = new Map(previous.filter(mesh => mesh.entityId).map(mesh => [mesh.entityId, mesh]))
  const started = performance.now()
  const reusableByAsset = new Map<GeometryAssetId, GMesh[]>()
  for (const mesh of previous) {
    if (!mesh.assetId) continue
    const candidates = reusableByAsset.get(mesh.assetId) ?? []
    candidates.push(mesh)
    reusableByAsset.set(mesh.assetId, candidates)
  }
  const retainedGeometryBuffers = new Set<GPUBuffer>()
  const next: GMesh[] = []
  let nextSceneIndex: SceneAabbIndex
  try {
    for (const m of meshes) {
      if (!m.indices.length && !m.vertices.length) continue
      if (m.vertices.length < 6 || m.vertices.length % 6 !== 0 || !m.indices.length) {
        throw new Error('Invalid mesh buffer layout')
      }

      const measured = ctx.measureMeshBounds(m.geometryAssetId ?? null, m.vertices, m.transform)
      if (!measured) throw new Error('Mesh contains no finite positions')
      const transform = new Float32Array(m.transform)
      const inverseTransform = invert(transform)

      const old = m.entityId ? previousById.get(m.entityId) : undefined
      // Equal index counts do not establish vertex correspondence after CSG.
      // Animate only rigid transforms of unchanged geometry, per entity.
      const matrix = animate && old && sameTypedArray(old.vertices, m.vertices)
        && sameTypedArray(old.indices, m.indices) ? geometryTransformTransition(old.morph?.currentMatrix ?? old.transform, transform) : null
      const morphFrom = animate && old && matrix
        && (!sameTypedArray(old.morph?.target ?? old.vertices, m.vertices) || !sameTypedArray(old.morph?.currentMatrix ?? old.transform, transform))
        ? (old.morph ? blendMorphPositions(old.morph, started) : extractPositions(old.vertices))
        : null
      const retainedMorph = matrix && old?.morph && sameTypedArray(old.transform, transform) ? old.morph : null
      // Animated geometry must own its buffer: instances may have different start shapes.
      const candidates = !animate && m.geometryAssetId ? reusableByAsset.get(m.geometryAssetId) : undefined
      // Prefer the already verified views staged earlier in this publication.
      // A newly built shared asset needs one content comparison, not one per instance.
      const reusable = candidates?.find(candidate => candidate.vertices === m.vertices && candidate.indices === m.indices)
        ?? candidates?.find(candidate => (
            sameTypedArray(candidate.vertices, m.vertices)
            && sameTypedArray(candidate.indices, m.indices)
          ))
      let vb: GPUBuffer | null = reusable?.vb ?? null
      let ib: GPUBuffer | null = reusable?.ib ?? null
      let ub: GPUBuffer | null = null
      // The morph source buffer is capacity-grown and inherited across the
      // publications of one entity, so repeated edits do not realloc it.
      let morphVB = old?.morphVB ?? null
      let morphVBCapacity = old?.morphVBCapacity ?? 0
      const ownsGeometryBuffers = !reusable
      try {
        if (!vb) vb = dev.createBuffer({ size: m.vertices.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
        if (!ib) ib = dev.createBuffer({ size: m.indices.byteLength, usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST })
        ub = dev.createBuffer({ size: OBJECT_UNIFORM_LAYOUT.bytes, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
        const materialAlpha = m.material ? m.material.alpha ?? 1 : ctx.defaultMaterial.alpha
        const initialAlpha = effectiveDisplayAlpha(m.color[3] * materialAlpha, ctx.displayMode)
        const initialEdge = ctx.displayMode === 'edges' ? 0.7 : 0
        if (ownsGeometryBuffers) {
          metrics.geometryBuffersCreated += 2
          metrics.geometryUploadBytes += m.vertices.byteLength + m.indices.byteLength
          // The vertex buffer always holds the destination geometry; the
          // shader blends the morph source from slot 1 toward it.
          dev.queue.writeBuffer(vb, 0, m.vertices)
          dev.queue.writeBuffer(ib, 0, m.indices)
        }
        if (!retainedMorph && morphFrom) {
          if (!morphVB || morphVBCapacity < morphFrom.byteLength) {
            morphVB = dev.createBuffer({ size: morphFrom.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
            morphVBCapacity = morphFrom.byteLength
          }
          dev.queue.writeBuffer(morphVB, 0, morphFrom)
        }
        // One contiguous upload per entity. writeBuffer snapshots the data,
        // so the same bounded scratch storage can serve the next entity.
        const uniform = ctx.objectUniformScratch
        fillObjectUniform(uniform, transform, inverseTransform, m.color, initialAlpha, initialEdge, m.material, ctx.defaultMaterial)
        dev.queue.writeBuffer(ub, 0, uniform)
        const bg = dev.createBindGroup({
          layout: ctx.objBGL,
          entries: [{ binding: 0, resource: { buffer: ub } }],
        })
        if (reusable) metrics.reusedEntities++
        const reuseEdges = reusable && sameTypedArray(reusable.edgeIndices, m.edgeIndices)
        const morph = retainedMorph ?? (morphFrom ? { from: morphFrom, target: m.vertices, started, matrix: matrix ?? undefined, currentMatrix: matrix?.(0) } : undefined)
        next.push({
          entityId: m.entityId,
          morph,
          morphVB, morphVBCapacity,
          morphSlot: morph ? morphVB : ctx.morphDummyVB,
          assetId: animate ? undefined : m.geometryAssetId,
          nativeGeometry: m.nativeGeometry, faceIdsAuthoritative: m.faceIdsAuthoritative,
          vb, ib, ic: m.indices.length, ub, bg,
          edgeIB: reuseEdges ? reusable.edgeIB : null,
          edgeIC: reuseEdges ? reusable.edgeIC : 0,
          vertices: m.vertices, indices: m.indices,
          edgeIndices: m.edgeIndices, faceIds: m.faceIds, bvh: m.bvh,
          provenance: m.provenance,
          transform, inverseTransform,
          color: [...m.color],
          alpha: initialAlpha,
          localBounds: measured.local,
          worldBounds: measured.world,
          visible: true,
          styleAlpha: initialAlpha,
          styleSelected: 0,
          styleEdge: initialEdge,
          styleHovered: 0,
          material: m.material,
          shadingModel: m.material?.shadingModel ?? ctx.defaultShadingModel,
        })
        const added = next[next.length - 1]
        if (added.assetId) {
          const candidates = reusableByAsset.get(added.assetId) ?? []
          candidates.push(added)
          reusableByAsset.set(added.assetId, candidates)
        }
        if (reusable && previousSet.has(reusable)) {
          retainedGeometryBuffers.add(reusable.vb)
          retainedGeometryBuffers.add(reusable.ib)
          if (reusable.edgeIB && added.edgeIB === reusable.edgeIB) retainedGeometryBuffers.add(reusable.edgeIB)
        }
        if (old && previousSet.has(old) && old.morphVB && morphVB === old.morphVB) {
          // The new mesh took over the morph source buffer.
          retainedGeometryBuffers.add(old.morphVB)
        }
      } catch (error) {
        if (ownsGeometryBuffers) { vb?.destroy(); ib?.destroy() }
        // A morph buffer created for the failed staged mesh must not leak;
        // an inherited one still belongs to the live previous mesh.
        if (morphVB && morphVB !== old?.morphVB) morphVB.destroy()
        ub?.destroy()
        throw error
      }
    }
    nextSceneIndex = buildSceneAabbIndex(next.map((mesh, id) => ({
      id, bounds: { min: mesh.worldBounds.min, max: mesh.worldBounds.max },
    })))
  } catch (error) {
    destroySceneMeshes(next, ctx.nativePicking, retainedGeometryBuffers)
    throw error
  }

  return { next, nextSceneIndex, retainedGeometryBuffers, metrics }
}

/** Rebuild at publication so cached handles never outlive their scene. */
export function rebuildEdgeBufferCacheMap(
  edgeBuffersByVertexBuffer: Map<GPUBuffer, GMesh[]>,
  meshes: readonly GMesh[],
) {
  edgeBuffersByVertexBuffer.clear()
  for (const mesh of meshes) {
    if (!mesh.edgeIB) continue
    const candidates = edgeBuffersByVertexBuffer.get(mesh.vb) ?? []
    if (!candidates.some(candidate => candidate.edgeIB === mesh.edgeIB)) candidates.push(mesh)
    edgeBuffersByVertexBuffer.set(mesh.vb, candidates)
  }
}

export function ensureMeshEdgeBuffer(
  dev: GPUDevice | null,
  mesh: GMesh,
  edgeBuffersByVertexBuffer: Map<GPUBuffer, GMesh[]>,
) {
  if (!dev || mesh.edgeIB || !mesh.edgeIndices.length) return
  const byteLength = mesh.edgeIndices.byteLength
  if (byteLength > MAX_EDGE_BUFFER_BYTES || byteLength > dev.limits.maxBufferSize) return

  const candidates = edgeBuffersByVertexBuffer.get(mesh.vb)
  const shared = candidates?.find(candidate => candidate.ib === mesh.ib
    && sameTypedArray(candidate.edgeIndices, mesh.edgeIndices))
  if (shared?.edgeIB) {
    mesh.edgeIB = shared.edgeIB
    mesh.edgeIC = shared.edgeIC
    return
  }

  let buffer: GPUBuffer | null = null
  try {
    buffer = dev.createBuffer({ size: byteLength, usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST })
    dev.queue.writeBuffer(buffer, 0, mesh.edgeIndices)
    mesh.edgeIB = buffer
    mesh.edgeIC = mesh.edgeIndices.length
    if (candidates) candidates.push(mesh)
    else edgeBuffersByVertexBuffer.set(mesh.vb, [mesh])
  } catch {
    buffer?.destroy()
  }
}

export function reconcileMeshStyles(
  dev: GPUDevice | null,
  meshes: readonly GMesh[],
  displayMode: DisplayMode,
  defaultMaterialAlpha: number,
  geometryFadeProgress: number,
  styleScratch: Float32Array,
  usesObjectSelectionStyle: (index: number) => boolean,
  usesObjectHoverStyle: (index: number) => boolean,
  ensureEdgeBuffer: (mesh: GMesh) => void,
) {
  if (!dev) return
  const ease = geometryFadeProgress * geometryFadeProgress * (3 - 2 * geometryFadeProgress)
  for (let index = 0; index < meshes.length; index++) {
    const mesh = meshes[index]
    const selected = usesObjectSelectionStyle(index) ? 1 : 0
    const hovered = usesObjectHoverStyle(index) && !selected ? 1 : 0
    const materialAlpha = mesh.material ? mesh.material.alpha ?? 1 : defaultMaterialAlpha
    const alpha = effectiveDisplayAlpha(mesh.color[3] * materialAlpha, displayMode) * ease
    const edgeOpacity = (selected || hovered ? 1 : displayMode === 'edges' ? 0.7 : 0) * ease
    mesh.alpha = alpha
    if (mesh.styleAlpha === alpha && mesh.styleSelected === selected
      && mesh.styleEdge === edgeOpacity && mesh.styleHovered === hovered) continue
    mesh.styleAlpha = alpha
    mesh.styleSelected = selected
    mesh.styleEdge = edgeOpacity
    mesh.styleHovered = hovered
    styleScratch[0] = alpha
    styleScratch[1] = selected
    styleScratch[2] = edgeOpacity
    styleScratch[3] = hovered
    dev.queue.writeBuffer(mesh.ub, OBJECT_UNIFORM_LAYOUT.styleByteOffset, styleScratch)
    if (edgeOpacity > 0) ensureEdgeBuffer(mesh)
  }
}

export function destroySceneMeshes(
  meshes: readonly GMesh[],
  nativePicking: NativePickingCache,
  preserved: ReadonlySet<GPUBuffer> = new Set(),
) {
  const destroyed = new Set<GPUBuffer>()
  const destroy = (buffer: GPUBuffer | null) => {
    if (!buffer || preserved.has(buffer) || destroyed.has(buffer)) return
    destroyed.add(buffer)
    buffer.destroy()
  }
  for (const g of meshes) {
    if (!preserved.has(g.vb)) nativePicking.release(g.vb)
    destroy(g.edgeIB)
    destroy(g.vb); destroy(g.ib); destroy(g.ub); destroy(g.morphVB)
  }
}
