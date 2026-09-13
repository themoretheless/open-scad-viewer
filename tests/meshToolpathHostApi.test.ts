import { describe, expect, it } from 'vitest'
import {
  emitPolygonMeshGcode,
  planPolygonMeshToolpaths,
  parseGcodePreview,
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

describe('mesh section / G-code host API', () => {
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
    expect(gcode.dialect).toBe('open-scad-viewer/print-preview 2')
    expect(gcode.preview.layers).toBe(gcode.layerCount)
    expect(parseGcodePreview(gcode.gcode)).toEqual(gcode.preview)
    expect(gcode.preview.depositedVolumeMm3).toBeGreaterThan(0)
    expect(gcode.preview.estimatedTimeS).toBeGreaterThan(0)
    expect(gcode.preview.moves.some(move => move.extruded)).toBe(true)
    expect(gcode.preview.bounds).not.toBeNull()
  })

  it('uses an indexed half-open layer schedule with no extra floating-point layer', () => {
    const result = emitPolygonMeshGcode(boxMesh(), 0, 2, { layerHeightMm: 0.2 })
    expect(result.layerCount).toBe(10)
    expect(result.preview.bounds!.max[2]).toBe(1.8)
    expect([...new Set(result.preview.moves.map(move => move.layerIndex))]).toEqual(Array.from({ length: 10 }, (_, i) => i))
  })

  it('rejects malformed settings and empty export rather than returning a plausible file', () => {
    for (const settings of [{ wallCount: 1.5 }, { wallCount: -1 }, { layerHeightMm: 0 }, { feedrateMmS: 'fast' }, { lineWidthMm: null }, { filamentDiameterMm: 1e-300 }]) {
      expect(() => emitPolygonMeshGcode(boxMesh(), 0, 1, settings as never)).toThrow()
    }
    expect(() => emitPolygonMeshGcode(boxMesh(), 20, 21)).toThrow(/No toolpaths/)
    expect(() => planPolygonMeshToolpaths(boxMesh(), 20, 21, { layerHeightMm: 1e-300 })).toThrow()
  })

  it('does not let untyped settings overwrite the requested operation or geometry', () => {
    const settings = { op: 'mesh_section', mesh: {}, zMin: 100, zMax: 100, layerHeightMm: 0.5 }
    expect(emitPolygonMeshGcode(boxMesh(), 0, 1, settings).layerCount).toBe(2)
  })

  it('rejects corrupted or unsupported G-code through the real WASM boundary', () => {
    const result = emitPolygonMeshGcode(boxMesh(), 0, 1)
    for (const suffix of ['G91', 'M83', 'G2 X1 Y1 I1', 'G1 XNaN', 'G1 ☃1']) {
      expect(() => parseGcodePreview(`${result.gcode}\n${suffix}\n`)).toThrow()
    }
  })
})
