import { describe, it, expect } from 'vitest'
import {
  addSketchConstraint,
  sketchConstraintResidual,
  validateSketchConstraints,
  type SketchConstraint,
} from '../src/app/sketchConstraints'
import {
  editSketchDimension,
  dimensionSolverExtras,
  sketchDimensionStatus,
  sketchDimensions,
} from '../src/app/directDimensions'
import {
  emptyDirectDocument,
  parseDirectDocument,
  serializeDirectDocument,
  type DirectSketch,
} from '../src/app/directModeling'
import { applySolidPointEdit } from '../src/kernel/solidPointEdit'

const TOL = 1e-6
const quad = (): DirectSketch => ({
  id: 'q',
  name: 'Quad',
  closed: true,
  points: [
    [0, 0],
    [10, 0.4],
    [10.3, 6],
    [0.2, 5.5],
  ],
})
const allHold = (s: DirectSketch) => (s.constraints ?? []).every(c => sketchConstraintResidual(s.points, c) <= TOL)

describe('persistent sketch constraints', () => {
  it('turns a skewed quad into a rectangle and keeps it through a length edit', () => {
    let s = quad()
    for (const c of [
      { kind: 'horizontal', a: 0, b: 1 },
      { kind: 'vertical', a: 1, b: 2 },
      { kind: 'parallel', a: 0, b: 1, c: 2, d: 3 },
      { kind: 'perpendicular', a: 2, b: 3, c: 3, d: 0 },
    ] as SketchConstraint[])
      s = addSketchConstraint(s, c)
    expect(allHold(s)).toBe(true)
    s = { ...s, dimensions: [{ kind: 'length', a: 0, b: 1 }] }
    const edited = editSketchDimension(s, 0, 25)
    expect(
      Math.hypot(edited.points[1][0] - edited.points[0][0], edited.points[1][1] - edited.points[0][1]),
    ).toBeCloseTo(25, 6)
    expect(allHold(edited)).toBe(true)
    expect(sketchDimensionStatus(edited)?.status).not.toBe('inconsistent')
  })

  it('holds other length dimensions while adding a constraint', () => {
    const base: DirectSketch = { ...quad(), dimensions: [{ kind: 'length', a: 1, b: 2 }] }
    const before = sketchDimensions(base).measurements[0].value!
    const s = addSketchConstraint(base, { kind: 'equal_length', a: 0, b: 1, c: 2, d: 3 }, dimensionSolverExtras(base))
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
    expect(() =>
      validateSketchConstraints({
        ...quad(),
        constraints: [{ kind: 'tangent', a: 0, b: 1 } as unknown as SketchConstraint],
      }),
    ).toThrow()
    expect(() =>
      validateSketchConstraints({
        ...quad(),
        constraints: [{ kind: 'horizontal', a: 0, b: 1, extra: 1 } as unknown as SketchConstraint],
      }),
    ).toThrow()
    expect(() =>
      validateSketchConstraints({
        ...quad(),
        analytic: { kind: 'circle', center: [0, 0], radius: 1, start: 0, sweep: 360 },
        constraints: [{ kind: 'horizontal', a: 0, b: 1 }],
      }),
    ).toThrow()
    const many: DirectSketch = {
      id: 'm',
      name: 'M',
      closed: false,
      points: Array.from({ length: 17 }, (_, i) => [i, i % 2] as [number, number]),
    }
    expect(() => validateSketchConstraints({ ...many, constraints: [{ kind: 'horizontal', a: 0, b: 1 }] })).toThrow()
  })

  it('refuses a length edit when a non-length dimension is combined with constraints', () => {
    const s: DirectSketch = {
      ...addSketchConstraint(quad(), { kind: 'horizontal', a: 0, b: 1 }),
      dimensions: [
        { kind: 'length', a: 0, b: 1 },
        { kind: 'angle', a: 0, b: 1, c: 2 },
      ],
    }
    expect(() => editSketchDimension(s, 0, 12)).toThrow(/conflict/)
  })

  it('edits a horizontal dimension while keeping constraints and other offsets', () => {
    const base = [
      { kind: 'horizontal', a: 0, b: 1 },
      { kind: 'vertical', a: 1, b: 2 },
      { kind: 'parallel', a: 0, b: 1, c: 2, d: 3 },
      { kind: 'perpendicular', a: 2, b: 3, c: 3, d: 0 },
    ] as SketchConstraint[]
    const rect = base.reduce((s, c) => addSketchConstraint(s, c), quad())
    const s: DirectSketch = {
      ...rect,
      dimensions: [
        { kind: 'horizontal', a: 0, b: 1 },
        { kind: 'vertical', a: 1, b: 2 },
      ],
    }
    const heightBefore = s.points[2][1] - s.points[1][1]
    const edited = editSketchDimension(s, 0, 18)
    expect(edited.points[1][0] - edited.points[0][0]).toBeCloseTo(18, 6)
    expect(edited.points[2][1] - edited.points[1][1]).toBeCloseTo(heightBefore, 6)
    expect(allHold(edited)).toBe(true)
    const state = sketchDimensionStatus(edited)
    expect(state).not.toBeNull()
    expect(state!.status).not.toBe('inconsistent')
  })

  it('flips a vertical offset sign and refuses a horizontal offset the constraints forbid', () => {
    const s: DirectSketch = {
      ...addSketchConstraint(quad(), { kind: 'vertical', a: 1, b: 2 }),
      dimensions: [{ kind: 'vertical', a: 1, b: 2 }],
    }
    const flipped = editSketchDimension(s, 0, -4)
    expect(flipped.points[2][1] - flipped.points[1][1]).toBeCloseTo(-4, 6)
    expect(allHold(flipped)).toBe(true)
    const locked: DirectSketch = {
      ...addSketchConstraint(quad(), { kind: 'vertical', a: 1, b: 2 }),
      dimensions: [{ kind: 'horizontal', a: 1, b: 2 }],
    }
    expect(() => editSketchDimension(locked, 0, 3)).toThrow(/conflict/)
    expect(editSketchDimension(locked, 0, 0).points[2][0]).toBeCloseTo(locked.points[1][0], 6)
  })

  it('holds a horizontal dimension while adding a constraint', () => {
    const base: DirectSketch = { ...quad(), dimensions: [{ kind: 'horizontal', a: 0, b: 2 }] }
    const dx = base.points[2][0] - base.points[0][0]
    const s = addSketchConstraint(base, { kind: 'vertical', a: 1, b: 2 }, dimensionSolverExtras(base))
    expect(s.points[2][0] - s.points[0][0]).toBeCloseTo(dx, 6)
    expect(allHold(s)).toBe(true)
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

describe('sketch constraints on vertex drag', () => {
  const doc = (sketch: DirectSketch) => ({ ...emptyDirectDocument(), sketches: [sketch] })
  const constraints: SketchConstraint[] = [
    { kind: 'horizontal', a: 0, b: 1 },
    { kind: 'vertical', a: 1, b: 2 },
  ]
  const constrained = () => constraints.reduce((s, c) => addSketchConstraint(s, c), quad())

  it('re-solves constraints with the dragged vertex held at the pointer', () => {
    const before = doc(constrained())
    const after = applySolidPointEdit(before, { kind: 'sketch-vertex', id: 'q', index: 1, point: [12, 1] })
    const points = after.sketches[0]!.points
    expect(points[1]![0]).toBeCloseTo(12, 6)
    expect(points[1]![1]).toBeCloseTo(1, 6)
    for (const c of constraints) expect(sketchConstraintResidual(points, c)).toBeLessThan(TOL)
    expect(after.sketches[0]!.constraints).toEqual(constraints)
    expect(before.sketches[0]!.points[1]![0]).not.toBeCloseTo(12, 3)
  })

  it('rejects a drag the constraints cannot follow', () => {
    const before = doc(addSketchConstraint(quad(), { kind: 'horizontal', a: 0, b: 1 }))
    const p0 = before.sketches[0]!.points[0]!
    // Holding p1 on p0 collapses the constrained segment.
    expect(() =>
      applySolidPointEdit(before, { kind: 'sketch-vertex', id: 'q', index: 1, point: [p0[0], p0[1]] }),
    ).toThrow(/conflict/)
  })

  it('leaves unconstrained sketches as a plain move', () => {
    const after = applySolidPointEdit(doc(quad()), { kind: 'sketch-vertex', id: 'q', index: 2, point: [20, 20] })
    expect(after.sketches[0]!.points[2]).toEqual([20, 20])
    expect(after.sketches[0]!.points[0]).toEqual(quad().points[0])
  })
})
