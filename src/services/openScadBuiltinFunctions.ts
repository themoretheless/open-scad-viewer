/**
 * Host-independent implementations of the stable OpenSCAD 2021.01 built-in
 * function inventory.
 *
 * This module only evaluates already-bound argument values. Parsing calls,
 * binding named/default arguments, lexical scope, user functions, diagnostics,
 * and geometry remain responsibilities of the host evaluator. Consequently,
 * having all names here is not by itself a full-language conformance claim.
 */

import {evaluateDegreeScalar as nativeDegreeMath} from './openScadDegreeMath'

import {
  TT,
  type Expr,
  type ExpressionArgument,
  type FunctionValue,
} from './openscadCompiler'
import { isWellFormedUnicode } from '../core/unicode'
import { nextUp } from '../core/float'
import { acosDegrees, arrayValue, asinDegrees, atan2Degrees, atanDegrees, binaryNumber, cosDegrees, equalValues, expectCount, fail, finiteNumberValue, formatValue, numberValue, registerArray, registerString, roundAwayFromZero, sinDegrees, tanDegrees, unaryNumber, warn } from './openScadBuiltinFunctions/createDeferredOpenScadBuiltinArguments'
import { characterString, minMax, predicate, returnMatches, searchNeedles, searchStringRows, seededRandom, unsignedSearchParameter } from './openScadBuiltinFunctions/isOpenScadBuiltinFunctionName'
export { createDeferredOpenScadBuiltinArguments } from './openScadBuiltinFunctions/createDeferredOpenScadBuiltinArguments'
export { isOpenScadBuiltinFunctionName, evaluateOpenScadBuiltinFunction } from './openScadBuiltinFunctions/isOpenScadBuiltinFunctionName'

export const OPENSCAD_2021_01_BUILTIN_FUNCTION_NAMES = Object.freeze([
  'abs', 'sign', 'rands', 'min', 'max', 'sin', 'cos', 'asin', 'acos', 'tan', 'atan', 'atan2',
  'round', 'ceil', 'floor', 'pow', 'sqrt', 'exp', 'len', 'log', 'ln', 'str', 'chr', 'ord',
  'concat', 'lookup', 'search', 'version', 'version_num', 'norm', 'cross', 'parent_module',
  'is_undef', 'is_list', 'is_num', 'is_bool', 'is_string', 'is_function',
] as const)

export type OpenScadBuiltinFunctionName = typeof OPENSCAD_2021_01_BUILTIN_FUNCTION_NAMES[number]

/** Values which can cross the built-in boundary before the future evaluator
 * gives function values a concrete public representation. */
export type OpenScadBuiltinValue =
  | number
  | string
  | boolean
  | undefined
  | readonly OpenScadBuiltinValue[]
  | object
  | ((...args: never[]) => unknown)

/**
 * Evaluation effects deliberately owned by the host:
 *
 * - `error` attaches source position and the host's diagnostic policy;
 * - registration methods account for value/string allocation budgets;
 * - `random` supplies the unseeded random stream (seeded rands is local and
 *   repeatable);
 * - stack and function-type queries depend on evaluator runtime state.
 */
export interface OpenScadBuiltinFunctionContext {
  readonly error: (message: string) => never
  /** Non-terminal diagnostics for calls which repair an input and continue.
   * Optional until both evaluator hosts expose positioned warning sinks. */
  readonly warning?: (message: string) => void
  readonly registerArray: (
    items: readonly OpenScadBuiltinValue[],
    label: string,
  ) => OpenScadBuiltinValue
  readonly registerString: (value: string, label: string) => OpenScadBuiltinValue
  readonly random: () => number
  readonly parentModule: (depth: number) => string | undefined
  readonly isFunction: (value: OpenScadBuiltinValue) => boolean
}

export type OpenScadBuiltinFunctionResult =
  | Readonly<{ recognized: false }>
  | Readonly<{ recognized: true; value: OpenScadBuiltinValue }>

export const NOT_RECOGNIZED: OpenScadBuiltinFunctionResult = Object.freeze({ recognized: false })
const MAX_GENERATED_ITEMS = 1_000_000
export const MAX_CHR_RANGE_ITEMS = 10_000
export const MAX_UINT32 = 0xffff_ffff
const HALF_MAX_DOUBLE = Number.MAX_VALUE / 2
export const UTF8 = new TextEncoder()

export type BuiltinHandler = (
  args: readonly OpenScadBuiltinValue[],
  context: OpenScadBuiltinFunctionContext,
) => OpenScadBuiltinValue

// OpenSCAD's public 2021.01 behavior requires exact results at common degree
// angles. Normalize once, solve a first-quadrant pair, and restore the signs
// by quadrant so those values also remain exact after full rotations.
export const DEG_TO_RAD = 0.017453292519943295769
export const RAD_TO_DEG = 57.2957795130823208767
export const SQRT_THREE_QUARTERS = 0.86602540378443859659
export const SQRT_ONE_THIRD = 0.57735026918962573106
export const TRIG_HUGE_VALUE = 360 * 2 ** 52

export interface ReducedDegrees {
  readonly angle: number
  readonly cycles: number
}

export class Mt19937 {
  private readonly state = new Uint32Array(624)
  private index = 624

  constructor(seed: number) {
    this.state[0] = seed >>> 0
    for (let index = 1; index < this.state.length; index++) {
      const previous = this.state[index - 1] ^ (this.state[index - 1] >>> 30)
      this.state[index] = (Math.imul(1_812_433_253, previous) + index) >>> 0
    }
  }

  next(): number {
    if (this.index >= this.state.length) this.twist()
    let value = this.state[this.index++]
    value ^= value >>> 11
    value ^= (value << 7) & 0x9d2c5680
    value ^= (value << 15) & 0xefc60000
    value ^= value >>> 18
    return value >>> 0
  }

  private twist(): void {
    for (let index = 0; index < this.state.length; index++) {
      const combined = (this.state[index] & 0x80000000)
        | (this.state[(index + 1) % this.state.length] & 0x7fffffff)
      this.state[index] = this.state[(index + 397) % this.state.length]
        ^ (combined >>> 1)
        ^ ((combined & 1) === 0 ? 0 : 0x9908b0df)
    }
    this.index = 0
  }
}

const rands: BuiltinHandler = (args, context) => {
  expectCount(context, 'rands', args, [3, 4])
  const minimumValue = args[0]
  if (typeof minimumValue !== 'number') return undefined
  let minimum = minimumValue
  if (!Number.isFinite(minimum)) {
    warn(context, 'rands', 'range minimum is non-finite; using a bounded minimum')
    minimum = -HALF_MAX_DOUBLE
  }
  const maximumValue = args[1]
  if (typeof maximumValue !== 'number') return undefined
  let maximum = maximumValue
  if (!Number.isFinite(maximum)) {
    warn(context, 'rands', 'range maximum is non-finite; using a bounded maximum')
    maximum = HALF_MAX_DOUBLE
  }
  if (maximum < minimum) [minimum, maximum] = [maximum, minimum]
  const countValue = args[2]
  if (typeof countValue !== 'number') return undefined
  let requestedCount = Math.abs(countValue)
  if (!Number.isFinite(requestedCount)) {
    warn(context, 'rands', 'result count is non-finite; using one result')
    requestedCount = 1
  }
  const count = Math.trunc(requestedCount)
  if (!Number.isSafeInteger(count) || count > MAX_GENERATED_ITEMS) {
    return fail(context, 'rands', `result exceeds ${MAX_GENERATED_ITEMS.toLocaleString('en-US')} items`)
  }
  let random = context.random
  if (args.length === 4) {
    const seed = args[3]
    if (typeof seed !== 'number') return undefined
    random = seededRandom(seed)
  }
  const result: number[] = []
  for (let index = 0; index < count; index++) {
    if (minimum === maximum) {
      result.push(minimum)
      continue
    }
    const unit = random()
    if (!Number.isFinite(unit) || unit < 0 || unit >= 1) {
      return fail(context, 'rands', 'random source must return a finite value in [0, 1)')
    }
    result.push(minimum + (maximum - minimum) * unit)
  }
  return registerArray(context, 'rands', result)
}

const len: BuiltinHandler = (args, context) => {
  expectCount(context, 'len', args, [1])
  const value = args[0]
  if (typeof value === 'string') return Array.from(value).length
  if (Array.isArray(value)) return value.length
  return fail(context, 'len', 'argument 1 must be a string or vector')
}

const log: BuiltinHandler = (args, context) => {
  expectCount(context, 'log', args, [1, 2])
  if (args.length === 1) return Math.log10(numberValue(context, 'log', args[0], 0))
  const base = numberValue(context, 'log', args[0], 0)
  const value = numberValue(context, 'log', args[1], 1)
  return Math.log(value) / Math.log(base)
}

const str: BuiltinHandler = (args, context) => (
  registerString(context, 'str', args.map(value => formatValue(value, context)).join(''))
)

const chr: BuiltinHandler = (args, context) => (
  registerString(context, 'chr', args.map(value => characterString(value, context)).join(''))
)

const ord: BuiltinHandler = (args, context) => {
  if (args.length === 0) return undefined
  if (args.length !== 1) return fail(context, 'ord', 'expects 1 argument')
  const value = args[0]
  if (typeof value !== 'string') return fail(context, 'ord', 'argument 1 must be a string')
  if (!isWellFormedUnicode(value)) return fail(context, 'ord', 'argument 1 must be valid Unicode')
  const first = Array.from(value)[0]
  return first === undefined ? undefined : first.codePointAt(0)
}

const concat: BuiltinHandler = (args, context) => {
  const result: OpenScadBuiltinValue[] = []
  for (const value of args) {
    const addition = Array.isArray(value) ? value.length : 1
    if (result.length + addition > MAX_GENERATED_ITEMS) {
      return fail(context, 'concat', `result exceeds ${MAX_GENERATED_ITEMS.toLocaleString('en-US')} items`)
    }
    if (Array.isArray(value)) {
      // Avoid Function#apply/spread argument limits for large, otherwise valid
      // vectors. The host allocation budget remains authoritative.
      for (const item of value) result.push(item)
    } else result.push(value)
  }
  return registerArray(context, 'concat', result)
}

const lookup: BuiltinHandler = (args, context) => {
  expectCount(context, 'lookup', args, [2])
  const positionValue = args[0]
  if (typeof positionValue !== 'number' || !Number.isFinite(positionValue)) {
    // The reference diagnostic formats a second evaluation of the first
    // expression and never reads the table on this path.
    void args[0]
    return fail(
      context,
      'lookup',
      typeof positionValue === 'number'
        ? 'argument 1 must be finite'
        : 'argument 1 must be a number',
    )
  }
  const position = positionValue
  const table = args[1]
  if (!Array.isArray(table) || table.length === 0) return undefined
  const rows = table
  const first = rows[0]
  if (!Array.isArray(first) || first.length !== 2
    || typeof first[0] !== 'number' || typeof first[1] !== 'number') return undefined
  let [lowPosition, lowValue] = first
  let [highPosition, highValue] = first
  for (let index = 1; index < rows.length; index++) {
    const row = rows[index]
    if (!Array.isArray(row) || row.length !== 2
      || typeof row[0] !== 'number' || typeof row[1] !== 'number') continue
    const [candidatePosition, candidateValue] = row
    if (candidatePosition <= position && (candidatePosition > lowPosition || lowPosition > position)) {
      lowPosition = candidatePosition
      lowValue = candidateValue
    }
    if (candidatePosition >= position && (candidatePosition < highPosition || highPosition < position)) {
      highPosition = candidatePosition
      highValue = candidateValue
    }
  }
  if (position <= lowPosition) return highValue
  if (position >= highPosition) return lowValue
  const fraction = (position - lowPosition) / (highPosition - lowPosition)
  return highValue * fraction + lowValue * (1 - fraction)
}

const search: BuiltinHandler = (args, context) => {
  if (args.length < 2) return fail(context, 'search', 'expects at least 2 arguments')
  const needle = args[0]
  const table = args[1]
  const maximum = args.length > 2
    ? unsignedSearchParameter(args[2])
    : 1
  const column = args.length > 3
    ? unsignedSearchParameter(args[3])
    : 0

  if (typeof needle === 'string' && typeof table === 'string') {
    const tableCharacters = Array.from(table)
    const output: OpenScadBuiltinValue[] = []
    for (const character of Array.from(needle)) {
      const matches: number[] = []
      for (let index = 0; index < tableCharacters.length; index++) {
        if (tableCharacters[index] === character) matches.push(index)
        if (maximum !== 0 && matches.length >= maximum) break
      }
      if (maximum === 1) {
        if (matches[0] !== undefined) output.push(matches[0])
      } else {
        output.push(returnMatches(context, matches, maximum))
      }
    }
    return registerArray(context, 'search', output)
  }

  const rows = Array.isArray(table) ? table : []
  if (typeof needle === 'number') {
    const matches: number[] = []
    for (let index = 0; index < rows.length; index++) {
      const row = rows[index]
      if ((column === 0 && equalValues(needle, row))
        || (Array.isArray(row) && column < row.length && equalValues(needle, row[column]))) matches.push(index)
      if (maximum !== 0 && matches.length >= maximum) break
    }
    return registerArray(context, 'search', matches)
  }
  if (typeof needle === 'string') {
    const characters = Array.from(needle)
    return searchStringRows(context, characters, rows, maximum, column)
  }
  if (Array.isArray(needle)) return searchNeedles(context, needle, rows, maximum, column, true)
  return undefined
}

const version: BuiltinHandler = (args, context) => {
  // The pinned 2021.01 implementation ignores surplus arguments.
  // The official 2021.01 release reports an explicit zero day component.
  return registerArray(context, 'version', [2021, 1, 0])
}

const versionNum: BuiltinHandler = (args, context) => {
  if (args.length === 0) return 20210100
  const value = args[0]
  if (!Array.isArray(value)) return undefined
  const parts = value
  if (parts.length !== 2 && parts.length !== 3) return undefined
  if (typeof parts[0] !== 'number' || typeof parts[1] !== 'number'
    || (parts.length === 3 && typeof parts[2] !== 'number')) return undefined
  const year = parts[0]
  const month = parts[1]
  const day = parts.length === 3 ? parts[2] as number : 0
  return year * 10_000 + month * 100 + day
}

const norm: BuiltinHandler = (args, context) => {
  expectCount(context, 'norm', args, [1])
  const value = args[0]
  if (!Array.isArray(value)) return undefined
  const vector = value
  let result = 0
  for (let index = 0; index < vector.length; index++) {
    const value = numberValue(context, 'norm', vector[index], index)
    result += value * value
  }
  return Math.sqrt(result)
}

const cross: BuiltinHandler = (args, context) => {
  expectCount(context, 'cross', args, [2])
  const leftValue = args[0]
  const rightValue = args[1]
  const left = arrayValue(context, 'cross', leftValue, 0)
  const right = arrayValue(context, 'cross', rightValue, 1)
  if (left.length !== right.length || (left.length !== 2 && left.length !== 3)) {
    return fail(context, 'cross', 'arguments must be matching 2D or 3D vectors')
  }
  if (left.length === 2) {
    // Historical 2021.01 behaviour coerces non-numeric 2D elements to zero;
    // 3D vectors use the stricter validation below.
    const a0 = typeof left[0] === 'number' ? left[0] : 0
    const a1 = typeof left[1] === 'number' ? left[1] : 0
    const b0 = typeof right[0] === 'number' ? right[0] : 0
    const b1 = typeof right[1] === 'number' ? right[1] : 0
    return a0 * b1 - a1 * b0
  }
  const a = left.map((value, index) => finiteNumberValue(context, 'cross', value, index))
  const b = right.map((value, index) => finiteNumberValue(context, 'cross', value, index))
  return registerArray(context, 'cross', [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
  ])
}

const parentModule: BuiltinHandler = (args, context) => {
  expectCount(context, 'parent_module', args, [0, 1])
  const value = args.length === 0 ? 1 : args[0]
  if (typeof value !== 'number') return undefined
  const rawDepth = value
  const depth = Number.isNaN(rawDepth)
    ? 0
    : rawDepth === Infinity
      ? 0x7fff_ffff
      : rawDepth === -Infinity
        ? -0x8000_0000
        : Math.trunc(rawDepth)
  if (depth < 0) {
    warn(context, 'parent_module', `negative index ${depth} is not allowed`)
    return undefined
  }
  const result = context.parentModule(depth)
  if (result === undefined) warn(context, 'parent_module', `index ${depth} is outside the active module stack`)
  return result
}

export const HANDLERS: Readonly<Record<OpenScadBuiltinFunctionName, BuiltinHandler>> = Object.freeze({
  abs: unaryNumber('abs', Math.abs),
  sign: unaryNumber('sign', value => value < 0 ? -1 : value > 0 ? 1 : 0),
  rands,
  min: minMax('min'),
  max: minMax('max'),
  sin: unaryNumber('sin', sinDegrees),
  cos: unaryNumber('cos', cosDegrees),
  asin: unaryNumber('asin', asinDegrees),
  acos: unaryNumber('acos', acosDegrees),
  tan: unaryNumber('tan', tanDegrees),
  atan: unaryNumber('atan', atanDegrees),
  atan2: binaryNumber('atan2', atan2Degrees),
  round: unaryNumber('round', roundAwayFromZero),
  ceil: unaryNumber('ceil', Math.ceil),
  floor: unaryNumber('floor', Math.floor),
  pow: binaryNumber('pow', (base, exponent) => base ** exponent),
  sqrt: unaryNumber('sqrt', Math.sqrt),
  exp: unaryNumber('exp', Math.exp),
  len,
  log,
  ln: unaryNumber('ln', Math.log),
  str,
  chr,
  ord,
  concat,
  lookup,
  search,
  version,
  version_num: versionNum,
  norm,
  cross,
  parent_module: parentModule,
  is_undef: predicate('is_undef', value => value === undefined),
  is_list: predicate('is_list', Array.isArray),
  is_num: predicate('is_num', value => typeof value === 'number' && !Number.isNaN(value)),
  is_bool: predicate('is_bool', value => typeof value === 'boolean'),
  is_string: predicate('is_string', value => typeof value === 'string'),
  is_function: (args, context) => {
    expectCount(context, 'is_function', args, [1])
    return context.isFunction(args[0])
  },
})

export const BUILTIN_NAMES = new Set<string>(OPENSCAD_2021_01_BUILTIN_FUNCTION_NAMES)
