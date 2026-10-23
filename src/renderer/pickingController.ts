import { rayAabbDistance, type Aabb3, type Vec3 } from '../services/math3d'
import type { MeshBvhHit } from '../services/meshBvh'
import type { NativePickingCache } from '../services/nativePickingCache'
import { selectedCornerInKernel } from '../services/geometry/viewport'
import type { MeshProvenanceRun } from '../core/mesh'
import type { PickHit, SelectionMode } from '../services/rendererContracts'
import {
  normalizeDepthCandidates,
  type DepthCandidate,
} from '../services/selectionCycling'
import {
  querySceneAabbIndex,
  type SceneAabbIndex,
} from '../services/sceneAabbIndex'
import {
  MAX_DEPTH_CANDIDATES,
  MAX_DEPTH_CONTINUATIONS,
  type Bounds,
  type GMesh,
} from './types'

export function sourceForTriangle(mesh: GMesh, triangleIndex: number): MeshProvenanceRun | null {
  let low = 0
  let high = mesh.provenance.length - 1
  while (low <= high) {
    const middle = (low + high) >>> 1
    const run = mesh.provenance[middle]
    if (triangleIndex < run.triangleStart) high = middle - 1
    else if (triangleIndex >= run.triangleEnd) low = middle + 1
    else return run
  }
  return null
}

export function pickHitFromBvh(
  meshIndex: number,
  mesh: GMesh,
  hit: MeshBvhHit,
  selectionMode: SelectionMode,
): PickHit | null {
  const run = sourceForTriangle(mesh, hit.triangleIndex)
  let point = hit.worldPoint as Vec3
  if (selectionMode === 'point') {
    const vertices = hit.triangleVertexIndices.map(index => {
      const offset = index * mesh.bvh.vertexStride
      return [mesh.vertices[offset], mesh.vertices[offset + 1], mesh.vertices[offset + 2]] as Vec3
    })
    const corner = selectedCornerInKernel(mesh.transform, vertices, hit.barycentric)
    if (!corner) return null
    point = corner
  }
  return {
    meshIndex,
    triangleIndex: hit.triangleIndex,
    faceId: mesh.faceIds[hit.triangleIndex] ?? null,
    point,
    normal: hit.worldNormal,
    barycentric: hit.barycentric,
    source: run?.source ?? null,
    backside: run?.backside ?? false,
  }
}

export function isSectionClipped(
  point: readonly number[],
  sectionEnabled: boolean,
  sectionNormal: Vec3,
  sectionOffset: number,
): boolean {
  return sectionEnabled && (
    point[0] * sectionNormal[0]
    + point[1] * sectionNormal[1]
    + point[2] * sectionNormal[2]
  ) < sectionOffset
}

export function selectionCandidateKey(hit: PickHit, selectionMode: SelectionMode): string {
  if (selectionMode === 'object') return `o:${hit.meshIndex}`
  if (selectionMode === 'face') {
    return hit.faceId === null
      ? `f:${hit.meshIndex}:t${hit.triangleIndex}`
      : `f:${hit.meshIndex}:${hit.faceId}`
  }
  // Flat-shaded meshes may duplicate a geometric vertex for each face. A
  // world-space key avoids multiple visually identical point-cycle stops.
  return `p:${hit.meshIndex}:${hit.point.map(value => value.toPrecision(12)).join(':')}`
}

export interface FindHitCandidatesContext {
  readonly ray: { origin: Vec3; direction: Vec3 } | null
  readonly bounds: Bounds | null
  readonly meshes: readonly GMesh[]
  readonly selectionMode: SelectionMode
  readonly sectionEnabled: boolean
  readonly sectionNormal: Vec3
  readonly sectionOffset: number
  readonly sceneAabbIndex: SceneAabbIndex
  readonly nativePicking: NativePickingCache
  readonly isMeshVisible: (index: number) => boolean
}

export function findHitCandidatesForRay(
  ctx: FindHitCandidatesContext,
  maximum = MAX_DEPTH_CANDIDATES,
): DepthCandidate<PickHit>[] {
  const limit = Math.max(0, Math.min(MAX_DEPTH_CANDIDATES, Math.trunc(maximum)))
  if (!limit) return []
  const { ray } = ctx
  if (!ray) return []

  if (ctx.bounds) {
    const margin = Math.max(1e-4, ctx.bounds.radius * 1e-4)
    const sceneBox: Aabb3 = {
      min: [ctx.bounds.min[0] - margin, ctx.bounds.min[1] - margin, ctx.bounds.min[2] - margin],
      max: [ctx.bounds.max[0] + margin, ctx.bounds.max[1] + margin, ctx.bounds.max[2] + margin],
    }
    if (rayAabbDistance(ray, sceneBox) === null) return []
  }

  type HitCursor = {
    meshIndex: number
    mesh: GMesh
    hit: MeshBvhHit
    /** Reusable growing buffer; only the first `excludedCount` entries are live. */
    excludedTriangles: Uint32Array
    excludedCount: number
  }
  const frontier: HitCursor[] = []
  const cursorBefore = (a: HitCursor, b: HitCursor) => a.hit.t < b.hit.t
    || (a.hit.t === b.hit.t && (
      a.meshIndex < b.meshIndex
      || (a.meshIndex === b.meshIndex && a.hit.triangleIndex < b.hit.triangleIndex)
    ))
  const insertFrontier = (cursor: HitCursor) => {
    frontier.push(cursor)
    let child = frontier.length - 1
    while (child > 0) {
      const parent = (child - 1) >>> 1
      if (!cursorBefore(frontier[child], frontier[parent])) break
      ;[frontier[parent], frontier[child]] = [frontier[child], frontier[parent]]
      child = parent
    }
  }
  const takeNearest = () => {
    const nearest = frontier[0]
    const last = frontier.pop()!
    if (frontier.length) {
      frontier[0] = last
      let parent = 0
      while (true) {
        const left = parent * 2 + 1
        const right = left + 1
        if (left >= frontier.length) break
        let child = left
        if (right < frontier.length && cursorBefore(frontier[right], frontier[left])) child = right
        if (!cursorBefore(frontier[child], frontier[parent])) break
        ;[frontier[parent], frontier[child]] = [frontier[child], frontier[parent]]
        parent = child
      }
    }
    return nearest
  }

  const boundsCandidates = querySceneAabbIndex(ctx.sceneAabbIndex, ray)
  let boundsCursor = 0
  const addNextBoundsCandidate = () => {
    const candidate = boundsCandidates[boundsCursor++]
    if (!candidate) return false
    const index = candidate.id
    if (!ctx.isMeshVisible(index)) return true
    const mesh = ctx.meshes[index]
    const hit = ctx.nativePicking.query(mesh.vb, mesh.vertices, mesh.indices, mesh.bvh.vertexStride, mesh.bvh.leafSize, ray, {
      localFromWorld: mesh.inverseTransform,
    })
    if (hit) insertFrontier({ meshIndex: index, mesh, hit, excludedTriangles: new Uint32Array(8), excludedCount: 0 })
    return true
  }

  const candidates: DepthCandidate<PickHit>[] = []
  const seen = new Set<string>()
  let continuations = 0
  while (candidates.length < limit) {
    // Bounds are front-to-back lower bounds. Touch triangle BVHs lazily until
    // no unopened object can beat the exact hit at the frontier. This makes
    // hover/measurement (`limit = 1`) independent of farther overlapping
    // bodies while preserving deterministic depth cycling for larger limits.
    while (!frontier.length && boundsCursor < boundsCandidates.length) addNextBoundsCandidate()
    while (
      frontier.length
      && boundsCursor < boundsCandidates.length
      && boundsCandidates[boundsCursor].distance <= frontier[0].hit.t
    ) addNextBoundsCandidate()
    if (!frontier.length) break

    const cursor = takeNearest()
    const clipped = isSectionClipped(
      cursor.hit.worldPoint,
      ctx.sectionEnabled,
      ctx.sectionNormal,
      ctx.sectionOffset,
    )
    if (!clipped) {
      const { meshIndex, mesh, hit } = cursor
      const value = pickHitFromBvh(meshIndex, mesh, hit, ctx.selectionMode)
      const key = value ? selectionCandidateKey(value, ctx.selectionMode) : null
      if (value && key !== null && !seen.has(key)) {
        seen.add(key)
        candidates.push({ key, distance: hit.t, value })
      }
    }

    const needsContinuation = clipped || ctx.selectionMode !== 'object'
    if (!needsContinuation || continuations >= MAX_DEPTH_CONTINUATIONS || candidates.length >= limit) continue
    continuations++
    // Append-only exclusions: grow the buffer amortized instead of rebuilding
    // an Array.from(Set) on every continuation query.
    if (cursor.excludedCount === cursor.excludedTriangles.length) {
      const grown = new Uint32Array(cursor.excludedTriangles.length * 2)
      grown.set(cursor.excludedTriangles)
      cursor.excludedTriangles = grown
    }
    cursor.excludedTriangles[cursor.excludedCount++] = cursor.hit.triangleIndex
    const next = ctx.nativePicking.query(cursor.mesh.vb, cursor.mesh.vertices, cursor.mesh.indices, cursor.mesh.bvh.vertexStride, cursor.mesh.bvh.leafSize, ray, {
      minT: cursor.hit.t,
      excludedTriangles: cursor.excludedTriangles.subarray(0, cursor.excludedCount),
      localFromWorld: cursor.mesh.inverseTransform,
    })
    if (next) insertFrontier({ ...cursor, hit: next })
  }
  return normalizeDepthCandidates(candidates, limit)
}
