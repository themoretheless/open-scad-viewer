import type { MeshData } from '../core/mesh'
import type { SvgOptions } from './svgDocument'
import { SVG_MAX_BYTES } from './svgLimits'
import { MAX_WORKSPACE_SOURCE_LENGTH } from './workspaceDocument'

export type SvgPreviewResult = { svg: string; widthMm: number; heightMm: number; warnings: string[] }
export type SvgGeometryResult = SvgPreviewResult & { source?: string }
export type SvgProjectionMesh = Pick<MeshData, 'vertices' | 'indices' | 'faceIds' | 'transform'>
export type SvgJob =
  | { kind: 'preview' | 'contours'; svg: string; options: SvgOptions }
  | { kind: 'extrude'; svg: string; options: SvgOptions; height: number }
  | { kind: 'project'; meshes: readonly SvgProjectionMesh[]; axis: 'x' | 'y' | 'z'; face?: { meshIndex: number; triangleIndex: number }; options: SvgOptions }
export type SvgWorkerRequest = { version: 1; id: number; job: SvgJob }
export type SvgWorkerResponse =
  | { version: 1; id: number; ok: true; result: SvgGeometryResult }
  | { version: 1; id: number; ok: false; error: { name: string; message: string; code?: string } }

export function svgWorkerResult(value: unknown): value is SvgGeometryResult {
  if (!value || typeof value !== 'object') return false
  const v = value as Partial<SvgGeometryResult>
  return typeof v.svg === 'string' && v.svg.length <= SVG_MAX_BYTES
    && typeof v.widthMm === 'number' && Number.isFinite(v.widthMm) && v.widthMm > 0
    && typeof v.heightMm === 'number' && Number.isFinite(v.heightMm) && v.heightMm > 0
    && Array.isArray(v.warnings) && v.warnings.length <= 1000 && v.warnings.every(w => typeof w === 'string' && w.length <= 10000)
    && (v.source === undefined || typeof v.source === 'string' && v.source.length <= MAX_WORKSPACE_SOURCE_LENGTH)
}
