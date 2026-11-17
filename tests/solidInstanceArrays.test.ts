import { expect, it } from 'vitest'
import { DirectHistory, emptyDirectDocument, parseDirectDocument } from '../src/app/directModeling'
import { stringifyMeshJson } from '../src/exchange/meshJson'
import { createBrepBox, tessellateNurbsBrep } from '../src/kernel/geometry/brep'
import { createSolidInstanceArray, solidInstanceArrayPlacements } from '../src/kernel/solidInstanceArrays'
import { detachSolidInstances } from '../src/kernel/solidInstances'
import { applySolidSceneEdit, type SolidSceneInstanceArray } from '../src/kernel/solidSceneEdit'

const box = (size: number) => {
  const brep = createBrepBox([0, 0, 0], [size, 2, 3])
  return { id: 'source', name: 'Source', brep, mesh: tessellateNurbsBrep(brep, 1) }
}
const axisRange = (positions: ArrayLike<number>, axis: number) => {
  const values = Array.from(positions).filter((_, i) => i % 3 === axis)
  return [Math.min(...values), Math.max(...values)]
}
const offsets = (matrices: number[][][]) => matrices.map(m => [m[0][3], m[1][3], m[2][3]])

it('spaces step and extent arrays along a direction', () => {
  expect(offsets(solidInstanceArrayPlacements({ kind: 'step', direction: [2, 0, 0], step: 5, count: 3 }))).toEqual([
    [5, 0, 0],
    [10, 0, 0],
  ])
  expect(offsets(solidInstanceArrayPlacements({ kind: 'extent', direction: [0, 1, 0], extent: 30, count: 4 }))).toEqual(
    [
      [0, 10, 0],
      [0, 20, 0],
      [0, 30, 0],
    ],
  )
})

it('spaces path arrays by arc length and aligns them with the tangent', () => {
  const points: [number, number, number][] = [
    [0, 0, 0],
    [10, 0, 0],
    [10, 10, 0],
  ]
  const plain = solidInstanceArrayPlacements({ kind: 'path', points, count: 5 })
  expect(offsets(plain).map(o => o.map(v => Math.round(v * 1e9) / 1e9))).toEqual([
    [5, 0, 0],
    [10, 0, 0],
    [10, 5, 0],
    [10, 10, 0],
  ])
  const aligned = solidInstanceArrayPlacements({ kind: 'path', points, count: 3, align: true })
  // On the second segment the copy is turned 90° about z around the path start, then moved.
  const last = aligned[1]
  expect(last[0][0]).toBeCloseTo(0, 12)
  expect(last[1][0]).toBeCloseTo(1, 12)
  const apply = (p: number[]) =>
    [0, 1, 2].map(r => last[r][0] * p[0] + last[r][1] * p[1] + last[r][2] * p[2] + last[r][3])
  expect(apply([0, 0, 0]).map(v => Math.round(v * 1e9) / 1e9)).toEqual([10, 10, 0])
  expect(apply([1, 0, 0]).map(v => Math.round(v * 1e9) / 1e9)).toEqual([10, 11, 0])
})

it('refuses invalid array input with a coded error', () => {
  expect(() => solidInstanceArrayPlacements({ kind: 'step', direction: [0, 0, 0], step: 1, count: 3 })).toThrow(
    expect.objectContaining({ code: 'DIRECT_PATTERN_ARRAY' }),
  )
  expect(() => solidInstanceArrayPlacements({ kind: 'extent', direction: [1, 0, 0], extent: 1, count: 1 })).toThrow()
  expect(() =>
    solidInstanceArrayPlacements({
      kind: 'path',
      points: [
        [1, 1, 1],
        [1, 1, 1],
      ],
      count: 3,
    }),
  ).toThrow()
  const seed = emptyDirectDocument()
  seed.bodies.push(box(1))
  const placements = solidInstanceArrayPlacements({ kind: 'step', direction: [1, 0, 0], step: 2, count: 3 })
  expect(() => createSolidInstanceArray(seed, 'missing', 'a', placements)).toThrow(
    expect.objectContaining({ code: 'DIRECT_PATTERN_SOURCE' }),
  )
  const linked = createSolidInstanceArray(seed, 'source', 'a', placements)
  expect(() => createSolidInstanceArray(linked, 'source', 'a', placements)).toThrow(
    expect.objectContaining({ code: 'DIRECT_PATTERN_ARRAY' }),
  )
  expect(() => createSolidInstanceArray(linked, 'a1', 'b', placements)).toThrow(
    expect.objectContaining({ code: 'DIRECT_PATTERN_SOURCE' }),
  )
})

it('updates 100 linked array instances from the source across undo, redo and reload, while detached copies keep their shape', () => {
  const seed = emptyDirectDocument()
  seed.bodies.push(box(1))
  const placements = solidInstanceArrayPlacements({ kind: 'step', direction: [1, 0, 0], step: 10, count: 101 })
  const linked = createSolidInstanceArray(seed, 'source', 'copy-', placements)
  expect(linked.bodies).toHaveLength(101)
  expect(axisRange(linked.bodies[100].mesh.positions, 0)).toEqual([1000, 1001])
  const detached = detachSolidInstances(linked, ['copy-1'])
  const history = new DirectHistory(detached)
  const edit = history.document
  edit.bodies[0] = box(4)
  history.commit(edit)
  for (const body of history.document.bodies.slice(2)) {
    expect(body.instance?.sourceId).toBe('source')
    const [min, max] = axisRange(body.mesh.positions, 0)
    expect(max - min).toBeCloseTo(4, 10)
  }
  expect(history.document.bodies[1].instance).toBeUndefined()
  const [min1, max1] = axisRange(history.document.bodies[1].mesh.positions, 0)
  expect(max1 - min1).toBeCloseTo(1, 10)
  const undone = history.undo()
  expect(axisRange(undone.bodies[100].mesh.positions, 0)).toEqual([1000, 1001])
  const redone = history.redo()
  const reloaded = parseDirectDocument(stringifyMeshJson(redone))
  expect(reloaded.bodies[100].instance?.sourceId).toBe('source')
  expect(axisRange(reloaded.bodies[100].mesh.positions, 0)).toEqual([1000, 1004])
})

it('creates step, extent and path arrays through the scene edit, with undo and redo', () => {
  const base = emptyDirectDocument()
  base.bodies.push(box(2))
  base.sketches.push({
    id: 'path',
    name: 'Path',
    closed: false,
    points: [
      [0, 0],
      [10, 0],
      [10, 10],
    ],
  })
  const edit = (array: SolidSceneInstanceArray, axis: 'x' | 'y' | 'z' = 'x') =>
    applySolidSceneEdit(base, {
      operation: 'instance-array',
      id: 'source',
      ids: ['source'],
      createdId: 'arr',
      x: 0,
      y: 0,
      z: 0,
      axis,
      angle: 0,
      scale: 1,
      array,
    })
  const placed = (d: ReturnType<typeof edit>) =>
    d.bodies
      .filter(b => b.instance)
      .map(b => [b.id, ...b.instance!.matrix.slice(0, 3).map(r => Math.round(r[3] * 1e9) / 1e9)])
  expect(placed(edit({ kind: 'step', count: 3, distance: 5 }))).toEqual([
    ['arr:1', 5, 0, 0],
    ['arr:2', 10, 0, 0],
  ])
  expect(placed(edit({ kind: 'extent', count: 3, distance: 30 }, 'y'))).toEqual([
    ['arr:1', 0, 15, 0],
    ['arr:2', 0, 30, 0],
  ])
  const path = edit({ kind: 'path', count: 3, distance: 0, pathId: 'path', align: true })
  expect(path.bodies.find(b => b.id === 'arr:1')!.instance!.matrix.map(r => r[3])).toEqual([10, 0, 0, 1])
  // The last copy is turned a quarter turn: its 2-wide x span becomes a y span.
  const [minX, maxX] = axisRange(path.bodies.find(b => b.id === 'arr:2')!.mesh.positions, 0)
  expect(maxX - minX).toBeCloseTo(2, 9)
  const [minY, maxY] = axisRange(path.bodies.find(b => b.id === 'arr:2')!.mesh.positions, 1)
  expect(maxY - minY).toBeCloseTo(2, 9)
  expect(minY).toBeCloseTo(10, 9)
  expect(() => edit({ kind: 'path', count: 3, distance: 0 })).toThrow()
  expect(() => edit({ kind: 'step', count: 1, distance: 5 })).toThrow()
  const history = new DirectHistory(base)
  history.commit(edit({ kind: 'step', count: 4, distance: 5 }))
  expect(history.document.bodies).toHaveLength(4)
  expect(history.undo().bodies).toHaveLength(1)
  expect(history.redo().bodies.filter(b => b.instance?.sourceId === 'source')).toHaveLength(3)
})
