export type SketchPoint = { id: string; position: [number, number] }
export type SketchConstraint =
  | { id: string; kind: 'fix'; point: string; at: [number, number] }
  | { id: string; kind: 'horizontal' | 'vertical' | 'coincident'; a: string; b: string }
  | { id: string; kind: 'distance'; a: string; b: string; value: number }
  | { id: string; kind: 'parallel' | 'perpendicular' | 'equal_length'; a: string; b: string; c: string; d: string }
const maximum = (values: number[]) => values.reduce((m, v) => Math.max(m, Math.abs(v)), 0)
function rank(matrix: number[][]) {
  if (!matrix.length) return 0
  const rows = matrix.map(row => [...row]); let pivot = 0
  for (let column = 0; column < rows[0]!.length && pivot < rows.length; column++) {
    let best = pivot
    for (let i = pivot + 1; i < rows.length; i++) if (Math.abs(rows[i]![column]!) > Math.abs(rows[best]![column]!)) best = i
    if (Math.abs(rows[best]![column]!) < 1e-7) continue
    ;[rows[pivot], rows[best]] = [rows[best]!, rows[pivot]!]
    const scale = rows[pivot]![column]!
    for (let j = column; j < rows[pivot]!.length; j++) rows[pivot]![j]! /= scale
    for (let i = pivot + 1; i < rows.length; i++) {
      const factor = rows[i]![column]!
      for (let j = column; j < rows[i]!.length; j++) rows[i]![j]! -= factor * rows[pivot]![j]!
    }
    pivot++
  }
  return pivot
}
function linearSolve(matrix: number[][], rhs: number[]) {
  const a = matrix.map((row, i) => [...row, rhs[i]!]), n = rhs.length
  for (let c = 0; c < n; c++) {
    let best = c
    for (let i = c + 1; i < n; i++) if (Math.abs(a[i]![c]!) > Math.abs(a[best]![c]!)) best = i
    if (Math.abs(a[best]![c]!) < 1e-16) return null
    ;[a[c], a[best]] = [a[best]!, a[c]!]
    for (let i = c + 1; i < n; i++) {
      const factor = a[i]![c]! / a[c]![c]!
      for (let j = c; j <= n; j++) a[i]![j]! -= factor * a[c]![j]!
    }
  }
  const result = Array<number>(n).fill(0)
  for (let i = n - 1; i >= 0; i--) {
    let sum = a[i]![n]!
    for (let j = i + 1; j < n; j++) sum -= a[i]![j]! * result[j]!
    result[i] = sum / a[i]![i]!
  }
  return result
}
/** Deterministic bounded damped least-squares solve. Local solution, not a global existence proof. */
export function solveModelGraphSketch(points: SketchPoint[], constraints: SketchConstraint[], tolerance = 1e-6) {
  if (points.length < 3 || points.length > 16 || constraints.length > 48 || !(tolerance >= 1e-8 && tolerance <= 0.1)) throw new Error('Sketch limits exceeded.')
  const index = new Map(points.map((p, i) => [p.id, i * 2]))
  if (index.size !== points.length || new Set(constraints.map(c => c.id)).size !== constraints.length) throw new Error('Sketch point and constraint IDs must be unique.')
  for (const constraint of constraints) {
    const refs = constraint.kind === 'fix' ? [constraint.point] : 'c' in constraint ? [constraint.a, constraint.b, constraint.c, constraint.d] : [constraint.a, constraint.b]
    for (const ref of refs) if (!index.has(ref)) throw new Error(`Unknown sketch point ${ref}.`)
    if (constraint.kind === 'fix' && constraint.at.some(v => !Number.isFinite(v) || Math.abs(v) > 1e6)) throw new Error('Invalid fixed target.')
    if (constraint.kind === 'distance' && (!(constraint.value > 0) || !Number.isFinite(constraint.value))) throw new Error('Distance must be positive; use coincident for zero distance.')
  }
  let x = points.flatMap(p => p.position)
  if (x.some(v => !Number.isFinite(v) || Math.abs(v) > 1e6)) throw new Error('Invalid sketch coordinates.')
  const evaluate = (values: number[]) => {
    const xy = (id: string) => { const i = index.get(id)!; return [values[i]!, values[i + 1]!] as const }
    return constraints.map(constraint => {
      if (constraint.kind === 'fix') { const p = xy(constraint.point); return [p[0] - constraint.at[0], p[1] - constraint.at[1]] }
      const a = xy(constraint.a), b = xy(constraint.b), u = [b[0] - a[0], b[1] - a[1]], lu = Math.hypot(...u)
      switch (constraint.kind) {
        case 'horizontal': return [u[1]!]
        case 'vertical': return [u[0]!]
        case 'coincident': return u
        case 'distance': return [lu - constraint.value]
      }
      const c = xy(constraint.c), d = xy(constraint.d), v = [d[0] - c[0], d[1] - c[1]], lv = Math.hypot(...v)
      if (constraint.kind === 'equal_length') return [lu - lv]
      const divisor = Math.max(lu, lv, 1e-12)
      return [constraint.kind === 'parallel' ? (u[0]! * v[1]! - u[1]! * v[0]!) / divisor : (u[0]! * v[0]! + u[1]! * v[1]!) / divisor]
    })
  }
  const jacobian = (values: number[], residual: number[]) => {
    const rows = residual.map(() => Array<number>(values.length).fill(0))
    for (let j = 0; j < values.length; j++) {
      const h = Math.max(1e-6, Math.abs(values[j]!) * 1e-8), plus = [...values], minus = [...values]
      plus[j]! += h; minus[j]! -= h
      const hi = evaluate(plus).flat(), lo = evaluate(minus).flat()
      for (let i = 0; i < rows.length; i++) rows[i]![j] = (hi[i]! - lo[i]!) / (2 * h)
    }
    return rows
  }
  let damping = 1e-3, iterations = 0
  for (; iterations < 64; iterations++) {
    const residual = evaluate(x).flat()
    if (maximum(residual) <= tolerance) break
    const j = jacobian(x, residual), n = x.length
    const lhs = Array.from({ length: n }, () => Array<number>(n).fill(0)), rhs = Array<number>(n).fill(0)
    for (let a = 0; a < n; a++) {
      for (let k = 0; k < j.length; k++) {
        rhs[a]! -= j[k]![a]! * residual[k]!
        for (let b = 0; b < n; b++) lhs[a]![b]! += j[k]![a]! * j[k]![b]!
      }
      lhs[a]![a]! += damping
    }
    const step = linearSolve(lhs, rhs)
    if (!step) { damping *= 10; continue }
    const candidate = x.map((v, i) => v + step[i]!)
    const error = (v: number[]) => v.reduce((sum, r) => sum + r * r, 0)
    if (candidate.every(v => Number.isFinite(v) && Math.abs(v) <= 1e6) && error(evaluate(candidate).flat()) < error(residual)) { x = candidate; damping = Math.max(1e-12, damping / 3) }
    else damping = Math.min(1e12, damping * 10)
  }
  const grouped = evaluate(x), residual = grouped.flat(), j = jacobian(x, residual), jacobianRank = rank(j)
  const degenerate = constraints.filter(c => 'c' in c && c.kind !== 'equal_length').filter(c => {
    if (!('c' in c)) return false
    const length = (a: string, b: string) => Math.hypot(x[index.get(a)!]! - x[index.get(b)!]!, x[index.get(a)! + 1]! - x[index.get(b)! + 1]!)
    return length(c.a, c.b) <= tolerance || length(c.c, c.d) <= tolerance
  })
  const converged = maximum(residual) <= tolerance && degenerate.length === 0
  const linear = constraints.every(c => ['fix', 'horizontal', 'vertical', 'coincident'].includes(c.kind))
  const status = converged ? (x.length - jacobianRank > 0 ? 'underconstrained' : 'solved') : linear && rank(j.map((row, i) => [...row, residual[i]!])) > jacobianRank ? 'inconsistent' : 'not_converged'
  return { status, iterations, tolerance_mm: tolerance, degrees_of_freedom: x.length - jacobianRank, redundant_equations: Math.max(0, residual.length - jacobianRank), maximum_residual_mm: maximum(residual), degenerate_constraints: degenerate.map(c => c.id), points: points.map((p, i) => ({ id: p.id, position: [x[i * 2]!, x[i * 2 + 1]!] as [number, number] })), constraints: constraints.map((c, i) => ({ id: c.id, residual_mm: maximum(grouped[i]!), satisfied: maximum(grouped[i]!) <= tolerance && !degenerate.some(d => d.id === c.id) })) }
}
