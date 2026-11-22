import type { MeshData } from '../core/mesh'
import type { ExportScenePartTrace } from '../parser/types'
import { flattenGroupGeometry } from '../kernel/meshFlatten'
import { inspectNurbsMesh, type NurbsMesh } from '../kernel/geometry/tessellation'

export interface ExportPartMetadata {
  readonly name: string
  readonly groupKey?: string | undefined
  readonly groupName?: string | undefined
  readonly color?: readonly [number, number, number, number] | undefined
}

export type ExportSceneMesh = NurbsMesh & {
  parts: NurbsMesh[]
  partMeta?: ExportPartMetadata[]
  title?: string
}

type Aabb3 = readonly [number, number, number, number, number, number]

const INTERNAL_CSG_MODULES = new Set([
  'clip_gusset_floor',
  'side_panel',
  'rr',
  'slot_strip_2d',
  'slot_strips_2d',
  'slot_strips_raw_2d',
  'gusset_profile_2d',
  'inner_cavity_2d',
  'floor_cavity_2d',
  'floor_cutters_2d',
  'floor_lattice_raw_2d',
  'floor_open_domain_2d',
  'wall_openings_2d',
])

const CLUSTER_GAP_MM = 0.35

function boundsFromPositions(positions: ArrayLike<number>): Aabb3 {
  let minX = Infinity
  let minY = Infinity
  let minZ = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  let maxZ = -Infinity
  for (let i = 0; i < positions.length; i += 3) {
    const x = positions[i]!
    const y = positions[i + 1]!
    const z = positions[i + 2]!
    if (x < minX) minX = x
    if (x > maxX) maxX = x
    if (y < minY) minY = y
    if (y > maxY) maxY = y
    if (z < minZ) minZ = z
    if (z > maxZ) maxZ = z
  }
  return [minX, minY, minZ, maxX, maxY, maxZ]
}

function boundsFromDisplayMesh(mesh: Pick<MeshData, 'vertices' | 'transform'>): Aabb3 {
  const v = mesh.vertices
  const m = mesh.transform
  const hasT = Array.isArray(m) || ArrayBuffer.isView(m) ? m.length === 16 : false
  let minX = Infinity
  let minY = Infinity
  let minZ = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  let maxZ = -Infinity
  for (let i = 0; i < v.length; i += 6) {
    const x = v[i]!
    const y = v[i + 1]!
    const z = v[i + 2]!
    const wx = hasT ? m[0]! * x + m[1]! * y + m[2]! * z + m[3]! : x
    const wy = hasT ? m[4]! * x + m[5]! * y + m[6]! * z + m[7]! : y
    const wz = hasT ? m[8]! * x + m[9]! * y + m[10]! * z + m[11]! : z
    if (wx < minX) minX = wx
    if (wx > maxX) maxX = wx
    if (wy < minY) minY = wy
    if (wy > maxY) maxY = wy
    if (wz < minZ) minZ = wz
    if (wz > maxZ) maxZ = wz
  }
  return [minX, minY, minZ, maxX, maxY, maxZ]
}

function boxesTouch(a: Aabb3, b: Aabb3): boolean {
  return (
    a[3] + CLUSTER_GAP_MM >= b[0] &&
    b[3] + CLUSTER_GAP_MM >= a[0] &&
    a[4] + CLUSTER_GAP_MM >= b[1] &&
    b[4] + CLUSTER_GAP_MM >= a[1] &&
    a[5] + CLUSTER_GAP_MM >= b[2] &&
    b[5] + CLUSTER_GAP_MM >= a[2]
  )
}

function clusterIndicesByProximity(indices: readonly number[], bounds: readonly Aabb3[]): number[][] {
  if (indices.length <= 1) return indices.length === 1 ? [[indices[0]!]] : []
  const visited = new Uint8Array(indices.length)
  const clusters: number[][] = []
  for (let start = 0; start < indices.length; start++) {
    if (visited[start]) continue
    visited[start] = 1
    const queue = [start]
    const cluster: number[] = []
    while (queue.length > 0) {
      const u = queue.shift()!
      const uIdx = indices[u]!
      cluster.push(uIdx)
      const uBox = bounds[uIdx]!
      for (let v = 0; v < indices.length; v++) {
        if (visited[v]) continue
        if (boxesTouch(uBox, bounds[indices[v]!]!)) {
          visited[v] = 1
          queue.push(v)
        }
      }
    }
    cluster.sort((a, b) => a - b)
    clusters.push(cluster)
  }
  return clusters
}

function matchPartTraces(
  nonEmptyMeshes: readonly MeshData[],
  traces?: readonly ExportScenePartTrace[],
): Array<ExportScenePartTrace | undefined> {
  if (!traces || traces.length === 0)
    return new Array<ExportScenePartTrace | undefined>(nonEmptyMeshes.length).fill(undefined)
  const used = new Uint8Array(traces.length)
  const matched: Array<ExportScenePartTrace | undefined> = new Array(nonEmptyMeshes.length).fill(undefined)
  for (let i = 0; i < nonEmptyMeshes.length; i++) {
    const entityId = nonEmptyMeshes[i]!.entityId
    if (!entityId) continue
    for (let j = 0; j < traces.length; j++) {
      if (!used[j] && traces[j]!.entityId === entityId) {
        used[j] = 1
        matched[i] = traces[j]
        break
      }
    }
  }
  if (traces.length === nonEmptyMeshes.length) {
    for (let i = 0; i < nonEmptyMeshes.length; i++) {
      if (!matched[i] && !used[i]) {
        used[i] = 1
        matched[i] = traces[i]
      }
    }
  }
  return matched
}

function resolveRawPartName(
  mesh: MeshData,
  trace: ExportScenePartTrace | undefined,
  isGrouped: boolean,
  isSinglePartTopModule: boolean,
): string {
  const meaningfulStack = trace ? trace.moduleStack.filter(name => !INTERNAL_CSG_MODULES.has(name)) : []
  const last = meaningfulStack[meaningfulStack.length - 1]
  let raw: string | undefined
  if (last === 'solid_corner_gusset') {
    raw = 'corner_gusset'
  } else if (last === 'wall_gusset_local') {
    const parent = meaningfulStack.length >= 2 ? meaningfulStack[meaningfulStack.length - 2]! : 'wall'
    raw = `${parent}_gusset`
  } else if (isSinglePartTopModule && meaningfulStack.length >= 1) {
    raw = meaningfulStack[0]
  } else if (meaningfulStack.length >= 2) {
    raw = last
  } else if (meaningfulStack.length === 1) {
    raw = isGrouped ? `${last}_${trace!.callName}` : last
  } else {
    const provenanceLabel = mesh.provenance?.find(run => run.source !== null)?.source?.label.replace(/\(\)$/, '')
    raw = trace?.callName ?? provenanceLabel
  }
  return (raw ?? 'part').trim().replace(/\s+/g, '_') || 'part'
}

function derivePartMetadataFromBounds(
  nonEmptyMeshes: readonly MeshData[],
  boundsList: readonly Aabb3[],
  traces?: readonly ExportScenePartTrace[],
): ExportPartMetadata[] {
  const n = nonEmptyMeshes.length
  if (n === 0) return []
  const matched = matchPartTraces(nonEmptyMeshes, traces)

  const buckets = new Map<string, number[]>()
  for (let i = 0; i < n; i++) {
    const trace = matched[i]
    if (!trace?.topModuleInstancePath || trace.moduleStack.length === 0) continue
    const existing = buckets.get(trace.topModuleInstancePath)
    if (existing) existing.push(i)
    else buckets.set(trace.topModuleInstancePath, [i])
  }

  const groupKeys: Array<string | undefined> = new Array(n).fill(undefined)
  const groupNames: Array<string | undefined> = new Array(n).fill(undefined)
  const multiClusters: Array<{ clusterId: string; baseName: string; members: number[] }> = []

  let clusterSeq = 0
  for (const [, bucketIndices] of buckets) {
    if (bucketIndices.length <= 1) {
      const idx = bucketIndices[0]!
      groupNames[idx] = matched[idx]!.moduleStack[0]
      continue
    }
    const clusters = clusterIndicesByProximity(bucketIndices, boundsList)
    for (const cluster of clusters) {
      if (cluster.length >= 2) {
        const clusterId = `grp:${++clusterSeq}`
        const baseName = matched[cluster[0]!]!.moduleStack[0]!
        multiClusters.push({ clusterId, baseName, members: cluster })
      } else {
        const idx = cluster[0]!
        groupNames[idx] = matched[idx]!.moduleStack[0]
      }
    }
  }

  const baseNameTotals = new Map<string, number>()
  for (const c of multiClusters) baseNameTotals.set(c.baseName, (baseNameTotals.get(c.baseName) ?? 0) + 1)
  const baseNameSeen = new Map<string, number>()
  for (const c of multiClusters) {
    const total = baseNameTotals.get(c.baseName) ?? 1
    const idx = (baseNameSeen.get(c.baseName) ?? 0) + 1
    baseNameSeen.set(c.baseName, idx)
    const resolvedGroupName = total > 1 ? `${c.baseName}_${idx}` : c.baseName
    for (const partIndex of c.members) {
      groupKeys[partIndex] = c.clusterId
      groupNames[partIndex] = resolvedGroupName
    }
  }

  const baseNames = nonEmptyMeshes.map((mesh, i) => {
    const trace = matched[i]
    const bucketSize = trace?.topModuleInstancePath ? (buckets.get(trace.topModuleInstancePath)?.length ?? 0) : 0
    return resolveRawPartName(mesh, trace, groupKeys[i] !== undefined, bucketSize === 1)
  })

  const scopeTotals = new Map<string, number>()
  for (let i = 0; i < n; i++) {
    const scopeKey = `${groupKeys[i] ?? '__standalone__'}::${baseNames[i]!}`
    scopeTotals.set(scopeKey, (scopeTotals.get(scopeKey) ?? 0) + 1)
  }
  const scopeSeen = new Map<string, number>()
  return nonEmptyMeshes.map((mesh, i) => {
    const base = baseNames[i]!
    const scopeKey = `${groupKeys[i] ?? '__standalone__'}::${base}`
    const total = scopeTotals.get(scopeKey) ?? 1
    const seq = (scopeSeen.get(scopeKey) ?? 0) + 1
    scopeSeen.set(scopeKey, seq)
    const name = total > 1 ? `${base}_${seq}` : base
    const resolvedColor =
      Array.isArray(mesh.color) && mesh.color.length === 4
        ? ([...mesh.color] as const)
        : matched[i]?.color
          ? ([...matched[i]!.color] as const)
          : undefined
    return {
      name,
      ...(groupKeys[i] !== undefined ? { groupKey: groupKeys[i] } : {}),
      ...(groupNames[i] !== undefined ? { groupName: groupNames[i] } : {}),
      ...(resolvedColor ? { color: resolvedColor } : {}),
    }
  })
}

export function deriveExportPartMetadata(
  nonEmptyMeshes: readonly MeshData[],
  flattenedParts: readonly NurbsMesh[],
  traces?: readonly ExportScenePartTrace[],
): ExportPartMetadata[] {
  return derivePartMetadataFromBounds(
    nonEmptyMeshes,
    flattenedParts.map(p => boundsFromPositions(p.positions)),
    traces,
  )
}

export function deriveExportPartMetadataForDisplay(
  meshes: readonly MeshData[],
  traces?: readonly ExportScenePartTrace[],
): ExportPartMetadata[] {
  const nonEmpty = meshes.filter(m => m.indices.length > 0)
  return derivePartMetadataFromBounds(nonEmpty, nonEmpty.map(boundsFromDisplayMesh), traces)
}

function flattenGroup(meshes: readonly MeshData[]): NurbsMesh {
  const { positions, indices } = flattenGroupGeometry(meshes)
  const report = inspectNurbsMesh(positions, indices)
  return {
    positions,
    indices,
    report: {
      ...report,
      construction: 'sampled_surface',
      errorBoundCertified: false,
      selfIntersectionStatus: 'not_checked',
    },
  }
}

/** Preserve scene objects and hierarchy metadata for 3MF/OBJ/AMF while keeping flat mesh for legacy formats. */
export function flattenExportMeshes(
  meshes: readonly MeshData[],
  traces?: readonly ExportScenePartTrace[],
  title?: string,
): ExportSceneMesh {
  const combined = flattenGroup(meshes)
  const nonEmpty = meshes.filter(m => m.indices.length > 0)
  const parts = nonEmpty.map(m => flattenGroup([m]))
  const partMeta = parts.length > 0 ? deriveExportPartMetadata(nonEmpty, parts, traces) : undefined
  return {
    ...combined,
    parts,
    ...(partMeta ? { partMeta } : {}),
    ...(title ? { title } : {}),
  }
}
