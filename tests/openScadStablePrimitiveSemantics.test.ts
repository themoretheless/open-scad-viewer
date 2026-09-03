import { describe, expect, it } from 'vitest'
import {
  resolveOpenScadCircle,
  resolveOpenScadCube,
  resolveOpenScadCylinder,
  resolveOpenScadPolygon,
  resolveOpenScadPolyhedron,
  resolveOpenScadSphere,
  resolveOpenScadSquare,
  type OpenScadStablePrimitiveContext,
  type OpenScadStablePrimitiveWarning,
} from '../src/services/openScadStablePrimitiveSemantics'

function harness(checkParameterRanges = false) {
  const warnings: OpenScadStablePrimitiveWarning[] = []
  const context: OpenScadStablePrimitiveContext = {
    warn: warning => warnings.push(warning),
    checkParameterRanges,
  }
  return { warnings, context }
}

describe('kernel-neutral OpenSCAD 2021 primitive semantics', () => {
  it('requires exact box vectors, falls back softly, and classifies empty ranges', () => {
    expect(resolveOpenScadCube()).toMatchObject({
      dimensions: [1, 1, 1], center: false, sizeSource: 'default', empty: false,
    })
    expect(resolveOpenScadCube({ size: 2, center: true })).toMatchObject({
      dimensions: [2, 2, 2], center: true, sizeSource: 'scalar', empty: false,
    })
    expect(resolveOpenScadSquare({ size: [2, 3] })).toMatchObject({
      dimensions: [2, 3], sizeSource: 'vector', empty: false,
    })

    const invalid = harness()
    expect(resolveOpenScadCube({ size: [2, 3] }, invalid.context)).toMatchObject({
      dimensions: [1, 1, 1], sizeSource: 'default', empty: false,
    })
    expect(resolveOpenScadSquare({ size: [2, 3, 4], center: 1 }, invalid.context)).toMatchObject({
      dimensions: [1, 1], center: false, sizeSource: 'default', empty: false,
    })
    expect(resolveOpenScadCube({ size: [2, 'bad', 4] }, invalid.context)).toMatchObject({
      dimensions: [2, 1, 1], sizeSource: 'partial-vector', empty: false,
    })
    expect(invalid.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_PRIMITIVE_SIZE_DEFAULTED',
      'OPENSCAD_PRIMITIVE_SIZE_DEFAULTED',
      'OPENSCAD_PRIMITIVE_SIZE_DEFAULTED',
    ])

    const ranged = harness(true)
    expect(resolveOpenScadCube({ size: [2, 0, 4] }, ranged.context)).toMatchObject({ empty: true })
    expect(resolveOpenScadSquare({ size: Number.POSITIVE_INFINITY }, ranged.context)).toMatchObject({ empty: true })
    expect(ranged.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_PRIMITIVE_RANGE_EMPTY',
      'OPENSCAD_PRIMITIVE_RANGE_EMPTY',
    ])
  })

  it('gives numeric diameter priority and preserves default/empty radius behavior', () => {
    const shadowed = harness()
    expect(resolveOpenScadSphere({ r: 3, d: 4 }, shadowed.context)).toMatchObject({
      radius: 2, radiusSource: 'diameter', fragmentRadius: 2, empty: false,
    })
    expect(resolveOpenScadCircle({ r: 3, d: 10 }, shadowed.context)).toMatchObject({
      radius: 5, radiusSource: 'diameter', fragmentRadius: 5, empty: false,
    })
    expect(shadowed.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_PRIMITIVE_RADIUS_SHADOWED',
      'OPENSCAD_PRIMITIVE_RADIUS_SHADOWED',
    ])

    expect(resolveOpenScadSphere({ r: 'bad' })).toMatchObject({
      radius: 1, radiusSource: 'default', empty: false,
    })
    expect(resolveOpenScadCircle({ r: 'bad', d: 8 })).toMatchObject({
      radius: 4, radiusSource: 'diameter', empty: false,
    })
    expect(resolveOpenScadSphere({ r: 0 })).toMatchObject({ radius: 0, empty: true })
    expect(resolveOpenScadCircle({ d: -2 })).toMatchObject({ radius: -1, empty: true })
  })

  it('layers common and endpoint cylinder radii without copying a lone endpoint', () => {
    expect(resolveOpenScadCylinder({ h: 2, r1: 2 })).toMatchObject({
      height: 2,
      radius1: 2,
      radius2: 1,
      radius1Source: 'radius1',
      radius2Source: 'default',
      empty: false,
    })

    const ambiguous = harness()
    expect(resolveOpenScadCylinder({ h: 2, r: 4, r1: 2 }, ambiguous.context)).toMatchObject({
      radius1: 2,
      radius2: 4,
      radius1Source: 'radius1',
      radius2Source: 'radius',
      fragmentRadius: 4,
    })
    expect(ambiguous.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_CYLINDER_AMBIGUOUS_RADII',
    ])

    const diameter = harness()
    expect(resolveOpenScadCylinder({ r: 8, d: 10, r1: 7, d1: 4, d2: 6 }, diameter.context))
      .toMatchObject({
        radius1: 2,
        radius2: 3,
        radius1Source: 'diameter1',
        radius2Source: 'diameter2',
      })
    expect(diameter.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_PRIMITIVE_RADIUS_SHADOWED',
      'OPENSCAD_PRIMITIVE_RADIUS_SHADOWED',
      'OPENSCAD_CYLINDER_AMBIGUOUS_RADII',
    ])

    expect(resolveOpenScadCylinder({ h: 'bad', r: 'bad', center: 'yes' })).toMatchObject({
      height: 1, radius1: 1, radius2: 1, center: false, empty: false,
    })
    expect(resolveOpenScadCylinder({ h: 0 })).toMatchObject({ empty: true })
    expect(resolveOpenScadCylinder({ r1: 0, r2: 0 })).toMatchObject({ empty: true })
    expect(resolveOpenScadCylinder({ r1: 0, r2: 1 })).toMatchObject({ empty: false })
  })

  it('accepts referenced polyhedron vec2 points and applies index conversion/skips', () => {
    const { context, warnings } = harness()
    const result = resolveOpenScadPolyhedron({
      points: [[0, 0], [1, 0, 0], [0, 1], ['unreferenced']],
      faces: [['not-a-number', 1.9, 99, 2]],
      convexity: 0,
    }, context)
    expect(result).toMatchObject({
      facesSource: 'faces', convexity: 1, aborted: false, empty: false, reduced: false,
    })
    expect(result.polygons).toEqual([[
      [0, 0, 0],
      [1, 0, 0],
      [0, 1, 0],
    ]])
    expect(warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_PRIMITIVE_INDEX_OUT_OF_BOUNDS',
    ])
  })

  it('keeps polyhedron face order and stops at the first referenced malformed point', () => {
    const { context, warnings } = harness()
    const result = resolveOpenScadPolyhedron({
      points: [[0, 0], [1, 0], [0, 1], [9]],
      faces: [[0, 1, 2], [0, 3, 1], [2, 1, 0]],
    }, context)
    expect(result).toMatchObject({ aborted: true, empty: false, reduced: false })
    expect(result.polygons).toEqual([
      [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
      [[0, 0, 0]],
    ])
    expect(warnings.map(warning => warning.code)).toEqual(['OPENSCAD_PRIMITIVE_POINT_INVALID'])
  })

  it('selects faces over the deprecated triangles alias and retains degenerate faces', () => {
    const legacy = harness()
    expect(resolveOpenScadPolyhedron({
      points: [[0, 0], [1, 0], [0, 1]],
      triangles: [[0, 1, 2]],
      convexity: 2.9,
    }, legacy.context)).toMatchObject({
      facesSource: 'triangles', convexity: 2, empty: false,
    })
    expect(legacy.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_POLYHEDRON_TRIANGLES_DEPRECATED',
    ])

    const explicit = resolveOpenScadPolyhedron({
      points: [[0, 0], [1, 0], [0, 1]],
      faces: [5, [0, 1]],
      triangles: [[0, 1, 2]],
    })
    expect(explicit.facesSource).toBe('faces')
    expect(explicit.polygons).toEqual([[], [[0, 0, 0], [1, 0, 0]]])
    expect(explicit.empty).toBe(true)
  })

  it('requires exact polygon vec2 points and distinguishes implicit and explicit paths', () => {
    expect(resolveOpenScadPolygon({
      points: [[0, 0], [1, 0], [0, 1]],
      paths: 'not-a-vector',
    })).toMatchObject({
      pathSource: 'implicit',
      outlines: [[[0, 0], [1, 0], [0, 1]]],
      empty: false,
      aborted: false,
    })

    expect(resolveOpenScadPolygon({
      points: [[0, 0], [1, 0], [0, 1]],
      paths: [0, 1, 2],
    })).toMatchObject({ pathSource: 'explicit', outlines: [[], [], []], empty: true })

    const skipped = harness()
    expect(resolveOpenScadPolygon({
      points: [[0, 0], [1, 0], [0, 1]],
      paths: [[0, 1, 99, 2]],
    }, skipped.context)).toMatchObject({
      outlines: [[[0, 0], [1, 0], [0, 1]]], empty: false,
    })
    expect(skipped.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_PRIMITIVE_INDEX_OUT_OF_BOUNDS',
    ])

    const invalid = harness()
    expect(resolveOpenScadPolygon({
      points: [[0, 0, 7], [1, 0], [0, 1]],
    }, invalid.context)).toMatchObject({ aborted: true, outlines: [], empty: true })
    expect(invalid.warnings.map(warning => warning.code)).toEqual(['OPENSCAD_PRIMITIVE_POINT_INVALID'])

    const nanPoint = resolveOpenScadPolygon({
      points: [[Number.NaN, 0], [1, 0], [0, 1]],
    })
    expect(Number.isNaN(nanPoint.points[0][0])).toBe(true)
    expect(nanPoint.aborted).toBe(false)
  })

  it('makes host-limit reductions observable instead of silently truncating topology', () => {
    const pointLimit = harness()
    expect(resolveOpenScadPolyhedron({
      points: [[0, 0], [1, 0]],
      faces: [[0, 1]],
      limits: { maximumPoints: 1 },
    }, pointLimit.context)).toMatchObject({
      empty: true,
      reduced: true,
      reduction: { reason: 'input-limit', resource: 'points', actual: 2, limit: 1 },
    })

    const indexLimit = harness()
    expect(resolveOpenScadPolygon({
      points: [[0, 0], [1, 0], [0, 1]],
      paths: [[0, 1, 2]],
      limits: { maximumIndices: 2 },
    }, indexLimit.context)).toMatchObject({
      reduced: true,
      aborted: true,
      reduction: { reason: 'input-limit', resource: 'indices', actual: 3, limit: 2 },
    })
    expect([...pointLimit.warnings, ...indexLimit.warnings].map(warning => warning.code)).toEqual([
      'OPENSCAD_PRIMITIVE_SAFETY_REDUCTION',
      'OPENSCAD_PRIMITIVE_SAFETY_REDUCTION',
    ])
  })
})
