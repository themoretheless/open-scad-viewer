import { describe, expect, it } from 'vitest'
import { selectDisplayRepresentation } from '../src/services/geometry/displayPolicy'

const base = {
  projectedPixels: 400,
  targetErrorPixels: 2,
  admittedBytes: 100,
  budgetBytes: 10_000,
  exactAvailable: true,
  previewBytes: 1_000,
  exactBytes: 5_000,
}

describe('display representation policy', () => {
  it('uses a bounding box when screen error does not justify a mesh', () => {
    expect(selectDisplayRepresentation({ ...base, projectedPixels: 1 })).toBe('bounding-box')
  })

  it('refuses refinement when the preview does not fit the remaining budget', () => {
    expect(selectDisplayRepresentation({ ...base, admittedBytes: 9_500 })).toBe('bounding-box')
  })

  it('promotes to exact only when it is available and fits', () => {
    expect(selectDisplayRepresentation(base)).toBe('exact')
    expect(selectDisplayRepresentation({ ...base, exactBytes: 20_000 })).toBe('preview')
    expect(selectDisplayRepresentation({ ...base, exactAvailable: false })).toBe('preview')
  })

  it('rejects non-finite and inconsistent budgets deterministically', () => {
    expect(selectDisplayRepresentation({ ...base, projectedPixels: Number.NaN })).toBe('bounding-box')
    expect(selectDisplayRepresentation({ ...base, admittedBytes: 20_000 })).toBe('bounding-box')
  })
})
