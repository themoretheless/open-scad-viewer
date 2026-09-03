import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { OPENSCAD_2021_01_SYNTAX_CONSTRUCTS } from '../src/core/openScad2021Contract'
import {
  compileOpenSCAD,
  TT,
  type Expr,
} from '../src/services/openscadCompiler'
import { OpenSCADParseError } from '../src/services/openscadErrors'

const FULL_PROFILE = { languageProfile: 'openscad/stable-2021.01' } as const
type CanonicalSyntaxId = typeof OPENSCAD_2021_01_SYNTAX_CONSTRUCTS[number]['id']
const STAGE_ONE_EXPRESSION_SYNTAX_IDS = [
  'let-expression',
  'assert-expression',
  'echo-expression',
  'list-comprehension-for',
  'list-comprehension-c-for',
  'list-comprehension-if',
  'list-comprehension-let',
  'list-comprehension-each',
] as const satisfies readonly CanonicalSyntaxId[]

function assignedValue(source: string): Expr {
  const statement = compileOpenSCAD(source, FULL_PROFILE)[0]
  if (statement?.type !== 'assign') throw new Error('Expected one assignment')
  return statement.value
}

function allFrozen(value: unknown): boolean {
  if (!value || typeof value !== 'object') return true
  return Object.isFrozen(value) && Object.values(value).every(allFrozen)
}

describe('OpenSCAD compiler front-end', () => {
  it('produces structured-clone-safe, deeply frozen operation IR with stable spans and IDs', () => {
    const source = 'translate([1, 2, 3]) cube(size = 2);'
    const program = compileOpenSCAD(source)
    const translate = program[0]
    expect(translate).toMatchObject({
      type: 'call',
      name: 'translate',
      operationId: 'op:root/call%3Atranslate%230',
    })
    if (translate?.type !== 'call') throw new Error('Expected translate call')
    expect(source.slice(translate.argSpans._0.start, translate.argSpans._0.end)).toBe('[1, 2, 3]')
    expect(translate.children[0]).toMatchObject({
      type: 'call',
      name: 'cube',
      operationId: 'op:root/call%3Atranslate%230/children/call%3Acube%230',
    })
    expect(structuredClone(program)).toEqual(program)
    expect(allFrozen(program)).toBe(true)
  })

  it('keeps syntax diagnostics positioned without loading the geometry kernel', () => {
    expect(() => compileOpenSCAD('cube(];')).toThrow(OpenSCADParseError)
    try {
      compileOpenSCAD('\n cube(];')
    } catch (error) {
      expect(error).toMatchObject({ line: 2, column: 7 })
    }
    const compilerSource = readFileSync(fileURLToPath(new URL('../src/services/openscadCompiler.ts', import.meta.url)), 'utf8')
    expect(compilerSource).not.toContain('manifold-3d')
  })

  it('rejects unsupported language features with stable diagnostic codes', () => {
    expect(() => compileOpenSCAD('include <part.scad>')).toThrow(expect.objectContaining({ code: 'E_FEATURE_INCLUDE' }))
    expect(() => compileOpenSCAD('function f(x) = x;')).toThrow(expect.objectContaining({ code: 'E_FEATURE_USER_FUNCTION' }))
    expect(() => compileOpenSCAD('f = function(x) x;')).toThrow(expect.objectContaining({ code: 'E_FEATURE_USER_FUNCTION' }))
    expect(() => compileOpenSCAD('#cube(1);')).toThrow(expect.objectContaining({ code: 'E_FEATURE_VIEWPORT_MODIFIER' }))
    expect(compileOpenSCAD('*cube(1);')).toEqual([])
  })

  it('represents named and anonymous functions in the full 2021.01 profile', () => {
    const program = compileOpenSCAD(`
      function scale_by(value, factor = 2) = value * factor;
      callback = function(value) value + 1;
      cube([scale_by(3, factor = 4), callback(2), [7, 8, 9].z]);
    `, { languageProfile: 'openscad/stable-2021.01' })

    expect(program.map(statement => statement.type)).toEqual(['function', 'assign', 'call'])
    expect(program[0]).toMatchObject({
      type: 'function',
      name: 'scale_by',
      params: [{ name: 'value' }, { name: 'factor' }],
    })
    expect(structuredClone(program)).toEqual(program)
    expect(allFrozen(program)).toBe(true)
  })

  it('parses the stage-one full-profile forms by canonical 2021.01 syntax ID', () => {
    const cases: readonly {
      id: CanonicalSyntaxId
      source: string
      expected: object
    }[] = [
      {
        id: 'let-expression',
        source: 'value = let(a = 1, b = a + 1) b;',
        expected: {
          kind: 'let',
          args: [
            { name: 'a', value: { kind: 'literal', value: 1 } },
            { name: 'b', value: { kind: 'binary', op: TT.Plus } },
          ],
          body: { kind: 'identifier', name: 'b' },
        },
      },
      {
        id: 'assert-expression',
        source: 'value = assert(true, "ok") 7;',
        expected: {
          kind: 'assert',
          args: [
            { value: { kind: 'literal', value: true } },
            { value: { kind: 'literal', value: 'ok' } },
          ],
          body: { kind: 'literal', value: 7 },
        },
      },
      {
        id: 'echo-expression',
        source: 'value = echo("x") 9;',
        expected: {
          kind: 'echo',
          args: [{ value: { kind: 'literal', value: 'x' } }],
          body: { kind: 'literal', value: 9 },
        },
      },
      {
        id: 'list-comprehension-for',
        source: 'value = [for(i = [0:2]) i * i];',
        expected: {
          kind: 'vector',
          items: [{
            kind: 'lc-for',
            args: [{ name: 'i', value: { kind: 'range' } }],
            body: { kind: 'binary', op: TT.Star },
          }],
        },
      },
      {
        id: 'list-comprehension-c-for',
        source: 'value = [for(i = 0; i < 3; i = i + 1) i];',
        expected: {
          kind: 'vector',
          items: [{
            kind: 'lc-for-c',
            init: [{ name: 'i', value: { kind: 'literal', value: 0 } }],
            condition: { kind: 'binary', op: TT.Lt },
            update: [{ name: 'i', value: { kind: 'binary', op: TT.Plus } }],
            body: { kind: 'identifier', name: 'i' },
          }],
        },
      },
      {
        id: 'list-comprehension-if',
        source: 'value = [if(true) 1 else 2];',
        expected: {
          kind: 'vector',
          items: [{
            kind: 'lc-if',
            condition: { kind: 'literal', value: true },
            yes: { kind: 'literal', value: 1 },
            no: { kind: 'literal', value: 2 },
          }],
        },
      },
      {
        id: 'list-comprehension-let',
        source: 'value = [let(a = 1) each [a]];',
        expected: {
          kind: 'vector',
          items: [{
            kind: 'lc-let',
            args: [{ name: 'a', value: { kind: 'literal', value: 1 } }],
            body: { kind: 'lc-each', value: { kind: 'vector' } },
          }],
        },
      },
      {
        id: 'list-comprehension-each',
        source: 'value = [each [1, 2]];',
        expected: {
          kind: 'vector',
          items: [{ kind: 'lc-each', value: { kind: 'vector' } }],
        },
      },
    ]

    const canonicalIds = new Set(OPENSCAD_2021_01_SYNTAX_CONSTRUCTS.map(item => item.id))
    expect(cases.map(({ id }) => id)).toEqual(STAGE_ONE_EXPRESSION_SYNTAX_IDS)
    expect(cases.every(({ id }) => canonicalIds.has(id))).toBe(true)
    for (const testCase of cases) {
      expect(assignedValue(testCase.source), testCase.id).toMatchObject(testCase.expected)
    }
  })

  it('keeps terminal let expressions distinct from list-comprehension let', () => {
    expect(assignedValue('value = [let(a = 1) a];')).toMatchObject({
      kind: 'vector',
      items: [{
        kind: 'let',
        args: [{ name: 'a' }],
        body: { kind: 'identifier', name: 'a' },
      }],
    })
    expect(assignedValue('value = [for(i = [0:2]) if(i > 0) each [i, -i]];')).toMatchObject({
      kind: 'vector',
      items: [{
        kind: 'lc-for',
        body: {
          kind: 'lc-if',
          yes: { kind: 'lc-each' },
        },
      }],
    })
  })

  it('represents omitted assert/echo bodies separately from explicit undef', () => {
    const program = compileOpenSCAD(`
      a = assert(true);
      b = assert(true) undef;
      c = echo("x");
      d = echo("x") undef;
    `, FULL_PROFILE)
    const values = program.map(statement => statement.type === 'assign' ? statement.value : undefined)
    expect(values[0]).toMatchObject({ kind: 'assert' })
    expect(values[0]).not.toHaveProperty('body')
    expect(values[1]).toMatchObject({ kind: 'assert', body: { kind: 'literal', value: undefined } })
    expect(values[2]).toMatchObject({ kind: 'echo' })
    expect(values[2]).not.toHaveProperty('body')
    expect(values[3]).toMatchObject({ kind: 'echo', body: { kind: 'literal', value: undefined } })
  })

  it('uses the full-profile equality level without changing legacy precedence', () => {
    const fullLeft = assignedValue('value = 1 == 2 < 3;')
    const fullRight = assignedValue('value = 1 < 2 == true;')
    expect(fullLeft).toMatchObject({
      kind: 'binary',
      op: TT.EqEq,
      right: { kind: 'binary', op: TT.Lt },
    })
    expect(fullRight).toMatchObject({
      kind: 'binary',
      op: TT.EqEq,
      left: { kind: 'binary', op: TT.Lt },
    })

    const legacy = compileOpenSCAD('value = 1 == 2 < 3;')[0]
    if (legacy?.type !== 'assign') throw new Error('Expected legacy assignment')
    expect(legacy.value).toMatchObject({
      kind: 'binary',
      op: TT.Lt,
      left: { kind: 'binary', op: TT.EqEq },
    })
  })

  it('allows positional arguments after named arguments only in the full profile', () => {
    const value = assignedValue('value = f(a = 1, 2, c = 3, 4);')
    if (value.kind !== 'call') throw new Error('Expected expression call')
    expect(value.args.map(argument => argument.name)).toEqual(['a', undefined, 'c', undefined])
    expect(() => compileOpenSCAD('value = f(a = 1, 2);')).toThrow(
      expect.objectContaining({ message: expect.stringContaining('Positional arguments must precede named arguments') }),
    )
  })

  it('retains duplicate named module arguments in stable authored order', () => {
    const call = compileOpenSCAD(
      'probe(a = echo("first") 1, a = echo("second") 2, 3);',
      FULL_PROFILE,
    )[0]
    if (call?.type !== 'call') throw new Error('Expected module call')
    expect(call.callArguments?.map(argument => argument.name)).toEqual(['a', 'a', undefined])
    expect(call.callArguments?.map(argument => argument.value)).toHaveLength(3)
    expect(call.args.a).toMatchObject({ kind: 'echo' })
    expect(() => compileOpenSCAD('probe(a = 1, a = 2);')).toThrow(/Duplicate argument a/)
  })

  it('keeps wrapper expressions at the 2021.01 expr grammar level', () => {
    expect(assignedValue('value = let(a = 1) a ? 2 : 3;')).toMatchObject({
      kind: 'let',
      body: { kind: 'ternary' },
    })
    expect(assignedValue('value = true ? echo("x") 1 : assert(true) 0;')).toMatchObject({
      kind: 'ternary',
      yes: { kind: 'echo' },
      no: { kind: 'assert' },
    })
    expect(assignedValue('value = (let(a = 1) a) + 2;')).toMatchObject({
      kind: 'binary',
      op: TT.Plus,
      left: { kind: 'let' },
    })
    expect(() => assignedValue('value = 1 + let(a = 1) a;')).toThrow(OpenSCADParseError)
  })

  it('accepts transparent standalone blocks only in the full profile', () => {
    const syntaxId = 'statement-block' satisfies CanonicalSyntaxId
    expect(OPENSCAD_2021_01_SYNTAX_CONSTRUCTS.some(item => item.id === syntaxId)).toBe(true)
    const program = compileOpenSCAD('{ a = 1; cube(a); }', FULL_PROFILE)
    expect(program.map(statement => statement.type)).toEqual(['assign', 'call'])
    expect(() => compileOpenSCAD('{ a = 1; cube(a); }')).toThrow(OpenSCADParseError)
  })
})
