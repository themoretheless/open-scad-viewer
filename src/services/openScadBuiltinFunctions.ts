/**
 * Host-independent implementations of the stable OpenSCAD 2021.01 built-in
 * function inventory.
 *
 * This module only evaluates already-bound argument values. Parsing calls,
 * binding named/default arguments, lexical scope, user functions, diagnostics,
 * and geometry remain responsibilities of the host evaluator. Consequently,
 * having all names here is not by itself a full-language conformance claim.
 */

import {
  TT,
  type Expr,
  type ExpressionArgument,
  type FunctionValue,
} from './openscadCompiler'

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

/**
 * Preserve OpenSCAD's call-by-name boundary for built-ins. The reference
 * runtime exposes argument expressions through EvalContext and evaluates an
 * expression every time the selected built-in reads that position. An actual
 * Array keeps the existing handler API and iteration semantics while accessor
 * slots defer those reads to the host evaluator.
 */
export function createDeferredOpenScadBuiltinArguments(
  count: number,
  evaluate: (index: number) => OpenScadBuiltinValue,
): readonly OpenScadBuiltinValue[] {
  const values = new Array<OpenScadBuiltinValue>(count)
  for (let index = 0; index < count; index++) {
    Object.defineProperty(values, index, {
      configurable: false,
      enumerable: true,
      get: () => evaluate(index),
    })
  }
  return Object.freeze(values)
}

const NOT_RECOGNIZED: OpenScadBuiltinFunctionResult = Object.freeze({ recognized: false })
const MAX_GENERATED_ITEMS = 1_000_000
const MAX_CHR_RANGE_ITEMS = 10_000
const MAX_UINT32 = 0xffff_ffff
const HALF_MAX_DOUBLE = Number.MAX_VALUE / 2
const UTF8 = new TextEncoder()

type BuiltinHandler = (
  args: readonly OpenScadBuiltinValue[],
  context: OpenScadBuiltinFunctionContext,
) => OpenScadBuiltinValue

function fail(context: OpenScadBuiltinFunctionContext, name: string, message: string): never {
  return context.error(`${name}() ${message}`)
}

function warn(context: OpenScadBuiltinFunctionContext, name: string, message: string): void {
  context.warning?.(`${name}() ${message}`)
}

function expectCount(
  context: OpenScadBuiltinFunctionContext,
  name: string,
  args: readonly OpenScadBuiltinValue[],
  counts: readonly number[],
): void {
  if (counts.includes(args.length)) return
  const expected = counts.length === 1 ? String(counts[0]) : counts.join(' or ')
  fail(context, name, `expects ${expected} argument${counts.every(count => count === 1) ? '' : 's'}`)
}

function numberValue(
  context: OpenScadBuiltinFunctionContext,
  name: string,
  value: OpenScadBuiltinValue,
  argument: number,
): number {
  if (typeof value !== 'number') fail(context, name, `argument ${argument + 1} must be a number`)
  return value
}

function finiteNumberValue(
  context: OpenScadBuiltinFunctionContext,
  name: string,
  value: OpenScadBuiltinValue,
  argument: number,
): number {
  const number = numberValue(context, name, value, argument)
  if (!Number.isFinite(number)) fail(context, name, `argument ${argument + 1} must be finite`)
  return number
}

function arrayValue(
  context: OpenScadBuiltinFunctionContext,
  name: string,
  value: OpenScadBuiltinValue,
  argument: number,
): readonly OpenScadBuiltinValue[] {
  if (!Array.isArray(value)) fail(context, name, `argument ${argument + 1} must be a vector`)
  return value
}

function registerArray(
  context: OpenScadBuiltinFunctionContext,
  name: string,
  items: readonly OpenScadBuiltinValue[],
): OpenScadBuiltinValue {
  return context.registerArray(items, `${name}() result`)
}

function registerString(
  context: OpenScadBuiltinFunctionContext,
  name: string,
  value: string,
): OpenScadBuiltinValue {
  return context.registerString(value, `${name}() result`)
}

function unaryNumber(name: string, operation: (value: number) => number): BuiltinHandler {
  return (args, context) => {
    expectCount(context, name, args, [1])
    return operation(numberValue(context, name, args[0], 0))
  }
}

function binaryNumber(name: string, operation: (left: number, right: number) => number): BuiltinHandler {
  return (args, context) => {
    expectCount(context, name, args, [2])
    // The 2021.01 built-ins read both expressions before validating either
    // value, so side effects in the right slot remain observable.
    const left = args[0]
    const right = args[1]
    return operation(
      numberValue(context, name, left, 0),
      numberValue(context, name, right, 1),
    )
  }
}

// OpenSCAD's public 2021.01 behavior requires exact results at common degree
// angles. Normalize once, solve a first-quadrant pair, and restore the signs
// by quadrant so those values also remain exact after full rotations.
const DEG_TO_RAD = 0.017453292519943295769
const RAD_TO_DEG = 57.2957795130823208767
const SQRT_THREE_QUARTERS = 0.86602540378443859659
const SQRT_ONE_THIRD = 0.57735026918962573106
const TRIG_HUGE_VALUE = 360 * 2 ** 52

interface ReducedDegrees {
  readonly angle: number
  readonly cycles: number
}

function reduceDegrees(value: number, period: number): ReducedDegrees | undefined {
  if (!(value < TRIG_HUGE_VALUE && value > -TRIG_HUGE_VALUE)) return undefined
  if (value >= 0 && value < period) return { angle: value, cycles: 0 }
  const cycles = Math.floor(value / period)
  return { angle: value - cycles * period, cycles }
}

function firstQuadrantComponents(angle: number): readonly [sin: number, cos: number] {
  if (angle === 30) return [0.5, SQRT_THREE_QUARTERS]
  if (angle === 45) return [Math.SQRT1_2, Math.SQRT1_2]
  if (angle === 60) return [SQRT_THREE_QUARTERS, 0.5]
  if (angle < 45) {
    const radians = angle * DEG_TO_RAD
    return [Math.sin(radians), Math.cos(radians)]
  }
  const complement = (90 - angle) * DEG_TO_RAD
  return [Math.cos(complement), Math.sin(complement)]
}

function unitCircleComponents(value: number): readonly [sin: number, cos: number] | undefined {
  const reduced = reduceDegrees(value, 360)
  if (reduced === undefined) return undefined
  const { angle } = reduced
  if (angle === 0) return [angle, 1]
  if (angle === 90) return [1, 0]
  if (angle === 180) return [-0, -1]
  if (angle === 270) return [-1, -0]

  const quadrant = Math.floor(angle / 90)
  const [sin, cos] = firstQuadrantComponents(angle - quadrant * 90)
  if (quadrant === 0) return [sin, cos]
  if (quadrant === 1) return [cos, -sin]
  if (quadrant === 2) return [-sin, -cos]
  return [-cos, sin]
}

function sinDegrees(value: number): number {
  return unitCircleComponents(value)?.[0] ?? Number.NaN
}

function cosDegrees(value: number): number {
  return unitCircleComponents(value)?.[1] ?? Number.NaN
}

function tanDegrees(value: number): number {
  const reduced = reduceDegrees(value, 180)
  if (reduced === undefined) return Number.NaN
  if (reduced.angle === 0) return reduced.cycles % 2 === 0 ? 0 : -0
  if (reduced.angle === 90) return reduced.cycles % 2 === 0 ? Infinity : -Infinity

  const oppose = reduced.angle > 90
  const acute = oppose ? 180 - reduced.angle : reduced.angle
  const magnitude = acute === 30
    ? SQRT_ONE_THIRD
    : acute === 45
      ? 1
      : acute === 60
        ? Math.sqrt(3)
        : Math.tan(acute * DEG_TO_RAD)
  return oppose ? -magnitude : magnitude
}

function asinDegrees(value: number): number {
  const degrees = Math.asin(value) * RAD_TO_DEG
  const whole = roundAwayFromZero(degrees)
  return sinDegrees(whole) === value ? whole : degrees
}

function acosDegrees(value: number): number {
  const degrees = Math.acos(value) * RAD_TO_DEG
  const whole = roundAwayFromZero(degrees)
  return cosDegrees(whole) === value ? whole : degrees
}

function atanDegrees(value: number): number {
  const degrees = Math.atan(value) * RAD_TO_DEG
  const whole = roundAwayFromZero(degrees)
  return tanDegrees(whole) === value ? whole : degrees
}

function atan2Degrees(y: number, x: number): number {
  const degrees = Math.atan2(y, x) * RAD_TO_DEG
  const whole = roundAwayFromZero(degrees)
  return Math.abs(degrees - whole) < 3e-14 ? whole : degrees
}

/** C++ std::round semantics: halfway cases round away from zero. */
function roundAwayFromZero(value: number): number {
  if (!Number.isFinite(value) || value === 0) return value
  return Math.sign(value) * Math.floor(Math.abs(value) + 0.5)
}

function formatNumber(value: number): string {
  if (Number.isNaN(value)) return 'nan'
  if (value === Infinity) return 'inf'
  if (value === -Infinity) return '-inf'
  if (value === 0) return '0'

  // OpenSCAD 2021.01 renders numbers with six significant digits. Unlike
  // Number#toString, it switches to exponent notation once the rounded base-10
  // exponent reaches +6 or -6 and retains an explicit '+' on positive
  // exponents. JavaScript's toPrecision already performs the same rounding;
  // only the -6 boundary and insignificant zero trimming need normalizing.
  let rendered = value.toPrecision(6)
  const exponent = Math.floor(Math.log10(Math.abs(value)))
  if (!rendered.includes('e') && exponent <= -6) rendered = value.toExponential(5)

  const exponentAt = rendered.indexOf('e')
  const suffix = exponentAt < 0 ? '' : rendered.slice(exponentAt)
  let significand = exponentAt < 0 ? rendered : rendered.slice(0, exponentAt)
  if (significand.includes('.')) significand = significand.replace(/0+$/u, '').replace(/\.$/u, '')
  return significand + suffix
}

function quotedExpressionString(value: string): string {
  return `"${value.replace(/[\t\n\r"\\]/gu, character => ({
    '\t': '\\t',
    '\n': '\\n',
    '\r': '\\r',
    '"': '\\"',
    '\\': '\\\\',
  })[character] ?? character)}"`
}

function binaryToken(operator: TT): string {
  return ({
    [TT.Plus]: '+',
    [TT.Minus]: '-',
    [TT.Star]: '*',
    [TT.Slash]: '/',
    [TT.Percent]: '%',
    [TT.Caret]: '^',
    [TT.Lt]: '<',
    [TT.Gt]: '>',
    [TT.LtEq]: '<=',
    [TT.GtEq]: '>=',
    [TT.EqEq]: '==',
    [TT.NotEq]: '!=',
    [TT.And]: '&&',
    [TT.Or]: '||',
  } as Partial<Record<TT, string>>)[operator] ?? TT[operator]
}

function expressionArguments(args: readonly ExpressionArgument[]): string {
  return args.map(argument => argument.name === undefined
    ? formatExpression(argument.value)
    : `${argument.name} = ${formatExpression(argument.value)}`).join(', ')
}

/** Canonical source-shaped representation used by str(function-value). */
function formatExpression(expression: Expr): string {
  switch (expression.kind) {
    case 'literal': {
      if (typeof expression.value === 'string') return quotedExpressionString(expression.value)
      return formatValue(expression.value as OpenScadBuiltinValue, undefined, true)
    }
    case 'identifier': return expression.name
    case 'vector': return `[${expression.items.map(formatExpression).join(', ')}]`
    case 'range': return `[${formatExpression(expression.start)} : ${expression.step === undefined
      ? ''
      : `${formatExpression(expression.step)} : `}${formatExpression(expression.end)}]`
    case 'unary': return `${expression.op === TT.Not ? '!' : expression.op === TT.Minus ? '-' : '+'}${formatExpression(expression.value)}`
    case 'binary': return `(${formatExpression(expression.left)} ${binaryToken(expression.op)} ${formatExpression(expression.right)})`
    case 'ternary': return `(${formatExpression(expression.test)} ? ${formatExpression(expression.yes)} : ${formatExpression(expression.no)})`
    case 'function': return formatFunction(expression)
    case 'call': {
      const callee = formatExpression(expression.callee)
      return `${expression.callee.kind === 'function' ? `(${callee})` : callee}(${expressionArguments(expression.args)})`
    }
    case 'index': return `${formatExpression(expression.value)}[${formatExpression(expression.index)}]`
    case 'member': return `${formatExpression(expression.value)}.${expression.name}`
    case 'let': return `let(${expressionArguments(expression.args)}) ${formatExpression(expression.body)}`
    case 'assert': return `assert(${expressionArguments(expression.args)})${expression.body === undefined ? '' : ` ${formatExpression(expression.body)}`}`
    case 'echo': return `echo(${expressionArguments(expression.args)})${expression.body === undefined ? '' : ` ${formatExpression(expression.body)}`}`
    case 'lc-for': return `for(${expressionArguments(expression.args)}) (${formatExpression(expression.body)})`
    case 'lc-for-c': return `for(${expressionArguments(expression.init)}; ${formatExpression(expression.condition)}; ${expressionArguments(expression.update)}) (${formatExpression(expression.body)})`
    case 'lc-if': return `if(${formatExpression(expression.condition)}) (${formatExpression(expression.yes)})${expression.no === undefined ? '' : ` else (${formatExpression(expression.no)})`}`
    case 'lc-let': return `let(${expressionArguments(expression.args)}) (${formatExpression(expression.body)})`
    case 'lc-each': return `each (${formatExpression(expression.value)})`
  }
}

function formatFunction(value: Pick<FunctionValue, 'params' | 'body'>): string {
  const parameters = value.params.map(parameter => parameter.defaultValue === undefined
    ? parameter.name
    : `${parameter.name} = ${formatExpression(parameter.defaultValue)}`).join(', ')
  return `function(${parameters}) ${formatExpression(value.body)}`
}

function isFunctionValue(value: OpenScadBuiltinValue): value is FunctionValue {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return false
  const candidate = value as Partial<FunctionValue>
  return candidate.kind === 'function-value'
    && Array.isArray(candidate.params)
    && typeof candidate.body === 'object'
    && candidate.body !== null
}

function formatValue(
  value: OpenScadBuiltinValue,
  context?: OpenScadBuiltinFunctionContext,
  nested = false,
): string {
  if (value === undefined) return 'undef'
  if (typeof value === 'boolean') return value ? 'true' : 'false'
  if (typeof value === 'number') return formatNumber(value)
  if (typeof value === 'string') return nested ? `"${value}"` : value
  if (Array.isArray(value)) return `[${value.map(item => formatValue(item, context, true)).join(', ')}]`
  if (isRangeValue(value)) {
    return `[${formatNumber(value.start)} : ${formatNumber(value.step)} : ${formatNumber(value.end)}]`
  }
  if (isFunctionValue(value)) return formatFunction(value)
  if (typeof value === 'function' || context?.isFunction(value) === true) return 'function(...)'
  return String(value)
}

function isRangeValue(value: OpenScadBuiltinValue): value is {
  readonly kind: 'range-value'
  readonly start: number
  readonly step: number
  readonly end: number
} {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return false
  const candidate = value as Partial<{ kind: string; start: unknown; step: unknown; end: unknown }>
  return candidate.kind === 'range-value'
    && typeof candidate.start === 'number'
    && typeof candidate.step === 'number'
    && typeof candidate.end === 'number'
}

function compareUtf8(left: string, right: string): number {
  const leftBytes = UTF8.encode(left)
  const rightBytes = UTF8.encode(right)
  const common = Math.min(leftBytes.length, rightBytes.length)
  for (let index = 0; index < common; index++) {
    if (leftBytes[index] !== rightBytes[index]) return leftBytes[index] < rightBytes[index] ? -1 : 1
  }
  return leftBytes.length - rightBytes.length
}

function nextUp(value: number): number {
  if (Number.isNaN(value) || value === Infinity) return value
  if (value === 0) return Number.MIN_VALUE
  const buffer = new ArrayBuffer(8)
  const view = new DataView(buffer)
  view.setFloat64(0, value, false)
  let bits = view.getBigUint64(0, false)
  bits = value > 0 ? bits + 1n : bits - 1n
  view.setBigUint64(0, bits, false)
  return view.getFloat64(0, false)
}

function rangeItemCount(range: { readonly start: number; readonly step: number; readonly end: number }): number {
  const { start, step, end } = range
  if ([start, step, end].some(Number.isNaN)) return 0
  if ((step < 0 && start < end) || (step >= 0 && start > end)) return 0
  if (start === end || !Number.isFinite(step)) return 1
  if (!Number.isFinite(start) || !Number.isFinite(end) || step === 0) return MAX_UINT32
  const steps = Math.floor(nextUp((end - start) / step))
  if (!Number.isFinite(steps) || steps >= MAX_UINT32) return MAX_UINT32
  return Math.max(0, steps + 1)
}

function compareValues(left: OpenScadBuiltinValue, right: OpenScadBuiltinValue): number | undefined {
  if (typeof left === 'number' && typeof right === 'number') return left < right ? -1 : left > right ? 1 : 0
  if (typeof left === 'string' && typeof right === 'string') return compareUtf8(left, right)
  if (typeof left === 'boolean' && typeof right === 'boolean') return Number(left) - Number(right)
  if (Array.isArray(left) && Array.isArray(right)) {
    const common = Math.min(left.length, right.length)
    for (let index = 0; index < common; index++) {
      const compared = compareValues(left[index], right[index])
      if (compared === undefined) return undefined
      if (compared !== 0) return compared
    }
    return left.length - right.length
  }
  if (isRangeValue(left) && isRangeValue(right)) {
    const leftCount = rangeItemCount(left)
    const rightCount = rangeItemCount(right)
    if (leftCount === 0 || rightCount === 0) {
      return leftCount === rightCount ? 0 : leftCount === 0 ? -1 : 1
    }
    if (left.start !== right.start) return left.start < right.start ? -1 : 1
    if (left.step !== right.step) return left.step < right.step ? -1 : 1
    return leftCount - rightCount
  }
  // Heterogeneous comparisons, undef/undef and function/function are
  // undefined in 2021.01. The min/max comparator converts that to false and
  // therefore preserves the value already selected.
  return undefined
}

function equalValues(left: OpenScadBuiltinValue, right: OpenScadBuiltinValue): boolean {
  if (Array.isArray(left) && Array.isArray(right)) {
    return left.length === right.length && left.every((value, index) => equalValues(value, right[index]))
  }
  if (isRangeValue(left) && isRangeValue(right)) {
    const leftCount = rangeItemCount(left)
    const rightCount = rangeItemCount(right)
    if (leftCount === 0) return rightCount === 0
    return rightCount !== 0 && left.start === right.start && left.step === right.step && leftCount === rightCount
  }
  return left === right
}

function minMax(name: 'min' | 'max'): BuiltinHandler {
  return (args, context) => {
    if (args.length === 0) return fail(context, name, 'expects at least 1 argument')
    const first = args[0]
    if (args.length === 1 && Array.isArray(first)) {
      const values = first
      if (values.length === 0) return fail(context, name, 'expects at least 1 vector element')
      let selected = values[0]
      for (let index = 1; index < values.length; index++) {
        const compared = name === 'min'
          ? compareValues(values[index], selected)
          : compareValues(selected, values[index])
        if (compared !== undefined && compared < 0) selected = values[index]
      }
      return selected
    }
    let selected = numberValue(context, name, first, 0)
    for (let index = 1; index < args.length; index++) {
      const value = numberValue(context, name, args[index], index)
      if ((name === 'min' && value < selected) || (name === 'max' && value > selected)) selected = value
    }
    return selected
  }
}

/** OpenSCAD 2021.01's 31-bit floating-point hash, used to seed mt19937. */
function hashFloatingPoint(value: number): number {
  if (!Number.isFinite(value)) return Number.isNaN(value) ? 0 : value > 0 ? 314159 : -314159
  if (value === 0) return 0

  let exponent = Math.floor(Math.log2(Math.abs(value))) + 1
  const divisor = 2 ** exponent
  let mantissa = Number.isFinite(divisor) && divisor !== 0
    ? value / divisor
    : value / (2 ** (exponent - 1)) / 2
  let sign = 1
  if (mantissa < 0) {
    sign = -1
    mantissa = -mantissa
  }

  const bits = 31n
  const modulus = (1n << bits) - 1n
  let hash = 0n
  while (mantissa !== 0) {
    hash = ((hash << 28n) & modulus) | (hash >> 3n)
    mantissa *= 268_435_456
    exponent -= 28
    const integral = Math.trunc(mantissa)
    mantissa -= integral
    hash += BigInt(integral)
    if (hash >= modulus) hash -= modulus
  }

  exponent = exponent >= 0 ? exponent % 31 : 30 - ((-1 - exponent) % 31)
  hash = ((hash << BigInt(exponent)) & modulus) | (hash >> BigInt(31 - exponent))
  return Number(BigInt.asIntN(32, hash * BigInt(sign)))
}

class Mt19937 {
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

/** libc++/libstdc++ generate_canonical<double, 53> over mt19937. */
function seededRandom(seed: number): () => number {
  const generator = new Mt19937(hashFloatingPoint(seed))
  return () => {
    const low = generator.next()
    const high = generator.next()
    return (low + high * 0x1_0000_0000) / 0x1_0000_0000_0000_0000
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
    return fail(context, 'rands', `result exceeds ${MAX_GENERATED_ITEMS.toLocaleString()} items`)
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

function characterString(
  value: OpenScadBuiltinValue,
  context: OpenScadBuiltinFunctionContext,
): string {
  if (typeof value === 'number') {
    const codePoint = Math.trunc(value)
    if (!(codePoint > 0) || codePoint > 0x10ffff || (codePoint >= 0xd800 && codePoint <= 0xdfff)) return ''
    return String.fromCodePoint(codePoint)
  }
  if (Array.isArray(value)) return value.map(item => characterString(item, context)).join('')
  if (isRangeValue(value)) {
    const count = rangeItemCount(value)
    if (count >= MAX_CHR_RANGE_ITEMS) {
      warn(context, 'chr', `range exceeds the ${MAX_CHR_RANGE_ITEMS.toLocaleString()}-item character limit`)
      return ''
    }
    // RangeType::numValues() reports one item when all three fields are equal,
    // but its 2021.01 iterator is nevertheless empty for every zero step.
    if (value.step === 0) return ''
    let result = ''
    for (let index = 0; index < count; index++) {
      result += characterString(value.start + value.step * index, context)
    }
    return result
  }
  return ''
}

const chr: BuiltinHandler = (args, context) => (
  registerString(context, 'chr', args.map(value => characterString(value, context)).join(''))
)

function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false
  }
  return true
}

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
      return fail(context, 'concat', `result exceeds ${MAX_GENERATED_ITEMS.toLocaleString()} items`)
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

function returnMatches(
  context: OpenScadBuiltinFunctionContext,
  matches: readonly number[],
  maximum: number,
): OpenScadBuiltinValue {
  const selected = maximum === 0 ? matches : matches.slice(0, maximum)
  return registerArray(context, 'search', selected)
}

function searchNeedles(
  context: OpenScadBuiltinFunctionContext,
  needles: readonly OpenScadBuiltinValue[],
  table: readonly OpenScadBuiltinValue[],
  maximum: number,
  column: number,
  includeEmptyFirstMatch: boolean,
): OpenScadBuiltinValue {
  const output: OpenScadBuiltinValue[] = []
  for (const needle of needles) {
    const matches: number[] = []
    for (let index = 0; index < table.length; index++) {
      const row = table[index]
      if ((column === 0 && equalValues(needle, row))
        || (Array.isArray(row) && column < row.length && equalValues(needle, row[column]))) matches.push(index)
      if (maximum !== 0 && matches.length >= maximum) break
    }
    if (maximum === 1) {
      if (matches[0] !== undefined) output.push(matches[0])
      else if (includeEmptyFirstMatch) output.push(registerArray(context, 'search', []))
    }
    else output.push(returnMatches(context, matches, maximum))
  }
  return registerArray(context, 'search', output)
}

function unsignedSearchParameter(value: OpenScadBuiltinValue): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) return 0
  return Math.trunc(value) >>> 0
}

function searchStringRows(
  context: OpenScadBuiltinFunctionContext,
  characters: readonly string[],
  rows: readonly OpenScadBuiltinValue[],
  maximum: number,
  column: number,
): OpenScadBuiltinValue {
  const output: OpenScadBuiltinValue[] = []
  for (const character of characters) {
    const matches: number[] = []
    for (let index = 0; index < rows.length; index++) {
      const row = rows[index]
      if (!Array.isArray(row) || row.length <= column) {
        warn(context, 'search', `table row ${index} does not contain column ${column}`)
        return registerArray(context, 'search', [])
      }
      const candidate = Array.from(formatValue(row[column], context))[0]
      if (candidate === character) matches.push(index)
      if (maximum !== 0 && matches.length >= maximum) break
    }
    if (matches.length === 0) warn(context, 'search', `term ${character} was not found`)
    if (maximum === 1) {
      if (matches[0] !== undefined) output.push(matches[0])
    } else output.push(returnMatches(context, matches, maximum))
  }
  return registerArray(context, 'search', output)
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

function predicate(name: string, test: (value: OpenScadBuiltinValue) => boolean): BuiltinHandler {
  return (args, context) => {
    expectCount(context, name, args, [1])
    return test(args[0])
  }
}

const HANDLERS: Readonly<Record<OpenScadBuiltinFunctionName, BuiltinHandler>> = Object.freeze({
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

const BUILTIN_NAMES = new Set<string>(OPENSCAD_2021_01_BUILTIN_FUNCTION_NAMES)

export function isOpenScadBuiltinFunctionName(name: string): name is OpenScadBuiltinFunctionName {
  return BUILTIN_NAMES.has(name)
}

/** Evaluate one already-bound call, or explicitly report that it is a user or
 * unknown function which the host must resolve. */
export function evaluateOpenScadBuiltinFunction(
  name: string,
  args: readonly OpenScadBuiltinValue[],
  context: OpenScadBuiltinFunctionContext,
): OpenScadBuiltinFunctionResult {
  if (!isOpenScadBuiltinFunctionName(name)) return NOT_RECOGNIZED
  return Object.freeze({ recognized: true, value: HANDLERS[name](args, context) })
}
