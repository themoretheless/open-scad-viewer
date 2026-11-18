import { describe, it, expect } from 'vitest'
import {
  addSketchCurveConstraint,
  carryCoincidentConstraints,
  coincidentEditRefusedNotice,
  COINCIDENT_EDIT_REFUSED_MESSAGE,
  dropCurveConstraintsTo,
  pruneSketchCurveConstraints,
  resolveDependentCurveSketches,
  sketchCoincidentResidual,
  sketchConstraintOwnerResidual,
  sketchCurveConstraintName,
  validateSketchCurveConstraints,
  type SketchCurveConstraint,
} from '../src/app/sketchCurveConstraints'
import { sketchConstraintGlyphs, removeSketchConstraintRef } from '../src/app/sketchConstraintGlyphs'
import {
  emptyDirectDocument,
  parseDirectDocument,
  serializeDirectDocument,
  type DirectSketch,
} from '../src/app/directModeling'
import { sampleCurve, type AnalyticCurve } from '../src/app/directSketchGeometry'

const TOL = 1e-6
const circle = (id: string, center: [number, number], radius: number): DirectSketch => {
  const analytic: AnalyticCurve = { kind: 'circle', center, radius, start: 0, sweep: Math.PI * 2 }
  return { id, name: id, points: sampleCurve(analytic), closed: true, analytic }
}
const poly = (id: string, points: [number, number][], extra: Partial<DirectSketch> = {}): DirectSketch => ({
  id,
  name: id,
  points,
  closed: false,
  ...extra,
})
const base = poly('B', [
  [0, 0],
  [10, 0],
  [10, 10],
])
const hole = circle('C', [30, 5], 2)
const dist = (p: readonly number[], q: readonly number[]) => Math.hypot(p[0]! - q[0]!, p[1]! - q[1]!)

describe('C7 coincident constraint', () => {
  it('moves a polyline vertex onto a vertex of another polyline and keeps the reference fixed', () => {
    const a = poly('A', [
      [11, 1],
      [20, 1],
      [20, 8],
    ])
    const c: SketchCurveConstraint = { kind: 'coincident', ref: 'B', point: 0, target: 1 }
    const out = addSketchCurveConstraint(a, c, [base, a])
    expect(dist(out.points[0]!, [10, 0])).toBeLessThan(TOL)
    expect(sketchCoincidentResidual(out.points, c, base)).toBeLessThan(TOL)
    expect(out.curveConstraints).toEqual([c])
    expect(base.points[1]).toEqual([10, 0])
  })

  it('moves a polyline vertex onto a circle centre', () => {
    const a = poly('A', [
      [0, 20],
      [28, 6],
    ])
    const c: SketchCurveConstraint = { kind: 'coincident', ref: 'C', point: 1, target: 'center' }
    const out = addSketchCurveConstraint(a, c, [hole, a])
    expect(dist(out.points[1]!, [30, 5])).toBeLessThan(TOL)
    expect(sketchConstraintOwnerResidual(out, c, hole)).toBeLessThan(TOL)
  })

  it('keeps the polyline constraints and held dimensions while solving', () => {
    const a = poly(
      'A',
      [
        [12, 2],
        [22, 2],
        [22, 9],
      ],
      { constraints: [{ kind: 'horizontal', a: 0, b: 1 }], dimensions: [{ kind: 'length', a: 1, b: 2 }] },
    )
    const out = addSketchCurveConstraint(a, { kind: 'coincident', ref: 'B', point: 0, target: 2 }, [base, a])
    expect(dist(out.points[0]!, [10, 10])).toBeLessThan(TOL)
    expect(Math.abs(out.points[1]![1] - out.points[0]![1])).toBeLessThan(TOL)
    expect(Math.abs(dist(out.points[1]!, out.points[2]!) - 7)).toBeLessThan(1e-5)
  })

  it('refuses an unsatisfiable coincident without changing the sketch', () => {
    const a = poly('A', [
      [11, 1],
      [20, 1],
      [20, 8],
    ])
    const first = addSketchCurveConstraint(a, { kind: 'coincident', ref: 'B', point: 0, target: 1 }, [base, a])
    // The same vertex cannot also sit on another point.
    expect(() =>
      addSketchCurveConstraint(first, { kind: 'coincident', ref: 'B', point: 0, target: 2 }, [base, first]),
    ).toThrow(/conflict/)
    // A horizontal segment cannot join points at different heights.
    const h = poly(
      'H',
      [
        [1, 1],
        [5, 1],
        [5, 5],
      ],
      { constraints: [{ kind: 'horizontal', a: 0, b: 1 }] },
    )
    const once = addSketchCurveConstraint(h, { kind: 'coincident', ref: 'B', point: 0, target: 0 }, [base, h])
    expect(() =>
      addSketchCurveConstraint(once, { kind: 'coincident', ref: 'B', point: 1, target: 2 }, [base, once]),
    ).toThrow(/conflict/)
    // Two neighbours on one target collapse a segment.
    expect(() =>
      addSketchCurveConstraint(
        addSketchCurveConstraint(a, { kind: 'coincident', ref: 'B', point: 0, target: 1 }, [base, a]),
        { kind: 'coincident', ref: 'B', point: 1, target: 1 },
        [base, a],
      ),
    ).toThrow(/conflict/)
    expect(first.points[0]).toEqual([10, 0])
  })

  it('rejects malformed coincident constraints', () => {
    const a = poly('A', [
      [1, 1],
      [2, 2],
    ])
    const bad = (c: unknown, owner: DirectSketch = a) =>
      expect(() =>
        validateSketchCurveConstraints({ ...owner, curveConstraints: [c as SketchCurveConstraint] }, [
          base,
          hole,
          owner,
        ]),
      ).toThrow()
    bad({ kind: 'coincident', ref: 'B', point: 5, target: 0 })
    bad({ kind: 'coincident', ref: 'B', point: 0, target: 3 })
    bad({ kind: 'coincident', ref: 'B', point: 0, target: 'center' })
    bad({ kind: 'coincident', ref: 'C', point: 0, target: 0 })
    bad({ kind: 'coincident', ref: 'A', point: 0, target: 1 })
    bad({ kind: 'coincident', ref: 'B', point: 0, target: 0, extra: 1 })
    bad({ kind: 'coincident', ref: 'B', point: 0, target: 0 }, circle('K', [0, 0], 1))
    bad({ kind: 'concentric', ref: 'C' })
  })

  it('round-trips through the document and prunes stale references on load', () => {
    const a = addSketchCurveConstraint(
      poly('A', [
        [11, 1],
        [20, 1],
      ]),
      { kind: 'coincident', ref: 'B', point: 0, target: 1 },
      [base],
    )
    const doc = { ...emptyDirectDocument(), sketches: [base, hole, a] }
    const back = parseDirectDocument(serializeDirectDocument(doc))
    expect(back.sketches.find(s => s.id === 'A')!.curveConstraints).toEqual(a.curveConstraints)
    const missing = parseDirectDocument(serializeDirectDocument({ ...doc, sketches: [hole, a] }))
    expect(missing.sketches.find(s => s.id === 'A')!.curveConstraints).toBeUndefined()
    const shrunk = pruneSketchCurveConstraints([
      poly('B', [
        [0, 0],
        [1, 0],
      ]),
      { ...a, curveConstraints: [{ kind: 'coincident', ref: 'B', point: 0, target: 2 }] },
    ])
    expect(shrunk[1]!.curveConstraints).toBeUndefined()
  })

  it('follows an edit of the reference and refuses when it cannot follow', () => {
    const a = addSketchCurveConstraint(
      poly('A', [
        [11, 1],
        [20, 1],
        [20, 8],
      ]),
      { kind: 'coincident', ref: 'B', point: 0, target: 1 },
      [base],
    )
    const moved = { ...base, points: [base.points[0]!, [12, 3], base.points[2]!] as [number, number][] }
    const out = resolveDependentCurveSketches([moved, a], ['B'])
    expect(dist(out[1]!.points[0]!, [12, 3])).toBeLessThan(TOL)
    const fixed = {
      ...a,
      curveConstraints: [...a.curveConstraints!, { kind: 'coincident', ref: 'C', point: 1, target: 'center' }],
    }
    const both = resolveDependentCurveSketches(
      [
        base,
        hole,
        {
          ...fixed,
          points: [
            [10, 0],
            [30, 5],
            [20, 8],
          ],
        },
      ],
      ['C'],
    )
    expect(dist(both[2]!.points[1]!, [30, 5])).toBeLessThan(TOL)
    const collapse = { ...base, points: [base.points[0]!, [30, 5], base.points[2]!] as [number, number][] }
    expect(() => resolveDependentCurveSketches([collapse, hole, both[2]!], ['B'])).toThrow(/cannot follow/)
  })

  it('carries through an edit of the owner with moved points held', () => {
    const a = addSketchCurveConstraint(
      poly('A', [
        [11, 1],
        [20, 1],
        [20, 8],
      ]),
      { kind: 'coincident', ref: 'B', point: 0, target: 1 },
      [base],
    )
    const dragged = { ...a, points: [a.points[0]!, [25, 2], a.points[2]!] as [number, number][] }
    const out = carryCoincidentConstraints(a, dragged, [base, a])
    expect(out.points[1]).toEqual([25, 2])
    expect(dist(out.points[0]!, [10, 0])).toBeLessThan(TOL)
    const away = { ...a, points: [[3, 3], a.points[1]!, a.points[2]!] as [number, number][] }
    expect(() => carryCoincidentConstraints(a, away, [base, a])).toThrow(COINCIDENT_EDIT_REFUSED_MESSAGE)
    expect(coincidentEditRefusedNotice('ru')).toMatch(/Ломаную/)
    const offset = { ...a, points: a.points.map(p => [p[0] + 1, p[1] + 1] as [number, number]) }
    expect(dist(carryCoincidentConstraints(a, offset, [base, a]).points[0]!, [10, 0])).toBeLessThan(TOL)
    expect(carryCoincidentConstraints(base, base, [base])).toBe(base)
  })

  it('drops the coincident when the reference is trimmed, names it and shows and deletes its glyph', () => {
    const c: SketchCurveConstraint = { kind: 'coincident', ref: 'B', point: 0, target: 1 }
    const a = addSketchCurveConstraint(
      poly('A', [
        [11, 1],
        [20, 1],
      ]),
      c,
      [base],
    )
    const dropped = dropCurveConstraintsTo([base, a], 'B')
    expect(dropped.dropped).toEqual([c])
    expect(dropped.sketches[1]!.curveConstraints).toBeUndefined()
    expect(sketchCurveConstraintName(c, 'ru')).toBe('совпадение точек')
    expect(sketchCurveConstraintName(c, 'en')).toBe('coincident')
    const glyphs = sketchConstraintGlyphs([base, a], 2)
    const g = glyphs.find(x => x.kind === 'coincident')!
    expect(g).toMatchObject({ sketchId: 'A', source: 'curve', index: 0, symbol: '●' })
    expect(Math.abs(dist(g.at, a.points[0]!) - 2)).toBeLessThan(1e-9)
    expect(removeSketchConstraintRef(a, g)!.curveConstraints).toBeUndefined()
  })
})
