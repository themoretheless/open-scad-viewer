/**
 * Pure-TypeScript port of math_core::viewport::inverse (partial-pivot
 * Gauss-Jordan elimination in f64). Bit-compatible with the native
 * implementation for finite inputs: same pivot policy (last maximum wins,
 * matching Rust's max_by), same operation order. Returns null exactly where
 * the native version returns None (non-finite input, zero pivot, non-finite
 * result) so callers can reproduce the kernel's refuse-on-singular contract
 * without a synchronous WASM round trip.
 */
const AUG = new Float64Array(32)
const ROW_BASE = new Int32Array(4)

export function invertMatrixF64(
  m: ArrayLike<number>,
  out?: number[] | Float64Array,
): number[] | Float64Array | null {
  for (let i = 0; i < 16; i++) if (!Number.isFinite(m[i])) return null
  AUG.fill(0)
  for (let r = 0; r < 4; r++) {
    const base = r * 8
    const mBase = r * 4
    AUG[base] = m[mBase]
    AUG[base + 1] = m[mBase + 1]
    AUG[base + 2] = m[mBase + 2]
    AUG[base + 3] = m[mBase + 3]
    AUG[base + 4 + r] = 1
    ROW_BASE[r] = base
  }
  for (let c = 0; c < 4; c++) {
    let pivot = c
    for (let r = c + 1; r < 4; r++) {
      if (Math.abs(AUG[ROW_BASE[r] + c]) >= Math.abs(AUG[ROW_BASE[pivot] + c])) pivot = r
    }
    const pivotBase = ROW_BASE[pivot]
    if (AUG[pivotBase + c] === 0) return null
    if (pivot !== c) {
      ROW_BASE[pivot] = ROW_BASE[c]
      ROW_BASE[c] = pivotBase
    }
    const cBase = ROW_BASE[c]
    const d = AUG[cBase + c]
    for (let k = 0; k < 8; k++) AUG[cBase + k] /= d
    for (let r = 0; r < 4; r++) {
      if (r === c) continue
      const rBase = ROW_BASE[r]
      const factor = AUG[rBase + c]
      for (let k = 0; k < 8; k++) AUG[rBase + k] -= factor * AUG[cBase + k]
    }
  }
  for (let r = 0; r < 4; r++) {
    const rBase = ROW_BASE[r]
    for (let c = 0; c < 4; c++) {
      if (!Number.isFinite(AUG[rBase + 4 + c])) return null
    }
  }
  const result = out ?? new Array<number>(16)
  for (let r = 0; r < 4; r++) {
    const rBase = ROW_BASE[r]
    const outBase = r * 4
    result[outBase] = AUG[rBase + 4]
    result[outBase + 1] = AUG[rBase + 5]
    result[outBase + 2] = AUG[rBase + 6]
    result[outBase + 3] = AUG[rBase + 7]
  }
  return result
}
