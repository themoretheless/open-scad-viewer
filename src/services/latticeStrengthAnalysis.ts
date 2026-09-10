import {inspectPolygonMesh} from './geometry/polygon'
import {
 isSpatialPattern,
 latticeStructureHint,
 lighteningCells,
 spatialGraph,
 type LatticeLoadClass,
 type LighteningOptions,
 type LighteningPattern,
} from './solidLightening'
import type {DirectBody} from './directModeling'

export type LatticeMaterialId = 'pla' | 'petg' | 'abs' | 'nylon' | 'al6061' | 'steel'
export type LatticeLoadCase = 'compression' | 'tension' | 'bending'

export interface LatticeMaterial {
 id: LatticeMaterialId
 ru: string
 en: string
 /** Young's modulus, MPa (= N/mm^2) */
 E: number
 /** Allowable stress proxy, MPa */
 sigmaAllow: number
 /** Density, g/cm^3 */
 density: number
}

export interface LatticeLoadInput {
 case: LatticeLoadCase
 /** Applied force, N */
 forceN: number
 /** Safety factor on allowable stress (>= 1) */
 safety: number
}

export interface LatticeStrengthReport {
 pattern: LighteningPattern
 loadClass: LatticeLoadClass
 relativeDensity: number
 volumeReductionPct: number
 ashbyExponent: number
 relativeStiffness: number
 relativeStrength: number
 specificStiffness: number
 meanStrutLengthMm: number | null
 strutAreaMm2: number | null
 strutStressMPa: number | null
 eulerBucklingN: number | null
 bucklingRatio: number | null
 utilization: number | null
 connectivity: number | null
 warnings: {ru: string; en: string}[]
 disclaimer: {ru: string; en: string}
 /** Ranked weak spots (corners, slender struts, hinges, bridges). */
 weakSpots: LatticeWeakSpot[]
}

export type LatticeWeakKind =
 | 'slender_strut'
 | 'free_end'
 | 'hinge_node'
 | 'underconnected'
 | 'body_corner'
 | 'sharp_corner'
 | 'bridge'
 | 'long_span'
 | 'thin_wall'

export type LatticeWeakSeverity = 'low' | 'medium' | 'high' | 'critical'

export interface LatticeWeakSpot {
 kind: LatticeWeakKind
 severity: LatticeWeakSeverity
 /** 0..1 score used for ranking */
 score: number
 /** Approximate location in model millimetres */
 at: [number, number, number]
 ru: string
 en: string
 detail?: Record<string, number>
}

export const LATTICE_MATERIALS: LatticeMaterial[] = [
 {id: 'pla', ru: 'PLA (FDM)', en: 'PLA (FDM)', E: 3500, sigmaAllow: 35, density: 1.24},
 {id: 'petg', ru: 'PETG (FDM)', en: 'PETG (FDM)', E: 2100, sigmaAllow: 30, density: 1.27},
 {id: 'abs', ru: 'ABS (FDM)', en: 'ABS (FDM)', E: 2000, sigmaAllow: 25, density: 1.04},
 {id: 'nylon', ru: 'Nylon (FDM)', en: 'Nylon (FDM)', E: 1700, sigmaAllow: 40, density: 1.14},
 {id: 'al6061', ru: 'Алюминий 6061', en: 'Aluminium 6061', E: 68900, sigmaAllow: 110, density: 2.7},
 {id: 'steel', ru: 'Сталь конструкционная', en: 'Mild steel', E: 200000, sigmaAllow: 160, density: 7.85},
]

const round = (v: number, digits = 3) => Math.round(v * 10 ** digits) / 10 ** digits

/** Gibson-Ashby power for relative modulus vs relative density. Stretch-dominated ~1, bending ~2. */
export function ashbyExponent(pattern: LighteningPattern, diagonals = false): number {
 if (pattern === 'octet' || pattern === 'isogrid' || pattern === 'triangles') return 1
 if (pattern === 'bcc') return 1.4
 if (pattern === 'spatial') return diagonals ? 1.2 : 1.8
 if (pattern === 'honeycomb') return 1.5
 if (pattern === 'bone') return 1.7
 const load = latticeStructureHint(pattern).load
 if (load === 'stretch') return 1.1
 if (load === 'bending') return 2
 return 1.6
}

function bodyBounds(body: DirectBody) {
 const min = [Infinity, Infinity, Infinity], max = [-Infinity, -Infinity, -Infinity]
 for (let i = 0; i < body.mesh.positions.length; i++) {
  const k = i % 3
  min[k] = Math.min(min[k], body.mesh.positions[i])
  max[k] = Math.max(max[k], body.mesh.positions[i])
 }
 return {min, max, size: max.map((v, i) => v - min[i])}
}

/** Geometric relative-density estimate before meshing. */
export function estimateRelativeDensity(body: DirectBody, o: LighteningOptions): number {
 const {size} = bodyBounds(body)
 if (isSpatialPattern(o.pattern)) {
  const radius = o.rib / 2, cell = Math.max(o.cell, o.rib * 2)
  const strutsPerCell =
   o.pattern === 'octet' ? 12 : o.pattern === 'bcc' ? 8 : o.pattern === 'bone' ? 7 : o.diagonals ? 4 : 3
  const length = cell * Math.sqrt(o.pattern === 'bcc' ? 3 : o.pattern === 'octet' ? 2 : 1)
  let rho = Math.min(0.95, (strutsPerCell * Math.PI * radius * radius * length) / cell ** 3)
  if ((o.wallDepth ?? 0) > 0) {
   const [sx, sy, sz] = size, vol = sx * sy * sz, band = o.wallDepth!
   const shell = Math.max(0, vol - Math.max(0, sx - 2 * band) * Math.max(0, sy - 2 * band) * Math.max(0, sz - 2 * band))
   rho = o.keepCore
    ? Math.min(0.98, (rho * shell) / vol + (vol - shell) / vol)
    : Math.min(0.95, Math.max((rho * shell) / vol, 0.02))
  }
  if ((o.skin ?? 0) > 0) {
   const skin = o.skin!, [sx, sy, sz] = size
   rho = Math.min(0.98, rho + (2 * (sx * sy + sx * sz + sy * sz) * skin) / (sx * sy * sz))
  }
  return round(Math.max(0.02, rho), 4)
 }
 const cell = Math.max(o.cell, o.rib * 2)
 const open =
  o.pattern === 'isogrid' || o.pattern === 'triangles' ? 0.72
  : o.pattern === 'honeycomb' ? 0.78
  : o.pattern === 'web' ? 0.7
  : 0.65
 const axis = ['x', 'y', 'z'].indexOf(o.axis), u = (axis + 1) % 3, v = (axis + 2) % 3
 const spanU = Math.max(1e-6, size[u] - 2 * o.rim), spanV = Math.max(1e-6, size[v] - 2 * o.rim)
 const solidChannel = 1 - open * (1 - o.rib / cell) * (1 - o.rib / cell)
 const skins = (o.bottom + o.top) / Math.max(size[axis], 1e-6)
 const rimFrac = 1 - (spanU * spanV) / (size[u] * size[v])
 return round(Math.min(0.98, Math.max(0.05, solidChannel * (1 - rimFrac) * (1 - skins) + skins + rimFrac * 0.5)), 4)
}

function graphMetrics(body: DirectBody, o: LighteningOptions) {
 if (!isSpatialPattern(o.pattern)) {
  return {connectivity: null as number | null, meanLength: null as number | null, edgeCount: 0, nodes: [] as number[][], edges: [] as number[][], degree: [] as number[]}
 }
 const g = spatialGraph(body, o), degree = new Array(g.nodes.length).fill(0)
 let lengthSum = 0
 for (const [a, b] of g.edges) {
  degree[a]++; degree[b]++
  const pa = g.nodes[a], pb = g.nodes[b]
  lengthSum += Math.hypot(pa[0] - pb[0], pa[1] - pb[1], pa[2] - pb[2])
 }
 return {
  connectivity: g.nodes.length ? round(degree.reduce((s: number, d: number) => s + d, 0) / g.nodes.length, 3) : null,
  meanLength: g.edges.length ? round(lengthSum / g.edges.length, 3) : null,
  edgeCount: g.edges.length,
  nodes: g.nodes,
  edges: g.edges,
  degree,
 }
}

function severityFromScore(score: number): LatticeWeakSeverity {
 if (score >= 0.85) return 'critical'
 if (score >= 0.65) return 'high'
 if (score >= 0.4) return 'medium'
 return 'low'
}

function polygonAngles(poly: [number, number][]) {
 const angles: {at: [number, number]; deg: number}[] = []
 for (let i = 0; i < poly.length; i++) {
  const a = poly[(i + poly.length - 1) % poly.length], b = poly[i], c = poly[(i + 1) % poly.length]
  const ab = [a[0] - b[0], a[1] - b[1]], cb = [c[0] - b[0], c[1] - b[1]]
  const lab = Math.hypot(ab[0], ab[1]), lcb = Math.hypot(cb[0], cb[1])
  if (lab < 1e-9 || lcb < 1e-9) continue
  const cos = Math.max(-1, Math.min(1, (ab[0] * cb[0] + ab[1] * cb[1]) / (lab * lcb)))
  angles.push({at: [b[0], b[1]], deg: round((Math.acos(cos) * 180) / Math.PI, 1)})
 }
 return angles
}

function loadAxis(load: LatticeLoadInput, o: LighteningOptions): [number, number, number] {
 if (load.case === 'bending') return [1, 0, 0]
 if (o.axis === 'x') return [1, 0, 0]
 if (o.axis === 'y') return [0, 1, 0]
 return [0, 0, 1]
}

/**
 * Locate likely weak spots: slender/buckling struts, free ends, hinge nodes,
 * body corners, acute cell corners and long horizontal bridges.
 * Heuristic strength-of-materials screening — not FEA hotspots.
 */
export function findLatticeWeakSpots(
 body: DirectBody,
 o: LighteningOptions,
 material: LatticeMaterial,
 load: LatticeLoadInput,
 limit = 12,
): LatticeWeakSpot[] {
 const {min, max, size} = bodyBounds(body)
 const spots: LatticeWeakSpot[] = []
 const push = (spot: Omit<LatticeWeakSpot, 'severity'> & {score: number}) => {
  spots.push({...spot, severity: severityFromScore(spot.score)})
 }
 const axis = loadAxis(load, o)
 const r = o.rib / 2
 const gyration = r / 2 // radius of gyration for circular strut ≈ r/2

 if (isSpatialPattern(o.pattern)) {
  const metrics = graphMetrics(body, o)
  const {nodes, edges, degree} = metrics
  const I = Math.PI * r ** 4 / 4
  for (let i = 0; i < nodes.length; i++) {
   const d = degree[i], p = nodes[i] as [number, number, number]
   if (d <= 1) {
    push({
     kind: 'free_end', score: 0.92, at: p,
     ru: `Свободный конец стержня (Z=${d}) — механизм/консоль, высокий изгиб.`,
     en: `Free strut end (Z=${d}) — mechanism/cantilever with high bending.`,
     detail: {degree: d},
    })
   } else if (d === 2) {
    push({
     kind: 'hinge_node', score: 0.78, at: p,
     ru: `Шарнирный узел (Z=2) — изгибная «цепочка», слабая на поперечную нагрузку.`,
     en: `Hinge node (Z=2) — bending chain, weak under transverse load.`,
     detail: {degree: d},
    })
   } else if (d < 4) {
    push({
     kind: 'underconnected', score: 0.55, at: p,
     ru: `Слабо связанный узел (Z=${d}) — локально ближе к изгибному режиму.`,
     en: `Under-connected node (Z=${d}) — locally closer to bending-dominated behaviour.`,
     detail: {degree: d},
    })
   }
   // Body-corner proximity: within 15% of bounding box diagonal from a corner
   const diag = Math.hypot(size[0], size[1], size[2]) || 1
   let nearCorner = false, corner: [number, number, number] = [min[0], min[1], min[2]]
   for (const x of [min[0], max[0]]) for (const y of [min[1], max[1]]) for (const z of [min[2], max[2]]) {
    const dist = Math.hypot(p[0] - x, p[1] - y, p[2] - z)
    if (dist <= diag * 0.12) {nearCorner = true; corner = [x, y, z]}
   }
   if (nearCorner && d < 6) {
    push({
     kind: 'body_corner', score: round(0.5 + (6 - d) * 0.08, 3), at: p,
     ru: `Узел у угла тела (Z=${d}) — типичный концентратор при сжатии/изгибе корпуса.`,
     en: `Node at body corner (Z=${d}) — typical concentrator under shell compression/bending.`,
     detail: {degree: d, cx: corner[0], cy: corner[1], cz: corner[2]},
    })
   }
  }
  for (const [a, b] of edges) {
   const pa = nodes[a], pb = nodes[b]
   const dx = pb[0] - pa[0], dy = pb[1] - pa[1], dz = pb[2] - pa[2]
   const L = Math.hypot(dx, dy, dz)
   if (L < 1e-9) continue
   const mid: [number, number, number] = [(pa[0] + pb[0]) / 2, (pa[1] + pb[1]) / 2, (pa[2] + pb[2]) / 2]
   const slenderness = L / Math.max(gyration, 1e-6)
   const dir = [dx / L, dy / L, dz / L]
   const align = Math.abs(dir[0] * axis[0] + dir[1] * axis[1] + dir[2] * axis[2])
   const Pcr = (Math.PI ** 2 * material.E * I) / (L * L)
   const paths = parallelPaths(o, size, edges.length)
   const force = load.forceN / paths
   const buckling = force / Math.max(Pcr, 1e-9)
   if (slenderness > 40 || buckling > 0.5) {
    const score = round(Math.min(1, Math.max(slenderness / 80, buckling)), 3)
    push({
     kind: 'slender_strut', score, at: mid,
     ru: `Тонкий/длинный стержень λ≈${round(slenderness, 1)}, P/Pcr≈${round(buckling, 2)} — риск потери устойчивости.`,
     en: `Slender strut λ≈${round(slenderness, 1)}, P/Pcr≈${round(buckling, 2)} — buckling risk.`,
     detail: {length: round(L, 2), slenderness: round(slenderness, 1), buckling: round(buckling, 3)},
    })
   }
   // Horizontal bridge relative to print +Z
   const horizontal = 1 - Math.abs(dir[2])
   if (horizontal > 0.85 && L > o.rib * 4) {
    push({
     kind: 'bridge', score: round(Math.min(1, 0.45 + L / (o.cell * 4)), 3), at: mid,
     ru: `Горизонтальный мост L≈${round(L, 1)} mm — изгиб + риск провисания при печати.`,
     en: `Horizontal bridge L≈${round(L, 1)} mm — bending and print-sag risk.`,
     detail: {length: round(L, 2), horizontal: round(horizontal, 2)},
    })
   }
   if (L > (metrics.meanLength ?? o.cell) * 1.35 && align > 0.7) {
    push({
     kind: 'long_span', score: round(Math.min(1, 0.4 + align * 0.3), 3), at: mid,
     ru: `Длинный пролёт вдоль нагрузки L≈${round(L, 1)} mm — повышенные σ и прогиб.`,
     en: `Long span aligned with load L≈${round(L, 1)} mm — elevated stress and deflection.`,
     detail: {length: round(L, 2), align: round(align, 2)},
    })
   }
  }
  if ((o.wallDepth ?? 0) > 0 && (o.wallDepth ?? 0) < o.rib * 2) {
   const c: [number, number, number] = [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2]
   push({
    kind: 'thin_wall', score: 0.7, at: c,
    ru: `Скелетная стенка тоньше 2·диаметра стержня (${o.wallDepth} < ${round(o.rib * 2, 1)} mm) — слабые стыки слоёв.`,
    en: `Skeletal wall thinner than 2× strut diameter (${o.wallDepth} < ${round(o.rib * 2, 1)} mm) — weak layer junctions.`,
    detail: {wallDepth: o.wallDepth ?? 0, rib: o.rib},
   })
  }
 } else {
  // Channel / isogrid openings: acute corners and frame corners
  const axisIdx = o.axis === 'x' ? 0 : o.axis === 'y' ? 1 : 2
  const u = (axisIdx + 1) % 3, v = (axisIdx + 2) % 3
  const midAxis = (min[axisIdx] + max[axisIdx]) / 2
  try {
   const cells = lighteningCells([min[u] + o.rim, min[v] + o.rim], [max[u] - o.rim, max[v] - o.rim], o)
   for (const cell of cells) {
    for (const ang of polygonAngles(cell as [number, number][])) {
     if (ang.deg < 55) {
      const at: [number, number, number] = [0, 0, 0]
      at[u] = ang.at[0]; at[v] = ang.at[1]; at[axisIdx] = midAxis
      push({
       kind: 'sharp_corner', score: round(Math.min(1, (55 - ang.deg) / 55 + 0.45), 3), at,
       ru: `Острый угол ячейки ≈${ang.deg}° — концентратор напряжений в перемычке.`,
       en: `Acute cell corner ≈${ang.deg}° — stress concentrator in the rib.`,
       detail: {angleDeg: ang.deg},
      })
     }
    }
   }
  } catch {
   // Generation limits should not block the rest of screening.
  }
  // Bounding-frame corners of the channel lattice
  for (const uu of [min[u] + o.rim, max[u] - o.rim]) for (const vv of [min[v] + o.rim, max[v] - o.rim]) {
   const at: [number, number, number] = [0, 0, 0]
   at[u] = uu; at[v] = vv; at[axisIdx] = midAxis
   const score = o.pattern === 'grid' || o.pattern === 'web' ? 0.62 : 0.48
   push({
    kind: 'body_corner', score, at,
    ru: 'Угол рамки канала — типичное место излома при изгибе панели.',
    en: 'Channel frame corner — typical break location under panel bending.',
   })
  }
  const opening = o.cell - o.rib
  if (opening > o.rib * 5) {
   const at: [number, number, number] = [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2]
   push({
    kind: 'long_span', score: round(Math.min(1, opening / (o.rib * 8)), 3), at,
    ru: `Широкий проём ${round(opening, 1)} mm при перемычке ${o.rib} mm — изгиб перемычек.`,
    en: `Wide opening ${round(opening, 1)} mm with rib ${o.rib} mm — rib bending.`,
    detail: {opening: round(opening, 2), rib: o.rib},
   })
  }
  if (o.top > 0 && o.axis === 'z') {
   const at: [number, number, number] = [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, max[2] - o.top / 2]
   push({
    kind: 'bridge', score: 0.58, at,
    ru: 'Верхняя кожа над каналами — мост при печати и изгибная пластинка.',
    en: 'Top skin over channels — print bridge and bending plate.',
   })
  }
 }

 // Rank by score, but keep kind diversity so corners/hinges are not drowned by many similar struts.
 spots.sort((a, b) => b.score - a.score || a.kind.localeCompare(b.kind))
 const kept: LatticeWeakSpot[] = []
 const perKind = new Map<LatticeWeakKind, number>()
 for (const spot of spots) {
  const count = perKind.get(spot.kind) ?? 0
  if (count >= 3) continue
  if (kept.some(k => k.kind === spot.kind && Math.hypot(k.at[0] - spot.at[0], k.at[1] - spot.at[1], k.at[2] - spot.at[2]) < o.cell * 0.35)) continue
  kept.push(spot)
  perKind.set(spot.kind, count + 1)
  if (kept.length >= limit) break
 }
 // Second pass: fill remaining slots with next-best unused kinds / locations.
 if (kept.length < limit) {
  for (const spot of spots) {
   if (kept.includes(spot)) continue
   if (kept.some(k => Math.hypot(k.at[0] - spot.at[0], k.at[1] - spot.at[1], k.at[2] - spot.at[2]) < o.cell * 0.35)) continue
   kept.push(spot)
   if (kept.length >= limit) break
  }
 }
 return kept.sort((a, b) => b.score - a.score)
}

function parallelPaths(o: LighteningOptions, size: number[], edgeCount: number): number {
 const axis = o.axis === 'x' ? 0 : o.axis === 'y' ? 1 : 2
 const cell = Math.max(o.cell, o.rib * 2)
 const cross = Math.max(1, Math.ceil(size[(axis + 1) % 3] / cell)) * Math.max(1, Math.ceil(size[(axis + 2) % 3] / cell))
 if (o.pattern === 'octet') return Math.max(1, Math.round(cross * 2))
 if (o.pattern === 'bcc') return Math.max(1, Math.round(cross * 1.5))
 if (isSpatialPattern(o.pattern)) return Math.max(1, Math.round(Math.sqrt(Math.max(edgeCount, 1)) / 2))
 return Math.max(1, cross)
}

/**
 * Analytical strength-of-materials estimate for a lightened body.
 * Gibson-Ashby scaling and optional strut Euler checks — not FEA.
 */
export function analyzeLatticeStrength(
 body: DirectBody,
 o: LighteningOptions,
 material: LatticeMaterial,
 load: LatticeLoadInput,
 volumes?: {beforeMm3: number; afterMm3: number},
): LatticeStrengthReport {
 if (!(load.forceN > 0) || !(load.safety >= 1) || !Number.isFinite(load.forceN) || !Number.isFinite(load.safety)) {
  throw Error('Check force (>0 N) and safety factor (>=1).')
 }
 const hint = latticeStructureHint(o.pattern)
 const relativeDensity = volumes && volumes.beforeMm3 > 0
  ? round(Math.max(1e-4, volumes.afterMm3 / volumes.beforeMm3), 4)
  : estimateRelativeDensity(body, o)
 const volumeReductionPct = round((1 - relativeDensity) * 100, 1)
 const n = ashbyExponent(o.pattern, !!o.diagonals)
 const C_E = hint.load === 'stretch' ? 0.5 : hint.load === 'bending' ? 0.8 : 0.6
 const C_S = hint.load === 'stretch' ? 0.35 : hint.load === 'bending' ? 0.5 : 0.4
 const relativeStiffness = round(C_E * relativeDensity ** n, 4)
 const relativeStrength = round(C_S * relativeDensity ** (load.case === 'bending' ? n : Math.max(1, n * 0.85)), 4)
 const specificStiffness = round(relativeStiffness / Math.max(relativeDensity, 1e-6), 3)
 const {size} = bodyBounds(body)
 const metrics = graphMetrics(body, o)
 const warnings: {ru: string; en: string}[] = []
 let strutAreaMm2: number | null = null
 let strutStressMPa: number | null = null
 let eulerBucklingN: number | null = null
 let bucklingRatio: number | null = null
 let utilization: number | null = null

 if (isSpatialPattern(o.pattern)) {
  const r = o.rib / 2
  strutAreaMm2 = round(Math.PI * r * r, 4)
  const I = (Math.PI * r ** 4) / 4
  const L = Math.max(metrics.meanLength ?? o.cell, o.rib)
  const paths = parallelPaths(o, size, metrics.edgeCount)
  const forcePerStrut = load.forceN / paths
  strutStressMPa = round(forcePerStrut / strutAreaMm2, 4)
  // Euler pin-ended strut; E in MPa = N/mm^2, I in mm^4, L in mm -> N
  eulerBucklingN = round((Math.PI ** 2 * material.E * I) / (L * L), 4)
  bucklingRatio = round(forcePerStrut / Math.max(eulerBucklingN, 1e-9), 3)
  utilization = round(strutStressMPa / (material.sigmaAllow / load.safety), 3)
  if (metrics.connectivity != null && metrics.connectivity < 6 && o.pattern !== 'bone') {
   warnings.push({
    ru: `Средняя связность узла Z≈${metrics.connectivity} < 6 — каркас ближе к изгибному (Maxwell/Deshpande).`,
    en: `Mean node connectivity Z≈${metrics.connectivity} < 6 — closer to bending-dominated (Maxwell/Deshpande).`,
   })
  }
  if (bucklingRatio > 0.7) {
   warnings.push({
    ru: `Запас по Эйлеру низкий (P/Pcr≈${bucklingRatio}). Утолщите стержни или уменьшите ячейку.`,
    en: `Euler buckling margin is low (P/Pcr≈${bucklingRatio}). Increase strut diameter or reduce cell size.`,
   })
  }
  if (utilization > 1) {
   warnings.push({
    ru: `Расчётные напряжения в стержне выше допуска с запасом (η≈${utilization}).`,
    en: `Estimated strut stress exceeds allowable with safety factor (η≈${utilization}).`,
   })
  }
  if ((o.wallDepth ?? 0) > 0 && !o.keepCore) {
   warnings.push({
    ru: 'Пустая сердцевина: оценка — для мембранной/оболочечной работы стенки, не для объёмного сжатия целиком.',
    en: 'Hollow core: estimate targets wall membrane/shell action, not bulk solid compression.',
   })
  }
 } else {
  if (o.pattern === 'grid' || o.pattern === 'web') {
   warnings.push({
    ru: 'Канальная сетка изгибно-доминантна: при той же плотности жёсткость ниже, чем у изогрида/октета.',
    en: 'Channel grid is bending-dominated: at the same density, stiffness trails isogrid/octet.',
   })
  }
  if (o.top > 0) {
   warnings.push({
    ru: 'Верхняя кожа образует мосты при печати; локальная прочность кожи не моделируется.',
    en: 'Top skin introduces print bridges; local skin strength is not modelled.',
   })
  }
  const paths = parallelPaths(o, size, 0)
  const axisIndex = o.axis === 'x' ? 0 : o.axis === 'y' ? 1 : 2
  strutAreaMm2 = round(o.rib * Math.max(size[axisIndex] - o.bottom - o.top, o.rib), 3)
  strutStressMPa = round(load.forceN / (paths * Math.max(o.rib * o.rib, 1e-6)), 4)
  utilization = round(strutStressMPa / (material.sigmaAllow / load.safety), 3)
  if (utilization > 1) {
   warnings.push({
    ru: `Оценка напряжения в перемычках выше допуска (η≈${utilization}).`,
    en: `Estimated rib stress exceeds allowable (η≈${utilization}).`,
   })
  }
 }

 if (load.case === 'bending' && relativeDensity < 0.25) {
  warnings.push({
   ru: 'При изгибе тонкого облегчённого тела важны положение материала и моменты инерции — Ashby даёт только порядок величины.',
   en: 'For bending of a light body, material placement and section moments dominate — Ashby only gives order-of-magnitude.',
  })
 }
 if (material.id === 'pla' || material.id === 'petg' || material.id === 'abs' || material.id === 'nylon') {
  warnings.push({
   ru: 'Для FDM модуль и прочность анизотропны (слои); значения — изотропный справочный порядок.',
   en: 'FDM modulus/strength are anisotropic (layers); values are isotropic order-of-magnitude references.',
  })
 }

 const weakSpots = findLatticeWeakSpots(body, o, material, load)
 if (weakSpots[0] && weakSpots[0].severity === 'critical') {
  warnings.push({
   ru: `Найдены критические слабые места (топ: ${weakSpots[0].kind}) — см. список координат ниже.`,
   en: `Critical weak spots found (top: ${weakSpots[0].kind}) — see coordinate list below.`,
  })
 } else if (weakSpots.some(w => w.severity === 'high')) {
  warnings.push({
   ru: 'Обнаружены зоны повышенного риска (углы/тонкие стержни/мосты) — проверьте список слабых мест.',
   en: 'Elevated-risk zones detected (corners/slender struts/bridges) — review the weak-spot list.',
  })
 }

 return {
  pattern: o.pattern,
  loadClass: hint.load,
  relativeDensity,
  volumeReductionPct,
  ashbyExponent: n,
  relativeStiffness,
  relativeStrength,
  specificStiffness,
  meanStrutLengthMm: metrics.meanLength,
  strutAreaMm2,
  strutStressMPa,
  eulerBucklingN,
  bucklingRatio,
  utilization,
  connectivity: metrics.connectivity,
  warnings,
  weakSpots,
  disclaimer: {
   ru: 'Аналитическая оценка сопромата (Ashby/Эйлер) и эвристика слабых мест, не МКЭ и не сертификат. Не заменяет испытания и FEA.',
   en: 'Analytical strength-of-materials estimate (Ashby/Euler) plus weak-spot heuristics, not FEA or certification. Does not replace tests or FEA.',
  },
 }
}

export function formatLatticeStrengthReport(
 r: LatticeStrengthReport,
 material: LatticeMaterial,
 load: LatticeLoadInput,
 locale: 'ru' | 'en',
): string {
 const L = locale === 'ru'
 const caseLabel = L
  ? load.case === 'compression' ? 'сжатие' : load.case === 'tension' ? 'растяжение' : 'изгиб'
  : load.case
 const lines = [
  L
   ? `Сопромат (оценка): ${material.ru}, ${caseLabel} ${load.forceN} N, n=${load.safety}`
   : `Strength estimate: ${material.en}, ${caseLabel} ${load.forceN} N, n=${load.safety}`,
  L
   ? `Плотность ρ*/ρ ≈ ${r.relativeDensity} (−${r.volumeReductionPct}% объёма), класс: ${r.loadClass}, показатель Ashby n=${r.ashbyExponent}`
   : `Relative density ρ*/ρ ≈ ${r.relativeDensity} (−${r.volumeReductionPct}% volume), class: ${r.loadClass}, Ashby n=${r.ashbyExponent}`,
  L
   ? `E*/E ≈ ${r.relativeStiffness}, σ*/σ ≈ ${r.relativeStrength}, удельная жёсткость (E*/E)/(ρ*/ρ) ≈ ${r.specificStiffness}`
   : `E*/E ≈ ${r.relativeStiffness}, σ*/σ ≈ ${r.relativeStrength}, specific stiffness (E*/E)/(ρ*/ρ) ≈ ${r.specificStiffness}`,
 ]
 if (r.connectivity != null) {
  lines.push(L
   ? `Связность Z≈${r.connectivity}, длина стержня ≈ ${r.meanStrutLengthMm} mm`
   : `Connectivity Z≈${r.connectivity}, strut length ≈ ${r.meanStrutLengthMm} mm`)
 }
 if (r.strutStressMPa != null) {
  lines.push(L
   ? `σ_стержня ≈ ${r.strutStressMPa} MPa, η ≈ ${r.utilization}${r.bucklingRatio != null ? `, P/Pcr ≈ ${r.bucklingRatio}` : ''}`
   : `σ_strut ≈ ${r.strutStressMPa} MPa, η ≈ ${r.utilization}${r.bucklingRatio != null ? `, P/Pcr ≈ ${r.bucklingRatio}` : ''}`)
 }
 for (const w of r.warnings) lines.push('⚠ ' + (L ? w.ru : w.en))
 if (r.weakSpots.length) {
  lines.push(L ? 'Слабые места (эвристика):' : 'Weak spots (heuristic):')
  for (const [i, spot] of r.weakSpots.slice(0, 8).entries()) {
   const xyz = `${round(spot.at[0], 1)}, ${round(spot.at[1], 1)}, ${round(spot.at[2], 1)}`
   lines.push(`${i + 1}. [${spot.severity}] (${xyz}) ${L ? spot.ru : spot.en}`)
  }
 }
 lines.push(L ? r.disclaimer.ru : r.disclaimer.en)
 return lines.join('\n')
}

export function volumeFromBodies(bodies: DirectBody[], ids: string[]) {
 return bodies.filter(b => ids.includes(b.id)).reduce((s, b) => s + inspectPolygonMesh(b.mesh).signedVolumeMm3, 0)
}
