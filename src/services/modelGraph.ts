import { prepareGraphRust } from './geometry/kernel'
import type { ModelGraphNumericType } from './modelGraphNumericType'
import { MECHANICAL_GENERATOR_GUIDE } from './mechanicalGeneratorContract'
import type { placeAssembly } from './modelGraphAssembly'
import type { solveModelGraphSketch } from './modelGraphSketch'
import { z } from 'zod/v4'
import type { Dimension, Unit } from './modelGraphUnits'
import { sha256Hex } from '../core/sha256'

export type CheckedValueType = {name:string;args?:CheckedValueType[];fields?:Record<string,CheckedValueType>}
export type MatchPattern =
  | {kind:'wildcard'}
  | {kind:'bind';name:string}
  | {kind:'literal';value:Expression}
  | {kind:'as';name:string;pattern:MatchPattern}
  | {kind:'or';patterns:MatchPattern[]}
  | {kind:'range';start:Expression;end:Expression;inclusive:boolean}
  | {kind:'list';prefix:MatchPattern[];suffix:MatchPattern[];rest?:string}
  | {kind:'record';fields:Record<string,MatchPattern>;exact:boolean}
  | {kind:'type';name:'int'|'f32'|'f64'|'str'|'length'|'angle'|'list'|'record';pattern:MatchPattern}
export type ScalarConstraint = {id:string;left:Expression;relation:'le'|'ge'|'eq'|'lt'|'gt';right:Expression;tolerance?:Expression;message:string}
export type GeometryAssertion = {id:string;target:string;check:'hasBodies'|'isWatertight'|'hasNoDegenerateTriangles'|'height'|'width'|'depth';expected?:Expression;tolerance?:Expression;message:string}
export type Expression =
  {op:'memo';id:string;value:Expression} |
  {op:'geometry_effects';input:string;value:Expression} |
  {op:'guarded';input:Expression;binding:string;steps:{kind:'assert'|'where'|'while';value:Expression}[]} |  {op:'assert_value';constraints?:ScalarConstraint[];geometry_assertions?:GeometryAssertion[];value:Expression} |
  { op: 'checked'; checks: Expression[]; value: Expression } |
  { op: 'typed'; type: ModelGraphNumericType; value: Expression } |
  { op: 'quantity'; value: number; unit: Unit }
  |   { op: 'geometry'; function: string; args: Record<string, Expression> }
  |   { op: 'lambda'; parameters: string[]; body: Expression }
  | { op: 'apply'; function: Expression; args: Expression[] }
  | { op: 'list'; items: Expression[] }
  | { op: 'range'; count: Expression; start: Expression; step: Expression }
  | { op: 'interval'; start: Expression; end: Expression; inclusive: boolean; count?: Expression; step?: Expression }
  | { op: 'zip'; inputs: Expression[] }
  | { op: 'enumerate'; input: Expression }
  | { op: 'map' | 'filter' | 'flatmap'; input: Expression; function: Expression }
  | { op: 'text'; value: string }
  | { op: 'record'; fields: Record<string, Expression> }
  | { op: 'field'; input: Expression; name: string }
  | { op: 'query'; method: string; input: Expression; function?: Expression; argument?: Expression; functions?: Expression[]; keys?: {function: Expression; descending: boolean}[] }
  | { op: 'reduce'; input: Expression; function: Expression; initial: Expression }
  | { op: 'at'; input: Expression; index: Expression }
  | { op: 'length'; input: Expression }
  | number | { param: string } | { local: string }
  | { op: 'add' | 'subtract' | 'multiply' | 'divide' | 'min' | 'max' | 'pow' | 'mod' | 'lt' | 'le' | 'eq' | 'and' | 'or'; args: [Expression, Expression] }
  | { op: 'negate' | 'abs' | 'sqrt' | 'sin' | 'cos' | 'floor' | 'ceil' | 'not'; value: Expression }
  | { op: 'typed_value'; value:Expression; type:CheckedValueType }
  | { op: 'match'; input:Expression; arms:{pattern:MatchPattern;guard?:Expression;body:Expression}[] }
  | { op: 'if'; condition: Expression; then: Expression; else: Expression }
  | { op: 'let'; name: string; value: Expression; body: Expression }
  | { op: 'call'; function: string; args: Record<string, Expression> }

// Keep MCP schema construction removable from browser builds, which validate in Rust.
function createModelGraphSchemas() {
const id = z.string().regex(/^[A-Za-z][A-Za-z0-9_]{0,31}$/)
const unit = z.enum(['mm', 'cm', 'm', 'in', 'deg', 'rad'])
const number = z.number().finite().min(-1_000_000).max(1_000_000)
const checkedValueTypeSchema: z.ZodType<CheckedValueType> = z.lazy(()=>z.object({name:id,args:z.array(checkedValueTypeSchema).max(16).optional(),fields:z.record(id,checkedValueTypeSchema).optional()}).strict())
const matchPatternSchema: z.ZodType<MatchPattern> = z.lazy(()=>z.discriminatedUnion('kind',[
  z.object({kind:z.literal('wildcard')}).strict(),
  z.object({kind:z.literal('bind'),name:id}).strict(),
  z.object({kind:z.literal('literal'),value:expressionSchema}).strict(),
  z.object({kind:z.literal('as'),name:id,pattern:matchPatternSchema}).strict(),
  z.object({kind:z.literal('or'),patterns:z.array(matchPatternSchema).min(1).max(32)}).strict(),
  z.object({kind:z.literal('range'),start:expressionSchema,end:expressionSchema,inclusive:z.boolean()}).strict(),
  z.object({kind:z.literal('list'),prefix:z.array(matchPatternSchema).max(256),suffix:z.array(matchPatternSchema).max(256),rest:z.union([id,z.literal('_')]).optional()}).strict(),
  z.object({kind:z.literal('record'),fields:z.record(id,matchPatternSchema),exact:z.boolean()}).strict(),
  z.object({kind:z.literal('type'),name:z.enum(['int','f32','f64','str','length','angle','list','record']),pattern:matchPatternSchema}).strict(),
]))
// MCP validates this schema before Rust execution; dispatch recursive operations once.
const expressionSchema: z.ZodType<Expression> = z.lazy(() => z.union([
  number, z.object({ param: id }).strict(), z.object({ local: id }).strict(),
  z.discriminatedUnion('op', [
    z.object({op:z.literal('memo'),id,value:expressionSchema}).strict(),
    z.object({op:z.literal('geometry_effects'),input:id,value:expressionSchema}).strict(),
    z.object({op:z.literal('guarded'),input:expressionSchema,binding:id,steps:z.array(z.object({kind:z.enum(['assert','where','while']),value:expressionSchema}).strict()).max(64)}).strict(),
    z.object({op:z.literal('assert_value'),constraints:z.array(scalarConstraintSchema).max(64).optional(),geometry_assertions:z.array(geometryAssertionSchema).max(64).optional(),value:expressionSchema}).strict(),
    z.object({op:z.literal('checked'),checks:z.array(expressionSchema).max(256),value:expressionSchema}).strict(),
    z.object({op:z.literal('typed'),type:z.enum(['int','f32','f64','length','angle']),value:expressionSchema}).strict(),
    z.object({ op: z.literal('quantity'), value: number, unit }).strict(),
    z.object({ op: z.literal('geometry'), function: id, args: z.record(id, expressionSchema) }).strict(),
    z.object({ op: z.literal('lambda'), parameters: z.array(id).max(32), body: expressionSchema }).strict(),
    z.object({ op: z.literal('apply'), function: expressionSchema, args: z.array(expressionSchema).max(32) }).strict(),
    z.object({ op: z.literal('list'), items: z.array(expressionSchema).max(256) }).strict(),
    z.object({ op: z.literal('range'), count: expressionSchema, start: expressionSchema, step: expressionSchema }).strict(),
    z.object({ op: z.literal('interval'), start: expressionSchema, end: expressionSchema, inclusive: z.boolean(), count: expressionSchema.optional(), step: expressionSchema.optional() }).strict(),
    z.object({ op: z.literal('zip'), inputs: z.array(expressionSchema).min(2).max(8) }).strict(),
    z.object({ op: z.literal('enumerate'), input: expressionSchema }).strict(),
    z.object({ op: z.enum(['map', 'filter', 'flatmap']), input: expressionSchema, function: expressionSchema }).strict(),
    z.object({op:z.literal('text'),value:z.string().max(4096)}).strict(),
    z.object({op:z.literal('record'),fields:z.record(id,expressionSchema)}).strict(),
    z.object({op:z.literal('field'),input:expressionSchema,name:id}).strict(),
    z.object({op:z.literal('query'),method:id,input:expressionSchema,function:expressionSchema.optional(),argument:expressionSchema.optional(),functions:z.array(expressionSchema).length(3).optional(),keys:z.array(z.object({function:expressionSchema,descending:z.boolean()}).strict()).min(1).max(8).optional()}).strict(),
    z.object({ op: z.literal('reduce'), input: expressionSchema, function: expressionSchema, initial: expressionSchema }).strict(),
    z.object({ op: z.literal('at'), input: expressionSchema, index: expressionSchema }).strict(),
    z.object({ op: z.literal('length'), input: expressionSchema }).strict(),
    z.object({ op: z.enum(['add', 'subtract', 'multiply', 'divide', 'min', 'max', 'pow', 'mod', 'lt', 'le', 'eq', 'and', 'or']), args: z.tuple([expressionSchema, expressionSchema]) }).strict(),
    z.object({ op: z.enum(['negate', 'abs', 'sqrt', 'sin', 'cos', 'floor', 'ceil', 'not']), value: expressionSchema }).strict(),
    z.object({op:z.literal('typed_value'),value:expressionSchema,type:checkedValueTypeSchema}).strict(),
    z.object({op:z.literal('match'),input:expressionSchema,arms:z.array(z.object({pattern:matchPatternSchema,guard:expressionSchema.optional(),body:expressionSchema}).strict()).min(1).max(32)}).strict(),
    z.object({ op: z.literal('if'), condition: expressionSchema, then: expressionSchema, else: expressionSchema }).strict(),
    z.object({ op: z.literal('let'), name: id, value: expressionSchema, body: expressionSchema }).strict(),
    z.object({ op: z.literal('call'), function: id, args: z.record(id, expressionSchema) }).strict(),
  ]),
]))
const scalarConstraintSchema: z.ZodType<ScalarConstraint> = z.object({id,left:expressionSchema,relation:z.enum(['le','ge','eq','lt','gt']),right:expressionSchema,tolerance:expressionSchema.optional(),message:z.string().min(1).max(256)}).strict()
const geometryAssertionSchema: z.ZodType<GeometryAssertion> = z.object({id,target:id,check:z.enum(['hasBodies','isWatertight','hasNoDegenerateTriangles','height','width','depth']),expected:expressionSchema.optional(),tolerance:expressionSchema.optional(),message:z.string().min(1).max(256)}).strict()
const scalar = expressionSchema
const vector = z.tuple([scalar, scalar, scalar])
const vector2 = z.tuple([scalar, scalar])
const sketchConstraint = z.discriminatedUnion('kind', [
  z.object({ id, kind: z.literal('fix'), point: id, at: vector2 }).strict(),
  z.object({ id, kind: z.enum(['horizontal', 'vertical', 'coincident']), a: id, b: id }).strict(),
  z.object({ id, kind: z.literal('distance'), a: id, b: id, value: scalar }).strict(),
  z.object({ id, kind: z.enum(['parallel', 'perpendicular', 'equal_length']), a: id, b: id, c: id, d: id }).strict(),
])
const frameSchema = z.object({ origin: vector, rotation: vector }).strict()
const affineRow = z.tuple([scalar, scalar, scalar, scalar])
const node = z.discriminatedUnion('op', [
  z.object({id,op:z.literal('assert'),input:id,constraints:z.array(scalarConstraintSchema).max(64).optional(),geometry_assertions:z.array(geometryAssertionSchema).max(64).optional()}).strict(),
  z.object({id,op:z.literal('gear'),teeth:scalar,module:scalar,pressure_angle:scalar.default(20),thickness:scalar,bore:scalar.default(0),backlash:scalar.default(0.15),clearance:scalar.default(0.5),internal:z.boolean().default(false),rim_width:scalar.default(6),flank_segments:scalar.default(6)}).strict(),
  z.object({id,op:z.literal('planetary_spinner'),inner_radius:scalar,outer_radius:scalar,bore:scalar,gap:scalar,height:scalar,helix_angle:scalar}).strict(),
  z.object({id,op:z.literal('planetary_gears'),sun_teeth:scalar,planet_teeth:scalar,planet_count:scalar,module:scalar,pressure_angle:scalar.default(20),thickness:scalar,bore:scalar.default(0),backlash:scalar.default(0.15),clearance:scalar.default(0.5),rim_width:scalar.default(6),flank_segments:scalar.default(6),carrier_angle:scalar.default(0)}).strict(),
  z.object({id,op:z.literal('thread'),diameter:scalar,pitch:scalar,length:scalar,internal:z.boolean().default(false),wall:scalar.default(3),clearance:scalar.default(0.2),starts:scalar.default(1),left_handed:z.boolean().default(false),segments_per_turn:scalar.default(32)}).strict(),

  z.object({ id, op: z.literal('affine'), input: id, rows: z.tuple([affineRow, affineRow, affineRow]) }).strict(),
  z.object({ id, op: z.literal('hull'), inputs: z.array(id).min(1).max(32) }).strict(),
  z.object({ id, op: z.literal('mirror'), normal: vector, input: id }).strict(),
  z.object({ id, op: z.literal('offset'), distance: scalar, input: id, mode: z.enum(['radius', 'delta']).optional() }).strict(),
  z.object({ id, op: z.literal('projection'), input: id }).strict(),
  z.object({ id, op: z.literal('section'), height: scalar, input: id }).strict(),
  z.object({ id, op: z.literal('advanced_extrude'), input: id, height: scalar, twist: scalar, top_scale: vector2, slices: z.number().int().min(1).max(64), center: z.boolean().default(false) }).strict(),
  z.object({ id, op: z.literal('cone'), radius_bottom: scalar, radius_top: scalar, height: scalar, center: z.boolean().default(false) }).strict(),
  z.object({ id, op: z.literal('torus'), major_radius: scalar, minor_radius: scalar }).strict(),
  z.object({ id, op: z.literal('linear_pattern'), input: id, count: scalar, step: vector }).strict(),
  z.object({ id, op: z.literal('circular_pattern'), input: id, count: scalar, angle_step: scalar }).strict(),

  z.object({ id, op: z.literal('loft'), profile: z.array(vector2).min(3).max(64), sections: z.array(z.object({ z: scalar, scale: vector2, offset: vector2 }).strict()).min(2).max(32) }).strict(),
  z.object({ id, op: z.literal('assembly'), components: z.array(z.object({ id, input: id, anchors: z.array(z.object({ id, origin: vector, rotation: vector }).strict()).max(32), placement: frameSchema.optional(), mate: z.object({ component: id, anchor: id, own_anchor: id, gap: scalar, rotation: vector, joint: z.object({ kind: z.enum(['revolute', 'slider']), position: scalar, min: scalar, max: scalar }).strict().optional() }).strict().optional() }).strict()).min(1).max(32) }).strict(),
  z.object({ id, op: z.literal('sketch'), points: z.array(z.object({ id, position: vector2 }).strict()).min(3).max(16), boundary: z.array(id).min(3).max(16), constraints: z.array(sketchConstraint).max(48), allow_underconstrained: z.boolean().default(false) }).strict(),
  z.object({ id, op: z.literal('rectangle'), size: z.union([vector2,scalar]), center: z.boolean().default(false) }).strict(),
  z.object({ id, op: z.literal('circle'), radius: scalar }).strict(),
  z.object({ id, op: z.literal('polygon'), points: z.array(vector2).min(3).max(256) }).strict(),
  z.object({ id, op: z.literal('extrude'), input: id, height: scalar, center: z.boolean().default(false) }).strict(),
  z.object({ id, op: z.literal('revolve'), input: id, angle: scalar }).strict(),
  z.object({ id, op: z.literal('evaluate'), value: scalar }).strict(),
  z.object({ id, op: z.literal('call'), function: id, args: z.record(id, scalar) }).strict(),
  z.object({id,op:z.literal('match'),value:scalar,arms:z.array(z.object({pattern:matchPatternSchema,guard:scalar.optional(),input:id}).strict()).min(1).max(32)}).strict(),
  z.object({ id, op: z.literal('if'), condition: scalar, then: id, else: id }).strict(),
  z.object({ id, op: z.literal('collect'), values: scalar, binding: id, input: id }).strict(),
  z.object({ id, op: z.literal('group'), inputs: z.array(id).max(256) }).strict(),
  z.object({ id, op: z.literal('map'), count: scalar, index: id, input: id }).strict(),
  z.object({ id, op: z.literal('box'), size: z.union([vector,scalar]), center: z.boolean().default(false) }).strict(),
  z.object({ id, op: z.literal('sphere'), radius: scalar }).strict(),
  z.object({ id, op: z.literal('cylinder'), radius: scalar, height: scalar, center: z.boolean().default(false) }).strict(),
  ...(['translate', 'rotate', 'scale'] as const).map(op => z.object({ id, op: z.literal(op), vector:z.union([vector,scalar]), input: id }).strict()),
  ...(['union', 'intersection'] as const).map(op => z.object({ id, op: z.literal(op), inputs: z.array(id).min(1).max(32) }).strict()),
  z.object({ id, op: z.literal('difference'), base: id, subtract: z.array(id).min(1).max(32) }).strict(),
])

const modelGraphSchema = z.object({
  language: z.literal('modelgraph/1'),
  units: z.literal('mm'),
  type_policy: z.enum(['legacy', 'strict']).optional(),
  constraints: z.array(scalarConstraintSchema).max(64).optional(),
  parameters: z.array(z.object({ id, value: number, unit: unit.optional(), min: number.optional(), max: number.optional(), integer: z.boolean().optional() }).strict()).max(64),
  nodes: z.array(node).min(1).max(128),
  root: id,
  functions: z.array(z.discriminatedUnion('kind', [
    z.object({ id, kind: z.enum(['scalar', 'value']), parameters: z.array(id).max(32), body: scalar }).strict(),
    z.object({ id, kind: z.literal('geometry'), parameters: z.array(id).max(32), nodes: z.array(node).min(1).max(128), root: id }).strict(),
  ])).max(32).optional(),
  geometry_assertions: z.array(geometryAssertionSchema).max(64).optional(),
  assertions: z.array(z.object({ condition: scalar, message: z.string().min(1).max(256) }).strict()).max(64).optional(),
  segments: z.number().int().min(12).max(128).default(48),
}).strict()
  return { modelGraphSchema, expressionSchema, matchPatternSchema }
}
export const { modelGraphSchema, expressionSchema, matchPatternSchema } = /* @__PURE__ */ createModelGraphSchemas()
export type ModelGraph = z.infer<typeof modelGraphSchema>
export class ModelGraphError extends Error {
  constructor(readonly code: string, readonly path: string, message: string, readonly details?: unknown) { super(message) }
}

function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`
  if (value !== null && typeof value === 'object') return `{${Object.keys(value).sort().map(key => `${JSON.stringify(key)}:${canonical((value as Record<string, unknown>)[key])}`).join(',')}}`
  return JSON.stringify(value)
}

export const hashModelGraphDocument = (document: unknown) => sha256Hex(canonical(document))

export type ModelGraphCompilation = {
  document: ModelGraph
  geometry_assertions: Array<Omit<NonNullable<ModelGraph['geometry_assertions']>[number], 'expected' | 'tolerance'> & {expected:number; tolerance:number; source?:string; instance_path?:string}>
  constraint_report: Array<{id:string;path:string;passed:boolean;status:'passed'|'failed';actual:number;expected:number;relation:string;tolerance:number;dimension:Dimension;message:string}>
  sketch_solutions: Array<ReturnType<typeof solveModelGraphSketch> & {instance_path:string}>
  assembly_components: Array<ReturnType<typeof placeAssembly>[number] & {instance_path:string;parent_path:string|null;is_assembly:boolean;source?:string}>
  mechanical_reports: Array<Record<string,unknown>>
  mechanical_parts: Array<Record<string,unknown>>
  document_sha256: string
  source: string
  source_map: Array<{node_id:string;line:number;instance_path:string}>
  execution_target: 'legacy/current+own-rust-cad'
}
/** Rust validates and evaluates the graph; JS only handles transport and revision hashing. */
export function compileModelGraph(value: unknown): ModelGraphCompilation {
  const result = prepareGraphRust<Omit<ModelGraphCompilation,'document_sha256'>>('graph',value)
  if (!result.ok) throw new ModelGraphError(result.error.code,result.error.path,result.error.message,result.error.details)
  return {...result.value,document_sha256:sha256Hex(canonical(result.value.document))}
}

export function setModelGraphParameters(value: unknown, expectedHash: string, updates: Array<{ id: string; value: number }>) {
  const compiled = compileModelGraph(value)
  if (compiled.document_sha256 !== expectedHash) throw new ModelGraphError('revision_conflict', '/expected_document_sha256', 'Document changed; read the current document and retry.')
  const known = new Set(compiled.document.parameters.map(item => item.id))
  const seen = new Set<string>()
  for (const update of updates) {
    if (!known.has(update.id) || seen.has(update.id)) throw new ModelGraphError('invalid_update', '/updates', 'Update IDs must exist and be unique.')
    seen.add(update.id)
  }
  const changed = new Map(updates.map(item => [item.id, item.value]))
  return compileModelGraph({ ...compiled.document, parameters: compiled.document.parameters.map(item => ({ ...item, value: changed.get(item.id) ?? item.value })) })
}

export const MODELGRAPH_EXAMPLE = {
  language: 'modelgraph/1', units: 'mm', parameters: [{ id: 'width', value: 40 }], segments: 48,
  nodes: [{ id: 'body', op: 'box', size: [{ param: 'width' }, 30, 8] }, { id: 'hole', op: 'cylinder', radius: 4, height: 10 }, { id: 'placedHole', op: 'translate', vector: [20, 15, -1], input: 'hole' }, { id: 'part', op: 'difference', base: 'body', subtract: ['placedHole'] }], root: 'part',
}

export const MODELGRAPH_GUIDE = MECHANICAL_GENERATOR_GUIDE + "\n\n" + `SVG workflows: modelgraph_svg_preview(svg,...) returns portable static artwork retaining colors, gradients, clipping, masks, filters and embedded images. modelgraph_svg_extrude(svg,height,...) converts geometry to self-contained SCAD and verifies its volume. Set includeModelGraph=true to also receive a validated editable modelgraph/1 document, document_sha256, modelgraph_source and modelgraph_analysis from a separate full build matching the SCAD volume and bounds. Its svg_height parameter controls extrusion height. Native conversion preserves holes and separate parts; graph limits (256 points per contour, 128 nodes and bounded profile validation work) return explicit errors without simplifying artwork. Default includeModelGraph=false retains SCAD-only output. CSS geometry (x/y/width/height/cx/cy/r/rx/ry and d:path()), selectors, !important and explicit use-context inheritance are supported. Non-scaling strokes, their dashes/markers and instance-dependent paint resources preserve intrinsic physical width; normalized artwork freezes them into filled outlines. CSS math/variables, viewport units and cascade at-rules require resolved values. CSS, use/symbol, nested viewports, text with bundled Noto Sans or supplied base64 fonts, styled/dashed strokes, markers and holes are supported. Vector mode uses tolerance in mm; effects require explicit geometryMode=silhouette with rasterSize=128..2048 and alphaThreshold=0.01..1 (pixel approximation of the rendered alpha channel). Default DPI is 96. modelgraph_svg_export(document,axis=x|y|z,face?) creates a silhouette or flattens one planar mesh face. Optional face={meshIndex,triangleIndex} uses zero-based full-build indices. No curved-surface unwrapping. CAD SVG crops to contour bounds and reimport resets the viewport origin. Limits: 4 MiB SVG input, 20000 contour points, 20000 source triangles, 4 MiB contour SVG output. Static SVG only: scripts, animation and external resources are rejected.\n\nMCP modelgraph_modify measures actual bounds and returns complete validated documents: modification {operation:"resize",size:[mm,mm,mm],anchor:"minimum"|"center"|"origin"}, or {operation:"split",axis:"x"|"y"|"z",position:mm}. Split returns negative and positive halves with no kerf. These edits use current measured bounds and must be repeated after parameter changes. Assemblies are excluded. Additional CAD operations: affine(input,rows:[[scalar,scalar,scalar,length],...]) applies three rows of an invertible 3x4 affine matrix, with implicit last row [0,0,0,1]. Profiles must remain in XY. hull(inputs) computes a convex hull of same-dimensional geometry. mirror(input,normal:[scalar,scalar,scalar]) reflects about a plane through the origin; normal is nonzero and must lie in XY for profiles. offset(input:profile,distance:length) is the current engine's rounded profile offset; negative distances erode. Corner discretization uses the engine default and is not controlled by document segments. projection(input:solid) projects onto XY; section(input:solid,height:length) intersects with the plane Z=height and returns an XY profile. Both require extrusion before using as root. advanced_extrude(input:profile,height:length,twist:angle,top_scale:[scalar,scalar],slices:1..64,center=false) makes a linearly tapered/twisted solid; height/scales must be positive and abs(twist)<=3600 degrees. cone(radius_bottom:length,radius_top:length,height:length,center=false) allows either radius zero, but not both. torus(major_radius:length,minor_radius:length) requires major>minor>0. linear_pattern(input,count:1..256,step:[length,length,length]) unions translated copies starting at index zero. circular_pattern(input,count:1..256,angle_step:angle) unions copies rotated around global Z; translate the input first for a nonzero orbit radius. Patterns preserve profile/solid dimension. All scalar fields support functional expressions and parameters. These are mesh operations and do not implement exact BRep fillets or face selection.
ModelGraph/1 is a declarative JSON modeling language for MCP. For our own rational NURBS kernel, read modelgraph_nurbs_language and select modelgraph/nurbs-1; it exposes curves, surfaces, mathematical editing and explicit mesh tessellation. Loft solids: {id,op:"loft",profile:[[length,length],...],sections:[{z:length,scale:[scalar,scalar],offset:[length,length]},...]}. The base XY polygon must be strictly convex, without collinear vertices; either winding is accepted. Use 3..64 vertices and 2..32 parallel XY sections, strictly increasing Z and positive scales. Each section scales the base profile about XY origin, then offsets it. Corresponding vertices are joined by explicit triangles; caps are closed. This is a piecewise planar mesh loft, not spline interpolation or NURBS. No twist, arbitrary profile changes or holes in the base profile; use solid difference for hollow adapters. Boolean results still require actual kernel validation. Produce documents matching the accompanying JSON Schema. Units: millimeters; rotations: degrees, OpenSCAD Euler order (X then Y then Z); right-handed Z-up. IDs match [A-Za-z][A-Za-z0-9_]{0,31}. Parameters and nodes each have unique IDs. Scalar fields accept numbers or {"param":"id"}; strings of code are forbidden; structured functional expressions are supported. box uses positive size[3], sphere positive radius, cylinder positive radius and height. Box/cylinder center defaults false. translate/rotate/scale take vector[3] and input node ID; scale factors cannot be zero. union/intersection take inputs; difference takes base and subtract. Transforms wrap the referenced geometry. Every node must be reachable from root. Shared references instantiate geometry at each use; cycles are forbidden. Limits: 64 parameters, 128 nodes, depth 32, 4096 expanded uses, segments 12..128 (default 48). Assembly nodes: {id,op:"assembly",components:[{id,input:solidNodeId,anchors:[{id,origin:[length,length,length],rotation:[angle,angle,angle]}],placement?:{origin,rotation},mate?:{component:targetComponentId,anchor:targetAnchorId,own_anchor:localAnchorId,gap:length,rotation:[angle,angle,angle]}}]}. An assembly must be the document root or a direct component of another assembly. Assemblies inside boolean operations, transforms or functions are rejected. There are at most 64 expanded components across all nesting levels. Each component must declare exactly one placement or mate. At most 32 components and 32 anchors per component. Component and local anchor IDs must be unique. Components remain separate top-level geometry objects; they are not implicitly unioned. References to the same solid instantiate independent components.
Placement is a fixed rigid frame using degrees with X then Y then Z rotation order. Fixed mates are acyclic dependencies, not a numerical joint solver. The transform is targetComponent * targetAnchor * localGapAndRotation * inverse(ownAnchor). Gap is a signed displacement along target anchor Z; rotation is an additional local Euler rotation. Axes align by default; opposing faces require an explicit rotation. Gaps are declared frame offsets, not measured surface clearances. Unknown components/anchors, cycles and ambiguous placements are errors. assembly_components in compile/check/report returns each component's id,input,world matrix and named world anchor matrices. Matrices are row-major. Combined STL/OBJ export does not preserve assembly identities; keep the ModelGraph JSON as the assembly source of truth. Nested assembly instances expose instance_path, parent_path and is_assembly; leaf components also expose standalone source in world coordinates. Optional mate.joint is {kind:"revolute"|"slider",position:scalar,min:scalar,max:scalar}. Revolute positions/limits are angles; slider positions/limits are lengths. Position must be within inclusive limits. The joint rotates around local Z or translates along local Z after gap/rotation and before inverse(ownAnchor). Parameters and expressions can drive positions. These are acyclic kinematic placements, not physics simulation or a closed-loop assembly constraint solver. Call modelgraph_interference to measure volume overlap between at most 8 leaf components at current positions. It returns pair paths, intersection_volume_mm3 and status overlap/no_volume_overlap/unknown. Kernel failures are unknown; never interpret them as clearance. A tolerance of 1e-6 mm3 applies. Contact, clearance and swept motion are not checked.
Call modelgraph_report for actual geometry measurements, operation provenance and front/top/isometric PNG views. Images are bounded to 20000 triangles and 8000000 raster candidates and may be unavailable; consult images_status. Reports do not certify printability or persistent CAD face identity. First call modelgraph_compile, fix errors using their JSON path, then modelgraph_check for actual geometry validation. Edit parameters through modelgraph_set_parameters using the returned document_sha256. Keep the returned document as the source of truth. Generated SCAD is an execution artifact usable with existing export tools. No tool here persists the graph. Stable IDs identify document nodes, not stable CAD faces. This frontend currently targets the existing Manifold mesh engine, not a new geometry kernel, exact B-rep, CUDA or a full OpenSCAD replacement.
Profiles and solid features: rectangle(size:[length,length],center=false), circle(radius:length), polygon(points:[[length,length],...]) create XY profiles. Polygon points implicitly close the boundary; do not repeat the first point. A polygon has 3..256 points and must be simple with nonzero area. Holes are modeled with profile difference; union/intersection/difference must combine the same geometry dimension. extrude(input:profileId,height:positiveLength,center=false) and revolve(input:profileId,angle:positiveAngle<=360deg) create solids. Revolve interprets profile X as radius and Y as height around Z; place the profile on one side of the axis and validate the actual result using modelgraph_check. Transforms of profiles must preserve XY: translate Z=0, rotate X=Y=0, scale Z=1. Root must be solid. Functions and geometry values can carry profiles; expected profile/solid dimension is checked when emitted. Node references use input as elsewhere. Polygon-validation work is capped at 262144 point-pair budget units across expanded uses. A sketch node adds a bounded straight-segment constraint solver; arc constraints and edge fillets are not implemented.`


export const MODELGRAPH_FUNCTIONAL_GUIDE = `Functional extension (backward compatible with ModelGraph/1):
Values are numbers, immutable lists, lexical closures and immutable geometry values. No mutation, IO, clock, random or eval. All numeric intermediates are finite and within +/-1000000. Predicates return 0 or 1; conditions treat zero as false. Trigonometry uses degrees.
Expressions: {param:id} reads a document parameter; {local:id} reads a lexical binding. Binary {op,args:[a,b]}: add, subtract, multiply, divide, mod, pow, min, max, lt, le, eq, and, or. Unary {op,value}: negate, abs, sqrt, sin, cos, floor, ceil, not. if uses condition, then, else and evaluates only the selected branch; and/or short circuit. let uses name,value,body; the new binding exists only in body. lambda uses parameters:[ids],body and captures its definition scope. apply uses function:expression,args:[expressions]; arguments are positional. Duplicate binders are errors; lexical shadowing is allowed.
Endpoint sequences: interval uses start,end,inclusive and optional count OR step expressions. Endpoints and step must share dimensions. Count is dimensionless integer 0..256, 0 gives empty, 1 gives start (exclusive equal endpoints with positive count are invalid); inclusive samples include end when count>1. Without count, step is nonzero; implicit +1 only for dimensionless integer endpoints. Wrong direction gives empty; unreachable endpoint is not appended. Count/step are mutually exclusive. zip(inputs) requires 2..8 equal-length lists; enumerate(input) produces [index,value] pairs. flatmap(input,function) concatenates returned sequences with maximum 256 output elements. Geometry collect node {id,op:"collect",values:expression,binding:localId,input:nodeId} expands separately emitted instances using each sequence value. group node {id,op:"group",inputs:[nodeIds]} likewise emits separate parts. They preserve separate scene meshes at root; enclosing booleans merge according to the boolean operation. Positional instance_path references are not persistent semantic keys. No joint/assembly semantics implied.
Lists: list uses items; range uses count,start,step (count 0..256); at uses input,index (zero based); length uses input. map and filter use input and function (one argument: element). reduce uses input,function,initial and folds left with callback(accumulator,element). Empty reduce returns initial. Lists may hold closures or lists; a geometric numeric field must resolve to a number.
Document functions: {id,kind:"scalar"|"value",parameters:[ids],body:expression} or {id,kind:"geometry",parameters:[ids],nodes:[...],root:id}. scalar must return a number; value can return a list or closure. Named call uses {op:"call",function:id,args:{parameter:expression}}. Geometry call is a node with an id and the same fields. Exact named argument matching is required. Named functions see document parameters and their own arguments, never caller locals. Pass closures as arguments to write higher-order value functions. The expression {op:"geometry",function:id,args:{name:expression}} constructs an immutable geometry value with bound arguments. A geometry node {id,op:"evaluate",value:expression} emits a geometry value. Closures may return geometry; lists and higher-order functions may carry geometry values. Geometry functions can accept geometry values and use evaluate nodes to compose them.
Geometry if nodes have condition,then:nodeId,else:nodeId. Geometry map nodes have count (1..256),index:localId,input:nodeId; each instance receives an immutable zero-based index and results are unioned. Node IDs are local to each geometry body. Document assertions:[{condition:expression,message:string}] run before geometry emission. source_map includes instance_path identifying function calls and map instances.
Bounds: 32 functions, 32 arguments per function, expression/call depth 32, 100000 expression steps, 16384 allocated list slots, 4096 expanded geometry nodes, geometry depth 32, 20000 input values and nesting depth 64. Recursion is permitted within these budgets; no tail-call optimization. Functions and inactive branches are schema checked; value-dependent errors are detected when evaluated. This is a bounded functional modeling DSL, not general-purpose JavaScript or a new CAD kernel.`

export const MODELGRAPH_FUNCTIONAL_EXAMPLE = {
  language: 'modelgraph/1', units: 'mm', parameters: [{ id: 'count', value: 3 }],
  functions: [{ id: 'block', kind: 'geometry', parameters: ['width'],
    nodes: [{ id: 'solid', op: 'box', size: [{ local: 'width' }, 2, 2] }], root: 'solid' }],
  assertions: [{ condition: { op: 'le', args: [{ param: 'count' }, 20] }, message: 'Use at most 20 blocks.' }],
  nodes: [
    { id: 'block', op: 'call', function: 'block', args: { width: { op: 'apply', function: { op: 'lambda', parameters: ['x'], body: { op: 'multiply', args: [{ local: 'x' }, 2] } }, args: [1] } } },
    { id: 'placed', op: 'translate', vector: [{ op: 'multiply', args: [{ local: 'i' }, 3] }, 0, 0], input: 'block' },
    { id: 'parts', op: 'map', count: { param: 'count' }, index: 'i', input: 'placed' },
  ], root: 'parts',
}


export const MODELGRAPH_UNITS_GUIDE = `Types, units and declared constraints:
For new documents use type_policy:"strict". Expressions {op:"quantity",value:number,unit:"mm"|"cm"|"m"|"in"|"deg"|"rad"} create typed lengths/angles. Values normalize internally to mm and degrees; returned documents retain the original units. Bare numbers are dimensionless. Default/legacy policy still interprets bare numbers as mm/degrees at geometry fields; strict policy requires correct dimensions there, except bare zero is accepted for any geometry dimension. Explicitly typed values are always checked, including in legacy mode.
Box sizes, radius, height and translation require length; rotation requires angle; scale, predicates, indices and counts require dimensionless numbers. Function arguments, closures, lists, geometry values and scalar-function returns preserve quantities. Checks occur during evaluation, not by whole-program static inference. Inactive branches retain lazy semantics.
Add/subtract/min/max/mod/comparisons require identical dimensions; implicit number-to-length promotion in arithmetic is forbidden. Multiply/divide combine dimensions; length/length is dimensionless. Powers require a dimensionless exponent; resulting length/angle exponents must be integers within +/-8. sqrt halves exponents. sin/cos accept angles, returning dimensionless values (legacy mode also accepts bare degrees). abs/negate preserve dimensions; floor/ceil round canonical mm/degrees, preserving dimensions. Typed ranges require matching start/step dimensions; count remains dimensionless. Numeric limits apply after conversion as well as during arithmetic.
Parameters may declare unit,min,max,integer. Bounds and update values are expressed in the parameter's declared unit; modelgraph_set_parameters preserves that unit and revalidates all bounds/constraints atomically. An omitted unit denotes a dimensionless parameter.
Document constraints:[{id,left:expression,relation:"le"|"ge"|"eq"|"lt"|"gt",right:expression,tolerance?:expression,message:string}] compare numeric values with identical dimensions. Tolerance must have the same dimensions and be nonnegative; omitted means exact comparison. lt/gt are strict comparisons without tolerance. le means left <= right+tolerance; ge means left >= right-tolerance; eq means abs(left-right) <= tolerance. IDs must be unique. Maximum 64 constraints. constraint_report contains id,path,passed,actual,expected,relation,tolerance,dimension:[lengthExponent,angleExponent],message; measurements use canonical mm/degrees. If any fail, compilation returns constraint_failed with the entire report in error.details and emits no geometry. Legacy assertions still work.
These are checks of declared expressions, not a constraint solver or measured mesh-wall analysis. A minimumWall constraint checks the expression you provide; it does not prove every wall in the finished mesh meets that minimum. No automatic parameter repair, sketches or geometric constraint solving is introduced.`

export const MODELGRAPH_UNITS_EXAMPLE = {
  language: 'modelgraph/1', units: 'mm', type_policy: 'strict',
  parameters: [{ id: 'width', value: 4, unit: 'cm', min: 1, max: 20 }, { id: 'wall', value: 2, unit: 'mm', min: 0.1, max: 10 }],
  constraints: [{ id: 'minimumWall', left: { param: 'wall' }, relation: 'ge', right: { op: 'quantity', value: 1.2, unit: 'mm' }, message: 'Declared plate thickness must be at least 1.2 mm.' }],
  nodes: [{ id: 'plate', op: 'box', size: [{ param: 'width' }, { op: 'quantity', value: 20, unit: 'mm' }, { param: 'wall' }] }], root: 'plate',
}


export const MODELGRAPH_SKETCH_GUIDE = `Sketch constraints (straight-segment profiles):
A profile node {id,op:"sketch",points:[{id,position:[x,y]}],boundary:[pointIds],constraints:[...],allow_underconstrained?:false} solves coordinates before profile emission. Positions are initial guesses, not fixed values. Point positions, fixed targets and distances accept length expressions, respecting strict units. Boundary is an ordered simple closed polygon; closure is implicit and IDs must be unique. Extra points may be construction points but their degrees of freedom still count.
Constraint forms: {id,kind:"fix",point:id,at:[x,y]}; {id,kind:"horizontal"|"vertical"|"coincident",a:pointId,b:pointId}; {id,kind:"distance",a,b,value:positiveLength}; {id,kind:"parallel"|"perpendicular"|"equal_length",a,b,c,d} relates segments a-b and c-d. Use coincident for zero distance. Parallel/perpendicular reject zero-length segments at the resulting solution.
The bounded numerical solver uses initial guesses to select a local solution. Limits: 3..16 points, at most 48 constraints, 64 iterations, 16 expanded sketch solves per compilation. Tolerance is 0.000001 mm. sketch_solutions contains solved coordinates, per-constraint residual_mm/satisfied, maximum residual, iterations, local degrees_of_freedom (Jacobian nullity) and redundant_equations (local equation-count minus rank). Rank is numerical, not a global uniqueness proof. Redundancy counts equations, not necessarily removable constraints.
Statuses: solved; underconstrained (satisfied but local freedom remains); inconsistent (unsatisfied linear system with augmented-rank conflict); not_converged (nonlinear solver exhausted its budget, singular start or degenerate segment). Unsatisfied residuals identify problematic constraints but are not a proven minimal conflict set. No global impossibility claim is made for nonlinear nonconvergence. By default only solved sketches emit geometry. allow_underconstrained:true explicitly accepts a locally underconstrained solution using the initial guess. Invalid or self-intersecting solved boundaries still fail profile validation. Compilation errors include the solver report in error.details; successful compilation and modelgraph_report include sketch_solutions.
No arc/circle/tangency solver, assembly solver or automatic repair is implemented by this node.`

export const MODELGRAPH_SKETCH_EXAMPLE = {
  language: 'modelgraph/1', units: 'mm', parameters: [{ id: 'width', value: 20 }],
  nodes: [{ id: 'outline', op: 'sketch', points: [
    { id: 'a', position: [0.2, -0.1] }, { id: 'b', position: [19, 1] }, { id: 'c', position: [21, 9] }, { id: 'd', position: [-1, 11] },
  ], boundary: ['a', 'b', 'c', 'd'], constraints: [
    { id: 'origin', kind: 'fix', point: 'a', at: [0, 0] },
    { id: 'bottom', kind: 'horizontal', a: 'a', b: 'b' }, { id: 'right', kind: 'vertical', a: 'b', b: 'c' },
    { id: 'top', kind: 'horizontal', a: 'c', b: 'd' }, { id: 'left', kind: 'vertical', a: 'd', b: 'a' },
    { id: 'width', kind: 'distance', a: 'a', b: 'b', value: { param: 'width' } }, { id: 'height', kind: 'distance', a: 'b', b: 'c', value: 10 },
  ] }, { id: 'part', op: 'extrude', input: 'outline', height: 3 }], root: 'part',
}


export const MODELGRAPH_ASSEMBLY_EXAMPLE = {
  language: 'modelgraph/1', units: 'mm', parameters: [{ id: 'gap', value: 0.3 }],
  nodes: [
    { id: 'body', op: 'box', size: [20, 10, 5] },
    { id: 'cover', op: 'box', size: [20, 10, 2] },
    { id: 'product', op: 'assembly', components: [
      { id: 'base', input: 'body', anchors: [{ id: 'top', origin: [0,0,5], rotation: [0,0,0] }], placement: { origin: [0,0,0], rotation: [0,0,0] } },
      { id: 'lid', input: 'cover', anchors: [{ id: 'bottom', origin: [0,0,0], rotation: [0,0,0] }], mate: { component: 'base', anchor: 'top', own_anchor: 'bottom', gap: { param: 'gap' }, rotation: [0,0,0] } },
    ] },
  ], root: 'product',
}

export const MODELGRAPH_LOFT_EXAMPLE = {
  language: 'modelgraph/1', units: 'mm', parameters: [],
  nodes: [{ id: 'adapter', op: 'loft', profile: [[-10,-10],[10,-10],[10,10],[-10,10]], sections: [
    { z: 0, scale: [1,1], offset: [0,0] },
    { z: 20, scale: [0.5,0.5], offset: [5,0] },
  ] }], root: 'adapter',
} as const
