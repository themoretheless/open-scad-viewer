import { describe, expect, it } from 'vitest'
import { OpenSCADParseError } from '../src/services/openscadErrors'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'

const stable = { languageProfile: 'openscad/stable-2021.01' as const }

function lower(source: string) {
  return lowerOpenSCADToSemanticProgram(source, stable)
}

function projectError(source: string): OpenSCADParseError {
  try {
    lower(source)
  } catch (error) {
    if (error instanceof OpenSCADParseError) return error
    throw error
  }
  throw new Error('Expected single-source semantic lowering to require a project asset context')
}

describe('OpenSCAD 2021.01 semantic compatibility modules', () => {
  it('evaluates assign() named values in parallel, ignores positional values, and lets body bindings win', () => {
    const artifact = lower(`
      outer = 10;
      assign(
        a = echo("first") 1,
        b = outer + 1,
        a = echo("second") 2,
        echo("ignored positional") 99
      ) {
        a = 4;
        echo(a, b);
        cube([a, b, 1]);
      }
    `)

    expect(artifact.warnings).toEqual([
      'ECHO: "first"',
      'ECHO: "second"',
      'ECHO: 4, 11',
    ])
    expect(artifact.warnings).not.toContain('ECHO: "ignored positional"')
    expect(artifact.program.core.nodes).toEqual([
      expect.objectContaining({ kind: 'box', size: [4, 11, 1] }),
    ])
    expect(artifact.program.core.operations.some(operation => (
      operation.category === 'control' && operation.name === '$assign'
    ))).toBe(true)
  })

  it('implements child() as the first-argument-only children continuation', () => {
    const artifact = lower(`
      module pick(which) { child(which, echo("ignored extra") 99); }
      pick() cube(1);
      pick("not a number") cube(2);
      pick(-1) cube(3);
      pick(5) { cube(4); cube(5); }
    `)

    expect(artifact.warnings).toEqual([
      'child() will be removed in future releases. Use children() instead.',
      'Negative child index (-1) not allowed',
      'Child index (5) out of bounds (2 children)',
    ])
    expect(artifact.warnings).not.toContain('ECHO: "ignored extra"')
    expect(artifact.program.core.nodes
      .filter(node => node.kind === 'box')
      .map(node => node.size)).toEqual([
      [1, 1, 1],
      [2, 2, 2],
    ])
    expect(artifact.program.core.operations.some(operation => (
      operation.category === 'control' && operation.name === 'children'
    ))).toBe(true)
  })

  it('lowers dxf_linear_extrude() child mode with its historical slots and height fallback', () => {
    const historical = lower(`
      dxf_linear_extrude(
        undef, "ignored layer", 3, [0, 0], [2, -1], true, 30, 4, $fn = 8
      ) square(1);
    `)
    const fallback = lower('dxf_linear_extrude(echo("height") 2) square(1);')

    expect(historical.warnings).toEqual([
      'The dxf_linear_extrude() module will be removed in future releases. Use linear_extrude() instead.',
    ])
    expect(historical.program.core.nodes.at(-1)).toMatchObject({
      kind: 'linear-extrude',
      height: 3,
      twistDegrees: 30,
      slices: 4,
      scale: [2, 0],
      center: true,
    })
    expect(historical.program.core.operations[0]).toMatchObject({
      category: 'geometry',
      name: 'linear_extrude',
    })

    expect(fallback.warnings).toEqual([
      'The dxf_linear_extrude() module will be removed in future releases. Use linear_extrude() instead.',
      'ECHO: "height"',
    ])
    expect(fallback.program.core.nodes.at(-1)).toMatchObject({
      kind: 'linear-extrude',
      height: 2,
    })
  })

  it('lowers dxf_rotate_extrude() child mode and preserves its 2021 angle normalization', () => {
    const partial = lower(`
      dxf_rotate_extrude(file = undef, angle = 90, $fn = 12)
        translate([2, 0]) square(1);
    `)
    const normalized = lower(`
      dxf_rotate_extrude(file = "", angle = 450, $fn = 8)
        translate([2, 0]) square(1);
    `)

    expect(partial.warnings).toEqual([
      'The dxf_rotate_extrude() module will be removed in future releases. Use rotate_extrude() instead.',
    ])
    expect(partial.program.core.nodes.at(-1)).toMatchObject({
      kind: 'rotate-extrude-polygonal',
      angleDegrees: 90,
      radialSegments: 12,
    })
    expect(partial.program.core.operations[0]).toMatchObject({
      category: 'geometry',
      name: 'rotate_extrude',
    })
    expect(normalized.program.core.nodes.at(-1)).toMatchObject({
      kind: 'rotate-extrude-polygonal',
      angleDegrees: 360,
      radialSegments: 8,
    })
  })

  it('fails every asset-only alias and DXF query with a positioned project-context diagnostic', () => {
    const cases = [
      { source: '  import_stl(file = "part.data");', callable: 'import_stl()', line: 1, column: 3 },
      { source: '\n  import_off(file = "part.data");', callable: 'import_off()', line: 2, column: 3 },
      { source: '  import_dxf(file = "part.data");', callable: 'import_dxf()', line: 1, column: 3 },
      {
        source: '  dxf_linear_extrude(file = "part.dxf") square(1);',
        callable: 'dxf_linear_extrude()',
        line: 1,
        column: 3,
      },
      {
        source: '  dxf_rotate_extrude(file = "part.dxf") square(1);',
        callable: 'dxf_rotate_extrude()',
        line: 1,
        column: 3,
      },
      { source: 'echo(dxf_dim(file = "part.dxf"));', callable: 'dxf_dim()', line: 1, column: 6 },
      { source: 'echo(dxf_cross(file = "part.dxf"));', callable: 'dxf_cross()', line: 1, column: 6 },
    ] as const

    for (const specification of cases) {
      const error = projectError(specification.source)
      expect(error, specification.callable).toMatchObject({
        name: 'OpenSCADParseError',
        code: 'E_IMPORT_PROJECT_REQUIRED',
        line: specification.line,
        column: specification.column,
      })
      expect(error.detail).toContain(specification.callable)
      expect(error.detail).toContain('OpenSCAD project asset context')
      expect(error.start).toBeGreaterThanOrEqual(0)
      expect(error.end).toBeGreaterThan(error.start)
    }
  })
})
