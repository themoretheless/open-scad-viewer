import {expect, it} from 'vitest'
import {fitsClonedBufferBudget} from '../src/services/clonedBufferBudget'

it('counts the buffers actually cloned, deduplicating aliased views', () => {
  const buffer = new ArrayBuffer(1024 * 1024)
  const views = [new Float32Array(buffer, 0, 6), new Uint32Array(buffer, 24, 3)]
  const cloned = structuredClone(views)
  expect(cloned[0].buffer).toBe(cloned[1].buffer)
  expect(cloned[0].buffer).not.toBe(buffer)
  expect(cloned[0].buffer.byteLength).toBe(buffer.byteLength)
  expect(fitsClonedBufferBudget(views, buffer.byteLength)).toBe(true)
  expect(fitsClonedBufferBudget(views, buffer.byteLength - 1)).toBe(false)
  expect(fitsClonedBufferBudget([...views, new Uint8Array(1)], buffer.byteLength)).toBe(false)
})

it('refuses shared memory and invalid budgets', () => {
  expect(fitsClonedBufferBudget([new Uint8Array(new SharedArrayBuffer(1))], 100)).toBe(false)
  for (const limit of [-1, NaN, Infinity, 0.5]) expect(fitsClonedBufferBudget([], limit)).toBe(false)
  expect(fitsClonedBufferBudget([], 0)).toBe(true)
})
