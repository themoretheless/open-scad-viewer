import { describe, expect, it } from 'vitest'
import { applySourceSplice } from '../src/services/sourceSplice'

describe('source splice transactions', () => {
  it('applies and reverses Unicode-safe replacements byte-exactly', () => {
    const source = 'label = "куб";\nsize = 10;'
    const from = source.indexOf('10')
    const applied = applySourceSplice(source, { from, to: from + 2, insert: '25.5', expected: '10', origin: 'customizer' })
    expect(applied.source).toBe('label = "куб";\nsize = 25.5;')
    expect(applied.selection).toEqual([from, from + 4])
    expect(applySourceSplice(applied.source, applied.inverse).source).toBe(source)
  })

  it('rejects stale and out-of-bounds edits without changing source', () => {
    expect(() => applySourceSplice('cube(1);', { from: 5, to: 6, insert: '2', expected: '9', origin: 'quick-fix' })).toThrow('stale')
    expect(() => applySourceSplice('cube(1);', { from: -1, to: 2, insert: '', expected: '', origin: 'refactor' })).toThrow(RangeError)
  })
})
