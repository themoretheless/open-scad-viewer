import { describe, expect, it } from 'vitest'
import { OpenSCADParseError, parseOpenSCAD } from '../src/services/openscadParser'

const stableProfile = { languageProfile: 'openscad/stable-2021.01' as const }

describe('direct independent OpenSCAD 2021.01 expression semantics', () => {
  it('evaluates wrappers, comprehensions, tagged ranges and shared value operations', async () => {
    const result = await parseOpenSCAD(`
      echo(
        let(a = 1, b = a + 1) b,
        assert(true) 7,
        echo("inner") 9
      );
      echo(
        [for (i = [0:2]) i * i],
        [for (i = [1, 2], j = [i, i + 1]) [i, j]],
        [for (i = 0; i < 4; i = i + 1) i],
        [if (false) 1 else 2],
        [let (a = 2) each [a + 1, a + 2]],
        [each [1, 2], 3]
      );
      echo(
        [0:2], [0:2][1],
        [1, 2] + [3, 4],
        [1, 2] * [3, 4],
        [[1, 2], [3, 4]] * [5, 6],
        [7, 8, 9].y,
        "😀x"[0], [10, 20][1.9], [10][-1],
        1 / 3, 1.23456789,
        [1:1:0] ? true : false
      );
      cube(1);
    `, stableProfile)

    expect(result.warnings).toEqual([
      'ECHO: "inner"',
      'ECHO: 2, 7, 9',
      'ECHO: [0, 1, 4], [[1, 1], [1, 2], [2, 2], [2, 3]], [0, 1, 2, 3], [2], [3, 4], [1, 2, 3]',
      'begin is greater than the end, but step is positive',
      'ECHO: [0 : 1 : 2], 1, [4, 6], 11, [17, 39], 8, "😀", 20, undef, 0.333333, 1.23457, true',
    ])
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(1, 6)
  })

  it('soft-fails invalid built-ins and keeps built-in argument source order', async () => {
    const result = await parseOpenSCAD(`
      echo(
        abs("x"), len([0:2]), cross([1], [2]),
        min([3, "x", 1]), pow(y = 3, 2)
      );
      cube(1);
    `, stableProfile)

    expect(result.warnings).toEqual([
      'abs() argument 1 must be a number',
      'len() argument 1 must be a string or vector',
      'cross() arguments must be matching 2D or 3D vectors',
      'ECHO: undef, undef, undef, 1, 9',
    ])
    expect(result.meshes).toHaveLength(1)
  })

  it('binds mixed user function and module arguments to the first free formal', async () => {
    const result = await parseOpenSCAD(`
      function f(a, b, c) = [a, b, c];
      module m(a, b, c) cube([a, b, c]);
      echo(f(b = 2, 1, 3), f(a = 1, a = 2, 3));
      m(b = 2, 1, 3);
    `, stableProfile)

    expect(result.warnings).toEqual([
      'Argument a was specified more than once',
      'ECHO: [1, 2, 3], [2, 3, undef]',
    ])
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(6, 6)
  })

  it('evaluates duplicate module arguments once and lets the last named value win', async () => {
    const result = await parseOpenSCAD(`
      module m(a, b) { echo(a, b); cube([a, b, 1]); }
      m(a = echo("first") 1, a = echo("second") 2, 3);
    `, stableProfile)

    expect(result.warnings).toEqual([
      'Argument a was specified more than once',
      'ECHO: "first"',
      'ECHO: "second"',
      'ECHO: 2, 3',
    ])
    expect(result.volume).toBeCloseTo(6, 6)
  })

  it('evaluates every default parameter in the definition environment', async () => {
    const result = await parseOpenSCAD(`
      function f(y = 4, x = y) = x;
      module r(y = 3, x = y) echo(y, x) cube(1);
      echo(f());
      r();
    `, stableProfile)

    expect(result.warnings).toEqual([
      "Ignoring unknown variable 'y'",
      'ECHO: undef',
      'ECHO: 3, undef',
    ])
    expect(result.meshes).toHaveLength(1)
  })

  it('iterates tagged ranges and multiple bindings in for and intersection_for', async () => {
    const result = await parseOpenSCAD(`
      for (i = [0:1], j = [0:1])
        translate([i * 2, j * 2, 0]) cube(1);
      intersection_for (i = [0:1], j = [0:1])
        translate([i * 0.25, j * 0.25, 0]) cube(1);
    `, stableProfile)

    expect(result.warnings).toEqual([])
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(4, 6)
  })

  it('keeps a mismatched descending range tagged but iterates it as empty', async () => {
    const result = await parseOpenSCAD('echo([3:1]); for (i = [3:1]) cube(i);', stableProfile)

    expect(result.warnings).toEqual([
      'begin is greater than the end, but step is positive',
      'ECHO: [3 : 1 : 1]',
    ])
    expect(result.meshes).toEqual([])
  })

  it('keeps false expression assertions positioned and legacy invalid arithmetic fatal', async () => {
    await expect(parseOpenSCAD(
      'echo(assert(false, "boom") 7); cube(1);',
      stableProfile,
    )).rejects.toBeInstanceOf(OpenSCADParseError)
    await expect(parseOpenSCAD(
      'echo(assert(false, "boom") 7); cube(1);',
      stableProfile,
    )).rejects.toThrow(/Assertion .* failed: "boom"/)

    await expect(parseOpenSCAD('cube([1, 2] + 1);'))
      .rejects.toThrow(/arithmetic operand must be a finite number/)
  })
})
