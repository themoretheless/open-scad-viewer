import { expect, it } from 'vitest'
import { applyDirectDocumentDelta, directDocumentDelta } from '../src/kernel/directDocumentDelta'
import { stringifyMeshJson } from '../src/exchange/meshJson'
import type { DirectDocument } from '../src/app/directModeling'

const body = (id: string, x: number) => ({ id, name: id, mesh: { positions: [x, 0, 0, 1, 0, 0, 0, 1, 0], indices: [0, 1, 2] } })
const base = (): DirectDocument =>
  ({ version: 1, sketches: [], bodies: [body('a', 0), body('b', 5), body('c', 9)], curves: [], surfaces: [], units: 'mm' }) as unknown as DirectDocument

it('sends unchanged items as ids and rebuilds the identical document sharing base objects', () => {
  const before = base()
  const next = { ...structuredClone(before), bodies: [body('a', 0), body('b', 7), body('d', 1)], blenderProjectId: 'p' } as DirectDocument
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
  expect(stringifyMeshJson(applyDirectDocumentDelta(before, directDocumentDelta(before, next)))).toBe(stringifyMeshJson(next))
})

it('refuses malformed deltas and unknown ids', () => {
  const before = base()
  expect(applyDirectDocumentDelta(before, { delta: 1, keys: ['bodies'], fields: {}, collections: { bodies: ['zzz'] } })).toBeUndefined()
  expect(applyDirectDocumentDelta(before, { delta: 1, keys: ['__proto__'], fields: {}, collections: {} })).toBeUndefined()
  expect(applyDirectDocumentDelta(before, { delta: 1, keys: ['missing'], fields: {}, collections: {} })).toBeUndefined()
  expect(applyDirectDocumentDelta(before, { delta: 1, keys: ['bodies'], fields: {}, collections: { bodies: [3] } })).toBeUndefined()
  expect(applyDirectDocumentDelta(before, { version: 1 })).toBeUndefined()
})
