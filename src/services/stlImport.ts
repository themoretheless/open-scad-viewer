import {decodeMeshBytesInKernel,renderTriangleSoupInKernel} from './geometry/meshImportKernel'
import {GeometryKernelError} from './geometry/kernel'
import type { MeshData, SceneEntityId } from '../core/mesh'
import { geometryAssetId } from '../core/scene'
import { buildMeshBvh } from './meshBvh'
import { extractSemanticEdges } from './meshTopology'
import { identity } from './math3d'

/** Conservative first-release cap: final artifacts plus topology scratch stay bounded. */
export const MAX_STL_IMPORT_TRIANGLES = 250_000
export const MAX_STL_IMPORT_BYTES = 84 + MAX_STL_IMPORT_TRIANGLES * 50

export type StlImportErrorCode =
  | 'unsupported-ascii'
  | 'invalid-header'
  | 'invalid-size'
  | 'too-many-triangles'
  | 'non-finite-coordinate'
  | 'empty-mesh'

export class StlImportError extends Error {
  constructor(readonly code: StlImportErrorCode, message: string) {
    super(message)
    this.name = 'StlImportError'
  }
}

/** Neutral, renderer-independent triangle soup decoded from a binary STL. */
export interface ImportedStl {
  readonly triangleCount: number
  /** Nine position floats per triangle. File normals are deliberately ignored. */
  readonly positions: Float32Array
}

/** Strict binary decoding is owned by mesh-io. */
export function parseBinaryStl(buffer: ArrayBuffer): ImportedStl {
 const positions = Float32Array.from(stlNative<number[]>('stl_decode_binary', {bytes: new Uint8Array(buffer)}))
 return Object.freeze({triangleCount: positions.length / 9, positions})
}
function stlNative<T>(op: string, payload: Record<string, unknown>): T {
 try { return (op==='stl_decode_binary' ? decodeMeshBytesInKernel('binary-stl',payload.bytes as Uint8Array).positions : renderTriangleSoupInKernel(payload.positions as Float32Array)) as T }
 catch (error) {
  if (error instanceof GeometryKernelError && error.code.startsWith('STL_')) {
   throw new StlImportError(error.code.slice(4).toLowerCase().replaceAll('_', '-') as StlImportErrorCode, error.message)
  }
  throw error
 }
}

/** Convert decoded STL geometry into the complete eager scene-mesh contract. */
export function importedStlToMeshData(
  imported: ImportedStl,
  color: [number, number, number, number] = [0.7, 0.7, 0.72, 1],
): MeshData {
  if (!Number.isSafeInteger(imported.triangleCount)
    || imported.triangleCount <= 0
    || imported.triangleCount > MAX_STL_IMPORT_TRIANGLES
    || imported.positions.length !== imported.triangleCount * 9) {
    throw new StlImportError('invalid-size', 'Decoded STL triangle storage is inconsistent')
  }
  if (!imported.positions.every(Number.isFinite)) {
    throw new StlImportError('non-finite-coordinate', 'Decoded STL contains a non-finite coordinate')
  }
  if (!color.every(value => Number.isFinite(value) && value >= 0 && value <= 1)) {
    throw new RangeError('STL mesh color channels must be finite values in [0, 1]')
  }

  const rendered = stlNative<{vertices: number[]; indices: number[]; faceIds: number[]; discarded: number}>(
    'mesh_soup_render', {positions: imported.positions})
  const vertices = Float32Array.from(rendered.vertices), indices = Uint32Array.from(rendered.indices)
  const faceIds = Uint32Array.from(rendered.faceIds), validTriangles = faceIds.length

  const semanticEdges = extractSemanticEdges(vertices, indices)
  const assetId = geometryAssetId(vertices, indices)
  const entityId = `entity:import/stl/${assetId.slice('asset:'.length)}` as SceneEntityId
  return {
    entityId,
    geometryAssetId: assetId,
    vertices,
    indices,
    bvh: buildMeshBvh(vertices, indices),
    edgeIndices: semanticEdges.indices,
    color: [...color],
    transform: identity(),
    faceIds,
    provenance: [{ triangleStart: 0, triangleEnd: validTriangles, source: null, backside: false }],
    topology: {
      ...semanticEdges.diagnostics,
      degenerate: semanticEdges.diagnostics.degenerate + imported.triangleCount - validTriangles,
    },
  }
}

/** Parse and adapt while keeping the two contracts independently testable. */
export function binaryStlToMeshData(buffer: ArrayBuffer): MeshData {
  return importedStlToMeshData(parseBinaryStl(buffer))
}
