import type { MeshData } from '../core/mesh'
import { GCODE_FLAVORS, type GcodePreviewResult, type JobSettingsInput, type ToolpathSettingsInput } from './geometry/polygon'

export const GCODE_PREVIEW_MAX_BYTES = 4 * 1024 * 1024
export const GCODE_PREVIEW_MAX_MESH_BYTES = 16 * 1024 * 1024
export type GcodeSceneMesh = Pick<MeshData, 'vertices' | 'indices' | 'transform'>
export type GcodePreviewJob =
  | { kind: 'slice'; mesh: GcodeSceneMesh; zMin: number; zMax: number; settings: ToolpathSettingsInput }
  | { kind: 'job'; mesh: GcodeSceneMesh; zMin: number; zMax: number; settings: JobSettingsInput }
  | { kind: 'parse'; gcode: string }
export interface GcodePreviewDocument {
  gcode: string
  preview: GcodePreviewResult
  dialect: string
  /** Present for `kind: 'job'` exports (standard base64 OPC package). */
  gcode3mfBase64?: string
  /** False when an opened file came from another slicer and was read tolerantly. */
  native?: boolean
  generator?: string
  /** Firmware flavor: the job's target, or the flavor declared in an opened file. */
  flavor?: string | null
}
export interface GcodePreviewRequest { version: 1; id: number; job: GcodePreviewJob }
export type GcodePreviewResponse =
  | { version: 1; id: number; ok: true; result: GcodePreviewDocument }
  | { version: 1; id: number; ok: false; error: string }

function checkMesh(mesh: GcodeSceneMesh, zMin: number, zMax: number): GcodeSceneMesh {
  if (!mesh || !(mesh.vertices instanceof Float32Array) || !(mesh.indices instanceof Uint32Array)
    || !(mesh.transform instanceof Float32Array) || !mesh.vertices.length || mesh.vertices.length % 6
    || !mesh.indices.length || mesh.indices.length % 3 || mesh.transform.length !== 16) throw new Error('Select a valid triangle mesh.')
  if (!mesh.transform.every(Number.isFinite) || mesh.transform[12] !== 0 || mesh.transform[13] !== 0 || mesh.transform[14] !== 0 || mesh.transform[15] !== 1) throw new Error('Mesh transform must be finite and affine.')
  if (mesh.indices.length > 300000 || mesh.vertices.byteLength + mesh.indices.byteLength > GCODE_PREVIEW_MAX_MESH_BYTES) {
    throw new Error('G-code slicing is limited to 100000 triangles and 16 MiB of mesh data.')
  }
  if (!Number.isFinite(zMin) || !Number.isFinite(zMax) || zMin >= zMax) throw new Error('Z max must be greater than Z min; both must be finite.')
  return { vertices: mesh.vertices, indices: mesh.indices, transform: mesh.transform }
}

function checkToolpathSettings(settings: ToolpathSettingsInput): ToolpathSettingsInput {
  if (!settings || typeof settings !== 'object' || Array.isArray(settings)) throw new Error('Invalid toolpath settings.')
  const cleaned: ToolpathSettingsInput = Object.fromEntries(Object.entries(settings).filter(([, value]) => value !== undefined))
  for (const [name, value] of Object.entries(cleaned)) {
    if (typeof value !== 'number' || !Number.isFinite(value) || value <= 0) throw new Error(`${name} must be a positive finite number.`)
  }
  if (cleaned.wallCount !== undefined && (!Number.isInteger(cleaned.wallCount) || cleaned.wallCount > 8)) throw new Error('Wall count must be an integer from 1 to 8.')
  return cleaned
}

function checkJobSettings(settings: JobSettingsInput): JobSettingsInput {
  if (!settings || typeof settings !== 'object' || Array.isArray(settings)) throw new Error('Invalid job settings.')
  const cleaned: JobSettingsInput = Object.fromEntries(Object.entries(settings).filter(([, value]) => value !== undefined))
  for (const [name, value] of Object.entries(cleaned)) {
    if (name === 'homeAxes') {
      if (typeof value !== 'boolean') throw new Error('homeAxes must be a boolean.')
      continue
    }
    if (name === 'flavor') {
      if (typeof value !== 'string' || !(GCODE_FLAVORS as readonly string[]).includes(value)) throw new Error(`flavor must be one of ${GCODE_FLAVORS.join(', ')}.`)
      continue
    }
    if (typeof value !== 'number' || !Number.isFinite(value)) throw new Error(`${name} must be a finite number.`)
    if (name === 'fanSpeed') {
      if (value < 0 || value > 255) throw new Error('fanSpeed must be between 0 and 255.')
      continue
    }
    if (value <= 0) throw new Error(`${name} must be a positive finite number.`)
  }
  if (cleaned.wallCount !== undefined && (!Number.isInteger(cleaned.wallCount) || cleaned.wallCount > 8)) throw new Error('Wall count must be an integer from 1 to 8.')
  return cleaned
}

export function checkGcodePreviewJob(job: GcodePreviewJob): GcodePreviewJob {
  if (!job || typeof job !== 'object' || (job.kind !== 'parse' && job.kind !== 'slice' && job.kind !== 'job')) throw new Error('Invalid G-code preview request.')
  if (job.kind === 'parse') {
    if (typeof job.gcode !== 'string' || !job.gcode.trim()) throw new Error('Choose a nonempty G-code preview file.')
    if (job.gcode.length > GCODE_PREVIEW_MAX_BYTES || new TextEncoder().encode(job.gcode).byteLength > GCODE_PREVIEW_MAX_BYTES) {
      throw new Error('G-code preview files are limited to 4 MiB.')
    }
    return { kind: 'parse', gcode: job.gcode }
  }
  const mesh = checkMesh(job.mesh, job.zMin, job.zMax)
  // Structured cloning snapshots these views. Never transfer buffers owned by the live scene.
  if (job.kind === 'job') {
    return { kind: 'job', mesh, zMin: job.zMin, zMax: job.zMax, settings: checkJobSettings(job.settings) }
  }
  return { kind: 'slice', mesh, zMin: job.zMin, zMax: job.zMax, settings: checkToolpathSettings(job.settings) }
}

/** Lightweight envelope checks; geometry validation runs inside the worker. */
export function isGcodePreviewDocument(value: unknown): value is GcodePreviewDocument {
  if (!value || typeof value !== 'object') return false
  const document = value as GcodePreviewDocument, preview = document.preview
  if (document.gcode3mfBase64 !== undefined && typeof document.gcode3mfBase64 !== 'string') return false
  if (document.native !== undefined && typeof document.native !== 'boolean') return false
  if (document.generator !== undefined && typeof document.generator !== 'string') return false
  if (document.flavor !== undefined && document.flavor !== null && typeof document.flavor !== 'string') return false
  return typeof document.gcode === 'string' && typeof document.dialect === 'string' && !!preview
    && Number.isInteger(preview.layers) && preview.layers >= 0 && Array.isArray(preview.moves)
    && [preview.extrusionMm, preview.depositedVolumeMm3, preview.travelDistanceMm, preview.printDistanceMm, preview.estimatedTimeS].every(value => Number.isFinite(value) && value >= 0)
    && (preview.bounds === null || (!!preview.bounds && [preview.bounds.min, preview.bounds.max].every(point => Array.isArray(point) && point.length === 3 && point.every(Number.isFinite))))
}
