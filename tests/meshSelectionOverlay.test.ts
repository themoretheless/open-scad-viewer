import { describe, expect, it } from 'vitest'
import { identity, translate } from '../src/services/math3d'
import {
  buildFaceOverlayGeometry,
  buildSourceOverlayGeometry,
  MAX_SOURCE_OVERLAY_TRIANGLES,
  pointOverlayPosition,
} from '../src/services/meshSelectionOverlay'
import type { MeshProvenanceRun, MeshSourceReference } from '../src/services/openscadParser'

const vertices = new Float32Array([
  0, 0, 0, 0, 0, 1,
  1, 0, 0, 0, 0, 1,
  0, 1, 0, 0, 0, 1,
  1, 1, 0, 0, 0, 1,
])
const indices = new Uint32Array([0, 1, 2, 1, 3, 2])

function source(id: number): MeshSourceReference {
  return { id, originalId: id + 100, start: id, end: id + 8, label: `source-${id}` }
}

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

  it('builds transformed geometry for an exact source id across provenance runs', () => {
    const runs: MeshProvenanceRun[] = [
      { triangleStart: 0, triangleEnd: 1, source: source(12), backside: false },
      { triangleStart: 1, triangleEnd: 2, source: source(12), backside: true },
    ]
    const overlay = buildSourceOverlayGeometry(
      vertices,
      indices,
      runs,
      translate(identity(), [2, 3, 4]),
      12,
    )

    expect(overlay.triangleCount).toBe(2)
    expect(overlay.truncated).toBe(false)
    expect(Array.from(overlay.triangles)).toEqual([
      2, 3, 4, 3, 3, 4, 2, 4, 4,
      3, 3, 4, 3, 4, 4, 2, 4, 4,
    ])
    expect(overlay.boundaryLines.length).toBe(4 * 2 * 3)
  })

  it('uses source.id rather than originalId and ignores unmatched runs', () => {
    const matching = { ...source(41), originalId: 7 }
    const runs: MeshProvenanceRun[] = [
      { triangleStart: 0, triangleEnd: 1, source: matching, backside: false },
      { triangleStart: 1, triangleEnd: 2, source: source(7), backside: false },
    ]

    const overlay = buildSourceOverlayGeometry(vertices, indices, runs, identity(), 41)
    expect(overlay.triangleCount).toBe(1)
    expect(Array.from(overlay.triangles)).toEqual([0, 0, 0, 1, 0, 0, 0, 1, 0])
    expect(buildSourceOverlayGeometry(vertices, indices, runs, identity(), 999).triangleCount).toBe(0)
  })

  it('caps matching triangles before allocating output geometry', () => {
    const repeatedIndices = new Uint32Array((MAX_SOURCE_OVERLAY_TRIANGLES + 1) * 3)
    const runs: MeshProvenanceRun[] = [{
      triangleStart: 0,
      triangleEnd: MAX_SOURCE_OVERLAY_TRIANGLES + 1,
      source: source(5),
      backside: false,
    }]
    const overlay = buildSourceOverlayGeometry(
      vertices.subarray(0, 6),
      repeatedIndices,
      runs,
      identity(),
      5,
      Number.MAX_SAFE_INTEGER,
    )

    expect(overlay.triangleCount).toBe(MAX_SOURCE_OVERLAY_TRIANGLES)
    expect(overlay.triangles.length).toBe(MAX_SOURCE_OVERLAY_TRIANGLES * 9)
    expect(overlay.truncated).toBe(true)
  })
})
