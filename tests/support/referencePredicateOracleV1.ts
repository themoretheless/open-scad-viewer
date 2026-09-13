/**
 * Independent G0.7 predicate oracle.
 * BigRational-style exact signs + outward interval filter.
 * Imports no production geometry/predicate modules.
 */

export type Sign = -1 | 0 | 1

/** Exact rational as reduced bigint numerator/denominator (denominator > 0). */
export type BigRational = {
  readonly n: bigint
  readonly d: bigint
}

function absBig(n: bigint): bigint {
  return n < 0n ? -n : n
}

function gcd(a: bigint, b: bigint): bigint {
  let x = absBig(a)
  let y = absBig(b)
  while (y !== 0n) {
    const t = y
    y = x % y
    x = t
  }
  return x
}

export function rational(n: bigint | number, d: bigint | number = 1n): BigRational {
  let nn = typeof n === 'number' ? BigInt(n) : n
  let dd = typeof d === 'number' ? BigInt(d) : d
  if (dd === 0n) throw new Error('denominator must be non-zero')
  if (dd < 0n) {
    nn = -nn
    dd = -dd
  }
  const g = gcd(nn, dd)
  return { n: nn / g, d: dd / g }
}

/** Exact binary64 bit pattern → rational (finite only). */
export function rationalFromBinary64(bits: bigint): BigRational {
  const u = bits & 0xffffffffffffffffn
  const sign = u >> 63n === 0n ? 1n : -1n
  const exp = Number((u >> 52n) & 0x7ffn)
  const frac = u & 0xfffffffffffffn
  if (exp === 0x7ff) throw new Error('NaN/Inf are not ExactInputLeaf values')
  if (exp === 0) {
    // subnormal: (-1)^s * frac * 2^(1-1023-52)
    if (frac === 0n) return rational(0n)
    return rational(sign * frac, 2n ** 1074n)
  }
  // normal: (-1)^s * (1+frac/2^52) * 2^(exp-1023)
  const mantissa = (1n << 52n) + frac
  const power = exp - 1023 - 52
  if (power >= 0) return rational(sign * mantissa * 2n ** BigInt(power))
  return rational(sign * mantissa, 2n ** BigInt(-power))
}

export function rationalFromNumber(value: number): BigRational {
  if (!Number.isFinite(value)) throw new Error('non-finite number')
  const buf = new ArrayBuffer(8)
  new Float64Array(buf)[0] = value
  return rationalFromBinary64(new DataView(buf).getBigUint64(0, true))
}

function mul(a: BigRational, b: BigRational): BigRational {
  return rational(a.n * b.n, a.d * b.d)
}

function sub(a: BigRational, b: BigRational): BigRational {
  return rational(a.n * b.d - b.n * a.d, a.d * b.d)
}

function add(a: BigRational, b: BigRational): BigRational {
  return rational(a.n * b.d + b.n * a.d, a.d * b.d)
}

export function cmpRational(a: BigRational, b: BigRational): Sign {
  const v = a.n * b.d - b.n * a.d
  return v === 0n ? 0 : v > 0n ? 1 : -1
}

export function signRational(a: BigRational): Sign {
  return a.n === 0n ? 0 : a.n > 0n ? 1 : -1
}

/** Exact orient2d = sign((bx-ax)*(cy-ay) - (by-ay)*(cx-ax)). */
export function orient2dExact(
  ax: BigRational,
  ay: BigRational,
  bx: BigRational,
  by: BigRational,
  cx: BigRational,
  cy: BigRational,
): Sign {
  const left = mul(sub(bx, ax), sub(cy, ay))
  const right = mul(sub(by, ay), sub(cx, ax))
  return signRational(sub(left, right))
}

/** Exact orient3d scalar triple (b-a, c-a, d-a). */
export function orient3dExact(
  a: readonly [BigRational, BigRational, BigRational],
  b: readonly [BigRational, BigRational, BigRational],
  c: readonly [BigRational, BigRational, BigRational],
  d: readonly [BigRational, BigRational, BigRational],
): Sign {
  const ab = [sub(b[0], a[0]), sub(b[1], a[1]), sub(b[2], a[2])] as const
  const ac = [sub(c[0], a[0]), sub(c[1], a[1]), sub(c[2], a[2])] as const
  const ad = [sub(d[0], a[0]), sub(d[1], a[1]), sub(d[2], a[2])] as const
  const cx0 = sub(mul(ac[1], ad[2]), mul(ac[2], ad[1]))
  const cx1 = sub(mul(ac[2], ad[0]), mul(ac[0], ad[2]))
  const cx2 = sub(mul(ac[0], ad[1]), mul(ac[1], ad[0]))
  const det = add(add(mul(ab[0], cx0), mul(ab[1], cx1)), mul(ab[2], cx2))
  return signRational(det)
}

function checkDistanceDimension(p: readonly unknown[], q: readonly unknown[]): void {
  if ((p.length !== 2 && p.length !== 3) || p.length !== q.length) {
    throw new Error('distance comparison requires matching 2D or 3D points')
  }
}

/** Exact sign of ||p-q||²-r²; the radius is an exact nonnegative bound. */
export function compareSquaredDistanceExact(
  p: readonly BigRational[],
  q: readonly BigRational[],
  radius: BigRational,
): Sign {
  checkDistanceDimension(p, q)
  if (radius.n < 0n) throw new Error('radius must be nonnegative')
  let squaredDistance = rational(0)
  for (let i = 0; i < p.length; i++) {
    const difference = sub(p[i], q[i])
    squaredDistance = add(squaredDistance, mul(difference, difference))
  }
  return signRational(sub(squaredDistance, mul(radius, radius)))
}

/** Outward interval enclosure, including unbounded intermediate results. */
export type Interval = { readonly lo: number; readonly hi: number }

const floatBits = new DataView(new ArrayBuffer(8))

function nextUp(value: number): number {
  if (Number.isNaN(value)) throw new Error('NaN has no interval endpoint')
  if (value === Infinity) return Infinity
  if (value === -Infinity) return -Number.MAX_VALUE
  if (value === 0) return Number.MIN_VALUE
  floatBits.setFloat64(0, value, false)
  const bits = floatBits.getBigUint64(0, false)
  floatBits.setBigUint64(0, value > 0 ? bits + 1n : bits - 1n, false)
  return floatBits.getFloat64(0, false)
}

function nextDown(value: number): number {
  return -nextUp(-value)
}

export function outwardInterval(value: number): Interval {
  if (!Number.isFinite(value)) throw new Error('non-finite')
  return { lo: nextDown(value), hi: nextUp(value) }
}

function encloseRounded(lo: number, hi: number): Interval {
  // Extended endpoints such as 0*Infinity or Infinity-Infinity do not
  // determine a finite bound. Widen instead of allowing NaN comparisons to
  // certify a sign. Finite overflow and underflow each get outward endpoints.
  if (Number.isNaN(lo) || Number.isNaN(hi)) return { lo: -Infinity, hi: Infinity }
  return { lo: nextDown(lo), hi: nextUp(hi) }
}

export function addInterval(a: Interval, b: Interval): Interval {
  return encloseRounded(a.lo + b.lo, a.hi + b.hi)
}

export function subInterval(a: Interval, b: Interval): Interval {
  return encloseRounded(a.lo - b.hi, a.hi - b.lo)
}

export function mulInterval(a: Interval, b: Interval): Interval {
  const products = [a.lo * b.lo, a.lo * b.hi, a.hi * b.lo, a.hi * b.hi]
  return encloseRounded(Math.min(...products), Math.max(...products))
}

export type FilterSign = Sign | 'indeterminate'

function intervalSign(value: Interval): FilterSign {
  if (value.hi < 0) return -1
  if (value.lo > 0) return 1
  // An overlapping enclosure cannot prove exact zero. Only the exact
  // rational stage reports zero, including underflowed determinants.
  return 'indeterminate'
}

export function orient2dFilter(
  ax: number,
  ay: number,
  bx: number,
  by: number,
  cx: number,
  cy: number,
): FilterSign {
  const Ax = outwardInterval(ax)
  const Ay = outwardInterval(ay)
  const Bx = outwardInterval(bx)
  const By = outwardInterval(by)
  const Cx = outwardInterval(cx)
  const Cy = outwardInterval(cy)
  const left = mulInterval(subInterval(Bx, Ax), subInterval(Cy, Ay))
  const right = mulInterval(subInterval(By, Ay), subInterval(Cx, Ax))
  const det = subInterval(left, right)
  return intervalSign(det)
}

export function orient3dFilter(
  a: readonly [number, number, number],
  b: readonly [number, number, number],
  c: readonly [number, number, number],
  d: readonly [number, number, number],
): FilterSign {
  const A = a.map(outwardInterval)
  const ab = b.map((value, i) => subInterval(outwardInterval(value), A[i]))
  const ac = c.map((value, i) => subInterval(outwardInterval(value), A[i]))
  const ad = d.map((value, i) => subInterval(outwardInterval(value), A[i]))
  const cross = [
    subInterval(mulInterval(ac[1], ad[2]), mulInterval(ac[2], ad[1])),
    subInterval(mulInterval(ac[2], ad[0]), mulInterval(ac[0], ad[2])),
    subInterval(mulInterval(ac[0], ad[1]), mulInterval(ac[1], ad[0])),
  ]
  return intervalSign(addInterval(
    addInterval(mulInterval(ab[0], cross[0]), mulInterval(ab[1], cross[1])),
    mulInterval(ab[2], cross[2]),
  ))
}

export function compareSquaredDistanceFilter(
  p: readonly number[],
  q: readonly number[],
  radius: number,
): FilterSign {
  checkDistanceDimension(p, q)
  if (radius < 0) throw new Error('radius must be nonnegative')
  const bound = outwardInterval(radius)
  let squaredDistance: Interval = { lo: 0, hi: 0 }
  for (let i = 0; i < p.length; i++) {
    const difference = subInterval(outwardInterval(p[i]), outwardInterval(q[i]))
    squaredDistance = addInterval(squaredDistance, mulInterval(difference, difference))
  }
  return intervalSign(subInterval(squaredDistance, mulInterval(bound, bound)))
}

export type ModelClass = 'Coincident' | 'Separate' | 'Indeterminate'

export function classifyResidual(
  residual: number,
  onTol: number,
  clearTol: number,
): ModelClass {
  if (!Number.isFinite(onTol) || !Number.isFinite(clearTol) || !(onTol > 0) || !(clearTol > onTol)) {
    throw new Error('invalid tolerance profile')
  }
  if (!Number.isFinite(residual) || residual < 0) return 'Indeterminate'
  if (residual < onTol) return 'Coincident'
  if (residual > clearTol) return 'Separate'
  return 'Indeterminate'
}
