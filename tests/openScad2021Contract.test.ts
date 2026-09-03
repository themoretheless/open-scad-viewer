import { describe, expect, it } from 'vitest'
import {
  OPENSCAD_2021_01_BUILTIN_FUNCTIONS,
  OPENSCAD_2021_01_BUILTIN_MODULES,
  OPENSCAD_2021_01_COMMIT,
  OPENSCAD_2021_01_COMPATIBILITY_ALIASES,
  OPENSCAD_2021_01_CONSTANTS,
  OPENSCAD_2021_01_CONTRACT,
  OPENSCAD_2021_01_EXECUTION_RUNTIME,
  OPENSCAD_2021_01_FUNCTION_COUNT,
  OPENSCAD_2021_01_INDEPENDENT_ENGINE,
  OPENSCAD_2021_01_MODIFIERS,
  OPENSCAD_2021_01_MODULE_COUNT,
  OPENSCAD_2021_01_OPERATORS,
  OPENSCAD_2021_01_SMOKE_FIXTURES,
  OPENSCAD_2021_01_SPECIAL_VARIABLES,
  OPENSCAD_2021_01_SYNTAX_CONSTRUCTS,
  OPENSCAD_2021_01_TAG,
} from '../src/core/openScad2021Contract'

const EXPECTED_FUNCTIONS = [
  'abs', 'sign', 'rands', 'min', 'max', 'sin', 'cos', 'asin', 'acos', 'tan', 'atan', 'atan2',
  'round', 'ceil', 'floor', 'pow', 'sqrt', 'exp', 'len', 'log', 'ln', 'str', 'chr', 'ord',
  'concat', 'lookup', 'search', 'version', 'version_num', 'norm', 'cross', 'parent_module',
  'is_undef', 'is_list', 'is_num', 'is_bool', 'is_string', 'is_function',
] as const

const EXPECTED_MODULES = [
  'render', 'color', 'offset', 'group', 'text', 'import', 'cube', 'sphere', 'cylinder',
  'polyhedron', 'square', 'circle', 'polygon', 'scale', 'rotate', 'mirror', 'translate',
  'multmatrix', 'union', 'difference', 'intersection', 'linear_extrude', 'minkowski', 'hull',
  'resize', 'children', 'echo', 'assert', 'for', 'let', 'intersection_for', 'if', 'projection',
  'surface', 'rotate_extrude',
] as const

const EXPECTED_SYNTAX = [
  'line-comment', 'block-comment', 'empty-statement', 'statement-block', 'assignment',
  'module-definition', 'function-definition', 'anonymous-function', 'module-instantiation',
  'if-else-module', 'literal-number', 'literal-string', 'literal-boolean', 'literal-undefined',
  'vector', 'range', 'function-call', 'index-lookup', 'member-lookup', 'ternary', 'let-expression',
  'assert-expression', 'echo-expression', 'list-comprehension-for', 'list-comprehension-c-for',
  'list-comprehension-if', 'list-comprehension-let', 'list-comprehension-each',
  'named-and-positional-arguments', 'default-parameters', 'trailing-commas', 'recursive-modules',
  'recursive-functions', 'include-directive', 'use-directive',
] as const

const EXPECTED_OPERATOR_IDS = [
  'postfix-call', 'postfix-index', 'postfix-member', 'exponent', 'unary', 'multiplicative',
  'additive', 'comparison', 'equality', 'logical-and', 'logical-or', 'conditional',
] as const

const EXPECTED_CONSTANTS = ['true', 'false', 'undef', 'PI'] as const
const EXPECTED_SPECIAL_VARIABLES = [
  '$fn', '$fs', '$fa', '$t', '$preview', '$vpt', '$vpr', '$vpd', '$vpf', '$children', '$parent_modules',
] as const
const EXPECTED_MODIFIERS = ['!', '#', '%', '*'] as const
const EXPECTED_COMPATIBILITY_CALLABLES = [
  'assign', 'child', 'dxf_linear_extrude', 'dxf_rotate_extrude', 'import_stl', 'import_off',
  'import_dxf', 'dxf_dim', 'dxf_cross',
] as const

function expectExactUniqueInventory(actual: readonly string[], expected: readonly string[]): void {
  expect(actual).toHaveLength(expected.length)
  expect(new Set(actual).size).toBe(actual.length)
  expect([...actual].sort()).toEqual([...expected].sort())
}

describe('OpenSCAD 2021.01 stable language contract', () => {
  it('pins the exact stable source revision independently from the execution snapshot', () => {
    expect(OPENSCAD_2021_01_TAG).toBe('openscad-2021.01')
    expect(OPENSCAD_2021_01_COMMIT).toBe('41f58fe57c03457a3a8b4dc541ef5654ec3e8c78')
    expect(OPENSCAD_2021_01_CONTRACT).toMatchObject({
      schemaVersion: 1,
      id: 'openscad/stable-2021.01',
      languageTarget: {
        release: '2021.01',
        tag: 'openscad-2021.01',
        commit: '41f58fe57c03457a3a8b4dc541ef5654ec3e8c78',
      },
    })
    expect(OPENSCAD_2021_01_EXECUTION_RUNTIME).toMatchObject({
      channel: 'official-snapshot',
      role: 'qualification-oracle',
      version: '2026.09.01',
      relationshipToLanguageTarget: 'different-release-validated-against-target-not-contract-source',
    })
    expect(OPENSCAD_2021_01_EXECUTION_RUNTIME.version).not.toBe(OPENSCAD_2021_01_CONTRACT.languageTarget.release)
    expect(OPENSCAD_2021_01_INDEPENDENT_ENGINE).toEqual({
      provider: 'open-scad-viewer',
      engineId: 'open-scad-viewer/independent-2021.01-dev.1',
      stage: 'development',
      upstreamRuntimeFallback: false,
      completeLanguageClaim: false,
    })
  })

  it('contains exactly the 38 stable built-in functions once each', () => {
    expect(OPENSCAD_2021_01_FUNCTION_COUNT).toBe(38)
    expectExactUniqueInventory(
      OPENSCAD_2021_01_BUILTIN_FUNCTIONS.map(entry => entry.name),
      EXPECTED_FUNCTIONS,
    )
  })

  it('contains exactly the 35 non-deprecated built-in modules once each', () => {
    expect(OPENSCAD_2021_01_MODULE_COUNT).toBe(35)
    expectExactUniqueInventory(
      OPENSCAD_2021_01_BUILTIN_MODULES.map(entry => entry.name),
      EXPECTED_MODULES,
    )
  })

  it('pins the grammar inventory, operators, constants, special variables, and modifiers', () => {
    expectExactUniqueInventory(OPENSCAD_2021_01_SYNTAX_CONSTRUCTS.map(entry => entry.id), EXPECTED_SYNTAX)
    expectExactUniqueInventory(OPENSCAD_2021_01_OPERATORS.map(entry => entry.id), EXPECTED_OPERATOR_IDS)
    expectExactUniqueInventory(OPENSCAD_2021_01_CONSTANTS.map(entry => entry.name), EXPECTED_CONSTANTS)
    expectExactUniqueInventory(OPENSCAD_2021_01_SPECIAL_VARIABLES.map(entry => entry.name), EXPECTED_SPECIAL_VARIABLES)
    expectExactUniqueInventory(OPENSCAD_2021_01_MODIFIERS.map(entry => entry.token), EXPECTED_MODIFIERS)
  })

  it('pins the 2021.01 default camera and animation context', () => {
    expect(Object.fromEntries(
      OPENSCAD_2021_01_SPECIAL_VARIABLES.map(entry => [entry.name, entry.defaultValue]),
    )).toMatchObject({
      $t: 0,
      $preview: false,
      $vpt: [0, 0, 0],
      $vpr: [55, 0, 25],
      $vpd: 140,
      $vpf: 22.5,
    })
  })

  it('keeps the deprecated and legacy callable tail explicit and separate from stable counts', () => {
    expectExactUniqueInventory(
      OPENSCAD_2021_01_COMPATIBILITY_ALIASES.map(entry => entry.symbol),
      EXPECTED_COMPATIBILITY_CALLABLES,
    )
    const stableNames = new Set([
      ...OPENSCAD_2021_01_BUILTIN_FUNCTIONS.map(entry => entry.name),
      ...OPENSCAD_2021_01_BUILTIN_MODULES.map(entry => entry.name),
    ])
    for (const entry of OPENSCAD_2021_01_COMPATIBILITY_ALIASES) {
      expect(stableNames.has(entry.symbol), entry.symbol).toBe(false)
    }
  })

  it('gives every stable function and module a directly attributable smoke program', () => {
    const entries = [
      ...OPENSCAD_2021_01_BUILTIN_FUNCTIONS,
      ...OPENSCAD_2021_01_BUILTIN_MODULES,
      ...OPENSCAD_2021_01_COMPATIBILITY_ALIASES,
    ]
    for (const entry of entries) {
      expect(entry.smoke.call.trim().length, entry.name ?? entry.symbol).toBeGreaterThan(2)
      expect(entry.smoke.source, entry.name ?? entry.symbol).toContain(entry.smoke.call)
      expect(entry.smoke.source.trim().endsWith(';'), entry.name ?? entry.symbol).toBe(true)
      expect(['defined-value-and-3d-mesh', '3d-mesh']).toContain(entry.smoke.expected)
    }
  })

  it('resolves every smoke fixture reference to one unique machine-readable requirement', () => {
    const fixtureIds = OPENSCAD_2021_01_SMOKE_FIXTURES.map(fixture => fixture.id)
    expect(new Set(fixtureIds).size).toBe(fixtureIds.length)
    const knownFixtureIds = new Set(fixtureIds)
    const smokeCases = [
      ...OPENSCAD_2021_01_BUILTIN_FUNCTIONS.map(entry => entry.smoke),
      ...OPENSCAD_2021_01_BUILTIN_MODULES.map(entry => entry.smoke),
      ...OPENSCAD_2021_01_COMPATIBILITY_ALIASES.map(entry => entry.smoke),
    ]
    const referencedFixtureIds = new Set(smokeCases.flatMap(smoke => smoke.fixtureIds))
    for (const fixtureId of referencedFixtureIds) expect(knownFixtureIds.has(fixtureId), fixtureId).toBe(true)
    expect([...referencedFixtureIds].sort()).toEqual([...knownFixtureIds].sort())

    for (const fixture of OPENSCAD_2021_01_SMOKE_FIXTURES) {
      expect(fixture.path).toMatch(/^[A-Za-z0-9._-]+(?:\/[A-Za-z0-9._-]+)+$/)
      expect(fixture.path.split('/')).not.toContain('..')
      expect(fixture.mediaType).toContain('/')
      expect(fixture.requirements.length).toBeGreaterThan(0)
      if (fixture.provisioning === 'inline-text') expect(fixture.text?.length).toBeGreaterThan(0)
      else expect(fixture.text).toBeUndefined()
    }
  })

  it('records the complete file-facing surface and tail-call behavior needed by an MCP VFS', () => {
    expect(OPENSCAD_2021_01_CONTRACT.fileSemantics.directives.map(entry => entry.name)).toEqual(['include', 'use'])
    expect(OPENSCAD_2021_01_CONTRACT.fileSemantics.import.extensions).toEqual([
      'stl', 'off', 'dxf', 'nef3', '3mf', 'amf', 'svg',
    ])
    expect(OPENSCAD_2021_01_CONTRACT.fileSemantics.surface.formats).toEqual([
      'PNG', 'whitespace-separated DAT height grid',
    ])
    expect(OPENSCAD_2021_01_CONTRACT.fileSemantics.resolution.mcpBoundary).toContain('MEMFS')
    expect(OPENSCAD_2021_01_CONTRACT.executionSemantics.tailCallOptimization).toMatchObject({
      kind: 'direct-self-tail-calls',
      stableEvaluatorIterationLimit: 1_000_001,
    })
  })
})
