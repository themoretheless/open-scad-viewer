import {callGeometryRust} from './geometry/kernel'
import { MAX_DOCUMENT_CHARACTERS } from './directDocumentLimits'
export { MAX_DOCUMENT_CHARACTERS } from './directDocumentLimits'
import { sampleCurve, worldPoints, dot3, type AnalyticCurve, type SketchPlane } from './directSketchGeometry'
import { extrudePolygonProfile, type PolygonMesh } from './geometry/polygon'
import { validateNurbsCurve } from './nurbsCurve'
import { validateNurbsSurface } from './nurbsSurface'
import type { SolidNurbsCurve, SolidNurbsSurface } from './solidNurbs'
import type { NurbsBrep } from './geometry/brep'
import { BrepInspectionCache } from './brepInspectionCache'

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
const DEEP_JSON_NORMALIZATION = Symbol('deep JSON normalization')
/** Only for a freshly parsed, exclusively owned JSON tree, never caller-owned objects. */
function normalizeOwnedJson(value: unknown, depth = 0): unknown {
  if (depth > 64) throw DEEP_JSON_NORMALIZATION
  if (typeof value === 'number') return Number.isFinite(value) ? (value === 0 ? 0 : value) : null
  if (Array.isArray(value)) {
    for (let i = 0; i < value.length; i++) value[i] = normalizeOwnedJson(value[i], depth + 1)
  } else if (value !== null && typeof value === 'object') {
    const object = value as Record<string, unknown>
    for (const key of Object.keys(object)) object[key] = normalizeOwnedJson(object[key], depth + 1)
  }
  return value
}
const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v) && Math.abs(v) <= 1e6

const inspectedBreps = new BrepInspectionCache()

/** localStorage holds about 5 MB per origin; bigger documents are kept in memory only. */
export const MAX_DRAFT_CHARACTERS = 4_000_000

function* directDocumentValidation(text: string): Generator<void, DirectDocument> {
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
    yield
  }
  for (const b of d.bodies) {
    if (b.group !== undefined && (typeof b.group !== 'string' || b.group.length === 0 || b.group.length > 100)) {
      throw new Error('Invalid body group.')
    }
    const m = b.mesh
    if (!m || !Array.isArray(m.positions) || !Array.isArray(m.indices) || m.positions.length < 9 || m.positions.length > 150_000 || m.positions.length % 3 || m.indices.length < 3 || m.indices.length > 150_000 || m.indices.length % 3 || !m.positions.every(finite) || !m.indices.every(i => Number.isInteger(i) && i >= 0 && i < m.positions.length / 3)) throw new Error('Invalid body mesh.')
    if (b.brep) {
      inspectedBreps.inspect(b.brep)
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
    yield
  }
  for (const item of d.curves) { validateNurbsCurve(item.curve); yield }
  for (const item of d.surfaces) {
    if (!Number.isInteger(item.segmentsU) || item.segmentsU < 2 || item.segmentsU > 64 ||
        !Number.isInteger(item.segmentsV) || item.segmentsV < 2 || item.segmentsV > 64) throw new Error('Invalid NURBS display tessellation.')
    validateNurbsSurface(item.surface)
    yield
  }
  // JSON.parse and newly sampled sketch points already have exclusive ownership.
  // Preserve JSON number normalization without serializing/copying the whole tree.
  try { normalizeOwnedJson(d); return d }
  catch (error) {
    if (error !== DEEP_JSON_NORMALIZATION) throw error
    // This is a fast-path depth threshold, not a new document admission limit.
    return clone(d)
  }
}

export function parseDirectDocument(text: string): DirectDocument {
  const validation = directDocumentValidation(text)
  for (;;) {
    const step = validation.next()
    if (step.done) return step.value
  }
}

/** Same checks as the synchronous history/import route, yielding between objects.
 * A single B-rep inspection remains synchronous; no partially checked document escapes.
 */
export async function parseDirectDocumentAsync(text: string, options: {
  signal?: AbortSignal
  yieldControl?: () => Promise<void>
} = {}): Promise<DirectDocument> {
  const validation = directDocumentValidation(text)
  const yieldControl = options.yieldControl ?? (() => new Promise<void>(resolve => setTimeout(resolve, 0)))
  let sliceStarted = performance.now()
  try {
    for (;;) {
      if (options.signal?.aborted) throw new DOMException('Operation cancelled', 'AbortError')
      const step = validation.next()
      if (step.done) return step.value
      if (performance.now() - sliceStarted >= 8) {
        await yieldControl()
        sliceStarted = performance.now()
      }
    }
  } finally {
    validation.return(undefined as never)
  }
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
interface DirectSnapshot { document: DirectDocument; characters: number }
export class DirectHistory {
  private past: DirectSnapshot[] = []
  private future: DirectSnapshot[] = []
  private current: DirectSnapshot
  constructor(document = emptyDirectDocument()) {
    const text = JSON.stringify(document)
    const validated = parseDirectDocument(text)
    this.current = { document: validated, characters: JSON.stringify(validated).length }
  }
  get document() { return clone(this.current.document) }
  get canUndo() { return this.past.length > 0 }
  get canRedo() { return this.future.length > 0 }
  commit(document: DirectDocument) {
    const text = JSON.stringify(document)
    const next = parseDirectDocument(text)
    const nextText = JSON.stringify(next)
    if (nextText.length === this.current.characters && nextText === JSON.stringify(this.current.document)) return
    this.past.push(this.current)
    // The existing bound counts serialized characters, not actual heap bytes.
    // Size travels with its snapshot through undo/redo; neither needs to serialize it again.
    let retained = this.past.reduce((sum, snapshot) => sum + snapshot.characters, 0)
    while (this.past.length > 80 || (this.past.length > 1 && retained > 16_000_000)) {
      retained -= this.past.shift()!.characters
    }
    this.current = { document: next, characters: nextText.length }; this.future = []
  }
  undo() {
    const d = this.past.pop()
    if (d) { this.future.push(this.current); this.current = d }
    return this.document
  }
  redo() {
    const d = this.future.pop()
    if (d) { this.past.push(this.current); this.current = d }
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
