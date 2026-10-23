import {callGeometryRust} from './geometry/kernel'
import type {LighteningOptions} from './solidLightening'
import type {LatticePrintSettings} from './latticePrintSettings'

export type Bilingual = {ru: string; en: string}
/** Optional FDM screening extras on top of the fitting settings. */
export interface LatticePrintScreenSettings extends LatticePrintSettings {
  maxOverhangDeg?: number
  minFeatureMm?: number
  printSpeedMms?: number
  fanPct?: number
}
export type PrintIssueKind = 'bridge' | 'overhang' | 'thin_wall' | 'tiny_feature' | 'cooling' | 'orientation' | 'skin' | 'layer'
export interface PrintIssue extends Bilingual {kind: PrintIssueKind; severity: 'info' | 'warn' | 'critical'}
export interface OrientationScore {axis: 'x' | 'y' | 'z'; score: number; bridgeMm: number; overhangRisk: number; notes: Bilingual}
export interface LatticePrintReport {
  lineWidthMm: number
  minWallMm: number
  bridgeSpanMm: number
  overhangRisk: number
  preferredAxis: 'x' | 'y' | 'z'
  orientations: OrientationScore[]
  issues: PrintIssue[]
  tips: Bilingual[]
  /** Machine-agnostic slicer hints (PrusaSlicer / Orca / Cura-style keys), not a vendor profile. */
  slicerHints: Record<string, string | number | boolean>
}

/** Native FDM printability screen: orientation ranking, issues, tips and slicer hints. */
export function analyzeLatticePrintability(options: LighteningOptions, settings: LatticePrintScreenSettings): LatticePrintReport {
  return callGeometryRust('cad_lattice_print_report', {options, settings})
}

/** Native reorient + fit + bridge-bounding optimizer; returns the new options and their report. */
export function optimizeLatticeForPrint(options: LighteningOptions, settings: LatticePrintScreenSettings,
  opts: {reorient?: boolean; shrinkBridges?: boolean} = {}): {options: LighteningOptions; report: LatticePrintReport; changed: string[]} {
  return callGeometryRust('cad_lattice_print_optimize', {options, settings, ...opts})
}

export function formatLatticePrintReport(r: LatticePrintReport, locale: string): string {
  const ru = locale === 'ru', t = (b: Bilingual) => ru ? b.ru : b.en
  const lines = [ru
    ? `Печать FDM: линия ${r.lineWidthMm} mm, мин. стенка ${r.minWallMm} mm, мост ≈${r.bridgeSpanMm} mm, свесы ${Math.round(r.overhangRisk * 100)}%, ось ${r.preferredAxis.toUpperCase()}`
    : `FDM print: line ${r.lineWidthMm} mm, min wall ${r.minWallMm} mm, bridge ≈${r.bridgeSpanMm} mm, overhangs ${Math.round(r.overhangRisk * 100)}%, axis ${r.preferredAxis.toUpperCase()}`]
  for (const issue of r.issues) lines.push(`${issue.severity === 'critical' ? '✖' : issue.severity === 'warn' ? '⚠' : '•'} ${t(issue)}`)
  if (r.tips.length) lines.push(ru ? 'Советы:' : 'Tips:', ...r.tips.slice(0, 6).map(tip => `• ${t(tip)}`))
  lines.push(`${ru ? 'Ориентации' : 'Orientations'}: ${r.orientations.map(o => `${o.axis.toUpperCase()}(${o.score})`).join(' › ')}`)
  return lines.join('\n')
}

/** JSON blob of slicer hints for copy/download (not a vendor profile). */
export function latticeSlicerHintsJson(r: LatticePrintReport): string {
  return JSON.stringify({generator: 'open-scad-viewer/lattice-print', hints: r.slicerHints}, null, 2)
}
