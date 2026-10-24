import { languageRequest } from './languages/kernel'
/**
 * Kernel-neutral parameter plans for the OpenSCAD 2021.01 primitives.
 *
 * These resolvers stop at the language/kernel boundary: they preserve the
 * permissive conversions and empty-object decisions, but never construct a
 * Manifold or parser node.  Host limits are opt-in and are always surfaced as
 * an explicit reduction instead of silently truncating topology.
 */

export type OpenScadStablePrimitiveName =
  | 'cube'
  | 'square'
  | 'sphere'
  | 'circle'
  | 'cylinder'
  | 'polyhedron'
  | 'polygon'

export type OpenScadStablePrimitiveWarningCode =
  | 'OPENSCAD_PRIMITIVE_SIZE_DEFAULTED'
  | 'OPENSCAD_PRIMITIVE_RANGE_EMPTY'
  | 'OPENSCAD_PRIMITIVE_RADIUS_SHADOWED'
  | 'OPENSCAD_CYLINDER_AMBIGUOUS_RADII'
  | 'OPENSCAD_POLYHEDRON_TRIANGLES_DEPRECATED'
  | 'OPENSCAD_PRIMITIVE_POINT_INVALID'
  | 'OPENSCAD_PRIMITIVE_INDEX_OUT_OF_BOUNDS'
  | 'OPENSCAD_PRIMITIVE_SAFETY_REDUCTION'

export interface OpenScadStablePrimitiveWarning {
  readonly code: OpenScadStablePrimitiveWarningCode
  readonly message: string
  readonly primitive: OpenScadStablePrimitiveName
  readonly field?: string
  readonly value?: unknown
  readonly index?: number
  readonly limit?: number
  readonly actual?: number
}

export interface OpenScadStablePrimitiveContext {
  readonly warn: (warning: OpenScadStablePrimitiveWarning) => void
  /** Mirrors the optional OpenSCAD parameter-range diagnostic switch. */
  readonly checkParameterRanges?: boolean
}

const SILENT_CONTEXT: OpenScadStablePrimitiveContext = Object.freeze({ warn() {} })

export type OpenScadVec2 = readonly [number, number]
export type OpenScadVec3 = readonly [number, number, number]

export interface OpenScadPrimitiveSafetyLimits {
  readonly maximumPoints?: number
  readonly maximumFacesOrPaths?: number
  readonly maximumIndices?: number
}

export interface OpenScadPrimitiveReduction {
  readonly reason: 'input-limit'
  readonly resource: 'points' | 'faces' | 'paths' | 'indices'
  readonly actual: number
  readonly limit: number
}

interface OpenScadPrimitivePlanBase {
  readonly kind: OpenScadStablePrimitiveName
  readonly empty: boolean
  readonly reduced: boolean
  readonly reduction: OpenScadPrimitiveReduction | null
}

export interface OpenScadBoxPrimitivePlan extends OpenScadPrimitivePlanBase {
  readonly kind: 'cube' | 'square'
  readonly dimensions: OpenScadVec3 | OpenScadVec2
  readonly center: boolean
  readonly sizeSource: 'default' | 'scalar' | 'vector' | 'partial-vector'
}

export interface OpenScadRadialPrimitivePlan extends OpenScadPrimitivePlanBase {
  readonly kind: 'sphere' | 'circle'
  readonly radius: number
  readonly radiusSource: 'default' | 'radius' | 'diameter'
  /** Radius passed to the shared fragment resolver. */
  readonly fragmentRadius: number
}

export interface OpenScadCylinderPrimitivePlan extends OpenScadPrimitivePlanBase {
  readonly kind: 'cylinder'
  readonly height: number
  readonly radius1: number
  readonly radius2: number
  readonly center: boolean
  readonly radius1Source: 'default' | 'radius' | 'diameter' | 'radius1' | 'diameter1'
  readonly radius2Source: 'default' | 'radius' | 'diameter' | 'radius2' | 'diameter2'
  /** Largest end radius passed to the shared fragment resolver. */
  readonly fragmentRadius: number
}

export interface OpenScadPolyhedronPrimitivePlan extends OpenScadPrimitivePlanBase {
  readonly kind: 'polyhedron'
  readonly vertices: readonly number[]
  readonly indices: readonly number[]
  readonly usable: boolean
  /** Coordinate polygons in authored face order, after index conversion/skips. */
  readonly polygons: readonly (readonly OpenScadVec3[])[]
  readonly facesSource: 'faces' | 'triangles' | 'default'
  readonly convexity: number
  /** A referenced malformed point ends conversion at that exact face. */
  readonly aborted: boolean
  readonly sourcePointCount: number
}

export interface OpenScadPolygonPrimitivePlan extends OpenScadPrimitivePlanBase {
  readonly kind: 'polygon'
  readonly points: readonly OpenScadVec2[]
  readonly outlines: readonly (readonly OpenScadVec2[])[]
  readonly pathSource: 'implicit' | 'explicit'
  readonly convexity: number
  readonly aborted: boolean
}

export interface OpenScadBoxPrimitiveInput {
  readonly size?: unknown
  readonly center?: unknown
}

export interface OpenScadRadialPrimitiveInput {
  readonly r?: unknown
  readonly d?: unknown
}

export interface OpenScadCylinderPrimitiveInput extends OpenScadRadialPrimitiveInput {
  readonly h?: unknown
  readonly r1?: unknown
  readonly r2?: unknown
  readonly d1?: unknown
  readonly d2?: unknown
  readonly center?: unknown
}

export interface OpenScadPolyhedronPrimitiveInput {
  readonly points?: unknown
  readonly faces?: unknown
  readonly triangles?: unknown
  readonly convexity?: unknown
  readonly limits?: OpenScadPrimitiveSafetyLimits
}

export interface OpenScadPolygonPrimitiveInput {
  readonly points?: unknown
  readonly paths?: unknown
  readonly convexity?: unknown
  readonly limits?: OpenScadPrimitiveSafetyLimits
}

function freeze2(x: number, y: number): OpenScadVec2 {
  return Object.freeze([x, y])
}

function freeze3(x: number, y: number, z: number): OpenScadVec3 {
  return Object.freeze([x, y, z])
}

function warn(
  context: OpenScadStablePrimitiveContext,
  warning: OpenScadStablePrimitiveWarning,
): void {
  context.warn(Object.freeze(warning))
}

function rangeEmpty(
  primitive: OpenScadStablePrimitiveName,
  value: unknown,
  context: OpenScadStablePrimitiveContext,
): void {
  if (!context.checkParameterRanges) return
  warn(context, {
    code: 'OPENSCAD_PRIMITIVE_RANGE_EMPTY',
    message: `${primitive} parameters describe an empty object`,
    primitive,
    value,
  })
}

function boxPlan(
  kind: 'cube' | 'square',
  axes: 2 | 3,
  input: OpenScadBoxPrimitiveInput,
  context: OpenScadStablePrimitiveContext,
): OpenScadBoxPrimitivePlan {
  const encodeNumber = (value: number): number | string => Number.isFinite(value) ? value : String(value)
  const size = typeof input.size === 'number' ? encodeNumber(input.size)
    : Array.isArray(input.size) && input.size.length <= 3
      ? input.size.map(value => typeof value === 'number' ? encodeNumber(value) : null)
      : null
  const response = languageRequest(17, {kind, size, missing: input.size === undefined, center: input.center === true}) as {
    ok: boolean; value: {dimensions: (number | string)[]; sizeSource: OpenScadBoxPrimitivePlan['sizeSource']; empty: boolean; defaulted: boolean}; error?: {message: string}
  }
  if (!response.ok) throw new Error(response.error?.message ?? 'Native box normalization failed')
  const values = response.value.dimensions.map(value => typeof value === 'number' ? value : Number(value))
  const sizeSource = response.value.sizeSource
  if (response.value.defaulted) {
    warn(context, {
      code: 'OPENSCAD_PRIMITIVE_SIZE_DEFAULTED',
      message: `${kind} size was not a scalar or exact ${axes}-component numeric vector; unit size is used`,
      primitive: kind,
      field: 'size',
      value: input.size,
    })
  }

  const empty = response.value.empty
  if (empty) rangeEmpty(kind, input.size, context)
  return Object.freeze({
    kind,
    empty,
    reduced: false,
    reduction: null,
    dimensions: axes === 3
      ? freeze3(values[0], values[1], values[2])
      : freeze2(values[0], values[1]),
    center: input.center === true,
    sizeSource,
  })
}

export function resolveOpenScadCube(
  input: OpenScadBoxPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadBoxPrimitivePlan {
  return boxPlan('cube', 3, input, context)
}

export function resolveOpenScadSquare(
  input: OpenScadBoxPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadBoxPrimitivePlan {
  return boxPlan('square', 2, input, context)
}

function nativeRadialPlan(
  kind: 'sphere' | 'circle' | 'cylinder',
  input: OpenScadCylinderPrimitiveInput,
): Record<string, unknown> & {shadowed: boolean[]; empty: boolean; ambiguous?: boolean} {
  const request: Record<string, unknown> = {kind}
  for (const field of ['r','d','r1','d1','r2','d2','h'] as const) {
    const value = input[field]
    request[field] = typeof value === 'number' ? Number.isFinite(value) ? value : String(value) : null
  }
  const response = languageRequest(18,request) as {ok:boolean;value:Record<string,unknown> & {shadowed:boolean[];empty:boolean;ambiguous?:boolean};error?:{message:string}}
  if (!response.ok) throw new Error(response.error?.message ?? 'Native radial normalization failed')
  for (const field of ['radius','radius1','radius2','height','fragmentRadius']) {
    const value = response.value[field]
    if (typeof value === 'string') response.value[field] = Number(value)
  }
  return response.value
}

function radiusWarnings(
  kind: 'sphere' | 'circle' | 'cylinder',
  input: OpenScadCylinderPrimitiveInput,
  shadowed: readonly boolean[],
  context: OpenScadStablePrimitiveContext,
): void {
  const fields = ['r','r1','r2'] as const
  const diameters = ['d','d1','d2'] as const
  shadowed.forEach((present,index) => {
    if (present) warn(context, {
      code: 'OPENSCAD_PRIMITIVE_RADIUS_SHADOWED',
      message: `${kind} uses ${diameters[index]}; the paired ${fields[index]} value has no effect`,
      primitive: kind, field: fields[index], value: input[fields[index]],
    })
  })
}

function radialPlan(
  kind: 'sphere' | 'circle',
  input: OpenScadRadialPrimitiveInput,
  context: OpenScadStablePrimitiveContext,
): OpenScadRadialPrimitivePlan {
  const {shadowed,...plan} = nativeRadialPlan(kind,input)
  radiusWarnings(kind,input,shadowed,context)
  if (plan.empty) rangeEmpty(kind,plan.radius,context)
  return Object.freeze({kind,...plan,reduced:false,reduction:null}) as unknown as OpenScadRadialPrimitivePlan
}

export function resolveOpenScadSphere(
  input: OpenScadRadialPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadRadialPrimitivePlan {
  return radialPlan('sphere', input, context)
}

export function resolveOpenScadCircle(
  input: OpenScadRadialPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadRadialPrimitivePlan {
  return radialPlan('circle', input, context)
}

export function resolveOpenScadCylinder(
  input: OpenScadCylinderPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadCylinderPrimitivePlan {
  const {shadowed,ambiguous,...plan} = nativeRadialPlan('cylinder',input)
  radiusWarnings('cylinder',input,shadowed,context)
  if (ambiguous) warn(context, {
    code: 'OPENSCAD_CYLINDER_AMBIGUOUS_RADII',
    message: 'cylinder combines a shared radius with an end-specific radius',
    primitive: 'cylinder',
  })
  if (plan.empty) rangeEmpty('cylinder', {height:plan.height,radius1:plan.radius1,radius2:plan.radius2}, context)
  return Object.freeze({kind:'cylinder',...plan,center:input.center===true,reduced:false,reduction:null}) as unknown as OpenScadCylinderPrimitivePlan
}

interface IndexedPolicy {
  readonly maximumPoints:number
  readonly maximumFacesOrPaths:number
  readonly maximumIndices:number
  readonly convexity:number
}
function indexedPolicy(limits:OpenScadPrimitiveSafetyLimits|undefined,convexity:unknown):IndexedPolicy {
  const encode=(value:unknown)=>typeof value==='number'?Number.isFinite(value)?value:String(value):null
  const values=[limits?.maximumPoints,limits?.maximumFacesOrPaths,limits?.maximumIndices]
  const response=languageRequest(30,{limits:values.map(value=>({missing:value===undefined,number:encode(value)})),convexity:encode(convexity)}) as {ok:boolean;field?:string;value:{limits:(number|string)[];convexity:number}}
  if(!response.ok)throw new RangeError(`${response.field} must be a non-negative safe integer`)
  const [maximumPoints,maximumFacesOrPaths,maximumIndices]=response.value.limits.map(Number)
  return Object.freeze({maximumPoints,maximumFacesOrPaths,maximumIndices,convexity:response.value.convexity})
}

function reduction(
  primitive: 'polyhedron' | 'polygon',
  resource: OpenScadPrimitiveReduction['resource'],
  actual: number,
  maximum: number,
  context: OpenScadStablePrimitiveContext,
): OpenScadPrimitiveReduction {
  const result = Object.freeze({ reason: 'input-limit' as const, resource, actual, limit: maximum })
  warn(context, {
    code: 'OPENSCAD_PRIMITIVE_SAFETY_REDUCTION',
    message: `${primitive} was not expanded because its ${resource} exceed the configured engine limit`,
    primitive,
    field: resource,
    actual,
    limit: maximum,
  })
  return result
}

function asVector(value: unknown): readonly unknown[] {
  return Array.isArray(value) ? value : []
}

function frozenPolygons3(polygons: OpenScadVec3[][]): readonly (readonly OpenScadVec3[])[] {
  return Object.freeze(polygons.map(polygon => Object.freeze([...polygon])))
}

function frozenOutlines2(outlines: OpenScadVec2[][]): readonly (readonly OpenScadVec2[])[] {
  return Object.freeze(outlines.map(outline => Object.freeze([...outline])))
}

function indexedNumber(value:unknown):number|string|null {
  return typeof value==='number'?Number.isFinite(value)?value:String(value):null
}
/** Bound transport at the first native index-limit event, preserving authored positions. */
function indexedRows(rows:readonly unknown[],maximum:number):(number|string|null)[][] {
  let remaining=maximum+1
  return Array.from(rows,row=>{
    const entries=asVector(row),count=Math.min(entries.length,remaining)
    remaining-=count
    return Array.from(entries.slice(0,count),indexedNumber)
  })
}

export function resolveOpenScadPolyhedron(
  input: OpenScadPolyhedronPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadPolyhedronPrimitivePlan {
  const limits = indexedPolicy(input.limits,input.convexity)
  const points = asVector(input.points)
  if (points.length > limits.maximumPoints) {
    const limited = reduction('polyhedron', 'points', points.length, limits.maximumPoints, context)
    return Object.freeze({
      kind: 'polyhedron', vertices: Object.freeze([]), indices: Object.freeze([]), usable: false, polygons: Object.freeze([]), facesSource: 'default', convexity: limits.convexity,
      aborted: true, sourcePointCount: points.length, empty: true, reduced: true, reduction: limited,
    })
  }

  const facesSource: OpenScadPolyhedronPrimitivePlan['facesSource'] = input.faces !== undefined
    ? 'faces'
    : input.triangles !== undefined ? 'triangles' : 'default'
  if (facesSource === 'triangles') warn(context, {
    code: 'OPENSCAD_POLYHEDRON_TRIANGLES_DEPRECATED',
    message: 'polyhedron triangles is a legacy alias; faces is the stable spelling',
    primitive: 'polyhedron',
    field: 'triangles',
  })
  const faces = asVector(facesSource === 'faces' ? input.faces : facesSource === 'triangles' ? input.triangles : undefined)
  if (faces.length > limits.maximumFacesOrPaths) {
    const limited = reduction('polyhedron', 'faces', faces.length, limits.maximumFacesOrPaths, context)
    return Object.freeze({
      kind: 'polyhedron', vertices: Object.freeze([]), indices: Object.freeze([]), usable: false, polygons: Object.freeze([]), facesSource, convexity: limits.convexity,
      aborted: true, sourcePointCount: points.length, empty: true, reduced: true, reduction: limited,
    })
  }

  const nativePoints=points.map(point=>Array.isArray(point)&&point.length<=3?Array.from(point,indexedNumber):[])
  const maximumIndices=Number.isFinite(limits.maximumIndices)?limits.maximumIndices:Number.MAX_SAFE_INTEGER
  const nativeFaces=indexedRows(faces,maximumIndices)
  type Event={kind:'bounds'|'point'|'limit';face?:number;entry?:number;index?:number;actual?:number}
  const response=languageRequest(28,{points:nativePoints,faces:nativeFaces,maximumIndices}) as {ok:boolean;value:{vertices:number[];indices:number[];usable:boolean;polygons:number[][][];events:Event[];aborted:boolean;empty:boolean};error?:{message:string}}
  if(!response.ok)throw new Error(response.error?.message??'Native polyhedron expansion failed')
  let limited:OpenScadPrimitiveReduction|null=null
  for(const event of response.value.events){
    if(event.kind==='limit')limited=reduction('polyhedron','indices',event.actual!,limits.maximumIndices,context)
    else if(event.kind==='bounds')warn(context,{
      code:'OPENSCAD_PRIMITIVE_INDEX_OUT_OF_BOUNDS',
      message:'polyhedron skipped a face entry whose point index is outside the point vector',
      primitive:'polyhedron',field:`faces[${event.face}]`,index:event.index,
      value:asVector(faces[event.face!])[event.entry!],
    })
    else warn(context,{
      code:'OPENSCAD_PRIMITIVE_POINT_INVALID',
      message:'polyhedron stopped after a referenced point failed exact vec2/vec3 conversion',
      primitive:'polyhedron',field:`points[${event.index}]`,index:event.index,value:points[event.index!],
    })
  }
  const polygons=response.value.polygons.map(polygon=>polygon.map(point=>freeze3(point[0],point[1],point[2])))
  return Object.freeze({kind:'polyhedron',vertices:Object.freeze(response.value.vertices),indices:Object.freeze(response.value.indices),usable:response.value.usable,polygons:frozenPolygons3(polygons),facesSource,
    convexity:limits.convexity,aborted:response.value.aborted,sourcePointCount:points.length,
    empty:response.value.empty,reduced:limited!==null,reduction:limited})
}

export function resolveOpenScadPolygon(
  input: OpenScadPolygonPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadPolygonPrimitivePlan {
  const limits = indexedPolicy(input.limits,input.convexity)
  const authoredPoints = asVector(input.points)
  if (authoredPoints.length > limits.maximumPoints) {
    const limited = reduction('polygon', 'points', authoredPoints.length, limits.maximumPoints, context)
    return Object.freeze({
      kind: 'polygon', points: Object.freeze([]), outlines: Object.freeze([]), pathSource: 'implicit',
      convexity: limits.convexity, aborted: true, empty: true, reduced: true, reduction: limited,
    })
  }

  const points=Array.from(authoredPoints,point=>Array.isArray(point)&&point.length===2?Array.from(point,indexedNumber):[])
  const authoredPaths=asVector(input.paths)
  const maximumPaths=Number.isFinite(limits.maximumFacesOrPaths)?limits.maximumFacesOrPaths:Number.MAX_SAFE_INTEGER
  const maximumIndices=Number.isFinite(limits.maximumIndices)?limits.maximumIndices:Number.MAX_SAFE_INTEGER
  const paths=authoredPaths.length>maximumPaths?[]:indexedRows(authoredPaths,maximumIndices)
  type Event={kind:'bounds'|'point'|'limit'|'paths';face?:number;entry?:number;index?:number;actual?:number}
  type NativePoint=(number|string)[]
  const response=languageRequest(29,{points,paths,pathCount:authoredPaths.length,maximumPaths,maximumIndices}) as {ok:boolean;value:{points:NativePoint[];outlines:NativePoint[][];events:Event[];pathSource:'implicit'|'explicit';aborted:boolean;empty:boolean};error?:{message:string}}
  if(!response.ok)throw new Error(response.error?.message??'Native polygon expansion failed')
  let limited:OpenScadPrimitiveReduction|null=null
  for(const event of response.value.events){
    if(event.kind==='limit'||event.kind==='paths')limited=reduction('polygon',event.kind==='limit'?'indices':'paths',event.actual!,event.kind==='limit'?limits.maximumIndices:limits.maximumFacesOrPaths,context)
    else if(event.kind==='bounds')warn(context,{
      code:'OPENSCAD_PRIMITIVE_INDEX_OUT_OF_BOUNDS',
      message:'polygon skipped a path entry whose point index is outside the point vector',
      primitive:'polygon',field:`paths[${event.face}]`,index:event.index,
      value:asVector(authoredPaths[event.face!])[event.entry!],
    })
    else warn(context,{
      code:'OPENSCAD_PRIMITIVE_POINT_INVALID',
      message:'polygon produced no outlines because a point failed exact vec2 conversion',
      primitive:'polygon',field:`points[${event.index}]`,index:event.index,value:authoredPoints[event.index!],
    })
  }
  const point=(values:NativePoint)=>freeze2(Number(values[0]),Number(values[1]))
  return Object.freeze({kind:'polygon',points:Object.freeze(response.value.points.map(point)),
    outlines:frozenOutlines2(response.value.outlines.map(outline=>outline.map(point))),
    pathSource:response.value.pathSource,convexity:limits.convexity,aborted:response.value.aborted,
    empty:response.value.empty,reduced:limited!==null,reduction:limited})
}
