import { describe, expect, it } from 'vitest'
import { contoursExtrusion, contoursSvg, meshSvgContours, svgContours, type SvgContours, type SvgProjectionMesh } from '../src/services/svgGeometry'
import { identity } from '../src/services/math3d'

function triangle(): SvgProjectionMesh {
  return { vertices: new Float32Array([0, 0, 0, 0, 0, 1, 10, 0, 0, 0, 0, 1, 0, 10, 0, 0, 0, 1]),
    indices: new Uint32Array([0, 1, 2]), transform: identity(), faceIds: new Uint32Array([0]) }
}
const area = (rings: SvgContours) => Math.abs(rings.reduce((sum, ring) => sum + ring.reduce((total, a, i) => {
  const b = ring[(i + 1) % ring.length]
  return total + (a[0] * b[1] - a[1] * b[0]) / 2
}, 0), 0))

describe('SVG production export admission', () => {
  it('rejects corrupt geometry instead of silently omitting triangles', async () => {
    for (const change of [
      (mesh: SvgProjectionMesh) => { mesh.vertices[0] = NaN },
      (mesh: SvgProjectionMesh) => { mesh.indices[1] = 200 },
      (mesh: SvgProjectionMesh) => { mesh.transform[3] = Infinity },
      (mesh: SvgProjectionMesh) => { mesh.transform[15] = 0 },
      (mesh: SvgProjectionMesh) => { mesh.indices = new Uint32Array([0, 1]) },
      (mesh: SvgProjectionMesh) => { mesh.faceIds = new Uint32Array([0, 1]) },
    ]) {
      const corrupt = triangle()
      change(corrupt)
      await expect(meshSvgContours([triangle(), corrupt])).rejects.toThrow(/SVG projection/)
    }
  })

  it('exports a small selected face from a model exceeding the projection triangle budget', async () => {
    const large = triangle()
    large.indices = new Uint32Array(Array.from({ length: 20001 }, () => [0, 1, 2]).flat())
    large.faceIds = new Uint32Array(20001).fill(9)
    large.faceIds[0] = 3
    await expect(meshSvgContours([large])).rejects.toThrow('20000 triangles')
    expect(area(await meshSvgContours([large], { face: { meshIndex: 0, triangleIndex: 0 } }))).toBeCloseTo(50, 5)
    expect(area(await meshSvgContours([large, triangle()], { face: { meshIndex: 1, triangleIndex: 0 } }))).toBeCloseTo(50, 5)
  })

  it('applies mirrored nonuniform transforms and rejects invalid projection selectors', async () => {
    const mesh = triangle()
    mesh.transform[0] = -2
    mesh.transform[5] = 3
    mesh.transform[3] = 40
    mesh.transform[7] = -20
    expect(area(await meshSvgContours([mesh]))).toBeCloseTo(300, 5)
    await expect(meshSvgContours([mesh], { axis: 'w' as 'x' })).rejects.toThrow('axis')
    await expect(meshSvgContours([mesh], { face: { meshIndex: 0.5, triangleIndex: 0 } })).rejects.toThrow('valid planar face')
    await expect(meshSvgContours([mesh], { face: { meshIndex: 0, triangleIndex: NaN } })).rejects.toThrow('valid planar face')
  })

  it('rejects source that the target model cannot compile before insertion', () => {
    const ring: [number, number][] = Array.from({ length: 10000 }, (_, i) => [Math.cos(i * Math.PI / 5000) * 10, Math.sin(i * Math.PI / 5000) * 10])
    expect(() => contoursExtrusion([ring], 5)).toThrow('250000-character model limit')
    expect(() => contoursSvg([ring])).not.toThrow()
    expect(() => contoursExtrusion([[[0, 0], [1, 1]]], 2)).toThrow('finite area')
    expect(() => contoursSvg([[[0, 0], [1, 1], [2, 2]]])).toThrow('finite area')
    expect(() => contoursSvg([[[-1e308, 0], [1e308, 0], [1e308, 1]]])).toThrow('finite area')
  })

  it('retains small edges and holes when exporting a distant world-space profile', async () => {
    const offset = 100_000_000
    const rings: SvgContours = [
      [[0, 0], [40, 0], [40, 30], [0, 30]],
      [[10, 10], [10, 20], [30, 20], [30, 10]],
    ].map(ring => ring.map(([x, y]) => [x + offset, y - offset]))
    const svg = contoursSvg(rings)
    expect(svg).toContain('viewBox="0 0 40 30"')
    expect(svg).not.toContain('100000000')
    const imported = await svgContours(svg)
    expect(imported).toHaveLength(2)
    expect(area(imported)).toBeCloseTo(1000, 6)
  })
})
