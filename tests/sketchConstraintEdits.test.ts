import { describe, it, expect } from 'vitest'
import {
  remapSketchConstraints,
  resolveEditedSketchConstraints,
  sketchConstraintResidual,
  type SketchConstraint,
} from '../src/app/sketchConstraints'
import type { DirectSketch, Point2 } from '../src/app/directModeling'

const TOL = 1e-6
const satisfied = (s: DirectSketch) => (s.constraints ?? []).every(c => sketchConstraintResidual(s.points, c) <= TOL)
const square = (constraints: SketchConstraint[]): DirectSketch => ({
  id: 'sq',
  name: 'Square',
  closed: true,
  points: [
    [0, 0],
    [10, 0],
    [10, 10],
    [0, 10],
  ],
  constraints,
})

describe('persistent constraints across sketch geometry edits', () => {
  it('re-solves with moved points held and refuses an unreachable edit', () => {
    const before = square([
      { kind: 'horizontal', a: 0, b: 1 },
      { kind: 'vertical', a: 1, b: 2 },
    ])
    const moved = { ...before, points: before.points.map((p, i) => (i === 2 ? [13, 12] : p)) as Point2[] }
    const solved = resolveEditedSketchConstraints(before, moved)
    expect(solved.points[2]![0]).toBeCloseTo(13, 6)
    expect(satisfied(solved)).toBe(true)
    expect(solved.points[1]![0]).toBeCloseTo(13, 6)
    const conflict = {
      ...before,
      points: before.points.map((p, i) => (i === 1 ? [10, 3] : i === 0 ? [0, -1] : p)) as Point2[],
    }
    expect(() => resolveEditedSketchConstraints(before, conflict)).toThrow(/conflict/)
    expect(() => resolveEditedSketchConstraints(before, { ...before, points: before.points.slice(0, 3) })).toThrow(
      /remap/,
    )
    const plain = { ...moved, constraints: undefined }
    expect(resolveEditedSketchConstraints(before, plain)).toBe(plain)
  })

  it('remaps constraints onto renumbered pieces and drops the ones without a surviving segment', () => {
    const before = square([
      { kind: 'horizontal', a: 0, b: 1 },
      { kind: 'vertical', a: 1, b: 2 },
      { kind: 'horizontal', a: 2, b: 3 },
      { kind: 'vertical', a: 3, b: 0 },
      { kind: 'equal_length', a: 1, b: 2, c: 3, d: 0 },
    ])
    const piece: DirectSketch = {
      ...before,
      closed: false,
      points: [
        [7, 0],
        [10, 0],
        [10, 10],
        [0, 10],
        [0, 0],
        [3, 0],
      ],
    }
    const { sketches, dropped } = remapSketchConstraints(before, [piece])
    expect(dropped).toEqual([{ kind: 'horizontal', a: 0, b: 1 }])
    expect(sketches[0]!.constraints).toEqual([
      { kind: 'vertical', a: 1, b: 2 },
      { kind: 'horizontal', a: 2, b: 3 },
      { kind: 'vertical', a: 3, b: 4 },
      { kind: 'equal_length', a: 1, b: 2, c: 3, d: 4 },
    ])
    expect(satisfied(sketches[0]!)).toBe(true)
    const tiny = remapSketchConstraints(before, [{ ...piece, points: piece.points.slice(0, 2) }])
    expect(tiny.sketches[0]!.constraints).toBeUndefined()
    expect(tiny.dropped).toHaveLength(5)
  })

  it('trim, extend and offset keep constraints valid through the geometry kernel', async () => {
    const { trimSketch, extendSketch, offsetSketch } = await import('../src/app/directSketchGeometry')
    const cuts = [3, 7].map((x, i): DirectSketch => ({
      id: `cut${i}`,
      name: 'Cut',
      closed: false,
      points: [
        [x, -1],
        [x, 1],
      ],
    }))
    const trimmed = trimSketch(
      square([
        { kind: 'horizontal', a: 0, b: 1 },
        { kind: 'vertical', a: 1, b: 2 },
      ]),
      0,
      0.5,
      cuts,
    )
    expect(trimmed).toHaveLength(1)
    expect(trimmed[0]!.constraints).toEqual([{ kind: 'vertical', a: 1, b: 2 }])

    const open: DirectSketch = {
      id: 'open',
      name: 'Open',
      closed: false,
      points: [
        [0, 0],
        [0, 2],
        [2, 2],
      ],
      constraints: [
        { kind: 'vertical', a: 0, b: 1 },
        { kind: 'equal_length', a: 0, b: 1, c: 1, d: 2 },
      ],
    }
    const wall: DirectSketch = {
      id: 'wall',
      name: 'Wall',
      closed: false,
      points: [
        [5, -10],
        [5, 10],
      ],
    }
    const extended = extendSketch(open, 'end', [wall])
    expect(extended.points[2]![0]).toBeCloseTo(5, 6)
    expect(satisfied(extended)).toBe(true)
    expect(extended.constraints).toEqual(open.constraints)

    const rect = square([{ kind: 'equal_length', a: 0, b: 1, c: 1, d: 2 }])
    const offset = offsetSketch(rect, 1)
    expect(offset.constraints).toBeDefined()
    expect(satisfied(offset)).toBe(true)
  })

  it('extend holds length dimensions with constraints, offset refuses a dimensioned sketch', async () => {
    const { extendSketch, offsetSketch, carrySketchConstraints } = await import('../src/app/directSketchGeometry')
    const open: DirectSketch = {
      id: 'open',
      name: 'Open',
      closed: false,
      points: [
        [0, 0],
        [0, 2],
        [2, 2],
      ],
      constraints: [{ kind: 'vertical', a: 0, b: 1 }],
      dimensions: [{ kind: 'length', a: 0, b: 1 }],
    }
    const wall: DirectSketch = {
      id: 'wall',
      name: 'Wall',
      closed: false,
      points: [
        [5, -10],
        [5, 10],
      ],
    }
    const extended = extendSketch(open, 'end', [wall])
    expect(extended.points[2]![0]).toBeCloseTo(5, 6)
    expect(satisfied(extended)).toBe(true)
    const len = Math.hypot(
      extended.points[1]![0] - extended.points[0]![0],
      extended.points[1]![1] - extended.points[0]![1],
    )
    expect(len).toBeCloseTo(2, 5)
    // Without the dimension the same vertex drag stretches the constrained segment; with it the length holds.
    const dragged = { ...open, points: open.points.map((p, i) => (i === 1 ? [0.5, 5] : p)) as Point2[] }
    const held = carrySketchConstraints(open, dragged)
    expect(held.points[1]![1]).toBeCloseTo(5, 6)
    expect(Math.hypot(held.points[1]![0] - held.points[0]![0], held.points[1]![1] - held.points[0]![1])).toBeCloseTo(
      2,
      5,
    )
    expect(satisfied(held)).toBe(true)
    const pinned = { ...dragged, points: dragged.points.map((p, i) => (i === 0 ? [0, -4] : p)) as Point2[] }
    expect(() => carrySketchConstraints(open, pinned)).toThrow(/conflict/)
    const rect = {
      ...square([{ kind: 'equal_length', a: 0, b: 1, c: 1, d: 2 }]),
      dimensions: [{ kind: 'length' as const, a: 0, b: 1 }],
    }
    expect(() => offsetSketch(rect, 1)).toThrow(/dimensions/)
  })

  it('trim reports dropped constraints and the notice names them in both locales', async () => {
    const { trimSketchConstraints } = await import('../src/app/directSketchGeometry')
    const { droppedConstraintsNotice } = await import('../src/app/sketchConstraints')
    const cuts = [3, 7].map((x, i): DirectSketch => ({
      id: `cut${i}`,
      name: 'Cut',
      closed: false,
      points: [
        [x, -1],
        [x, 1],
      ],
    }))
    const result = trimSketchConstraints(
      square([
        { kind: 'horizontal', a: 0, b: 1 },
        { kind: 'vertical', a: 1, b: 2 },
      ]),
      0,
      0.5,
      cuts,
    )
    expect(result.dropped).toEqual([{ kind: 'horizontal', a: 0, b: 1 }])
    expect(droppedConstraintsNotice(result.dropped, 'en')).toBe(
      'Trim removed sketch constraints (1): horizontal (1–2).',
    )
    expect(droppedConstraintsNotice(result.dropped, 'ru')).toBe(
      'Обрезка сняла ограничения эскиза (1): горизонталь (1–2).',
    )
    expect(droppedConstraintsNotice([{ kind: 'parallel', a: 0, b: 1, c: 2, d: 3 }], 'en')).toContain(
      'parallel (1–2, 3–4)',
    )
    expect(droppedConstraintsNotice([], 'ru')).toBe('')
  })
})
