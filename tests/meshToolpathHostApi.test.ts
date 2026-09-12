import { describe, expect, it } from 'vitest'
import {
  emitPolygonMeshGcode,
  planPolygonMeshToolpaths,
  sectionPolygonMesh,
  type PolygonMesh,
} from '../src/services/geometry/polygon'

function boxMesh(): PolygonMesh {
  // Unit cube [0,10]^3 as independent triangles (positions xyz, indices).
  const p = [
    [0, 0, 0],
    [10, 0, 0],
    [10, 10, 0],
    [0, 10, 0],
    [0, 0, 10],
    [10, 0, 10],
    [10, 10, 10],
    [0, 10, 10],
  ]
  const faces = [
    [0, 1, 2, 0, 2, 3],
    [4, 6, 5, 4, 7, 6],
    [0, 5, 1, 0, 4, 5],
    [1, 6, 2, 1, 5, 6],
    [2, 7, 3, 2, 6, 7],
    [3, 4, 0, 3, 7, 4],
  ]
  const positions: number[] = []
  const indices: number[] = []
  for (const face of faces) {
    for (let i = 0; i < 6; i += 1) {
      const v = p[face[i]!]!
      const index = positions.length / 3
      positions.push(v[0]!, v[1]!, v[2]!)
      indices.push(index)
    }
  }
  return { positions, indices }
}

describe('P2 mesh section / toolpath host API', () => {
  it('sections a box and plans walls through the WASM bridge', () => {
    const mesh = boxMesh()
    const section = sectionPolygonMesh(mesh, 1)
    expect(section.contours.length).toBeGreaterThan(0)
    const plan = planPolygonMeshToolpaths(mesh, 0, 2, {
      layerHeightMm: 0.5,
      wallCount: 1,
    })
    expect(plan.layers.length).toBeGreaterThan(0)
    const gcode = emitPolygonMeshGcode(mesh, 0, 1, {
      layerHeightMm: 0.5,
      wallCount: 1,
    })
    expect(gcode.layerCount).toBeGreaterThan(0)
    expect(gcode.gcode).toContain('G1')
  })
})
