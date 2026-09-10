import {inspectPolygonMesh} from './geometry/polygon'
import {
 isSpatialPattern,
 latticeStructureHint,
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
  return {connectivity: null as number | null, meanLength: null as number | null, edgeCount: 0}
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
 }
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
  disclaimer: {
   ru: 'Аналитическая оценка сопромата (Ashby/Эйлер), не МКЭ и не сертификат. Не заменяет испытания и FEA.',
   en: 'Analytical strength-of-materials estimate (Ashby/Euler), not FEA or certification. Does not replace tests or FEA.',
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
 lines.push(L ? r.disclaimer.ru : r.disclaimer.en)
 return lines.join('\n')
}

export function volumeFromBodies(bodies: DirectBody[], ids: string[]) {
 return bodies.filter(b => ids.includes(b.id)).reduce((s, b) => s + inspectPolygonMesh(b.mesh).signedVolumeMm3, 0)
}
