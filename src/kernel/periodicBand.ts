import { canonicalJson } from '../core/canonicalJson'
import { callGeometryRust } from './geometry/kernel'
import type { NurbsSurface } from './nurbsSurface'
import type { NurbsBrep } from './geometry/brep/core'

export interface PeriodicBandRequest {
  side: NurbsSurface
  planes: [NurbsSurface, NurbsSurface]
  toleranceMm: number
}

export interface PeriodicBandReport {
  source: PeriodicBandRequest & { op: 'brep_qualify_periodic_band' }
  bodies: 1
  faces: 3
  edges: number
  boundaryEdges: 0
  volumeBoundsMm3: [number, number]
}

const op = 'brep_qualify_periodic_band' as const

export function periodicBandSourceKey({ side, planes, toleranceMm }: PeriodicBandRequest): string {
  return canonicalJson({ op, side, planes, toleranceMm })
}

function validBandSummary(value: unknown, sourceKey: string): boolean {
  const report = value as Partial<PeriodicBandReport> | null
  const bounds = report?.volumeBoundsMm3
  if (report?.bodies !== 1 || report.faces !== 3 || report.boundaryEdges !== 0
    || typeof report.edges !== 'number' || !Number.isSafeInteger(report.edges) || report.edges < 2
    || !Array.isArray(bounds) || bounds.length !== 2
    || !Number.isFinite(bounds[0]) || !Number.isFinite(bounds[1])
    || bounds[0] <= 0 || bounds[1] < bounds[0]) return false
  try {
    return canonicalJson(report.source) === sourceKey
  } catch {
    return false
  }
}

export function validPeriodicBandReport(value: unknown, sourceKey: string): value is PeriodicBandReport {
  return validBandSummary(value, sourceKey)
}

export function qualifyPeriodicBand(request: PeriodicBandRequest): PeriodicBandReport {
  const { side, planes, toleranceMm } = request
  return callGeometryRust(op, { side, planes, toleranceMm })
}

export interface PeriodicStockIntersectionRequest {
  side: NurbsBrep
  stock: NurbsBrep
  toleranceUv: number
}

export interface PeriodicStockIntersectionReport extends Omit<PeriodicBandReport, 'source'> {
  source: PeriodicStockIntersectionRequest & { op: 'brep_intersect_periodic_stock_band' }
}

const stockIntersectionOp = 'brep_intersect_periodic_stock_band' as const

export function periodicStockIntersectionSourceKey({
  side, stock, toleranceUv,
}: PeriodicStockIntersectionRequest): string {
  return canonicalJson({ op: stockIntersectionOp, side, stock, toleranceUv })
}

export function validPeriodicStockIntersectionReport(
  value: unknown, sourceKey: string,
): value is PeriodicStockIntersectionReport {
  return validBandSummary(value, sourceKey)
}

export function intersectPeriodicStockBand({
  side, stock, toleranceUv,
}: PeriodicStockIntersectionRequest): PeriodicStockIntersectionReport {
  return callGeometryRust(stockIntersectionOp, { side, stock, toleranceUv })
}
