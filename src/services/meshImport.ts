import {decodeMeshBytesInKernel} from './geometry/meshImportKernel'
import {callGeometryRust, GeometryKernelError, warmGeometryKernel} from './geometry/kernel'
/**
 * Unified mesh file import. Decodes STL (ASCII/binary), OBJ, PLY (ASCII/binary),
 * OFF, AMF and 3MF into one welded, indexed triangle mesh that the mesh
 * modeler, the Solid workbench and the format converter can all consume.
 *
 * STL/OFF/OBJ/PLY parsing and all welding run in the Rust mesh-io kernel.
 * AMF/3MF reuse the bounded OpenSCAD XML/ZIP decoders. File normals, colours, UVs, textures
 * and per-object identities are deliberately dropped: only positions and
 * triangle connectivity cross this boundary.
 */
import { sha256Hex } from '../core/sha256'
import {
  OPENSCAD_IMPORT_MAX_TRIANGLES,
  OPENSCAD_IMPORT_MAX_VERTICES,
  OpenScadImportDataError,
  parseOpenScad3mf,
  parseOpenScadAmf,
  type OpenScadImportGeometry3D,
} from './openScadImport'
import type { OpenScadProjectBlobFile } from './openScadProject'
import type { PolygonMesh } from './geometry/polygon'
import { detectMeshImportFormat, MESH_IMPORT_FORMATS, MESH_IMPORT_MAX_BYTES, type MeshImportFormat } from './meshFormats'

export { detectMeshImportFormat, MESH_IMPORT_ACCEPT, MESH_IMPORT_FORMATS, MESH_IMPORT_MAX_BYTES, stripMeshExtension, type MeshImportFormat } from './meshFormats'

export const MESH_IMPORT_MAX_TRIANGLES = OPENSCAD_IMPORT_MAX_TRIANGLES
export const MESH_IMPORT_MAX_VERTICES = OPENSCAD_IMPORT_MAX_VERTICES

export type MeshImportErrorCode =
  | 'unsupported-format'
  | 'too-large'
  | 'encoding'
  | 'invalid-data'
  | 'limit'
  | 'empty'

export class MeshImportError extends Error {
  constructor(readonly code: MeshImportErrorCode, message: string, readonly format?: MeshImportFormat) {
    super(message)
    this.name = 'MeshImportError'
  }
}

export interface ImportedMesh extends PolygonMesh {
  readonly format: MeshImportFormat
  readonly triangleCount: number
  readonly vertexCount: number
  /** Vertex count before welding; equal to `vertexCount` for already indexed files. */
  readonly sourceVertexCount: number
  /** Triangles dropped because welding collapsed them. */
  readonly degenerateTriangles: number
}

export interface MeshImportOptions {
  /** Override extension-based detection. */
  readonly format?: MeshImportFormat
  /**
   * Absolute welding tolerance. `0` (default) merges only bit-identical
   * positions, which is enough to close well-formed STL/PLY soups without
   * distorting geometry. `false` keeps the file's own indexing.
   */
  readonly weld?: number | false
  readonly maxBytes?: number
}

/** Decode a mesh file into one welded indexed triangle mesh. */
export async function importMeshFile(
  fileName: string,
  input: Uint8Array | ArrayBuffer,
  options: MeshImportOptions = {},
): Promise<ImportedMesh> {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input)
  const maxBytes = options.maxBytes ?? MESH_IMPORT_MAX_BYTES
  if (bytes.byteLength > maxBytes) {
    throw new MeshImportError('too-large', `${fileName} exceeds the ${(maxBytes / 1_000_000).toFixed(0)} MB import limit.`)
  }
  const format = options.format ?? detectMeshImportFormat(fileName, bytes)
  if (!format) {
    throw new MeshImportError('unsupported-format', `Cannot detect a supported mesh format for ${fileName}. Supported: ${MESH_IMPORT_FORMATS.map(f => f.toUpperCase()).join(', ')}.`)
  }
  await warmGeometryKernel()
  const raw = await decode(format, fileName, bytes)
  return finalize(format, raw.positions, raw.indices, options.weld ?? 0)
}

/** Convenience wrapper for browser `File` objects. */
export async function importMeshFromFile(file: Blob & { name?: string }, options: MeshImportOptions = {}): Promise<ImportedMesh> {
  return importMeshFile(file.name ?? 'mesh', await file.arrayBuffer(), options)
}

interface RawMesh { positions: ArrayLike<number>; indices: ArrayLike<number> }

async function decode(format: MeshImportFormat, fileName: string, bytes: Uint8Array): Promise<RawMesh> {
  try {
    switch (format) {
      case 'stl': return nativeImport(format, bytes)
      case 'off': return nativeImport(format, bytes)
      case 'amf': return fromOpenScad(await parseOpenScadAmf(blobFile(fileName, bytes)))
      case '3mf': return fromOpenScad(await parseOpenScad3mf(blobFile(fileName, bytes)))
      case 'obj': return nativeImport(format, bytes)
      case 'ply': return nativeImport(format, bytes)
    }
  } catch (error) {
    if (error instanceof MeshImportError) throw error
    if (error instanceof OpenScadImportDataError) {
      const code: MeshImportErrorCode = error.code === 'E_IMPORT_LIMIT' ? 'limit'
        : error.code === 'E_IMPORT_EMPTY' ? 'empty'
        : error.code === 'E_IMPORT_ENCODING' ? 'encoding'
        : 'invalid-data'
      throw new MeshImportError(code, error.message, format)
    }
    throw new MeshImportError('invalid-data', error instanceof Error ? error.message : String(error), format)
  }
}

function blobFile(path: string, data: Uint8Array): OpenScadProjectBlobFile {
  return { kind: 'blob', path, data, byteLength: data.byteLength, sha256: sha256Hex(data) }
}

function fromOpenScad(geometry: OpenScadImportGeometry3D): RawMesh {
  return { positions: geometry.vertices, indices: geometry.triangles }
}

/** File decoding and welding run in mesh-io; errors retain the host API codes. */
function nativeImport(format: MeshImportFormat, bytes: Uint8Array): RawMesh {
 return nativeImportCall('mesh_decode', {format, bytes}, format)
}
function nativeImportCall<T>(op: string, payload: Record<string, unknown>, format: MeshImportFormat): T {
 try { return op==='mesh_decode' ? decodeMeshBytesInKernel(format as 'obj'|'ply'|'stl'|'off',payload.bytes as Uint8Array) as T : callGeometryRust<T>(op, payload) }
 catch (error) {
  const code: MeshImportErrorCode = error instanceof GeometryKernelError && error.code === 'MESH_IMPORT_EMPTY' ? 'empty'
   : error instanceof GeometryKernelError && error.code === 'MESH_IMPORT_LIMIT' ? 'limit'
   : error instanceof GeometryKernelError && error.code === 'MESH_IMPORT_ENCODING' ? 'encoding' : 'invalid-data'
  throw new MeshImportError(code, error instanceof Error ? error.message : String(error), format)
 }
}
function finalize(format: MeshImportFormat, positions: ArrayLike<number>, indices: ArrayLike<number>, weld: number | false): ImportedMesh {
 const result = nativeImportCall<{positions: number[]; indices: number[]; sourceVertexCount: number; degenerateTriangles: number}>(
  'mesh_import_finalize', {positions, indices, weld}, format)
 return Object.freeze({...result, format, positions: Float64Array.from(result.positions), indices: Uint32Array.from(result.indices),
  triangleCount: result.indices.length / 3, vertexCount: result.positions.length / 3})
}
