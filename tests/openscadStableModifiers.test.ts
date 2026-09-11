import { describe, expect, it } from 'vitest'
import {
  compileOpenSCAD,
  type CallNode,
  type Statement,
} from '../src/services/openscadCompiler'
import { OpenScadProject } from '../src/services/openScadProject'
import {
  compileOpenScadProject,
  OpenScadProjectCompileError,
} from '../src/services/openScadProjectCompiler'
import { parseOpenSCAD, parseOpenScadProject } from '../src/services/openscadParser'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'

const stable = { languageProfile: 'openscad/stable-2021.01' as const }

function lower(source: string, quality: 'preview' | 'full' = 'full') {
  return lowerOpenSCADToSemanticProgram(source, { ...stable, quality })
}

function operationSlots(
  artifact: ReturnType<typeof lower>,
  name: string,
): readonly string[][] {
  const operationIds = new Set(
    artifact.program.core.operations
      .filter(operation => operation.name === name)
      .map(operation => operation.id),
  )
  return artifact.program.core.occurrences
    .filter(occurrence => operationIds.has(occurrence.operation))
    .map(occurrence => occurrence.dynamicSlots.map(slot => slot.name))
}

function calls(program: readonly Statement[]): CallNode[] {
  return program.filter((statement): statement is CallNode => statement.type === 'call')
}

function xBounds(meshes: readonly { readonly vertices: Float32Array }[]): [number, number] {
  let min = Infinity
  let max = -Infinity
  for (const mesh of meshes) {
    for (let index = 0; index < mesh.vertices.length; index += 6) {
      min = Math.min(min, mesh.vertices[index])
      max = Math.max(max, mesh.vertices[index])
    }
  }
  return [min, max]
}

describe('OpenSCAD 2021.01 viewport modifiers', () => {
  it('retains ordered modifier chains and exact token spans in the full AST', () => {
    const source = '  # %! * cube(1);'
    const statement = compileOpenSCAD(source, stable)[0]
    expect(statement?.type).toBe('call')
    if (statement?.type !== 'call') throw new Error('Expected call')

    expect(statement.viewportModifiers?.map(modifier => modifier.kind)).toEqual([
      'highlight', 'background', 'root', 'disable',
    ])
    expect(statement.viewportModifiers?.map(modifier => (
      source.slice(modifier.span.start, modifier.span.end)
    ))).toEqual(['#', '%', '!', '*'])
    expect(Object.isFrozen(statement.viewportModifiers)).toBe(true)

    // The frozen legacy profile keeps both its eager `*` removal and error.
    expect(compileOpenSCAD('*cube(1);')).toEqual([])
    expect(() => compileOpenSCAD('#cube(1);')).toThrow(
      expect.objectContaining({ code: 'E_FEATURE_VIEWPORT_MODIFIER' }),
    )
    expect(() => compileOpenSCAD('#value = 1;', stable)).toThrow(
      /may prefix only a module instantiation/,
    )
  })

  it('makes disable dominant and skips the complete subtree without effects', () => {
    const result = lower('*echo(assert(false), missing) cube(9); cube(2);')

    expect(result.warnings).toEqual([])
    expect(result.program.core.nodes).toHaveLength(1)
    expect(result.program.core.nodes[0]).toMatchObject({ kind: 'box', size: [2, 2, 2] })
    expect(result.program.core.operations.some(operation => operation.name === 'echo')).toBe(false)
  })

  it('publishes highlight as explicit occurrence metadata without changing geometry', () => {
    const result = lower('#cube(1); cube(2);')

    expect(result.program.core.nodes.map(node => node.kind)).toEqual(['box', 'box'])
    expect(operationSlots(result, 'cube')).toEqual([
      ['$viewport-highlight'],
      ['$evaluation'],
    ])
  })

  it('evaluates background effects, excludes its geometry from full, and layers preview metadata', () => {
    const source = '%echo("background") cube(1); translate([2, 0, 0]) cube(2);'
    const full = lower(source, 'full')
    const preview = lower(source, 'preview')

    expect(full.warnings).toEqual(['ECHO: "background"'])
    expect(full.program.core.nodes.map(node => node.kind)).toEqual(['box', 'transform'])
    expect(operationSlots(full, 'cube').flat()).not.toContain('$viewport-background')

    expect(preview.warnings).toEqual(['ECHO: "background"'])
    expect(preview.program.core.nodes.map(node => node.kind)).toEqual(['box', 'box', 'transform'])
    expect(operationSlots(preview, 'cube').flat()).toContain('$viewport-background')
    expect(preview.fullEquivalent).toBe(false)
  })

  it('selects the first root reached at runtime and detaches geometry ancestors', () => {
    const nested = lower('translate([9, 0, 0]) { cube(1); !sphere(2); } cylinder(3);')
    expect(nested.program.core.nodes).toHaveLength(1)
    expect(nested.program.core.nodes[0]).toMatchObject({ kind: 'sphere-polygonal', radius: 2 })

    const branch = lower('if (false) !cube(7); !sphere(3); cylinder(1);')
    expect(branch.program.core.nodes).toHaveLength(1)
    expect(branch.program.core.nodes[0]).toMatchObject({ kind: 'sphere-polygonal', radius: 3 })

    const loop = lower('for (i = [2:3]) !cube(i); sphere(1);')
    expect(loop.program.core.nodes).toHaveLength(1)
    expect(loop.program.core.nodes[0]).toMatchObject({ kind: 'box', size: [2, 2, 2] })

    const emptyCandidate = lower('!if (false) cube(7); sphere(1);')
    expect(emptyCandidate.program.core.nodes).toHaveLength(1)
    expect(emptyCandidate.program.core.nodes[0]).toMatchObject({
      kind: 'sphere-polygonal', radius: 1,
    })

    const emptyContainer = lower('!union(); sphere(1);')
    expect(emptyContainer.program.core.nodes).toEqual([])
  })

  it('locks nested roots after an ancestor wins and lets root override its own presentation chain', () => {
    const ancestor = lower('!translate([4, 0, 0]) { cube(1); !sphere(2); } cylinder(3);')
    expect(ancestor.program.core.nodes.filter(node => node.kind === 'transform')).toHaveLength(2)
    expect(ancestor.program.core.nodes.some(node => node.kind === 'box')).toBe(true)
    expect(ancestor.program.core.nodes.some(node => node.kind === 'sphere-polygonal')).toBe(true)

    const chained = lower('%#!cube(1); sphere(2);', 'preview')
    expect(chained.program.core.nodes).toHaveLength(1)
    expect(chained.program.core.nodes[0]).toMatchObject({ kind: 'box' })
    const slots = chained.program.core.occurrences.flatMap(occurrence => (
      occurrence.dynamicSlots.map(slot => slot.name)
    ))
    expect(slots).not.toContain('$viewport-highlight')
    expect(slots).not.toContain('$viewport-background')
  })

  it('preserves module parameters while stripping an outer invocation transform', () => {
    const result = lower(`
      module marked(size) { !cube(size); }
      translate([10, 0, 0]) marked(2);
      translate([20, 0, 0]) marked(3);
    `)

    expect(result.program.core.nodes).toHaveLength(1)
    expect(result.program.core.nodes[0]).toMatchObject({ kind: 'box', size: [2, 2, 2] })
  })

  it('preserves a non-empty children() continuation when that call becomes root', () => {
    const result = lower(`
      module pass() { !children(); }
      translate([10, 0, 0]) pass() cube(2);
      cube(3);
    `)

    expect(result.program.core.nodes).toHaveLength(1)
    expect(result.program.core.nodes[0]).toMatchObject({ kind: 'box', size: [2, 2, 2] })

    const nestedContinuation = lower(`
      module inner() { !children(); }
      module outer() { inner() children(); }
      outer() cube(2);
      cube(3);
    `)
    expect(nestedContinuation.program.core.nodes).toHaveLength(1)
    expect(nestedContinuation.program.core.nodes[0]).toMatchObject({ kind: 'box', size: [2, 2, 2] })
  })
})

describe('viewport modifiers in direct kernel evaluation', () => {
  it('keeps disabled subtrees entirely unevaluated', async () => {
    const result = await parseOpenSCAD(
      '*echo(assert(false), missing) cube(9); cube(2);',
      stable,
    )

    expect(result.warnings).toEqual([])
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(8, 6)
  })

  it('renders highlight normally and conservatively omits unrepresentable preview backgrounds', async () => {
    const highlighted = await parseOpenSCAD('#cube(2);', stable)
    const background = await parseOpenSCAD(
      '%echo("background") cube(3); cube(2);',
      { ...stable, quality: 'preview' },
    )

    expect(highlighted.meshes).toHaveLength(1)
    expect(highlighted.volume).toBeCloseTo(8, 6)
    expect(background.meshes).toHaveLength(1)
    expect(background.volume).toBeCloseTo(8, 6)
    expect(background.warnings).toEqual([
      'ECHO: "background"',
      'Viewport background (%) geometry is omitted in preview because the mesh result contract has no background-layer metadata',
    ])
  })

  it('detaches the first runtime root while retaining its dynamic values', async () => {
    const nested = await parseOpenSCAD(
      'translate([9, 0, 0]) { cube(1); !cube(2); } cube(3);',
      stable,
    )
    const branch = await parseOpenSCAD(
      'if (false) !cube(7); for (i = [2:3]) !cube(i); cube(4);',
      stable,
    )
    const ancestor = await parseOpenSCAD(
      '!translate([4, 0, 0]) { cube(1); !cube(2); } cube(3);',
      stable,
    )

    expect(nested.meshes).toHaveLength(1)
    expect(nested.volume).toBeCloseTo(8, 6)
    expect(xBounds(nested.meshes)).toEqual([0, 2])
    expect(branch.meshes).toHaveLength(1)
    expect(branch.volume).toBeCloseTo(8, 6)
    expect(ancestor.meshes).toHaveLength(1)
    expect(ancestor.volume).toBeCloseTo(8, 6)
    expect(xBounds(ancestor.meshes)).toEqual([4, 6])
  })

  it('keeps caller children when children() itself is the selected root', async () => {
    const result = await parseOpenSCAD(`
      module pass() { !children(); }
      translate([10, 0, 0]) pass() cube(2);
      cube(3);
    `, stable)

    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(8, 6)
  })

  it('distinguishes transparent empty roots from empty CSG containers', async () => {
    const transparent = await parseOpenSCAD('!if (false) cube(7); cube(2);', stable)
    const container = await parseOpenSCAD('!union(); cube(2);', stable)

    expect(transparent.volume).toBeCloseTo(8, 6)
    expect(container.meshes).toEqual([])
    expect(container.volume).toBe(0)
  })

  it('applies transferred include modifiers during project evaluation', async () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: '!include <part.scad> cube(9);' },
        { kind: 'source', path: 'part.scad', source: 'cube(2);' },
      ],
    })
    const result = await parseOpenScadProject(project)

    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(8, 6)
  })
})

describe('viewport modifiers through project expansion', () => {
  it('carries a modifier through recursive and empty includes with source provenance', () => {
    const recursive = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: '#include <a.scad> sphere(2);' },
        { kind: 'source', path: 'a.scad', source: 'include <nested/b.scad>' },
        { kind: 'source', path: 'nested/b.scad', source: 'cube(1);' },
      ],
    })
    const expanded = calls(compileOpenScadProject(recursive))
    expect(expanded.map(call => call.name)).toEqual(['cube', 'sphere'])
    expect(expanded[0].viewportModifiers).toMatchObject([{
      kind: 'highlight',
      token: '#',
      span: { start: 0, end: 1 },
      sourcePath: 'main.scad',
    }])

    const empty = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: '%include <empty.scad> cube(2);' },
        { kind: 'source', path: 'empty.scad', source: '' },
      ],
    })
    expect(calls(compileOpenScadProject(empty))[0].viewportModifiers).toMatchObject([{
      kind: 'background', sourcePath: 'main.scad',
    }])
  })

  it('rejects a transferred modifier whose first dependency statement is not a call', () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: '#include <values.scad> cube(1);' },
        { kind: 'source', path: 'values.scad', source: 'value = 1; cube(value);' },
      ],
    })

    expect(() => compileOpenScadProject(project)).toThrow(
      expect.objectContaining<Partial<OpenScadProjectCompileError>>({
        code: 'E_PROJECT_MODIFIER_TARGET',
      }),
    )
  })
})
