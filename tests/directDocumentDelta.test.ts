import { expect, it } from 'vitest'
import { applyDirectDocumentDelta, directDocumentDelta, directSnapshotDelta } from '../src/kernel/directDocumentDelta'
import { stringifyMeshJson } from '../src/exchange/meshJson'
import type { DirectDocument } from '../src/app/directModeling'

const body = (id: string, x: number) => ({
  id,
  name: id,
  mesh: { positions: [x, 0, 0, 1, 0, 0, 0, 1, 0], indices: [0, 1, 2] },
})
const base = (): DirectDocument =>
  ({
    version: 1,
    sketches: [],
    bodies: [body('a', 0), body('b', 5), body('c', 9)],
    curves: [],
    surfaces: [],
    units: 'mm',
  }) as unknown as DirectDocument

it('sends unchanged items as ids and rebuilds the identical document sharing base objects', () => {
  const before = base()
  const next = {
    ...structuredClone(before),
    bodies: [body('a', 0), body('b', 7), body('d', 1)],
    blenderProjectId: 'p',
  } as DirectDocument
  const delta = directDocumentDelta(before, next)
  expect(delta.collections.bodies).toEqual(['a', body('b', 7), body('d', 1)])
  expect(Object.keys(delta.fields)).toEqual(['blenderProjectId'])
  const rebuilt = applyDirectDocumentDelta(before, structuredClone(delta))!
  expect(stringifyMeshJson(rebuilt)).toBe(stringifyMeshJson(next))
  expect(rebuilt.bodies[0]).toBe(before.bodies[0])
})

it('keeps key order and removed fields exact', () => {
  const before = base()
  const { units: _units, ...rest } = before as unknown as Record<string, unknown>
  const next = rest as unknown as DirectDocument
  expect(stringifyMeshJson(applyDirectDocumentDelta(before, directDocumentDelta(before, next)))).toBe(
    stringifyMeshJson(next),
  )
})

it('refuses malformed deltas and unknown ids', () => {
  const before = base()
  expect(
    applyDirectDocumentDelta(before, { delta: 1, keys: ['bodies'], fields: {}, collections: { bodies: ['zzz'] } }),
  ).toBeUndefined()
  expect(
    applyDirectDocumentDelta(before, { delta: 1, keys: ['__proto__'], fields: {}, collections: {} }),
  ).toBeUndefined()
  expect(applyDirectDocumentDelta(before, { delta: 1, keys: ['missing'], fields: {}, collections: {} })).toBeUndefined()
  expect(
    applyDirectDocumentDelta(before, { delta: 1, keys: ['bodies'], fields: {}, collections: { bodies: [3] } }),
  ).toBeUndefined()
  expect(applyDirectDocumentDelta(before, { version: 1 })).toBeUndefined()
})

it('answers a history restore by snapshot text and resends instances whose source changed', async () => {
  const { createSolidInstance } = await import('../src/kernel/solidInstances')
  const { createBrepBox, tessellateNurbsBrep } = await import('../src/kernel/geometry/brep')
  const { emptyDirectDocument, parseDirectDocument, serializeDirectDocument } =
    await import('../src/app/directModeling')
  const brep = createBrepBox([0, 0, 0], [1, 2, 3])
  const seed = emptyDirectDocument()
  seed.bodies.push(
    { id: 'source', name: 'Source', brep, mesh: tessellateNurbsBrep(brep, 1) },
    body('plain', 3) as never,
  )
  const placement = [
    [1, 0, 0, 10],
    [0, 1, 0, 0],
    [0, 0, 1, 0],
    [0, 0, 0, 1],
  ]
  const linked = parseDirectDocument(
    serializeDirectDocument(createSolidInstance(seed, 'source', 'instance', placement)),
  )
  const baseText = serializeDirectDocument(linked)
  const sent = (delta: { collections: Record<string, unknown[]> }) =>
    delta.collections.bodies!.filter(item => typeof item === 'object').map(item => (item as { id: string }).id)
  // Renaming bodies, the source included: its geometry is unchanged, so the instance is an id.
  const renamed = structuredClone(linked)
  renamed.bodies.find(b => b.id === 'plain')!.name = 'renamed'
  renamed.bodies.find(b => b.id === 'source')!.name = 'renamed source'
  const renamedText = serializeDirectDocument(renamed)
  const next = parseDirectDocument(renamedText)
  const delta = directSnapshotDelta(baseText, renamedText, next)!
  expect(sent(delta)).toEqual(['source', 'plain'])
  expect(stringifyMeshJson(applyDirectDocumentDelta(linked, structuredClone(delta)))).toBe(stringifyMeshJson(next))
  // Growing the source keeps the instance's snapshot text but not its geometry: resent in full.
  const grown = structuredClone(linked)
  const bigger = createBrepBox([0, 0, 0], [4, 2, 3])
  Object.assign(
    grown.bodies.find(b => b.id === 'source')!,
    { brep: bigger, mesh: tessellateNurbsBrep(bigger, 1) },
  )
  const grownText = serializeDirectDocument(grown)
  const grownNext = parseDirectDocument(grownText)
  const grownDelta = directSnapshotDelta(baseText, grownText, grownNext)!
  expect(sent(grownDelta).sort()).toEqual(['instance', 'source'])
  expect(stringifyMeshJson(applyDirectDocumentDelta(linked, structuredClone(grownDelta)))).toBe(
    stringifyMeshJson(grownNext),
  )
  expect(directSnapshotDelta('not json', grownText, grownNext)).toBeUndefined()
})
