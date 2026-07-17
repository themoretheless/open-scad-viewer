import { describe, expect, it } from 'vitest'
import { identity, translate } from '../src/services/math3d'
import { buildFaceOverlayGeometry, pointOverlayPosition } from '../src/services/meshSelectionOverlay'

const vertices = new Float32Array([
  0, 0, 0, 0, 0, 1,
  1, 0, 0, 0, 0, 1,
  0, 1, 0, 0, 0, 1,
  1, 1, 0, 0, 0, 1,
])
const indices = new Uint32Array([0, 1, 2, 1, 3, 2])

describe('mesh selection overlays', () => {
  it('fills all triangles of a kernel face and removes its internal diagonal', () => {
    const overlay = buildFaceOverlayGeometry(
      vertices,
      indices,
      new Uint32Array([7, 7]),
      translate(identity(), [2, 3, 4]),
      0,
      7,
    )

    expect(Array.from(overlay.triangles)).toEqual([
      2, 3, 4, 3, 3, 4, 2, 4, 4,
      3, 3, 4, 3, 4, 4, 2, 4, 4,
    ])
    expect(overlay.boundaryLines.length).toBe(4 * 2 * 3)
  })

  it('falls back to the picked triangle when a face exceeds the safety limit', () => {
    const overlay = buildFaceOverlayGeometry(vertices, indices, new Uint32Array([1, 1]), identity(), 1, 1, 1)
    expect(Array.from(overlay.triangles)).toEqual([1, 0, 0, 1, 1, 0, 0, 1, 0])
    expect(overlay.boundaryLines.length).toBe(18)
  })

  it('snaps a point overlay to the corner with the largest barycentric weight', () => {
    expect(pointOverlayPosition(vertices, indices, translate(identity(), [4, -2, 8]), 0, [0.1, 0.7, 0.2]))
      .toEqual([5, -2, 8])
    expect(pointOverlayPosition(vertices, indices, identity(), 5, [1, 0, 0])).toBeNull()
  })
})
