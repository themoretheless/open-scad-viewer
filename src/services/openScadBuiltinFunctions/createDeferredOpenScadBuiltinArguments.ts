import { DEG_TO_RAD, MAX_UINT32, RAD_TO_DEG, SQRT_ONE_THIRD, SQRT_THREE_QUARTERS, TRIG_HUGE_VALUE, UTF8 } from '../openScadBuiltinFunctions'
import type { BuiltinHandler, OpenScadBuiltinFunctionContext, OpenScadBuiltinValue, ReducedDegrees } from '../openScadBuiltinFunctions'
import { TT } from '../openscadCompiler'
import type { Expr, ExpressionArgument, FunctionValue } from '../openscadCompiler'
import { nextUp } from '../../core/float'

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

export function fail(context: OpenScadBuiltinFunctionContext, name: string, message: string): never {
  return context.error(`${name}() ${message}`)
}

export function warn(context: OpenScadBuiltinFunctionContext, name: string, message: string): void {
  context.warning?.(`${name}() ${message}`)
}

export function expectCount(
  context: OpenScadBuiltinFunctionContext,
  name: string,
  args: readonly OpenScadBuiltinValue[],
  counts: readonly number[],
): void {
  if (counts.includes(args.length)) return
  const expected = counts.length === 1 ? String(counts[0]) : counts.join(' or ')
  fail(context, name, `expects ${expected} argument${counts.every(count => count === 1) ? '' : 's'}`)
}

export function numberValue(
  context: OpenScadBuiltinFunctionContext,
  name: string,
  value: OpenScadBuiltinValue,
  argument: number,
): number {
  if (typeof value !== 'number') fail(context, name, `argument ${argument + 1} must be a number`)
  return value
}

export function finiteNumberValue(
  context: OpenScadBuiltinFunctionContext,
  name: string,
  value: OpenScadBuiltinValue,
  argument: number,
): number {
  const number = numberValue(context, name, value, argument)
  if (!Number.isFinite(number)) fail(context, name, `argument ${argument + 1} must be finite`)
  return number
}

export function arrayValue(
  context: OpenScadBuiltinFunctionContext,
  name: string,
  value: OpenScadBuiltinValue,
  argument: number,
): readonly OpenScadBuiltinValue[] {
  if (!Array.isArray(value)) fail(context, name, `argument ${argument + 1} must be a vector`)
  return value
}

export function registerArray(
  context: OpenScadBuiltinFunctionContext,
  name: string,
  items: readonly OpenScadBuiltinValue[],
): OpenScadBuiltinValue {
  return context.registerArray(items, `${name}() result`)
}

export function registerString(
  context: OpenScadBuiltinFunctionContext,
  name: string,
  value: string,
): OpenScadBuiltinValue {
  return context.registerString(value, `${name}() result`)
}

export function unaryNumber(name: string, operation: (value: number) => number): BuiltinHandler {
  return (args, context) => {
    expectCount(context, name, args, [1])
    return operation(numberValue(context, name, args[0], 0))
  }
}

export function binaryNumber(name: string, operation: (left: number, right: number) => number): BuiltinHandler {
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

export function reduceDegrees(value: number, period: number): ReducedDegrees | undefined {
  if (!(value < TRIG_HUGE_VALUE && value > -TRIG_HUGE_VALUE)) return undefined
  if (value >= 0 && value < period) return { angle: value, cycles: 0 }
  const cycles = Math.floor(value / period)
  return { angle: value - cycles * period, cycles }
}

export function firstQuadrantComponents(angle: number): readonly [sin: number, cos: number] {
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

export function unitCircleComponents(value: number): readonly [sin: number, cos: number] | undefined {
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

export function sinDegrees(value: number): number {
  return unitCircleComponents(value)?.[0] ?? Number.NaN
}

export function cosDegrees(value: number): number {
  return unitCircleComponents(value)?.[1] ?? Number.NaN
}

export function tanDegrees(value: number): number {
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

export function asinDegrees(value: number): number {
  const degrees = Math.asin(value) * RAD_TO_DEG
  const whole = roundAwayFromZero(degrees)
  return sinDegrees(whole) === value ? whole : degrees
}

export function acosDegrees(value: number): number {
  const degrees = Math.acos(value) * RAD_TO_DEG
  const whole = roundAwayFromZero(degrees)
  return cosDegrees(whole) === value ? whole : degrees
}

export function atanDegrees(value: number): number {
  const degrees = Math.atan(value) * RAD_TO_DEG
  const whole = roundAwayFromZero(degrees)
  return tanDegrees(whole) === value ? whole : degrees
}

export function atan2Degrees(y: number, x: number): number {
  const degrees = Math.atan2(y, x) * RAD_TO_DEG
  const whole = roundAwayFromZero(degrees)
  return Math.abs(degrees - whole) < 3e-14 ? whole : degrees
}

/** C++ std::round semantics: halfway cases round away from zero. */
export function roundAwayFromZero(value: number): number {
  if (!Number.isFinite(value) || value === 0) return value
  return Math.sign(value) * Math.floor(Math.abs(value) + 0.5)
}

export function formatNumber(value: number): string {
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

export function quotedExpressionString(value: string): string {
  return `"${value.replace(/[\t\n\r"\\]/gu, character => ({
    '\t': '\\t',
    '\n': '\\n',
    '\r': '\\r',
    '"': '\\"',
    '\\': '\\\\',
  })[character] ?? character)}"`
}

export function binaryToken(operator: TT): string {
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

export function expressionArguments(args: readonly ExpressionArgument[]): string {
  return args.map(argument => argument.name === undefined
    ? formatExpression(argument.value)
    : `${argument.name} = ${formatExpression(argument.value)}`).join(', ')
}

/** Canonical source-shaped representation used by str(function-value). */
export function formatExpression(expression: Expr): string {
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

export function formatFunction(value: Pick<FunctionValue, 'params' | 'body'>): string {
  const parameters = value.params.map(parameter => parameter.defaultValue === undefined
    ? parameter.name
    : `${parameter.name} = ${formatExpression(parameter.defaultValue)}`).join(', ')
  return `function(${parameters}) ${formatExpression(value.body)}`
}

export function isFunctionValue(value: OpenScadBuiltinValue): value is FunctionValue {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return false
  const candidate = value as Partial<FunctionValue>
  return candidate.kind === 'function-value'
    && Array.isArray(candidate.params)
    && typeof candidate.body === 'object'
    && candidate.body !== null
}

export function formatValue(
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

export function isRangeValue(value: OpenScadBuiltinValue): value is {
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

export function compareUtf8(left: string, right: string): number {
  const leftBytes = UTF8.encode(left)
  const rightBytes = UTF8.encode(right)
  const common = Math.min(leftBytes.length, rightBytes.length)
  for (let index = 0; index < common; index++) {
    if (leftBytes[index] !== rightBytes[index]) return leftBytes[index] < rightBytes[index] ? -1 : 1
  }
  return leftBytes.length - rightBytes.length
}

export function rangeItemCount(range: { readonly start: number; readonly step: number; readonly end: number }): number {
  const { start, step, end } = range
  if ([start, step, end].some(Number.isNaN)) return 0
  if ((step < 0 && start < end) || (step >= 0 && start > end)) return 0
  if (start === end || !Number.isFinite(step)) return 1
  if (!Number.isFinite(start) || !Number.isFinite(end) || step === 0) return MAX_UINT32
  const steps = Math.floor(nextUp((end - start) / step))
  if (!Number.isFinite(steps) || steps >= MAX_UINT32) return MAX_UINT32
  return Math.max(0, steps + 1)
}

export function compareValues(left: OpenScadBuiltinValue, right: OpenScadBuiltinValue): number | undefined {
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

export function equalValues(left: OpenScadBuiltinValue, right: OpenScadBuiltinValue): boolean {
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
