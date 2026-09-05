/** Dimensional arithmetic in canonical millimeters and degrees. */
export type Unit = 'mm' | 'cm' | 'm' | 'in' | 'deg' | 'rad'
export type Dimension = readonly [length: number, angle: number]
export type Quantity = { kind: 'quantity'; value: number; dimension: Dimension }
export type NumericValue = number | Quantity
export const LENGTH: Dimension = [1, 0]
export const ANGLE: Dimension = [0, 1]
export const SCALAR: Dimension = [0, 0]
export type UnitFailure = (code: string, path: string, message: string) => never
export const dimensionOf = (value: NumericValue): Dimension => typeof value === 'number' ? SCALAR : value.dimension
export const magnitude = (value: NumericValue): number => typeof value === 'number' ? value : value.value
export const sameDimension = (a: Dimension, b: Dimension) => a[0] === b[0] && a[1] === b[1]
export const dimensionLabel = (dimension: Dimension) => `length^${dimension[0]} angle^${dimension[1]}`
export function createUnitArithmetic(fail: UnitFailure) {
  const pack = (value: number, dimension: Dimension, path: string): NumericValue => {
    if (!Number.isFinite(value) || Math.abs(value) > 1_000_000) return fail('invalid_number', path, 'Expression must produce a finite number within +/-1000000 in canonical units.')
    if (dimension.some(n => !Number.isInteger(n) || Math.abs(n) > 8)) return fail('dimension_limit', path, 'Dimension exponents must be integers within +/-8.')
    return sameDimension(dimension, SCALAR) ? value : { kind: 'quantity', value, dimension }
  }
  const equal = (a: NumericValue, b: NumericValue, path: string) => {
    if (!sameDimension(dimensionOf(a), dimensionOf(b))) fail('unit_mismatch', path, `Incompatible dimensions: ${dimensionLabel(dimensionOf(a))} and ${dimensionLabel(dimensionOf(b))}.`)
  }
  const scalar = (value: NumericValue, path: string): number => {
    if (!sameDimension(dimensionOf(value), SCALAR)) return fail('unit_mismatch', path, 'Expected a dimensionless number.')
    return magnitude(value)
  }
  const quantity = (value: number, unit: Unit, path: string) => {
    const scales = { mm: 1, cm: 10, m: 1000, in: 25.4, deg: 1, rad: 180 / Math.PI }
    return pack(value * scales[unit], unit === 'deg' || unit === 'rad' ? ANGLE : LENGTH, path)
  }
  const binary = (op: string, left: NumericValue, right: NumericValue, path: string): NumericValue => {
    const a = magnitude(left), b = magnitude(right), ad = dimensionOf(left), bd = dimensionOf(right)
    switch (op) {
      case 'multiply': return pack(a * b, [ad[0] + bd[0], ad[1] + bd[1]], path)
      case 'divide': return pack(a / b, [ad[0] - bd[0], ad[1] - bd[1]], path)
      case 'pow': scalar(right, path); return pack(a ** b, [ad[0] * b, ad[1] * b], path)
      case 'and': return +(scalar(left, path) !== 0 && scalar(right, path) !== 0)
      case 'or': return +(scalar(left, path) !== 0 || scalar(right, path) !== 0)
    }
    equal(left, right, path)
    switch (op) {
      case 'add': return pack(a + b, ad, path)
      case 'subtract': return pack(a - b, ad, path)
      case 'mod': return pack(a % b, ad, path)
      case 'min': return pack(Math.min(a, b), ad, path)
      case 'max': return pack(Math.max(a, b), ad, path)
      case 'lt': return +(a < b)
      case 'le': return +(a <= b)
      case 'eq': return +(a === b)
      default: return fail('type_error', path, 'Unknown numeric operation.')
    }
  }
  const unary = (op: string, value: NumericValue, path: string, strict: boolean): NumericValue => {
    const a = magnitude(value), dimension = dimensionOf(value)
    switch (op) {
      case 'not': return +(scalar(value, path) === 0)
      case 'negate': return pack(-a, dimension, path)
      case 'abs': return pack(Math.abs(a), dimension, path)
      case 'floor': return pack(Math.floor(a), dimension, path)
      case 'ceil': return pack(Math.ceil(a), dimension, path)
      case 'sqrt': return pack(Math.sqrt(a), [dimension[0] / 2, dimension[1] / 2], path)
      case 'sin': case 'cos':
        if (!sameDimension(dimension, ANGLE) && (strict || !sameDimension(dimension, SCALAR))) return fail('unit_mismatch', path, 'Trigonometry expects an angle.')
        return pack(Math[op](a * Math.PI / 180), SCALAR, path)
      default: return fail('type_error', path, 'Unknown numeric operation.')
    }
  }
  const field = (value: NumericValue, expected: Dimension, path: string, strict: boolean): number => {
    if (sameDimension(dimensionOf(value), expected)) return magnitude(value)
    // Legacy bare geometry numbers use mm/degrees. Strict mode only permits a bare zero.
    if (typeof value === 'number' && (!strict || value === 0)) return value
    return fail('unit_mismatch', path, `Expected ${dimensionLabel(expected)}, received ${dimensionLabel(dimensionOf(value))}.`)
  }
  return { quantity, binary, unary, field, equal, scalar }
}
