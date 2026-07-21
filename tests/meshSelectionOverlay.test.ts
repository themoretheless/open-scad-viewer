import { describe, expect, it } from 'vitest'
import type { MeshProvenanceRun, MeshSourceReference } from '../src/core/mesh'
import { identity, translate } from '../src/services/math3d'
import {
  buildFaceOverlayGeometry,
  buildFaceTriangleIndex,
  buildSourceOverlayGeometry,
  MAX_SOURCE_OVERLAY_TRIANGLES,
  pointOverlayPosition,
} from '../src/services/meshSelectionOverlay'

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

  it('groups triangles by face id in ascending CSR rows', () => {
    const index = buildFaceTriangleIndex(new Uint32Array([3, 5, 3, 9]), 4)
    expect(index).not.toBeNull()
    expect(index!.triangleCount).toBe(4)
    const rowOf = (faceId: number) => Array.from(
      index!.triangles.subarray(index!.offsets[faceId], index!.offsets[faceId + 1]),
    )
    expect(rowOf(3)).toEqual([0, 2])
    expect(rowOf(5)).toEqual([1])
    expect(rowOf(9)).toEqual([3])
    expect(rowOf(4)).toEqual([])
    expect(buildFaceTriangleIndex(new Uint32Array([]), 0)).toBeNull()
    expect(buildFaceTriangleIndex(new Uint32Array([1]), 2)).toBeNull()
  })

  it('produces identical overlay geometry through the CSR face index', () => {
    const gridVertices = new Float32Array([
      0, 0, 0, 0, 0, 1,
      1, 0, 0, 0, 0, 1,
      1, 1, 0, 0, 0, 1,
      0, 1, 0, 0, 0, 1,
      2, 0, 0, 0, 0, 1,
      2, 1, 0, 0, 0, 1,
    ])
    const gridIndices = new Uint32Array([0, 1, 2, 0, 2, 3, 1, 4, 5, 1, 5, 2])
    const gridFaceIds = new Uint32Array([3, 3, 5, 9])
    const transform = translate(identity(), [1, 2, 3])
    const index = buildFaceTriangleIndex(gridFaceIds, 4)
    expect(index).not.toBeNull()

    for (let triangle = 0; triangle < 4; triangle++) {
      const faceId = gridFaceIds[triangle]
      const direct = buildFaceOverlayGeometry(gridVertices, gridIndices, gridFaceIds, transform, triangle, faceId)
      const viaIndex = buildFaceOverlayGeometry(
        gridVertices, gridIndices, gridFaceIds, transform, triangle, faceId, undefined, index,
      )
      expect(Array.from(viaIndex.triangles)).toEqual(Array.from(direct.triangles))
      expect(Array.from(viaIndex.boundaryLines)).toEqual(Array.from(direct.boundaryLines))
      expect(direct.triangles.length).toBeGreaterThan(0)
    }

    // Over-limit faces fall back to the picked triangle on both paths.
    const directCapped = buildFaceOverlayGeometry(gridVertices, gridIndices, gridFaceIds, transform, 1, 3, 1)
    const indexedCapped = buildFaceOverlayGeometry(gridVertices, gridIndices, gridFaceIds, transform, 1, 3, 1, index)
    expect(Array.from(indexedCapped.triangles)).toEqual(Array.from(directCapped.triangles))
    expect(Array.from(indexedCapped.boundaryLines)).toEqual(Array.from(directCapped.boundaryLines))
    expect(directCapped.triangles.length).toBe(9)

    // Unknown face ids fall back to the picked triangle on both paths.
    const directUnknown = buildFaceOverlayGeometry(gridVertices, gridIndices, gridFaceIds, transform, 0, 4)
    const indexedUnknown = buildFaceOverlayGeometry(gridVertices, gridIndices, gridFaceIds, transform, 0, 4, undefined, index)
    expect(Array.from(indexedUnknown.triangles)).toEqual(Array.from(directUnknown.triangles))
    expect(Array.from(indexedUnknown.boundaryLines)).toEqual(Array.from(directUnknown.boundaryLines))
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
