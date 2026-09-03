import { describe, expect, it } from 'vitest'
import { OpenSCADParseError } from '../src/services/openscadErrors'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'

const stableProfile = { languageProfile: 'openscad/stable-2021.01' as const }

function lower(source: string) {
  return lowerOpenSCADToSemanticProgram(source, stableProfile)
}

describe('independent OpenSCAD 2021.01 expression semantics', () => {
  it('evaluates sequential let, assert and echo expressions without consuming geometry', () => {
    const result = lower(`
      echo(
        let(a = 1, b = a + 1) b,
        assert(true) 7,
        echo("inner") 9
      );
      cube(1);
    `)

    expect(result.terminalError).toBeNull()
    expect(result.warnings).toEqual([
      'ECHO: "inner"',
      'ECHO: 2, 7, 9',
    ])
    expect(result.program.core.nodes.map(node => node.kind)).toEqual(['box'])
  })

  it('returns undef for body-less assert/echo expressions and fails a false assertion', () => {
    const result = lower('echo(assert(true), echo("side")); cube(1);')

    expect(result.warnings).toEqual([
      'ECHO: "side"',
      'ECHO: undef, undef',
    ])
    expect(() => lower('echo(assert(false, "boom") 7); cube(1);'))
      .toThrow(OpenSCADParseError)
    expect(() => lower('echo(assert(false, "boom") 7); cube(1);'))
      .toThrow(/Assertion .* failed: "boom"/)
  })

  it('flattens all five list-comprehension forms with nested iterator scope', () => {
    const result = lower(`
      echo(
        [for (i = [0:2]) i * i],
        [for (i = [1, 2], j = [i, i + 1]) [i, j]],
        [for (i = 0; i < 4; i = i + 1) i],
        [if (false) 1 else 2],
        [let (a = 2) each [a + 1, a + 2]],
        [each [1, 2], 3]
      );
      cube(1);
    `)

    expect(result.warnings).toEqual([
      'ECHO: [0, 1, 4], [[1, 1], [1, 2], [2, 2], [2, 3]], [0, 1, 2, 3], [2], [3, 4], [1, 2, 3]',
    ])
    expect(result.program.core.nodes.map(node => node.kind)).toEqual(['box'])
  })

  it('routes tagged ranges and vector/matrix/index/member operations through shared semantics', () => {
    const result = lower(`
      echo(
        [0:2],
        [0:2][1],
        [1, 2] + [3, 4],
        [1, 2] * [3, 4],
        [[1, 2], [3, 4]] * [5, 6],
        [7, 8, 9].y,
        "😀x"[0],
        [10, 20][1.9],
        [10][-1],
        1 / 3,
        1.23456789,
        [1:1:0] ? true : false
      );
      cube(1);
    `)

    expect(result.warnings).toEqual([
      'begin is greater than the end, but step is positive',
      'ECHO: [0 : 1 : 2], 1, [4, 6], 11, [17, 39], 8, "😀", 20, undef, 0.333333, 1.23457, true',
    ])
  })

  it('preserves descending two-value range endpoints and iterates the empty direction', () => {
    const result = lower('echo([3:1]); for (i = [3:1]) cube(i);')

    expect(result.warnings).toEqual([
      'begin is greater than the end, but step is positive',
      'ECHO: [3 : 1 : 1]',
    ])
    expect(result.program.core.nodes).toEqual([])
    expect(result.program.core.result).toEqual({ tag: 'empty', type: 'never' })
  })

  it('uses undef plus warnings in stable while preserving fatal legacy arithmetic', () => {
    const result = lower('echo([1, 2] + 1, missing); cube(1);')

    expect(result.warnings).toEqual([
      'Undefined operation (vector + number)',
      "Ignoring unknown variable 'missing'",
      'ECHO: undef, undef',
    ])
    expect(() => lowerOpenSCADToSemanticProgram('cube([1, 2] + 1);'))
      .toThrow(/arithmetic operand must be a finite number/)
  })

  it('materializes a tagged range only for statement iteration', () => {
    const result = lower('for (i = [0:2]) translate([i, 0, 0]) cube(1);')

    expect(result.warnings).toEqual([])
    expect(result.program.core.nodes.filter(node => node.kind === 'box')).toHaveLength(3)
    expect(result.program.core.nodes.filter(node => node.kind === 'transform')).toHaveLength(3)
    expect(result.program.core.result.tag).toBe('multi')
  })

  it('fills the first unsupplied user function/module parameter after named arguments', () => {
    const result = lower(`
      function f(a, b, c) = [a, b, c];
      module m(a, b, c) cube([a, b, c]);
      echo(f(b = 2, 1, 3), f(a = 1, a = 2, 3));
      m(b = 2, 1, 3);
    `)

    expect(result.warnings).toEqual([
      'Argument a was specified more than once',
      'ECHO: [1, 2, 3], [2, 3, undef]',
    ])
    expect(result.program.core.nodes).toHaveLength(1)
    expect(result.program.core.nodes[0]).toMatchObject({ kind: 'box', size: [1, 2, 3] })
  })

  it('evaluates duplicate module arguments once and lets the last named value win', () => {
    const result = lower(`
      module m(a, b) { echo(a, b); cube([a, b, 1]); }
      m(a = echo("first") 1, a = echo("second") 2, 3);
    `)

    expect(result.warnings).toEqual([
      'Argument a was specified more than once',
      'ECHO: "first"',
      'ECHO: "second"',
      'ECHO: 2, 3',
    ])
    expect(result.program.core.nodes).toContainEqual(expect.objectContaining({
      kind: 'box', size: [2, 3, 1],
    }))
  })

  it('soft-fails invalid built-ins and preserves mixed argument source order', () => {
    const result = lower(`
      echo(
        abs("x"),
        len([0:2]),
        cross([1], [2]),
        min([3, "x", 1]),
        pow(y = 3, 2)
      );
      cube(1);
    `)

    expect(result.warnings).toEqual([
      'abs() argument 1 must be a number',
      'len() argument 1 must be a string or vector',
      'cross() arguments must be matching 2D or 3D vectors',
      'ECHO: undef, undef, undef, 1, 9',
    ])
  })

  it('evaluates defaults in the definition environment rather than prior parameters', () => {
    const result = lower(`
      function f(y = 4, x = y) = x;
      module r(y = 3, x = y) echo(y, x) cube(1);
      echo(f());
      r();
    `)

    expect(result.warnings).toEqual([
      "Ignoring unknown variable 'y'",
      'ECHO: undef',
      'ECHO: 3, undef',
    ])
    expect(result.program.core.nodes).toHaveLength(1)
  })
})
