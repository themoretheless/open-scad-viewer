import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { defaultGeometryKernel } from '../src/services/manifoldGeometryKernel'
import { OpenScadProject, type OpenScadProjectFileInput } from '../src/services/openScadProject'
import {
  OPENSCAD_2021_TEXT_EM_SCALE,
  OPENSCAD_TEXT_MAX_CODE_POINTS,
  OpenScadTextError,
  calculateOpenScadTextSegments,
  prepareOpenScadTextAssets,
  renderOpenScadText,
  type OpenScadTextContour,
  type OpenScadTextParameters,
} from '../src/services/openScadText'

const BASIC = new Uint8Array(readFileSync(
  new URL('./fixtures/Basic-Regular.ttf', import.meta.url),
))
const BASIC_LICENSE = readFileSync(
  new URL('./fixtures/Basic-OFL.txt', import.meta.url),
)
const BASIC_METADATA = JSON.parse(readFileSync(
  new URL('./fixtures/Basic-Regular.metadata.json', import.meta.url),
  'utf8',
)) as Record<string, unknown>
const NOTO_ROOT = new URL('../node_modules/harfbuzzjs/test/fonts/noto/', import.meta.url)
const NOTO_SANS = new Uint8Array(readFileSync(new URL('NotoSans-Regular.ttf', NOTO_ROOT)))
const NOTO_ARABIC = new Uint8Array(readFileSync(new URL('NotoSansArabic-Variable.ttf', NOTO_ROOT)))
const NOTO_DEVANAGARI = new Uint8Array(readFileSync(new URL('NotoSansDevanagari-Regular.otf', NOTO_ROOT)))

function projectWith(files: readonly OpenScadProjectFileInput[], entrypoint = 'main.scad'): OpenScadProject {
  return new OpenScadProject({
    entrypoint,
    files: [{ kind: 'source', path: entrypoint, source: '' }, ...files],
  })
}

function basicProject(): OpenScadProject {
  return projectWith([{ kind: 'blob', path: 'fonts/Basic-Regular.ttf', data: BASIC }])
}

async function renderBasic(parameters: OpenScadTextParameters) {
  const project = basicProject()
  return renderOpenScadText(
    project,
    await prepareOpenScadTextAssets(project),
    { font: 'Basic:style=Regular', ...parameters },
  )
}

function signedArea(contour: OpenScadTextContour): number {
  let area = 0
  for (let index = 0; index < contour.length; index++) {
    const current = contour[index]
    const next = contour[(index + 1) % contour.length]
    area += current[0] * next[1] - current[1] * next[0]
  }
  return area / 2
}

describe('independent OpenSCAD text service', () => {
  it('indexes only bounded project-VFS fonts and resolves Fontconfig family/style names', async () => {
    expect(createHash('sha256').update(BASIC).digest('hex'))
      .toBe('f2487f20e2241002d007831ec0e7e9c24ad39e36b49492326c79f3d70fd3b270')
    expect(createHash('sha256').update(BASIC_LICENSE).digest('hex'))
      .toBe('d3dbe81ed315fba5ab88aa2ceb53f92892e4f58e405f31e07e0156b15ddb9b60')
    expect(BASIC_METADATA).toMatchObject({
      sourceRepository: 'https://github.com/SorkinType/Basic',
      sourceCommit: '202e65ac93bd6977e83b2f10db6b1467e0b348db',
      byteLength: BASIC.byteLength,
      sha256: 'f2487f20e2241002d007831ec0e7e9c24ad39e36b49492326c79f3d70fd3b270',
      license: 'SIL Open Font License 1.1',
      licenseFile: 'Basic-OFL.txt',
      licenseSha256: 'd3dbe81ed315fba5ab88aa2ceb53f92892e4f58e405f31e07e0156b15ddb9b60',
    })
    const project = basicProject()
    const prepared = await prepareOpenScadTextAssets(project)

    expect(prepared.rejected).toEqual([])
    expect(prepared.fonts).toEqual([expect.objectContaining({
      path: 'fonts/Basic-Regular.ttf',
      family: 'Basic',
      style: 'Regular',
      fullName: 'Basic Regular',
      postscriptName: 'Basic-Regular',
      unitsPerEm: 2_048,
    })])
    expect(OPENSCAD_2021_TEXT_EM_SCALE).toBe(1.3888)
    expect(renderOpenScadText(project, prepared, {
      text: 'A',
      font: 'Basic:style=Regular',
    }).font.path).toBe('fonts/Basic-Regular.ttf')
  })

  it('matches the pinned OpenSCAD 2021 Basic A oracle bounds and alignment', async () => {
    const baseline = await renderBasic({ text: 'A', size: 10 })

    // Pinned official 2021.01 SVG oracle:
    // x [-0.0678, 7.6696], y [0, 9.2096]. The remaining sub-millimetre
    // difference is FreeType grid hinting; the SFNT outline and scale are real.
    expect(baseline.bounds?.min[0]).toBeCloseTo(-0.0678, 3)
    expect(baseline.bounds?.max[0]).toBeCloseTo(7.6696, 3)
    expect(baseline.bounds?.min[1]).toBeCloseTo(0, 3)
    expect(baseline.bounds?.max[1]).toBeCloseTo(9.2096, 2)
    expect(baseline.advance[0]).toBeCloseTo(7.6018, 3)
    expect(baseline.glyphs).toHaveLength(1)
    expect(baseline.glyphs[0].glyphId).toBe(46)
    expect(baseline.glyphs[0].contours.map(contour => contour.length)).toEqual([8, 3])

    const centered = await renderBasic({
      text: 'A',
      size: 10,
      halign: 'center',
      valign: 'center',
    })
    expect(centered.bounds?.min[0]).toBeCloseTo(-3.8689, 3)
    expect(centered.bounds?.max[0]).toBeCloseTo(3.8685, 3)
    expect(centered.bounds?.min[1]).toBeCloseTo(-4.6048, 3)
    expect(centered.bounds?.max[1]).toBeCloseTo(4.6048, 3)
  })

  it('keeps holes and overlapping glyphs in separate EvenOdd-ready polygon groups', async () => {
    const layout = await renderBasic({ text: 'AA', size: 10 })

    expect(layout.glyphs).toHaveLength(2)
    expect(layout.glyphs.map(glyph => glyph.contours.length)).toEqual([2, 2])
    for (const glyph of layout.glyphs) {
      expect(Math.sign(signedArea(glyph.contours[0])))
        .toBe(-Math.sign(signedArea(glyph.contours[1])))
    }

    const session = await defaultGeometryKernel.openSession()
    try {
      const sections = layout.glyphs.map(glyph => (
        session.module.CrossSection.ofPolygons(glyph.contours, 'EvenOdd')
      ))
      expect(sections.every(section => !section.isEmpty())).toBe(true)
      expect(sections.every(section => section.area() > 0)).toBe(true)
      const union = session.module.CrossSection.union(sections)
      expect(union.isEmpty()).toBe(false)
      expect(union.area()).toBeGreaterThan(0)
    } finally {
      session.dispose()
    }
  })

  it('applies OpenSCAD spacing, direction, and four alignment modes to positioned outlines', async () => {
    const normal = await renderBasic({ text: 'AV', size: 10 })
    const spaced = await renderBasic({ text: 'AV', size: 10, spacing: 2 })
    expect(spaced.glyphs[0].offset).toEqual(normal.glyphs[0].offset)
    expect(spaced.glyphs[1].offset[0]).toBeCloseTo(normal.glyphs[1].offset[0] * 2, 10)
    expect(spaced.width).toBeCloseTo(normal.width * 2, 10)

    const rightTop = await renderBasic({
      text: 'A',
      size: 10,
      halign: 'right',
      valign: 'top',
    })
    expect(rightTop.glyphs[0].offset[0]).toBeCloseTo(-rightTop.width, 10)
    expect(rightTop.glyphs[0].offset[1]).toBeCloseTo(-rightTop.ascent, 10)

    const vertical = await renderBasic({ text: 'AV', direction: 'ttb', size: 10 })
    expect(vertical.direction).toBe('ttb')
    expect(vertical.advance[1]).toBeLessThan(0)
    expect(vertical.glyphs[1].offset[1]).toBeLessThan(vertical.glyphs[0].offset[1])

    const scriptedRtl = await renderBasic({ text: 'AB', script: 'Arab', size: 10 })
    expect(scriptedRtl.direction).toBe('rtl')
    expect(scriptedRtl.glyphs.map(glyph => glyph.cluster)).toEqual([1, 0])
  })

  it('uses HarfBuzz GPOS kerning and GSUB ligatures instead of codepoint boxes', async () => {
    const project = projectWith([{ kind: 'blob', path: 'fonts/NotoSans-Regular.ttf', data: NOTO_SANS }])
    const prepared = await prepareOpenScadTextAssets(project)
    const render = (text: string) => renderOpenScadText(project, prepared, {
      text,
      font: 'Noto Sans:style=Regular',
      size: 10,
      language: 'en',
      script: 'latin',
      direction: 'ltr',
    })

    const a = render('A')
    const v = render('V')
    const av = render('AV')
    expect(av.advance[0]).toBeLessThan(a.advance[0] + v.advance[0])

    const ligature = render('ffi')
    expect(ligature.glyphs).toHaveLength(1)
    expect(ligature.glyphs[0].glyphId).not.toBe(0)
    expect(ligature.glyphs[0].contours.length).toBeGreaterThan(0)
  })

  it('shapes Arabic RTL and Devanagari conjunct/reordering clusters', async () => {
    const project = projectWith([
      { kind: 'blob', path: 'fonts/NotoSansArabic-Variable.ttf', data: NOTO_ARABIC },
      { kind: 'blob', path: 'fonts/NotoSansDevanagari-Regular.otf', data: NOTO_DEVANAGARI },
    ])
    const prepared = await prepareOpenScadTextAssets(project)

    const arabic = renderOpenScadText(project, prepared, {
      text: 'سلام',
      font: 'Noto Sans Arabic:style=Regular',
      language: 'ar',
      script: 'arabic',
      direction: 'rtl',
    })
    expect(arabic.direction).toBe('rtl')
    expect(arabic.glyphs.map(glyph => glyph.cluster)).toEqual([3, 1, 0])
    expect(arabic.glyphs).toHaveLength(3)
    expect(arabic.glyphs.every(glyph => glyph.glyphId !== 0 && glyph.contours.length > 0)).toBe(true)

    const devanagari = renderOpenScadText(project, prepared, {
      text: 'क्षि',
      font: 'Noto Sans Devanagari:style=Regular',
      language: 'hi',
      script: 'deva',
      direction: 'ltr',
    })
    expect(devanagari.glyphs).toHaveLength(2)
    expect(devanagari.glyphs.map(glyph => glyph.cluster)).toEqual([0, 0])
    expect(devanagari.glyphs.every(glyph => glyph.glyphId !== 0 && glyph.contours.length > 0)).toBe(true)
  })

  it('resolves explicit font files relative to the defining source path', async () => {
    const project = projectWith([
      { kind: 'source', path: 'models/lib/label.scad', source: '' },
      { kind: 'blob', path: 'models/fonts/Basic-Regular.ttf', data: BASIC },
    ], 'models/main.scad')
    const prepared = await prepareOpenScadTextAssets(project)
    const layout = renderOpenScadText(project, prepared, {
      text: 'A',
      font: '../fonts/Basic-Regular.ttf',
    }, 'models/lib/label.scad')

    expect(layout.font.path).toBe('models/fonts/Basic-Regular.ttf')
  })

  it('never falls back to ambient fonts and reports malformed VFS fonts with typed errors', async () => {
    const empty = projectWith([])
    const emptyPrepared = await prepareOpenScadTextAssets(empty)
    expect(() => renderOpenScadText(empty, emptyPrepared, {
      text: 'A',
      font: 'Basic:style=Regular',
    })).toThrow(expect.objectContaining({
      name: OpenScadTextError.name,
      code: 'E_TEXT_FONT_NOT_FOUND',
    }))

    const malformed = projectWith([{
      kind: 'blob',
      path: 'fonts/bad.ttf',
      data: Uint8Array.from([0, 1, 2, 3]),
    }])
    const malformedPrepared = await prepareOpenScadTextAssets(malformed)
    expect(malformedPrepared.rejected).toEqual([expect.objectContaining({
      path: 'fonts/bad.ttf',
      code: 'E_TEXT_FONT_INVALID',
    })])
    expect(() => renderOpenScadText(malformed, malformedPrepared, {
      text: 'A',
      font: 'fonts/bad.ttf',
    })).toThrow(expect.objectContaining({
      name: OpenScadTextError.name,
      code: 'E_TEXT_FONT_INVALID',
      details: expect.objectContaining({ fontPath: 'fonts/bad.ttf' }),
    }))
  })

  it('bounds text, curves, and project identity before expensive outline expansion', async () => {
    expect(calculateOpenScadTextSegments(10, 0, 2, 12)).toBe(4)
    expect(calculateOpenScadTextSegments(10, 8, 2, 12)).toBe(2)
    expect(calculateOpenScadTextSegments(10, 64, 2, 12)).toBe(9)

    const project = basicProject()
    const prepared = await prepareOpenScadTextAssets(project)
    expect(() => renderOpenScadText(project, prepared, {
      text: 'A'.repeat(OPENSCAD_TEXT_MAX_CODE_POINTS + 1),
      font: 'Basic',
    })).toThrow(expect.objectContaining({
      code: 'E_TEXT_PARAMETER_LIMIT',
      details: expect.objectContaining({
        limit: OPENSCAD_TEXT_MAX_CODE_POINTS,
        actual: OPENSCAD_TEXT_MAX_CODE_POINTS + 1,
      }),
    }))

    const otherProject = projectWith([
      { kind: 'blob', path: 'fonts/Basic-Regular.ttf', data: BASIC },
      { kind: 'source', path: 'different.scad', source: 'cube();' },
    ])
    expect(() => renderOpenScadText(otherProject, prepared, { text: 'A', font: 'Basic' }))
      .toThrow(expect.objectContaining({ code: 'E_TEXT_PROJECT_MISMATCH' }))
  })
})
