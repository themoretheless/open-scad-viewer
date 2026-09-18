import {callGeometryRust} from './geometry/kernel'
import { sampleCurve, worldPoints, dot3, type AnalyticCurve, type SketchPlane } from './directSketchGeometry'
import { extrudePolygonProfile, type PolygonMesh } from './geometry/polygon'
import { validateNurbsCurve } from './nurbsCurve'
import { validateNurbsSurface } from './nurbsSurface'
import type { SolidNurbsCurve, SolidNurbsSurface } from './solidNurbs'
import { inspectNurbsBrep, type NurbsBrep } from './geometry/brep'

export type Point2 = [number, number]
export interface DirectSketch { id: string; name: string; points: Point2[]; closed: boolean; analytic?: AnalyticCurve; plane?: SketchPlane }
/** `group` names a flat, optional grouping shown in the scene list. Bodies built from
 * source share one, so a rebuild can be recognised, replaced or deleted as a unit. */
export interface DirectBody { id: string; name: string; mesh: PolygonMesh; brep?: NurbsBrep; group?: string }
/** A group owns the source its bodies were built from, so it stays editable and rebuildable. */
export interface DirectGroup { name: string; source: string }
export interface DirectDocument { version: 1; sketches: DirectSketch[]; bodies: DirectBody[]; curves?: SolidNurbsCurve[]; surfaces?: SolidNurbsSurface[]; groups?: DirectGroup[] }
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
  // Absent rather than empty, so a document with no groups stays identical to one
  // written before groups existed.
  if (d.groups !== undefined && (!Array.isArray(d.groups) || d.groups.length > 64)) throw new Error('Invalid group collection.')
  const groupNames = new Set<string>()
  for (const g of d.groups ?? []) {
    if (!g || typeof g.name !== 'string' || g.name.length === 0 || g.name.length > 100 || groupNames.has(g.name)) throw new Error('Invalid group name.')
    if (typeof g.source !== 'string' || g.source.length > 100_000) throw new Error('Invalid group source.')
    groupNames.add(g.name)
  }
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
    if (b.group !== undefined && (typeof b.group !== 'string' || b.group.length === 0 || b.group.length > 100)) {
      throw new Error('Invalid body group.')
    }
    const m = b.mesh
    if (!m || !Array.isArray(m.positions) || !Array.isArray(m.indices) || m.positions.length < 9 || m.positions.length > 150_000 || m.positions.length % 3 || m.indices.length < 3 || m.indices.length > 150_000 || m.indices.length % 3 || !m.positions.every(finite) || !m.indices.every(i => Number.isInteger(i) && i >= 0 && i < m.positions.length / 3)) throw new Error('Invalid body mesh.')
    if (b.brep) {
      inspectNurbsBrep(b.brep)
      if ([b.brep.vertices,b.brep.edges,b.brep.loops,b.brep.faces,b.brep.shells,b.brep.bodies].every(items=>items.length===0)) throw new Error('An empty B-rep cannot be stored as a displayed body; remove the body entry.')
      if (b.brep.faces.length === 0) throw new Error('Displayed body B-rep must include at least one face.')
      const meshAabb = aabbFromPositions(m.positions)
      const brepAabb = aabbFromBrep(b.brep)
      const diag = Math.hypot(
        Math.max(meshAabb[1] - meshAabb[0], brepAabb[1] - brepAabb[0]),
        Math.max(meshAabb[3] - meshAabb[2], brepAabb[3] - brepAabb[2]),
        Math.max(meshAabb[5] - meshAabb[4], brepAabb[5] - brepAabb[4]),
      )
      const slack = Math.max(1e-3, 1e-4 * Math.max(diag, 1))
      for (let i = 0; i < 6; i++) {
        if (Math.abs(meshAabb[i] - brepAabb[i]) > slack) {
          throw new Error('Displayed body B-rep and mesh AABBs disagree; refuse stale mesh pairing.')
        }
      }
    }
  }
  for (const item of d.curves) validateNurbsCurve(item.curve)
  for (const item of d.surfaces) {
    if (!Number.isInteger(item.segmentsU) || item.segmentsU < 2 || item.segmentsU > 64 ||
        !Number.isInteger(item.segmentsV) || item.segmentsV < 2 || item.segmentsV > 64) throw new Error('Invalid NURBS display tessellation.')
    validateNurbsSurface(item.surface)
  }
  return clone(d)
}

const aabbFromPositions = (positions: number[]): [number, number, number, number, number, number] => {
  let minX = positions[0], maxX = positions[0], minY = positions[1], maxY = positions[1], minZ = positions[2], maxZ = positions[2]
  for (let i = 0; i < positions.length; i += 3) {
    const x = positions[i], y = positions[i + 1], z = positions[i + 2]
    if (x < minX) minX = x; if (x > maxX) maxX = x
    if (y < minY) minY = y; if (y > maxY) maxY = y
    if (z < minZ) minZ = z; if (z > maxZ) maxZ = z
  }
  return [minX, maxX, minY, maxY, minZ, maxZ]
}

const aabbFromBrep = (brep: NurbsBrep): [number, number, number, number, number, number] => {
  if (!brep.vertices.length) throw new Error('B-rep vertices are required for mesh correspondence.')
  let minX = brep.vertices[0].point[0], maxX = minX
  let minY = brep.vertices[0].point[1], maxY = minY
  let minZ = brep.vertices[0].point[2], maxZ = minZ
  for (const vertex of brep.vertices) {
    const [x, y, z] = vertex.point
    if (x < minX) minX = x; if (x > maxX) maxX = x
    if (y < minY) minY = y; if (y > maxY) maxY = y
    if (z < minZ) minZ = z; if (z > maxZ) maxZ = z
  }
  return [minX, maxX, minY, maxY, minZ, maxZ]
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
  return { id, name: sketch.name + ' · 3D', mesh: { positions: worldPoints(Array.from({length:built.positions.length/3},(_,i)=>built.positions.slice(i*3,i*3+3)),sketch.plane).flat(), indices: [...built.indices] } }
}

/** Author the sketch extrusion before deriving a display mesh. */
export const extrudeSketchBrep=(sketch:DirectSketch,height:number,baseZ=0):NurbsBrep=>callGeometryRust('brep_nurbs_sketch_extrude',{sketch,height,baseZ})

export function transformDirectPoints(points: number[][], delta: number[], angle: number, scale: number): number[][] {
  return callGeometryRust('cad_transform_points',{points,delta,angle,scale})
}

export { bodyPoints, directBodiesScad } from './directBodiesScad'
