import type {NativeGeometryArtifact} from './nativeGeometry'
/** Stable identity of a static geometry operation in the parsed source tree. */
export type SourceOperationId = `op:${string}`

/** Stable identity of one evaluated operation instance in the scene. */
export type SceneEntityId = `entity:${string}`

export interface MeshSourceReference {
  /** Legacy source-selection key. Prefer operationId/instanceId for identity. */
  id: number
  /** Static operation identity; stable across whitespace, quality and unrelated sibling edits. */
  operationId?: SourceOperationId
  /** Evaluated instance identity; distinguishes module calls and loop iterations. */
  instanceId?: SceneEntityId
  originalId: number
  start: number
  end: number
  label: string
}

export interface MeshProvenanceRun {
  triangleStart: number
  triangleEnd: number
  source: MeshSourceReference | null
  backside: boolean
}

/** Compact, transferable triangle hierarchy used for CPU picking. */
export interface MeshBvh {
  readonly version: 1
  readonly vertexStride: number
  readonly leafSize: number
  readonly nodeCount: number
  readonly bounds: Float32Array
  readonly nodes: Uint32Array
  readonly triangles: Uint32Array
}

export interface MeshTopologyDiagnostics {
  boundary: number
  crease: number
  nonManifold: number
  degenerate: number
}

/** Renderer-neutral output of a geometry build. */
export interface MeshData {
  /** Native source snapshot; render vertices are not its authority. */
  nativeGeometry?: NativeGeometryArtifact
  /** Stable identity for this evaluated scene entity, independent of tessellation quality. */
  entityId?: SceneEntityId
  /** Verified identity of the exact vertex/index payload, independent of entity presentation. */
  geometryAssetId?: `asset:${string}`
  /** Interleaved position(3) + normal(3). */
  vertices: Float32Array
  indices: Uint32Array
  bvh: MeshBvh
  edgeIndices: Uint32Array
  color: [number, number, number, number]
  /** Row-major affine transform. */
  transform: Float32Array
  /** Manifold coplanar-face identifier for every triangle. */
  faceIds: Uint32Array
  /** True when non-authoritative IDs are already connected smooth surface groups from the geometry kernel. */
  faceIdsSurfaceGroups?: boolean
  /** True when IDs refer to authored surfaces/B-rep faces rather than mesh facets. */
  faceIdsAuthoritative?: boolean
  /** Compact triangle runs mapped back to the source operation that created them. */
  provenance: MeshProvenanceRun[]
  topology: MeshTopologyDiagnostics
}

/**
 * Return every transferable buffer owned by a mesh publication exactly once.
 * SharedArrayBuffer-backed views are intentionally omitted because they cannot
 * appear in a postMessage transfer list.
 */
export function meshTransferables(meshes: readonly MeshData[]): ArrayBuffer[] {
  const buffers = new Set<ArrayBuffer>()
  const add = (buffer: ArrayBufferLike) => {
    if (buffer instanceof ArrayBuffer) buffers.add(buffer)
  }

  for (const mesh of meshes) {
    add(mesh.vertices.buffer)
    add(mesh.indices.buffer)
    add(mesh.edgeIndices.buffer)
    add(mesh.transform.buffer)
    add(mesh.faceIds.buffer)
    add(mesh.bvh.bounds.buffer)
    add(mesh.bvh.nodes.buffer)
    add(mesh.bvh.triangles.buffer)
  }

  return [...buffers]
}
