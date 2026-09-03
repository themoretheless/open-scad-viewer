import {
  TT,
  type FunctionValue,
  type RangeValue,
  type Value,
} from './openscadCompiler'

export interface OpenScadValueSemanticsContext {
  readonly warn: (message: string) => void
  readonly maxRangeItems: number
  readonly registerArray?: (values: Value[], label: string) => Value[]
}

export function isOpenScadRange(value: Value): value is RangeValue {
  return !Array.isArray(value) && typeof value === 'object' && value !== null
    && value.kind === 'range-value'
}

export function isOpenScadFunction(value: Value): value is FunctionValue {
  return !Array.isArray(value) && typeof value === 'object' && value !== null
    && value.kind === 'function-value'
}

export function openScadTruthy(value: Value): boolean {
  if (value === undefined || value === false) return false
  if (typeof value === 'number') return value !== 0 && !Number.isNaN(value)
  if (typeof value === 'string') return value.length > 0
  if (Array.isArray(value)) return value.length > 0
  // The range object remains truthy even when its direction makes iteration
  // empty (for example `[1:0]`); iteration and boolean conversion are distinct.
  if (isOpenScadRange(value)) return true
  return true
}

export function materializeOpenScadRange(
  range: RangeValue,
  context: OpenScadValueSemanticsContext,
): Value[] {
  const output: Value[] = []
  if (range.step === 0 || ![range.start, range.step, range.end].every(Number.isFinite)) {
    context.warn('Invalid range bounds produce an empty range')
    return register(output, 'range', context)
  }
  const forward = range.step > 0
  const epsilon = Math.max(1, Math.abs(range.start), Math.abs(range.end)) * 1e-12
  for (
    let value = range.start;
    forward ? value <= range.end + epsilon : value >= range.end - epsilon;
    value += range.step
  ) {
    if (output.length >= context.maxRangeItems) {
      context.warn(`Range exceeds ${context.maxRangeItems.toLocaleString()} items`)
      return []
    }
    output.push(value)
  }
  return register(output, 'range', context)
}

export function openScadUnary(
  operator: TT,
  value: Value,
  context: OpenScadValueSemanticsContext,
): Value {
  if (operator === TT.Not) return !openScadTruthy(value)
  if (typeof value === 'number') return operator === TT.Minus ? -value : value
  if (Array.isArray(value)) {
    return register(value.map(item => openScadUnary(operator, item, context)), 'unary vector', context)
  }
  context.warn(`Undefined unary operation (${token(operator)}${openScadType(value)})`)
  return undefined
}

export function openScadBinary(
  operator: TT,
  left: Value,
  right: Value,
  context: OpenScadValueSemanticsContext,
): Value {
  if (operator === TT.EqEq) return openScadDeepEqual(left, right)
  if (operator === TT.NotEq) return !openScadDeepEqual(left, right)
  if (operator === TT.And) return openScadTruthy(left) && openScadTruthy(right)
  if (operator === TT.Or) return openScadTruthy(left) || openScadTruthy(right)
  if (operator === TT.Lt || operator === TT.Gt || operator === TT.LtEq || operator === TT.GtEq) {
    const comparison = compareValues(left, right)
    if (comparison === undefined) {
      context.warn(`Undefined operation (${openScadType(left)} ${token(operator)} ${openScadType(right)})`)
      return undefined
    }
    if (operator === TT.Lt) return comparison < 0
    if (operator === TT.Gt) return comparison > 0
    if (operator === TT.LtEq) return comparison <= 0
    return comparison >= 0
  }

  if (operator === TT.Plus || operator === TT.Minus) {
    if (typeof left === 'number' && typeof right === 'number') {
      return operator === TT.Plus ? left + right : left - right
    }
    if (Array.isArray(left) && Array.isArray(right) && left.length === right.length) {
      return register(left.map((item, index) => openScadBinary(operator, item, right[index], context)), 'vector arithmetic', context)
    }
    return undefinedOperation(operator, left, right, context)
  }

  if (operator === TT.Star) return multiply(left, right, context)

  if (operator === TT.Slash) {
    if (typeof left === 'number' && typeof right === 'number') return left / right
    if (Array.isArray(left) && typeof right === 'number') {
      return register(left.map(item => openScadBinary(operator, item, right, context)), 'vector division', context)
    }
    if (typeof left === 'number' && Array.isArray(right)) {
      return register(right.map(item => openScadBinary(operator, left, item, context)), 'vector division', context)
    }
    return undefinedOperation(operator, left, right, context)
  }

  if (operator === TT.Percent || operator === TT.Caret) {
    if (typeof left === 'number' && typeof right === 'number') {
      return operator === TT.Percent ? left % right : left ** right
    }
    return undefinedOperation(operator, left, right, context)
  }

  return undefinedOperation(operator, left, right, context)
}

export function openScadIndex(
  value: Value,
  indexValue: Value,
  context: OpenScadValueSemanticsContext,
): Value {
  if (typeof indexValue !== 'number' || !Number.isFinite(indexValue)) {
    context.warn(`Undefined index ${formatOpenScadValue(indexValue)}`)
    return undefined
  }
  // Numeric indices are truncated toward zero by OpenSCAD.
  const index = Math.trunc(indexValue)
  if (index < 0) return undefined
  if (Array.isArray(value)) return value[index]
  if (typeof value === 'string') return Array.from(value)[index]
  if (isOpenScadRange(value)) {
    const candidate = value.start + value.step * index
    if (value.step > 0 ? candidate > value.end : candidate < value.end) return undefined
    return candidate
  }
  context.warn(`Undefined index operation on ${openScadType(value)}`)
  return undefined
}

export function openScadMember(
  value: Value,
  member: string,
  context: OpenScadValueSemanticsContext,
): Value {
  if (Array.isArray(value)) {
    const index = ({ x: 0, y: 1, z: 2 } as const)[member as 'x' | 'y' | 'z']
    if (index !== undefined) return value[index]
  }
  context.warn(`${openScadType(value)} has no member ${member}`)
  return undefined
}

export function openScadDeepEqual(left: Value, right: Value): boolean {
  if (Array.isArray(left) || Array.isArray(right)) {
    return Array.isArray(left) && Array.isArray(right)
      && left.length === right.length
      && left.every((value, index) => openScadDeepEqual(value, right[index]))
  }
  if (isOpenScadRange(left) || isOpenScadRange(right)) {
    return isOpenScadRange(left) && isOpenScadRange(right)
      && left.start === right.start && left.step === right.step && left.end === right.end
  }
  return left === right
}

export function formatOpenScadValue(value: Value): string {
  if (value === undefined) return 'undef'
  if (typeof value === 'string') return JSON.stringify(value)
  if (typeof value === 'boolean') return value ? 'true' : 'false'
  if (typeof value === 'number') {
    if (Number.isNaN(value)) return 'nan'
    if (value === Infinity) return 'inf'
    if (value === -Infinity) return '-inf'
    if (Object.is(value, -0)) return '0'
    // OpenSCAD's default value printer uses six significant digits.
    return Number(value.toPrecision(6)).toString()
  }
  if (Array.isArray(value)) return `[${value.map(formatOpenScadValue).join(', ')}]`
  if (isOpenScadRange(value)) {
    return `[${formatOpenScadValue(value.start)} : ${formatOpenScadValue(value.step)} : ${formatOpenScadValue(value.end)}]`
  }
  return 'function(...)'
}

export function openScadType(value: Value): string {
  if (value === undefined) return 'undefined'
  if (Array.isArray(value)) return 'vector'
  if (isOpenScadRange(value)) return 'range'
  if (isOpenScadFunction(value)) return 'function'
  return typeof value
}

function register(
  values: Value[],
  label: string,
  context: OpenScadValueSemanticsContext,
): Value[] {
  return context.registerArray?.(values, label) ?? values
}

function numericVector(value: Value): value is number[] {
  return Array.isArray(value) && value.every(item => typeof item === 'number')
}

function numericMatrix(value: Value): value is number[][] {
  return Array.isArray(value) && value.length > 0 && value.every(numericVector)
}

function scaleVector(
  value: Value[],
  scalar: number,
  context: OpenScadValueSemanticsContext,
): Value[] {
  return register(value.map(item => Array.isArray(item)
    ? scaleVector(item, scalar, context)
    : typeof item === 'number'
      ? item * scalar
      : undefinedOperation(TT.Star, item, scalar, context)), 'scaled vector', context)
}

function dot(left: readonly number[], right: readonly number[]): number | undefined {
  if (left.length !== right.length) return undefined
  let result = 0
  for (let index = 0; index < left.length; index++) result += left[index] * right[index]
  return result
}

function multiply(
  left: Value,
  right: Value,
  context: OpenScadValueSemanticsContext,
): Value {
  if (typeof left === 'number' && typeof right === 'number') return left * right
  if (typeof left === 'number' && Array.isArray(right)) return scaleVector(right, left, context)
  if (Array.isArray(left) && typeof right === 'number') return scaleVector(left, right, context)

  if (numericVector(left) && numericVector(right)) {
    const result = dot(left, right)
    if (result !== undefined) return result
    context.warn(`vector*vector requires matching lengths (${left.length} != ${right.length})`)
    return undefined
  }

  if (numericMatrix(left) && numericVector(right)) {
    if (left.some(row => row.length !== right.length)) {
      context.warn('matrix*vector requires matching dimensions')
      return undefined
    }
    return register(left.map(row => dot(row, right)!), 'matrix-vector product', context)
  }

  if (numericVector(left) && numericMatrix(right)) {
    if (right.length !== left.length || right.some(row => row.length !== right[0].length)) {
      context.warn('vector*matrix requires matching rectangular dimensions')
      return undefined
    }
    return register(Array.from({ length: right[0].length }, (_, column) => {
      let value = 0
      for (let row = 0; row < left.length; row++) value += left[row] * right[row][column]
      return value
    }), 'vector-matrix product', context)
  }

  if (numericMatrix(left) && numericMatrix(right)) {
    const shared = left[0].length
    if (left.some(row => row.length !== shared)
      || right.length !== shared
      || right.some(row => row.length !== right[0].length)) {
      context.warn('matrix*matrix requires matching rectangular dimensions')
      return undefined
    }
    return register(left.map(row => register(
      Array.from({ length: right[0].length }, (_, column) => {
        let value = 0
        for (let index = 0; index < shared; index++) value += row[index] * right[index][column]
        return value
      }),
      'matrix product row',
      context,
    )), 'matrix product', context)
  }

  return undefinedOperation(TT.Star, left, right, context)
}

function compareValues(left: Value, right: Value): number | undefined {
  if (typeof left === 'number' && typeof right === 'number') return left - right
  if (typeof left === 'string' && typeof right === 'string') return left < right ? -1 : left > right ? 1 : 0
  if (typeof left === 'boolean' && typeof right === 'boolean') return Number(left) - Number(right)
  if (Array.isArray(left) && Array.isArray(right)) {
    const length = Math.min(left.length, right.length)
    for (let index = 0; index < length; index++) {
      if (openScadDeepEqual(left[index], right[index])) continue
      const compared = compareValues(left[index], right[index])
      return compared === undefined ? undefined : compared
    }
    return left.length - right.length
  }
  return undefined
}

function undefinedOperation(
  operator: TT,
  left: Value,
  right: Value,
  context: OpenScadValueSemanticsContext,
): undefined {
  context.warn(`Undefined operation (${openScadType(left)} ${token(operator)} ${openScadType(right)})`)
  return undefined
}

function token(operator: TT): string {
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
  } as Partial<Record<TT, string>>)[operator] ?? TT[operator]
}
