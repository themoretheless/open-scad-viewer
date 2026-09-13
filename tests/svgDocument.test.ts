import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { contoursExtrusion, contoursSvg, svgContours, svgPreview, SVG_MAX_BYTES } from '../src/services/svgGeometry'
import { parseOpenScadSvg, loadOpenScadImport, OpenScadImportDataError } from '../src/services/openScadImport'
import { OpenScadProject } from '../src/services/openScadProject'
import { HeadlessGeometryService } from '../src/mcp/geometryService'

const wrap = (body: string, size = 40) => `<svg xmlns="http://www.w3.org/2000/svg" width="${size}mm" height="${size}mm" viewBox="0 0 ${size} ${size}">${body}</svg>`
const area = (rings: readonly (readonly (readonly [number, number])[])[]) => Math.abs(rings.reduce((sum, ring) => sum + ring.reduce((s, a, i) => {
  const b = ring[(i + 1) % ring.length]
  return s + (a[0] * b[1] - b[0] * a[1]) / 2
}, 0), 0))
const sourceFile = (source: string) => ({ kind: 'source' as const, path: 'art.svg', source, byteLength: new TextEncoder().encode(source).length, sha256: '' })

describe('static SVG document support', () => {
  it('resolves CSS cascade, reuse, inherited paint and transforms into real geometry', async () => {
    const source = wrap(`<style>.part { fill: #369; } #hole {fill:none;stroke:#000;stroke-width:2}</style>
      <defs><rect id="tile" width="10" height="5"/></defs>
      <g class="part" transform="translate(2 3)"><use href="#tile"/><use href="#tile" x="12"/></g>
      <rect width="40" height="40" opacity="0"/>`)
    expect(area(await svgContours(source))).toBeCloseTo(100, 3)
    const preview = await svgPreview(source)
    expect(preview.widthMm).toBeCloseTo(40, 4)
    expect(preview.svg).toMatch(/#336699|#369|rgb\(51[, ]+102[, ]+153\)/)
    expect(preview.svg).not.toContain('<use')
    expect(area(await svgContours(preview.svg))).toBeCloseTo(100, 3)
  })

  it('clips compound paths with holes and nested percentage viewports', async () => {
    const source = wrap(`<defs><clipPath id="clip"><rect width="20" height="40"/></clipPath></defs>
      <g clip-path="url(#clip)"><path fill-rule="evenodd" d="M0 0H40V40H0Z M10 10H30V30H10Z"/></g>`)
    expect(area(await svgContours(source))).toBeCloseTo(600, 3)
    const nested = wrap('<svg x="25%" y="25%" width="50%" height="50%" viewBox="0 0 10 10"><rect width="100%" height="100%"/></svg>')
    expect(area(await svgContours(nested))).toBeCloseTo(400, 3)
  })

  it('creates proper dash gaps and cap geometry, with adaptive curve accuracy', async () => {
    expect(area(await svgContours(wrap('<path d="M0 10H40" fill="none" stroke="black" stroke-width="2" stroke-dasharray="5 5"/>')))).toBeCloseTo(40, 3)
    expect(area(await svgContours(wrap('<path d="M10 10H20" fill="none" stroke="black" stroke-width="2" stroke-linecap="square"/>')))).toBeCloseTo(24, 3)
    const circle = wrap('<circle cx="20" cy="20" r="15"/>')
    const coarse = await svgContours(circle, { tolerance: 0.5 })
    const fine = await svgContours(circle, { tolerance: 0.001 })
    expect(Math.abs(area(fine) - Math.PI * 225)).toBeLessThan(Math.abs(area(coarse) - Math.PI * 225))
    expect(area(fine)).toBeCloseTo(Math.PI * 225, 0)
  })

  it('outlines Latin and Cyrillic text and accepts deterministic custom font data', async () => {
    const source = wrap('<text x="2" y="20" font-size="9">SVG Привет</text>', 100)
    const preview = await svgPreview(source)
    expect(preview.svg).toContain('<path')
    expect(preview.svg).not.toMatch(/<text[ >]/)
    expect(area(await svgContours(source))).toBeGreaterThan(30)
    const font = new Uint8Array(readFileSync(new URL('./fixtures/Basic-Regular.ttf', import.meta.url)))
    const custom = await svgPreview(wrap('<text x="2" y="20" font-family="Basic" font-size="12">TEST</text>'), { fonts: [font] })
    expect(area(await svgContours(custom.svg))).toBeGreaterThan(20)
  })

  it('preserves gradients, masks and filters in artwork and explicitly traces their appearance', async () => {
    const source = wrap(`<defs>
      <linearGradient id="g"><stop stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient>
      <mask id="m"><rect width="20" height="40" fill="white"/></mask>
      <filter id="f"><feGaussianBlur stdDeviation="0.5"/></filter></defs>
      <rect width="40" height="40" fill="url(#g)" mask="url(#m)" filter="url(#f)"/>`)
    const preview = await svgPreview(source)
    expect(preview.svg).toContain('<linearGradient')
    expect(preview.svg).toContain('<mask')
    expect(preview.svg).toContain('feGaussianBlur')
    await expect(svgContours(source)).rejects.toThrow(/silhouette/i)
    const rings = await svgContours(source, { geometryMode: 'silhouette', rasterSize: 256 })
    expect(area(rings)).toBeGreaterThan(760)
    expect(area(rings)).toBeLessThan(820)
  })

  it('preserves holes, physical size and volume through SVG → solid → SVG', async () => {
    const source = wrap('<style>path { fill-rule: evenodd }</style><path d="M0 0H40V40H0Z M10 10H30V30H10Z"/>')
    const profile = await svgContours(source)
    const exported = contoursSvg(profile)
    const reimported = await svgContours(exported)
    expect(area(reimported)).toBeCloseTo(1200, 3)
    const geometry = new HeadlessGeometryService()
    expect((await geometry.analyze(contoursExtrusion(reimported, 3), 'full')).volume).toBeCloseTo(3600, 3)
  })

  it('keeps standalone 96 DPI and OpenSCAD 72 DPI conventions explicit', async () => {
    const source = '<svg width="96" height="96"><rect width="96" height="48"/></svg>'
    expect(area(await svgContours(source))).toBeCloseTo(25.4 * 12.7, 3)
    const legacy = parseOpenScadSvg(sourceFile(source))
    expect(area(legacy.regions.flatMap(r => r.contours))).toBeCloseTo((96 * 25.4 / 72) * (48 * 25.4 / 72), 3)
    const project = new OpenScadProject({ entrypoint: 'main.scad', files: [
      { kind: 'source', path: 'main.scad', source: 'linear_extrude(2) import("art.svg",dpi=96);' },
      { kind: 'source', path: 'art.svg', source },
    ] })
    const imported = await loadOpenScadImport(project, 'main.scad', 'art.svg', { dpi: 96, center: true })
    expect(imported.dimension).toBe(2)
    if (imported.dimension === 2) expect(area(imported.regions.flatMap(r => r.contours))).toBeCloseTo(25.4 * 12.7, 3)
  })

  it('rejects active/external content and malformed data without partial imports', async () => {
    for (const source of [
      wrap('<script>alert(1)</script><rect width="10" height="10"/>'),
      wrap('<rect width="10" height="10" onload="alert(1)"/>'),
      wrap('<image href="file:///private/secret.png" width="10" height="10"/>'),
      wrap('<style>@import url(https://example.com/style.css);</style><rect width="10" height="10"/>'),
      wrap('<rect width="10" height="10"><animate attributeName="x" to="20"/></rect>'),
      wrap('<path d="M0 0R1 1"/><rect width="10" height="10"/>'),
    ]) {
      await expect(svgPreview(source)).rejects.toThrow()
      expect(() => parseOpenScadSvg(sourceFile(source))).toThrow(OpenScadImportDataError)
    }
    await expect(svgPreview(' '.repeat(SVG_MAX_BYTES + 1))).rejects.toThrow('4 MiB')
    await expect(svgContours(wrap('<rect width="10" height="10"/>'), { tolerance: 0 })).rejects.toThrow('tolerance')
  })
})
