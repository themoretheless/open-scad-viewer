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

describe('reusable transparency ordering', () => {
  it('matches reference sorting across camera, visibility, and center changes', async () => {
    const { TransparentSortBuffer } = await import('../src/services/transparentOrdering')
    const buffer = new TransparentSortBuffer()
    const objects = Array.from({ length: 200 }, (_, index) => ({ index, center: [Math.sin(index) * 10, Math.cos(index) * 10, index % 7] as [number, number, number] }))
    for (let frame = 0; frame < 20; frame++) {
      const visible = objects.filter(o => (o.index + frame) % 3 !== 0)
      objects[0].center[2] = frame
      const eye: [number, number, number] = [Math.sin(frame) * 30, Math.cos(frame) * 30, 20]
      buffer.begin()
      visible.forEach(o => buffer.add(o.index, o.center))
      expect(buffer.sort(eye, 0, 0, 0).map(o => o.index)).toEqual(sortTransparentBackToFront(visible, eye, [0, 0, 0]))
    }
    buffer.clear()
    expect(buffer.sort([0, 0, 0], 0, 0, 0)).toEqual([])
  })

  it('reuses records while preserving deterministic ties and fallback direction', async () => {
    const { TransparentSortBuffer } = await import('../src/services/transparentOrdering')
    const buffer = new TransparentSortBuffer()
    const center = [0, 0, -5] as const
    buffer.add(9, center); buffer.add(2, center)
    const first = [...buffer.sort([0, 0, 0], 0, 0, 0)]
    expect(first.map(o => o.index)).toEqual([2, 9])
    buffer.begin(); buffer.add(9, center); buffer.add(2, center)
    const second = buffer.sort([1, 2, 3], 0, 0, 0)
    expect(second[0]).toBe(first[0]); expect(second[1]).toBe(first[1])
  })
})
