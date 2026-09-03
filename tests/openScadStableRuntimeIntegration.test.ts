import { describe, expect, it } from 'vitest'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'

const source = `
  echo($t, $preview, $vpt, $vpr, $vpd, $vpf, $parent_modules);
  cube(1);
`

describe('OpenSCAD 2021.01 runtime variables in independent execution lanes', () => {
  it('injects animation, preview, and camera values in the direct evaluator', async () => {
    const result = await parseOpenSCAD(source, {
      languageProfile: 'openscad/stable-2021.01',
      quality: 'preview',
      animationTime: 0.375,
    })

    expect(result.warnings).toEqual([
      "Ignoring unknown variable '$parent_modules'",
      'ECHO: 0.375, true, [0, 0, 0], [55, 0, 25], 140, 22.5, undef',
    ])
    expect(result.meshes).toHaveLength(1)
  })

  it('injects the same values in semantic lowering without defining top-level parent depth', () => {
    const result = lowerOpenSCADToSemanticProgram(source, {
      languageProfile: 'openscad/stable-2021.01',
      quality: 'full',
      animationTime: 1,
    })

    expect(result.warnings).toEqual([
      "Ignoring unknown variable '$parent_modules'",
      'ECHO: 1, false, [0, 0, 0], [55, 0, 25], 140, 22.5, undef',
    ])
    expect(result.program.core.nodes.map(node => node.kind)).toEqual(['box'])
  })

  it('keeps is_undef probes quiet and forwards repaired built-in warnings', async () => {
    const result = await parseOpenSCAD(`
      echo(is_undef(not_declared));
      echo(parent_module(999));
      cube(1);
    `, { languageProfile: 'openscad/stable-2021.01' })

    expect(result.warnings).toEqual([
      'ECHO: true',
      'parent_module() index 999 is outside the active module stack',
      'ECHO: undef',
    ])
  })

  it('matches the same quiet probe and warning behavior in semantic lowering', () => {
    const result = lowerOpenSCADToSemanticProgram(`
      echo(is_undef(not_declared));
      echo(parent_module(999));
      cube(1);
    `, { languageProfile: 'openscad/stable-2021.01' })

    expect(result.warnings).toEqual([
      'ECHO: true',
      'parent_module() index 999 is outside the active module stack',
      'ECHO: undef',
    ])
  })
})
