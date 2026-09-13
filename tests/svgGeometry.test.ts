import { describe, it, expect } from 'vitest'
import { svgContours, contoursSvg, contoursExtrusion, meshSvgContours, SVG_MAX_BYTES } from '../src/services/svgGeometry'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
const geometry = new HeadlessGeometryService()
const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M0 0H20V20H0Z M5 5H15V15H5Z"/></svg>'
describe('SVG geometry workflows', () => {
  it('extrudes SVG holes with physical units, then roundtrips the silhouette', async () => {
    const contours = await svgContours(svg)
    const source = contoursExtrusion(contours, 4)
    const analysis = await geometry.analyze(source, 'full')
    expect(analysis.volume).toBeCloseTo(1200, 3)
    const built = await geometry.compile(source, 'full')
    const projected = contoursSvg(await meshSvgContours(built.meshes))
    const again = await geometry.analyze(contoursExtrusion(await svgContours(projected), 4), 'full')
    expect(again.volume).toBeCloseTo(1200, 3)
    expect(again.bounds).toEqual(analysis.bounds)
  })
  it('flattens a rotated planar face at true scale', async () => {
    const built = await geometry.compile('rotate([20,30,15]) cube([20,10,4]);', 'full')
    const rings = await meshSvgContours(built.meshes, { face: {meshIndex:0,triangleIndex:0} })
    const result = await geometry.analyze(contoursExtrusion(rings,1), 'full')
    expect([40,80,200].some(area=>Math.abs(result.volume!-area)<0.001)).toBe(true)
  })
  it('supports every projection axis and rejects invalid faces', async () => {
    const built = await geometry.compile('cube([20,10,4]);', 'full')
    for (const [axis, area] of [['x',40],['y',80],['z',200]] as const) {
      const analysis = await geometry.analyze(contoursExtrusion(await meshSvgContours(built.meshes,{axis}),1),'full')
      expect(analysis.volume).toBeCloseTo(area,3)
    }
    await expect(meshSvgContours(built.meshes,{face:{meshIndex:0,triangleIndex:999}})).rejects.toThrow('valid planar face')
  })
  it('keeps all four rounded rectangle corners and straight sides', async () => {
    const rings = await svgContours('<svg width="40mm" height="30mm" viewBox="0 0 40 30"><rect x="2" y="2" width="36" height="26" rx="4"/></svg>')
    const result = await geometry.analyze(contoursExtrusion(rings,1), 'full')
    expect(result.volume).toBeCloseTo(36*26 - (4-Math.PI)*16, 0)
  })
  it('rejects unsupported SVG and invalid extrusion inputs', async () => {
    await expect(svgContours('<svg><foreignObject>hello</foreignObject></svg>')).rejects.toThrow()
    await expect(svgContours('x'.repeat(SVG_MAX_BYTES + 1))).rejects.toThrow('4 MiB')
    expect(()=>contoursExtrusion([[[0,0],[1,0],[0,1]]],0)).toThrow('height')
  })
})
