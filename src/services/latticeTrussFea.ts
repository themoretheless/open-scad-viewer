/**
 * Lightweight 3D truss FEA for lattice spatial graphs.
 * Pin-jointed bars, small-displacement linear elasticity — screening only.
 */
import {
 actionToForceN,
 faceMetricsFromSize,
 sectionProperties,
 type LatticeAction,
 type LatticeSupport,
 type StrutSection,
 type Vec3,
} from './latticeStrengthScenario'

export interface TrussGraph {
 nodes: number[][]
 edges: number[][]
}

export interface TrussMemberResult {
 edgeIndex: number
 a: number
 b: number
 length: number
 axialN: number
 stressMPa: number
 utilization: number
 bucklingRatio: number
 combinedUtil: number
 mid: Vec3
}

export interface TrussFeaResult {
 displacements: Vec3[]
 members: TrussMemberResult[]
 maxDeflectionMm: number
 maxUtilization: number
 maxBucklingRatio: number
 reactionN: number
 homogenized: {EStar: number; sigmaStar: number; relativeDensity: number}
 warnings: {ru: string; en: string}[]
}

const EPS = 1e-9
const round = (v: number, d = 3) => Math.round(v * 10 ** d) / 10 ** d

function bodyBounds(nodes: number[][]) {
 const min = [Infinity, Infinity, Infinity], max = [-Infinity, -Infinity, -Infinity]
 for (const p of nodes) for (let k = 0; k < 3; k++) {
  min[k] = Math.min(min[k], p[k]); max[k] = Math.max(max[k], p[k])
 }
 return {min, max, size: [max[0] - min[0], max[1] - min[1], max[2] - min[2]] as Vec3}
}

function faceNodes(nodes: number[][], normal: Vec3, tolFrac = 0.08): number[] {
 const {min, max, size} = bodyBounds(nodes)
 const tol = Math.max(1e-3, Math.hypot(size[0], size[1], size[2]) * tolFrac)
 const out: number[] = []
 for (let i = 0; i < nodes.length; i++) {
  const p = nodes[i]
  let on = false
  if (normal[0] < -0.5) on = Math.abs(p[0] - min[0]) <= tol
  else if (normal[0] > 0.5) on = Math.abs(p[0] - max[0]) <= tol
  else if (normal[1] < -0.5) on = Math.abs(p[1] - min[1]) <= tol
  else if (normal[1] > 0.5) on = Math.abs(p[1] - max[1]) <= tol
  else if (normal[2] < -0.5) on = Math.abs(p[2] - min[2]) <= tol
  else if (normal[2] > 0.5) on = Math.abs(p[2] - max[2]) <= tol
  if (on) out.push(i)
 }
 return out
}

function centroid(nodes: number[][]): Vec3 {
 const s: Vec3 = [0, 0, 0]
 for (const p of nodes) {s[0] += p[0]; s[1] += p[1]; s[2] += p[2]}
 const n = Math.max(nodes.length, 1)
 return [s[0] / n, s[1] / n, s[2] / n]
}

function nearestNode(nodes: number[][], at: Vec3): number {
 let best = 0, bestD = Infinity
 for (let i = 0; i < nodes.length; i++) {
  const d = Math.hypot(nodes[i][0] - at[0], nodes[i][1] - at[1], nodes[i][2] - at[2])
  if (d < bestD) {bestD = d; best = i}
 }
 return best
}

function assembleLoads(nodes: number[][], actions: LatticeAction[], size: Vec3): Float64Array {
 const F = new Float64Array(nodes.length * 3)
 const fallback = faceMetricsFromSize(size, [0, 0, 1])
 for (const action of actions) {
  const idxs = action.faceNormal
   ? faceNodes(nodes, action.faceNormal)
   : [nearestNode(nodes, action.at ?? centroid(nodes))]
  if (!idxs.length) continue
  const metrics = action.faceNormal
   ? faceMetricsFromSize(size, action.faceNormal)
   : fallback
  const L = Math.hypot(action.direction[0], action.direction[1], action.direction[2]) || 1
  const dir: Vec3 = [action.direction[0] / L, action.direction[1] / L, action.direction[2] / L]
  if (action.kind === 'moment' && idxs.length >= 2) {
   // Force couple on face extremities along the load direction.
   let minI = idxs[0], maxI = idxs[0], minP = Infinity, maxP = -Infinity
   for (const i of idxs) {
    const p = nodes[i][0] * dir[0] + nodes[i][1] * dir[1] + nodes[i][2] * dir[2]
    if (p < minP) {minP = p; minI = i}
    if (p > maxP) {maxP = p; maxI = i}
   }
   const lever = Math.max(Math.abs(maxP - minP), 0.25 * metrics.spanMm, 1)
   const couple = action.magnitude / lever
   F[minI * 3] -= couple * dir[0]; F[minI * 3 + 1] -= couple * dir[1]; F[minI * 3 + 2] -= couple * dir[2]
   F[maxI * 3] += couple * dir[0]; F[maxI * 3 + 1] += couple * dir[1]; F[maxI * 3 + 2] += couple * dir[2]
   continue
  }
  const force = actionToForceN(action, metrics.areaMm2, metrics.spanMm)
  const share = force / idxs.length
  for (const i of idxs) {
   F[i * 3] += share * dir[0]
   F[i * 3 + 1] += share * dir[1]
   F[i * 3 + 2] += share * dir[2]
  }
 }
 return F
}

/** Nodes near ≥2 bounding-box faces (geometric corners / edge junctions). */
function cornerNodeFlags(nodes: number[][], tolFrac = 0.1): boolean[] {
 const {min, max, size} = bodyBounds(nodes)
 const tol = Math.max(1e-3, Math.hypot(size[0], size[1], size[2]) * tolFrac)
 return nodes.map(p => {
  let hits = 0
  if (Math.abs(p[0] - min[0]) <= tol || Math.abs(p[0] - max[0]) <= tol) hits++
  if (Math.abs(p[1] - min[1]) <= tol || Math.abs(p[1] - max[1]) <= tol) hits++
  if (Math.abs(p[2] - min[2]) <= tol || Math.abs(p[2] - max[2]) <= tol) hits++
  return hits >= 2
 })
}

function restrain(nodes: number[][], supports: LatticeSupport[]): boolean[] {
 const fixed = new Array(nodes.length * 3).fill(false)
 for (const s of supports) {
  const face = faceNodes(nodes, s.normal)
  if (!face.length) continue
  if (s.kind === 'roller') {
   const ax = Math.abs(s.normal[0]) > 0.5 ? 0 : Math.abs(s.normal[1]) > 0.5 ? 1 : 2
   for (const i of face) fixed[i * 3 + ax] = true
   continue
  }
  if (s.kind === 'pinned') {
   // Pin on a face: restrain normal on all face nodes; fully fix the face centroid
   // so the truss stays determinate without over-constraining in-plane expansion.
   const ax = Math.abs(s.normal[0]) > 0.5 ? 0 : Math.abs(s.normal[1]) > 0.5 ? 1 : 2
   for (const i of face) fixed[i * 3 + ax] = true
   const c = centroid(face.map(i => nodes[i]))
   const anchor = nearestNode(nodes, c)
   fixed[anchor * 3] = fixed[anchor * 3 + 1] = fixed[anchor * 3 + 2] = true
   continue
  }
  // fixed / encastre face
  for (const i of face) fixed[i * 3] = fixed[i * 3 + 1] = fixed[i * 3 + 2] = true
 }
 if (!fixed.some(Boolean)) {
  const {min} = bodyBounds(nodes)
  for (let i = 0; i < nodes.length; i++) {
   if (Math.abs(nodes[i][2] - min[2]) <= 1e-6) fixed[i * 3] = fixed[i * 3 + 1] = fixed[i * 3 + 2] = true
  }
 }
 return fixed
}

function solveDense(A: Float64Array, b: Float64Array, n: number): Float64Array {
 const M = A.slice(), x = b.slice()
 for (let k = 0; k < n; k++) {
  let piv = k, best = Math.abs(M[k * n + k])
  for (let i = k + 1; i < n; i++) {
   const v = Math.abs(M[i * n + k])
   if (v > best) {best = v; piv = i}
  }
  if (best < EPS) continue
  if (piv !== k) {
   for (let j = k; j < n; j++) {
    const tmp = M[k * n + j]; M[k * n + j] = M[piv * n + j]; M[piv * n + j] = tmp
   }
   const tb = x[k]; x[k] = x[piv]; x[piv] = tb
  }
  const akk = M[k * n + k]
  for (let i = k + 1; i < n; i++) {
   const f = M[i * n + k] / akk
   x[i] -= f * x[k]
   for (let j = k; j < n; j++) M[i * n + j] -= f * M[k * n + j]
  }
 }
 for (let i = n - 1; i >= 0; i--) {
  let s = x[i]
  for (let j = i + 1; j < n; j++) s -= M[i * n + j] * x[j]
  x[i] = Math.abs(M[i * n + i]) < EPS ? 0 : s / M[i * n + i]
 }
 return x
}

export function solveLatticeTruss(options: {
 graph: TrussGraph
 E: number
 sigmaAllow: number
 safety: number
 section: StrutSection
 supports: LatticeSupport[]
 actions: LatticeAction[]
 relativeDensity: number
 /** Local stress concentration at corner nodes (default 1). */
 Kt?: number
}): TrussFeaResult {
 const {nodes, edges} = options.graph
 const warnings: {ru: string; en: string}[] = []
 if (!nodes.length || !edges.length) {
  return {
   displacements: [], members: [], maxDeflectionMm: 0, maxUtilization: 0, maxBucklingRatio: 0, reactionN: 0,
   homogenized: {EStar: 0, sigmaStar: 0, relativeDensity: options.relativeDensity},
   warnings: [{ru: 'Пустой граф — FEA пропущена.', en: 'Empty graph — FEA skipped.'}],
  }
 }
 const sec = sectionProperties(options.section)
 const {area, I, W} = sec
 const ecc = Math.max(0, options.section.eccentricity ?? 0)
 const joint = Math.min(1, Math.max(0, options.section.jointStiffness ?? 0.5))
 const Klen = 1 - 0.5 * joint
 const Kt = Math.max(1, options.Kt ?? 1)
 const corner = cornerNodeFlags(nodes)
 const n = nodes.length, ndof = n * 3
 const Kglob = new Float64Array(ndof * ndof)
 const {size} = bodyBounds(nodes)
 const F = assembleLoads(nodes, options.actions, size)
 const fixed = restrain(nodes, options.supports)

 for (const [a, b] of edges) {
  const pa = nodes[a], pb = nodes[b]
  const dx = pb[0] - pa[0], dy = pb[1] - pa[1], dz = pb[2] - pa[2]
  const L = Math.hypot(dx, dy, dz) || EPS
  const cx = dx / L, cy = dy / L, cz = dz / L
  const k = options.E * area / L
  const coeffs: [number, number][] = [
   [a * 3, cx], [a * 3 + 1, cy], [a * 3 + 2, cz],
   [b * 3, -cx], [b * 3 + 1, -cy], [b * 3 + 2, -cz],
  ]
  for (const [i, ci] of coeffs) for (const [j, cj] of coeffs) Kglob[i * ndof + j] += k * ci * cj
 }

 const map = new Int32Array(ndof).fill(-1)
 let m = 0
 for (let i = 0; i < ndof; i++) if (!fixed[i]) map[i] = m++
 if (m === 0) {
  return {
   displacements: nodes.map(() => [0, 0, 0] as Vec3), members: [], maxDeflectionMm: 0, maxUtilization: 0,
   maxBucklingRatio: 0, reactionN: 0, homogenized: {EStar: 0, sigmaStar: 0, relativeDensity: options.relativeDensity},
   warnings: [{ru: 'Все степени свободы закреплены.', en: 'All degrees of freedom are restrained.'}],
  }
 }
 const Kred = new Float64Array(m * m), Fred = new Float64Array(m)
 for (let i = 0; i < ndof; i++) {
  const ir = map[i]; if (ir < 0) continue
  Fred[ir] = F[i]
  for (let j = 0; j < ndof; j++) {
   const jr = map[j]; if (jr < 0) continue
   Kred[ir * m + jr] = Kglob[i * ndof + j]
  }
 }
 const uRed = solveDense(Kred, Fred, m)
 const u = new Float64Array(ndof)
 for (let i = 0; i < ndof; i++) if (map[i] >= 0) u[i] = uRed[map[i]]

 const displacements: Vec3[] = []
 let maxDefl = 0
 for (let i = 0; i < n; i++) {
  const d: Vec3 = [u[i * 3], u[i * 3 + 1], u[i * 3 + 2]]
  displacements.push(d)
  maxDefl = Math.max(maxDefl, Math.hypot(d[0], d[1], d[2]))
 }

 const members: TrussMemberResult[] = []
 let maxUtil = 0, maxBuck = 0, reaction = 0
 for (let ei = 0; ei < edges.length; ei++) {
  const [a, b] = edges[ei]
  const pa = nodes[a], pb = nodes[b]
  const dx = pb[0] - pa[0], dy = pb[1] - pa[1], dz = pb[2] - pa[2]
  const L = Math.hypot(dx, dy, dz) || EPS
  const cx = dx / L, cy = dy / L, cz = dz / L
  const elongation = (u[b * 3] - u[a * 3]) * cx + (u[b * 3 + 1] - u[a * 3 + 1]) * cy + (u[b * 3 + 2] - u[a * 3 + 2]) * cz
  const axialN = options.E * area / L * elongation
  const localKt = (corner[a] || corner[b]) ? Kt : 1
  const stress = axialN / area * localKt
  const util = Math.abs(stress) / Math.max(options.sigmaAllow / options.safety, EPS)
  const Pcr = (Math.PI ** 2 * options.E * I) / ((Klen * L) ** 2)
  const buck = axialN < 0 ? Math.abs(axialN) / Math.max(Pcr, EPS) : 0
  const bending = ecc > 0 && W > 0 ? Math.abs(axialN) * ecc / W : 0
  const combined = (Math.abs(stress) + bending * localKt) / Math.max(options.sigmaAllow / options.safety, EPS)
  maxUtil = Math.max(maxUtil, util, combined)
  maxBuck = Math.max(maxBuck, buck)
  reaction += Math.abs(axialN)
  members.push({
   edgeIndex: ei, a, b, length: L, axialN, stressMPa: stress, utilization: util,
   bucklingRatio: buck, combinedUtil: combined,
   mid: [(pa[0] + pb[0]) / 2, (pa[1] + pb[1]) / 2, (pa[2] + pb[2]) / 2],
  })
 }

 const forceProxy = Math.max(1e-6, [...F].reduce((s, v) => s + Math.abs(v), 0) / 2)
 const span = Math.max(size[0], size[1], size[2], 1)
 const face = Math.max(size[0] * size[1], size[0] * size[2], size[1] * size[2], 1)
 const EStar = Math.min(options.E, forceProxy * span / Math.max(maxDefl * face, EPS))
 const sigmaStar = Math.min(options.sigmaAllow, forceProxy / face)

 if (maxUtil > 1) warnings.push({
  ru: `Truss FEA: η_max≈${round(maxUtil)} > 1 — превышение допуска.`,
  en: `Truss FEA: η_max≈${round(maxUtil)} > 1 — allowable exceeded.`,
 })
 if (maxBuck > 0.7) warnings.push({
  ru: `Truss FEA: P/Pcr_max≈${round(maxBuck)} — низкий запас устойчивости.`,
  en: `Truss FEA: P/Pcr_max≈${round(maxBuck)} — low buckling margin.`,
 })
 if (ecc > 0) warnings.push({
  ru: `Учтено сжатие+изгиб (e=${ecc} mm).`,
  en: `Combined compression+bending included (e=${ecc} mm).`,
 })
 if (Kt > 1.01 && corner.some(Boolean)) warnings.push({
  ru: `Локальный Kt=${round(Kt, 2)} на стержнях у углов графа.`,
  en: `Local Kt=${round(Kt, 2)} on members at graph corners.`,
 })

 return {
  displacements,
  members: members.sort((x, y) => y.combinedUtil - x.combinedUtil),
  maxDeflectionMm: maxDefl,
  maxUtilization: maxUtil,
  maxBucklingRatio: maxBuck,
  reactionN: reaction / 2,
  homogenized: {EStar, sigmaStar, relativeDensity: options.relativeDensity},
  warnings,
 }
}

export function panelBucklingStressMPa(opts: {
 E: number
 wallThickness: number
 cellSize: number
 poisson?: number
}): number {
 const nu = opts.poisson ?? 0.3
 const t = Math.max(opts.wallThickness, 1e-6)
 const b = Math.max(opts.cellSize, t)
 return (4 * Math.PI ** 2 * opts.E) / (12 * (1 - nu * nu)) * (t / b) ** 2
}

export function compareLatticeVariants(
 a: {label: string; massProxy: number; stiffnessProxy: number; utilization: number; bucklingRatio: number},
 b: {label: string; massProxy: number; stiffnessProxy: number; utilization: number; bucklingRatio: number},
): {
 rows: {metric: string; a: number; b: number; deltaPct: number}[]
 winner: 'a' | 'b' | 'tie'
 summary: {ru: string; en: string}
} {
 const row = (metric: string, av: number, bv: number) => ({
  metric, a: av, b: bv,
  deltaPct: bv === 0 ? (av === 0 ? 0 : 100) : ((av - bv) / Math.abs(bv)) * 100,
 })
 const rows = [
  row('mass', a.massProxy, b.massProxy),
  row('stiffness', a.stiffnessProxy, b.stiffnessProxy),
  row('utilization', a.utilization, b.utilization),
  row('bucklingRatio', a.bucklingRatio, b.bucklingRatio),
 ]
 const score = (v: typeof a) =>
  v.stiffnessProxy / Math.max(v.massProxy, 1e-9) / Math.max(v.utilization, 0.05) / Math.max(v.bucklingRatio, 0.05)
 const sa = score(a), sb = score(b)
 const winner = Math.abs(sa - sb) / Math.max(sa, sb, 1e-9) < 0.05 ? 'tie' : sa > sb ? 'a' : 'b'
 return {
  rows, winner,
  summary: {
   ru: winner === 'tie' ? `Варианты ${a.label} и ${b.label} близки.`
    : winner === 'a' ? `${a.label} лучше по жёсткость/(масса·η·P/Pcr).`
    : `${b.label} лучше по жёсткость/(масса·η·P/Pcr).`,
   en: winner === 'tie' ? `Variants ${a.label} and ${b.label} are close.`
    : winner === 'a' ? `${a.label} wins on stiffness/(mass·η·P/Pcr).`
    : `${b.label} wins on stiffness/(mass·η·P/Pcr).`,
  },
 }
}

/** Ashby-based expected delta E* / delta m for a relative-density scale factor. */
function ashbyDeltas(rhoScale: number, n: number): {deltaStiffnessPct: number; deltaMassPct: number} {
 const r = Math.max(1e-6, rhoScale)
 return {
  deltaMassPct: Math.round((r - 1) * 100),
  deltaStiffnessPct: Math.round((r ** n - 1) * 100),
 }
}

export function latticeDesignAdvice(input: {
 cell: number
 rib: number
 pattern: string
 utilization: number
 bucklingRatio: number
 maxBridgeMm: number
 minWallMm: number
 hasDiagonals: boolean
 wallDepth?: number
 /** Gibson–Ashby exponent for Δη estimates (default 1.5). */
 ashbyExponent?: number
}): {ru: string; en: string; deltaStiffnessPct: number; deltaMassPct: number}[] {
 const tips: {ru: string; en: string; deltaStiffnessPct: number; deltaMassPct: number}[] = []
 const n = Math.max(1, input.ashbyExponent ?? 1.5)
 // Spatial strut lattices: ρ* ∼ (rib/cell)²; channel grids closer to rib/cell.
 const strutScale = input.pattern === 'grid' || input.pattern === 'web' || input.pattern === 'honey' ? 1 : 2
 if (input.bucklingRatio > 0.7 || input.utilization > 1) {
  const ribScale = 1.25
  const dRib = ashbyDeltas(ribScale ** strutScale, n)
  tips.push({
   ru: `Увеличить диаметр стержня ${input.rib.toFixed(1)} → ${(input.rib * ribScale).toFixed(1)} mm`,
   en: `Increase strut diameter ${input.rib.toFixed(1)} → ${(input.rib * ribScale).toFixed(1)} mm`,
   ...dRib,
  })
  const cellScale = 0.85
  const dCell = ashbyDeltas((1 / cellScale) ** strutScale, n)
  tips.push({
   ru: `Уменьшить ячейку ${input.cell.toFixed(1)} → ${(input.cell * cellScale).toFixed(1)} mm`,
   en: `Reduce cell ${input.cell.toFixed(1)} → ${(input.cell * cellScale).toFixed(1)} mm`,
   ...dCell,
  })
 }
 if (!input.hasDiagonals && (input.pattern === 'spatial' || input.pattern === 'grid' || input.pattern === 'bcc')) {
  tips.push({
   ru: 'Добавить диагонали (stretch-dominated) — рост E* при той же ρ*',
   en: 'Add diagonals (stretch-dominated) — higher E* at same ρ*',
   deltaStiffnessPct: Math.round(25 + 10 * Math.max(0, n - 1)),
   deltaMassPct: 12,
  })
 }
 if (input.cell - input.rib > input.maxBridgeMm) {
  tips.push({
   ru: `Мост ${(input.cell - input.rib).toFixed(1)} mm > лимита ${input.maxBridgeMm} mm`,
   en: `Bridge ${(input.cell - input.rib).toFixed(1)} mm > limit ${input.maxBridgeMm} mm`,
   deltaStiffnessPct: 0, deltaMassPct: 0,
  })
 }
 if (input.rib < input.minWallMm) {
  tips.push({
   ru: `Стержень ${input.rib} mm < min wall ${input.minWallMm} mm`,
   en: `Strut ${input.rib} mm < min wall ${input.minWallMm} mm`,
   deltaStiffnessPct: 0, deltaMassPct: 0,
  })
 }
 if ((input.wallDepth ?? 0) > 0 && (input.wallDepth ?? 0) < input.rib * 2) {
  tips.push({
   ru: 'Углубить скелетную стенку ≥ 2·диаметра стержня',
   en: 'Deepen skeletal wall to ≥ 2× strut diameter',
   ...ashbyDeltas(1.15, n),
  })
 }
 if (!tips.length) {
  tips.push({
   ru: 'Запасы приемлемы; можно слегка увеличить ячейку для экономии массы',
   en: 'Margins OK; slightly larger cell can save mass',
   ...ashbyDeltas(0.88, n),
  })
 }
 return tips
}

export function utilizationColor(util: number): [number, number, number, number] {
 const t = Math.max(0, Math.min(1.5, util)) / 1.5
 if (t < 0.33) {
  const u = t / 0.33
  return [0.15 * u, 0.55 + 0.35 * u, 0.95, 1]
 }
 if (t < 0.66) {
  const u = (t - 0.33) / 0.33
  return [0.2 + 0.8 * u, 0.9, 0.3 * (1 - u), 1]
 }
 const u = (t - 0.66) / 0.34
 return [1, 0.85 * (1 - u), 0.05, 1]
}
