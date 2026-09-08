import { ANGLE, LENGTH, SCALAR, dimensionOf, magnitude, sameDimension, type NumericValue } from './modelGraphUnits'

export type ModelGraphNumericType = 'int' | 'f32' | 'f64' | 'length' | 'angle'
export function checkModelGraphNumericType(
  value: NumericValue, type: ModelGraphNumericType, fail: (message: string) => never,
): NumericValue {
  const expected = type === 'length' ? LENGTH : type === 'angle' ? ANGLE : SCALAR
  if (!sameDimension(dimensionOf(value), expected)) return fail(`Expected ${type}; incompatible units`)
  const n = magnitude(value)
  if (type === 'int' && (!Number.isInteger(n) || n < -2147483648 || n > 2147483647)) return fail('Expected int (signed 32-bit integer)')
  if (type === 'f32') {
    const rounded = Math.fround(n)
    if (!Number.isFinite(rounded)) return fail('Value is outside f32 range')
    return rounded
  }
  return value
}
