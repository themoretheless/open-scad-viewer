/**
 * Mesh file conversion: any importable mesh format → any exportable one.
 * Decoding is TypeScript (`meshImport`), serialization is the Rust kernel
 * (`meshExportFormats`). Geometry only: colours, textures, units metadata and
 * per-object identities are not carried across.
 */
import type { MeshData } from '../core/mesh'
import type { PolygonMesh } from './geometry/polygon'
import { inspectNurbsMesh, type NurbsMesh } from './geometry/tessellation'
import { flattenExportMeshes } from './meshExportAdapter'
import { exportMeshFormat, exportMeshFormatCompressed } from './meshExportFormats'
import {
  isMeshExportFormat,
  MESH_EXPORT_FORMATS,
  MESH_FORMAT_LABELS,
  MESH_IMPORT_FORMATS,
  MESH_PRINTING_FORMATS,
  convertedMeshFileName,
  type MeshExportFormat,
  type MeshImportFormat,
} from './meshFormats'
import { importMeshFile, type MeshImportOptions } from './meshImport'

export { isMeshExportFormat, MESH_EXPORT_FORMATS, MESH_FORMAT_LABELS, MESH_IMPORT_FORMATS, MESH_PRINTING_FORMATS }
export type { MeshExportFormat, MeshImportFormat }

export interface MeshExportOptions {
  /** 3MF only: DEFLATE the OPC package. Default true. */
  readonly compressed?: boolean
}
export interface MeshConvertOptions extends MeshImportOptions, MeshExportOptions {}
export type MeshExportArtifact = Awaited<ReturnType<typeof exportMeshFormatCompressed>>
export interface MeshExportProgress {
  readonly phase: 'inspect' | 'export'
  readonly completed: number
  readonly total: 2
}

export interface MeshConvertProgress {
  readonly phase: 'decode' | 'finalize' | 'check' | 'repair' | 'verify' | 'inspect' | 'export'
  readonly completed: number
  readonly total: 7
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

/** Build the export-ready mesh wrapper (Rust inspection report attached). */
export function polygonMeshToExportMesh(mesh: PolygonMesh): NurbsMesh {
  const positions = mesh.positions.slice(), indices = mesh.indices.slice()
  const report = inspectNurbsMesh(positions, indices)
  return { positions, indices, report: { ...report, errorBoundCertified: false, selfIntersectionStatus: 'not_checked' } }
}

export function convertedFileName(inputName: string, extension: string): string {
  return convertedMeshFileName(inputName, extension)
}

export type MeshExportInput = PolygonMesh | readonly MeshData[]

/** Inspect and serialize one body or a complete scene without blocking the UI. */
export async function exportMesh(
  mesh: MeshExportInput,
  target: MeshExportFormat,
  options: MeshExportOptions = {},
  onProgress?: (progress: MeshExportProgress) => void,
): Promise<MeshExportArtifact> {
  if (!isMeshExportFormat(target)) throw new Error(`Unsupported target format ${JSON.stringify(target)}.`)
  onProgress?.({ phase: 'inspect', completed: 0, total: 2 })
  const prepared = Array.isArray(mesh)
    ? flattenExportMeshes(mesh as readonly MeshData[])
    : polygonMeshToExportMesh(mesh as PolygonMesh)
  onProgress?.({ phase: 'export', completed: 1, total: 2 })
  return options.compressed ?? true
    ? exportMeshFormatCompressed(prepared, target)
    : exportMeshFormat(prepared, target)
}

export function exportPolygonMesh(
  mesh: PolygonMesh,
  target: MeshExportFormat,
  options: MeshExportOptions = {},
  onProgress?: (progress: MeshExportProgress) => void,
): Promise<MeshExportArtifact> {
  return exportMesh(mesh, target, options, onProgress)
}

/** Decode `input` and re-serialize it as `target`. */
export async function convertMeshFile(
  fileName: string,
  input: Uint8Array | ArrayBuffer,
  target: MeshExportFormat,
  options: MeshConvertOptions = {},
  onProgress?: (progress: MeshConvertProgress) => void,
): Promise<MeshConvertResult> {
  if (!isMeshExportFormat(target)) throw new Error(`Unsupported target format ${JSON.stringify(target)}.`)
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input)
  const { compressed = true, ...importOptions } = options
  const imported = await importMeshFile(fileName, bytes, importOptions, progress => {
    onProgress?.({ ...progress, total: 7 })
  })
  const artifact = await exportPolygonMesh(imported, target, { compressed }, progress => {
    onProgress?.({ phase: progress.phase, completed: progress.completed + 5, total: 7 })
  })
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
