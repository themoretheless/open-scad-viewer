import { describe, it, expect } from 'vitest'
import {
  addSketchCurveConstraint,
  dropCurveConstraintsTo,
  droppedCurveConstraintsNotice,
  pruneSketchCurveConstraints,
  resolveDependentCurveSketches,
  sketchCurveConstraintName,
  sketchCurveConstraintResidual,
  validateSketchCurveConstraints,
} from '../src/app/sketchCurveConstraints'
import {
  emptyDirectDocument,
  parseDirectDocument,
  serializeDirectDocument,
  type DirectSketch,
} from '../src/app/directModeling'
import { sampleCurve, type AnalyticCurve } from '../src/app/directSketchGeometry'
import { applySolidPointEdit } from '../src/kernel/solidPointEdit'

const circle = (
  id: string,
  center: [number, number],
  radius: number,
  kind: 'circle' | 'arc' = 'circle',
): DirectSketch => {
  const analytic: AnalyticCurve = {
    kind,
    center,
    radius,
    start: 0,
    sweep: kind === 'circle' ? Math.PI * 2 : Math.PI / 2,
  }
  return { id, name: id, points: sampleCurve(analytic), closed: kind === 'circle', analytic }
}
const line: DirectSketch = {
  id: 'L',
  name: 'L',
  points: [
    [0, 0],
    [20, 0],
    [20, 10],
  ],
  closed: false,
}
const ref = circle('R', [0, 0], 5)

describe('sketch curve constraints', () => {
  it('makes a circle concentric with a fixed reference and keeps the reference', () => {
    const out = addSketchCurveConstraint(circle('A', [3, 4], 2), { kind: 'concentric', ref: 'R' }, [ref])
    expect(out.analytic!.center[0]).toBeCloseTo(0, 6)
    expect(out.analytic!.center[1]).toBeCloseTo(0, 6)
    expect(out.curveConstraints).toHaveLength(1)
    expect(ref.analytic!.center).toEqual([0, 0])
  })

  it('combines concentric and equal radius on an arc', () => {
    let a = addSketchCurveConstraint(circle('A', [3, 4], 2, 'arc'), { kind: 'concentric', ref: 'R' }, [ref])
    a = addSketchCurveConstraint(a, { kind: 'equal_radius', ref: 'R' }, [ref])
    expect(a.analytic!.radius).toBeCloseTo(5, 6)
    expect(a.analytic!.kind).toBe('arc')
    for (const c of a.curveConstraints!) expect(sketchCurveConstraintResidual(a.analytic!, c, ref)).toBeLessThan(1e-5)
  })

  it('makes a circle tangent to a polyline segment', () => {
    const out = addSketchCurveConstraint(circle('A', [10, 6], 3), { kind: 'tangent_line', ref: 'L', a: 0, b: 1 }, [
      line,
    ])
    expect(Math.abs(out.analytic!.center[1])).toBeCloseTo(out.analytic!.radius, 4)
  })

  it('makes circles tangent externally and internally', () => {
    const ext = addSketchCurveConstraint(
      circle('A', [12, 0], 3),
      { kind: 'tangent_circle', ref: 'R', internal: false },
      [ref],
    )
    expect(Math.hypot(...ext.analytic!.center)).toBeCloseTo(ext.analytic!.radius + 5, 4)
    const int = addSketchCurveConstraint(circle('A', [1, 0], 2), { kind: 'tangent_circle', ref: 'R', internal: true }, [
      ref,
    ])
    expect(Math.hypot(...int.analytic!.center)).toBeCloseTo(Math.abs(int.analytic!.radius - 5), 4)
  })

  it('holds a dimensioned radius and rejects a conflict', () => {
    const a: DirectSketch = { ...circle('A', [3, 4], 2), dimensions: [{ kind: 'radius' }] }
    const concentric = addSketchCurveConstraint(a, { kind: 'concentric', ref: 'R' }, [ref])
    expect(concentric.analytic!.radius).toBeCloseTo(2, 6)
    expect(() => addSketchCurveConstraint(concentric, { kind: 'equal_radius', ref: 'R' }, [ref])).toThrow(/conflict/)
    expect(() =>
      addSketchCurveConstraint(concentric, { kind: 'tangent_circle', ref: 'R', internal: false }, [ref]),
    ).toThrow(/conflict/)
  })

  it('rejects invalid references', () => {
    const a = circle('A', [3, 4], 2)
    const bad = (c: unknown, sketches: DirectSketch[] = [ref, line]) =>
      expect(() => validateSketchCurveConstraints({ ...a, curveConstraints: [c as never] }, sketches)).toThrow()
    bad({ kind: 'concentric', ref: 'missing' })
    bad({ kind: 'concentric', ref: 'A' })
    bad({ kind: 'concentric', ref: 'L' })
    bad({ kind: 'tangent_line', ref: 'R', a: 0, b: 1 })
    bad({ kind: 'tangent_line', ref: 'L', a: 0, b: 2 })
    bad({ kind: 'tangent_circle', ref: 'R' })
    bad({ kind: 'concentric', ref: 'R', extra: 1 })
    bad({ kind: 'concentric', ref: 'R' }, [{ ...ref, plane: { origin: [0, 0, 5], u: [1, 0, 0], v: [0, 1, 0] } }])
    expect(() =>
      validateSketchCurveConstraints({ ...line, curveConstraints: [{ kind: 'concentric', ref: 'R' }] }, [ref]),
    ).toThrow(/circle or arc/)
  })

  it('round-trips through the document and drops dangling references on load', () => {
    const a = addSketchCurveConstraint(circle('A', [3, 4], 2), { kind: 'concentric', ref: 'R' }, [ref])
    const doc = { ...emptyDirectDocument(), sketches: [ref, a] }
    const back = parseDirectDocument(serializeDirectDocument(doc))
    expect(back.sketches[1]!.curveConstraints).toEqual([{ kind: 'concentric', ref: 'R' }])
    const loaded = parseDirectDocument(serializeDirectDocument({ ...doc, sketches: [a] }))
    expect(loaded.sketches[0]!.curveConstraints).toBeUndefined()
    expect(loaded.sketches[0]!.analytic!.center[0]).toBeCloseTo(0, 6)
  })

  it('drops constraints that reference a deleted sketch and keeps the rest', () => {
    const other = circle('S', [30, 0], 4)
    let a = addSketchCurveConstraint(circle('A', [3, 4], 2), { kind: 'concentric', ref: 'R' }, [ref, other])
    a = addSketchCurveConstraint(a, { kind: 'equal_radius', ref: 'S' }, [ref, other])
    const all = [ref, other, a]
    expect(pruneSketchCurveConstraints(all)).toBe(all)
    const pruned = pruneSketchCurveConstraints([other, a])
    expect(pruned[1]!.curveConstraints).toEqual([{ kind: 'equal_radius', ref: 'S' }])
    expect(pruneSketchCurveConstraints([a])[0]!.curveConstraints).toBeUndefined()
  })

  it('names constraints in RU and EN', () => {
    expect(sketchCurveConstraintName({ kind: 'tangent_circle', ref: 'R', internal: true }, 'ru')).toBe(
      'касание окружностей (внутреннее)',
    )
    expect(sketchCurveConstraintName({ kind: 'concentric', ref: 'R' }, 'en')).toBe('concentric')
  })
  it('re-solves dependent circles transitively when a referenced sketch moves', () => {
    const a = addSketchCurveConstraint(circle('A', [3, 4], 2), { kind: 'concentric', ref: 'R' }, [ref])
    const b = addSketchCurveConstraint(circle('B', [20, 0], 1), { kind: 'tangent_circle', ref: 'A', internal: false }, [
      a,
    ])
    const all = [ref, a, b]
    expect(resolveDependentCurveSketches(all, ['L'])).toBe(all)
    const moved = circle('R', [7, 1], 5)
    const out = resolveDependentCurveSketches([moved, a, b], ['R'])
    expect(out[1]!.analytic!.center[0]).toBeCloseTo(7, 6)
    expect(out[1]!.analytic!.center[1]).toBeCloseTo(1, 6)
    expect(sketchCurveConstraintResidual(out[2]!.analytic!, b.curveConstraints![0]!, out[1]!)).toBeLessThan(1e-5)
    expect(out[0]).toBe(moved)
  })

  // Concentric with R and a held radius of 3 tangent to y=0: fully determined, so moving the line cannot be followed.
  const pinned = () => {
    const r = circle('R', [10, 3], 1)
    let a = { ...circle('A', [10, 3], 3), dimensions: [{ kind: 'radius' }] } as DirectSketch
    a = addSketchCurveConstraint(a, { kind: 'concentric', ref: 'R' }, [r, line])
    a = addSketchCurveConstraint(a, { kind: 'tangent_line', ref: 'L', a: 0, b: 1 }, [r, line])
    return [line, r, a]
  }

  it('refuses an edit the dependent cannot follow', () => {
    const [, r, a] = pinned()
    const lifted = { ...line, points: line.points.map(([x, y]) => [x, y + 1] as [number, number]) }
    expect(() => resolveDependentCurveSketches([lifted, r!, a!], ['L'])).toThrow(/cannot follow/)
  })

  it('follows a dragged polyline vertex in the point edit and refuses an unreachable drag', () => {
    const a = addSketchCurveConstraint(circle('A', [10, 6], 3), { kind: 'tangent_line', ref: 'L', a: 0, b: 1 }, [line])
    const doc = { ...emptyDirectDocument(), sketches: [line, a] }
    const out = applySolidPointEdit(doc, { kind: 'sketch-vertex', id: 'L', index: 1, point: [20, 8] })
    const c = out.sketches[1]!
    expect(sketchCurveConstraintResidual(c.analytic!, c.curveConstraints![0]!, out.sketches[0]!)).toBeLessThan(1e-5)
    expect(() =>
      applySolidPointEdit(
        { ...emptyDirectDocument(), sketches: pinned() },
        { kind: 'sketch-vertex', id: 'L', index: 1, point: [20, 8] },
      ),
    ).toThrow(/cannot follow/)
  })

  it('drops constraints to a trimmed sketch and reports them in RU and EN', () => {
    const a = addSketchCurveConstraint(circle('A', [10, 6], 3), { kind: 'tangent_line', ref: 'L', a: 0, b: 1 }, [line])
    const all = [line, a]
    expect(dropCurveConstraintsTo(all, 'X').sketches).toBe(all)
    const { sketches, dropped } = dropCurveConstraintsTo(all, 'L')
    expect(sketches[1]!.curveConstraints).toBeUndefined()
    expect(dropped).toEqual([{ kind: 'tangent_line', ref: 'L', a: 0, b: 1 }])
    expect(droppedCurveConstraintsNotice(dropped, 'en')).toBe(
      'Trim removed circle and arc constraints: tangent to segment.',
    )
    expect(droppedCurveConstraintsNotice(dropped, 'ru')).toContain('касание отрезка')
    expect(droppedCurveConstraintsNotice([], 'en')).toBe('')
  })
})
