import { describe, it, expect } from 'vitest'
import { addSketchConstraint, sketchConstraintResidual, validateSketchConstraints, type SketchConstraint } from '../src/app/sketchConstraints'
import { editSketchDimension, lengthDimensionConstraints, sketchDimensionStatus, sketchDimensions } from '../src/app/directDimensions'
import { emptyDirectDocument, parseDirectDocument, serializeDirectDocument, type DirectSketch } from '../src/app/directModeling'

const TOL = 1e-6
const quad = (): DirectSketch => ({ id: 'q', name: 'Quad', closed: true, points: [[0, 0], [10, 0.4], [10.3, 6], [0.2, 5.5]] })
const allHold = (s: DirectSketch) => (s.constraints ?? []).every(c => sketchConstraintResidual(s.points, c) <= TOL)

describe('persistent sketch constraints', () => {
  it('turns a skewed quad into a rectangle and keeps it through a length edit', () => {
    let s = quad()
    for (const c of [{ kind: 'horizontal', a: 0, b: 1 }, { kind: 'vertical', a: 1, b: 2 }, { kind: 'parallel', a: 0, b: 1, c: 2, d: 3 }, { kind: 'perpendicular', a: 2, b: 3, c: 3, d: 0 }] as SketchConstraint[]) s = addSketchConstraint(s, c)
    expect(allHold(s)).toBe(true)
    s = { ...s, dimensions: [{ kind: 'length', a: 0, b: 1 }] }
    const edited = editSketchDimension(s, 0, 25)
    expect(Math.hypot(edited.points[1][0] - edited.points[0][0], edited.points[1][1] - edited.points[0][1])).toBeCloseTo(25, 6)
    expect(allHold(edited)).toBe(true)
    expect(sketchDimensionStatus(edited)?.status).not.toBe('inconsistent')
  })

  it('holds other length dimensions while adding a constraint', () => {
    const base: DirectSketch = { ...quad(), dimensions: [{ kind: 'length', a: 1, b: 2 }] }
    const before = sketchDimensions(base).measurements[0].value!
    const s = addSketchConstraint(base, { kind: 'equal_length', a: 0, b: 1, c: 2, d: 3 }, lengthDimensionConstraints(base))
    expect(allHold(s)).toBe(true)
    expect(sketchDimensions(s).measurements[0].value!).toBeCloseTo(before, 6)
  })

  it('refuses conflicting constraints without changing the sketch', () => {
    const s = addSketchConstraint(quad(), { kind: 'horizontal', a: 0, b: 1 })
    expect(() => addSketchConstraint(s, { kind: 'vertical', a: 0, b: 1 })).toThrow(/conflict/)
    expect(() => addSketchConstraint(s, { kind: 'perpendicular', a: 0, b: 1, c: 0, d: 1 })).toThrow()
  })

  it('refuses invalid structure: indices, kinds, analytic sketches and point limits', () => {
    expect(() => validateSketchConstraints({ ...quad(), constraints: [{ kind: 'horizontal', a: 0, b: 9 }] })).toThrow()
    expect(() => validateSketchConstraints({ ...quad(), constraints: [{ kind: 'horizontal', a: 1, b: 1 }] })).toThrow()
    expect(() => validateSketchConstraints({ ...quad(), constraints: [{ kind: 'tangent', a: 0, b: 1 } as unknown as SketchConstraint] })).toThrow()
    expect(() => validateSketchConstraints({ ...quad(), constraints: [{ kind: 'horizontal', a: 0, b: 1, extra: 1 } as unknown as SketchConstraint] })).toThrow()
    expect(() => validateSketchConstraints({ ...quad(), analytic: { kind: 'circle', center: [0, 0], radius: 1, start: 0, sweep: 360 }, constraints: [{ kind: 'horizontal', a: 0, b: 1 }] })).toThrow()
    const many: DirectSketch = { id: 'm', name: 'M', closed: false, points: Array.from({ length: 17 }, (_, i) => [i, i % 2] as [number, number]) }
    expect(() => validateSketchConstraints({ ...many, constraints: [{ kind: 'horizontal', a: 0, b: 1 }] })).toThrow()
  })

  it('refuses a length edit when a non-length dimension is combined with constraints', () => {
    const s: DirectSketch = { ...addSketchConstraint(quad(), { kind: 'horizontal', a: 0, b: 1 }), dimensions: [{ kind: 'length', a: 0, b: 1 }, { kind: 'angle', a: 0, b: 1, c: 2 }] }
    expect(() => editSketchDimension(s, 0, 12)).toThrow(/conflict/)
  })

  it('round-trips through the document and rejects out-of-range indices on load', () => {
    const s = addSketchConstraint(quad(), { kind: 'horizontal', a: 0, b: 1 })
    const doc = { ...emptyDirectDocument(), sketches: [s] }
    const parsed = parseDirectDocument(serializeDirectDocument(doc))
    expect(parsed.sketches[0].constraints).toEqual([{ kind: 'horizontal', a: 0, b: 1 }])
    const broken = { ...doc, sketches: [{ ...s, constraints: [{ kind: 'vertical', a: 0, b: 7 }] }] }
    expect(() => parseDirectDocument(JSON.stringify(broken))).toThrow()
  })
})
