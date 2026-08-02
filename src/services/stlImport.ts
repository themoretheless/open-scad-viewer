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

const HEADER_BYTES = 84
const TRIANGLE_BYTES = 50

/**
 * Decode a binary STL using exact-size and allocation bounds.
 *
 * STL normals are untrusted derived data, so only vertex positions cross this
 * boundary. Normals and inspection artifacts are rebuilt by the scene adapter.
 */
export function parseBinaryStl(buffer: ArrayBuffer): ImportedStl {
  if (buffer.byteLength < HEADER_BYTES) {
    if (startsWithAsciiSolid(new Uint8Array(buffer))) {
      throw new StlImportError('unsupported-ascii', 'ASCII STL is not supported; choose a binary STL')
    }
    throw new StlImportError('invalid-header', 'Binary STL is shorter than its 84-byte header')
  }

  const view = new DataView(buffer)
  const triangleCount = view.getUint32(80, true)
  if (triangleCount > MAX_STL_IMPORT_TRIANGLES) {
    throw new StlImportError(
      'too-many-triangles',
      `STL exceeds the ${MAX_STL_IMPORT_TRIANGLES.toLocaleString()} triangle import limit`,
    )
  }

  const expectedBytes = HEADER_BYTES + triangleCount * TRIANGLE_BYTES
  if (buffer.byteLength !== expectedBytes) {
    if (startsWithAsciiSolid(new Uint8Array(buffer))) {
      throw new StlImportError('unsupported-ascii', 'ASCII STL is not supported; choose a binary STL')
    }
    throw new StlImportError('invalid-size', 'Binary STL size does not match its declared triangle count')
  }

  const positions = new Float32Array(triangleCount * 9)
  let recordOffset = HEADER_BYTES
  let targetOffset = 0
  for (let triangle = 0; triangle < triangleCount; triangle++) {
    let vertexOffset = recordOffset + 12 // Ignore the stored face normal.
    for (let channel = 0; channel < 9; channel++, vertexOffset += 4) {
      const value = view.getFloat32(vertexOffset, true)
      if (!Number.isFinite(value)) {
        throw new StlImportError(
          'non-finite-coordinate',
          `STL triangle ${triangle + 1} contains a non-finite coordinate`,
        )
      }
      positions[targetOffset++] = value
    }
    recordOffset += TRIANGLE_BYTES
  }

  return Object.freeze({ triangleCount, positions })
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

  let validTriangles = 0
  for (let triangle = 0; triangle < imported.triangleCount; triangle++) {
    const normal = triangleNormal(imported.positions, triangle * 9)
    if (normal) validTriangles++
  }
  if (validTriangles === 0) {
    throw new StlImportError('empty-mesh', 'STL contains no non-degenerate triangles')
  }

  const vertices = new Float32Array(validTriangles * 18)
  const indices = new Uint32Array(validTriangles * 3)
  const faceIds = new Uint32Array(validTriangles)
  let outputTriangle = 0
  for (let triangle = 0; triangle < imported.triangleCount; triangle++) {
    const sourceOffset = triangle * 9
    const normal = triangleNormal(imported.positions, sourceOffset)
    if (!normal) continue
    for (let vertex = 0; vertex < 3; vertex++) {
      const outputVertex = outputTriangle * 3 + vertex
      const target = outputVertex * 6
      const source = sourceOffset + vertex * 3
      vertices[target] = imported.positions[source]
      vertices[target + 1] = imported.positions[source + 1]
      vertices[target + 2] = imported.positions[source + 2]
      vertices[target + 3] = normal[0]
      vertices[target + 4] = normal[1]
      vertices[target + 5] = normal[2]
      indices[outputVertex] = outputVertex
    }
    faceIds[outputTriangle] = outputTriangle
    outputTriangle++
  }

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

function triangleNormal(positions: Float32Array, offset: number): [number, number, number] | null {
  const ax = positions[offset], ay = positions[offset + 1], az = positions[offset + 2]
  const abx = positions[offset + 3] - ax
  const aby = positions[offset + 4] - ay
  const abz = positions[offset + 5] - az
  const acx = positions[offset + 6] - ax
  const acy = positions[offset + 7] - ay
  const acz = positions[offset + 8] - az
  const nx = aby * acz - abz * acy
  const ny = abz * acx - abx * acz
  const nz = abx * acy - aby * acx
  const length = Math.hypot(nx, ny, nz)
  if (!(length > 0) || !Number.isFinite(length)) return null
  return [nx / length, ny / length, nz / length]
}

function startsWithAsciiSolid(bytes: Uint8Array): boolean {
  let offset = bytes.length >= 3 && bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf ? 3 : 0
  while (offset < bytes.length && (
    bytes[offset] === 0x09 || bytes[offset] === 0x0a || bytes[offset] === 0x0b
    || bytes[offset] === 0x0c || bytes[offset] === 0x0d || bytes[offset] === 0x20
  )) offset++
  const token = [0x73, 0x6f, 0x6c, 0x69, 0x64]
  return token.every((value, index) => (bytes[offset + index] | 0x20) === value)
}
