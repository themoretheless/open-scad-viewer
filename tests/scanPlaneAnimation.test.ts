import { describe, expect, it } from 'vitest'
import { advanceScanPlane } from '../src/features/scanPlaneAnimation'

describe('scanning plane playback', () => {
  it('crosses the whole model in eight seconds and reverses at the upper edge', () => {
    expect(advanceScanPlane(-10, 1, -10, 30, 8_000)).toEqual({ offset: 30, direction: -1 })
    expect(advanceScanPlane(30, -1, -10, 30, 8_000)).toEqual({ offset: -10, direction: 1 })
  })

  it('preserves distance through either edge and across several complete scans', () => {
    expect(advanceScanPlane(9, 1, 0, 10, 2_400)).toEqual({ offset: 8, direction: -1 })
    expect(advanceScanPlane(1, -1, 0, 10, 2_400)).toEqual({ offset: 2, direction: 1 })
    expect(advanceScanPlane(9, 1, 0, 10, 34_400)).toEqual({ offset: 8, direction: -1 })
  })

  it('gives the same position at different frame rates', () => {
    const expected = advanceScanPlane(2, 1, -8, 12, 19_200)
    let state = { offset: 2, direction: 1 as 1 | -1 }
    for (let frame = 0; frame < 600; frame++) state = advanceScanPlane(state.offset, state.direction, -8, 12, 32)
    expect(state.offset).toBeCloseTo(expected.offset, 9)
    expect(state.direction).toBe(expected.direction)
  })

  it('handles a flat model and clamps a starting position outside a replaced model', () => {
    expect(advanceScanPlane(4, -1, 3, 3, 100)).toEqual({ offset: 3, direction: 1 })
    expect(advanceScanPlane(40, 1, -10, 10, 800)).toEqual({ offset: 8, direction: -1 })
  })

  it('rejects invalid ranges or timing before producing a non-finite camera coordinate', () => {
    expect(() => advanceScanPlane(0, 1, 10, 0, 100)).toThrow(RangeError)
    expect(() => advanceScanPlane(0, 1, 0, Infinity, 100)).toThrow(RangeError)
    expect(() => advanceScanPlane(0, 1, 0, 10, 100, 0)).toThrow(RangeError)
  })
})
