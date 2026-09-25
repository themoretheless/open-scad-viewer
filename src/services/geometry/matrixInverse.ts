/**
 * Pure-TypeScript port of math_core::viewport::inverse (partial-pivot
 * Gauss-Jordan elimination in f64). Bit-compatible with the native
 * implementation for finite inputs: same pivot policy (last maximum wins,
 * matching Rust's max_by), same operation order. Returns null exactly where
 * the native version returns None (non-finite input, zero pivot, non-finite
 * result) so callers can reproduce the kernel's refuse-on-singular contract
 * without a synchronous WASM round trip.
 */
export function invertMatrixF64(m: ArrayLike<number>): number[] | null {
  for (let i = 0; i < 16; i++) if (!Number.isFinite(m[i])) return null
  const rows: number[][] = []
  for (let r = 0; r < 4; r++) {
    const row = new Array<number>(8).fill(0)
    for (let c = 0; c < 4; c++) row[c] = m[r * 4 + c]
    row[r + 4] = 1
    rows.push(row)
  }
  for (let c = 0; c < 4; c++) {
    let pivot = c
    for (let r = c + 1; r < 4; r++) {
      if (Math.abs(rows[r][c]) >= Math.abs(rows[pivot][c])) pivot = r
    }
    if (rows[pivot][c] === 0) return null
    if (pivot !== c) {
      const swap = rows[c]
      rows[c] = rows[pivot]
      rows[pivot] = swap
    }
    const d = rows[c][c]
    for (let k = 0; k < 8; k++) rows[c][k] /= d
    for (let r = 0; r < 4; r++) {
      if (r === c) continue
      const factor = rows[r][c]
      for (let k = 0; k < 8; k++) rows[r][k] -= factor * rows[c][k]
    }
  }
  const result = new Array<number>(16)
  for (let i = 0; i < 16; i++) result[i] = rows[(i / 4) | 0][(i % 4) + 4]
  for (let i = 0; i < 16; i++) if (!Number.isFinite(result[i])) return null
  return result
}
