import { SILENT_CONTEXT } from '../openScadStableGeometrySemantics'
import type { OpenScadResizeResolution, OpenScadResizeResolutionInput, OpenScadRotateExtrudeResolution, OpenScadRotateExtrudeResolutionInput, OpenScadStableGeometryContext } from '../openScadStableGeometrySemantics'
import { resolveOpenScadSweepFragments } from '../openScadStableModuleSemantics'
import type { OpenScadStableModuleSemanticsContext } from '../openScadStableModuleSemantics'
import { autoAxes, finiteNumberOr } from './resolveOpenScadLinearExtrude'

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
