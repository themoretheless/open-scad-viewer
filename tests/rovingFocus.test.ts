import { describe, expect, it } from 'vitest'
import { nextRovingIndex } from '../src/services/rovingFocus'

describe('nextRovingIndex', () => {
  it('wraps horizontal arrow navigation', () => {
    expect(nextRovingIndex(2, 3, 'ArrowRight')).toBe(0)
    expect(nextRovingIndex(0, 3, 'ArrowLeft')).toBe(2)
  })

  it('supports Home and End and guards empty controls', () => {
    expect(nextRovingIndex(1, 3, 'Home')).toBe(0)
    expect(nextRovingIndex(1, 3, 'End')).toBe(2)
    expect(nextRovingIndex(0, 0, 'ArrowRight')).toBe(-1)
  })
})
