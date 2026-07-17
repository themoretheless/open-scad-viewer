import { describe, expect, it } from 'vitest'
import {
  buildMeshBvh,
  meshBvhTransferables,
  raycastMeshBvh,
} from '../src/services/meshBvh'

function vertexBuffer(points: Array<[number, number, number]>): Float32Array {
  return new Float32Array(points.flatMap(([x, y, z]) => [x, y, z, 0, 0, 1]))
}

describe('mesh BVH construction', () => {
  it('returns a transferable empty structure for empty geometry', () => {
    const bvh = buildMeshBvh(new Float32Array(), new Uint32Array())
    expect(bvh.nodeCount).toBe(0)
    expect(bvh.bounds).toHaveLength(0)
    expect(bvh.nodes).toHaveLength(0)
    expect(bvh.triangles).toHaveLength(0)
    expect(meshBvhTransferables(bvh)).toHaveLength(3)
  })

  it('omits invalid and exactly degenerate triangles', () => {
    const vertices = vertexBuffer([
      [0, 0, 0], [1, 0, 0], [0, 1, 0],
      [2, 0, 0], [3, 0, 0], [4, 0, 0],
    ])
    const indices = new Uint32Array([
      0, 1, 2, // valid triangle 0
      3, 4, 5, // collinear triangle 1
      0, 1, 99, // invalid triangle 2
    ])
    const bvh = buildMeshBvh(vertices, indices)
    expect([...bvh.triangles]).toEqual([0])
    expect(bvh.nodeCount).toBe(1)
  })

  it('builds a deterministic balanced hierarchy without mutating inputs', () => {
    const points: Array<[number, number, number]> = []
    const indexValues: number[] = []
    for (let x = 0; x < 25; x++) {
      const first = points.length
      points.push([x, 0, 0], [x + 0.8, 0, 0], [x, 0.8, 0])
      indexValues.push(first, first + 1, first + 2)
    }
    const vertices = vertexBuffer(points)
    const indices = new Uint32Array(indexValues)
    const originalIndices = indices.slice()
    const first = buildMeshBvh(vertices, indices, { leafSize: 2 })
    const second = buildMeshBvh(vertices, indices, { leafSize: 2 })

    expect(first.nodeCount).toBeGreaterThan(1)
    expect(new Set(first.triangles).size).toBe(25)
    expect([...first.triangles].sort((a, b) => a - b)).toEqual([...Array(25).keys()])
    expect(first.nodes).toEqual(second.nodes)
    expect(first.bounds).toEqual(second.bounds)
    expect(first.triangles).toEqual(second.triangles)
    expect(indices).toEqual(originalIndices)
  })
})

describe('mesh BVH raycasting', () => {
  it('returns triangle identity, hit points, barycentrics and face orientation', () => {
    const vertices = vertexBuffer([[0, 0, 0], [1, 0, 0], [0, 1, 0]])
    const indices = new Uint32Array([0, 1, 2])
    const bvh = buildMeshBvh(vertices, indices)
    const hit = raycastMeshBvh(bvh, vertices, indices, {
      origin: [0.25, 0.25, 2],
      direction: [0, 0, -1],
    })

    expect(hit).not.toBeNull()
    expect(hit!.triangleIndex).toBe(0)
    expect(hit!.triangleVertexIndices).toEqual([0, 1, 2])
    expect(hit!.t).toBeCloseTo(2, 12)
    expect(hit!.localPoint).toEqual([0.25, 0.25, 0])
    expect(hit!.worldPoint).toEqual([0.25, 0.25, 0])
    expect(hit!.barycentric[0]).toBeCloseTo(0.5, 12)
    expect(hit!.barycentric[1]).toBeCloseTo(0.25, 12)
    expect(hit!.barycentric[2]).toBeCloseTo(0.25, 12)
    expect(hit!.localNormal).toEqual([0, 0, 1])
    expect(hit!.worldNormal).toEqual([0, 0, 1])
    expect(hit!.frontFace).toBe(true)
  })

  it('traverses front-to-back and returns the nearest original triangle', () => {
    const vertices = vertexBuffer([
      [0, 0, 0], [1, 0, 0], [0, 1, 0],
      [0, 0, 1], [1, 0, 1], [0, 1, 1],
    ])
    // The farther triangle intentionally comes first in the source index list.
    const indices = new Uint32Array([0, 1, 2, 3, 4, 5])
    const bvh = buildMeshBvh(vertices, indices, { leafSize: 1 })
    const hit = raycastMeshBvh(bvh, vertices, indices, {
      origin: [0.2, 0.2, 3],
      direction: [0, 0, -1],
    })
    expect(hit?.triangleIndex).toBe(1)
    expect(hit?.t).toBeCloseTo(2, 12)
  })

  it('honours min/max ray parameters and rejects misses or zero rays', () => {
    const vertices = vertexBuffer([[0, 0, 0], [1, 0, 0], [0, 1, 0]])
    const indices = new Uint32Array([0, 1, 2])
    const bvh = buildMeshBvh(vertices, indices)
    const ray = { origin: [0.2, 0.2, 2] as const, direction: [0, 0, -1] as const }
    expect(raycastMeshBvh(bvh, vertices, indices, ray, { maxT: 1.9 })).toBeNull()
    expect(raycastMeshBvh(bvh, vertices, indices, ray, { minT: 2.1 })).toBeNull()
    expect(raycastMeshBvh(bvh, vertices, indices, {
      origin: [2, 2, 2], direction: [0, 0, -1],
    })).toBeNull()
    expect(raycastMeshBvh(bvh, vertices, indices, {
      origin: [0.2, 0.2, 2], direction: [0, 0, 0],
    })).toBeNull()
  })

  it('preserves world-space t and returns local/world data through an affine transform', () => {
    const vertices = vertexBuffer([[0, 0, 0], [1, 0, 0], [0, 1, 0]])
    const indices = new Uint32Array([0, 1, 2])
    const bvh = buildMeshBvh(vertices, indices)
    // The local mesh is translated to world Z=5; this is inverse(model).
    const localFromWorld = new Float32Array([
      1, 0, 0, 0,
      0, 1, 0, 0,
      0, 0, 1, -5,
      0, 0, 0, 1,
    ])
    const hit = raycastMeshBvh(bvh, vertices, indices, {
      origin: [0.25, 0.25, 10], direction: [0, 0, -1],
    }, { localFromWorld })

    expect(hit?.t).toBeCloseTo(5, 12)
    expect(hit?.localPoint).toEqual([0.25, 0.25, 0])
    expect(hit?.worldPoint).toEqual([0.25, 0.25, 5])
    expect(hit?.worldNormal).toEqual([0, 0, 1])
  })

  it('reports back faces while keeping the geometric normal', () => {
    const vertices = vertexBuffer([[0, 0, 0], [1, 0, 0], [0, 1, 0]])
    const indices = new Uint32Array([0, 1, 2])
    const bvh = buildMeshBvh(vertices, indices)
    const hit = raycastMeshBvh(bvh, vertices, indices, {
      origin: [0.25, 0.25, -2], direction: [0, 0, 1],
    })
    expect(hit?.frontFace).toBe(false)
    expect(hit?.localNormal).toEqual([0, 0, 1])
  })
})

describe('mesh BVH transfer data', () => {
  it('exposes every typed-array backing buffer exactly once', () => {
    const vertices = vertexBuffer([[0, 0, 0], [1, 0, 0], [0, 1, 0]])
    const bvh = buildMeshBvh(vertices, new Uint32Array([0, 1, 2]))
    const buffers = meshBvhTransferables(bvh)
    expect(buffers).toEqual([bvh.bounds.buffer, bvh.nodes.buffer, bvh.triangles.buffer])
    expect(new Set(buffers).size).toBe(3)
  })
})
