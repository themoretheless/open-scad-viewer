import { describe, expect, it } from 'vitest'
import {
  openScadTransform2dSubmatrix,
  resolveOpenScadMirror,
  resolveOpenScadMultmatrix,
  resolveOpenScadRotate,
  resolveOpenScadScale,
  resolveOpenScadTranslate,
  type OpenScadMatrix4,
  type OpenScadStableTransformContext,
  type OpenScadStableTransformWarning,
} from '../src/services/openScadStableTransformSemantics'

const identity: OpenScadMatrix4 = [
  1, 0, 0, 0,
  0, 1, 0, 0,
  0, 0, 1, 0,
  0, 0, 0, 1,
]

function harness(checkParameterRanges = false) {
  const warnings: OpenScadStableTransformWarning[] = []
  const context: OpenScadStableTransformContext = {
    warn: warning => warnings.push(warning),
    checkParameterRanges,
  }
  return { warnings, context }
}

describe('kernel-neutral OpenSCAD 2021 transform semantics', () => {
  it('accepts only finite exact vec2/vec3 translations and otherwise uses identity', () => {
    expect(resolveOpenScadTranslate([2, 3])).toMatchObject({
      matrix: [
        1, 0, 0, 0,
        0, 1, 0, 0,
        0, 0, 1, 0,
        2, 3, 0, 1,
      ],
      matrix2d: [1, 0, 0, 0, 1, 0, 2, 3, 1],
      parameterValid: true,
      dropsChildren: false,
    })

    const invalid = harness()
    expect(resolveOpenScadTranslate([2], invalid.context).matrix).toEqual(identity)
    expect(resolveOpenScadTranslate([2, 3, Number.POSITIVE_INFINITY], invalid.context).matrix).toEqual(identity)
    expect(resolveOpenScadTranslate(undefined, invalid.context).matrix).toEqual(identity)
    expect(invalid.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
      'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
      'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
    ])
  })

  it('uses z=1 for vec2 scale, identity for invalid vectors, and exposes singular scale', () => {
    expect(resolveOpenScadScale([2, 3])).toMatchObject({
      matrix: [
        2, 0, 0, 0,
        0, 3, 0, 0,
        0, 0, 1, 0,
        0, 0, 0, 1,
      ],
      matrix2d: [2, 0, 0, 0, 3, 0, 0, 0, 1],
      matrix3dSingular: false,
    })
    expect(resolveOpenScadScale(2).matrix).toEqual([
      2, 0, 0, 0,
      0, 2, 0, 0,
      0, 0, 2, 0,
      0, 0, 0, 1,
    ])

    const invalid = harness()
    expect(resolveOpenScadScale([2], invalid.context).matrix).toEqual(identity)
    expect(resolveOpenScadScale([2, 'bad', 4], invalid.context).matrix).toEqual([
      2, 0, 0, 0,
      0, 1, 0, 0,
      0, 0, 1, 0,
      0, 0, 0, 1,
    ])
    // The vec2 compatibility overload reports success even when its numeric
    // pair conversion leaves the initialized identity components in place.
    expect(resolveOpenScadScale([2, 'bad'], invalid.context)).toMatchObject({
      matrix: identity, parameterValid: true,
    })
    expect(invalid.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
      'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
    ])

    const zero = harness(true)
    expect(resolveOpenScadScale([0, 2, 3], zero.context)).toMatchObject({
      dropsChildren: false, matrix3dSingular: true, matrix2dSingular: true,
    })
    expect(zero.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_TRANSFORM_PARAMETER_RANGE',
    ])

    const infinite = harness(true)
    expect(resolveOpenScadScale(Number.POSITIVE_INFINITY, infinite.context)).toMatchObject({
      dropsChildren: true, matrix3dSingular: false,
    })
    expect(infinite.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_TRANSFORM_PARAMETER_RANGE',
      'OPENSCAD_TRANSFORM_NONFINITE_EMPTY',
    ])
  })

  it('defaults mirror to the x normal while a zero normal is identity', () => {
    const invalid = harness()
    expect(resolveOpenScadMirror('bad', invalid.context)).toMatchObject({
      matrix: [
        -1, 0, 0, 0,
        0, 1, 0, 0,
        0, 0, 1, 0,
        0, 0, 0, 1,
      ],
      parameterValid: false,
    })
    expect(invalid.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
    ])
    expect(resolveOpenScadMirror([0, 0, 0]).matrix).toEqual(identity)
    expect(resolveOpenScadMirror([2, 'bad', 3])).toMatchObject({
      matrix: [
        -1, 0, 0, 0,
        0, 1, 0, 0,
        0, 0, 1, 0,
        0, 0, 0, 1,
      ],
      parameterValid: false,
    })
    expect(resolveOpenScadMirror([0, 0, 1])).toMatchObject({
      matrix2d: [1, 0, 0, 0, 1, 0, 0, 0, 1],
      matrix2dSingular: false,
    })
  })

  it('uses the full 3D rotation submatrix for 2D children', () => {
    const eulerX = resolveOpenScadRotate({ a: [90, 0, 0] })
    expect(eulerX.matrix2d).toEqual([
      1, 0, 0,
      0, 0, 0,
      0, 0, 1,
    ])
    expect(eulerX).toMatchObject({ matrix3dSingular: false, matrix2dSingular: true })

    const axisX = resolveOpenScadRotate({ a: 90, v: [1, 0, 0] })
    expect(axisX.matrix2d).toEqual(eulerX.matrix2d)
    expect(resolveOpenScadRotate({ a: 90, v: [0, 0, 0] }).matrix).toEqual(identity)
    expect(resolveOpenScadRotate({ a: 90 }).matrix).toEqual([
      0, 1, 0, 0,
      -1, 0, 0, 0,
      0, 0, 1, 0,
      0, 0, 0, 1,
    ])
    expect(resolveOpenScadRotate({ a: 30, v: [2, 'bad', 3] }).parameterValid).toBe(false)
    expect(resolveOpenScadRotate({ a: 30, v: [2, 'bad'] }).matrix).toEqual(identity)
  })

  it('preserves vector-angle fallback behavior and reports ignored axes', () => {
    const ignored = harness()
    expect(resolveOpenScadRotate({ a: [0, 0, 90], v: [1, 0, 0] }, ignored.context))
      .toMatchObject({ parameterValid: true })
    expect(ignored.warnings.map(warning => warning.code)).toEqual(['OPENSCAD_ROTATE_AXIS_IGNORED'])

    const excess = harness()
    expect(resolveOpenScadRotate({ a: [10, 20, 30, 40] }, excess.context)).toMatchObject({
      parameterValid: false,
      dropsChildren: false,
    })
    expect(excess.warnings.map(warning => warning.code)).toEqual(['OPENSCAD_ROTATE_PARTIAL_VECTOR'])

    // The component walk is z, y, x and a non-number retains the most recently
    // converted component. Here y therefore reuses z=30 before x becomes 10.
    const partial = resolveOpenScadRotate({ a: [10, 'bad', 30] })
    const substituted = resolveOpenScadRotate({ a: [10, 30, 30] })
    expect(partial.matrix).toEqual(substituted.matrix)
    expect(partial.parameterValid).toBe(false)
  })

  it('fills partial multmatrix cells from identity and preserves projective 2D rows', () => {
    expect(resolveOpenScadMultmatrix([[2]])).toMatchObject({
      matrix: [
        2, 0, 0, 0,
        0, 1, 0, 0,
        0, 0, 1, 0,
        0, 0, 0, 1,
      ],
      parameterValid: true,
    })
    expect(resolveOpenScadMultmatrix([[2, 'bad'], 7])).toMatchObject({
      matrix: [
        2, 0, 0, 0,
        0, 1, 0, 0,
        0, 0, 1, 0,
        0, 0, 0, 1,
      ],
    })
    expect(resolveOpenScadMultmatrix('bad')).toMatchObject({
      matrix: identity, parameterValid: false, dropsChildren: false,
    })
    expect(resolveOpenScadMultmatrix([[1e-16]]).matrix[0]).toBe(1e-16)

    const projective = resolveOpenScadMultmatrix([
      [2, 3, 0, 4],
      [5, 6, 0, 7],
      [0, 0, 1, 0],
      [8, 9, 0, 1],
    ])
    expect(projective.matrix2d).toEqual([
      2, 5, 8,
      3, 6, 9,
      4, 7, 1,
    ])
    expect(projective).toMatchObject({ affine3d: false, affine2d: false })
    expect(openScadTransform2dSubmatrix(projective.matrix)).toEqual(projective.matrix2d)
  })

  it('normalizes homogeneous w and explicitly classifies non-finite removal', () => {
    expect(resolveOpenScadMultmatrix([
      [2, 0, 0, 4],
      [0, 4, 0, 8],
      [0, 0, 2, 0],
      [0, 0, 0, 2],
    ])).toMatchObject({
      matrix: [
        1, 0, 0, 0,
        0, 2, 0, 0,
        0, 0, 1, 0,
        2, 4, 0, 1,
      ],
      matrix2d: [1, 0, 0, 0, 2, 0, 2, 4, 1],
      dropsChildren: false,
    })

    const invalidW = harness()
    expect(resolveOpenScadMultmatrix([
      [1, 0, 0, 0],
      [0, 1, 0, 0],
      [0, 0, 1, 0],
      [0, 0, 0, 0],
    ], invalidW.context)).toMatchObject({ dropsChildren: true, reduced: false, reduction: null })
    expect(invalidW.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_TRANSFORM_NONFINITE_EMPTY',
    ])
  })
})
