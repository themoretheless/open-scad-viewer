import type { DirectDocument, DirectSketch, Point2 } from './directModeling'
import { buildOwnNurbs } from './modelGraphNurbsKernel'
import type { NurbsCurve } from './nurbsCurve'
import { evaluateNurbsCurve, validateNurbsCurve } from './nurbsCurve'
import type { NurbsSurface } from './nurbsSurface'
import { validateNurbsSurface } from './nurbsSurface'
import { tessellateNurbsSurface } from './geometry/tessellation'
import type { PolygonMesh } from './geometry/polygon'

export interface SolidNurbsCurve {
  id: string
  name: string
  curve: NurbsCurve
}

export interface SolidNurbsSurface {
  id: string
  name: string
  surface: NurbsSurface
  segmentsU: number
  segmentsV: number
}

export type SolidDocumentWithNurbs = DirectDocument & {
  curves: SolidNurbsCurve[]
  surfaces: SolidNurbsSurface[]
}

export function ensureSolidNurbs(document: DirectDocument): SolidDocumentWithNurbs {
  const result = document as SolidDocumentWithNurbs
  result.curves ??= []
  result.surfaces ??= []
  return result
}

export function validateSolidNurbs(document: DirectDocument): void {
  const d = ensureSolidNurbs(document)
  if (d.curves.length + d.surfaces.length > 128) throw new Error('Solid NURBS object limit exceeded.')
  for (const item of d.curves) {
    if (!item || typeof item.id !== 'string' || typeof item.name !== 'string') throw new Error('Invalid NURBS curve identity.')
    validateNurbsCurve(item.curve)
  }
  for (const item of d.surfaces) {
    if (!item || typeof item.id !== 'string' || typeof item.name !== 'string') throw new Error('Invalid NURBS surface identity.')
    if (!Number.isInteger(item.segmentsU) || item.segmentsU < 2 || item.segmentsU > 64 ||
        !Number.isInteger(item.segmentsV) || item.segmentsV < 2 || item.segmentsV > 64) throw new Error('Invalid NURBS display tessellation.')
    validateNurbsSurface(item.surface)
  }
}

export function sampleSolidNurbsCurve(curve: NurbsCurve, segments = 48): number[][] {
  if (!Number.isInteger(segments) || segments < 2 || segments > 256) throw new Error('Invalid curve sampling.')
  const start = curve.knots[curve.degree]
  const end = curve.knots[curve.controlPoints.length]
  return Array.from({ length: segments + 1 }, (_, index) =>
    evaluateNurbsCurve(curve, start + (end - start) * index / segments).point,
  )
}

export function tessellateSolidNurbsSurface(item: SolidNurbsSurface): PolygonMesh {
  const mesh = tessellateNurbsSurface(item.surface, { segmentsU: item.segmentsU, segmentsV: item.segmentsV })
  return { positions: [...mesh.positions], indices: [...mesh.indices] }
}

export function nurbsCurveToSketch(item: SolidNurbsCurve): DirectSketch {
  if (item.curve.controlPoints.some(point => point.length < 2 || Math.abs(point[2] ?? 0) > 1e-8)) {
    throw new Error('Only planar XY NURBS curves can be copied to a sketch.')
  }
  const points = sampleSolidNurbsCurve(item.curve).map(point => [point[0], point[1]] as Point2)
  const closed = Math.hypot(points[0][0] - points.at(-1)![0], points[0][1] - points.at(-1)![1]) < 1e-7
  if (closed) points.pop()
  return { id: crypto.randomUUID(), name: `${item.name} · sampled sketch`, points, closed }
}

export function importModelGraphNurbs(document: unknown): Pick<SolidDocumentWithNurbs, 'curves' | 'surfaces'> {
  const built = buildOwnNurbs(document, { action: 'build', display: { segments: 16, subdivisionLevels: 1 } })
  const definitions = built.report.definitions as Record<string, ({ kind: string } & Record<string, unknown>)>
  const curves: SolidNurbsCurve[] = []
  const surfaces: SolidNurbsSurface[] = []
  for (const [nodeId, definition] of Object.entries(definitions)) {
    const { kind, ...data } = definition
    if (kind === 'curve') {
      const curve = data as unknown as NurbsCurve
      validateNurbsCurve(curve)
      curves.push({ id: crypto.randomUUID(), name: nodeId, curve })
    } else if (kind === 'surface') {
      const surface = data as unknown as NurbsSurface
      validateNurbsSurface(surface)
      surfaces.push({ id: crypto.randomUUID(), name: nodeId, surface, segmentsU: 16, segmentsV: 16 })
    }
  }
  if (!curves.length && !surfaces.length) throw new Error('ModelGraph has no reachable NURBS curves or surfaces.')
  return { curves, surfaces }
}

export function createSolidNurbsCurve(id = crypto.randomUUID()): SolidNurbsCurve {
  return {
    id,
    name: 'NURBS curve',
    curve: {
      degree: 3,
      knots: [0, 0, 0, 0, 1, 1, 1, 1],
      controlPoints: [[-20, 0, 0], [-8, 18, 0], [8, -18, 0], [20, 0, 0]],
      weights: [1, 1, 1, 1],
    },
  }
}

export function createSolidNurbsSurface(id = crypto.randomUUID()): SolidNurbsSurface {
  return {
    id,
    name: 'NURBS surface',
    surface: {
      degreeU: 2,
      degreeV: 2,
      knotsU: [0, 0, 0, 1, 1, 1],
      knotsV: [0, 0, 0, 1, 1, 1],
      controlPoints: Array.from({ length: 3 }, (_, u) =>
        Array.from({ length: 3 }, (_, v) => [(u - 1) * 20, (v - 1) * 20, u === 1 && v === 1 ? 10 : 0]),
      ),
      weights: Array.from({ length: 3 }, () => [1, 1, 1]),
    },
    segmentsU: 16,
    segmentsV: 16,
  }
}

export function updateSolidNurbsControlPoint(
  document: DirectDocument,
  id: string,
  u: number,
  v: number,
  point: number[],
  weight?: number,
): DirectDocument {
  if (!Number.isInteger(u) || u < 0 || !Number.isInteger(v) || v < 0 ||
      point.length !== 3 || !point.every(value => Number.isFinite(value) && Math.abs(value) <= 1e6) ||
      (weight !== undefined && (!Number.isFinite(weight) || weight <= 0 || weight > 1e6))) {
    throw new Error('Invalid NURBS control point edit.')
  }
  const next = JSON.parse(JSON.stringify(document)) as DirectDocument
  const curve = next.curves?.find(item => item.id === id)
  const surface = next.surfaces?.find(item => item.id === id)
  if (curve) {
    if (!curve.curve.controlPoints[u]) throw new Error('NURBS curve CV is out of range.')
    curve.curve.controlPoints[u] = [...point]
    if (weight !== undefined) curve.curve.weights[u] = weight
  } else if (surface) {
    if (!surface.surface.controlPoints[u]?.[v]) throw new Error('NURBS surface CV is out of range.')
    surface.surface.controlPoints[u][v] = [...point]
    if (weight !== undefined) surface.surface.weights[u][v] = weight
  } else {
    throw new Error('NURBS object was not found.')
  }
  validateSolidNurbs(next)
  return next
}
