import { describe, it, expect } from 'vitest'
import {
  buildFlatBvh,
  referenceBatchedBvhRaycast,
  runGpuBatchedBvhRaycast,
  buildMarqueeRayBatch,
  BVH_RAYCAST_WGSL,
  type AabbItem,
} from '../src/services/gpuBvhRaycast'
import {
  buildSceneAabbIndex,
  querySceneAabbIndex,
  disposeSceneAabbIndex,
  type SceneAabbItem,
} from '../src/services/sceneAabbIndex'

describe('WebGPU Compute Batched BVH Raycaster', () => {
  it('builds a flat linear BVH and matches single-ray CPU BVH results across a 3D grid', () => {
    const items: AabbItem[] = []
    const sceneItems: SceneAabbItem[] = []
    let numId = 0
    for (let x = 0; x < 6; x++) {
      for (let y = 0; y < 6; y++) {
        for (let z = 0; z < 4; z++) {
          const min: [number, number, number] = [x * 3, y * 3, z * 3]
          const max: [number, number, number] = [x * 3 + 1.5, y * 3 + 1.5, z * 3 + 1.5]
          items.push({
            id: `box-${String(numId).padStart(4, '0')}`,
            min,
            max,
          })
          sceneItems.push({
            id: numId,
            bounds: { min, max },
          })
          numId++
        }
      }
    }

    const flatBvh = buildFlatBvh(items)
    const cpuIndex = buildSceneAabbIndex(sceneItems)

    expect(flatBvh.nodeCount).toBe(items.length * 2 - 1)
    expect(flatBvh.idTable.length).toBe(items.length)

    const rays = [
      { origin: [0.75, 0.75, -10] as [number, number, number], direction: [0, 0, 1] as [number, number, number] },
      { origin: [9.75, 12.75, 50] as [number, number, number], direction: [0, 0, -1] as [number, number, number] },
      { origin: [-10, 6.75, 3.75] as [number, number, number], direction: [1, 0, 0] as [number, number, number] },
      { origin: [100, 100, 100] as [number, number, number], direction: [0, 1, 0] as [number, number, number] },
    ]

    const batchedHits = referenceBatchedBvhRaycast(flatBvh, rays)
    for (let i = 0; i < rays.length; i++) {
      const candidates = querySceneAabbIndex(cpuIndex, rays[i]!)
      if (candidates.length === 0) {
        expect(batchedHits[i]).toBeNull()
      } else {
        const closest = candidates[0]!
        expect(batchedHits[i]?.id).toBe(`box-${String(closest.id).padStart(4, '0')}`)
        expect(batchedHits[i]?.tNear).toBeCloseTo(closest.distance, 4)
      }
    }
    disposeSceneAabbIndex(cpuIndex)
  })


  it('breaks ties deterministically by lexicographical item id', () => {
    const items: AabbItem[] = [
      { id: 'zeta', min: [0, 0, 5], max: [2, 2, 7] },
      { id: 'alpha', min: [0, 0, 5], max: [2, 2, 7] },
      { id: 'beta', min: [0, 0, 5], max: [2, 2, 7] },
    ]
    const flatBvh = buildFlatBvh(items)
    const hits = referenceBatchedBvhRaycast(flatBvh, [
      { origin: [1, 1, 0], direction: [0, 0, 1] },
    ])
    expect(hits[0]).toEqual({ id: 'alpha', tNear: 5 })
  })

  it('evaluates marquee frustum ray batches via runGpuBatchedBvhRaycast', async () => {
    const items: AabbItem[] = [
      { id: 'left', min: [-3, -1, 4], max: [-1, 1, 6] },
      { id: 'center', min: [-0.8, -0.8, 3], max: [0.8, 0.8, 5] },
      { id: 'right', min: [1, -1, 4], max: [3, 1, 6] },
    ]
    const bvh = buildFlatBvh(items)
    const marqueeRays = buildMarqueeRayBatch(
      [0, 0, 0],
      [0, 0, 1],
      [1, 0, 0],
      [0, 1, 0],
      5,
      5,
      0.6,
      0.3
    )

    expect(marqueeRays.length).toBe(25)
    const hits = await runGpuBatchedBvhRaycast(bvh, marqueeRays)
    expect(hits.length).toBe(25)

    const pickedIds = new Set(hits.filter((h): h is NonNullable<typeof h> => h !== null).map((h) => h.id))
    expect(pickedIds.has('left')).toBe(true)
    expect(pickedIds.has('center')).toBe(true)
    expect(pickedIds.has('right')).toBe(true)
  })

  it('declares valid WGSL compute shader with ordered child stack traversal', () => {
    expect(BVH_RAYCAST_WGSL).toContain('@compute @workgroup_size(WG)')
    expect(BVH_RAYCAST_WGSL).toContain('slab_test')
    expect(BVH_RAYCAST_WGSL).toContain('var stack: array<u32, 32>')
  })
})
