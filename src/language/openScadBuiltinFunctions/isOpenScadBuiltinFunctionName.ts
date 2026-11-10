import { BUILTIN_NAMES, HANDLERS, MAX_CHR_RANGE_ITEMS, NOT_RECOGNIZED } from '../openScadBuiltinFunctions'
import type { BuiltinHandler, OpenScadBuiltinFunctionContext, OpenScadBuiltinFunctionName, OpenScadBuiltinFunctionResult, OpenScadBuiltinValue } from '../openScadBuiltinFunctions'
import { equalValues, expectCount, formatValue, isRangeValue, rangeItemCount, registerArray, warn } from './createDeferredOpenScadBuiltinArguments'

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
      warn(context, 'chr', `range exceeds the ${MAX_CHR_RANGE_ITEMS.toLocaleString('en-US')}-item character limit`)
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

export function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false
  }
  return true
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
