import { checkManifoldInKernel, type KernelManifoldReport } from './geometry/manifoldAnalysis'
import type { PolygonMesh } from './geometry/polygon'
import { inspectSolidIntersections, type SolidIntersectionOptions } from './solidDiagnostics'

export const PRINT_READINESS_ERROR_CODES = [
  'PRINT_NOT_WATERTIGHT',
  'PRINT_ORIENTATION',
  'PRINT_SELF_INTERSECTIONS',
  'PRINT_CHECK_INCOMPLETE',
  'PRINT_CHECK_FAILED',
] as const

export type PrintReadinessErrorCode = typeof PRINT_READINESS_ERROR_CODES[number]

export interface PrintReadinessErrorDetails {
  boundaryEdges?: number
  nonManifoldEdges?: number
  nonManifoldVertices?: number
  degenerateTriangles?: number
  isolatedVertices?: number
  orientationConflicts?: number
  contacts?: number
  triangleIds?: number[]
  stopReason?: 'work-limit' | 'contact-limit'
  work?: number
  maxWork?: number
  maxContacts?: number
  reason?: string
}

export function isPrintReadinessErrorCode(value: unknown): value is PrintReadinessErrorCode {
  return typeof value === 'string' && (PRINT_READINESS_ERROR_CODES as readonly string[]).includes(value)
}

function listed(values: Array<[string, string, number | undefined]>, locale: string): string {
  return values.filter(([, , count]) => (count ?? 0) > 0)
    .map(([en, ru, count]) => `${locale === 'ru' ? ru : en}: ${count}`).join(locale === 'ru' ? '；' : '; ')
}

/** Stable machine code + structured evidence; the worker envelope preserves both. */
export class PrintReadinessError extends Error {
  readonly name = 'PrintReadinessError'
  constructor(
    readonly code: PrintReadinessErrorCode,
    readonly details: PrintReadinessErrorDetails = {},
    message = formatPrintReadiness(code, details, 'en'),
  ) {
    super(message)
  }
}

export function formatPrintReadiness(code: PrintReadinessErrorCode, details: PrintReadinessErrorDetails, locale: string): string {
  const ru = locale === 'ru'
  if (code === 'PRINT_NOT_WATERTIGHT') {
    const defects = listed([
      ['boundary edges', 'граничные рёбра', details.boundaryEdges],
      ['non-manifold edges', 'немногообразные рёбра', details.nonManifoldEdges],
      ['pinched vertices', 'ущемлённые вершины', details.nonManifoldVertices],
      ['degenerate triangles', 'вырожденные треугольники', details.degenerateTriangles],
      ['isolated vertices', 'изолированные вершины', details.isolatedVertices],
    ], locale)
    return ru
      ? `Печать заблокирована: меш не является замкнутым многообразием${defects ? ` (${defects})` : ''}. Почините меш и повторите проверку.`
      : `3D printing blocked: the mesh is not a watertight manifold${defects ? ` (${defects})` : ''}. Repair the mesh and try again.`
  }
  if (code === 'PRINT_ORIENTATION') {
    return ru
      ? `Печать заблокирована: несогласованная ориентация треугольников (конфликтующие рёбра: ${details.orientationConflicts ?? 0}). Исправьте направление нормалей.`
      : `3D printing blocked: inconsistent triangle orientation (conflicting edges: ${details.orientationConflicts ?? 0}). Repair the winding and try again.`
  }
  if (code === 'PRINT_SELF_INTERSECTIONS') {
    const triangles = details.triangleIds?.length
      ? `${ru ? 'Треугольники' : 'Triangles'}: ${details.triangleIds.slice(0, 12).join(', ')}${details.triangleIds.length > 12 ? '…' : ''}`
      : ''
    return ru
      ? `Печать заблокирована: обнаружены самопересечения (контакты: ${details.contacts ?? 0}${triangles ? `; ${triangles}` : ''}). Устраните пересекающиеся участки меша.`
      : `3D printing blocked: self-intersections found (contacts: ${details.contacts ?? 0}${triangles ? `; ${triangles}` : ''}). Remove the intersecting mesh regions and try again.`
  }
  if (code === 'PRINT_CHECK_INCOMPLETE') {
    return ru
      ? `Печать заблокирована: проверка самопересечений не завершена (${details.stopReason ?? 'unknown'}; выполнено ${details.work ?? 0} из ${details.maxWork ?? 0}). Отсутствие пересечений не доказано. Упростите или почините меш.`
      : `3D printing blocked: the self-intersection check did not finish (${details.stopReason ?? 'unknown'}; work ${details.work ?? 0} of ${details.maxWork ?? 0}). Absence of intersections is unproven. Repair or simplify the mesh and try again.`
  }
  return ru
    ? `Печать заблокирована: проверка меша не выполнена${details.reason ? ` (${details.reason})` : ''}. Почините меш и повторите попытку.`
    : `3D printing blocked: the mesh check could not be completed${details.reason ? ` (${details.reason})` : ''}. Repair the mesh and try again.`
}

export function formatPrintReadinessError(reason: unknown, locale: string): string {
  if (reason instanceof PrintReadinessError) return formatPrintReadiness(reason.code, reason.details, locale)
  return reason instanceof Error ? reason.message : String(reason)
}

export interface PrintReadinessReport {
  manifold: KernelManifoldReport
  intersections: ReturnType<typeof inspectSolidIntersections>
}

/** Mandatory slicer gate. All geometric predicates and intersection work stay in Rust. */
export function assertPrintReady(mesh: PolygonMesh, options: SolidIntersectionOptions = {}): PrintReadinessReport {
  const manifold = checkManifoldInKernel(mesh.positions, mesh.indices)
  const orientationConflicts = manifold.orientationEdges.length / 2
  if (orientationConflicts) throw new PrintReadinessError('PRINT_ORIENTATION', { orientationConflicts })
  const details: PrintReadinessErrorDetails = {
    boundaryEdges: manifold.boundaryEdges.length / 2,
    nonManifoldEdges: manifold.nonManifoldEdges.length / 2,
    nonManifoldVertices: manifold.nonManifoldVertices.length,
    degenerateTriangles: manifold.degenerateTriangles.length,
    isolatedVertices: manifold.isolatedVertices.length,
  }
  if (!manifold.isManifold || manifold.triangleCount === 0 || details.isolatedVertices) {
    throw new PrintReadinessError('PRINT_NOT_WATERTIGHT', details)
  }
  let intersections: ReturnType<typeof inspectSolidIntersections>
  try {
    intersections = inspectSolidIntersections(mesh, options)
  } catch (reason) {
    throw new PrintReadinessError('PRINT_CHECK_FAILED', { reason: reason instanceof Error ? reason.message : String(reason) })
  }
  if (intersections.contacts.length) {
    throw new PrintReadinessError('PRINT_SELF_INTERSECTIONS', {
      contacts: intersections.contacts.length,
      triangleIds: intersections.triangleIds,
    })
  }
  if (!intersections.complete) {
    throw new PrintReadinessError('PRINT_CHECK_INCOMPLETE', {
      stopReason: intersections.stopReason ?? undefined,
      work: intersections.work,
      maxWork: intersections.maxWork,
      maxContacts: intersections.maxContacts,
    })
  }
  return { manifold, intersections }
}
