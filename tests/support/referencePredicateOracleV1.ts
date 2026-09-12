/**
 * Independent G0.7 / G2a predicate oracle.
 * BigRational exact-sign arithmetic only. Imports no production predicate module.
 */

export type Sign = 'Negative' | 'Zero' | 'Positive' | 'Indeterminate'
export type ModelClass = 'Coincident' | 'Separate' | 'Indeterminate'

export type Rational = { readonly n: bigint; readonly d: bigint }

export type ToleranceContext = {
  readonly linear_abs: number
  readonly linear_rel: number
  readonly on_tol: number
  readonly clear_tol: number
  readonly angular: number
  readonly param_floor: number
  readonly ulp_guard: number
  readonly max_entity_error: number
  readonly policy: string
}

function gcd(a: bigint, b: bigint): bigint {
  let x = a < 0n ? -a : a
  let y = b < 0n ? -b : b
  while (y !== 0n) {
    const t = x % y
    x = y
    y = t
  }
  return x === 0n ? 1n : x
}

export function rat(n: bigint | number, d: bigint | number = 1n): Rational {
  let nn = typeof n === 'number' ? BigInt(n) : n
  let dd = typeof d === 'number' ? BigInt(d) : d
  if (dd === 0n) throw new Error('zero denominator')
  if (dd < 0n) {
    nn = -nn
    dd = -dd
  }
  const g = gcd(nn, dd)
  return { n: nn / g, d: dd / g }
}

/** Exact binary64 bit pattern as a rational via mantissa/exponent (finite only). */
export function ratFromBinary64(value: number): Rational | null {
  if (!Number.isFinite(value)) return null
  if (Object.is(value, -0) || value === 0) return rat(0n)
  const buf = new ArrayBuffer(8)
  new DataView(buf).setFloat64(0, value, false)
  const hi = BigInt(new DataView(buf).getUint32(0, false))
  const lo = BigInt(new DataView(buf).getUint32(4, false))
  const bits = (hi << 32n) | lo
  const sign = bits >> 63n === 0n ? 1n : -1n
  const expBits = (bits >> 52n) & 0x7ffn
  const frac = bits & ((1n << 52n) - 1n)
  if (expBits === 0x7ffn) return null
  if (expBits === 0n) {
    // subnormal: (-1)^s * frac * 2^(1-1023-52)
    return rat(sign * frac, 1n << 1074n)
  }
  const mant = (1n << 52n) | frac
  const exp = expBits - 1023n - 52n
  if (exp >= 0n) return rat(sign * mant * (1n << exp))
  return rat(sign * mant, 1n << -exp)
}

function mul(a: Rational, b: Rational): Rational {
  return rat(a.n * b.n, a.d * b.d)
}

function sub(a: Rational, b: Rational): Rational {
  return rat(a.n * b.d - b.n * a.d, a.d * b.d)
}

function add(a: Rational, b: Rational): Rational {
  return rat(a.n * b.d + b.n * a.d, a.d * b.d)
}

function signOf(a: Rational): Sign {
  if (a.n === 0n) return 'Zero'
  return a.n > 0n ? 'Positive' : 'Negative'
}

function asLeaf(p: readonly [Rational, Rational] | readonly [number, number]): readonly [Rational, Rational] | null {
  if (typeof p[0] === 'number') {
    const x = ratFromBinary64(p[0])
    const y = ratFromBinary64(p[1] as number)
    if (!x || !y) return null
    return [x, y]
  }
  return p as readonly [Rational, Rational]
}

function asLeaf3(
  p: readonly [Rational, Rational, Rational] | readonly [number, number, number],
): readonly [Rational, Rational, Rational] | null {
  if (typeof p[0] === 'number') {
    const x = ratFromBinary64(p[0])
    const y = ratFromBinary64(p[1] as number)
    const z = ratFromBinary64(p[2] as number)
    if (!x || !y || !z) return null
    return [x, y, z]
  }
  return p as readonly [Rational, Rational, Rational]
}

/** sign((b-a)×(c-a)) in 2D. */
export function orient2d(
  a: readonly [Rational, Rational] | readonly [number, number],
  b: readonly [Rational, Rational] | readonly [number, number],
  c: readonly [Rational, Rational] | readonly [number, number],
): Sign {
  const A = asLeaf(a)
  const B = asLeaf(b)
  const C = asLeaf(c)
  if (!A || !B || !C) return 'Indeterminate'
  const abx = sub(B[0], A[0])
  const aby = sub(B[1], A[1])
  const acx = sub(C[0], A[0])
  const acy = sub(C[1], A[1])
  return signOf(sub(mul(abx, acy), mul(aby, acx)))
}

/** sign(scalar_triple(b-a, c-a, d-a)). */
export function orient3d(
  a: readonly [Rational, Rational, Rational] | readonly [number, number, number],
  b: readonly [Rational, Rational, Rational] | readonly [number, number, number],
  c: readonly [Rational, Rational, Rational] | readonly [number, number, number],
  d: readonly [Rational, Rational, Rational] | readonly [number, number, number],
): Sign {
  const A = asLeaf3(a)
  const B = asLeaf3(b)
  const C = asLeaf3(c)
  const D = asLeaf3(d)
  if (!A || !B || !C || !D) return 'Indeterminate'
  const ab = [sub(B[0], A[0]), sub(B[1], A[1]), sub(B[2], A[2])] as const
  const ac = [sub(C[0], A[0]), sub(C[1], A[1]), sub(C[2], A[2])] as const
  const ad = [sub(D[0], A[0]), sub(D[1], A[1]), sub(D[2], A[2])] as const
  const cx = sub(mul(ac[1], ad[2]), mul(ac[2], ad[1]))
  const cy = sub(mul(ac[2], ad[0]), mul(ac[0], ad[2]))
  const cz = sub(mul(ac[0], ad[1]), mul(ac[1], ad[0]))
  return signOf(add(add(mul(ab[0], cx), mul(ab[1], cy)), mul(ab[2], cz)))
}

/** sign(||p-q||² − r²) with exact leaves only. */
export function compareSquaredDistance(
  p: readonly [Rational, Rational, Rational] | readonly [number, number, number],
  q: readonly [Rational, Rational, Rational] | readonly [number, number, number],
  rSquared: Rational | number,
): Sign {
  const P = asLeaf3(p)
  const Q = asLeaf3(q)
  const R = typeof rSquared === 'number' ? ratFromBinary64(rSquared) : rSquared
  if (!P || !Q || !R) return 'Indeterminate'
  const dx = sub(P[0], Q[0])
  const dy = sub(P[1], Q[1])
  const dz = sub(P[2], Q[2])
  const dist2 = add(add(mul(dx, dx), mul(dy, dy)), mul(dz, dz))
  return signOf(sub(dist2, R))
}

export function validateToleranceContext(ctx: ToleranceContext): string | null {
  if (!(ctx.on_tol > 0) || !(ctx.clear_tol > ctx.on_tol)) return 'on_tol_clear_tol'
  if (!(ctx.max_entity_error >= 0) || !Number.isFinite(ctx.max_entity_error)) {
    return 'max_entity_error'
  }
  for (const key of [
    'linear_abs',
    'linear_rel',
    'angular',
    'param_floor',
    'ulp_guard',
  ] as const) {
    if (!(ctx[key] >= 0) || !Number.isFinite(ctx[key])) return key
  }
  return null
}

/**
 * Model classifier with proven residual/enclosure bounds.
 * Gray band or missing proof → Indeterminate. Never coerces to false.
 */
export function classifyModel(
  residual: number,
  provenBound: number | null,
  ctx: ToleranceContext,
): ModelClass {
  if (validateToleranceContext(ctx)) return 'Indeterminate'
  if (provenBound === null || !Number.isFinite(residual) || !Number.isFinite(provenBound)) {
    return 'Indeterminate'
  }
  const hi = residual + provenBound
  const lo = residual - provenBound
  if (hi < ctx.on_tol) return 'Coincident'
  if (lo > ctx.clear_tol) return 'Separate'
  return 'Indeterminate'
}

/** Kill: Indeterminate must not become boolean success/false. */
export function coerceIndeterminateForbidden(value: Sign | ModelClass): never | void {
  if (value === 'Indeterminate') {
    throw new Error('Indeterminate must not coerce to false or WithinTolerance success')
  }
}
