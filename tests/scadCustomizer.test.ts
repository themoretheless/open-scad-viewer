import { describe, expect, it } from 'vitest'
import { extractCustomizerParameters, replaceCustomizerValue } from '../src/language/scadCustomizer'

describe('OpenSCAD Customizer parameters', () => {
  it('extracts top-level slider, boolean and choice metadata', () => {
    const source = `width = 20; // [5:0.5:40]\ncentered = true;\nmaterial = "steel"; // [steel,wood]\nmodule hidden() { local = 4; }`
    const parameters = extractCustomizerParameters(source)
    expect(parameters.map(parameter => parameter.name)).toEqual(['width', 'centered', 'material'])
    expect(parameters[0]).toMatchObject({ value: 20, min: 5, step: 0.5, max: 40 })
    expect(parameters[2].options).toEqual(['steel', 'wood'])
  })

  it('replaces only the literal value at its source range', () => {
    const source = 'width = 20; // [5:40]\ncube([width, 20, 20]);'
    const [parameter] = extractCustomizerParameters(source)
    expect(replaceCustomizerValue(source, parameter, 32)).toBe('width = 32; // [5:40]\ncube([width, 20, 20]);')
  })

  it('does not corrupt names that contain the value substring', () => {
    // indexOf-based offset matched the "1" INSIDE "x1", splicing the name.
    const source = 'x1 = 1;\ncube([x1, 2, 3]);'
    const [parameter] = extractCustomizerParameters(source)
    expect(replaceCustomizerValue(source, parameter, 2)).toBe('x1 = 2;\ncube([x1, 2, 3]);')
  })

  it('uses trailing inline parameter comments as human labels while preserving range metadata', () => {
    const source = [
      'width = 120; // [50:1:300] Ширина коробки, мм',
      'bottom_fillet = true; // Внутренняя галтель стыка со дном',
      'skadis_part = 0; // [0, 1, 2, 3] Выходная деталь',
      'plain = 5;',
    ].join('\n')
    const params = extractCustomizerParameters(source)
    expect(params[0]).toMatchObject({
      name: 'width',
      label: 'Ширина коробки, мм',
      value: 120,
      min: 50,
      step: 1,
      max: 300,
    })
    expect(params[1]).toMatchObject({ name: 'bottom_fillet', label: 'Внутренняя галтель стыка со дном', value: true })
    expect(params[1]!.options).toBeUndefined()
    expect(params[2]).toMatchObject({ name: 'skadis_part', label: 'Выходная деталь', value: 0, options: [0, 1, 2, 3] })
    expect(params[3]).toMatchObject({ name: 'plain', label: 'Plain', value: 5 })
    expect(replaceCustomizerValue(source, params[0]!, 180)).toContain('width = 180; // [50:1:300] Ширина коробки, мм')
  })
})
