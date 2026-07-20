import { describe, expect, it } from 'vitest'
import { sortTransparentBackToFront } from '../src/services/transparentOrdering'

describe('transparent object ordering', () => {
  it('sorts by camera depth rather than source order or radial distance', () => {
    const objects = [
      { index: 0, center: [0, 0, -3] as const },
      { index: 1, center: [100, 0, -4] as const },
      { index: 2, center: [0, 0, -9] as const },
    ]

    expect(sortTransparentBackToFront(objects, [0, 0, 0], [0, 0, -1])).toEqual([2, 1, 0])
    expect(objects.map(object => object.index)).toEqual([0, 1, 2])
  })

  it('tracks a rotated camera and has a stable source-index tie break', () => {
    const objects = [
      { index: 8, center: [3, 0, 0] as const },
      { index: 4, center: [8, 0, 0] as const },
      { index: 2, center: [3, 0, 0] as const },
    ]

    expect(sortTransparentBackToFront(objects, [0, 0, 0], [1, 0, 0])).toEqual([4, 2, 8])
  })
})
