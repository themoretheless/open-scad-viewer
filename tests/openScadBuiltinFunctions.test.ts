import { describe, expect, it } from 'vitest'
import { OPENSCAD_2021_01_BUILTIN_FUNCTIONS } from '../src/core/openScad2021Contract'
import { compileOpenSCAD, type AssignNode, type FunctionValue } from '../src/services/openscadCompiler'
import {
  OPENSCAD_2021_01_BUILTIN_FUNCTION_NAMES,
  evaluateOpenScadBuiltinFunction,
  isOpenScadBuiltinFunctionName,
  type OpenScadBuiltinFunctionContext,
  type OpenScadBuiltinValue,
} from '../src/services/openScadBuiltinFunctions'

const functionSentinel = Object.freeze({ kind: 'test-function' })

function context(overrides: Partial<OpenScadBuiltinFunctionContext> = {}) {
  const registeredArrays: Array<{ items: readonly OpenScadBuiltinValue[]; label: string }> = []
  const registeredStrings: Array<{ value: string; label: string }> = []
  const warnings: string[] = []
  const value: OpenScadBuiltinFunctionContext = {
    error(message): never { throw new TypeError(message) },
    warning(message) { warnings.push(message) },
    registerArray(items, label) {
      const copy = [...items]
      registeredArrays.push({ items: copy, label })
      return copy
    },
    registerString(text, label) {
      registeredStrings.push({ value: text, label })
      return text
    },
    random: () => 0.25,
    parentModule: depth => ['current', 'parent', 'grandparent'][depth],
    isFunction: candidate => candidate === functionSentinel,
    ...overrides,
  }
  return { value, registeredArrays, registeredStrings, warnings }
}

function evaluate(
  name: string,
  args: readonly OpenScadBuiltinValue[],
  evaluationContext = context().value,
): OpenScadBuiltinValue {
  const result = evaluateOpenScadBuiltinFunction(name, args, evaluationContext)
  if (!result.recognized) throw new Error(`Expected ${name} to be recognized`)
  return result.value
}

describe('shared OpenSCAD 2021.01 built-in functions', () => {
  it('owns exactly the canonical 38-name stable inventory and leaves user functions unresolved', () => {
    expect(OPENSCAD_2021_01_BUILTIN_FUNCTION_NAMES).toHaveLength(38)
    expect(new Set(OPENSCAD_2021_01_BUILTIN_FUNCTION_NAMES).size).toBe(38)
    expect([...OPENSCAD_2021_01_BUILTIN_FUNCTION_NAMES].sort()).toEqual(
      OPENSCAD_2021_01_BUILTIN_FUNCTIONS.map(entry => entry.name).sort(),
    )
    for (const name of OPENSCAD_2021_01_BUILTIN_FUNCTION_NAMES) {
      expect(isOpenScadBuiltinFunctionName(name), name).toBe(true)
    }
    expect(isOpenScadBuiltinFunctionName('user_defined')).toBe(false)
    expect(evaluateOpenScadBuiltinFunction('user_defined', [], context().value)).toEqual({ recognized: false })
  })

  it('evaluates the complete scalar math, trigonometry and sequence-length group', () => {
    expect(evaluate('abs', [-3])).toBe(3)
    expect(evaluate('sign', [-3])).toBe(-1)
    expect(evaluate('sign', [0])).toBe(0)
    expect(evaluate('min', [[3, 1, 2]])).toBe(1)
    expect(evaluate('max', [3, 1, 2])).toBe(3)
    expect(evaluate('min', [[3, 'x', 1]])).toBe(1)
    expect(evaluate('sin', [30])).toBeCloseTo(0.5)
    expect(evaluate('cos', [60])).toBeCloseTo(0.5)
    expect(evaluate('asin', [0.5])).toBeCloseTo(30)
    expect(evaluate('acos', [0.5])).toBeCloseTo(60)
    expect(evaluate('tan', [45])).toBeCloseTo(1)
    expect(evaluate('atan', [1])).toBeCloseTo(45)
    expect(evaluate('atan2', [1, 1])).toBeCloseTo(45)
    expect(evaluate('round', [2.5])).toBe(3)
    expect(evaluate('round', [-2.5])).toBe(-3)
    expect(evaluate('ceil', [2.1])).toBe(3)
    expect(evaluate('floor', [2.9])).toBe(2)
    expect(evaluate('pow', [2, 3])).toBe(8)
    expect(evaluate('sqrt', [9])).toBe(3)
    expect(evaluate('exp', [1])).toBeCloseTo(Math.E)
    expect(evaluate('len', ['A🂡Л'])).toBe(3)
    expect(evaluate('len', [[1, 2, 3]])).toBe(3)
    expect(evaluate('log', [100])).toBe(2)
    expect(evaluate('log', [2, 8])).toBe(3)
    expect(evaluate('ln', [Math.E])).toBeCloseTo(1)
    expect(evaluate('norm', [[3, 4]])).toBe(5)
  })

  it('matches exact 2021.01 degree-trigonometry reductions and special values', () => {
    expect(evaluate('sin', [30])).toBe(0.5)
    expect(evaluate('sin', [45])).toBe(Math.SQRT1_2)
    expect(evaluate('sin', [60])).toBe(0.8660254037844386)
    expect(Object.is(evaluate('sin', [180]), -0)).toBe(true)
    expect(evaluate('sin', [360])).toBe(0)
    expect(Object.is(evaluate('sin', [-180]), -0)).toBe(true)

    expect(evaluate('cos', [60])).toBe(0.5)
    expect(evaluate('cos', [90])).toBe(0)
    expect(evaluate('cos', [180])).toBe(-1)
    expect(evaluate('cos', [360])).toBe(1)
    expect(Object.is(evaluate('cos', [270]), -0)).toBe(true)

    expect(evaluate('tan', [30])).toBe(0.5773502691896257)
    expect(evaluate('tan', [45])).toBe(1)
    expect(evaluate('tan', [60])).toBe(Math.sqrt(3))
    expect(evaluate('tan', [90])).toBe(Infinity)
    expect(evaluate('tan', [-90])).toBe(-Infinity)
    expect(Object.is(evaluate('tan', [180]), -0)).toBe(true)
    expect(Object.is(evaluate('tan', [360]), 0)).toBe(true)

    expect(evaluate('asin', [0.5])).toBe(30)
    expect(evaluate('acos', [0.5])).toBe(60)
    expect(evaluate('atan', [1])).toBe(45)
    expect(evaluate('atan2', [1, 1])).toBe(45)
    expect(evaluate('atan2', [1, -1])).toBe(135)

    const hugeAngle = 360 * 2 ** 52
    expect(evaluate('sin', [hugeAngle])).toBeNaN()
    expect(evaluate('cos', [Infinity])).toBeNaN()
    expect(evaluate('tan', [Number.NaN])).toBeNaN()
    expect(evaluate('asin', [2])).toBeNaN()
  })

  it('uses the injected random stream when unseeded and a repeatable local stream when seeded', () => {
    expect(evaluate('rands', [0, 8, 3], context({ random: () => 0.25 }).value)).toEqual([2, 2, 2])
    expect(evaluate('rands', [4, 4, 3], context({ random: () => { throw new Error('must not run') } }).value))
      .toEqual([4, 4, 4])

    const first = evaluate('rands', [-1, 1, 5, 17])
    const repeated = evaluate('rands', [-1, 1, 5, 17])
    const other = evaluate('rands', [-1, 1, 5, 18])
    expect(first).toEqual(repeated)
    expect(other).not.toEqual(first)
    expect(first).toHaveLength(5)
    expect((first as number[]).every(value => value >= -1 && value < 1)).toBe(true)
    expect((first as number[])[0]).toBeCloseTo(-0.6364444562724489, 14)
    expect((first as number[])[1]).toBeCloseTo(0.6884676844489699, 14)
  })

  it('matches 2021.01 rands repairs, silent type rejection and non-finite seed hashing', () => {
    const repaired = context()
    expect(evaluate('rands', [Number.NaN, 2, Number.POSITIVE_INFINITY, 17], repaired.value))
      .toEqual([expect.any(Number)])
    expect(repaired.warnings).toEqual([
      'rands() range minimum is non-finite; using a bounded minimum',
      'rands() result count is non-finite; using one result',
    ])

    expect(evaluate('rands', ['0', 1, 2])).toBeUndefined()
    expect(evaluate('rands', [0, 1, 2, 'seed'])).toBeUndefined()
    expect(evaluate('rands', [0, 1, 3, Number.NaN])).toEqual([
      0.5928446165166826,
      0.8442657442565983,
      0.8579456199898299,
    ])
    expect(evaluate('rands', [0, 1, 3, Number.POSITIVE_INFINITY])).toEqual([
      0.07767170663409012,
      0.9976615881433353,
      0.38583915294608456,
    ])
  })

  it('preserves 2021.01 min/max generic-vector ordering and sequential NaN rules', () => {
    const range = (start: number, step: number, end: number) => ({ kind: 'range-value' as const, start, step, end })
    expect(evaluate('min', [['z', 'a', 'm']])).toBe('a')
    expect(evaluate('max', [[false, true, false]])).toBe(true)
    expect(evaluate('min', [[[2], [1, 9], [1, 8, 0], [1, 8]]])).toEqual([1, 8])
    expect(evaluate('min', [[range(3, -1, 1), range(1, 1, 3)]])).toEqual(range(1, 1, 3))
    expect(evaluate('min', [[3, 'incomparable', 1]])).toBe(1)
    expect(evaluate('max', [[3, undefined, 5]])).toBe(5)
    expect(evaluate('min', [1, Number.NaN, -1])).toBe(-1)
    expect(evaluate('max', [1, Number.NaN, 2])).toBe(2)
    expect(evaluate('min', [Number.NaN, 1])).toBeNaN()
    expect(() => evaluate('min', [[]])).toThrow('at least 1 vector element')
  })

  it('handles Unicode strings, shallow concatenation, lookup interpolation and search result shapes', () => {
    const state = context()
    expect(evaluate('str', ['Open', 'SCAD', 2021, true, [1, 'x']], state.value))
      .toBe('OpenSCAD2021true[1, "x"]')
    expect(evaluate('str', [{ kind: 'range-value', start: 0, step: 1, end: 2 }], state.value))
      .toBe('[0 : 1 : 2]')
    expect(evaluate('chr', [65, [0x1f0a1, 1051]], state.value)).toBe('A🂡Л')
    expect(evaluate('ord', ['🂡'], state.value)).toBe(0x1f0a1)
    expect(evaluate('ord', [''], state.value)).toBeUndefined()
    expect(evaluate('concat', [[1, 2], 3, [4]], state.value)).toEqual([1, 2, 3, 4])
    expect(evaluate('lookup', [0.5, [[0, 0], [1, 10]]], state.value)).toBe(5)
    expect(evaluate('lookup', [-1, [[0, 2], [1, 10]]], state.value)).toBe(2)
    expect(evaluate('lookup', [2, [[0, 2], [1, 10]]], state.value)).toBe(10)
    expect(evaluate('search', ['a', 'abcdabcd'], state.value)).toEqual([0])
    expect(evaluate('search', ['🂡a', 'a🂡a', 0], state.value)).toEqual([[1], [0, 2]])
    expect(evaluate('search', ['ab', [['a', 1], ['b', 2], ['a', 3]], 0], state.value))
      .toEqual([[0, 2], [1]])
    expect(evaluate('search', ['z', [['a', 1]]], state.value)).toEqual([])
    expect(evaluate('search', [3, [['a', 1], ['b', 3], ['c', 3]], 0, 1], state.value))
      .toEqual([1, 2])
    expect(evaluate('search', [['a', 'c'], [['a'], ['b'], ['c']], 1, 0], state.value)).toEqual([0, 2])
    expect(evaluate('search', [['z'], [['a']], 1, 0], state.value)).toEqual([[]])

    expect(state.registeredStrings.map(entry => entry.label)).toEqual([
      'str() result',
      'str() result',
      'chr() result',
    ])
    expect(state.registeredArrays.some(entry => entry.label === 'concat() result')).toBe(true)
    expect(state.registeredArrays.some(entry => entry.label === 'search() result')).toBe(true)
  })

  it('uses canonical six-digit str formatting and source-shaped function values', () => {
    const assignment = compileOpenSCAD(
      'f = function(x = 1, y) x + y * 2;',
      { languageProfile: 'openscad/stable-2021.01' },
    )[0] as AssignNode
    const expression = assignment.value
    if (expression.kind !== 'function') throw new Error('Expected a function expression fixture')
    const fn: FunctionValue = {
      kind: 'function-value',
      name: null,
      params: expression.params,
      body: expression.body,
      closure: new Map(),
    }

    expect(evaluate('str', [1_234_567, 1e-6, 999_999.5, -0, Infinity, -Infinity, Number.NaN]))
      .toBe('1.23457e+61e-61e+60inf-infnan')
    expect(evaluate('str', [fn])).toBe('function(x = 1, y) (x + (y * 2))')
    expect(evaluate('str', [[fn]])).toBe('[function(x = 1, y) (x + (y * 2))]')
  })

  it('implements chr range expansion and ord Unicode boundaries without spurious zero-arity failure', () => {
    const state = context()
    expect(evaluate('chr', [
      { kind: 'range-value', start: 65, step: 1, end: 67 },
      0,
      -1,
      0xd800,
      0x10ffff,
      0x110000,
    ], state.value)).toBe(`ABC${String.fromCodePoint(0x10ffff)}`)
    expect(evaluate('chr', [
      { kind: 'range-value', start: 65, step: 0, end: 65 },
    ], state.value)).toBe('')
    expect(evaluate('ord', [], state.value)).toBeUndefined()
    expect(state.warnings).toEqual([])
    expect(evaluate('ord', ['😀x'], state.value)).toBe(0x1f600)
    expect(() => evaluate('ord', ['\ud800'], state.value)).toThrow('valid Unicode')

    const tooLarge = context()
    expect(evaluate('chr', [{ kind: 'range-value', start: 1, step: 1, end: 10_000 }], tooLarge.value)).toBe('')
    expect(tooLarge.warnings).toEqual([
      'chr() range exceeds the 10,000-item character limit',
    ])
  })

  it('keeps lookup first-row validity, skips later malformed rows and accepts numeric non-finites', () => {
    expect(evaluate('lookup', [0.5, [['bad'], [0, 0], [1, 10]]])).toBeUndefined()
    expect(evaluate('lookup', [0.5, [[0, 0], ['bad'], [1, 10]]])).toBe(5)
    expect(evaluate('lookup', [0.5, [[0, 0, 99], [1, 10]]])).toBeUndefined()
    expect(evaluate('lookup', [0.5, [[0, Number.POSITIVE_INFINITY], [1, 10]]])).toBe(Infinity)
    expect(evaluate('lookup', [0.5, 'not-a-table'])).toBeUndefined()
  })

  it('matches the 2021.01 search overload shapes, coercions and surplus-argument behavior', () => {
    expect(evaluate('search', ['a', 'aba', -1])).toEqual([[0, 2]])
    expect(evaluate('search', ['a', 'aba', 'all'])).toEqual([[0, 2]])
    expect(evaluate('search', ['a', 'aba', 1.9])).toEqual([0])
    expect(evaluate('search', ['a', 'aba', 1, 0, 99])).toEqual([0])
    expect(evaluate('search', [2, [[1, 2], [2, 1]], 1, -1])).toEqual([])
    expect(evaluate('search', [[[1, 2]], [[1, 2]], 1, 0])).toEqual([0])
    expect(evaluate('search', [['missing'], [[1], [2]], 1, 0])).toEqual([[]])
    expect(evaluate('search', ['ab', [[1], ['b']], 0, 0])).toEqual([[], [1]])
    expect(evaluate('search', ['a', [['a'], []], 1, 0])).toEqual([0])
    expect(evaluate('search', ['a', [['a'], []], 0, 0])).toEqual([])
    expect(evaluate('search', [1, 'not-a-vector'])).toEqual([])
    expect(evaluate('search', ['a', true, 1])).toEqual([])
  })

  it('exposes the pinned language version, vector products, stack lookup and all type predicates', () => {
    const state = context()
    expect(evaluate('version', [], state.value)).toEqual([2021, 1, 0])
    expect(evaluate('version_num', [], state.value)).toBe(20210100)
    expect(evaluate('version_num', [[2021, 1]], state.value)).toBe(20210100)
    expect(evaluate('cross', [[1, 0], [0, 1]], state.value)).toBe(1)
    expect(evaluate('cross', [[1, 0, 0], [0, 1, 0]], state.value)).toEqual([0, 0, 1])
    expect(evaluate('parent_module', [0], state.value)).toBe('current')
    expect(evaluate('parent_module', [], state.value)).toBe('parent')
    expect(evaluate('parent_module', [99], state.value)).toBeUndefined()

    expect(evaluate('is_undef', [undefined], state.value)).toBe(true)
    expect(evaluate('is_undef', [false], state.value)).toBe(false)
    expect(evaluate('is_list', [[]], state.value)).toBe(true)
    expect(evaluate('is_num', [1], state.value)).toBe(true)
    expect(evaluate('is_num', [Number.NaN], state.value)).toBe(false)
    expect(evaluate('is_num', [Infinity], state.value)).toBe(true)
    expect(evaluate('is_bool', [false], state.value)).toBe(true)
    expect(evaluate('is_string', [''], state.value)).toBe(true)
    expect(evaluate('is_function', [functionSentinel], state.value)).toBe(true)
    expect(evaluate('is_function', [{}], state.value)).toBe(false)
  })

  it('matches silent/surplus version rules, norm overflow and the historical 2D cross coercion', () => {
    expect(evaluate('version', [1, 'ignored'])).toEqual([2021, 1, 0])
    expect(evaluate('version_num', [[2021, 1, 2], 'ignored'])).toBe(20210102)
    expect(evaluate('version_num', [[2021]])).toBeUndefined()
    expect(evaluate('version_num', ['2021.01'])).toBeUndefined()
    expect(evaluate('norm', [1])).toBeUndefined()
    expect(evaluate('norm', [[1e308, 1e308]])).toBe(Infinity)
    expect(evaluate('cross', [['not-a-number', 1], [2, 3]])).toBe(-2)
    expect(() => evaluate('cross', [['not-a-number', 1, 2], [2, 3, 4]])).toThrow('must be a number')
  })

  it('truncates parent_module indices and reports repairable stack-domain failures through warning', () => {
    const state = context()
    expect(evaluate('parent_module', [0.9], state.value)).toBe('current')
    expect(evaluate('parent_module', [-0.9], state.value)).toBe('current')
    expect(evaluate('parent_module', [Number.NaN], state.value)).toBe('current')
    expect(evaluate('parent_module', ['bad'], state.value)).toBeUndefined()
    expect(evaluate('parent_module', [-1], state.value)).toBeUndefined()
    expect(evaluate('parent_module', [99], state.value)).toBeUndefined()
    expect(state.warnings).toEqual([
      'parent_module() negative index -1 is not allowed',
      'parent_module() index 99 is outside the active module stack',
    ])
  })

  it('delegates invalid-call diagnostics and allocation limits to its host context', () => {
    const state = context()
    expect(() => evaluate('abs', [], state.value)).toThrow('abs() expects 1 argument')
    expect(() => evaluate('cross', [[1], [1]], state.value)).toThrow('matching 2D or 3D vectors')
    expect(() => evaluate('rands', [0, 1, 1_000_001], state.value)).toThrow('exceeds 1,000,000 items')

    const bounded = context({
      registerArray: (_items, label) => { throw new RangeError(`${label} exceeded host budget`) },
    })
    expect(() => evaluate('concat', [[1], [2]], bounded.value)).toThrow('concat() result exceeded host budget')
  })

  it('keeps fixed-arity and conversion failures soft-failable for every strict built-in family', () => {
    const unaryNumbers = [
      'abs', 'sign', 'sin', 'cos', 'asin', 'acos', 'tan', 'atan',
      'round', 'ceil', 'floor', 'sqrt', 'exp', 'ln',
    ] as const
    for (const name of unaryNumbers) {
      expect(() => evaluate(name, []), `${name} arity`).toThrow(`${name}() expects 1 argument`)
      expect(() => evaluate(name, ['bad']), `${name} type`).toThrow('must be a number')
    }
    for (const name of ['atan2', 'pow'] as const) {
      expect(() => evaluate(name, [1]), `${name} arity`).toThrow(`${name}() expects 2 arguments`)
      expect(() => evaluate(name, [1, 'bad']), `${name} type`).toThrow('must be a number')
    }

    expect(() => evaluate('len', [1])).toThrow('string or vector')
    expect(() => evaluate('log', [])).toThrow('expects 1 or 2 arguments')
    expect(() => evaluate('log', [2, 'bad'])).toThrow('must be a number')
    expect(() => evaluate('min', [])).toThrow('at least 1 argument')
    expect(() => evaluate('max', ['bad'])).toThrow('must be a number')
    expect(() => evaluate('lookup', [0])).toThrow('expects 2 arguments')
    expect(() => evaluate('lookup', ['bad', []])).toThrow('must be a number')
    expect(() => evaluate('search', [1])).toThrow('at least 2 arguments')
    expect(() => evaluate('norm', [])).toThrow('expects 1 argument')
    expect(() => evaluate('cross', [[1, 2]])).toThrow('expects 2 arguments')
    expect(() => evaluate('parent_module', [0, 1])).toThrow('expects 0 or 1 arguments')

    for (const name of ['is_undef', 'is_list', 'is_num', 'is_bool', 'is_string', 'is_function'] as const) {
      expect(() => evaluate(name, []), `${name} arity`).toThrow(`${name}() expects 1 argument`)
    }
  })
})
