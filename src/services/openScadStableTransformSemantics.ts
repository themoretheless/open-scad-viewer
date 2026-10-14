import { languageRequest } from './languages/kernel'
/**
 * Kernel-neutral transform plans for OpenSCAD 2021.01.
 *
 * Matrices are column-major so they can flow directly into the independent
 * semantic/runtime contracts. Authored multmatrix values remain row-major at
 * the API boundary, matching the language. No safety approximation is made:
 * every plan reports `reduced: false`, while non-finite matrices explicitly
 * report that OpenSCAD removes their child geometry during evaluation.
 */

export type OpenScadStableTransformName =
  | 'translate'
  | 'scale'
  | 'mirror'
  | 'rotate'
  | 'multmatrix'

export type OpenScadStableTransformWarningCode =
  | 'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED'
  | 'OPENSCAD_TRANSFORM_PARAMETER_RANGE'
  | 'OPENSCAD_ROTATE_AXIS_IGNORED'
  | 'OPENSCAD_ROTATE_PARTIAL_VECTOR'
  | 'OPENSCAD_TRANSFORM_NONFINITE_EMPTY'

export interface OpenScadStableTransformWarning {
  readonly code: OpenScadStableTransformWarningCode
  readonly message: string
  readonly transform: OpenScadStableTransformName
  readonly field?: string
  readonly value?: unknown
}

export interface OpenScadStableTransformContext {
  readonly warn: (warning: OpenScadStableTransformWarning) => void
  /** Mirrors the optional OpenSCAD parameter-range diagnostic switch. */
  readonly checkParameterRanges?: boolean
}

const SILENT_CONTEXT: OpenScadStableTransformContext = Object.freeze({ warn() {} })

export type OpenScadMatrix4 = readonly [
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
]

export type OpenScadMatrix3 = readonly [
  number, number, number,
  number, number, number,
  number, number, number,
]

export interface OpenScadStableTransformPlan {
  readonly kind: OpenScadStableTransformName
  readonly matrix: OpenScadMatrix4
  /** Full XY/homogeneous submatrix, including an authored projective row. */
  readonly matrix2d: OpenScadMatrix3
  readonly parameterValid: boolean
  /** OpenSCAD discards the transformed node when this is true. */
  readonly dropsChildren: boolean
  readonly matrix3dSingular: boolean
  readonly matrix2dSingular: boolean
  readonly affine3d: boolean
  readonly affine2d: boolean
  /** Pure language normalization never hides an engine approximation. */
  readonly reduced: false
  readonly reduction: null
}

export interface OpenScadRotateInput {
  readonly a?: unknown
  readonly v?: unknown
}

function warn(
  context: OpenScadStableTransformContext,
  warning: OpenScadStableTransformWarning,
): void {
  context.warn(Object.freeze(warning))
}

function canonical(value: number): number {
  // Do not round authored matrix/translation/scale values. The degree helpers
  // already return exact quadrant values; only signed zero is observationally
  // irrelevant and awkward for deterministic plans.
  return Object.is(value, -0) ? 0 : value
}

function freezeMatrix4(values: readonly number[]): OpenScadMatrix4 {
  return Object.freeze(values.map(canonical)) as unknown as OpenScadMatrix4
}

function freezeMatrix3(values: readonly number[]): OpenScadMatrix3 {
  return Object.freeze(values.map(canonical)) as unknown as OpenScadMatrix3
}



/** Extract OpenSCAD's complete XY/homogeneous transform from a 4x4 matrix. */
type NativeMatrixAnalysis = Pick<OpenScadStableTransformPlan,'matrix2d'|'dropsChildren'|'matrix3dSingular'|'matrix2dSingular'|'affine3d'|'affine2d'>
function analyzeMatrix(matrix: OpenScadMatrix4): NativeMatrixAnalysis {
  const response = languageRequest(19,{matrix:matrix.map(value=>Number.isFinite(value)?value:String(value))}) as {
    ok:boolean;value:Omit<NativeMatrixAnalysis,'matrix2d'> & {matrix2d:(number|string)[]};error?:{message:string}
  }
  if (!response.ok) throw new Error(response.error?.message ?? 'Native transform analysis failed')
  return {...response.value,matrix2d:freezeMatrix3(response.value.matrix2d.map(value=>typeof value==='number'?value:Number(value)))}
}
export function openScadTransform2dSubmatrix(matrix: OpenScadMatrix4): OpenScadMatrix3 {
  return analyzeMatrix(matrix).matrix2d
}

interface NativeMatrixPlan {matrix:OpenScadMatrix4;analysis:NativeMatrixAnalysis}
interface NativeMatrixWire {matrix:(number|string)[];analysis:Omit<NativeMatrixAnalysis,'matrix2d'> & {matrix2d:(number|string)[]}}
function decodeMatrixPlan(value:NativeMatrixWire):NativeMatrixPlan {
  const number=(value:number|string)=>typeof value==='number'?value:Number(value)
  return {matrix:freezeMatrix4(value.matrix.map(number)),analysis:{...value.analysis,matrix2d:freezeMatrix3(value.analysis.matrix2d.map(number))}}
}
export function resolveOpenScadViewerAxisAngle(axis: readonly number[], degrees: number): OpenScadMatrix4 | null {
  const encode = (value: number) => Number.isFinite(value) ? value : String(value)
  const response = languageRequest(22, { viewer: true, parameters: [degrees, axis[0] ?? 0, axis[1] ?? 0, axis[2] ?? 0].map(encode) }) as { ok: boolean; value: NativeMatrixWire | null; error?: { message: string } }
  if (!response.ok) throw new Error(response.error?.message ?? 'Native viewer rotation failed')
  return response.value === null ? null : decodeMatrixPlan(response.value).matrix
}

function finish(
  kind: OpenScadStableTransformName,
  input: NativeMatrixPlan,
  parameterValid: boolean,
  context: OpenScadStableTransformContext,
): OpenScadStableTransformPlan {
  const {matrix,analysis} = input
  if (analysis.dropsChildren) warn(context, {
    code: 'OPENSCAD_TRANSFORM_NONFINITE_EMPTY',
    message: `${kind} produced a non-finite matrix, so its child geometry is removed`,
    transform: kind,
  })
  return Object.freeze({
    kind,
    matrix,
    parameterValid,
    ...analysis,
    reduced: false,
    reduction: null,
  })
}

function vectorTransform(kind:'translate'|'scale'|'mirror',value:unknown):{plan:NativeMatrixPlan;valid:boolean;rangeWarning:boolean} {
  const encode=(value:number)=>Number.isFinite(value)?value:String(value)
  const vector=Array.isArray(value)&&value.length<=3?Array.from(value,item=>typeof item==='number'?encode(item):null):null
  const scalar=typeof value==='number'?encode(value):null
  const response=languageRequest(26,{kind,vector,scalar}) as {ok:boolean;value:{plan:NativeMatrixWire;valid:boolean;rangeWarning:boolean};error?:{message:string}}
  if(!response.ok)throw new Error(response.error?.message??'Native vector transform failed')
  return {...response.value,plan:decodeMatrixPlan(response.value.plan)}
}

export function resolveOpenScadTranslate(
  value: unknown,
  context: OpenScadStableTransformContext = SILENT_CONTEXT,
): OpenScadStableTransformPlan {
  const {plan,valid} = vectorTransform('translate',value)
  if (!valid) warn(context, {
    code: 'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
    message: 'translate uses identity because v is not a finite exact vec2 or vec3',
    transform: 'translate',
    field: 'v',
    value,
  })
  return finish('translate',plan,valid,context)
}

export function resolveOpenScadScale(
  value: unknown,
  context: OpenScadStableTransformContext = SILENT_CONTEXT,
): OpenScadStableTransformPlan {
  const {plan,valid,rangeWarning} = vectorTransform('scale',value)
  if (!valid) {
    warn(context, {
      code: 'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
      message: 'scale uses identity because v is not a scalar or exact vec2/vec3',
      transform: 'scale',
      field: 'v',
      value,
    })
  }
  if (context.checkParameterRanges && rangeWarning) {
    warn(context, {
      code: 'OPENSCAD_TRANSFORM_PARAMETER_RANGE',
      message: 'scale contains a zero or non-finite component',
      transform: 'scale',
      field: 'v',
      value,
    })
  }
  return finish('scale',plan,valid,context)
}


export function resolveOpenScadMirror(
  value: unknown,
  context: OpenScadStableTransformContext = SILENT_CONTEXT,
): OpenScadStableTransformPlan {
  const {plan,valid} = vectorTransform('mirror',value)
  if (!valid) warn(context, {
    code: 'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
    message: 'mirror uses its x-normal default because v is not an exact vec2 or vec3',
    transform: 'mirror',
    field: 'v',
    value,
  })
  return finish('mirror',plan,valid,context)
}



function vectorEulerAngles(value: readonly unknown[]): {
  readonly matrix: NativeMatrixPlan
  readonly valid: boolean
} {
  const components = Array.from(value.slice(0,3),item=>typeof item==='number'?Number.isFinite(item)?item:String(item):null)
  const response = languageRequest(24,{components,length:value.length}) as {ok:boolean;value:{plan:NativeMatrixWire;valid:boolean};error?:{message:string}}
  if (!response.ok) throw new Error(response.error?.message ?? 'Native Euler argument conversion failed')
  return Object.freeze({matrix:decodeMatrixPlan(response.value.plan),valid:response.value.valid})
}

export function resolveOpenScadRotate(
  input: OpenScadRotateInput = {},
  context: OpenScadStableTransformContext = SILENT_CONTEXT,
): OpenScadStableTransformPlan {
  if (Array.isArray(input.a)) {
    const angles = vectorEulerAngles(input.a)
    if (angles.valid && input.v !== undefined) warn(context, {
      code: 'OPENSCAD_ROTATE_AXIS_IGNORED',
      message: 'rotate ignores v when a is a vector',
      transform: 'rotate',
      field: 'v',
      value: input.v,
    })
    if (!angles.valid) warn(context, {
      code: 'OPENSCAD_ROTATE_PARTIAL_VECTOR',
      message: 'rotate retained its component-wise fallback matrix after a vector conversion problem',
      transform: 'rotate',
      field: 'a',
      value: input.a,
    })
    return finish('rotate', angles.matrix, angles.valid, context)
  }

  const encode=(value:number)=>Number.isFinite(value)?value:String(value)
  const axis=Array.isArray(input.v)&&input.v.length<=3?Array.from(input.v,value=>typeof value==='number'?encode(value):null):null
  const response=languageRequest(27,{angle:typeof input.a==='number'?encode(input.a):null,axis,axisProvided:input.v!==undefined}) as {ok:boolean;value:{plan:NativeMatrixWire;valid:boolean;angleValid:boolean};error?:{message:string}}
  if(!response.ok)throw new Error(response.error?.message??'Native scalar rotation failed')
  const {valid,angleValid}=response.value
  if (!valid) warn(context, {
    code: 'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
    message: 'rotate replaced an invalid scalar angle or axis with its neutral/default value',
    transform: 'rotate',
    field: !angleValid ? 'a' : 'v',
    value: !angleValid ? input.a : input.v,
  })
  return finish('rotate',decodeMatrixPlan(response.value.plan),valid,context)
}

export function resolveOpenScadMultmatrix(
  value: unknown,
  context: OpenScadStableTransformContext = SILENT_CONTEXT,
): OpenScadStableTransformPlan {
  const rows = Array.isArray(value) ? Array.from(value.slice(0,4),row=>Array.isArray(row)
    ? Array.from(row.slice(0,4),cell=>typeof cell==='number'?Number.isFinite(cell)?cell:String(cell):null) : []) : null
  const response = languageRequest(25,{rows}) as {ok:boolean;value:{plan:NativeMatrixWire;valid:boolean};error?:{message:string}}
  if (!response.ok) throw new Error(response.error?.message ?? 'Native authored matrix failed')
  const matrix = decodeMatrixPlan(response.value.plan)
  return finish('multmatrix',matrix,response.value.valid,context)
}
