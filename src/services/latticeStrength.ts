import {callGeometryRust} from './geometry/kernel'
import type {MeshData, SceneEntityId} from '../core/mesh'
import type {LighteningOptions, LighteningPattern} from './solidLightening'
import type {LatticeGraphMesh} from './latticeGraphProtocol'
import {flattenGroupGeometry} from './meshFlatten'
import {importedStlToMeshData} from './stlImport'
import type {Bilingual} from './latticePrintReport'

export type LatticeMaterialId = 'pla' | 'petg' | 'abs' | 'nylon' | 'al6061' | 'steel'
export type LatticeLoadCase = 'compression' | 'tension' | 'bending'
export type LatticeLoadClass = 'stretch' | 'mixed' | 'bending' | 'organic'
/** Nominal preset or caller-supplied properties (MPa, g/cm³); presets are typical values, not datasheets. */
export interface LatticeMaterial extends Bilingual {
  id: string; E: number; sigmaAllow: number; density: number; process: 'fdm' | 'metal'
  notch: number; layer: number; ductility: number; EParallel: number; EPerp: number
  sigmaInterlayer: number; sigmaYield: number; fatigueRatio: number; Kt: number
}
export interface LatticeLoadInput {case: LatticeLoadCase; forceN: number; safety: number; direction?: [number, number, number]}
export interface LatticeServiceConditions {
  printAxis?: 'x' | 'y' | 'z'; temperatureC?: number; humidityPct?: number; infill?: number
  perimeters?: number; minWallMm?: number; maxBridgeMm?: number
}
export interface StrutSection {kind: 'circle' | 'square' | 'rectangle' | 'tube'; a?: number; b?: number; eccentricity?: number}
export type LatticeWeakKind = 'slender_strut' | 'free_end' | 'hinge_node' | 'underconnected' | 'body_corner'
  | 'sharp_corner' | 'bridge' | 'long_span' | 'thin_wall'
export type LatticeWeakSeverity = 'low' | 'medium' | 'high' | 'critical'
export interface LatticeWeakSpot extends Bilingual {
  kind: LatticeWeakKind; severity: LatticeWeakSeverity; score: number
  at: [number, number, number]; color: [number, number, number, number]; materialBias: number
  detail: Record<string, number>
}
export interface LatticeStrengthReport {
  pattern: LighteningPattern; loadClass: LatticeLoadClass; loadCase: LatticeLoadCase; forceN: number; safety: number
  relativeDensity: number; volumeReductionPct: number; ashbyExponent: number
  relativeStiffness: number; relativeStrength: number; specificStiffness: number
  meanStrutLengthMm: number | null; connectivity: number | null; strutAreaMm2: number | null
  strutStressMPa: number | null; eulerBucklingN: number | null; bucklingRatio: number | null
  utilization: number; panelBucklingMPa: number | null; sectionLabel: string
  material: {id: string} & Bilingual
  materialEffective: {E: number; EParallel: number; EPerp: number; sigmaAllow: number; sigmaInterlayer: number; sigmaYield: number; envFactor: number; layerAlignment: number}
  homogenized: {EStar: number; sigmaStar: number; relativeDensity: number}
  advice: (Bilingual & {deltaStiffnessPct: number; deltaMassPct: number})[]
  warnings: Bilingual[]
  weakSpots: LatticeWeakSpot[]
  disclaimer: Bilingual
}

export function latticeMaterials(): LatticeMaterial[] {return callGeometryRust('cad_lattice_materials', {})}
export function latticeStructureHint(pattern: LighteningPattern): Bilingual & {rank: number; load: LatticeLoadClass} {
  return callGeometryRust('cad_lattice_structure_hint', {pattern})
}

/** Native analytical estimate (Gibson–Ashby, Euler, FDM orthotropy) with ranked weak spots. */
export function analyzeLatticeStrength(mesh: LatticeGraphMesh, options: LighteningOptions, material: LatticeMaterialId | string | LatticeMaterial,
  load: LatticeLoadInput, extra: {volumes?: {beforeMm3: number; afterMm3: number}; service?: LatticeServiceConditions; section?: StrutSection} = {}): LatticeStrengthReport {
  return callGeometryRust('cad_lattice_strength', {mesh: flattenGroupGeometry([mesh]), options, material, load, ...extra})
}

/** Score-scaled octahedron viewport markers, tinted by severity and material. */
export function weakSpotMarkerMeshes(spots: readonly LatticeWeakSpot[], markerMm = 1.6): MeshData[] {
  const soups = callGeometryRust<number[][]>('cad_lattice_weak_spot_markers', {spots: spots.map(s => ({at: s.at, score: s.score})), markerMm})
  return soups.map((soup, i) => {
    const positions = new Float32Array(soup)
    const mesh = importedStlToMeshData({positions, triangleCount: positions.length / 9}, spots[i].color)
    return {...mesh, entityId: `entity:weak-spot/${spots[i].kind}/${i}` as SceneEntityId}
  })
}

export function formatLatticeStrengthReport(r: LatticeStrengthReport, locale: string): string {
  const ru = locale === 'ru', t = (b: Bilingual) => ru ? b.ru : b.en
  const caseLabel = ru ? {compression: 'сжатие', tension: 'растяжение', bending: 'изгиб'}[r.loadCase] : r.loadCase
  const e = r.materialEffective
  const lines = [
    `${ru ? 'Сопромат (оценка)' : 'Strength estimate'}: ${t(r.material)}, ${caseLabel} ${r.forceN} N, n=${r.safety}`,
    ru ? `Плотность ρ*/ρ ≈ ${r.relativeDensity} (−${r.volumeReductionPct}% объёма), класс: ${r.loadClass}, Ashby n=${r.ashbyExponent}`
      : `Relative density ρ*/ρ ≈ ${r.relativeDensity} (−${r.volumeReductionPct}% volume), class: ${r.loadClass}, Ashby n=${r.ashbyExponent}`,
    `E*/E ≈ ${r.relativeStiffness}, σ*/σ ≈ ${r.relativeStrength}, ${ru ? 'удельная жёсткость' : 'specific stiffness'} ≈ ${r.specificStiffness}`,
    `${ru ? 'Материал эфф.' : 'Effective material'}: E=${e.E} MPa (∥${e.EParallel}/⊥${e.EPerp}), [σ]=${e.sigmaAllow}, σ_inter=${e.sigmaInterlayer}, env×${e.envFactor}, align=${e.layerAlignment}`,
    `${ru ? 'Сечение стержня' : 'Strut section'}: ${r.sectionLabel}; E*≈${r.homogenized.EStar} MPa, σ*≈${r.homogenized.sigmaStar} MPa`,
  ]
  if (r.connectivity != null) lines.push(`${ru ? 'Связность' : 'Connectivity'} Z≈${r.connectivity}, L≈${r.meanStrutLengthMm} mm`)
  if (r.strutStressMPa != null) lines.push(`σ ≈ ${r.strutStressMPa} MPa, η ≈ ${r.utilization}${r.bucklingRatio != null ? `, P/Pcr ≈ ${r.bucklingRatio}` : ''}${r.panelBucklingMPa != null ? `, σ_panel≈${r.panelBucklingMPa}` : ''}`)
  for (const w of r.warnings) lines.push('⚠ ' + t(w))
  if (r.advice.length) lines.push(ru ? 'Рекомендации:' : 'Recommendations:', ...r.advice.slice(0, 5).map(a =>
    `• ${t(a)} (ΔE*≈${a.deltaStiffnessPct > 0 ? '+' : ''}${a.deltaStiffnessPct}%, Δm≈${a.deltaMassPct > 0 ? '+' : ''}${a.deltaMassPct}%)`))
  if (r.weakSpots.length) {
    lines.push(ru ? 'Слабые места (эвристика, подсветка в превью):' : 'Weak spots (heuristic, highlighted in preview):')
    r.weakSpots.slice(0, 8).forEach((s, i) => lines.push(`${i + 1}. [${s.severity} ×${s.materialBias}] (${s.at.map(v => Math.round(v * 10) / 10).join(', ')}) ${t(s)}`))
  }
  lines.push(t(r.disclaimer))
  return lines.join('\n')
}
