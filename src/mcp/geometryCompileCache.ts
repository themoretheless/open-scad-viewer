import type { GeometryExecutionDescriptor } from '../core/geometryExecution'
import type { MeshBvh, MeshData, MeshLodLevel, MeshProvenanceRun } from '../core/mesh'
import { sha256Hex } from '../core/sha256'
import type { CompiledGeometry } from './geometryService'

export const DEFAULT_COMPILE_CACHE_MAX_ENTRIES = 32
export const DEFAULT_COMPILE_CACHE_MAX_BYTES = 32 * 1024 * 1024
const ESTIMATED_ANALYSIS_OVERHEAD_BYTES = 4_096

export interface GeometryCompileCacheStats {
  readonly entries: number
  readonly bytes: number
  readonly hits: number
  readonly misses: number
  readonly evictions: number
}

interface CacheEntry {
  readonly key: string
  readonly value: CompiledGeometry
  readonly bytes: number
}

/**
 * Derive a content-addressed key bound to the resolved execution route,
 * kernel identity, manifest digest, quality, purpose, and exact source hash.
 */
export function buildCompileCacheKey(planned: GeometryExecutionDescriptor, sourceSha256: string): string {
  return sha256Hex(
    JSON.stringify([
      planned.languageContract,
      planned.engineClass,
      planned.engineKey,
      planned.kernelFingerprint,
      planned.capabilityManifestVersion,
      planned.manifestDigest,
      planned.quality,
      planned.purpose,
      planned.representation,
      sourceSha256,
      ...planned.requiredCapabilities,
    ]),
  )
}

function estimateBvhBytes(bvh: MeshBvh): number {
  return bvh.bounds.byteLength + bvh.nodes.byteLength + bvh.triangles.byteLength
}

export function estimateCompiledGeometryBytes(value: CompiledGeometry): number {
  let bytes = ESTIMATED_ANALYSIS_OVERHEAD_BYTES
  for (const mesh of value.meshes) {
    bytes +=
      mesh.vertices.byteLength +
      mesh.indices.byteLength +
      mesh.edgeIndices.byteLength +
      mesh.faceIds.byteLength +
      mesh.transform.byteLength +
      estimateBvhBytes(mesh.bvh)
    if (mesh.nativeGeometry) {
      bytes += 2 * (mesh.nativeGeometry.geometryJson.length + mesh.nativeGeometry.documentJson.length)
    }
    for (const level of mesh.lods ?? []) {
      bytes +=
        level.vertices.byteLength +
        level.indices.byteLength +
        level.edgeIndices.byteLength +
        level.faceIds.byteLength +
        estimateBvhBytes(level.bvh)
    }
  }
  return bytes
}

function cloneBvh(bvh: MeshBvh): MeshBvh {
  return {
    version: bvh.version,
    vertexStride: bvh.vertexStride,
    leafSize: bvh.leafSize,
    nodeCount: bvh.nodeCount,
    bounds: new Float32Array(bvh.bounds),
    nodes: new Uint32Array(bvh.nodes),
    triangles: new Uint32Array(bvh.triangles),
  }
}

function cloneProvenance(runs: readonly MeshProvenanceRun[]): MeshProvenanceRun[] {
  return runs.map(run => ({
    triangleStart: run.triangleStart,
    triangleEnd: run.triangleEnd,
    backside: run.backside,
    source: run.source ? { ...run.source } : null,
  }))
}

function cloneLodLevel(level: MeshLodLevel): MeshLodLevel {
  return {
    vertices: new Float32Array(level.vertices),
    indices: new Uint32Array(level.indices),
    edgeIndices: new Uint32Array(level.edgeIndices),
    faceIds: new Uint32Array(level.faceIds),
    bvh: cloneBvh(level.bvh),
    provenance: cloneProvenance(level.provenance),
    triangleCount: level.triangleCount,
    errorUpper: level.errorUpper,
  }
}

function cloneMeshData(mesh: MeshData): MeshData {
  return {
    ...(mesh.nativeGeometry ? { nativeGeometry: structuredClone(mesh.nativeGeometry) } : {}),
    ...(mesh.entityId !== undefined ? { entityId: mesh.entityId } : {}),
    ...(mesh.geometryAssetId !== undefined ? { geometryAssetId: mesh.geometryAssetId } : {}),
    vertices: new Float32Array(mesh.vertices),
    indices: new Uint32Array(mesh.indices),
    bvh: cloneBvh(mesh.bvh),
    edgeIndices: new Uint32Array(mesh.edgeIndices),
    color: [...mesh.color] as [number, number, number, number],
    ...(mesh.material
      ? {
          material: {
            ...mesh.material,
            baseColor: [...mesh.material.baseColor] as [number, number, number],
            emissive: [...mesh.material.emissive] as [number, number, number],
          },
        }
      : {}),
    transform: new Float32Array(mesh.transform),
    faceIds: new Uint32Array(mesh.faceIds),
    ...(mesh.faceIdsAuthoritative !== undefined ? { faceIdsAuthoritative: mesh.faceIdsAuthoritative } : {}),
    ...(mesh.faceIdsInferred !== undefined ? { faceIdsInferred: mesh.faceIdsInferred } : {}),
    provenance: cloneProvenance(mesh.provenance),
    topology: { ...mesh.topology },
    ...(mesh.lods ? { lods: mesh.lods.map(cloneLodLevel) } : {}),
  }
}

export function cloneCompiledGeometry(value: CompiledGeometry): CompiledGeometry {
  return {
    analysis: structuredClone(value.analysis),
    meshes: value.meshes.map(cloneMeshData),
  }
}

export interface GeometryCompileCacheOptions {
  maxEntries?: number | undefined
  maxBytes?: number | undefined
}

/**
 * Bounded LRU cache for compiled geometry results. Every read and write clones
 * buffers so callers cannot alias or mutate cached typed arrays.
 */
export class GeometryCompileCache {
  private readonly entries = new Map<string, CacheEntry>()
  private readonly maxEntries: number
  private readonly maxBytes: number
  private totalBytes = 0
  private hitCount = 0
  private missCount = 0
  private evictionCount = 0

  constructor(
    maxEntriesOrOptions: number | GeometryCompileCacheOptions = DEFAULT_COMPILE_CACHE_MAX_ENTRIES,
    maxBytesArg = DEFAULT_COMPILE_CACHE_MAX_BYTES,
  ) {
    const maxEntries =
      typeof maxEntriesOrOptions === 'number'
        ? maxEntriesOrOptions
        : (maxEntriesOrOptions.maxEntries ?? DEFAULT_COMPILE_CACHE_MAX_ENTRIES)
    const maxBytes =
      typeof maxEntriesOrOptions === 'number' ? maxBytesArg : (maxEntriesOrOptions.maxBytes ?? maxBytesArg)
    if (!Number.isSafeInteger(maxEntries) || maxEntries < 0) {
      throw new RangeError('maxEntries must be a non-negative safe integer')
    }
    if (!Number.isSafeInteger(maxBytes) || maxBytes < 0) {
      throw new RangeError('maxBytes must be a non-negative safe integer')
    }
    this.maxEntries = maxEntries
    this.maxBytes = maxBytes
  }

  get(key: string): CompiledGeometry | null {
    const entry = this.entries.get(key)
    if (!entry) {
      this.missCount += 1
      return null
    }
    // Promote to most-recently-used position without mutating byte accounting.
    this.entries.delete(key)
    this.entries.set(key, entry)
    this.hitCount += 1
    return cloneCompiledGeometry(entry.value)
  }

  set(key: string, value: CompiledGeometry): boolean {
    if (this.maxEntries === 0 || this.maxBytes === 0) return false
    const cloned = cloneCompiledGeometry(value)
    const bytes = estimateCompiledGeometryBytes(cloned)
    if (bytes > this.maxBytes) return false

    const existing = this.entries.get(key)
    if (existing) {
      this.entries.delete(key)
      this.totalBytes -= existing.bytes
    }

    this.entries.set(key, { key, value: cloned, bytes })
    this.totalBytes += bytes
    this.evictIfNeeded()
    return true
  }

  clear(): void {
    this.entries.clear()
    this.totalBytes = 0
  }

  stats(): GeometryCompileCacheStats {
    return Object.freeze({
      entries: this.entries.size,
      bytes: this.totalBytes,
      hits: this.hitCount,
      misses: this.missCount,
      evictions: this.evictionCount,
    })
  }

  private evictIfNeeded(): void {
    while (this.entries.size > this.maxEntries || this.totalBytes > this.maxBytes) {
      const oldestKey = this.entries.keys().next().value
      if (oldestKey === undefined) break
      const oldest = this.entries.get(oldestKey)
      this.entries.delete(oldestKey)
      if (oldest) this.totalBytes -= oldest.bytes
      this.evictionCount += 1
    }
  }
}
