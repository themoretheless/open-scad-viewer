/** The next representable binary64 value above `value` (NaN and +Infinity map to themselves). */
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
