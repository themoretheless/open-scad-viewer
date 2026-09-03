/**
 * Kernel-neutral OpenSCAD 2021.01 geometry-module rules.
 *
 * This file intentionally has no parser or Manifold dependency.  It resolves
 * language parameters and describes the small amount of kernel adaptation
 * needed by the independent executor (notably OpenSCAD slices versus
 * Manifold's interior-division count).
 */

import type { RangeValue } from './openscadCompiler'
import {
  OPENSCAD_2021_GEOMETRY_EPSILON,
  OPENSCAD_DEFAULT_FA,
  OPENSCAD_DEFAULT_FN,
  OPENSCAD_DEFAULT_FS,
  OPENSCAD_MIN_FA,
  OPENSCAD_MIN_FS,
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

function finiteNumberOr(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

function resolveScale2(
  value: unknown,
  context: OpenScadStableGeometryContext,
): OpenScadScale2 {
  let x = 1
  let y = 1
  let valid = value === undefined
  if (typeof value === 'number' && Number.isFinite(value)) {
    x = value
    y = value
    valid = true
  } else if (Array.isArray(value) && value.length === 2) {
    const first = value[0]
    const second = value[1]
    if (typeof first === 'number' && Number.isFinite(first)
      && typeof second === 'number' && Number.isFinite(second)) {
      x = first
      y = second
      valid = true
    }
  }
  if (!valid) context.warn({
    code: 'OPENSCAD_LINEAR_EXTRUDE_SCALE_DEFAULTED',
    message: 'Invalid linear_extrude scale was replaced with [1, 1]',
    value,
  })
  if (!valid) return Object.freeze([1, 1])
  return Object.freeze([Math.max(0, x), Math.max(0, y)])
}

function fragmentSpecial(value: unknown, fallback: number, minimum?: number): number {
  const resolved = typeof value === 'number' ? value : fallback
  if (!Number.isFinite(resolved)) return resolved
  return minimum === undefined ? resolved : Math.max(resolved, minimum)
}

function farthestRadiusSquared(points: readonly OpenScadPoint2[]): number {
  let maximum = 0
  for (const [x, y] of points) maximum = Math.max(maximum, x * x + y * y)
  return maximum
}

function farthestScaleDeltaSquared(
  points: readonly OpenScadPoint2[],
  scale: OpenScadScale2,
): number {
  let maximum = 0
  for (const [x, y] of points) {
    const dx = x * (1 - scale[0])
    const dy = y * (1 - scale[1])
    maximum = Math.max(maximum, dx * dx + dy * dy)
  }
  return maximum
}

function spiralPlanarLength(radius: number, radians: number, scale: number): number {
  const radialSlope = radius * (scale - 1) / radians
  const primitive = (radial: number): number => {
    const slopeMagnitude = Math.abs(radialSlope)
    return 0.5 * (
      radial * Math.hypot(radial, radialSlope)
      + radialSlope * radialSlope * Math.asinh(radial / slopeMagnitude)
    )
  }
  return Math.abs((primitive(radius * scale) - primitive(radius)) / radialSlope)
}

function automaticTwistSlices(
  points: readonly OpenScadPoint2[],
  height: number,
  twist: number,
  scale: OpenScadScale2,
  fn: number,
  fa: number,
  fs: number,
): { slices: number; source: OpenScadLinearExtrudeSliceSource } {
  const absoluteTwist = Math.abs(twist)
  const minimum = Math.max(1, Math.ceil(absoluteTwist / 120))
  const radius = Math.sqrt(farthestRadiusSquared(points))
  if (radius < OPENSCAD_2021_GEOMETRY_EPSILON || !Number.isFinite(fn)) {
    return { slices: minimum, source: 'twist-minimum' }
  }
  if (fn > 0) {
    return {
      slices: Math.max(minimum, Math.ceil(absoluteTwist * fn / 360)),
      source: 'twist-$fn',
    }
  }

  const radians = absoluteTwist * Math.PI / 180
  const planarLength = scale[0] === scale[1] && scale[0] !== 1
    ? spiralPlanarLength(radius, radians, scale[0])
    : radius * radians
  const pathLength = Math.hypot(planarLength, height)
  const angularSlices = Math.ceil(absoluteTwist / fa)
  const lengthSlices = Math.ceil(pathLength / fs)
  return {
    slices: Math.max(minimum, Math.min(angularSlices, lengthSlices)),
    source: 'twist-$fa/$fs',
  }
}

function automaticNonuniformSlices(
  points: readonly OpenScadPoint2[],
  height: number,
  scale: OpenScadScale2,
  fn: number,
  fs: number,
): { slices: number; source: OpenScadLinearExtrudeSliceSource } {
  const displacement = Math.sqrt(farthestScaleDeltaSquared(points, scale))
  if (displacement < OPENSCAD_2021_GEOMETRY_EPSILON || !Number.isFinite(fn)) {
    return { slices: 1, source: 'single' }
  }
  if (fn > 0) {
    return { slices: Math.max(1, Math.trunc(fn)), source: 'nonuniform-$fn' }
  }
  return {
    slices: Math.max(1, Math.ceil(Math.hypot(displacement, height) / fs)),
    source: 'nonuniform-$fs',
  }
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

  let height = finiteNumberOr(input.height, OPENSCAD_LINEAR_EXTRUDE_DEFAULT_HEIGHT)
  if (input.height !== undefined && height === OPENSCAD_LINEAR_EXTRUDE_DEFAULT_HEIGHT
    && input.height !== OPENSCAD_LINEAR_EXTRUDE_DEFAULT_HEIGHT) {
    context.warn({
      code: 'OPENSCAD_LINEAR_EXTRUDE_HEIGHT_DEFAULTED',
      message: 'Invalid linear_extrude height was replaced with 100',
      value: input.height,
    })
  }
  height = Math.max(0, height)
  const scale = resolveScale2(input.scale, context)
  const twist = finiteNumberOr(input.twist, 0)
  const center = input.center === true

  if (height === 0) return Object.freeze({
    empty: true,
    height,
    scale,
    center,
    twist,
    slices: 0,
    unboundedSlices: 0,
    sliceSource: 'empty',
    manifoldNDivisions: 0,
    maximumSlices,
    reduced: false,
  })

  const authoredSlices = typeof input.slices === 'number' && Number.isFinite(input.slices)
    ? Math.trunc(input.slices)
    : 0
  let unboundedSlices: number
  let sliceSource: OpenScadLinearExtrudeSliceSource
  if (authoredSlices > 0) {
    unboundedSlices = authoredSlices
    sliceSource = 'explicit'
  } else {
    const points = input.profilePoints ?? []
    const fn = fragmentSpecial(input.fn, OPENSCAD_DEFAULT_FN)
    const fa = fragmentSpecial(input.fa, OPENSCAD_DEFAULT_FA, OPENSCAD_MIN_FA)
    const fs = fragmentSpecial(input.fs, OPENSCAD_DEFAULT_FS, OPENSCAD_MIN_FS)
    const automatic = twist !== 0
      ? automaticTwistSlices(points, height, twist, scale, fn, fa, fs)
      : scale[0] !== scale[1]
        ? automaticNonuniformSlices(points, height, scale, fn, fs)
        : { slices: 1, source: 'single' as const }
    unboundedSlices = automatic.slices
    sliceSource = automatic.source
  }

  if (!Number.isFinite(unboundedSlices) || unboundedSlices < 1) unboundedSlices = 1
  const slices = Math.min(unboundedSlices, maximumSlices)
  const reduced = slices !== unboundedSlices
  if (reduced) context.warn({
    code: 'OPENSCAD_LINEAR_EXTRUDE_SLICES_CLAMPED',
    message: `linear_extrude slices were clamped to the engine limit ${maximumSlices}`,
    value: unboundedSlices,
    limit: maximumSlices,
  })
  return Object.freeze({
    empty: false,
    height,
    scale,
    center,
    twist,
    slices,
    unboundedSlices,
    sliceSource,
    manifoldNDivisions: Math.max(0, slices - 1),
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
  if (!Array.isArray(value)) return null
  const matrix = [
    [1, 0, 0, 0],
    [0, 1, 0, 0],
    [0, 0, 1, 0],
    [0, 0, 0, 1],
  ]
  for (let row = 0; row < Math.min(4, value.length); row++) {
    const authoredRow = value[row]
    if (!Array.isArray(authoredRow)) continue
    for (let column = 0; column < Math.min(4, authoredRow.length); column++) {
      const cell = authoredRow[column]
      if (typeof cell === 'number' && Number.isFinite(cell)) matrix[row][column] = cell
    }
  }
  const w = matrix[3][3]
  if (!Number.isFinite(w) || w === 0) return null
  const normalized = (row: number, column: number) => matrix[row][column] / w
  return Object.freeze([
    normalized(0, 0), normalized(1, 0), 0,
    normalized(0, 1), normalized(1, 1), 0,
    normalized(0, 3), normalized(1, 3), 1,
  ])
}

export interface OpenScadMinkowski2dKernel<Section, Solid> {
  /** Must return a point contained in the section (a contour vertex is sufficient). */
  readonly anchor: (section: Section) => OpenScadPoint2
  readonly translate: (section: Section, offset: OpenScadPoint2) => Section
  readonly extrudeUnitPrism: (section: Section) => Solid
  readonly minkowskiSum: (left: Solid, right: Solid) => Solid
  readonly projectTo2d: (solid: Solid) => Section
}

/**
 * Exact product lift for planar Minkowski sums:
 * (A x [0,1]) + (B x [0,1]) = (A + B) x [0,2]. Projecting the resulting
 * product back to XY therefore yields exactly A + B. Each section is first
 * translated by a contained anchor because Manifold's operation is exposed as
 * anchored dilation; summing and restoring those anchors preserves the same
 * set. The identity cases avoid unnecessary kernel work and preserve metadata.
 */
export function openScadMinkowski2dViaProduct<Section, Solid>(
  sections: readonly Section[],
  kernel: OpenScadMinkowski2dKernel<Section, Solid>,
): Section | undefined {
  if (sections.length === 0) return undefined
  if (sections.length === 1) return sections[0]
  const anchors = sections.map(section => kernel.anchor(section))
  const normalized = sections.map((section, index) => kernel.translate(
    section,
    [-anchors[index][0], -anchors[index][1]],
  ))
  let product = kernel.extrudeUnitPrism(normalized[0])
  for (let index = 1; index < sections.length; index++) {
    product = kernel.minkowskiSum(product, kernel.extrudeUnitPrism(normalized[index]))
  }
  const anchorSum = anchors.reduce<OpenScadPoint2>(
    (sum, anchor) => [sum[0] + anchor[0], sum[1] + anchor[1]],
    [0, 0],
  )
  return kernel.translate(kernel.projectTo2d(product), anchorSum)
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
  if (range.step === 0 || ![range.start, range.step, range.end].every(Number.isFinite)) {
    context.warn({
      code: 'OPENSCAD_CHILDREN_BAD_SELECTION',
      message: 'Invalid children range was ignored',
      value: range,
    })
    return []
  }
  const output: number[] = []
  const forward = range.step > 0
  const epsilon = Math.max(1, Math.abs(range.start), Math.abs(range.end)) * 1e-12
  for (
    let item = range.start;
    forward ? item <= range.end + epsilon : item >= range.end - epsilon;
    item += range.step
  ) {
    if (output.length >= maximum) {
      context.warn({
        code: 'OPENSCAD_CHILDREN_RANGE_LIMIT',
        message: `children range exceeds the engine limit ${maximum}`,
        limit: maximum,
      })
      return []
    }
    output.push(item)
  }
  return output
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

  const selected: number[] = []
  for (const candidate of candidates) {
    if (typeof candidate !== 'number' || !Number.isFinite(candidate)) {
      context.warn({
        code: 'OPENSCAD_CHILDREN_BAD_INDEX',
        message: 'Non-numeric children index was ignored',
        value: candidate,
      })
      continue
    }
    // Math.trunc(-0.5) is negative zero in JavaScript; OpenSCAD's integer
    // child slot is ordinary zero and must compare/serialize as such.
    const truncated = Math.trunc(candidate)
    const index = Object.is(truncated, -0) ? 0 : truncated
    if (index < 0 || index >= childCount) {
      context.warn({
        code: 'OPENSCAD_CHILDREN_INDEX_OUT_OF_BOUNDS',
        message: `Children index ${index} is outside 0..${Math.max(0, childCount - 1)}`,
        value: index,
      })
      continue
    }
    selected.push(index)
  }
  return Object.freeze(selected)
}

export interface OpenScadResizeResolutionInput {
  readonly dimension: 2 | 3
  readonly extents: readonly number[]
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
  const direct: Array<number | undefined> = []
  for (let axis = 0; axis < input.dimension; axis++) {
    const target = input.newsize[axis]
    if (typeof target !== 'number' || !Number.isFinite(target) || target <= 0) {
      direct.push(undefined)
      continue
    }
    const extent = input.extents[axis]
    if (typeof extent !== 'number' || !Number.isFinite(extent) || extent <= 0) {
      context.warn({
        code: 'OPENSCAD_RESIZE_ZERO_EXTENT',
        message: `resize cannot map a zero-width axis ${axis} to a positive target`,
        value: extent,
      })
      return Object.freeze({ scales: identity, applied: false, valid: false })
    }
    direct.push(target / extent)
  }
  const authored = direct.filter((scale): scale is number => scale !== undefined)
  const aspectScale = authored.length === 0 ? 1 : Math.max(...authored)
  const scales = direct.map((scale, axis) => scale ?? (automatic[axis] ? aspectScale : 1))
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
  let angle = finiteNumberOr(input.angle, 360)
  if (input.angle !== undefined && angle === 360 && input.angle !== 360) context.warn({
    code: 'OPENSCAD_ROTATE_EXTRUDE_ANGLE_DEFAULTED',
    message: 'Invalid rotate_extrude angle was replaced with 360',
    value: input.angle,
  })
  if (angle <= -360 || angle > 360) angle = 360

  const profileSide = input.profileXMin < 0 && input.profileXMax > 0
    ? 'crossing'
    : input.profileXMax <= 0 && input.profileXMin < 0
      ? 'negative'
      : input.profileXMin === 0 && input.profileXMax === 0
        ? 'axis'
        : 'positive'
  const crossesAxis = profileSide === 'crossing'
  const empty = angle === 0 || profileSide === 'axis' || crossesAxis
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
      radius: Math.max(Math.abs(input.profileXMin), Math.abs(input.profileXMax)),
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
  const negative = profileSide === 'negative'
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
