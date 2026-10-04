import {validCurveOffsetRegion} from './curveOffsetRegion'
import {validCurveOffsetConstruction} from './curveOffsetConstruction'
import {extrudeSketchProfile} from './directExtrusion'
import {withRetainedProfile} from './retainedSketchProfile'
import {validateBrepProfile,type BrepProfile} from './geometry/brepProfile'
import type {SolidInstanceBatchCache} from './solidInstanceBatchCache'
import {resolveSolidInstances,type SolidInstanceLink} from './solidInstances'
import { sketchDimensions, type SketchDimension } from './directDimensions'
import {callGeometryRust} from './geometry/kernel'
import { MAX_DOCUMENT_CHARACTERS, MAX_BODY_MESH_COMPONENTS } from './directDocumentLimits'
export { MAX_DOCUMENT_CHARACTERS } from './directDocumentLimits'
import { sampleCurve, worldPoints, dot3, type AnalyticCurve, type SketchPlane } from './directSketchGeometry'
import { extrudePolygonProfile, normalizePolygonMesh, type PolygonMesh } from './geometry/polygon'
import { stringifyMeshJson } from './meshJson'
import { validateNurbsCurve } from './nurbsCurve'
import { validateNurbsSurface } from './nurbsSurface'
import type { SolidNurbsCurve, SolidNurbsSurface } from './solidNurbs'
import type { NurbsBrep } from './geometry/brep'
import { BrepInspectionCache } from './brepInspectionCache'

export type Point2 = [number, number]
export interface DirectSketch { group?: string; id: string; name: string; points: Point2[]; closed: boolean; retainedProfile?: BrepProfile; analytic?: AnalyticCurve; plane?: SketchPlane; supportBodyId?: string; dimensions?: SketchDimension[] }
/** `group` names a flat, optional grouping shown in the scene list. Bodies built from
 * source share one, so a rebuild can be recognised, replaced or deleted as a unit. */
export interface DirectMaterial { name: string; color: string; metallic?: number; roughness?: number; opacity?: number }
export interface DirectBody { id: string; name: string; mesh: PolygonMesh; brep?: NurbsBrep; group?: string; material?: DirectMaterial; instance?: SolidInstanceLink }
/** A group owns the source its bodies were built from, so it stays editable and rebuildable. */
export interface DirectGroup { name: string; source: string }
export interface DirectInterchangeMetadata {
  step?:{route:string;retained:boolean;refusalBoundary:string[];identityLoss:string[];metadataLoss:string[];
    definitionIdentities:string[];occurrenceIdentities:string[];productHierarchy:string[];externalReferences:string[]}
}
export interface DirectDocument { version: 1; blenderProjectId?: string; sketches: DirectSketch[]; bodies: DirectBody[]; curves?: SolidNurbsCurve[]; surfaces?: SolidNurbsSurface[]; groups?: DirectGroup[]; interchange?: DirectInterchangeMetadata }
export const emptyDirectDocument = (): DirectDocument => ({ version: 1, sketches: [], bodies: [], curves: [], surfaces: [] })
const clone = <T>(value: T): T => structuredClone(value)
const DEEP_JSON_NORMALIZATION = Symbol('deep JSON normalization')
/** Only for a freshly parsed, exclusively owned JSON tree, never caller-owned objects. */
function normalizeOwnedJson(value: unknown, depth = 0): unknown {
  if (depth > 64) throw DEEP_JSON_NORMALIZATION
  if (typeof value === 'number') return Number.isFinite(value) ? (value === 0 ? 0 : value) : null
  if (ArrayBuffer.isView(value)) return value
  if (Array.isArray(value)) {
    for (let i = 0; i < value.length; i++) value[i] = normalizeOwnedJson(value[i], depth + 1)
  } else if (value !== null && typeof value === 'object') {
    const object = value as Record<string, unknown>
    for (const key of Object.keys(object)) object[key] = normalizeOwnedJson(object[key], depth + 1)
  }
  return value
}
const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v) && Math.abs(v) <= 1e6

// Lazy construction avoids initialization-order coupling between shared CAD chunks.
let inspectedBreps: BrepInspectionCache | undefined

/** localStorage holds about 5 MB per origin; bigger documents are kept in memory only. */
export const MAX_DRAFT_CHARACTERS = 4_000_000
export const MAX_DIRECT_INSTANCES = 1000

/** Persist each independent shape once; linked bodies retain identity and placement. */
export function serializeDirectDocument(document:DirectDocument):string {
 return stringifyMeshJson({...document,bodies:document.bodies.map(body=>{
  if(!body.instance)return body
  const {mesh:_mesh,brep:_brep,...linked}=body
  return linked
 })})
}

function* directDocumentValidation(text: string, instanceCache?:SolidInstanceBatchCache): Generator<void, DirectDocument> {
  // In-memory bound only; the browser draft has its own, smaller quota (see persist in DirectModeler).
  if (text.length > MAX_DOCUMENT_CHARACTERS) throw new Error('Document exceeds 64 MB.')
  const d = JSON.parse(text) as DirectDocument
  if (!d || d.version !== 1 || !Array.isArray(d.sketches) || !Array.isArray(d.bodies) || d.sketches.length + d.bodies.length > 200+MAX_DIRECT_INSTANCES) throw new Error('Invalid direct modeling document.')
  if(d.blenderProjectId!==undefined&&(typeof d.blenderProjectId!=='string'||!d.blenderProjectId.trim()||d.blenderProjectId.length>100))throw Error('Invalid Blender project identity.')
  const instanceCount=d.bodies.filter(body=>body?.instance).length
  if(instanceCount>MAX_DIRECT_INSTANCES||d.sketches.length+d.bodies.length-instanceCount>200)throw Error('Invalid direct modeling document: supports at most 200 independent bodies/sketches and 1000 linked instances.')
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
    if (item.group !== undefined && (typeof item.group !== 'string' || !item.group.length || item.group.length > 100)) throw new Error('Invalid object group.')
    ids.add(item.id)
  }
  for (const s of d.sketches) {
    if(s.supportBodyId!==undefined&&(typeof s.supportBodyId!=='string'||s.supportBodyId.length>200))throw new Error('Invalid sketch support body.')
    if(s.retainedProfile){
      if(s.analytic||s.dimensions?.length)throw Error('Retained profiles cannot carry polygon dimensions or a second analytic definition.')
      const profile=validateBrepProfile(s.retainedProfile.loops,'material-left',s.retainedProfile.toleranceMm)
      Object.assign(s,withRetainedProfile(s,profile))
    }
    if (s.analytic) { s.points = sampleCurve(s.analytic); s.closed = s.analytic.kind === 'circle' }
    if (s.plane) {
      const {origin,u,v}=s.plane
      if (![origin,u,v].every(p=>Array.isArray(p)&&p.length===3&&p.every(finite)) || Math.abs(dot3(u,u)-1)>1e-6 || Math.abs(dot3(v,v)-1)>1e-6 || Math.abs(dot3(u,v))>1e-6) throw new Error('Invalid sketch workplane.')
    }
    if (typeof s.closed !== 'boolean' || !Array.isArray(s.points) || s.points.length < 2 || s.points.length > (s.retainedProfile?8192:512) || (s.closed && s.points.length < 3) || !s.points.every(p => Array.isArray(p) && p.length === 2 && p.every(finite))) throw new Error('Invalid sketch.')
    if (s.dimensions !== undefined) {
      if (!Array.isArray(s.dimensions)) throw Error('Invalid sketch dimensions.')
      sketchDimensions(s)
    }
    yield
  }
  for (const b of d.bodies) {
    if(b.material!==undefined&&(!b.material||typeof b.material.name!=='string'||!b.material.name.length||b.material.name.length>100||typeof b.material.color!=='string'||!/^#[0-9a-f]{6}$/i.test(b.material.color)))throw Error('Invalid body material.')
    if(b.material&&[b.material.metallic,b.material.roughness,b.material.opacity].some(v=>v!==undefined&&(!Number.isFinite(v)||v<0||v>1)))throw Error('Invalid body material.')
    if (b.group !== undefined && (typeof b.group !== 'string' || b.group.length === 0 || b.group.length > 100)) {
      throw new Error('Invalid body group.')
    }
    // A compact instance omits both caches. Partially supplied caches still fail validation.
    if(b.instance&&b.mesh===undefined&&b.brep===undefined){yield;continue}
    const m = b.mesh
    if (!m || !Array.isArray(m.positions) || !Array.isArray(m.indices) || m.positions.length < 9 || m.positions.length > MAX_BODY_MESH_COMPONENTS || m.positions.length % 3 || m.indices.length < 3 || m.indices.length > MAX_BODY_MESH_COMPONENTS || m.indices.length % 3 || !m.positions.every(finite) || !m.indices.every(i => Number.isInteger(i) && i >= 0 && i < m.positions.length / 3)) throw new Error('Invalid body mesh.')
    // JSON boundary: plain parsed arrays are boxed into typed views exactly once.
    normalizePolygonMesh(m)
    if (b.brep) {
      ;(inspectedBreps ??= new BrepInspectionCache()).inspect(b.brep)
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
  // Validate serialized caches first, then derive linked geometry from validated sources.
  if(d.bodies.some(body=>body.instance)) {
    // Bound expansion before allocating geometry: a small reference document must not
    // multiply a large source into an unbounded set of materialized copies.
    const sourceSizes=new Map(d.bodies.filter(body=>!body.instance).map(body=>[
      body.id,
      // A transformed float may serialize much longer than an integer source value.
      // 24 digits plus JSON quotes conservatively covers every finite JSON number.
      JSON.stringify(JSON.parse(stringifyMeshJson(body)),(_key,value)=>typeof value==='number'?'0'.repeat(24):value).length,
    ]))
    // Use the same bound for full and compact input so every accepted state can
    // be restored from its compact history representation.
    let expanded=serializeDirectDocument(d).length
    for(const body of d.bodies)if(body.instance){
      expanded+=sourceSizes.get(body.instance.sourceId)??0
      if(expanded>MAX_DOCUMENT_CHARACTERS)throw Error('Expanded instance document exceeds 64 MB.')
    }
    d.bodies=resolveSolidInstances(d,instanceCache).bodies
    if(d.bodies.some(body=>body.instance&&!body.mesh.positions.every(finite)))throw Error('Instance placement exceeds document coordinate bounds.')
  }
  for (const item of d.curves) { validateNurbsCurve(item.curve); if(item.offsetRegion!==undefined&&!validCurveOffsetRegion(item.offsetRegion))throw Error('Invalid offset loop membership.'); if(item.offsetConstruction!==undefined&&!validCurveOffsetConstruction(item.offsetConstruction))throw Error('Invalid offset construction evidence.'); yield }
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
    // Keep the JSON round trip here: it also normalizes numbers (e.g. -0 → 0).
    return JSON.parse(JSON.stringify(d))
  }
}

export function parseDirectDocument(text: string, instanceCache?:SolidInstanceBatchCache): DirectDocument {
  const validation = directDocumentValidation(text,instanceCache)
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

const aabbFromPositions = (positions: ArrayLike<number>): [number, number, number, number, number, number] => {
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

/** Inactive snapshots retain compact text only; at most one current state owns geometry. */
interface DirectSnapshot { characters: number; text: string }
export class DirectHistory {
  private restoreGeneration = 0
  private past: DirectSnapshot[] = []
  private future: DirectSnapshot[] = []
  private current: DirectSnapshot & {document?:DirectDocument}
  constructor(document = emptyDirectDocument()) {
    const text = stringifyMeshJson(document)
    const validated = parseDirectDocument(text)
    const validatedText = serializeDirectDocument(validated)
    this.current = { document: validated, characters: validatedText.length, text: validatedText }
  }
  private get currentDocument():DirectDocument {
    return this.current.document ??= parseDirectDocument(this.current.text)
  }
  /** Export identity is metadata: retain it across existing Undo/Redo states without adding a geometry edit. */
  ensureBlenderProjectId(): string {
    if(this.currentDocument.blenderProjectId)return this.currentDocument.blenderProjectId
    const id=crypto.randomUUID()
    const stamp=(snapshot:DirectSnapshot):DirectSnapshot=>{
      const document=JSON.parse(snapshot.text) as DirectDocument
      if(document.blenderProjectId)return snapshot
      document.blenderProjectId=id
      const text=stringifyMeshJson(document)
      return {text,characters:text.length}
    }
    this.past=this.past.map(stamp);this.future=this.future.map(stamp)
    this.current={...stamp(this.current),document:{...this.currentDocument,blenderProjectId:id}}
    return id
  }
  get document() { return clone(this.currentDocument) }
  /** Detached metadata projection: callers needing only identities do not copy geometry. */
  get objectIds():string[] {
    // Async restore intentionally retains only a validated compact snapshot.
    // Reading identities must not resolve every linked instance on the UI thread.
    const d=this.current.document??JSON.parse(this.current.text) as DirectDocument
    return [...d.bodies,...d.sketches,...d.curves??[],...d.surfaces??[]].map(object=>object.id)
  }
  /** Immutable identity for consumers caching a committed snapshot across document copies. */
  get snapshotKey() { return this.current.text }
  get canUndo() { return this.past.length > 0 }
  get canRedo() { return this.future.length > 0 }
  get storageStats() {
    return {undoStates:this.past.length,redoStates:this.future.length,materializedStates:this.current.document?1:0,
      retainedCharacters:this.current.characters+[...this.past,...this.future].reduce((sum,snapshot)=>sum+snapshot.characters,0)}
  }
  commit(document: DirectDocument, validateChange?: (resolved:DirectDocument)=>void) {
    this.commitValidated(parseDirectDocument(stringifyMeshJson(document)),validateChange)
  }
  /** Loader must run the authoritative parser; no history changes while it runs. */
  async commitAsync(load:()=>Promise<DirectDocument>,validateChange?:(resolved:DirectDocument)=>void):Promise<boolean> {
    const generation=++this.restoreGeneration,base=this.current
    const loaded=await load()
    if(generation!==this.restoreGeneration||this.current!==base)return false
    this.commitValidated(clone(loaded),validateChange)
    return true
  }
  private commitValidated(next:DirectDocument,validateChange?:(resolved:DirectDocument)=>void) {
    const base=this.current
    next.blenderProjectId ??= this.currentDocument.blenderProjectId
    // UI policies (for example locked linked bodies) must see resolved geometry.
    // Give the policy its own copy so it cannot mutate the validated state.
    validateChange?.(clone(next))
    if(this.current!==base)throw Error('History changed during commit validation.')
    const nextText = serializeDirectDocument(next)
    if (nextText.length === this.current.characters && nextText === this.current.text) return
    this.past.push({text:this.current.text,characters:this.current.characters})
    // The existing bound counts serialized characters, not actual heap bytes.
    // Size travels with its snapshot through undo/redo; neither needs to serialize it again.
    let retained = this.past.reduce((sum, snapshot) => sum + snapshot.characters, 0)
    while (this.past.length > 80 || (this.past.length > 1 && retained > 16_000_000)) {
      retained -= this.past.shift()!.characters
    }
    this.current = { document: next, characters: nextText.length, text: nextText }; this.future = []
  }
  undo() {
    const d = this.past.at(-1)
    if (d) {
      const document=parseDirectDocument(d.text)
      this.past.pop();this.future.push({text:this.current.text,characters:this.current.characters});this.current={...d,document}
    }
    return this.document
  }
  redo() {
    const d = this.future.at(-1)
    if (d) {
      const document=parseDirectDocument(d.text)
      this.future.pop();this.past.push({text:this.current.text,characters:this.current.characters});this.current={...d,document}
    }
    return this.document
  }
  /** The loader must validate geometry with the authoritative parser. Startup
   * recovery replaces the baseline only if no intervening edit or cancellation occurred. */
  async resetAsync(load:()=>Promise<DirectDocument>):Promise<boolean> {
    const generation=++this.restoreGeneration,base=this.current
    const loaded=await load()
    if(generation!==this.restoreGeneration||this.current!==base)return false
    const document=clone(loaded),text=serializeDirectDocument(document)
    this.current={document,text,characters:text.length};this.past=[];this.future=[]
    return true
  }
  cancelRestore() { this.restoreGeneration++ }
  /** The loader must run the authoritative document parser (normally in a worker).
   * Keep both stacks intact until it succeeds against this exact history state. */
  async restoreAsync(direction:'undo'|'redo',load:(text:string)=>Promise<DirectDocument>):Promise<boolean> {
    const generation=++this.restoreGeneration,base=this.current
    const from=direction==='undo'?this.past:this.future,to=direction==='undo'?this.future:this.past
    const snapshot=from.at(-1)
    if(!snapshot)return false
    const loaded=await load(snapshot.text)
    if(generation!==this.restoreGeneration||this.current!==base||from.at(-1)!==snapshot)return false
    // A response for another snapshot cannot move the stacks. Geometry validation
    // belongs to the loader; re-parsing here would block the main thread again.
    if(serializeDirectDocument(loaded)!==snapshot.text)throw Error('History restore returned a different document.')
    // Retain the already validated immutable snapshot; materialize a private copy
    // only if a synchronous history consumer asks for geometry later.
    from.pop();to.push({text:base.text,characters:base.characters})
    this.current={...snapshot}
    return true
  }
}

export function extrudeDirectSketch(sketch: DirectSketch, height: number, id: string): DirectBody {
  if(sketch.retainedProfile)throw Error('Use the exact profile extrusion for retained curves.')
  if (!sketch.closed || sketch.points.length < 3) throw new Error('Close the contour before extrusion.')
  if (!finite(height) || height <= 0) throw new Error('Height must be positive.')
  const built = extrudePolygonProfile({ outer: sketch.points.map(p => [...p]) }, [0, 0, height])
  return { id, name: sketch.name + ' · 3D', mesh: { positions: Float64Array.from(worldPoints(Array.from({length:built.positions.length/3},(_,i)=>Array.from(built.positions.slice(i*3,i*3+3))),sketch.plane).flat()), indices: built.indices.slice() } }
}

/** Author the sketch extrusion before deriving a display mesh. */
export const extrudeSketchBrep=(sketch:DirectSketch,height:number,baseZ=0):NurbsBrep=>sketch.retainedProfile?extrudeSketchProfile([sketch],height,baseZ):callGeometryRust('brep_nurbs_sketch_extrude',{sketch,height,baseZ})

export function transformDirectPoints(points: number[][], delta: number[], angle: number, scale: number): number[][] {
  return callGeometryRust('cad_transform_points',{points,delta,angle,scale})
}

export { bodyPoints, directBodiesScad } from './directBodiesScad'
