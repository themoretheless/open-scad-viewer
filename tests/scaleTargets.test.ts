import { describe, expect, it } from 'vitest'
import { QUALITY_TARGETS } from '../src/core/qualityTargets'
import type { GeometryScene } from '../src/core/scene'
import { geometrySceneTransferables } from '../src/core/scene'
import { buildSceneAabbIndex, querySceneAabbIndex } from '../src/services/sceneAabbIndex'

describe('deterministic scale targets', () => {
  it('indexes and queries 1,000 scene bodies with a bounded binary hierarchy', () => {
    const items = Array.from({ length: QUALITY_TARGETS.sceneEntities }, (_, id) => ({
      id,
      bounds: { min: [id * 2, 0, 0] as [number, number, number], max: [id * 2 + 1, 1, 1] as [number, number, number] },
    }))
    const index = buildSceneAabbIndex(items)
    const hits = querySceneAabbIndex(index, { origin: [-1, 0.5, 0.5], direction: [1, 0, 0] })

    expect(index.itemCount).toBe(QUALITY_TARGETS.sceneEntities)
    expect(index.nodeCount).toBeLessThanOrEqual(2 * QUALITY_TARGETS.sceneEntities - 1)
    expect(hits).toHaveLength(QUALITY_TARGETS.sceneEntities)
    expect(hits[0]).toEqual({ id: 0, distance: 1 })
  })

  it('keeps a 750,000-triangle publication on the zero-copy transfer path', () => {
    const vertices = new Float32Array(18)
    const indices = new Uint32Array(QUALITY_TARGETS.renderTriangles * 3)
    const transform = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1])
    const scene: GeometryScene = {
      version: 1,
      assets: [{ version: 1, id: 'asset:scale-target', vertices, indices }],
      entities: [{
        id: 'entity:scale-target',
        geometryAssetId: 'asset:scale-target',
        color: [1, 1, 1, 1],
        transform,
        inspection: { state: 'absent', version: 1 },
      }],
    }

    const transferables = geometrySceneTransferables(scene)
    expect(transferables).toEqual([vertices.buffer, indices.buffer, transform.buffer])
    expect(indices.byteLength).toBe(QUALITY_TARGETS.renderTriangles * 3 * Uint32Array.BYTES_PER_ELEMENT)
  })
})
