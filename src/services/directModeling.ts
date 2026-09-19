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
export interface DirectInterchangeMetadata {
  step?:{route:string;retained:boolean;refusalBoundary:string[];identityLoss:string[];metadataLoss:string[];
    definitionIdentities:string[];occurrenceIdentities:string[];productHierarchy:string[];externalReferences:string[]}
}
export interface DirectDocument { version: 1; sketches: DirectSketch[]; bodies: DirectBody[]; curves?: SolidNurbsCurve[]; surfaces?: SolidNurbsSurface[]; groups?: DirectGroup[]; interchange?: DirectInterchangeMetadata }
export const emptyDirectDocument = (): DirectDocument => ({ version: 1, sketches: [], bodies: [], curves: [], surfaces: [] })
const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value))
const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v) && Math.abs(v) <= 1e6

/**
 * B-rep inspection memoized by content.
 *
 * Every commit re-validates the whole document, and inspecting the topology of each
 * B-rep body took ~11 ms of a ~12 ms validation on a six-body scene, felt as a hitch
 * after each drag, undo or redo. Bodies are cloned on every commit, so identity cannot
 * serve as the key; the serialized topology can, and hashing it costs well under 1 ms.
 */
const inspectedBreps = new Map<string, true>()
const INSPECTED_BREP_LIMIT = 128
function inspectBrepOnce(brep: NurbsBrep): void {
  const key = JSON.stringify(brep)
  if (inspectedBreps.has(key)) return
  inspectNurbsBrep(brep)
  if (inspectedBreps.size >= INSPECTED_BREP_LIMIT) inspectedBreps.delete(inspectedBreps.keys().next().value!)
  inspectedBreps.set(key, true)
}

/**
 * A Solid document lives in memory as JSON. Twenty exact herringbone gears serialize
 * to about 20 MB (each helical wall face is a cubic loft with hundreds of control
 * points), so the bound is the one the other in-memory artifacts use, not the size
 * of a browser draft.
 */
export const MAX_DOCUMENT_CHARACTERS = 64 * 1024 * 1024
/** localStorage holds about 5 MB per origin; bigger documents are kept in memory only. */
export const MAX_DRAFT_CHARACTERS = 4_000_000

export function parseDirectDocument(text: string): DirectDocument {
  // In-memory bound only; the browser draft has its own, smaller quota (see persist in DirectModeler).
  if (text.length > MAX_DOCUMENT_CHARACTERS) throw new Error('Document exceeds 64 MB.')
  const d = JSON.parse(text) as DirectDocument
  if (!d || d.version !== 1 || !Array.isArray(d.sketches) || !Array.isArray(d.bodies) || d.sketches.length + d.bodies.length > 200) throw new Error('Invalid direct modeling document.')
  d.curves ??= []
  d.surfaces ??= []
  if(d.interchange){
    const step=d.interchange.step
    if(step&&(typeof step.route!=='string'||typeof step.retained!=='boolean'
      ||![step.refusalBoundary,step.identityLoss,step.metadataLoss,step.definitionIdentities,step.occurrenceIdentities,step.productHierarchy,step.externalReferences]
        .every(values=>Array.isArray(values)&&values.length<=1024&&values.every(value=>typeof value==='string'&&value.length<=4096)))){
      throw Error('Invalid interchange metadata.')
    }
  }
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
      inspectBrepOnce(b.brep)
      if ([b.brep.vertices,b.brep.edges,b.brep.loops,b.brep.faces,b.brep.shells,b.brep.bodies].every(items=>items.length===0)) throw new Error('An empty B-rep cannot be stored as a displayed body; remove the body entry.')
      if (b.brep.faces.length === 0) throw new Error('Displayed body B-rep must include at least one face.')
      const meshAabb = aabbFromPositions(m.positions)
      const { inner, outer } = aabbFromBrep(b.brep)
      const diag = Math.hypot(outer[1] - outer[0], outer[3] - outer[2], outer[5] - outer[4])
      const slack = Math.max(1e-3, 1e-4 * Math.max(diag, 1))
      // The mesh samples the B-rep, so it can never leave the control hull and
      // must reach every vertex; curved edges and trimmed faces may legitimately
      // extend past the vertices (a tilted section circle has no vertex at its
      // extreme), so the vertex box is only a lower bound.
      for (let axis = 0; axis < 3; axis++) {
        const lo = 2 * axis, hi = 2 * axis + 1
        if (meshAabb[lo] < outer[lo] - slack || meshAabb[hi] > outer[hi] + slack ||
            meshAabb[lo] > inner[lo] + slack || meshAabb[hi] < inner[hi] - slack) {
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

type Aabb = [number, number, number, number, number, number]
const growAabb = (box: Aabb, p: readonly number[]) => {
  for (let axis = 0; axis < 3; axis++) {
    if (p[axis] < box[2 * axis]) box[2 * axis] = p[axis]
    if (p[axis] > box[2 * axis + 1]) box[2 * axis + 1] = p[axis]
  }
}
/** `inner`: the vertex box (the mesh must reach it); `outer`: the control hull of every
 *  surface and edge curve (positive rational weights keep the geometry inside it). */
const aabbFromBrep = (brep: NurbsBrep): { inner: Aabb; outer: Aabb } => {
  if (!brep.vertices.length) throw new Error('B-rep vertices are required for mesh correspondence.')
  const inner: Aabb = [Infinity, -Infinity, Infinity, -Infinity, Infinity, -Infinity]
  for (const vertex of brep.vertices) growAabb(inner, vertex.point)
  const outer: Aabb = [...inner]
  for (const edge of brep.edges) for (const p of edge.curve.controlPoints) growAabb(outer, p)
  for (const face of brep.faces) for (const row of face.surface.controlPoints) for (const p of row) growAabb(outer, p)
  return { inner, outer }
}

/** Snapshots contain independent geometry, never sketch references or a feature tree. */
export class DirectHistory {
  private past: DirectDocument[] = []
  private future: DirectDocument[] = []
  private current: DirectDocument
  /** Serialized size of `current`, kept so the byte bound never re-serializes the stack. */
  private currentBytes: number
  private pastBytes: number[] = []
  constructor(document = emptyDirectDocument()) {
    const text = JSON.stringify(document)
    this.current = parseDirectDocument(text)
    this.currentBytes = JSON.stringify(this.current).length
  }
  get document() { return clone(this.current) }
  get canUndo() { return this.past.length > 0 }
  get canRedo() { return this.future.length > 0 }
  commit(document: DirectDocument) {
    const text = JSON.stringify(document)
    const next = parseDirectDocument(text)
    const nextText = JSON.stringify(next)
    if (nextText.length === this.currentBytes && nextText === JSON.stringify(this.current)) return
    this.past.push(this.current)
    this.pastBytes.push(this.currentBytes)
    // Bound retained snapshots by both count and bytes. Sizes are tracked as snapshots are
    // pushed: measuring by serializing the whole stack made every commit cost O(history),
    // which is heavy once bodies carry B-rep topology.
    let retained = this.pastBytes.reduce((sum, bytes) => sum + bytes, 0)
    while (this.past.length > 80 || (this.past.length > 1 && retained > 16_000_000)) {
      this.past.shift()
      retained -= this.pastBytes.shift() ?? 0
    }
    this.current = next; this.currentBytes = nextText.length; this.future = []
  }
  undo() {
    const d = this.past.pop()
    if (d) { this.pastBytes.pop(); this.future.push(this.current); this.current = d; this.currentBytes = JSON.stringify(d).length }
    return this.document
  }
  redo() {
    const d = this.future.pop()
    if (d) { this.past.push(this.current); this.pastBytes.push(this.currentBytes); this.current = d; this.currentBytes = JSON.stringify(d).length }
    return this.document
  }
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
