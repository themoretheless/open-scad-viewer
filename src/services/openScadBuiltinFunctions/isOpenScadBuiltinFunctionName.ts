import { BUILTIN_NAMES, HANDLERS, MAX_CHR_RANGE_ITEMS, Mt19937, NOT_RECOGNIZED } from '../openScadBuiltinFunctions'
import type { BuiltinHandler, OpenScadBuiltinFunctionContext, OpenScadBuiltinFunctionName, OpenScadBuiltinFunctionResult, OpenScadBuiltinValue } from '../openScadBuiltinFunctions'
import { compareValues, equalValues, expectCount, fail, formatValue, isRangeValue, numberValue, rangeItemCount, registerArray, warn } from './createDeferredOpenScadBuiltinArguments'

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

export function characterString(
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

export function returnMatches(
  context: OpenScadBuiltinFunctionContext,
  matches: readonly number[],
  maximum: number,
): OpenScadBuiltinValue {
  const selected = maximum === 0 ? matches : matches.slice(0, maximum)
  return registerArray(context, 'search', selected)
}

export function searchNeedles(
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

export function unsignedSearchParameter(value: OpenScadBuiltinValue): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) return 0
  return Math.trunc(value) >>> 0
}

export function searchStringRows(
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

export function predicate(name: string, test: (value: OpenScadBuiltinValue) => boolean): BuiltinHandler {
  return (args, context) => {
    expectCount(context, name, args, [1])
    return test(args[0])
  }
}

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
