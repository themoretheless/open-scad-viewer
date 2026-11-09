import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { SPINNER_DEFAULTS, SPINNER_SAFE_DEFAULTS, computeSpinnerParameterSuggestions, generateSpinner, generateSpinnerSafe } from '../src/services/spinnerGenerator'
import { compileRushFrontend } from '../src/services/rushFrontend'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { exportMesh } from '../src/services/meshConvert'

describe('spinner generator', () => {
  it('matches the canonical planetary-spinner.r example at default parameters', () => {
    const canonical = readFileSync('examples/rush-frontend/planetary-spinner.r', 'utf8')
    expect(generateSpinner(SPINNER_DEFAULTS)).toBe(canonical)
  })

  it('compiles with 6 customizer parameters and builds 20 gear bodies', async () => {
    const source = generateSpinner(SPINNER_DEFAULTS)
    const compiled = compileRushFrontend(source)
    expect(compiled.customizer).toHaveLength(6)
    const parsed = await parseOpenSCAD(source)
    expect(parsed.meshes).toHaveLength(20)
    const scaled = await parseOpenSCAD(generateSpinner({ ...SPINNER_DEFAULTS, inner_radius: 35, outer_radius: 38, hole: 38 }))
    expect(scaled.meshes).toHaveLength(20)
    const safeSource = generateSpinnerSafe(SPINNER_SAFE_DEFAULTS)
    expect(compileRushFrontend(safeSource).customizer).toHaveLength(6)
    const safeParsed = await parseOpenSCAD(safeSource)
    expect(safeParsed.meshes).toHaveLength(20)
  }, 120000)

  it('validates rim thickness, bore diameter, and backlash gap', () => {
    expect(() => generateSpinner({ ...SPINNER_DEFAULTS, outer_radius: 30 })).toThrow('outer_radius')
    expect(() => generateSpinner({ ...SPINNER_DEFAULTS, outer_radius: 31 })).toThrow('толщина наружного кольца')
    expect(() => generateSpinner({ ...SPINNER_DEFAULTS, hole: 80 })).toThrow('Центральное отверстие')
    expect(() => generateSpinner({ ...SPINNER_DEFAULTS, gap: 0.6, inner_radius: 20 })).toThrow('Зазор')
  })

  it('suggests recalculating dependent parameters when one parameter changes', () => {
    const v1FromInner = computeSpinnerParameterSuggestions('inner_radius', {
      ...SPINNER_DEFAULTS,
      inner_radius: 35,
    })
    expect(v1FromInner.length).toBeGreaterThan(0)
    expect(v1FromInner[0].changes).toEqual({ outer_radius: 38.36, hole: 45.83 })

    const v1FromHole = computeSpinnerParameterSuggestions('hole', {
      ...SPINNER_DEFAULTS,
      hole: 44,
    })
    expect(v1FromHole.length).toBeGreaterThan(0)
    expect(v1FromHole[0].changes).toEqual({ inner_radius: 33.74, outer_radius: 37.07 })

    const v2FromInnerRim = computeSpinnerParameterSuggestions('inner_rim', {
      ...SPINNER_SAFE_DEFAULTS,
      inner_rim: 4,
    })
    expect(v2FromInnerRim.length).toBeGreaterThan(0)
    expect(v2FromInnerRim[0].changes).toEqual({ outer_rim: 4 })

    const v2SmallHole = computeSpinnerParameterSuggestions('hole', {
      hole: 18,
      inner_rim: 0.8,
      outer_rim: 0.8,
      gap: 0.35,
      height: 6,
      helix: 35,
    })
    expect(v2SmallHole.length).toBeGreaterThan(0)
    expect(v2SmallHole[0].changes).toEqual({ gap: 0.26 })
  })

  it('produces denser meshes when exporting with high or ultra quality than in the viewport', async () => {
    const source = 'brep_gear(module = 1.5, teeth = 8, height = 5, helix = 15, herringbone = true);'
    const viewportParsed = await parseOpenSCAD(source)
    const viewportTris = viewportParsed.meshes.reduce((sum, m) => sum + m.indices.length / 3, 0)
    const highParsed = await parseOpenSCAD(source, { quality: 'full', exportQuality: 'high' })
    const highTris = highParsed.meshes.reduce((sum, m) => sum + m.indices.length / 3, 0)
    const ultraParsed = await parseOpenSCAD(source, { quality: 'full', exportQuality: 'ultra' })
    const ultraTris = ultraParsed.meshes.reduce((sum, m) => sum + m.indices.length / 3, 0)
    expect(highTris).toBeGreaterThan(viewportTris)
    expect(ultraTris).toBeGreaterThan(highTris)

    const viewportStl = await exportMesh(viewportParsed.meshes, 'stl_binary', { quality: 'viewport', source })
    const highStl = await exportMesh(viewportParsed.meshes, 'stl_binary', { quality: 'high', source })
    expect(highStl.data.byteLength).toBeGreaterThan(viewportStl.data.byteLength)
  }, 60000)
})
