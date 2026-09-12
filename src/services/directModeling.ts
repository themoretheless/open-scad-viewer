import { sampleCurve, worldPoint, dot3, type AnalyticCurve, type SketchPlane } from './directSketchGeometry'
import { extrudePolygonProfile, type PolygonMesh } from './geometry/polygon'
import { validateNurbsCurve } from './nurbsCurve'
import { validateNurbsSurface } from './nurbsSurface'
import type { SolidNurbsCurve, SolidNurbsSurface } from './solidNurbs'

export type Point2 = [number, number]
export interface DirectSketch { id: string; name: string; points: Point2[]; closed: boolean; analytic?: AnalyticCurve; plane?: SketchPlane }
export interface DirectBody { id: string; name: string; mesh: PolygonMesh }
export interface DirectDocument { version: 1; sketches: DirectSketch[]; bodies: DirectBody[]; curves?: SolidNurbsCurve[]; surfaces?: SolidNurbsSurface[] }
export const emptyDirectDocument = (): DirectDocument => ({ version: 1, sketches: [], bodies: [], curves: [], surfaces: [] })
const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value))
const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v) && Math.abs(v) <= 1e6

export function parseDirectDocument(text: string): DirectDocument {
  if (text.length > 4_000_000) throw new Error('Document exceeds 4 MB.')
  const d = JSON.parse(text) as DirectDocument
  if (!d || d.version !== 1 || !Array.isArray(d.sketches) || !Array.isArray(d.bodies) || d.sketches.length + d.bodies.length > 200) throw new Error('Invalid direct modeling document.')
  d.curves ??= []
  d.surfaces ??= []
  if (!Array.isArray(d.curves) || !Array.isArray(d.surfaces) || d.curves.length + d.surfaces.length > 128) throw new Error('Invalid Solid NURBS collection.')
  const ids = new Set<string>()
  for (const item of [...d.sketches, ...d.bodies, ...d.curves, ...d.surfaces]) {
    if (typeof item.id !== 'string' || ids.has(item.id) || typeof item.name !== 'string' || item.name.length > 100) throw new Error('Invalid object identity.')
    ids.add(item.id)
  }
  for (const s of d.sketches) {
    if (s.analytic) { s.points = sampleCurve(s.analytic); s.closed = s.analytic.kind === 'circle' }
    if (s.plane) {
      const {origin,u,v}=s.plane
      if (![origin,u,v].every(p=>Array.isArray(p)&&p.length===3&&p.every(finite)) || Math.abs(dot3(u,u)-1)>1e-6 || Math.abs(dot3(v,v)-1)>1e-6 || Math.abs(dot3(u,v))>1e-6) throw new Error('Invalid sketch workplane.')
    }
    if (typeof s.closed !== 'boolean' || !Array.isArray(s.points) || s.points.length < 2 || s.points.length > 512 || (s.closed && s.points.length < 3) || !s.points.every(p => Array.isArray(p) && p.length === 2 && p.every(finite))) throw new Error('Invalid sketch.')
  }
  for (const b of d.bodies) {
    const m = b.mesh
    if (!m || !Array.isArray(m.positions) || !Array.isArray(m.indices) || m.positions.length < 9 || m.positions.length > 150_000 || m.positions.length % 3 || m.indices.length < 3 || m.indices.length > 150_000 || m.indices.length % 3 || !m.positions.every(finite) || !m.indices.every(i => Number.isInteger(i) && i >= 0 && i < m.positions.length / 3)) throw new Error('Invalid body mesh.')
  }
  for (const item of d.curves) validateNurbsCurve(item.curve)
  for (const item of d.surfaces) {
    if (!Number.isInteger(item.segmentsU) || item.segmentsU < 2 || item.segmentsU > 64 ||
        !Number.isInteger(item.segmentsV) || item.segmentsV < 2 || item.segmentsV > 64) throw new Error('Invalid NURBS display tessellation.')
    validateNurbsSurface(item.surface)
  }
  return clone(d)
}

/** Snapshots contain independent geometry, never sketch references or a feature tree. */
export class DirectHistory {
  private past: DirectDocument[] = []
  private future: DirectDocument[] = []
  private current: DirectDocument
  constructor(document = emptyDirectDocument()) { this.current = parseDirectDocument(JSON.stringify(document)) }
  get document() { return clone(this.current) }
  get canUndo() { return this.past.length > 0 }
  get canRedo() { return this.future.length > 0 }
  commit(document: DirectDocument) {
    const next = parseDirectDocument(JSON.stringify(document))
    if (JSON.stringify(next) === JSON.stringify(this.current)) return
    this.past.push(this.current)
    // Bound retained snapshots by both count and bytes.
    while (this.past.length > 80 || (this.past.length > 1 && JSON.stringify(this.past).length > 16_000_000)) this.past.shift()
    this.current = next; this.future = []
  }
  undo() { const d = this.past.pop(); if (d) { this.future.push(this.current); this.current = d } return this.document }
  redo() { const d = this.future.pop(); if (d) { this.past.push(this.current); this.current = d } return this.document }
}

export function extrudeDirectSketch(sketch: DirectSketch, height: number, id: string): DirectBody {
  if (!sketch.closed || sketch.points.length < 3) throw new Error('Close the contour before extrusion.')
  if (!finite(height) || height <= 0) throw new Error('Height must be positive.')
  const built = extrudePolygonProfile({ outer: sketch.points.map(p => [...p]) }, [0, 0, height])
  return { id, name: sketch.name + ' · 3D', mesh: { positions: Array.from({length:built.positions.length/3},(_,i)=>worldPoint(built.positions.slice(i*3,i*3+3),sketch.plane)).flat(), indices: [...built.indices] } }
}

export function transformDirectPoints(points: number[][], delta: number[], angle: number, scale: number): number[][] {
  if (!points.length || !delta.every(finite) || !finite(angle) || !finite(scale) || scale <= 0) throw new Error('Invalid transform.')
  const center = [0, 1, 2].map(axis => points.reduce((sum, p) => sum + (p[axis] ?? 0), 0) / points.length)
  const a = angle * Math.PI / 180, c = Math.cos(a), s = Math.sin(a)
  return points.map(p => {
    const x = (p[0] - center[0]) * scale, y = (p[1] - center[1]) * scale
    return [center[0] + c * x - s * y + delta[0], center[1] + s * x + c * y + delta[1], center[2] + ((p[2] ?? 0) - center[2]) * scale + (delta[2] ?? 0)].slice(0, p.length)
  })
}
export { bodyPoints, directBodiesScad } from './directBodiesScad'
