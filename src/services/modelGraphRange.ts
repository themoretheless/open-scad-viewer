import { createUnitArithmetic, magnitude, type NumericValue, type UnitFailure } from './modelGraphUnits'

/** Resolve bounded endpoints without repeated addition or implicit physical units. */
export function resolveInterval(
  start: NumericValue, end: NumericValue, inclusive: boolean,
  options: { count?: NumericValue; step?: NumericValue },
  fail: UnitFailure, path: string,
): NumericValue[] {
  const math = createUnitArithmetic(fail)
  math.equal(start, end, path)
  if (options.count !== undefined && options.step !== undefined) fail('invalid_range', path, 'Use either by or count, never both.')
  const a = magnitude(start), b = magnitude(end)
  let count: number, step: NumericValue
  if (options.count !== undefined) {
    count = math.scalar(options.count, path)
    if (!Number.isInteger(count) || count < 0 || count > 256) fail('invalid_count', path, 'Range count must be an integer from 0 to 256.')
    if (!inclusive && a === b && count > 0) fail('invalid_range', path, 'An empty exclusive interval cannot contain samples.')
    step = count <= 1 ? math.binary('subtract', start, start, path)
      : math.binary('divide', math.binary('subtract', end, start, path), inclusive ? count - 1 : count, path)
  } else {
    if (options.step === undefined && (typeof start !== 'number' || typeof end !== 'number' || !Number.isInteger(a) || !Number.isInteger(b))) fail('invalid_range', path, 'Only integer dimensionless ranges have an implicit step; specify by or count.')
    step = options.step ?? 1
    math.equal(start, step, path)
    const s = magnitude(step)
    if (s === 0) fail('invalid_range', path, 'Range step cannot be zero.')
    const distance = (b - a) / s
    // Snap near-integer quotients to neutralize endpoint roundoff (e.g. 0.3 / 0.1).
    const rounded = Math.round(distance)
    const q = Math.abs(distance - rounded) <= 8 * Number.EPSILON * Math.max(1, Math.abs(distance)) ? rounded : distance
    count = q < 0 ? 0 : inclusive ? Math.floor(q) + 1 : Math.ceil(q)
    if (!Number.isFinite(count) || count > 256) fail('invalid_count', path, `Range creates ${count} values; maximum 256. Increase by or reduce count.`)
  }
  return Array.from({ length: count }, (_, i) =>
    options.count !== undefined && inclusive && count > 1 && i === count - 1 ? end
      : math.binary('add', start, math.binary('multiply', i, step, path), path))
}
