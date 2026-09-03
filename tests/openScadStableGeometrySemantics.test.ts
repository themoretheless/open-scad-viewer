import { describe, expect, it } from 'vitest'
import {
  openScadMinkowski2dViaProduct,
  resolveOpenScad2dMultmatrix,
  resolveOpenScadChildrenSelection,
  resolveOpenScadLinearExtrude,
  resolveOpenScadResize,
  resolveOpenScadRotateExtrude,
  type OpenScadStableGeometryWarning,
  type OpenScadStableGeometryContext,
} from '../src/services/openScadStableGeometrySemantics'

function harness() {
  const warnings: OpenScadStableGeometryWarning[] = []
  const context: OpenScadStableGeometryContext = { warn: warning => warnings.push(warning) }
  return { warnings, context }
}

const unitSquare = Object.freeze([
  [0, 0], [1, 0], [1, 1], [0, 1],
] as const)

describe('kernel-neutral OpenSCAD 2021 geometry-module semantics', () => {
  it('resolves linear_extrude defaults, empty height, scale clamp, and center', () => {
    expect(resolveOpenScadLinearExtrude({ profilePoints: unitSquare })).toMatchObject({
      empty: false,
      height: 100,
      scale: [1, 1],
      center: false,
      twist: 0,
      slices: 1,
      manifoldNDivisions: 0,
      sliceSource: 'single',
    })
    expect(resolveOpenScadLinearExtrude({ height: 0, slices: 10 })).toMatchObject({
      empty: true, height: 0, slices: 0, manifoldNDivisions: 0, sliceSource: 'empty',
    })
    expect(resolveOpenScadLinearExtrude({ height: -10 })).toMatchObject({ empty: true, height: 0 })
    expect(resolveOpenScadLinearExtrude({ height: 2, scale: -2, center: true })).toMatchObject({
      empty: false, height: 2, scale: [0, 0], center: true,
    })
    expect(resolveOpenScadLinearExtrude({ height: 2, scale: [2, -1] })).toMatchObject({
      scale: [2, 0],
    })
    const invalidScale = harness()
    expect(resolveOpenScadLinearExtrude({
      height: 2, scale: [2],
    }, invalidScale.context)).toMatchObject({ scale: [1, 1] })
    expect(resolveOpenScadLinearExtrude({
      height: 2, scale: [2, 3, 4],
    }, invalidScale.context)).toMatchObject({ scale: [1, 1] })
    expect(invalidScale.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_LINEAR_EXTRUDE_SCALE_DEFAULTED',
      'OPENSCAD_LINEAR_EXTRUDE_SCALE_DEFAULTED',
    ])
  })

  it('maps OpenSCAD interval slices to Manifold interior divisions and clamps before the kernel', () => {
    expect(resolveOpenScadLinearExtrude({ height: 1, slices: 1 })).toMatchObject({
      slices: 1, unboundedSlices: 1, manifoldNDivisions: 0, sliceSource: 'explicit',
    })
    expect(resolveOpenScadLinearExtrude({ height: 1, slices: 2 })).toMatchObject({
      slices: 2, manifoldNDivisions: 1,
    })
    expect(resolveOpenScadLinearExtrude({ height: 1, slices: 9.9 })).toMatchObject({
      slices: 9, manifoldNDivisions: 8,
    })
    const { context, warnings } = harness()
    expect(resolveOpenScadLinearExtrude({
      height: 1, slices: 1_000_000, maximumSlices: 32,
    }, context)).toMatchObject({
      slices: 32, unboundedSlices: 1_000_000, manifoldNDivisions: 31, reduced: true,
    })
    expect(warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_LINEAR_EXTRUDE_SLICES_CLAMPED',
    ])
  })

  it('uses twist $fn and the 120-degree safety floor for automatic slices', () => {
    expect(resolveOpenScadLinearExtrude({
      height: 1, twist: 90, fn: 40, profilePoints: unitSquare,
    })).toMatchObject({
      slices: 10, manifoldNDivisions: 9, sliceSource: 'twist-$fn',
    })
    expect(resolveOpenScadLinearExtrude({
      height: 1, twist: 361, fn: 1, profilePoints: unitSquare,
    })).toMatchObject({ slices: 4, sliceSource: 'twist-$fn' })
    expect(resolveOpenScadLinearExtrude({
      height: 1, twist: 240, fn: Number.POSITIVE_INFINITY, profilePoints: unitSquare,
    })).toMatchObject({ slices: 2, sliceSource: 'twist-minimum' })
    expect(resolveOpenScadLinearExtrude({
      height: 1, twist: 90, fn: 100, profilePoints: [[0, 0]],
    })).toMatchObject({ slices: 1, sliceSource: 'twist-minimum' })
  })

  it('uses path length for automatic twist/nonuniform slices', () => {
    // r=1, 360 degrees, h=1: angular limit 36, path/$fs limit 7.
    expect(resolveOpenScadLinearExtrude({
      height: 1, twist: 360, fa: 10, fs: 1, profilePoints: [[1, 0]],
    })).toMatchObject({ slices: 7, sliceSource: 'twist-$fa/$fs' })
    const conical = resolveOpenScadLinearExtrude({
      height: 1, twist: 360, scale: 0, fa: 1, fs: 0.25, profilePoints: unitSquare,
    })
    expect(conical).toMatchObject({ slices: 20, sliceSource: 'twist-$fa/$fs' })

    expect(resolveOpenScadLinearExtrude({
      height: 3, scale: [2, 1], fn: 5.9, profilePoints: [[4, 0]],
    })).toMatchObject({ slices: 5, manifoldNDivisions: 4, sliceSource: 'nonuniform-$fn' })
    expect(resolveOpenScadLinearExtrude({
      height: 3, scale: [2, 1], fs: 2, profilePoints: [[4, 0]],
    })).toMatchObject({ slices: 3, manifoldNDivisions: 2, sliceSource: 'nonuniform-$fs' })
    expect(resolveOpenScadLinearExtrude({
      height: 3, scale: [2, 2], fs: 0.1, profilePoints: [[4, 0]],
    })).toMatchObject({ slices: 1, sliceSource: 'single' })
  })

  it('converts permissive row-major 4x4 values to a 2D column-major affine matrix', () => {
    expect(resolveOpenScad2dMultmatrix([
      [2, 3, 0, 4],
      [5, 6, 0, 7],
      [0, 0, 1, 0],
      [0, 0, 0, 1],
    ])).toEqual([
      2, 5, 0,
      3, 6, 0,
      4, 7, 1,
    ])
    expect(resolveOpenScad2dMultmatrix([
      [2, 0, 0, 4],
      [0, 4, 0, 8],
      [0, 0, 2, 0],
      [0, 0, 0, 2],
    ])).toEqual([
      1, 0, 0,
      0, 2, 0,
      2, 4, 1,
    ])
    expect(resolveOpenScad2dMultmatrix([[2]])).toEqual([
      2, 0, 0,
      0, 1, 0,
      0, 0, 1,
    ])
    expect(resolveOpenScad2dMultmatrix('bad')).toBeNull()
    expect(resolveOpenScad2dMultmatrix([
      [1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 0],
    ])).toBeNull()
  })

  it('computes exact planar Minkowski sums through the product identity', () => {
    type Rect = readonly [number, number, number, number]
    type Prism = readonly [number, number, number, number, number, number]
    const calls: string[] = []
    const result = openScadMinkowski2dViaProduct<Rect, Prism>([
      [0, 2, 0, 3],
      [-1, 4, 2, 5],
      [10, 11, -2, 1],
    ], {
      anchor: section => {
        calls.push('anchor')
        return [section[0], section[2]]
      },
      translate: (section, offset) => {
        calls.push('translate')
        return [
          section[0] + offset[0], section[1] + offset[0],
          section[2] + offset[1], section[3] + offset[1],
        ]
      },
      extrudeUnitPrism: section => {
        calls.push('extrude')
        return [section[0], section[1], section[2], section[3], 0, 1]
      },
      minkowskiSum: (left, right) => {
        calls.push('sum')
        return [
          left[0] + right[0], left[1] + right[1],
          left[2] + right[2], left[3] + right[3],
          left[4] + right[4], left[5] + right[5],
        ]
      },
      projectTo2d: solid => {
        calls.push('project')
        return [solid[0], solid[1], solid[2], solid[3]]
      },
    })

    expect(result).toEqual([9, 17, 0, 9])
    expect(calls).toEqual([
      'anchor', 'anchor', 'anchor',
      'translate', 'translate', 'translate',
      'extrude', 'extrude', 'sum', 'extrude', 'sum', 'project', 'translate',
    ])
  })

  it('preserves empty/single-child Minkowski identities without kernel calls', () => {
    const kernel = {
      anchor: (_value: number) => { throw new Error('must not run') },
      translate: (_value: number, _offset: readonly [number, number]) => { throw new Error('must not run') },
      extrudeUnitPrism: (_value: number) => { throw new Error('must not run') },
      minkowskiSum: (_left: never, _right: never) => { throw new Error('must not run') },
      projectTo2d: (_value: never) => { throw new Error('must not run') },
    }
    expect(openScadMinkowski2dViaProduct([], kernel)).toBeUndefined()
    expect(openScadMinkowski2dViaProduct([7], kernel)).toBe(7)
  })

  it('resolves children empty/scalar/vector/range selection in authored order', () => {
    expect(resolveOpenScadChildrenSelection({ provided: false, childCount: 3 })).toEqual([0, 1, 2])
    expect(resolveOpenScadChildrenSelection({ provided: true, value: -0.5, childCount: 3 })).toEqual([0])
    expect(resolveOpenScadChildrenSelection({ provided: true, value: 1.9, childCount: 3 })).toEqual([1])
    expect(resolveOpenScadChildrenSelection({
      provided: true, value: [2, 0, 2], childCount: 3,
    })).toEqual([2, 0, 2])
    expect(resolveOpenScadChildrenSelection({
      provided: true,
      value: { kind: 'range-value', start: 2, step: -1, end: 0 },
      childCount: 3,
    })).toEqual([2, 1, 0])
  })

  it('softly skips invalid/out-of-bounds children indices and bounds ranges', () => {
    const { context, warnings } = harness()
    expect(resolveOpenScadChildrenSelection({
      provided: true, value: [-1, 0, undefined, 9, 1], childCount: 3,
    }, context)).toEqual([0, 1])
    expect(warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_CHILDREN_INDEX_OUT_OF_BOUNDS',
      'OPENSCAD_CHILDREN_BAD_INDEX',
      'OPENSCAD_CHILDREN_INDEX_OUT_OF_BOUNDS',
    ])

    const invalid = harness()
    expect(resolveOpenScadChildrenSelection({
      provided: true, value: undefined, childCount: 3,
    }, invalid.context)).toEqual([])
    expect(invalid.warnings.map(warning => warning.code)).toEqual(['OPENSCAD_CHILDREN_BAD_SELECTION'])

    const bounded = harness()
    expect(resolveOpenScadChildrenSelection({
      provided: true,
      value: { kind: 'range-value', start: 0, step: 1, end: 100 },
      childCount: 200,
      maximumRangeItems: 4,
    }, bounded.context)).toEqual([])
    expect(bounded.warnings.map(warning => warning.code)).toEqual(['OPENSCAD_CHILDREN_RANGE_LIMIT'])
  })

  it('resolves resize zero targets and maximum authored scale for automatic axes', () => {
    expect(resolveOpenScadResize({
      dimension: 3, extents: [10, 20, 30], newsize: [100, 40, 0], auto: true,
    })).toEqual({ scales: [10, 2, 10], applied: true, valid: true })
    expect(resolveOpenScadResize({
      dimension: 3, extents: [10, 20, 30], newsize: [0, 40, 0], auto: [true, false, true],
    })).toEqual({ scales: [2, 2, 2], applied: true, valid: true })
    expect(resolveOpenScadResize({
      dimension: 3, extents: [10, 20, 30], newsize: [0, 0, 0], auto: true,
    })).toEqual({ scales: [1, 1, 1], applied: false, valid: true })
    expect(resolveOpenScadResize({
      dimension: 2, extents: [4, 8], newsize: [2], auto: true,
    })).toEqual({ scales: [0.5, 0.5], applied: true, valid: true })
  })

  it('keeps resize invalid inputs soft and rejects a positive target for a zero extent', () => {
    const invalid = harness()
    expect(resolveOpenScadResize({
      dimension: 3, extents: [1, 2, 3], newsize: 10,
    }, invalid.context)).toEqual({ scales: [1, 1, 1], applied: false, valid: false })
    expect(invalid.warnings.map(warning => warning.code)).toEqual(['OPENSCAD_RESIZE_BAD_NEWSIZE'])

    const zero = harness()
    expect(resolveOpenScadResize({
      dimension: 2, extents: [0, 2], newsize: [4, 4], auto: true,
    }, zero.context)).toEqual({ scales: [1, 1], applied: false, valid: false })
    expect(zero.warnings.map(warning => warning.code)).toEqual(['OPENSCAD_RESIZE_ZERO_EXTENT'])
  })

  it('normalizes rotate_extrude angle/sign and classifies profile sides', () => {
    expect(resolveOpenScadRotateExtrude({
      angle: 90, profileXMin: 2, profileXMax: 3, fn: 8,
    })).toMatchObject({
      empty: false, crossesAxis: false, angle: 90, profileSide: 'positive',
      reflectProfileX: false, postRotateDegrees: 0, circularSegments: 2,
    })
    expect(resolveOpenScadRotateExtrude({
      angle: -90, profileXMin: 2, profileXMax: 3, fn: 8,
    })).toMatchObject({ angle: -90, circularSegments: 2 })
    expect(resolveOpenScadRotateExtrude({
      angle: -360, profileXMin: 2, profileXMax: 3, fn: 8,
    })).toMatchObject({ angle: 360, circularSegments: 8 })
    expect(resolveOpenScadRotateExtrude({
      angle: 450, profileXMin: 2, profileXMax: 3, fn: 8,
    })).toMatchObject({ angle: 360, circularSegments: 8 })
    expect(resolveOpenScadRotateExtrude({
      angle: 90, profileXMin: -3, profileXMax: -2, fn: 8,
    })).toMatchObject({
      profileSide: 'negative', reflectProfileX: true, postRotateDegrees: 180,
    })
  })

  it('marks zero-angle, axial, and axis-crossing rotate_extrude profiles empty', () => {
    expect(resolveOpenScadRotateExtrude({
      angle: 0, profileXMin: 2, profileXMax: 3,
    })).toMatchObject({ empty: true, crossesAxis: false, circularSegments: 0 })
    expect(resolveOpenScadRotateExtrude({
      angle: 90, profileXMin: 0, profileXMax: 0,
    })).toMatchObject({ empty: true, profileSide: 'axis' })
    expect(resolveOpenScadRotateExtrude({
      angle: 90, profileXMin: -1, profileXMax: 1,
    })).toMatchObject({ empty: true, crossesAxis: true, profileSide: 'crossing' })
  })
})
