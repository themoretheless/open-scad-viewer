import { it, expect, beforeAll } from 'vitest'
import { circleNurbsCurve } from '../src/kernel/nurbsConstructors'
import { applySolidSweepBody } from '../src/kernel/solidSweepBody'
import { buildSolidSurface } from '../src/kernel/solidSurfaceConstruction'
import * as geometryKernel from '../src/kernel/geometry/kernel'
import type { DirectDocument } from '../src/app/directModeling'

beforeAll(async () => {
  await geometryKernel.warmGeometryKernel()
})
const line = (a: number[], b: number[]) => ({ degree: 1, controlPoints: [a, b], weights: [1, 1], knots: [0, 0, 1, 1] })
const volume = (mesh: { positions: ArrayLike<number>; indices: ArrayLike<number> }) => {
  let v = 0
  const p = mesh.positions,
    ix = mesh.indices
  for (let i = 0; i < ix.length; i += 3) {
    const a = ix[i] * 3,
      b = ix[i + 1] * 3,
      c = ix[i + 2] * 3
    v +=
      p[a] * (p[b + 1] * p[c + 2] - p[b + 2] * p[c + 1]) -
      p[a + 1] * (p[b] * p[c + 2] - p[b + 2] * p[c]) +
      p[a + 2] * (p[b] * p[c + 1] - p[b + 1] * p[c])
  }
  return Math.abs(v / 6)
}
const doc = (): DirectDocument =>
  ({
    version: 1,
    sketches: [],
    bodies: [],
    curves: [
      { id: 'big', name: 'Big profile', curve: circleNurbsCurve([0, 0, 0], [0, 0, 1], 3) },
      { id: 'far', name: 'Far profile', curve: circleNurbsCurve([20, 0, 0], [0, 0, 1], 3) },
      { id: 'small', name: 'Small profile', curve: circleNurbsCurve([0, 0, -1], [0, 0, 1], 1) },
      { id: 'path', name: 'Path', curve: line([0, 0, 0], [0, 0, 10]) },
      { id: 'farPath', name: 'Far path', curve: line([20, 0, 0], [20, 0, 10]) },
      { id: 'cutPath', name: 'Cut path', curve: line([0, 0, -1], [0, 0, 11]) },
      {
        id: 'arcPath',
        name: 'Arc path',
        curve: {
          degree: 2,
          controlPoints: [
            [0, 0, 0],
            [0, 0, 5],
            [5, 0, 5],
          ],
          weights: [1, 1, 1],
          knots: [0, 0, 0, 1, 1, 1],
        },
      },
    ],
  }) as DirectDocument

it('C9: sweep builds a capped solid and cuts several targets in one command', () => {
  let d = applySolidSweepBody(doc(), {
    profileId: 'big',
    pathId: 'path',
    operation: 'new',
    targetId: '',
    id: 'a',
    name: 'A',
  })
  d = applySolidSweepBody(d, { profileId: 'big', pathId: 'path', operation: 'new', targetId: '', id: 'b', name: 'B' })
  expect(d.bodies.map(b => b.id)).toEqual(['a', 'b'])
  const full = volume(d.bodies[0].mesh)
  expect(full).toBeGreaterThan(Math.PI * 9 * 10 * 0.9)
  const cut = applySolidSweepBody(d, {
    profileId: 'small',
    pathId: 'cutPath',
    operation: 'difference',
    targetId: 'a',
    targetIds: ['b'],
    id: 'tool',
    name: 'Tool',
  })
  expect(cut.bodies.map(b => b.id)).toEqual(['a', 'b'])
  for (const body of cut.bodies) {
    expect(body.brep).toBeTruthy()
    expect(volume(body.mesh)).toBeLessThan(full - Math.PI * 10 * 0.8)
  }
  expect(d.bodies[0].mesh).toEqual(
    applySolidSweepBody(doc(), { profileId: 'big', pathId: 'path', operation: 'new', targetId: '', id: 'a', name: 'A' })
      .bodies[0].mesh,
  )
})

it('C9: an empty sweep intersection removes only that target; refusals leave the document intact', () => {
  let d = applySolidSweepBody(doc(), {
    profileId: 'big',
    pathId: 'path',
    operation: 'new',
    targetId: '',
    id: 'a',
    name: 'A',
  })
  d = applySolidSweepBody(d, {
    profileId: 'far',
    pathId: 'farPath',
    operation: 'new',
    targetId: '',
    id: 'farBody',
    name: 'Far',
  })
  const r = applySolidSweepBody(d, {
    profileId: 'small',
    pathId: 'cutPath',
    operation: 'intersection',
    targetId: 'a',
    targetIds: ['farBody'],
    id: 't',
    name: 'T',
  })
  expect(r.bodies.map(b => b.id)).toEqual(['a'])
  expect(() =>
    applySolidSweepBody(d, {
      profileId: 'small',
      pathId: 'arcPath',
      operation: 'union',
      targetId: 'a',
      id: 't',
      name: 'T',
    }),
  ).toThrow(/plane normal to the path/)
  expect(() =>
    applySolidSweepBody(d, {
      profileId: 'small',
      pathId: 'cutPath',
      operation: 'union',
      targetId: 'a',
      targetIds: ['a'],
      id: 't',
      name: 'T',
    }),
  ).toThrow(/only once/)
})

it('C9: the sweep surface builder routes a body result step instead of adding a surface', () => {
  const r = buildSolidSurface(doc(), {
    kind: 'nurbs-sweep',
    ids: ['big', 'path'],
    id: 'new-body',
    reversed: [],
    sweep: { mode: 'translation', normal: [0, 0, 1], sections: 24, maxDeviation: 0.01 },
    body: { operation: 'new', targetId: '', targetIds: [], name: 'Sweep body' },
  })
  expect(r.error).toBe('')
  expect(r.document!.surfaces ?? []).toHaveLength(0)
  expect(r.document!.bodies).toMatchObject([{ id: 'new-body', name: 'Sweep body' }])
})
