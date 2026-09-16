/**
 * Mesh file conversion: any importable mesh format → any exportable one.
 * Decoding is TypeScript (`meshImport`), serialization is the Rust kernel
 * (`meshExportFormats`). Geometry only: colours, textures, units metadata and
 * per-object identities are not carried across.
 */
import type { PolygonMesh } from './geometry/polygon'
import { inspectNurbsMesh, type NurbsMesh } from './geometry/tessellation'
import { MESH_EXPORT_FORMATS, exportMeshFormat, exportMeshFormatCompressed, type MeshExportFormat } from './meshExportFormats'
import {
  importMeshFile,
  MESH_IMPORT_FORMATS,
  stripMeshExtension,
  type MeshImportFormat,
  type MeshImportOptions,
} from './meshImport'

export { MESH_EXPORT_FORMATS, MESH_IMPORT_FORMATS }
export type { MeshExportFormat, MeshImportFormat }

/** Human-readable labels shared by the UI and MCP descriptions. */
export const MESH_FORMAT_LABELS: Readonly<Record<MeshExportFormat | MeshImportFormat, string>> = Object.freeze({
  stl: 'STL (ASCII)',
  stl_binary: 'STL (binary)',
  obj: 'OBJ',
  ply: 'PLY',
  off: 'OFF',
  amf: 'AMF',
  '3mf': '3MF',
})

/** Formats whose writers require closed, consistently oriented geometry. */
export const MESH_PRINTING_FORMATS: readonly MeshExportFormat[] = Object.freeze(['3mf', 'amf'])

export interface MeshConvertOptions extends MeshImportOptions {
  /** 3MF only: DEFLATE the OPC package. Default true. */
  readonly compressed?: boolean
}

export interface MeshConvertResult {
  readonly data: Uint8Array
  readonly mimeType: string
  readonly extension: string
  /** Suggested output file name derived from the input name. */
  readonly fileName: string
  readonly format: MeshExportFormat
  readonly source: {
    readonly fileName: string
    readonly format: MeshImportFormat
    readonly byteLength: number
    readonly triangleCount: number
    readonly vertexCount: number
    readonly sourceVertexCount: number
    readonly degenerateTriangles: number
  }
}

export function isMeshExportFormat(value: string): value is MeshExportFormat {
  return (MESH_EXPORT_FORMATS as readonly string[]).includes(value)
}

/** Build the export-ready mesh wrapper (Rust inspection report attached). */
export function polygonMeshToExportMesh(mesh: PolygonMesh): NurbsMesh {
  const positions = [...mesh.positions], indices = [...mesh.indices]
  const report = inspectNurbsMesh(positions, indices)
  return { positions, indices, report: { ...report, errorBoundCertified: false, selfIntersectionStatus: 'not_checked' } }
}

export function convertedFileName(inputName: string, extension: string): string {
  const base = stripMeshExtension(inputName).replace(/[/\\\0]/gu, '_').trim() || 'mesh'
  return `${base}.${extension}`
}

/** Decode `input` and re-serialize it as `target`. */
export async function convertMeshFile(
  fileName: string,
  input: Uint8Array | ArrayBuffer,
  target: MeshExportFormat,
  options: MeshConvertOptions = {},
): Promise<MeshConvertResult> {
  if (!isMeshExportFormat(target)) throw new Error(`Unsupported target format ${JSON.stringify(target)}.`)
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input)
  const { compressed = true, ...importOptions } = options
  const imported = await importMeshFile(fileName, bytes, importOptions)
  const exportMesh = polygonMeshToExportMesh(imported)
  const artifact = compressed
    ? await exportMeshFormatCompressed(exportMesh, target)
    : exportMeshFormat(exportMesh, target)
  return Object.freeze({
    data: artifact.data,
    mimeType: artifact.mimeType,
    extension: artifact.extension,
    fileName: convertedFileName(fileName, artifact.extension),
    format: target,
    source: Object.freeze({
      fileName,
      format: imported.format,
      byteLength: bytes.byteLength,
      triangleCount: imported.triangleCount,
      vertexCount: imported.vertexCount,
      sourceVertexCount: imported.sourceVertexCount,
      degenerateTriangles: imported.degenerateTriangles,
    }),
  })
}
