import { describe, expect, it } from 'vitest'
import type { Point2 } from '../src/app/directModeling'
import { worldPoint, type SketchPlane } from '../src/app/directSketchGeometry'
import { projectSketchToPlane } from '../src/app/sketchProjection'
import { DirectInputError, presentKernelError } from '../src/i18n/kernelErrors'

const TOL = 1e-9
const xy = (z = 0): SketchPlane => ({ origin: [0, 0, z], u: [1, 0, 0], v: [0, 1, 0] })
// Parallel to XY but rotated 90 degrees and seen from below (normal -Z).
const flippedXY: SketchPlane = { origin: [5, 5, -7], u: [0, 1, 0], v: [1, 0, 0] }
const xz: SketchPlane = { origin: [0, 0, 0], u: [1, 0, 0], v: [0, 0, 1] }
const code = (fn: () => unknown) => {
  try {
    fn()
  } catch (e) {
    return e instanceof DirectInputError ? e.code : String(e)
  }
  return 'no error'
}
const close = (a: Point2, b: Point2) => expect(Math.hypot(a[0] - b[0], a[1] - b[1])).toBeLessThan(TOL)
const arcEnd = (c: { center: Point2; radius: number; start: number; sweep: number }): Point2 => {
  const t = ((c.start + c.sweep) * Math.PI) / 180
  return [c.center[0] + c.radius * Math.cos(t), c.center[1] + c.radius * Math.sin(t)]
}

describe('sketch contour projection', () => {
  it('projects polyline segments exactly onto parallel and oblique planes', () => {
    const square = {
      closed: true,
      points: [
        [0, 0],
        [10, 0],
        [10, 10],
        [0, 10],
      ] as Point2[],
      plane: xy(3),
    }
    const same = projectSketchToPlane(square, xy())
    same.points.forEach((p, i) => close(p, square.points[i]!))
    expect(same.closed).toBe(true)
    // Oblique plane: lines stay lines; the projected point is the foot of the perpendicular.
    const tilted: SketchPlane = { origin: [0, 0, 0], u: [1, 0, 0], v: [0, Math.SQRT1_2, Math.SQRT1_2] }
    const oblique = projectSketchToPlane(square, tilted)
    oblique.points.forEach((p, i) => {
      const w = worldPoint(square.points[i]!, square.plane)
      close(p, [w[0], (w[1] + w[2]) * Math.SQRT1_2])
    })
    expect(oblique.plane).toEqual(tilted)
  })

  it('projects circles and arcs from a parallel plane, reversing the sweep when seen from behind', () => {
    const arc = { kind: 'arc' as const, center: [2, 3] as Point2, radius: 4, start: 0, sweep: 90 }
    const src = { closed: false, points: [] as Point2[], analytic: arc, plane: xy(10) }
    const out = projectSketchToPlane(src, flippedXY).analytic!
    expect(out.radius).toBeCloseTo(4, 12)
    // World arc end points must match after mapping back through the target plane.
    const startWorld = worldPoint([6, 3], xy(10)),
      endWorld = worldPoint([2, 7], xy(10))
    const toTarget = (w: number[]): Point2 => [w[1]! - 5, w[0]! - 5]
    close(
      [
        out.center[0] + 4 * Math.cos((out.start * Math.PI) / 180),
        out.center[1] + 4 * Math.sin((out.start * Math.PI) / 180),
      ],
      toTarget(startWorld),
    )
    close(arcEnd(out), toTarget(endWorld))
    expect(out.sweep).toBeCloseTo(-90, 12)
    const circle = projectSketchToPlane(
      {
        closed: true,
        points: [],
        analytic: { kind: 'circle', center: [1, 1], radius: 2, start: 0, sweep: 360 },
        plane: xy(),
      },
      xy(5),
    )
    expect(circle.analytic).toEqual({ kind: 'circle', center: [1, 1], radius: 2, start: 0, sweep: 360 })
  })

  it('refuses inexact and degenerate projections with registered codes', () => {
    const circle = {
      closed: true,
      points: [] as Point2[],
      analytic: { kind: 'circle' as const, center: [0, 0] as Point2, radius: 1, start: 0, sweep: 360 },
    }
    expect(code(() => projectSketchToPlane(circle, xz))).toBe('DIRECT_SKETCH_PROJECTION_NOT_EXACT')
    // Almost parallel is still refused: the result would be an ellipse, however slight.
    const nearly: SketchPlane = { origin: [0, 0, 0], u: [1, 0, 0], v: [0, Math.cos(1e-6), Math.sin(1e-6)] }
    expect(code(() => projectSketchToPlane(circle, nearly))).toBe('DIRECT_SKETCH_PROJECTION_NOT_EXACT')
    // A segment along the projection direction collapses to a point.
    const upright = {
      closed: false,
      points: [
        [0, 0],
        [0, 5],
        [3, 5],
      ] as Point2[],
      plane: xz,
    }
    expect(
      code(() =>
        projectSketchToPlane(
          {
            ...upright,
            points: [
              [0, 0],
              [0, 5],
            ],
          },
          xy(),
        ),
      ),
    ).toBe('DIRECT_SKETCH_PROJECTION_INPUT')
    expect(code(() => projectSketchToPlane({ closed: false, points: [[0, 0]] }, xy()))).toBe(
      'DIRECT_SKETCH_PROJECTION_INPUT',
    )
    expect(
      code(() =>
        projectSketchToPlane(
          {
            closed: false,
            points: [
              [0, 0],
              [1, 0],
            ],
            retainedProfile: {} as never,
          },
          xy(),
        ),
      ),
    ).toBe('DIRECT_SKETCH_PROJECTION_INPUT')
    const ru = presentKernelError(new DirectInputError('DIRECT_SKETCH_PROJECTION_NOT_EXACT', 'x'), 'ru')
    const en = presentKernelError(new DirectInputError('DIRECT_SKETCH_PROJECTION_NOT_EXACT', 'x'), 'en')
    expect(JSON.stringify(ru)).toContain('эллипс')
    expect(JSON.stringify(en)).toContain('ellipse')
  })
})
