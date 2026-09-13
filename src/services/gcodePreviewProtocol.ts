import type { MeshData } from '../core/mesh'
import type { GcodePreviewResult, ToolpathSettingsInput } from './geometry/polygon'

export const GCODE_PREVIEW_MAX_BYTES = 4 * 1024 * 1024
export const GCODE_PREVIEW_MAX_MESH_BYTES = 16 * 1024 * 1024
export type GcodeSceneMesh = Pick<MeshData, 'vertices' | 'indices' | 'transform'>
export type GcodePreviewJob =
  | { kind: 'slice'; mesh: GcodeSceneMesh; zMin: number; zMax: number; settings: ToolpathSettingsInput }
  | { kind: 'parse'; gcode: string }
export interface GcodePreviewDocument {
  gcode: string
  preview: GcodePreviewResult
  dialect: string
}
export interface GcodePreviewRequest { version: 1; id: number; job: GcodePreviewJob }
export type GcodePreviewResponse =
  | { version: 1; id: number; ok: true; result: GcodePreviewDocument }
  | { version: 1; id: number; ok: false; error: string }

export function checkGcodePreviewJob(job: GcodePreviewJob): GcodePreviewJob {
  if (!job || typeof job !== 'object' || (job.kind !== 'parse' && job.kind !== 'slice')) throw new Error('Invalid G-code preview request.')
  if (job.kind === 'parse') {
    if (typeof job.gcode !== 'string' || !job.gcode.trim()) throw new Error('Choose a nonempty G-code preview file.')
    if (job.gcode.length > GCODE_PREVIEW_MAX_BYTES || new TextEncoder().encode(job.gcode).byteLength > GCODE_PREVIEW_MAX_BYTES) {
      throw new Error('G-code preview files are limited to 4 MiB.')
    }
    return { kind: 'parse', gcode: job.gcode }
  }
  const { mesh, zMin, zMax } = job
  if (!mesh || !(mesh.vertices instanceof Float32Array) || !(mesh.indices instanceof Uint32Array)
    || !(mesh.transform instanceof Float32Array) || !mesh.vertices.length || mesh.vertices.length % 6
    || !mesh.indices.length || mesh.indices.length % 3 || mesh.transform.length !== 16) throw new Error('Select a valid triangle mesh.')
  if (!mesh.transform.every(Number.isFinite) || mesh.transform[12] !== 0 || mesh.transform[13] !== 0 || mesh.transform[14] !== 0 || mesh.transform[15] !== 1) throw new Error('Mesh transform must be finite and affine.')
  if (mesh.indices.length > 300000 || mesh.vertices.byteLength + mesh.indices.byteLength > GCODE_PREVIEW_MAX_MESH_BYTES) {
    throw new Error('G-code slicing is limited to 100000 triangles and 16 MiB of mesh data.')
  }
  if (!Number.isFinite(zMin) || !Number.isFinite(zMax) || zMin >= zMax) throw new Error('Z max must be greater than Z min; both must be finite.')
  if (!job.settings || typeof job.settings !== 'object' || Array.isArray(job.settings)) throw new Error('Invalid toolpath settings.')
  const settings: ToolpathSettingsInput = Object.fromEntries(Object.entries(job.settings).filter(([, value]) => value !== undefined))
  for (const [name, value] of Object.entries(settings)) {
    if (typeof value !== 'number' || !Number.isFinite(value) || value <= 0) throw new Error(`${name} must be a positive finite number.`)
  }
  if (settings.wallCount !== undefined && (!Number.isInteger(settings.wallCount) || settings.wallCount > 8)) throw new Error('Wall count must be an integer from 1 to 8.')
  // Structured cloning snapshots these views. Never transfer buffers owned by the live scene.
  return { kind: 'slice', mesh: { vertices: mesh.vertices, indices: mesh.indices, transform: mesh.transform }, zMin, zMax, settings }
}

/** Lightweight envelope checks; geometry validation runs inside the worker. */
export function isGcodePreviewDocument(value: unknown): value is GcodePreviewDocument {
  if (!value || typeof value !== 'object') return false
  const document = value as GcodePreviewDocument, preview = document.preview
  return typeof document.gcode === 'string' && typeof document.dialect === 'string' && !!preview
    && Number.isInteger(preview.layers) && preview.layers >= 0 && Array.isArray(preview.moves)
    && [preview.extrusionMm, preview.depositedVolumeMm3, preview.travelDistanceMm, preview.printDistanceMm, preview.estimatedTimeS].every(value => Number.isFinite(value) && value >= 0)
    && (preview.bounds === null || (!!preview.bounds && [preview.bounds.min, preview.bounds.max].every(point => Array.isArray(point) && point.length === 3 && point.every(Number.isFinite))))
}
