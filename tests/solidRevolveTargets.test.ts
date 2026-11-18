import { expect, it } from 'vitest'
import { warmGeometryKernel } from '../src/kernel/geometry/kernel'
import { applySolidRevolve, type SolidRevolveOptions } from '../src/kernel/solidRevolve'
import type { DirectDocument } from '../src/app/directModeling'
import { inspectPolygonMesh } from '../src/kernel/geometry/polygon'
import { createBoxMesh, transformMesh } from '../src/kernel/meshEditing'

const base: SolidRevolveOptions = {
  sketchId: '',
  geometry: 'faceted',
  operation: 'new',
  targetId: '',
  id: '',
  name: 'Ring',
  tessellation: 4,
  axis: 'y',
  offset: 0,
  angle: 360,
  segments: 12,
}
const rect = (id: string, x0: number, x1: number, y0: number, y1: number) => ({
  id,
  name: id,
  closed: true,
  points: [
    [x0, y0],
    [x1, y0],
    [x1, y1],
    [x0, y1],
  ] as [number, number][],
})

/** Two mesh slabs around the revolve axis: A spans y 0..4, B spans y -4..8. */
async function twoSlabs(): Promise<DirectDocument> {
  await warmGeometryKernel()
  const slab = (id: string, y0: number, h: number) => ({
    id,
    name: id,
    mesh: transformMesh(createBoxMesh([20, h, 20]), [-10, y0, -10], 0, 1),
  })
  return { version: 1, bodies: [slab('A', 0, 4), slab('B', -4, 12)], sketches: [rect('tool', 4, 5, 2, 6)] }
}
const volume = (doc: DirectDocument, id: string) =>
  inspectPolygonMesh(doc.bodies.find(b => b.id === id)!.mesh).signedVolumeMm3

it('intersects one revolve tool with several target bodies in one command', async () => {
  const doc = await twoSlabs(),
    before = structuredClone(doc)
  const result = applySolidRevolve(doc, {
    ...base,
    sketchId: 'tool',
    operation: 'intersection',
    targetId: 'A',
    targetIds: ['B'],
    id: 'unused',
  })
  expect(result.bodies.map(b => b.id)).toEqual(['A', 'B'])
  // A spans y 0..4, so it keeps the tool's 2..4 band; B covers the whole tool (2..6).
  const ring = (h: number) => Math.PI * (25 - 16) * h
  expect(Math.abs(volume(result, 'A') / ring(2) - 1)).toBeLessThan(0.1)
  expect(Math.abs(volume(result, 'B') / ring(4) - 1)).toBeLessThan(0.1)
  expect(doc).toEqual(before)
})
it('matches a single-target intersection and refuses duplicate or missing targets without changes', async () => {
  const doc = await twoSlabs(),
    before = structuredClone(doc)
  const single = applySolidRevolve(doc, {
    ...base,
    sketchId: 'tool',
    operation: 'intersection',
    targetId: 'A',
    id: 'unused',
  })
  expect(single.bodies.map(b => b.id)).toEqual(['A', 'B'])
  expect(single.bodies[1]).toEqual(doc.bodies[1])
  expect(() =>
    applySolidRevolve(doc, {
      ...base,
      sketchId: 'tool',
      operation: 'difference',
      targetId: 'A',
      targetIds: ['A'],
      id: 'x',
    }),
  ).toThrow(/only once/)
  expect(() =>
    applySolidRevolve(doc, {
      ...base,
      sketchId: 'tool',
      operation: 'union',
      targetId: 'A',
      targetIds: ['missing'],
      id: 'x',
    }),
  ).toThrow(/target body/)
  expect(doc).toEqual(before)
})
