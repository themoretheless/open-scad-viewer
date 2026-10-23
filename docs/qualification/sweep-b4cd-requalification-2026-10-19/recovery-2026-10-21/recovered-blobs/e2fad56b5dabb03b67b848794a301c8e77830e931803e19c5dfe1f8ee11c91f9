/**
 * Kernel-neutral OpenSCAD 2021.01 geometry-module rules.
 *
 * This file intentionally has no parser or Manifold dependency.  It resolves
 * language parameters and describes the small amount of kernel adaptation
 * needed by the independent executor (notably OpenSCAD slices versus
 * Manifold's interior-division count).
 */

import { languageRequest } from './languages/kernel'
import type { RangeValue } from './openscadCompiler'
import {
  OPENSCAD_2021_GEOMETRY_EPSILON,
  resolveOpenScadSweepFragments,
  type OpenScadStableModuleSemanticsContext,
} from './openScadStableModuleSemantics'

export type OpenScadPoint2 = readonly [number, number]
export type OpenScadScale2 = readonly [number, number]

export type OpenScadStableGeometryWarningCode =
  | 'OPENSCAD_LINEAR_EXTRUDE_HEIGHT_DEFAULTED'
  | 'OPENSCAD_LINEAR_EXTRUDE_SCALE_DEFAULTED'
  | 'OPENSCAD_LINEAR_EXTRUDE_SLICES_CLAMPED'
  | 'OPENSCAD_CHILDREN_BAD_SELECTION'
  | 'OPENSCAD_CHILDREN_BAD_INDEX'
  | 'OPENSCAD_CHILDREN_INDEX_OUT_OF_BOUNDS'
  | 'OPENSCAD_CHILDREN_RANGE_LIMIT'
  | 'OPENSCAD_RESIZE_BAD_NEWSIZE'
  | 'OPENSCAD_RESIZE_ZERO_EXTENT'
  | 'OPENSCAD_ROTATE_EXTRUDE_ANGLE_DEFAULTED'
  | 'OPENSCAD_ROTATE_EXTRUDE_FRAGMENT_PARAMETER'
  | 'OPENSCAD_ROTATE_EXTRUDE_FRAGMENTS_CLAMPED'

export interface OpenScadStableGeometryWarning {
  readonly code: OpenScadStableGeometryWarningCode
  readonly message: string
  readonly value?: unknown
  readonly limit?: number
}

export interface OpenScadStableGeometryContext {
  warn(warning: OpenScadStableGeometryWarning): void
}

const SILENT_CONTEXT: OpenScadStableGeometryContext = Object.freeze({ warn() {} })

export const OPENSCAD_LINEAR_EXTRUDE_DEFAULT_HEIGHT = 100
export const OPENSCAD_LINEAR_EXTRUDE_MAX_SLICES = 512

export interface OpenScadLinearExtrudeResolutionInput {
  readonly height?: unknown
  readonly scale?: unknown
  readonly center?: unknown
  readonly twist?: unknown
  readonly slices?: unknown
  readonly fn?: unknown
  readonly fa?: unknown
  readonly fs?: unknown
  /** Vertices after 2D child union, used by OpenSCAD's automatic slice rule. */
  readonly profilePoints?: readonly OpenScadPoint2[]
  /** Independent-engine safety bound; not an upstream language limit. */
  readonly maximumSlices?: number
}

export type OpenScadLinearExtrudeSliceSource =
  | 'explicit'
  | 'twist-$fn'
  | 'twist-$fa/$fs'
  | 'twist-minimum'
  | 'nonuniform-$fn'
  | 'nonuniform-$fs'
  | 'single'
  | 'empty'

export interface OpenScadLinearExtrudeResolution {
  readonly empty: boolean
  readonly height: number
  readonly scale: OpenScadScale2
  readonly center: boolean
  readonly twist: number
  /** OpenSCAD vertical intervals. */
  readonly slices: number
  readonly unboundedSlices: number
  readonly sliceSource: OpenScadLinearExtrudeSliceSource
  /** Manifold counts only extra interior copies, hence slices - 1. */
  readonly manifoldNDivisions: number
  readonly maximumSlices: number
  readonly reduced: boolean
}

/** Resolve stable linear_extrude parameters and the Manifold adapter count. */
export function resolveOpenScadLinearExtrude(
  input: OpenScadLinearExtrudeResolutionInput,
  context: OpenScadStableGeometryContext = SILENT_CONTEXT,
): OpenScadLinearExtrudeResolution {
  const maximumSlices = input.maximumSlices ?? OPENSCAD_LINEAR_EXTRUDE_MAX_SLICES
  if (!Number.isSafeInteger(maximumSlices) || maximumSlices < 1) {
    throw new RangeError('OpenSCAD linear_extrude slice maximum must be a positive safe integer')
  }

  const numeric=(value:unknown)=>typeof value==='number' ? Number.isFinite(value)?value:String(value) : null
  const scaleValue=Array.isArray(input.scale) && input.scale.length===2 ? input.scale.map(numeric) : numeric(input.scale)
  const response=languageRequest(31,{height:numeric(input.height),heightProvided:input.height!==undefined,scale:scaleValue,scaleMissing:input.scale===undefined,twist:numeric(input.twist),center:input.center===true,slices:numeric(input.slices),resolveSlices:true,maximumSlices,points:input.profilePoints??[],fn:numeric(input.fn),fa:numeric(input.fa),fs:numeric(input.fs)}) as {ok:boolean;value:{height:number;scale:[number,number];twist:number;center:boolean;slices:number;unboundedSlices:number;sliceSource:OpenScadLinearExtrudeSliceSource;manifoldNDivisions:number;reduced:boolean;heightDefaulted:boolean;scaleDefaulted:boolean};error?:{message:string}}
  if(!response.ok)throw new Error(response.error?.message??'Native extrusion parameter normalization failed')
  const {height,twist,center}=response.value
  const scale=Object.freeze(response.value.scale)
  if(response.value.heightDefaulted)context.warn({code:'OPENSCAD_LINEAR_EXTRUDE_HEIGHT_DEFAULTED',message:'Invalid linear_extrude height was replaced with 100',value:input.height})
  if(response.value.scaleDefaulted)context.warn({code:'OPENSCAD_LINEAR_EXTRUDE_SCALE_DEFAULTED',message:'Invalid linear_extrude scale was replaced with [1, 1]',value:input.scale})

  const {slices,unboundedSlices,sliceSource,manifoldNDivisions,reduced}=response.value
  if (reduced) context.warn({
    code: 'OPENSCAD_LINEAR_EXTRUDE_SLICES_CLAMPED',
    message: `linear_extrude slices were clamped to the engine limit ${maximumSlices}`,
    value: unboundedSlices,
    limit: maximumSlices,
  })
  return Object.freeze({
    empty: height === 0,
    height,
    scale,
    center,
    twist,
    slices,
    unboundedSlices,
    sliceSource,
    manifoldNDivisions,
    maximumSlices,
    reduced,
  })
}

export type OpenScadAffine2dMatrix = readonly [
  number, number, number,
  number, number, number,
  number, number, number,
]

/**
 * Convert OpenSCAD's row-major 4x4 value into a column-major affine 2D matrix.
 * Missing/non-number entries retain their identity defaults, matching the
 * language's permissive matrix fill. The homogeneous w normalizes the matrix.
 */
export function resolveOpenScad2dMultmatrix(value: unknown): OpenScadAffine2dMatrix | null {
  const rows = Array.isArray(value)
    ? value.slice(0, 4).map(row => Array.isArray(row)
      ? row.slice(0, 4).map(cell => typeof cell === 'number' && Number.isFinite(cell) ? cell : null)
      : [])
    : null
  const response = languageRequest(25, { rows, legacy2d: true }) as {
    ok: boolean; value: (number | string)[] | null; error?: { message: string }
  }
  if (!response.ok) throw new Error(response.error?.message ?? 'Native XY matrix normalization failed')
  return response.value === null ? null
    : Object.freeze(response.value.map(Number)) as unknown as OpenScadAffine2dMatrix
}

function isRangeValue(value: unknown): value is RangeValue {
  return !Array.isArray(value) && typeof value === 'object' && value !== null
    && (value as { kind?: unknown }).kind === 'range-value'
    && typeof (value as { start?: unknown }).start === 'number'
    && typeof (value as { step?: unknown }).step === 'number'
    && typeof (value as { end?: unknown }).end === 'number'
}

export interface OpenScadChildrenSelectionInput {
  /** Distinguishes children() from children(undef). */
  readonly provided: boolean
  readonly value?: unknown
  readonly childCount: number
  readonly maximumRangeItems?: number
}

function materializeRangeBounded(
  range: RangeValue,
  maximum: number,
  context: OpenScadStableGeometryContext,
): number[] {
  const encode = (value: number) => Number.isFinite(value) ? value : String(value)
  const response = languageRequest(34, { range: [range.start, range.step, range.end].map(encode), maximum }) as {
    ok: boolean; value: { candidates: (number | string)[]; issue: 'invalid' | 'limit' | null }; error?: { message: string }
  }
  if (!response.ok) throw new Error(response.error?.message ?? 'Native children range expansion failed')
  if (response.value.issue === 'invalid') context.warn({
    code: 'OPENSCAD_CHILDREN_BAD_SELECTION', message: 'Invalid children range was ignored', value: range,
  })
  if (response.value.issue === 'limit') context.warn({
    code: 'OPENSCAD_CHILDREN_RANGE_LIMIT', message: `children range exceeds the engine limit ${maximum}`, limit: maximum,
  })
  return response.value.candidates.map(Number)
}

/** Resolve children() scalar/vector/range selection, preserving order and duplicates. */
export function resolveOpenScadChildrenSelection(
  input: OpenScadChildrenSelectionInput,
  context: OpenScadStableGeometryContext = SILENT_CONTEXT,
): readonly number[] {
  const { childCount } = input
  if (!Number.isSafeInteger(childCount) || childCount < 0) {
    throw new RangeError('OpenSCAD child count must be a non-negative safe integer')
  }
  const maximumRangeItems = input.maximumRangeItems ?? 10_000
  if (!Number.isSafeInteger(maximumRangeItems) || maximumRangeItems < 1) {
    throw new RangeError('OpenSCAD children range maximum must be a positive safe integer')
  }
  if (!input.provided) return Object.freeze(Array.from({ length: childCount }, (_, index) => index))

  let candidates: readonly unknown[]
  if (typeof input.value === 'number') candidates = [input.value]
  else if (Array.isArray(input.value)) candidates = input.value
  else if (isRangeValue(input.value)) {
    candidates = materializeRangeBounded(input.value, maximumRangeItems, context)
  } else {
    context.warn({
      code: 'OPENSCAD_CHILDREN_BAD_SELECTION',
      message: 'children accepts an empty argument list, number, vector, or range',
      value: input.value,
    })
    return Object.freeze([])
  }

  const response = languageRequest(34, { childCount, candidates: candidates.map(value =>
    typeof value === 'number' && Number.isFinite(value) ? value : null) }) as {
    ok: boolean; value: { indices: number[]; issues: ({ kind: 'invalid'; candidate: number } | { kind: 'out-of-bounds'; index: number })[] }; error?: { message: string }
  }
  if (!response.ok) throw new Error(response.error?.message ?? 'Native child index selection failed')
  for (const issue of response.value.issues) {
    if (issue.kind === 'invalid') context.warn({
      code: 'OPENSCAD_CHILDREN_BAD_INDEX', message: 'Non-numeric children index was ignored', value: candidates[issue.candidate],
    })
    else context.warn({
      code: 'OPENSCAD_CHILDREN_INDEX_OUT_OF_BOUNDS', message: `Children index ${issue.index} is outside 0..${Math.max(0, childCount - 1)}`, value: issue.index,
    })
  }
  return Object.freeze(response.value.indices)
}

export interface OpenScadResizeResolutionInput {
  readonly dimension: 2 | 3
  readonly extents?: readonly number[]
  readonly bounds?: readonly { readonly min: readonly number[]; readonly max: readonly number[] }[]
  readonly newsize: unknown
  readonly auto?: unknown
}

export interface OpenScadResizeResolution {
  readonly scales: readonly number[]
  readonly applied: boolean
  readonly valid: boolean
}

function autoAxes(value: unknown, dimension: 2 | 3): readonly boolean[] {
  if (Array.isArray(value)) {
    return Array.from({ length: dimension }, (_, axis) => value[axis] === true)
  }
  return Array.from({ length: dimension }, () => value === true)
}

/** Resolve resize's zero-target and automatic-aspect axes from aggregate bounds. */
export function resolveOpenScadResize(
  input: OpenScadResizeResolutionInput,
  context: OpenScadStableGeometryContext = SILENT_CONTEXT,
): OpenScadResizeResolution {
  const identity = Object.freeze(Array.from({ length: input.dimension }, () => 1))
  if (!Array.isArray(input.newsize)) {
    context.warn({
      code: 'OPENSCAD_RESIZE_BAD_NEWSIZE',
      message: 'resize newsize must be a vector',
      value: input.newsize,
    })
    return Object.freeze({ scales: identity, applied: false, valid: false })
  }
  const automatic = autoAxes(input.auto, input.dimension)
  const positive = (value: unknown): number | null => typeof value === 'number' && Number.isFinite(value) && value > 0 ? value : null
  const encode = (value: number) => Number.isFinite(value) ? value : String(value)
  const response = languageRequest(13, {
    targets: Array.from({length: input.dimension}, (_, axis) => positive((input.newsize as unknown[])[axis])),
    ...(input.bounds ? { bounds: input.bounds.map(bound => ({
      min: Array.from({length: input.dimension}, (_, axis) => encode(bound.min[axis] ?? Infinity)),
      max: Array.from({length: input.dimension}, (_, axis) => encode(bound.max[axis] ?? -Infinity)),
    })) } : { extents: Array.from({length: input.dimension}, (_, axis) => encode(input.extents?.[axis] ?? NaN)) }),
    automatic,
  }) as {ok: boolean; value: {scales: (number | 'Infinity')[]; invalidAxis: number | null; extents: (number | string)[]}; error?: {message: string}}
  if (!response.ok) throw new Error(response.error?.message ?? 'OpenSCAD resize calculation failed')
  const axis = response.value.invalidAxis
  if (axis !== null) {
    context.warn({
      code: 'OPENSCAD_RESIZE_ZERO_EXTENT',
      message: `resize cannot map a zero-width axis ${axis} to a positive target`,
      value: Number(response.value.extents[axis]),
    })
    return Object.freeze({scales: identity, applied: false, valid: false})
  }
  const scales = response.value.scales.map(Number)
  return Object.freeze({
    scales: Object.freeze(scales),
    applied: scales.some(scale => scale !== 1),
    valid: true,
  })
}

export interface OpenScadRotateExtrudeResolutionInput {
  readonly angle?: unknown
  readonly profileXMin: number
  readonly profileXMax: number
  readonly fn?: unknown
  readonly fa?: unknown
  readonly fs?: unknown
  readonly quality?: 'preview' | 'full'
  readonly maximumFragments?: number
}

export interface OpenScadRotateExtrudeResolution {
  readonly empty: boolean
  readonly crossesAxis: boolean
  readonly angle: number
  readonly profileSide: 'positive' | 'negative' | 'axis' | 'crossing'
  readonly reflectProfileX: boolean
  readonly postRotateDegrees: 0 | 180
  readonly circularSegments: number
  readonly reduced: boolean
}

/** Resolve 2021 angle normalization and the negative-X profile adaptation. */
export function resolveOpenScadRotateExtrude(
  input: OpenScadRotateExtrudeResolutionInput,
  context: OpenScadStableGeometryContext = SILENT_CONTEXT,
): OpenScadRotateExtrudeResolution {
  const authoredAngle = typeof input.angle === 'number'
    ? Number.isFinite(input.angle) ? input.angle : String(input.angle)
    : null
  const response = languageRequest(32, {
    angle: authoredAngle,
    angleProvided: input.angle !== undefined,
    minimum: Number.isFinite(input.profileXMin) ? input.profileXMin : String(input.profileXMin),
    maximum: Number.isFinite(input.profileXMax) ? input.profileXMax : String(input.profileXMax),
  }) as { ok: boolean; value: { angle: number; angleDefaulted: boolean; profileSide: OpenScadRotateExtrudeResolution['profileSide']; empty: boolean; reflectProfileX: boolean; radius: number }; error?: { message: string } }
  if (!response.ok) throw new Error(response.error?.message ?? 'Native revolution parameter normalization failed')
  const { angle, profileSide, empty } = response.value
  if (response.value.angleDefaulted) context.warn({
    code: 'OPENSCAD_ROTATE_EXTRUDE_ANGLE_DEFAULTED',
    message: 'Invalid rotate_extrude angle was replaced with 360',
    value: input.angle,
  })
  const crossesAxis = profileSide === 'crossing'
  let circularSegments = 0
  let reduced = false
  if (!empty) {
    const moduleContext: OpenScadStableModuleSemanticsContext = {
      warn: warning => context.warn({
        code: warning.code === 'OPENSCAD_FRAGMENTS_CLAMPED'
          ? 'OPENSCAD_ROTATE_EXTRUDE_FRAGMENTS_CLAMPED'
          : 'OPENSCAD_ROTATE_EXTRUDE_FRAGMENT_PARAMETER',
        message: warning.message,
        value: warning.value,
        limit: warning.limit,
      }),
    }
    const fragments = resolveOpenScadSweepFragments({
      radius: response.value.radius,
      sweepDegrees: angle,
      ...(input.fn === undefined ? {} : { fn: input.fn }),
      ...(input.fa === undefined ? {} : { fa: input.fa }),
      ...(input.fs === undefined ? {} : { fs: input.fs }),
      ...(input.quality === undefined ? {} : { quality: input.quality }),
      ...(input.maximumFragments === undefined ? {} : { maximum: input.maximumFragments }),
    }, moduleContext)
    circularSegments = fragments.sweepFragments
    reduced = fragments.reduced
  }
  const negative = response.value.reflectProfileX
  return Object.freeze({
    empty,
    crossesAxis,
    angle,
    profileSide,
    reflectProfileX: negative,
    postRotateDegrees: negative ? 180 : 0,
    circularSegments,
    reduced,
  })
}
