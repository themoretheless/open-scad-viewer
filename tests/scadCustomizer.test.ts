import { describe, expect, it } from 'vitest'
import { extractCustomizerParameters, replaceCustomizerValue } from '../src/services/scadCustomizer'

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
})
