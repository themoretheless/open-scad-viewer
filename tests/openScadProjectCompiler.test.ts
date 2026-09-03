import { describe, expect, it } from 'vitest'
import {
  compileOpenSCAD,
  parseOpenScadProjectSource,
  type CallNode,
  type Statement,
} from '../src/services/openscadCompiler'
import { OpenScadProject, OpenScadProjectError } from '../src/services/openScadProject'
import {
  OPENSCAD_PROJECT_COMPILE_ERROR_CODES,
  OPENSCAD_PROJECT_MAX_DEPENDENCY_EXPANSIONS,
  OPENSCAD_PROJECT_MAX_EXPANDED_SYNTAX_NODES,
  OpenScadProjectCompileError,
  compileOpenScadProject,
} from '../src/services/openScadProjectCompiler'

function allFrozen(value: unknown): boolean {
  if (!value || typeof value !== 'object') return true
  return Object.isFrozen(value) && Object.values(value).every(allFrozen)
}

function containsDirective(value: unknown): boolean {
  if (!value || typeof value !== 'object') return false
  if ((value as { type?: unknown }).type === 'directive') return true
  return Object.values(value).some(containsDirective)
}

function calls(program: readonly Statement[]): CallNode[] {
  return program.filter((statement): statement is CallNode => statement.type === 'call')
}

function expectCompileError(
  operation: () => unknown,
  code: typeof OPENSCAD_PROJECT_COMPILE_ERROR_CODES[number],
): OpenScadProjectCompileError {
  try {
    operation()
  } catch (error) {
    expect(error).toBeInstanceOf(OpenScadProjectCompileError)
    expect(error).toMatchObject({ code })
    return error as OpenScadProjectCompileError
  }
  throw new Error(`Expected ${code}`)
}

describe('OpenSCAD full-profile dependency directives', () => {
  it('retains robust include/use nodes only in the full profile', () => {
    const source = [
      'include /* comment */ <lib/my part-v1.scad>;',
      'use <nested/café.scad>',
    ].join('\n')
    const program = parseOpenScadProjectSource(source)

    expect(program).toMatchObject([
      {
        type: 'directive',
        directive: 'include',
        path: 'lib/my part-v1.scad',
      },
      {
        type: 'directive',
        directive: 'use',
        path: 'nested/café.scad',
      },
    ])
    for (const statement of program) {
      if (statement.type !== 'directive') continue
      expect(source.slice(statement.pathSpan.start, statement.pathSpan.end)).toBe(statement.path)
    }
    expect(structuredClone(program)).toEqual(program)
    expect(allFrozen(program)).toBe(true)

    expect(() => compileOpenSCAD('include <lib/my part-v1.scad>'))
      .toThrow(expect.objectContaining({ code: 'E_FEATURE_INCLUDE' }))
    expect(() => compileOpenSCAD('use <unterminated'))
      .toThrow(expect.objectContaining({ code: 'E_FEATURE_USE' }))
    expect(() => compileOpenSCAD('include <part.scad>', {
      languageProfile: 'openscad/stable-2021.01',
    })).toThrow(/requires project compilation/)
  })

  it('positions malformed full-profile directive diagnostics', () => {
    for (const source of ['include <>', 'use <unterminated', 'include "part.scad"']) {
      expect(() => parseOpenScadProjectSource(source))
        .toThrow(expect.objectContaining({ name: 'OpenSCADParseError' }))
    }
  })
})

describe('OpenSCAD project compiler', () => {
  it('expands recursive includes in statement position and recomputes call identities once', () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        {
          kind: 'source',
          path: 'main.scad',
          source: 'cube(0); include <lib/part.scad>; cube(4);',
        },
        {
          kind: 'source',
          path: 'lib/part.scad',
          source: 'cube(1); include <nested/more.scad>; cube(3);',
        },
        { kind: 'source', path: 'lib/nested/more.scad', source: 'cube(2);' },
      ],
    })

    const program = compileOpenScadProject(project)
    const expandedCalls = calls(program)
    const flatCalls = calls(compileOpenSCAD('cube(0); cube(1); cube(2); cube(3); cube(4);'))

    expect(expandedCalls.map(call => (call.args._0 as { value: number }).value))
      .toEqual([0, 1, 2, 3, 4])
    expect(expandedCalls.map(call => call.sourcePath)).toEqual([
      'main.scad',
      'lib/part.scad',
      'lib/nested/more.scad',
      'lib/part.scad',
      'main.scad',
    ])
    expect(expandedCalls.map(call => call.operationId))
      .toEqual(flatCalls.map(call => call.operationId))
    expect(new Set(expandedCalls.map(call => call.operationId)).size).toBe(5)
    expect(structuredClone(program)).toEqual(program)
    expect(allFrozen(program)).toBe(true)
  })

  it('clones repeated include expansions instead of aliasing frozen parse trees', () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        {
          kind: 'source',
          path: 'main.scad',
          source: 'include <part.scad> include <part.scad>',
        },
        { kind: 'source', path: 'part.scad', source: 'translate([1, 0, 0]) cube(1);' },
      ],
    })

    const program = compileOpenScadProject(project)
    expect(program).toHaveLength(2)
    expect(program[0]).not.toBe(program[1])
    if (program[0]?.type !== 'call' || program[1]?.type !== 'call') {
      throw new Error('Expected repeated calls')
    }
    expect(program[0].children[0]).not.toBe(program[1].children[0])
    expect(program[0].operationId).not.toBe(program[1].operationId)
  })

  it('makes use recursive but imports only module and function definitions', () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        {
          kind: 'source',
          path: 'main.scad',
          source: 'use <lib/defs.scad> widget(); helper(); cube(scale_by(2));',
        },
        {
          kind: 'source',
          path: 'lib/defs.scad',
          source: [
            'ignored = 1;',
            'sphere(10);',
            'include <more.scad>',
            'module widget() { include <body.scad> }',
            'function scale_by(value) = value * 2;',
          ].join('\n'),
        },
        {
          kind: 'source',
          path: 'lib/more.scad',
          source: [
            'module helper() { cube(2); }',
            'function extra(value) = value;',
            'translate([9, 0, 0]) cube(9);',
          ].join('\n'),
        },
        {
          kind: 'source',
          path: 'lib/body.scad',
          source: 'color("red") cube(1);',
        },
      ],
    })

    const program = compileOpenScadProject(project)
    expect(program.map(statement => statement.type === 'call'
      ? `call:${statement.name}`
      : `${statement.type}:${statement.name}`)).toEqual([
      'module:helper',
      'function:extra',
      'module:widget',
      'function:scale_by',
      'call:widget',
      'call:helper',
      'call:cube',
    ])
    const widget = program.find(statement => statement.type === 'module' && statement.name === 'widget')
    expect(widget).toMatchObject({
      type: 'module',
      children: [{ type: 'call', name: 'color', children: [{ type: 'call', name: 'cube' }] }],
    })
    expect(containsDirective(program)).toBe(false)
  })

  it('allows repeated diamond dependencies without mistaking them for a cycle', () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        {
          kind: 'source',
          path: 'main.scad',
          source: 'include <left.scad> include <right.scad>',
        },
        { kind: 'source', path: 'left.scad', source: 'include <shared.scad>' },
        { kind: 'source', path: 'right.scad', source: 'include <shared.scad>' },
        { kind: 'source', path: 'shared.scad', source: 'cube(1);' },
      ],
    })

    const program = compileOpenScadProject(project)
    expect(calls(program).map(call => call.operationId)).toEqual([
      'op:root/call%3Acube%230',
      'op:root/call%3Acube%231',
    ])
    expect(containsDirective(program)).toBe(false)
  })

  it('reports missing, non-source, and cyclic dependencies with typed details', () => {
    const missing = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [{ kind: 'source', path: 'main.scad', source: 'include <lib/missing.scad>' }],
    })
    expect(expectCompileError(
      () => compileOpenScadProject(missing),
      'E_PROJECT_DEPENDENCY_MISSING',
    ).details).toMatchObject({
      directive: 'include',
      importer: 'main.scad',
      specifier: 'lib/missing.scad',
      path: 'lib/missing.scad',
    })

    const binary = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: 'use <lib/data.scad>' },
        { kind: 'blob', path: 'lib/data.scad', data: new Uint8Array([1]) },
      ],
    })
    expect(expectCompileError(
      () => compileOpenScadProject(binary),
      'E_PROJECT_DEPENDENCY_NOT_SOURCE',
    ).details).toMatchObject({ directive: 'use', path: 'lib/data.scad' })

    const cyclic = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: 'include <lib/a.scad>' },
        { kind: 'source', path: 'lib/a.scad', source: 'use <../main.scad>' },
      ],
    })
    const cycleError = expectCompileError(
      () => compileOpenScadProject(cyclic),
      'E_PROJECT_DEPENDENCY_CYCLE',
    )
    expect(cycleError.details.chain).toEqual(['main.scad', 'lib/a.scad', 'main.scad'])
    expect(Object.isFrozen(cycleError.details.chain)).toBe(true)
  })

  it('preserves typed path-containment failures from the project VFS', () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [{ kind: 'source', path: 'main.scad', source: 'include <../escape.scad>' }],
    })
    expect(() => compileOpenScadProject(project)).toThrow(expect.objectContaining({
      name: 'OpenScadProjectError',
      code: 'E_PROJECT_PATH_ESCAPE',
    } satisfies Partial<OpenScadProjectError>))
  })

  it('bounds repeated dependency expansion even when dependencies are empty', () => {
    const atLimit = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        {
          kind: 'source',
          path: 'main.scad',
          source: 'include <empty.scad>\n'.repeat(OPENSCAD_PROJECT_MAX_DEPENDENCY_EXPANSIONS),
        },
        { kind: 'source', path: 'empty.scad', source: '' },
      ],
    })
    expect(compileOpenScadProject(atLimit)).toEqual([])

    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        {
          kind: 'source',
          path: 'main.scad',
          source: 'include <empty.scad>\n'.repeat(OPENSCAD_PROJECT_MAX_DEPENDENCY_EXPANSIONS + 1),
        },
        { kind: 'source', path: 'empty.scad', source: '' },
      ],
    })

    const error = expectCompileError(
      () => compileOpenScadProject(project),
      'E_PROJECT_EXPANSION_LIMIT',
    )
    expect(error.details).toMatchObject({
      metric: 'dependency-expansions',
      limit: OPENSCAD_PROJECT_MAX_DEPENDENCY_EXPANSIONS,
      actual: OPENSCAD_PROJECT_MAX_DEPENDENCY_EXPANSIONS + 1,
    })
  })

  it('bounds expression-heavy expansions independently of statement count', () => {
    const values = Array.from({ length: 2_000 }, () => '0').join(',')
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        {
          kind: 'source',
          path: 'main.scad',
          source: 'include <heavy.scad>\n'.repeat(80),
        },
        { kind: 'source', path: 'heavy.scad', source: `values = [${values}];` },
      ],
    })

    const error = expectCompileError(
      () => compileOpenScadProject(project),
      'E_PROJECT_EXPANSION_LIMIT',
    )
    expect(error.details).toMatchObject({
      metric: 'syntax-nodes',
      limit: OPENSCAD_PROJECT_MAX_EXPANDED_SYNTAX_NODES,
      actual: expect.any(Number),
    })
    expect(error.details.actual).toBeGreaterThan(OPENSCAD_PROJECT_MAX_EXPANDED_SYNTAX_NODES)
  })

  it('keeps dependency parse diagnostics local to the failing source', () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: 'include <broken.scad>' },
        { kind: 'source', path: 'broken.scad', source: '\n cube(];' },
      ],
    })

    try {
      compileOpenScadProject(project)
    } catch (error) {
      expect(error).toMatchObject({
        name: 'OpenSCADParseError',
        line: 2,
        column: 7,
      })
      return
    }
    throw new Error('Expected dependency parse error')
  })

  it('publishes a frozen, unique project-compile error inventory', () => {
    expect(Object.isFrozen(OPENSCAD_PROJECT_COMPILE_ERROR_CODES)).toBe(true)
    expect(new Set(OPENSCAD_PROJECT_COMPILE_ERROR_CODES).size)
      .toBe(OPENSCAD_PROJECT_COMPILE_ERROR_CODES.length)
  })
})
