import { MAX_UINT32, Mt19937, UTF8 } from '../openScadBuiltinFunctions'
import type { BuiltinHandler, OpenScadBuiltinFunctionContext, OpenScadBuiltinValue } from '../openScadBuiltinFunctions'
import { TT } from '../openscadCompiler'
import type { Expr, ExpressionArgument, FunctionValue } from '../openscadCompiler'

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

export function nextUp(value: number): number {
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

export function minMax(name: 'min' | 'max'): BuiltinHandler {
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
export function hashFloatingPoint(value: number): number {
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

/** libc++/libstdc++ generate_canonical<double, 53> over mt19937. */
export function seededRandom(seed: number): () => number {
  const generator = new Mt19937(hashFloatingPoint(seed))
  return () => {
    const low = generator.next()
    const high = generator.next()
    return (low + high * 0x1_0000_0000) / 0x1_0000_0000_0000_0000
  }
}
