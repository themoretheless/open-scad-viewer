import { describe, expect, it } from 'vitest'
import type { Point2 } from '../src/app/directModeling'
import {
  analyticSlotBoundary,
  ellipseSketchCurve,
  resizeSlot,
  slotDimensions,
  tangentArc,
  tangentArcConstraint,
  slotMatchesSketch,
  validateSlotDefinition,
} from '../src/app/sketchPrimitives'
import { resolveDependentCurveSketches, sketchCurveConstraintResidual } from '../src/app/sketchCurveConstraints'
import { DirectInputError, presentKernelError } from '../src/i18n/kernelErrors'

const TOL = 1e-9
const at = (c: { center: Point2; radius: number }, deg: number): Point2 => [
  c.center[0] + c.radius * Math.cos((deg * Math.PI) / 180),
  c.center[1] + c.radius * Math.sin((deg * Math.PI) / 180),
]
const code = (fn: () => unknown) => {
  try {
    fn()
  } catch (e) {
    return e instanceof DirectInputError ? e.code : 'other'
  }
  return 'none'
}

describe('C7 ellipse', () => {
  it('keeps an equal-radius ellipse exact as a circle', () => {
    expect(ellipseSketchCurve({ center: [3, -4], radiusA: 7, radiusB: 7, rotation: 33 })).toEqual({
      kind: 'circle',
      center: [3, -4],
      radius: 7,
      start: 0,
      sweep: 360,
    })
  })

  it('refuses an unequal ellipse with a registered code instead of approximating it', () => {
    expect(code(() => ellipseSketchCurve({ center: [0, 0], radiusA: 10, radiusB: 5, rotation: 0 }))).toBe(
      'DIRECT_SKETCH_ELLIPSE_NOT_EXACT',
    )
    expect(code(() => ellipseSketchCurve({ center: [0, 0], radiusA: 10, radiusB: 10 * (1 + 1e-6), rotation: 0 }))).toBe(
      'DIRECT_SKETCH_ELLIPSE_NOT_EXACT',
    )
    const ru = presentKernelError(new DirectInputError('DIRECT_SKETCH_ELLIPSE_NOT_EXACT', 'x'), 'ru')
    expect(ru.message).toMatch(/эллипс/i)
  })

  it.each([
    [[0, 0], 0.001, 1, 0],
    [[0, 0], 1, -1, 0],
    [[NaN, 0], 1, 1, 0],
    [[0, 2e6], 1, 1, 0],
    [[0, 0], 1, 1, Infinity],
  ] as const)('rejects invalid input %#', (center, a, b, rotation) => {
    expect(code(() => ellipseSketchCurve({ center: [...center] as Point2, radiusA: a, radiusB: b, rotation }))).toBe(
      'DIRECT_SKETCH_ELLIPSE_INPUT',
    )
  })
})

describe('C7 slot', () => {
  const slot = { a: [0, 0] as Point2, b: [30, 40] as Point2, width: 10 }

  it('measures overall length and width', () => {
    const d = slotDimensions(slot)!
    expect(d.centerDistance).toBeCloseTo(50, 12)
    expect(d.length).toBeCloseTo(60, 12)
    expect(d.width).toBe(10)
  })

  it('applies length and width dimensions along the center line', () => {
    const longer = resizeSlot(slot, { length: 110 })!
    expect(longer.b[0]).toBeCloseTo(60, 9)
    expect(longer.b[1]).toBeCloseTo(80, 9)
    const wider = resizeSlot(slot, { width: 20 })!
    expect(slotDimensions(wider)!.length).toBeCloseTo(60, 9)
    expect(slotDimensions(wider)!.centerDistance).toBeCloseTo(40, 9)
  })

  it('refuses dimensions that collapse the center line or invalid slots', () => {
    expect(resizeSlot(slot, { length: 10 })).toBeNull()
    expect(resizeSlot(slot, { length: 5 })).toBeNull()
    expect(resizeSlot(slot, { width: 0.01 })).toBeNull()
    expect(resizeSlot(slot, { length: NaN })).toBeNull()
    expect(slotDimensions({ a: [1, 1], b: [1, 1], width: 2 })).toBeNull()
    expect(analyticSlotBoundary({ a: [0, 0], b: [1, 0], width: 0 })).toBeNull()
  })

  it('builds two semicircles joined tangentially by the side lines', () => {
    const { lines, caps } = analyticSlotBoundary(slot)!
    for (const cap of caps) {
      expect(cap.radius).toBe(5)
      expect(cap.sweep).toBe(180)
    }
    const [capB, capA] = caps
    const close = (p: Point2, q: Point2) => expect(Math.hypot(p[0] - q[0], p[1] - q[1])).toBeLessThan(TOL)
    close(at(capB, capB.start), lines[0][1])
    close(at(capB, capB.start + capB.sweep), lines[1][0])
    close(at(capA, capA.start), lines[1][1])
    close(at(capA, capA.start + capA.sweep), lines[0][0])
    // Tangency: the radius at each joint is perpendicular to the line.
    const dir = [lines[0][1][0] - lines[0][0][0], lines[0][1][1] - lines[0][0][1]]
    const rad = [lines[0][1][0] - capB.center[0], lines[0][1][1] - capB.center[1]]
    expect(Math.abs(dir[0]! * rad[0]! + dir[1]! * rad[1]!)).toBeLessThan(1e-9)
  })
})

describe('C7 tangent arc', () => {
  const line = {
    points: [
      [0, 0],
      [10, 0],
    ] as Point2[],
    closed: false,
  }

  it('continues counterclockwise from the end tangent to the last segment', () => {
    const arc = tangentArc(line, 'end', [20, 10])!
    expect(arc.center[0]).toBeCloseTo(10, 12)
    expect(arc.center[1]).toBeCloseTo(10, 12)
    expect(arc.radius).toBeCloseTo(10, 12)
    expect(arc.start).toBeCloseTo(-90, 9)
    expect(arc.sweep).toBeCloseTo(90, 9)
  })

  it('turns clockwise when the target is to the right and supports the start end', () => {
    const cw = tangentArc(line, 'end', [20, -10])!
    expect(cw.sweep).toBeCloseTo(-90, 9)
    expect(cw.center[1]).toBeCloseTo(-10, 12)
    const back = tangentArc(line, 'start', [-10, 10])!
    expect(back.center[0]).toBeCloseTo(0, 12)
    expect(back.center[1]).toBeCloseTo(10, 12)
    expect(back.sweep).toBeCloseTo(-90, 9)
  })

  it('is tangent to the segment and ends at the target for an oblique case', () => {
    const poly = {
      points: [
        [1, 2],
        [4, 6],
      ] as Point2[],
      closed: false,
    }
    const target: Point2 = [3, 12]
    const arc = tangentArc(poly, 'end', target)!
    const s = at(arc, arc.start),
      e = at(arc, arc.start + arc.sweep)
    expect(Math.hypot(s[0] - 4, s[1] - 6)).toBeLessThan(TOL)
    expect(Math.hypot(e[0] - target[0], e[1] - target[1])).toBeLessThan(1e-8)
    const radial = [4 - arc.center[0], 6 - arc.center[1]]
    expect(Math.abs(radial[0]! * 3 + radial[1]! * 4)).toBeLessThan(1e-9)
    // Direction of travel at the start matches the segment direction.
    const sign = Math.sign(arc.sweep)
    expect(-radial[1]! * sign * 3 + radial[0]! * sign * 4).toBeGreaterThan(0)
  })

  it('refuses collinear, coincident, closed, analytic and degenerate inputs', () => {
    expect(tangentArc(line, 'end', [30, 0])).toBeNull()
    expect(tangentArc(line, 'end', [-5, 0])).toBeNull()
    expect(tangentArc(line, 'end', [10, 0])).toBeNull()
    expect(tangentArc({ ...line, closed: true }, 'end', [20, 10])).toBeNull()
    expect(
      tangentArc(
        { ...line, analytic: { kind: 'arc', center: [0, 0], radius: 1, start: 0, sweep: 90 } },
        'end',
        [20, 10],
      ),
    ).toBeNull()
    expect(
      tangentArc(
        {
          points: [
            [1, 1],
            [1, 1],
          ],
          closed: false,
        },
        'end',
        [5, 5],
      ),
    ).toBeNull()
    expect(tangentArc({ points: [[0, 0]], closed: false }, 'end', [5, 5])).toBeNull()
    expect(tangentArc(line, 'end', [10, 0.001])).toBeNull()
  })
})

describe('persistent tangent arc', () => {
  const path = {
    id: 'path',
    name: 'Path',
    closed: false,
    points: [
      [0, 0],
      [10, 0],
    ] as Point2[],
  }
  const arcSketch = (end: 'start' | 'end', target: Point2, extra: object = {}) => {
    const analytic = tangentArc(path, end, target)!
    return {
      id: 'arc',
      name: 'Arc',
      closed: false,
      points: [] as Point2[],
      analytic,
      curveConstraints: [tangentArcConstraint(path, end)],
      ...extra,
    }
  }

  it('stores tangency to the end segment it continues, from either end', () => {
    expect(tangentArcConstraint(path, 'end')).toEqual({ kind: 'tangent_line', ref: 'path', a: 0, b: 1, point: 1 })
    const three = {
      id: 'p3',
      points: [
        [0, 0],
        [5, 0],
        [5, 5],
      ] as Point2[],
    }
    expect(tangentArcConstraint(three, 'end')).toEqual({ kind: 'tangent_line', ref: 'p3', a: 1, b: 2, point: 2 })
    expect(tangentArcConstraint(three, 'start')).toEqual({ kind: 'tangent_line', ref: 'p3', a: 0, b: 1, point: 0 })
    const fromStart = tangentArc(path, 'start', [-10, 10])!
    expect(fromStart.center[0]).toBeCloseTo(0, 9)
    expect(fromStart.center[1]).toBeCloseTo(10, 9)
    expect(fromStart.radius).toBeCloseTo(10, 9)
  })

  it('re-solves the arc tangent when the polyline moves and refuses an unreachable edit', () => {
    const arc = arcSketch('end', [20, 10])
    const moved = {
      ...path,
      points: [
        [0, 3],
        [10, 5],
      ] as Point2[],
    }
    const [, next] = resolveDependentCurveSketches([moved, arc], ['path'])
    expect(sketchCurveConstraintResidual(next!.analytic!, arc.curveConstraints[0]!, moved)).toBeLessThan(1e-6)
    // A held radius plus tangency to a far parallel line cannot both hold: the whole edit is refused.
    const wall = {
      id: 'wall',
      name: 'Wall',
      closed: false,
      points: [
        [0, 20],
        [10, 20],
      ] as Point2[],
    }
    const pinned = {
      ...arc,
      dimensions: [{ kind: 'radius' }],
      curveConstraints: [...arc.curveConstraints, { kind: 'tangent_line' as const, ref: 'wall', a: 0, b: 1 }],
    }
    const far = {
      ...path,
      points: [
        [0, -30],
        [10, -30],
      ] as Point2[],
    }
    expect(() => resolveDependentCurveSketches([far, wall, pinned], ['path'])).toThrow(/not applied/)
  })
  it('C7 tangent arc keeps its start point on the polyline end when the polyline moves', () => {
    const arc = arcSketch('end', [20, 10])
    const moved = {
      ...path,
      points: [
        [0, 3],
        [12, 8],
      ] as Point2[],
    }
    const [, next] = resolveDependentCurveSketches([moved, arc], ['path'])
    const a = next!.analytic!
    const t = (a.start * Math.PI) / 180
    expect(
      Math.hypot(a.center[0] + a.radius * Math.cos(t) - 12, a.center[1] + a.radius * Math.sin(t) - 8),
    ).toBeLessThan(1e-9)
    expect(a.radius).toBeCloseTo(10, 12)
    expect(a.sweep).toBeCloseTo(arc.analytic.sweep, 12)
    expect(sketchCurveConstraintResidual(a, arc.curveConstraints[0]!, moved)).toBeLessThan(1e-9)
    // Tangent direction at the start equals the segment direction.
    const dir = [12 / 13, 5 / 13]
    const tangent = [-Math.sin(t), Math.cos(t)]
    expect(tangent[0]! * dir[0]! + tangent[1]! * dir[1]!).toBeCloseTo(1, 12)
    // A start constraint on a vertex outside the tangent segment is invalid.
    const bad = { ...arc, curveConstraints: [{ kind: 'tangent_line' as const, ref: 'path', a: 0, b: 1, point: 5 }] }
    expect(() => resolveDependentCurveSketches([moved, bad], ['path'])).toThrow(/not applied/)
  })
})

describe('C7 slot definition', () => {
  const slot = { a: [0, 0] as Point2, b: [10, 0] as Point2, width: 4 }
  it('matches the outline it describes and validates its shape', () => {
    const outline = analyticSlotBoundary(slot)!
    const points = [...outline.lines.flat(), [12, 0], [-2, 0], [10, 2]] as Point2[]
    expect(slotMatchesSketch(slot, points)).toBe(true)
    expect(slotMatchesSketch(slot, [...points, [5, 0]])).toBe(false)
    expect(slotMatchesSketch({ ...slot, width: 4.1 }, points)).toBe(false)
    expect(() => validateSlotDefinition(slot)).not.toThrow()
    for (const bad of [
      null,
      { ...slot, extra: 1 },
      { ...slot, a: null },
      { ...slot, b: [0, 0] },
      { ...slot, width: 0 },
    ])
      expect(() => validateSlotDefinition(bad)).toThrow(/Invalid sketch slot/)
  })
})
