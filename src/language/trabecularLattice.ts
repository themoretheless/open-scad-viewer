export type Pt = [number, number]

export interface BBox {
  minX: number
  minY: number
  maxX: number
  maxY: number
  width: number
  height: number
}

export interface CoupleWallsSpec {
  perimeter: number
  sShift: number
  spanX: number
  sideSpan: number
  cornerArc: number
  innerR: number
  cx: number
  cyBack: number
  cyFront: number
  wallOverhang: number
  wallCell: number
  wallRib: number
  wallJitter: number
  wallSeed: number
  wallHeight?: number | undefined
}

export interface TrabecularCellsOptions {
  min: readonly [number, number]
  max: readonly [number, number]
  cell: number
  rib: number
  seed: number
  jitter: number
  overhang: number
  gradient: number
  gradientMode: 'none' | 'vertical' | 'radial'
  slope: number
  frame: number
  radius: number
  keepOut: readonly (readonly [number, number, number, number])[]
  seamOffset: number
  seamDir: number
  perimeter: number
  openLeft: boolean
  openRight: boolean
  openBottom?: boolean | undefined
  coupleWalls?: CoupleWallsSpec | undefined
}

export interface FinalizeWallCellOptions {
  min: readonly [number, number]
  max: readonly [number, number]
  rib: number
  overhang: number
  slope: number
  frame: number
  radius: number
  maxSegs: number
  keepOut: readonly (readonly [number, number, number, number])[]
  openLeft: boolean
  openRight: boolean
  openBottom?: boolean | undefined
}

function fract(x: number): number {
  return x - Math.floor(x)
}

function hash(u: number, v: number, seed: number, off: number): number {
  return fract(Math.sin(u * 127.1 + v * 311.7 + seed * 91.3 + off) * 43758.5453) - 0.5
}

function wrapMod(n: number, period: number): number {
  return ((n % period) + period) % period
}

export function signedArea(poly: readonly Pt[]): number {
  if (poly.length < 3) return 0
  let sum = 0
  for (let i = 0; i < poly.length; i++) {
    const a = poly[i]!
    const b = poly[(i + 1) % poly.length]!
    sum += a[0] * b[1] - a[1] * b[0]
  }
  return sum * 0.5
}

export function bboxOfPoly(poly: readonly Pt[]): BBox {
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  for (const [x, y] of poly) {
    if (x < minX) minX = x
    if (y < minY) minY = y
    if (x > maxX) maxX = x
    if (y > maxY) maxY = y
  }
  return {
    minX,
    minY,
    maxX,
    maxY,
    width: maxX - minX,
    height: maxY - minY,
  }
}

export function sanitizeAndCCW(poly: readonly Pt[], eps = 1e-5): Pt[] {
  const clean: Pt[] = []
  for (const pt of poly) {
    if (!Number.isFinite(pt[0]) || !Number.isFinite(pt[1])) return []
    const last = clean[clean.length - 1]
    if (last && Math.hypot(pt[0] - last[0], pt[1] - last[1]) < eps) continue
    clean.push([pt[0], pt[1]])
  }
  while (clean.length >= 2) {
    const first = clean[0]!
    const last = clean[clean.length - 1]!
    if (Math.hypot(first[0] - last[0], first[1] - last[1]) < eps) clean.pop()
    else break
  }
  if (clean.length >= 3 && signedArea(clean) < 0) clean.reverse()
  return clean
}

export function clipPolygon(poly: readonly Pt[], n: readonly [number, number], d: number): Pt[] {
  if (poly.length < 3) return []
  const result: Pt[] = []
  for (let i = 0; i < poly.length; i++) {
    const a = poly[i]!
    const b = poly[(i + 1) % poly.length]!
    const da = a[0] * n[0] + a[1] * n[1] - d
    const db = b[0] * n[0] + b[1] * n[1] - d
    if (da <= 1e-9) result.push([a[0], a[1]])
    if (da < 0 !== db < 0) {
      const t = da / (da - db)
      result.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])])
    }
  }
  return result
}

function insetPolygon(poly: readonly Pt[], distance: number): Pt[] {
  if (distance <= 0) return sanitizeAndCCW(poly)
  const ccw = sanitizeAndCCW(poly)
  let result: Pt[] = ccw
  for (let i = 0; i < ccw.length && result.length >= 3; i++) {
    const a = ccw[i]!
    const b = ccw[(i + 1) % ccw.length]!
    const dx = b[0] - a[0]
    const dy = b[1] - a[1]
    const len = Math.hypot(dx, dy)
    if (len < 1e-8) continue
    const nx = dy / len
    const ny = -dx / len
    result = clipPolygon(result, [nx, ny], a[0] * nx + a[1] * ny - distance)
  }
  return sanitizeAndCCW(result)
}

function clipOverhangRoof(poly: readonly Pt[], angleDeg: number): Pt[] {
  if (poly.length < 3 || angleDeg <= 0) return sanitizeAndCCW(poly)
  const alpha = Math.atan(Math.tan((angleDeg * Math.PI) / 180) * 0.92)
  let yTop = -Infinity
  for (const [, y] of poly) if (y > yTop) yTop = y
  let xMinTop = Infinity
  let xMaxTop = -Infinity
  for (const [x, y] of poly) {
    if (y >= yTop - 1e-6) {
      if (x < xMinTop) xMinTop = x
      if (x > xMaxTop) xMaxTop = x
    }
  }
  const xApex = 0.5 * (xMinTop + xMaxTop)
  const c = Math.cos(alpha)
  const s = Math.sin(alpha)
  let out = clipPolygon(poly, [-c, s], -c * xApex + s * yTop)
  out = clipPolygon(out, [c, s], c * xApex + s * yTop)
  return sanitizeAndCCW(out, 1e-6)
}

function pointInConvexPoly(poly: readonly Pt[], pt: Pt): boolean {
  let pos = false
  let neg = false
  for (let i = 0; i < poly.length; i++) {
    const a = poly[i]!
    const b = poly[(i + 1) % poly.length]!
    const c = (b[0] - a[0]) * (pt[1] - a[1]) - (b[1] - a[1]) * (pt[0] - a[0])
    if (c > 1e-6) pos = true
    else if (c < -1e-6) neg = true
    if (pos && neg) return false
  }
  return true
}

function segmentIntersectsRect(a: Pt, b: Pt, r: readonly [number, number, number, number]): boolean {
  let t0 = 0
  let t1 = 1
  const dx = b[0] - a[0]
  const dy = b[1] - a[1]
  for (const [p, q] of [
    [-dx, a[0] - r[0]],
    [dx, r[2] - a[0]],
    [-dy, a[1] - r[1]],
    [dy, r[3] - a[1]],
  ] as const) {
    if (Math.abs(p) <= 1e-12) {
      if (q < 0) return false
    } else {
      const t = q / p
      if (p < 0) t0 = Math.max(t0, t)
      else t1 = Math.min(t1, t)
      if (t0 > t1) return false
    }
  }
  if (t1 - t0 <= 1e-6) return false
  const tm = 0.5 * (t0 + t1)
  const mx = a[0] + tm * dx
  const my = a[1] + tm * dy
  return mx > r[0] + 1e-6 && mx < r[2] - 1e-6 && my > r[1] + 1e-6 && my < r[3] - 1e-6
}

export function overlapsKeepOut(poly: readonly Pt[], rect: readonly [number, number, number, number]): boolean {
  const r: [number, number, number, number] = [
    Math.min(rect[0], rect[2]),
    Math.min(rect[1], rect[3]),
    Math.max(rect[0], rect[2]),
    Math.max(rect[1], rect[3]),
  ]
  if (r[2] - r[0] <= 1e-6 || r[3] - r[1] <= 1e-6) return false
  const bb = bboxOfPoly(poly)
  for (const [x, y] of poly) {
    if (x > r[0] + 1e-6 && x < r[2] - 1e-6 && y > r[1] + 1e-6 && y < r[3] - 1e-6) return true
  }
  if (bb.maxX <= r[0] + 1e-6 || bb.minX >= r[2] - 1e-6 || bb.maxY <= r[1] + 1e-6 || bb.minY >= r[3] - 1e-6) return false
  const probes: Pt[] = [
    [r[0], r[1]],
    [r[2], r[1]],
    [r[2], r[3]],
    [r[0], r[3]],
    [0.5 * (r[0] + r[2]), 0.5 * (r[1] + r[3])],
  ]
  if (probes.some(pt => pointInConvexPoly(poly, pt))) return true
  for (let i = 0; i < poly.length; i++) {
    if (segmentIntersectsRect(poly[i]!, poly[(i + 1) % poly.length]!, r)) return true
  }
  return false
}

export function roundVoronoiCell(
  poly: readonly Pt[],
  radius: number,
  overhangDeg: number,
  maxSegs = 12,
  skipVertex?: (index: number) => boolean,
): Pt[] {
  const n = poly.length
  if (n < 3 || radius <= 1e-4) return poly.map(([x, y]) => [x, y])
  const sinLimit = overhangDeg > 0 ? Math.sin(((overhangDeg + 0.05) * Math.PI) / 180) : 2
  const out: Pt[] = []
  for (let i = 0; i < n; i++) {
    if (skipVertex?.(i)) {
      const pt = poly[i]!
      out.push([pt[0], pt[1]])
      continue
    }
    const prev = poly[(i + n - 1) % n]!
    const curr = poly[i]!
    const next = poly[(i + 1) % n]!
    const ux = prev[0] - curr[0]
    const uy = prev[1] - curr[1]
    const vx = next[0] - curr[0]
    const vy = next[1] - curr[1]
    const lu = Math.hypot(ux, uy)
    const lv = Math.hypot(vx, vy)
    if (lu < 1e-4 || lv < 1e-4) {
      out.push([curr[0], curr[1]])
      continue
    }
    const udx = ux / lu
    const udy = uy / lu
    const vdx = vx / lv
    const vdy = vy / lv
    const cosA = Math.max(-0.99999, Math.min(0.99999, udx * vdx + udy * vdy))
    const angle = Math.acos(cosA)
    const half = 0.5 * angle
    const tanHalf = Math.tan(half)
    if (tanHalf <= 1e-4 || angle > Math.PI - 1e-3) {
      out.push([curr[0], curr[1]])
      continue
    }
    const t = Math.min(radius / tanHalf, 0.48 * Math.min(lu, lv))
    const effR = t * tanHalf
    if (effR <= 0.05) {
      out.push([curr[0], curr[1]])
      continue
    }
    const bx = udx + vdx
    const by = udy + vdy
    const bl = Math.hypot(bx, by)
    if (bl <= 1e-6) {
      out.push([curr[0], curr[1]])
      continue
    }
    const dist = effR / Math.sin(half)
    const cx = curr[0] + (bx / bl) * dist
    const cy = curr[1] + (by / bl) * dist
    const p0x = curr[0] + udx * t
    const p0y = curr[1] + udy * t
    const p1x = curr[0] + vdx * t
    const p1y = curr[1] + vdy * t
    const a0 = Math.atan2(p0y - cy, p0x - cx)
    const a1 = Math.atan2(p1y - cy, p1x - cx)
    let da = a1 - a0
    while (da <= -Math.PI) da += 2 * Math.PI
    while (da > Math.PI) da -= 2 * Math.PI
    const segs = Math.max(2, Math.min(maxSegs, Math.ceil((Math.abs(da) * effR) / 0.25)))
    const arc: Pt[] = []
    let safe = true
    for (let s = 0; s <= segs; s++) {
      const th = a0 + da * (s / segs)
      const pt: Pt = [cx + effR * Math.cos(th), cy + effR * Math.sin(th)]
      if (s > 0 && overhangDeg > 0) {
        const pPrev = arc[s - 1]!
        const dx = pt[0] - pPrev[0]
        const dy = pt[1] - pPrev[1]
        const len = Math.hypot(dx, dy)
        if (len > 1e-6 && -dx / len > sinLimit) {
          safe = false
          break
        }
      }
      arc.push(pt)
    }
    if (safe) out.push(...arc)
    else out.push([curr[0], curr[1]])
  }
  return out
}

/** Extend flat bottom chord edges straight down so rounded/clipped wall cells break cleanly through y = yLine. */
function extendBottomChords(poly: readonly Pt[], yLine: number, drop: number, eps = 1e-4): Pt[] {
  const n = poly.length
  if (n < 3) return poly.map(([x, y]) => [x, y])
  const onBot = poly.map(pt => Math.abs(pt[1] - yLine) <= eps)
  if (!onBot.some(Boolean) || onBot.every(Boolean)) return poly.map(([x, y]) => [x, y])
  const start = onBot.findIndex(b => !b)
  const out: Pt[] = []
  let i = 0
  while (i < n) {
    const idx = (start + i) % n
    if (!onBot[idx]) {
      const pt = poly[idx]!
      out.push([pt[0], pt[1]])
      i++
    } else {
      const firstPt = poly[idx]!
      let lastPt = firstPt
      while (i < n && onBot[(start + i) % n]) {
        lastPt = poly[(start + i) % n]!
        i++
      }
      if (Math.abs(lastPt[0] - firstPt[0]) > 0.02) {
        out.push([firstPt[0], yLine], [firstPt[0], yLine - drop], [lastPt[0], yLine - drop], [lastPt[0], yLine])
      } else {
        out.push([firstPt[0], firstPt[1]])
      }
    }
  }
  return out
}

function preFilterWallCell(
  rawPoly: readonly Pt[],
  opts: {
    min: readonly [number, number]
    max: readonly [number, number]
    rib: number
    overhang: number
    slope: number
    frame: number
    keepOut: readonly (readonly [number, number, number, number])[]
    openLeft: boolean
    openRight: boolean
    openBottom: boolean
  },
): Pt[] | null {
  const { min, max, rib, overhang, slope, frame, keepOut, openLeft, openRight, openBottom } = opts
  const inv = 1 / Math.hypot(1, slope)
  let poly: Pt[] = rawPoly.map(([x, y]) => [x, y])
  if (!openBottom) poly = clipPolygon(poly, [0, -1], -(min[1] + frame))
  poly = clipPolygon(poly, [0, 1], max[1] - frame)
  if (!openLeft) poly = clipPolygon(poly, [-1, 0], -(min[0] + frame))
  if (!openRight || Math.abs(slope) > 1e-6) {
    poly = clipPolygon(poly, [inv, -slope * inv], (max[0] - slope * min[1]) * inv - frame)
  }
  poly = sanitizeAndCCW(poly)
  if (poly.length < 3) return null
  if (overhang > 0) {
    poly = clipOverhangRoof(poly, overhang)
    if (poly.length < 3) return null
  }
  const checkPoly = openBottom ? sanitizeAndCCW(clipPolygon(poly, [0, -1], -min[1])) : poly
  if (checkPoly.length < 3) return null
  const bb = bboxOfPoly(checkPoly)
  if (bb.width < rib * 1.35 || bb.height < rib * 1.35 || Math.abs(signedArea(checkPoly)) < rib * rib * 0.9) return null
  if (keepOut.some(rect => overlapsKeepOut(checkPoly, rect))) return null
  return poly
}

function postRoundWallCell(
  poly: readonly Pt[],
  opts: {
    min: readonly [number, number]
    max: readonly [number, number]
    rib: number
    overhang: number
    radius: number
    maxSegs: number
    openLeft: boolean
    openRight: boolean
    openBottom: boolean
  },
): Pt[] | null {
  const { min, max, rib, overhang, radius, maxSegs, openLeft, openRight, openBottom } = opts
  const inputPoly = openBottom ? extendBottomChords(poly, min[1] - rib, Math.max(12, radius * 3), 2e-3) : poly
  let rounded = roundVoronoiCell(inputPoly, radius, overhang, maxSegs)
  if (openLeft) rounded = clipPolygon(rounded, [-1, 0], -(min[0] - 0.5))
  if (openRight) rounded = clipPolygon(rounded, [1, 0], max[0] + 0.5)
  if (openBottom) {
    rounded = clipPolygon(rounded, [0, -1], -min[1])
    rounded = sanitizeAndCCW(rounded, 0.02)
    if (rounded.length < 3 || Math.abs(signedArea(rounded)) <= 0.05) return null
    const bbPost = bboxOfPoly(rounded)
    if (bbPost.maxX <= min[0] + 0.02 || bbPost.minX >= max[0] - 0.02 || bbPost.maxY <= min[1] + 0.02) return null
    return extendBottomChords(rounded, min[1], 10)
  }
  rounded = sanitizeAndCCW(rounded, 0.02)
  if (rounded.length < 3 || Math.abs(signedArea(rounded)) <= 0.05) return null
  const bbPost = bboxOfPoly(rounded)
  if (bbPost.maxX <= min[0] + 0.02 || bbPost.minX >= max[0] - 0.02) return null
  return rounded
}

export function finalizeWallCell(rawPoly: readonly Pt[], opts: FinalizeWallCellOptions): Pt[] | null {
  const openBottom = opts.openBottom === true
  const pre = preFilterWallCell(rawPoly, { ...opts, openBottom })
  if (!pre) return null
  return postRoundWallCell(pre, { ...opts, openBottom })
}

export function deterministicWallMaxSegs(
  perimeter: number,
  panelWidth: number,
  height: number,
  colPitch: number,
  rowPitch: number,
): number {
  const spanEst = perimeter > 0 ? Math.max(perimeter * 0.45, panelWidth) : panelWidth
  const estCorners = Math.ceil(spanEst / Math.max(1, colPitch)) * Math.ceil(height / Math.max(1, rowPitch)) * 6
  return Math.max(3, Math.min(8, Math.floor(2000 / Math.max(1, estCorners))))
}

export function clampMaxSegsForPanel(polys: readonly Pt[][], baseMaxSegs: number): number {
  const panelCorners = polys.reduce((sum, p) => sum + p.length, 0)
  if (panelCorners > 900) return Math.max(1, Math.min(baseMaxSegs, Math.floor(3200 / panelCorners) - 1))
  return baseMaxSegs
}

export function survivesClipOut(
  poly: readonly Pt[],
  clipOut: readonly (readonly [number, number, number, number])[],
  rib: number,
): boolean {
  for (const rect of clipOut) {
    const rx0 = Math.min(rect[0], rect[2])
    const ry0 = Math.min(rect[1], rect[3])
    const rx1 = Math.max(rect[0], rect[2])
    const ry1 = Math.max(rect[1], rect[3])
    if (!overlapsKeepOut(poly, [rx0, ry0, rx1, ry1])) continue
    const halfSpaces: Array<[readonly [number, number], number]> = [
      [[1, 0], rx0],
      [[-1, 0], -rx1],
      [[0, 1], ry0],
      [[0, -1], -ry1],
    ]
    let survivesRect = false
    for (const [n, d] of halfSpaces) {
      const piece = sanitizeAndCCW(clipPolygon(poly, n, d))
      if (piece.length < 3) continue
      const bb = bboxOfPoly(piece)
      if (bb.width >= rib * 1.1 && bb.height >= rib * 1.1 && Math.abs(signedArea(piece)) >= rib * rib * 0.75) {
        survivesRect = true
        break
      }
    }
    if (!survivesRect) return false
  }
  return true
}

export function clipToBox(poly: readonly Pt[], min: readonly [number, number], max: readonly [number, number]): Pt[] {
  let out = clipPolygon(poly, [0, -1], -min[1])
  out = clipPolygon(out, [0, 1], max[1])
  out = clipPolygon(out, [-1, 0], -min[0])
  out = clipPolygon(out, [1, 0], max[0])
  return sanitizeAndCCW(out)
}

export function clipToInnerCavity(
  poly: readonly Pt[],
  min: readonly [number, number],
  max: readonly [number, number],
  innerR: number,
): Pt[] {
  let out = clipToBox(poly, min, max)
  if (out.length < 3 || innerR <= 0.05) return out
  const r = Math.min(innerR, 0.5 * (max[0] - min[0]) - 0.05, 0.5 * (max[1] - min[1]) - 0.05)
  if (r <= 0.05) return out
  const corners: ReadonlyArray<readonly [number, number, number, number, number]> = [
    [min[0] + r, min[1] + r, -1, -1, Math.PI],
    [max[0] - r, min[1] + r, 1, -1, -0.5 * Math.PI],
    [max[0] - r, max[1] - r, 1, 1, 0],
    [min[0] + r, max[1] - r, -1, 1, 0.5 * Math.PI],
  ]
  const segs = 6
  for (const [cx, cy, sx, sy, a0] of corners) {
    if (!out.some(([px, py]) => (px - cx) * sx > 0 && (py - cy) * sy > 0)) continue
    for (let k = 1; k < segs && out.length >= 3; k++) {
      const th = a0 + (k / segs) * (0.5 * Math.PI)
      const nx = Math.cos(th)
      const ny = Math.sin(th)
      out = clipPolygon(out, [nx, ny], cx * nx + cy * ny + r)
    }
  }
  return sanitizeAndCCW(out)
}

function finalizeOrDropForPanel(
  rawPolys: Pt[][],
  o: TrabecularCellsOptions,
  colPitch: number,
  rowPitch: number,
): Pt[][] {
  const height = o.max[1] - o.min[1]
  const baseMaxSegs = deterministicWallMaxSegs(o.perimeter, o.max[0] - o.min[0], height, colPitch, rowPitch)
  const openBottom = o.openBottom === true
  const preFiltered: Pt[][] = []
  for (const inset of rawPolys) {
    const pre = preFilterWallCell(inset, { ...o, openBottom })
    if (pre) preFiltered.push(pre)
  }
  const maxSegs = clampMaxSegsForPanel(preFiltered, baseMaxSegs)
  const out: Pt[][] = []
  for (const poly of preFiltered) {
    const finalized = postRoundWallCell(poly, { ...o, maxSegs, openBottom })
    if (finalized) out.push(finalized)
  }
  return out
}

function gammaAndNormalOnInnerCavity(
  sRaw: number,
  min: readonly [number, number],
  max: readonly [number, number],
  spec: CoupleWallsSpec,
): { p: Pt; n: Pt } {
  const { perimeter, spanX, sideSpan, cornerArc, innerR } = spec
  const xL = min[0]
  const xR = max[0]
  const yB = min[1]
  const yF = max[1]
  let s = wrapMod(sRaw, perimeter)
  if (s < spanX) return { p: [xL + innerR + s, yB], n: [0, -1] }
  s -= spanX
  if (s < cornerArc) {
    const a = cornerArc > 0 ? (s / cornerArc) * (Math.PI * 0.5) : 0
    return {
      p: [xR - innerR + innerR * Math.sin(a), yB + innerR - innerR * Math.cos(a)],
      n: [Math.sin(a), -Math.cos(a)],
    }
  }
  s -= cornerArc
  if (s < sideSpan) return { p: [xR, yB + innerR + s], n: [1, 0] }
  s -= sideSpan
  if (s < cornerArc) {
    const a = cornerArc > 0 ? (s / cornerArc) * (Math.PI * 0.5) : 0
    return {
      p: [xR - innerR + innerR * Math.cos(a), yF - innerR + innerR * Math.sin(a)],
      n: [Math.cos(a), Math.sin(a)],
    }
  }
  s -= cornerArc
  if (s < spanX) return { p: [xR - innerR - s, yF], n: [0, 1] }
  s -= spanX
  if (s < cornerArc) {
    const a = cornerArc > 0 ? (s / cornerArc) * (Math.PI * 0.5) : 0
    return {
      p: [xL + innerR - innerR * Math.sin(a), yF - innerR + innerR * Math.cos(a)],
      n: [-Math.sin(a), Math.cos(a)],
    }
  }
  s -= cornerArc
  if (s < sideSpan) return { p: [xL, yF - innerR - s], n: [-1, 0] }
  s -= sideSpan
  const a = cornerArc > 0 ? (s / cornerArc) * (Math.PI * 0.5) : 0
  return {
    p: [xL + innerR - innerR * Math.cos(a), yB + innerR - innerR * Math.sin(a)],
    n: [-Math.cos(a), -Math.sin(a)],
  }
}

function gammaOnInnerCavity(
  sRaw: number,
  min: readonly [number, number],
  max: readonly [number, number],
  spec: CoupleWallsSpec,
): Pt {
  return gammaAndNormalOnInnerCavity(sRaw, min, max, spec).p
}

function distToInnerCavity(
  px: number,
  py: number,
  min: readonly [number, number],
  max: readonly [number, number],
  innerR: number,
): number {
  const x0 = min[0] + innerR
  const x1 = max[0] - innerR
  const y0 = min[1] + innerR
  const y1 = max[1] - innerR
  const qx = Math.max(x0, Math.min(x1, px))
  const qy = Math.max(y0, Math.min(y1, py))
  if (px !== qx && py !== qy) return innerR - Math.hypot(px - qx, py - qy)
  return Math.min(px - min[0], max[0] - px, py - min[1], max[1] - py)
}

function normalizeWallOpenings(
  openings: ReadonlyArray<readonly [number, number]>,
  perimeter: number,
): Array<readonly [number, number]> {
  const out = openings
    .filter(([a, b]) => b - a > 0.05)
    .map(([a, b]) => {
      const aNorm = wrapMod(a, perimeter)
      return [aNorm, aNorm + (b - a)] as const
    })
    .sort((u, v) => u[0] - v[0])
  if (out.length >= 2 && out[0]![0] < 1e-3 && out[out.length - 1]![1] > perimeter - 1e-3) {
    const first = out.shift()!
    const last = out[out.length - 1]!
    out[out.length - 1] = [last[0], perimeter + first[1]]
  }
  return out
}

function computeTrabecularWallOpenings(
  spec: CoupleWallsSpec,
  gradient: number,
  radius: number,
): Array<readonly [number, number]> {
  const cells = generateTrabecularCells({
    min: [0, 0],
    max: [spec.perimeter, Math.max(20, spec.wallHeight ?? 76)],
    cell: spec.wallCell,
    rib: spec.wallRib,
    seed: spec.wallSeed,
    jitter: spec.wallJitter,
    overhang: spec.wallOverhang,
    gradient,
    gradientMode: 'vertical',
    slope: 0,
    frame: 0,
    radius,
    keepOut: [],
    seamOffset: -spec.sShift,
    seamDir: 1,
    perimeter: spec.perimeter,
    openLeft: true,
    openRight: true,
    openBottom: true,
  })
  const rawIv: Array<readonly [number, number]> = []
  const yProbe = 0.01
  for (const poly of cells) {
    const xs: number[] = []
    for (let i = 0; i < poly.length; i++) {
      const a = poly[i]!
      const b = poly[(i + 1) % poly.length]!
      if ((a[1] <= yProbe && b[1] >= yProbe) || (b[1] <= yProbe && a[1] >= yProbe)) {
        const dy = b[1] - a[1]
        if (Math.abs(dy) > 1e-9) xs.push(a[0] + ((yProbe - a[1]) / dy) * (b[0] - a[0]))
      }
    }
    if (xs.length >= 2) {
      const s0 = Math.max(0, Math.min(...xs))
      const s1 = Math.min(spec.perimeter, Math.max(...xs))
      if (s1 - s0 > 0.05) rawIv.push([s0, s1])
    }
  }
  rawIv.sort((a, b) => a[0] - b[0])
  const deduped: Array<readonly [number, number]> = []
  for (const iv of rawIv) {
    const prev = deduped[deduped.length - 1]
    if (prev && iv[0] < prev[1] - 0.05) {
      if (iv[1] > prev[1]) deduped[deduped.length - 1] = [prev[0], iv[1]]
    } else {
      deduped.push(iv)
    }
  }
  return normalizeWallOpenings(deduped, spec.perimeter)
}

function isSimplePolygon(poly: readonly Pt[]): boolean {
  const n = poly.length
  if (n < 3) return false
  const orient = (a: Pt, b: Pt, c: Pt) => (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
  for (let i = 0; i < n; i++) {
    const a = poly[i]!
    const b = poly[(i + 1) % n]!
    for (let j = i + 2; j < n; j++) {
      if (i === 0 && j === n - 1) continue
      const c = poly[j]!
      const d = poly[(j + 1) % n]!
      if (
        Math.max(a[0], b[0]) < Math.min(c[0], d[0]) ||
        Math.max(c[0], d[0]) < Math.min(a[0], b[0]) ||
        Math.max(a[1], b[1]) < Math.min(c[1], d[1]) ||
        Math.max(c[1], d[1]) < Math.min(a[1], b[1])
      ) {
        continue
      }
      const o1 = orient(a, b, c)
      const o2 = orient(a, b, d)
      const o3 = orient(c, d, a)
      const o4 = orient(c, d, b)
      if (
        ((o1 > 1e-8 && o2 < -1e-8) || (o1 < -1e-8 && o2 > 1e-8)) &&
        ((o3 > 1e-8 && o4 < -1e-8) || (o3 < -1e-8 && o4 > 1e-8))
      ) {
        return false
      }
    }
  }
  return true
}

/** Funnel a perimeter floor cell cleanly to exact wall opening endpoints [s0, s1] on inner_cavity_2d(). */
function funnelPerimeterCell(
  inset: readonly Pt[],
  s0: number,
  s1: number,
  o: Pick<TrabecularCellsOptions, 'min' | 'max' | 'rib' | 'radius'>,
  spec: CoupleWallsSpec,
  maxSegs: number,
): Pt[] | null {
  const dBlend = spec.innerR > 0 ? Math.min(2.0, Math.max(0.6, spec.innerR * 0.55)) : 1.8
  if (o.max[0] - o.min[0] <= 2 * dBlend + 0.5 || o.max[1] - o.min[1] <= 2 * dBlend + 0.5) return null
  const coreMin: [number, number] = [o.min[0] + dBlend, o.min[1] + dBlend]
  const coreMax: [number, number] = [o.max[0] - dBlend, o.max[1] - dBlend]
  const coreR = Math.max(0, spec.innerR - dBlend)
  const core = clipToInnerCavity(inset, coreMin, coreMax, coreR)
  if (core.length < 3) return null
  const onCore = core.map(pt => Math.abs(distToInnerCavity(pt[0], pt[1], coreMin, coreMax, coreR)) <= 0.06)
  if (!onCore.some(Boolean) || onCore.every(Boolean)) return null

  const n = core.length
  let runCount = 0
  let runStart = -1
  let runEnd = -1
  for (let i = 0; i < n; i++) {
    const prevOn = onCore[(i + n - 1) % n]!
    const currOn = onCore[i]!
    const nextOn = onCore[(i + 1) % n]!
    if (!prevOn && currOn) {
      runCount++
      runStart = i
    }
    if (currOn && !nextOn) runEnd = i
  }
  if (runCount !== 1 || runStart < 0 || runEnd < 0) return null

  const orderedInterior: Pt[] = []
  let idx = (runEnd + 1) % n
  while (idx !== runStart) {
    orderedInterior.push(core[idx]!)
    idx = (idx + 1) % n
  }
  if (orderedInterior.length === 0) return null

  const qStart = core[runStart]!
  const qEnd = core[runEnd]!
  const p0 = gammaAndNormalOnInnerCavity(s0, o.min, o.max, spec).p
  const p1 = gammaAndNormalOnInnerCavity(s1, o.min, o.max, spec).p
  const kSteps = Math.max(2, Math.min(6, Math.ceil((s1 - s0) / 3)))
  const outerArc: Pt[] = []
  for (let k = 0; k <= kSteps; k++) {
    const gt = gammaAndNormalOnInnerCavity(s0 + ((s1 - s0) * k) / kSteps, o.min, o.max, spec)
    outerArc.push([gt.p[0] + 1.2 * gt.n[0], gt.p[1] + 1.2 * gt.n[1]])
  }

  const mouthStart = orderedInterior.length + 1
  const mouthEnd = mouthStart + outerArc.length + 1
  const rawFunneled: Pt[] = [...orderedInterior, qStart, p0, ...outerArc, p1, qEnd]
  if (signedArea(rawFunneled) <= o.rib * o.rib * 0.5 || !isSimplePolygon(rawFunneled)) return null
  const rounded = roundVoronoiCell(rawFunneled, o.radius, 0, maxSegs, vIdx => vIdx >= mouthStart && vIdx <= mouthEnd)
  const clean = sanitizeAndCCW(rounded, 0.02)
  if (clean.length < 3 || Math.abs(signedArea(clean)) <= o.rib * o.rib * 0.5) return null
  return clean
}

function computeTrabecularWallRibS(spec: CoupleWallsSpec): number[] {
  const jScale = Math.max(0, Math.min(1.4, spec.wallJitter / 0.7))
  const ribs: number[] = []
  if (spec.wallOverhang > 0) {
    const wallH = Math.max(1, spec.wallHeight ?? 75)
    const localRatio = Math.max(0.25, Math.tan((spec.wallOverhang * Math.PI) / 180) * 0.9)
    const effCell = Math.min(40, Math.max(30, 26 + spec.wallCell))
    const rows = Math.min(80, Math.max(1, Math.ceil(wallH / (effCell * 0.65))))
    const ch = wallH / (0.542 * rows + 0.28)
    const cwNom = Math.min(ch * 0.56, Math.max(spec.wallRib * 2.8, effCell * 0.8)) * localRatio
    const periodCols = Math.max(4, Math.round(spec.perimeter / Math.max(1, cwNom)))
    const periodU = periodCols * 2
    const cw = spec.perimeter / periodCols
    const node = (ix: number, jy: number): [number, number] => {
      const ixMod = wrapMod(ix, periodU)
      const rowIdx = Math.floor(jy / 3)
      const phase = jy - 3 * rowIdx > 1.5 ? 1 : 0.16
      return [
        ix * cw * 0.5 + hash(ixMod, jy, spec.wallSeed, 17) * cw * 0.17 * jScale,
        (1.29 * rowIdx + phase) * ch * 0.42 +
          hash(ixMod, jy, spec.wallSeed, 83) * ch * 0.032 * jScale +
          Math.sin(ixMod * 25 + rowIdx * 103 + spec.wallSeed * 53) * ch * 0.04 * jScale,
      ]
    }
    for (let m = 0; m < periodCols; m++) {
      const [sA, yA] = node(2 * m + 1, -1)
      const [sB, yB] = node(2 * m + 1, 1)
      const denom = yB - yA
      const t = Math.abs(denom) > 1e-9 ? -yA / denom : 0.5
      ribs.push(wrapMod(sA + t * (sB - sA) + spec.sShift, spec.perimeter))
    }
  } else {
    const r = Math.max(spec.wallRib * 2.2, spec.wallCell)
    const rxNom = r * Math.sqrt(3) * 0.5
    const ry = r * 0.58
    const periodCols = Math.max(4, Math.round(spec.perimeter / (2 * rxNom)))
    const periodU = periodCols * 2
    const rx = spec.perimeter / periodU
    const node = (u: number, v: number): [number, number] => {
      const uMod = wrapMod(u, periodU)
      return [
        u * rx + hash(uMod, v, spec.wallSeed, 17) * rx * 0.32 * jScale,
        v * ry + hash(uMod, v, spec.wallSeed, 83) * ry * 0.48 * jScale,
      ]
    }
    for (let m = 0; m < periodCols; m++) {
      const [sA, yA] = node(2 * m + 1, -1)
      const [sB, yB] = node(2 * m + 1, 1)
      const denom = yB - yA
      const t = Math.abs(denom) > 1e-9 ? -yA / denom : 0.5
      ribs.push(wrapMod(sA + t * (sB - sA) + spec.sShift, spec.perimeter))
    }
  }
  ribs.sort((a, b) => a - b)
  return ribs
}

function polygonCentroid(poly: readonly Pt[]): Pt {
  let area2 = 0
  let cx = 0
  let cy = 0
  for (let i = 0; i < poly.length; i++) {
    const a = poly[i]!
    const b = poly[(i + 1) % poly.length]!
    const cross = a[0] * b[1] - b[0] * a[1]
    area2 += cross
    cx += (a[0] + b[0]) * cross
    cy += (a[1] + b[1]) * cross
  }
  if (Math.abs(area2) > 1e-9) return [cx / (3 * area2), cy / (3 * area2)]
  let sx = 0
  let sy = 0
  for (const [x, y] of poly) {
    sx += x
    sy += y
  }
  return [sx / Math.max(1, poly.length), sy / Math.max(1, poly.length)]
}

/** Generate 2D Voronoi floor cells whose perimeter openings and ribs meet all 4 walls continuously. */
export function emitCoupledVoronoiFloor(
  o: Pick<
    TrabecularCellsOptions,
    'min' | 'max' | 'cell' | 'rib' | 'seed' | 'jitter' | 'gradient' | 'radius' | 'keepOut'
  >,
  spec: CoupleWallsSpec,
  customWallRibS?: readonly number[],
  customWallOpenings?: ReadonlyArray<readonly [number, number]>,
): Pt[][] {
  if (o.max[0] - o.min[0] <= o.rib * 1.5 || o.max[1] - o.min[1] <= o.rib * 1.5 || spec.perimeter <= 0) return []
  const wallOpenings = customWallOpenings
    ? normalizeWallOpenings(customWallOpenings, spec.perimeter)
    : !customWallRibS
      ? computeTrabecularWallOpenings(spec, o.gradient, o.radius)
      : undefined
  const derivedRibSFromOpenings =
    wallOpenings && wallOpenings.length >= 3 && !customWallRibS
      ? wallOpenings.map(([, s1], k) => {
          const nextS0 = k === wallOpenings.length - 1 ? spec.perimeter + wallOpenings[0]![0] : wallOpenings[k + 1]![0]
          return 0.5 * (s1 + nextS0)
        })
      : undefined
  const sRibsRaw = customWallRibS
    ? customWallRibS.map(s => wrapMod(s, spec.perimeter)).sort((a, b) => a - b)
    : derivedRibSFromOpenings
      ? derivedRibSFromOpenings.map(s => wrapMod(s, spec.perimeter)).sort((a, b) => a - b)
      : computeTrabecularWallRibS(spec)
  const sRibs: number[] = []
  for (const s of sRibsRaw) {
    const prev = sRibs[sRibs.length - 1]
    if (prev !== undefined && s - prev < 1e-3) continue
    sRibs.push(s)
  }
  if (sRibs.length >= 2 && sRibs[0]! + spec.perimeter - sRibs[sRibs.length - 1]! < 1e-3) sRibs.pop()
  const nPerim = sRibs.length
  if (nPerim < 3) return []

  let sSites = sRibs.map((currWall, m) => {
    const prevWall = m === 0 ? sRibs[nPerim - 1]! - spec.perimeter : sRibs[m - 1]!
    return 0.5 * (prevWall + currWall)
  })
  for (let iter = 0; iter < 80; iter++) {
    const next = [...sSites]
    for (let m = 0; m < nPerim; m++) {
      const prevIdx = (m + nPerim - 1) % nPerim
      const nextIdx = (m + 1) % nPerim
      const Pm = gammaOnInnerCavity(sSites[m]!, o.min, o.max, spec)
      const Pprev = gammaOnInnerCavity(sSites[prevIdx]!, o.min, o.max, spec)
      const Pnext = gammaOnInnerCavity(sSites[nextIdx]!, o.min, o.max, spec)
      const Rl = gammaOnInnerCavity(sRibs[prevIdx]!, o.min, o.max, spec)
      const Rr = gammaOnInnerCavity(sRibs[m]!, o.min, o.max, spec)
      const errL = Math.hypot(Rl[0] - Pm[0], Rl[1] - Pm[1]) - Math.hypot(Rl[0] - Pprev[0], Rl[1] - Pprev[1])
      const errR = Math.hypot(Rr[0] - Pnext[0], Rr[1] - Pnext[1]) - Math.hypot(Rr[0] - Pm[0], Rr[1] - Pm[1])
      const prevWall = m === 0 ? sRibs[nPerim - 1]! - spec.perimeter : sRibs[m - 1]!
      const currWall = sRibs[m]!
      const span = Math.max(1e-3, currWall - prevWall)
      const minGap = Math.min(spec.wallRib * 0.6, 0.25 * span)
      next[m] = Math.max(prevWall + minGap, Math.min(currWall - minGap, sSites[m]! - 0.35 * (errL + errR)))
    }
    sSites = next
  }

  const perimSites: Pt[] = sSites.map(s => gammaOnInnerCavity(s, o.min, o.max, spec))
  const sites: Pt[] = [...perimSites]
  const w = o.max[0] - o.min[0]
  const h = o.max[1] - o.min[1]
  const ox = 0.5 * (o.min[0] + o.max[0])
  const oy = 0.5 * (o.min[1] + o.max[1])
  const basePitch = Math.min(spec.perimeter / nPerim, Math.max(o.rib * 2.2, o.cell) * 1.45)
  const pitchYNom = basePitch * Math.sqrt(3) * 0.5
  const minWall = Math.max(o.rib * 2.6, Math.min(0.32 * Math.min(w, h), pitchYNom * 0.85))
  const jFloor = Math.max(0, Math.min(1.4, o.jitter / 0.7))
  if (w > 2 * minWall - 2 && h > 2 * minWall - 2) {
    const spanIntX = Math.max(0.5, w - 2 * minWall)
    const spanIntY = Math.max(0.5, h - 2 * minWall)
    const nRows = Math.max(1, Math.min(80, Math.round(spanIntY / Math.max(1, pitchYNom)) + 1))
    const pitchY = nRows > 1 ? spanIntY / (nRows - 1) : spanIntY
    const nColsBase = Math.max(1, Math.min(80, Math.round(spanIntX / Math.max(1, basePitch)) + 1))
    const pitchX = nColsBase > 1 ? spanIntX / (nColsBase - 1) : spanIntX
    for (let r = 0; r < nRows && sites.length < 700; r++) {
      const odd = r % 2 === 1
      const cols = odd && nColsBase > 1 ? nColsBase - 1 : nColsBase
      const y0 = nRows > 1 ? o.min[1] + minWall + r * pitchY : oy
      for (let c = 0; c < cols && sites.length < 700; c++) {
        const x0 = cols > 1 ? o.min[0] + minWall + (odd ? (c + 0.5) * pitchX : c * pitchX) : ox
        const sx = x0 + hash(c, r, o.seed, 17) * pitchX * 0.22 * jFloor
        const sy = y0 + hash(c, r, o.seed, 83) * pitchY * 0.22 * jFloor
        if (distToInnerCavity(sx, sy, o.min, o.max, spec.innerR) < minWall * 0.78) continue
        if (perimSites.some(([px, py]) => Math.hypot(sx - px, sy - py) < pitchY * 0.55)) continue
        sites.push([sx, sy])
      }
    }

    const lloydIters = o.jitter > 0.02 ? 4 : 6
    const lloydAlpha = o.jitter > 0.02 ? 0.52 : 0.72
    const lloydBox: Pt[] = [
      [o.min[0], o.min[1]],
      [o.max[0], o.min[1]],
      [o.max[0], o.max[1]],
      [o.min[0], o.max[1]],
    ]
    for (let it = 0; it < lloydIters; it++) {
      const next = sites.map(([sx, sy]): Pt => [sx, sy])
      for (let i = nPerim; i < sites.length; i++) {
        const a = sites[i]!
        let poly: Pt[] = lloydBox
        for (let j = 0; j < sites.length && poly.length >= 3; j++) {
          if (i === j) continue
          const b = sites[j]!
          const n: [number, number] = [b[0] - a[0], b[1] - a[1]]
          const d = 0.5 * (b[0] * b[0] + b[1] * b[1] - a[0] * a[0] - a[1] * a[1])
          poly = clipPolygon(poly, n, d)
        }
        poly = clipToInnerCavity(poly, o.min, o.max, spec.innerR)
        if (poly.length < 3) continue
        const [cx, cy] = polygonCentroid(poly)
        const nx = a[0] + lloydAlpha * (cx - a[0])
        const ny = a[1] + lloydAlpha * (cy - a[1])
        if (distToInnerCavity(nx, ny, o.min, o.max, spec.innerR) >= minWall * 0.78) {
          next[i] = [nx, ny]
        }
      }
      for (let i = nPerim; i < sites.length; i++) sites[i] = next[i]!
    }
  }

  const pad = 12
  const bBox: Pt[] = [
    [o.min[0] - pad, o.min[1] - pad],
    [o.max[0] + pad, o.min[1] - pad],
    [o.max[0] + pad, o.max[1] + pad],
    [o.min[0] - pad, o.max[1] + pad],
  ]
  const flareOut = o.radius > 0.1 ? o.radius + 1.5 : 0
  const clipMin: [number, number] = [o.min[0] - flareOut, o.min[1] - flareOut]
  const clipMax: [number, number] = [o.max[0] + flareOut, o.max[1] + flareOut]
  const clipR = spec.innerR > 0 ? spec.innerR + flareOut : 0
  const keptCells: Array<{ inset: Pt[]; fallback: Pt[]; perimIdx: number }> = []
  for (let i = 0; i < sites.length; i++) {
    const a = sites[i]!
    let poly: Pt[] = bBox
    for (let j = 0; j < sites.length && poly.length >= 3; j++) {
      if (i === j) continue
      const b = sites[j]!
      const n: [number, number] = [b[0] - a[0], b[1] - a[1]]
      const d = 0.5 * (b[0] * b[0] + b[1] * b[1] - a[0] * a[0] - a[1] * a[1])
      poly = clipPolygon(poly, n, d)
    }
    if (poly.length < 3) continue
    const cx = Math.max(o.min[0], Math.min(o.max[0], a[0]))
    const cy = Math.max(o.min[1], Math.min(o.max[1], a[1]))
    const rxNorm = Math.min(1, Math.abs(cx - ox) / Math.max(1e-6, 0.5 * w))
    const ryNorm = Math.min(1, Math.abs(cy - oy) / Math.max(1e-6, 0.5 * h))
    const localRib = o.rib * Math.max(0.45, 1 + o.gradient * (-0.18 + 0.3 * Math.max(rxNorm, ryNorm)))
    const inset = insetPolygon(poly, localRib * 0.5)
    if (inset.length < 3) continue
    const innerPiece = clipToInnerCavity(inset, o.min, o.max, spec.innerR)
    if (innerPiece.length < 3) continue
    const bbInner = bboxOfPoly(innerPiece)
    if (
      bbInner.width < o.rib * 1.1 ||
      bbInner.height < o.rib * 1.1 ||
      Math.abs(signedArea(innerPiece)) < o.rib * o.rib * 0.75
    ) {
      continue
    }
    if (!o.keepOut.some(rect => overlapsKeepOut(innerPiece, rect))) {
      keptCells.push({
        inset,
        fallback: flareOut > 0 ? clipToInnerCavity(inset, clipMin, clipMax, clipR) : innerPiece,
        perimIdx: i < nPerim ? i : -1,
      })
    }
  }

  const findOpening = (perimIdx: number): readonly [number, number] | null => {
    if (!wallOpenings || perimIdx < 0) return null
    const siteS = sSites[perimIdx]!
    const prevRib = perimIdx === 0 ? sRibs[nPerim - 1]! - spec.perimeter : sRibs[perimIdx - 1]!
    const currRib = sRibs[perimIdx]!
    let best: readonly [number, number] | null = null
    let bestDist = Infinity
    for (const [raw0, raw1] of wallOpenings) {
      for (const shift of [-spec.perimeter, 0, spec.perimeter]) {
        const s0 = raw0 + shift
        const s1 = raw1 + shift
        const mid = 0.5 * (s0 + s1)
        if (mid < prevRib - 0.5 || mid > currRib + 0.5) continue
        const d = Math.abs(mid - siteS)
        if (d < bestDist && d < Math.max(currRib - prevRib, s1 - s0) * 0.65) {
          bestDist = d
          best = [s0, s1]
        }
      }
    }
    return best
  }

  const totalCorners = keptCells.reduce((sum, c) => sum + c.fallback.length, 0)
  const maxSegs = Math.max(1, Math.min(8, Math.floor(2500 / Math.max(1, totalCorners)) - 1))
  const out: Pt[][] = []
  for (const cell of keptCells) {
    const opening = findOpening(cell.perimIdx)
    if (opening) {
      const funneled = funnelPerimeterCell(cell.inset, opening[0], opening[1], o, spec, maxSegs)
      if (funneled) {
        out.push(funneled)
        continue
      }
    }
    const rounded = sanitizeAndCCW(roundVoronoiCell(cell.fallback, o.radius, 0, maxSegs), 0.02)
    if (rounded.length >= 3) out.push(rounded)
  }
  return out
}

function emitFloorCells(o: TrabecularCellsOptions, jScale: number): Pt[][] {
  const width = o.max[0] - o.min[0]
  const height = o.max[1] - o.min[1]
  const ox = 0.5 * (o.min[0] + o.max[0])
  const oy = 0.5 * (o.min[1] + o.max[1])
  const r = Math.max(o.rib * 2.2, o.cell)
  const dx = r * 0.5
  const dy = r * Math.sqrt(3) * 0.5
  const nc = Math.min(80, Math.ceil(width / (3 * r)) + 2)
  const nr = Math.min(80, Math.ceil(height / (Math.sqrt(3) * r)) + 2)
  const node = (u: number, v: number): Pt => [
    ox + u * dx + hash(u, v, o.seed, 17) * r * 0.28 * jScale,
    oy + v * dy + hash(u, v, o.seed, 83) * r * 0.28 * jScale,
  ]
  const inv = 1 / Math.hypot(1, o.slope)
  const preFiltered: Pt[][] = []
  for (let col = -nc; col <= nc && preFiltered.length < 1200; col++) {
    for (let row = -nr; row <= nr && preFiltered.length < 1200; row++) {
      const ix = 3 * col
      const iy = 2 * row + (Math.abs(col) % 2)
      const stress =
        o.gradient *
        (-0.18 + 0.32 * Math.min(1, Math.max(Math.abs(ix * r) / width, Math.abs(iy * Math.sqrt(3) * r) / height)))
      const localRib = o.rib * Math.max(0.45, 1 + stress)
      const hex: Pt[] = [
        node(ix + 2, iy),
        node(ix + 1, iy + 1),
        node(ix - 1, iy + 1),
        node(ix - 2, iy),
        node(ix - 1, iy - 1),
        node(ix + 1, iy - 1),
      ]
      const border = o.frame + localRib * 0.5
      let poly = insetPolygon(hex, localRib * 0.5)
      poly = clipPolygon(poly, [0, -1], -(o.min[1] + border))
      poly = clipPolygon(poly, [0, 1], o.max[1] - border)
      poly = clipPolygon(poly, [-1, 0], -(o.min[0] + border))
      poly = clipPolygon(poly, [inv, -o.slope * inv], (o.max[0] - o.slope * o.min[1]) * inv - border)
      poly = sanitizeAndCCW(poly)
      if (poly.length < 3) continue
      const bb = bboxOfPoly(poly)
      if (bb.width < o.rib * 1.25 || bb.height < o.rib * 1.25 || Math.abs(signedArea(poly)) < o.rib * o.rib * 0.75)
        continue
      if (!o.keepOut.some(rect => overlapsKeepOut(poly, rect))) preFiltered.push(poly)
    }
  }
  if (o.radius <= 1e-4) return preFiltered
  const maxSegs = clampMaxSegsForPanel(preFiltered, 8)
  return preFiltered
    .map(poly => sanitizeAndCCW(roundVoronoiCell(poly, o.radius, 0, maxSegs)))
    .filter(p => p.length >= 3)
}

function emitSupportFreeWallCells(o: TrabecularCellsOptions, jScale: number): Pt[][] {
  const width = o.max[0] - o.min[0]
  const height = o.max[1] - o.min[1]
  const usableH = Math.max(1, height - 2 * o.frame)
  const localRatio = Math.max(0.25, (Math.tan((o.overhang * Math.PI) / 180) - Math.abs(o.slope)) * 0.9)
  const effCell = Math.min(40, Math.max(30, 26 + o.cell))
  const rows = Math.min(80, Math.max(1, Math.ceil(usableH / (effCell * 0.65))))
  const ch = usableH / (0.542 * rows + 0.28)
  const cwNom = Math.min(ch * 0.56, Math.max(o.rib * 2.8, effCell * 0.8)) * localRatio
  const periodCols = o.perimeter > 0 ? Math.max(4, Math.round(o.perimeter / Math.max(1, cwNom))) : 0
  const periodU = periodCols * 2
  const cw = periodCols > 0 ? o.perimeter / periodCols : cwNom
  const spanX = Math.max(width, width + Math.abs(o.slope) * height)
  const sA = o.perimeter > 0 ? o.seamOffset : 0
  const sB = o.perimeter > 0 ? o.seamOffset + o.seamDir * spanX : spanX
  const sMin = Math.min(sA, sB)
  const sMax = Math.max(sA, sB)
  const c0 = Math.floor(sMin / Math.max(1, cw)) - 2
  const c1 = Math.min(c0 + 200, Math.ceil(sMax / Math.max(1, cw)) + 2)
  const sDir = o.perimeter > 0 ? o.seamDir : 1
  const sOff = o.perimeter > 0 ? o.seamOffset : 0
  const ox = o.min[0] + (o.perimeter > 0 ? 0 : o.frame)
  const oy = o.min[1] + o.frame

  const node = (ix: number, jy: number): Pt => {
    const ixMod = periodU > 0 ? wrapMod(ix, periodU) : ix
    const rowIdx = Math.floor(jy / 3)
    const phase = jy - 3 * rowIdx > 1.5 ? 1 : 0.16
    const sNode = ix * cw * 0.5 + hash(ixMod, jy, o.seed, 17) * cw * 0.17 * jScale
    return [
      ox + (sNode - sOff) * sDir,
      oy +
        (1.29 * rowIdx + phase) * ch * 0.42 +
        hash(ixMod, jy, o.seed, 83) * ch * 0.032 * jScale +
        Math.sin(ixMod * 25 + rowIdx * 103 + o.seed * 53) * ch * 0.04 * jScale,
    ]
  }
  const rawPolys: Pt[][] = []
  for (let row = -1; row <= rows + 1 && rawPolys.length < 1200; row++) {
    for (let col = c0; col <= c1 && rawPolys.length < 1200; col++) {
      const ix = 2 * col + (Math.abs(row) % 2)
      const jy = 3 * row
      const cy = oy + (1.29 * row + 0.5) * ch * 0.42
      const t = Math.max(0, Math.min(1, (cy - o.min[1]) / Math.max(1, height)))
      const localRib = o.rib * Math.max(0.45, 1 + o.gradient * (0.12 - 0.24 * t))
      const raw: Pt[] = [
        node(ix, jy + 2),
        node(ix - 1, jy + 1),
        node(ix - 1, jy - 1),
        node(ix, jy - 2),
        node(ix + 1, jy - 1),
        node(ix + 1, jy + 1),
      ]
      const poly = insetPolygon(raw, localRib * 0.5)
      if (poly.length >= 3) rawPolys.push(poly)
    }
  }
  return finalizeOrDropForPanel(rawPolys, o, cw, ch * 0.542)
}

function emitOrganicWallCells(o: TrabecularCellsOptions, jScale: number): Pt[][] {
  const width = o.max[0] - o.min[0]
  const height = o.max[1] - o.min[1]
  const r = Math.max(o.rib * 2.2, o.cell)
  const rxNom = r * Math.sqrt(3) * 0.5
  const ry = r * 0.58
  const periodCols = o.perimeter > 0 ? Math.max(4, Math.round(o.perimeter / (2 * rxNom))) : 0
  const periodU = periodCols * 2
  const rx = periodU > 0 ? o.perimeter / periodU : rxNom
  const span = Math.max(width, width + Math.abs(o.slope) * height)
  const sA = o.seamOffset
  const sB = o.seamOffset + o.seamDir * span
  const sMin = Math.min(sA, sB)
  const sMax = Math.max(sA, sB)
  const c0 = Math.floor(sMin / (2 * rx)) - 2
  const c1 = Math.min(c0 + 200, Math.ceil(sMax / (2 * rx)) + 2)
  const r0 = -2
  const r1 = Math.min(80, Math.ceil(height / (3 * ry)) + 2)

  const node = (u: number, v: number): Pt => {
    const uMod = periodU > 0 ? wrapMod(u, periodU) : u
    const sNode = u * rx + hash(uMod, v, o.seed, 17) * rx * 0.32 * jScale
    return [
      o.min[0] + (sNode - o.seamOffset) * o.seamDir,
      o.min[1] + v * ry + hash(uMod, v, o.seed, 83) * ry * 0.48 * jScale,
    ]
  }
  const rawPolys: Pt[][] = []
  for (let row = r0; row <= r1 && rawPolys.length < 1200; row++) {
    for (let col = c0; col <= c1 && rawPolys.length < 1200; col++) {
      const ix = 2 * col + (Math.abs(row) % 2)
      const iy = 3 * row
      const cy = o.min[1] + iy * ry
      const t = Math.max(0, Math.min(1, (cy - o.min[1]) / Math.max(1, height)))
      const localRib = o.rib * Math.max(0.45, 1 + o.gradient * (0.12 - 0.24 * t))
      const raw: Pt[] = [
        node(ix, iy + 2),
        node(ix - 1, iy + 1),
        node(ix - 1, iy - 1),
        node(ix, iy - 2),
        node(ix + 1, iy - 1),
        node(ix + 1, iy + 1),
      ]
      const poly = insetPolygon(raw, localRib * 0.5)
      if (poly.length >= 3) rawPolys.push(poly)
    }
  }
  return finalizeOrDropForPanel(rawPolys, o, 2 * rx, 3 * ry)
}

/** Generate shared-vertex trivalent organic Voronoi cells with continuous periodic wall wrap. */
export function generateTrabecularCells(o: TrabecularCellsOptions): Pt[][] {
  if (o.max[0] - o.min[0] <= o.rib * 1.5 || o.max[1] - o.min[1] <= o.rib * 1.5) return []
  const jScale = Math.max(0, Math.min(1.4, o.jitter / 0.7))
  if (o.gradientMode === 'radial') {
    return o.coupleWalls ? emitCoupledVoronoiFloor(o, o.coupleWalls) : emitFloorCells(o, jScale)
  }
  if (o.overhang > 0) return emitSupportFreeWallCells(o, jScale)
  return emitOrganicWallCells(o, jScale)
}
