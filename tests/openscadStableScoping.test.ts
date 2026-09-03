import { describe, expect, it } from 'vitest'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'

const stableProfile = { languageProfile: 'openscad/stable-2021.01' as const }

function lower(source: string) {
  return lowerOpenSCADToSemanticProgram(source, stableProfile)
}

describe('OpenSCAD 2021.01 stable scoping', () => {
  it('uses the last assignment retroactively and the first declaration environment for its RHS', async () => {
    const retroactive = 'a = 1; echo(a); a = 2; echo(a); cube(1);'
    const hiddenLaterBinding = 'a = 1; b = 2; a = b; echo(a); cube(1);'
    const visibleEarlierBinding = 'b = 2; a = 1; a = b; echo(a); cube(1);'

    const directRetroactive = await parseOpenSCAD(retroactive, stableProfile)
    const semanticRetroactive = lower(retroactive)
    expect(directRetroactive.warnings).toEqual(['ECHO: 2', 'ECHO: 2'])
    expect(semanticRetroactive.warnings).toEqual(['ECHO: 2', 'ECHO: 2'])

    const directHidden = await parseOpenSCAD(hiddenLaterBinding, stableProfile)
    const semanticHidden = lower(hiddenLaterBinding)
    expect(directHidden.warnings).toEqual([
      "Ignoring unknown variable 'b'",
      'ECHO: undef',
    ])
    expect(semanticHidden.warnings).toEqual(directHidden.warnings)

    const directVisible = await parseOpenSCAD(visibleEarlierBinding, stableProfile)
    const semanticVisible = lower(visibleEarlierBinding)
    expect(directVisible.warnings).toEqual(['ECHO: 2'])
    expect(semanticVisible.warnings).toEqual(directVisible.warnings)
  })

  it('captures ordinary names lexically while propagating special variables dynamically', async () => {
    const source = `
      x = 2;
      function capture() = [x, $foo];
      module capture_module() echo(x, $foo) cube(1);
      echo(let(x = 99, $foo = 3) capture());
      let(x = 99, $foo = 3) capture_module();
    `
    const expected = [
      'ECHO: [2, 3]',
      'ECHO: 2, 3',
    ]

    const direct = await parseOpenSCAD(source, stableProfile)
    const semantic = lower(source)
    expect(direct.warnings).toEqual(expected)
    expect(semantic.warnings).toEqual(expected)
    expect(direct.meshes).toHaveLength(1)
    expect(semantic.program.core.nodes.map(node => node.kind)).toEqual(['box'])
  })

  it('keeps nested declarations local and resolves local self-reference through the parent scope', async () => {
    const source = `
      a = 10;
      module q() {
        a = a + 1;
        function local_value() = a;
        echo(local_value());
        cube(1);
      }
      q();
    `

    const direct = await parseOpenSCAD(source, stableProfile)
    const semantic = lower(source)
    expect(direct.warnings).toEqual(['ECHO: 11'])
    expect(semantic.warnings).toEqual(['ECHO: 11'])

    await expect(parseOpenSCAD(
      'if (true) { function hidden() = 1; echo(hidden()); } echo(hidden()); cube(1);',
      stableProfile,
    )).rejects.toThrow(/Unsupported function hidden\(\)/)
    await expect(parseOpenSCAD(
      'if (true) { module hidden() cube(1); hidden(); } hidden();',
      stableProfile,
    )).rejects.toThrow(/Unsupported geometry operation hidden\(\)/)
    expect(() => lower(
      'if (true) { function hidden() = 1; echo(hidden()); } echo(hidden()); cube(1);',
    )).toThrow(/Unsupported function hidden\(\)/)
    expect(() => lower(
      'if (true) { module hidden() cube(1); hidden(); } hidden();',
    )).toThrow(/Unsupported geometry operation hidden\(\)/)
  })

  it('sees later lexical assignments, detects cycles, and keeps defaults outside parameter scope', async () => {
    const source = `
      function later() = x;
      module later_module() echo(x);
      function defaults(y = 4, x = y) = x;
      x = 2;
      echo(later(), defaults());
      later_module();
      cube(1);
    `
    const expected = [
      "Ignoring unknown variable 'y'",
      'ECHO: 2, undef',
      'ECHO: 2',
    ]
    const direct = await parseOpenSCAD(source, stableProfile)
    const semantic = lower(source)
    expect(direct.warnings).toEqual(expected)
    expect(semantic.warnings).toEqual(expected)

    const cycle = 'a = f(); function f() = a; echo(a); cube(1);'
    const directCycle = await parseOpenSCAD(cycle, stableProfile)
    const semanticCycle = lower(cycle)
    expect(directCycle.warnings).toEqual([
      "Ignoring cyclic variable reference 'a'",
      'ECHO: undef',
    ])
    expect(semanticCycle.warnings).toEqual(directCycle.warnings)
  })

  it('provides dynamic child and parent-module metadata with the lexical children continuation', async () => {
    const source = `
      function frame() = [$foo, $children, $parent_modules, parent_module(0)];
      module inner() {
        echo(frame(), parent_module(1));
        children();
      }
      module outer() {
        echo($children, $parent_modules, parent_module(0), $foo);
        inner() children();
      }
      let($foo = 3) outer() cube(1);
    `
    const expected = [
      'ECHO: 1, 1, "outer", 3',
      'ECHO: [3, 1, 2, "inner"], "outer"',
    ]

    const direct = await parseOpenSCAD(source, stableProfile)
    const semantic = lower(source)
    expect(direct.warnings).toEqual(expected)
    expect(semantic.warnings).toEqual(expected)
    expect(direct.meshes).toHaveLength(1)
    expect(semantic.program.core.nodes.filter(node => node.kind === 'box')).toHaveLength(1)
  })
})
