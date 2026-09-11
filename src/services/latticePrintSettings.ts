import {isSpatialPattern, type LighteningOptions, type LighteningPattern} from './solidLightening'

export interface LatticePrintSettings {
 nozzle: number
 layer: number
 lines: number
 skinLayers: number
 maxBridge: number
 openTop: boolean
 /** Max self-supporting overhang from horizontal, degrees (typical FDM 45–60). */
 maxOverhangDeg?: number
 /** Minimum printable feature / hole, mm (defaults to nozzle). */
 minFeatureMm?: number
 /** Perimeter speed mm/s — thin-lattice cooling heuristic. */
 printSpeedMms?: number
 /** Part cooling fan %, 0–100. */
 fanPct?: number
}

export type PrintIssueKind =
 | 'bridge'
 | 'overhang'
 | 'thin_wall'
 | 'tiny_feature'
 | 'cooling'
 | 'orientation'
 | 'skin'
 | 'layer'

export interface PrintIssue {
 kind: PrintIssueKind
 severity: 'info' | 'warn' | 'critical'
 ru: string
 en: string
}

export interface OrientationScore {
 axis: 'x' | 'y' | 'z'
 score: number
 bridgeMm: number
 overhangRisk: number
 notes: {ru: string; en: string}
}

export interface LatticePrintReport {
 lineWidthMm: number
 minWallMm: number
 bridgeSpanMm: number
 overhangRisk: number
 preferredAxis: 'x' | 'y' | 'z'
 orientations: OrientationScore[]
 issues: PrintIssue[]
 tips: {ru: string; en: string}[]
 /** Machine-agnostic slicer hints (PrusaSlicer / Cura / Orca-style keys). */
 slicerHints: Record<string, string | number | boolean>
}

const round = (v: number, d = 3) => Math.round(v * 10 ** d) / 10 ** d

function lineWidthOf(p: LatticePrintSettings) {
 return Math.round(p.nozzle * 1.125 * 1e6) / 1e6
}

function validatePrintSettings(p: LatticePrintSettings) {
 if (
  ![p.nozzle, p.layer, p.lines, p.skinLayers, p.maxBridge].every(Number.isFinite) ||
  p.nozzle <= 0 || p.layer <= 0 || p.layer > p.nozzle ||
  !Number.isInteger(p.lines) || p.lines < 1 || p.lines > 8 ||
  !Number.isInteger(p.skinLayers) || p.skinLayers < 0 || p.maxBridge <= 0
 ) throw Error('Проверьте сопло, высоту слоя, число линий и длину моста.')
}

/** Round lattice ribs/skins/floors to extrusion-line and layer multiples for FDM. */
export function fitLatticeToPrint(o: LighteningOptions, p: LatticePrintSettings): LighteningOptions {
 validatePrintSettings(p)
 const snap = (v: number, s: number) => Math.round(Math.ceil((v - 1e-8) / s) * s * 1e6) / 1e6
 const width = lineWidthOf(p)
 const rib = snap(Math.max(o.rib, width * p.lines), width)
 const spatial = isSpatialPattern(o.pattern)
 const result: LighteningOptions = {
  ...o, lineWidth: width, perimeters: p.lines, rib, cell: Math.max(o.cell, rib * 2 + width),
 }
 if (spatial) {
  result.skin = o.skin ? snap(Math.max(o.skin, width * p.lines), width) : 0
  result.openTop = p.openTop
  result.step = Math.min(
   o.step ?? rib / 3, rib / 3,
   result.skin ? result.skin / 2 : Infinity,
   o.wallDepth ? o.wallDepth / 2 : Infinity,
  )
 } else {
  result.axis = 'z'
  result.bottom = snap(Math.max(o.bottom, p.skinLayers * p.layer), p.layer)
  result.top = p.openTop ? 0 : snap(Math.max(o.top, p.skinLayers * p.layer), p.layer)
  result.rim = snap(Math.max(o.rim, width * p.lines), width)
 }
 return result
}

export function printBridgeWarning(o: LighteningOptions, p: LatticePrintSettings): boolean {
 return o.cell - o.rib > p.maxBridge
}

/** Clear span that must bridge for the lattice cell. */
export function bridgeSpanMm(o: LighteningOptions): number {
 return Math.max(0, o.cell - o.rib)
}

/**
 * Heuristic overhang risk 0..1 for lattice vs printer Z (after aligning `channelAxis` to Z).
 * Channel walls // Z → low; roofs / flat skins → high; ~45° spatial diagonals → medium.
 */
export function overhangRisk(
 o: LighteningOptions,
 channelAxis: 'x' | 'y' | 'z' = o.axis,
 maxOverhangDeg = 50,
): number {
 const limit = Math.max(20, Math.min(80, maxOverhangDeg))
 if (isSpatialPattern(o.pattern)) {
  const typical = o.pattern === 'bcc' ? 55 : o.pattern === 'octet' ? 45 : o.diagonals ? 45 : 0
  const excess = Math.max(0, typical - (90 - limit))
  let risk = Math.min(1, excess / 45)
  if ((o.skin ?? 0) > 0 && !o.openTop) risk = Math.max(risk, 0.85)
  return round(risk, 3)
 }
 if (channelAxis === 'z') {
  let risk = 0.05
  if (o.top > 0) risk = Math.max(risk, 0.9)
  if (o.bottom <= 0) risk = Math.max(risk, 0.2)
  return risk
 }
 // Horizontal tunnels relative to build Z — supports / reorient needed.
 return 0.95
}

function orientationNote(pattern: LighteningPattern, axis: 'x' | 'y' | 'z'): {ru: string; en: string} {
 if (isSpatialPattern(pattern)) {
  return {
   ru: `Печать по ${axis.toUpperCase()}: диагонали октет/ОЦК ~45–55° — умеренный риск свесов; кожу режьте openTop.`,
   en: `Print along ${axis.toUpperCase()}: octet/BCC diagonals ~45–55° — moderate overhang risk; prefer openTop.`,
  }
 }
 return {
  ru: axis === 'z'
   ? 'Каналы вдоль Z — стенки вертикальны, лучше для FDM без поддержек.'
   : `Каналы вдоль ${axis.toUpperCase()} при печати Z дают горизонтальные туннели — нужны поддержки или поворот.`,
  en: axis === 'z'
   ? 'Channels along Z keep walls vertical — best for support-light FDM.'
   : `Channels along ${axis.toUpperCase()} while printing Z make horizontal tunnels — need supports or reorient.`,
 }
}

/** Rank print axes for support-light FDM (higher score is better). */
export function suggestPrintOrientation(
 o: LighteningOptions,
 p: LatticePrintSettings,
): OrientationScore[] {
 validatePrintSettings(p)
 const maxOh = p.maxOverhangDeg ?? 50
 const bridge = bridgeSpanMm(o)
 const axes: Array<'x' | 'y' | 'z'> = ['z', 'y', 'x']
 const ranked = axes.map(axis => {
  const oh = overhangRisk(o, axis, maxOh)
  let score = 100 - bridge * 3 - oh * 40
  if (!isSpatialPattern(o.pattern)) {
   if (axis === 'z') score += 30 // vertical channel walls for FDM
   if (o.axis === axis) score += 5
  } else if (axis === 'z') {
   score += 5
  }
  if ((o.skin ?? 0) > 0 && !o.openTop && axis === 'z') score -= 15
  if (o.top > 0 && !o.openTop && axis === 'z' && !isSpatialPattern(o.pattern)) score -= 10
  return {
   axis,
   score: round(score, 1),
   bridgeMm: round(bridge, 2),
   overhangRisk: oh,
   notes: orientationNote(o.pattern, axis),
  }
 })
 return ranked.sort((a, b) => b.score - a.score)
}

/** Full FDM printability screen for a lattice option set. */
export function analyzeLatticePrintability(
 o: LighteningOptions,
 p: LatticePrintSettings,
): LatticePrintReport {
 validatePrintSettings(p)
 const width = lineWidthOf(p)
 const minWall = width * p.lines
 const minFeat = p.minFeatureMm ?? p.nozzle
 const maxOh = p.maxOverhangDeg ?? 50
 const speed = p.printSpeedMms ?? 45
 const fan = p.fanPct ?? 100
 const orientations = suggestPrintOrientation(o, p)
 const preferredAxis = orientations[0]?.axis ?? 'z'
 const bridge = bridgeSpanMm(o)
 const oh = overhangRisk(o, preferredAxis, maxOh)
 const issues: PrintIssue[] = []
 const tips: {ru: string; en: string}[] = []

 if (p.layer > p.nozzle * 0.8) {
  issues.push({
   kind: 'layer', severity: 'warn',
   ru: `Слой ${p.layer} mm высок для сопла ${p.nozzle} mm — хуже свесы и щели в решётке.`,
   en: `Layer ${p.layer} mm is high for a ${p.nozzle} mm nozzle — worse overhangs and lattice gaps.`,
  })
 }
 if (o.rib + 1e-9 < minWall) {
  issues.push({
   kind: 'thin_wall', severity: 'critical',
   ru: `Ребро ${o.rib} mm < ${p.lines}×линии (${round(minWall, 2)} mm).`,
   en: `Rib ${o.rib} mm < ${p.lines}×line width (${round(minWall, 2)} mm).`,
  })
 }
 if (o.rib < minFeat * 1.5) {
  issues.push({
   kind: 'tiny_feature', severity: 'warn',
   ru: `Тонкий элемент ${o.rib} mm близок к минимуму сопла (~${minFeat} mm).`,
   en: `Feature ${o.rib} mm is near the nozzle minimum (~${minFeat} mm).`,
  })
 }
 if (bridge > p.maxBridge) {
  issues.push({
   kind: 'bridge', severity: bridge > p.maxBridge * 1.5 ? 'critical' : 'warn',
   ru: `Мост ≈${round(bridge, 1)} mm > лимита ${p.maxBridge} mm.`,
   en: `Bridge ≈${round(bridge, 1)} mm > limit ${p.maxBridge} mm.`,
  })
  tips.push({
   ru: 'Уменьшите ячейку, увеличьте ребро, откройте верх или поверните каналы вдоль Z.',
   en: 'Reduce cell, thicken rib, open the top, or align channels with Z.',
  })
 }
 if (oh >= 0.6) {
  issues.push({
   kind: 'overhang', severity: oh >= 0.85 ? 'critical' : 'warn',
   ru: `Риск свесов высокий (≈${Math.round(oh * 100)}% при лимите ${maxOh}°).`,
   en: `Overhang risk high (≈${Math.round(oh * 100)}% at ${maxOh}° limit).`,
  })
  tips.push({
   ru: 'Выберите ориентацию с вертикальными стенками; для октета держите openTop и тонкую/нулевую кожу.',
   en: 'Pick an orientation with vertical walls; for octet keep openTop and thin/zero skin.',
  })
 }

 const loopProxy = Math.max(o.cell * 4, o.rib * 8)
 const layerTimeProxy = loopProxy / Math.max(speed, 1)
 if (layerTimeProxy < 4 && o.cell < 10) {
  issues.push({
   kind: 'cooling', severity: 'warn',
   ru: `Мелкая ячейка: оценка времени слоя ~${round(layerTimeProxy, 1)} s при ${speed} mm/s — риск перегрева.`,
   en: `Fine cell: ~${round(layerTimeProxy, 1)} s layer-time proxy at ${speed} mm/s — heat-soak risk.`,
  })
  tips.push({
   ru: fan < 80
    ? 'Включите обдув ≥80% или печатайте пачками / снизьте скорость периметров.'
    : 'Снизьте скорость периметров или увеличьте ячейку, чтобы слой успевал остыть.',
   en: fan < 80
    ? 'Enable fan ≥80% or print in batches / slow perimeters.'
    : 'Slow perimeters or enlarge the cell so layers can cool.',
  })
 }
 if (!isSpatialPattern(o.pattern) && o.axis !== 'z') {
  issues.push({
   kind: 'orientation', severity: 'warn',
   ru: `Каналы по ${o.axis.toUpperCase()} при печати Z — горизонтальные туннели.`,
   en: `Channels along ${o.axis.toUpperCase()} while printing Z make horizontal tunnels.`,
  })
 }
 if ((o.skin ?? 0) > 0 && !o.openTop && isSpatialPattern(o.pattern)) {
  issues.push({
   kind: 'skin', severity: 'info',
   ru: 'Сплошная верхняя кожа на скелете — мосты между стержнями.',
   en: 'Closed top skin on a skeletal lattice bridges between struts.',
  })
 }

 tips.push({
  ru: `Предпочтительная ось печати: ${preferredAxis.toUpperCase()} (оценка ${orientations[0]?.score}).`,
  en: `Preferred print axis: ${preferredAxis.toUpperCase()} (score ${orientations[0]?.score}).`,
 })
 tips.push({
  ru: 'Швы: на скрытой грани; для решёток удобен случайный/aligned шов у стойки.',
  en: 'Seams: hide on a back face; for lattices prefer random/aligned seams at a post.',
 })

 const slicerHints: Record<string, string | number | boolean> = {
  layer_height: p.layer,
  line_width: width,
  wall_loops: p.lines,
  top_shell_layers: p.openTop ? 0 : p.skinLayers,
  bottom_shell_layers: p.skinLayers,
  bridge_flow_ratio: bridge > p.maxBridge ? 0.85 : 0.95,
  bridge_speed_mms: Math.min(25, speed),
  fan_max_pct: Math.max(fan, layerTimeProxy < 4 ? 100 : fan),
  detect_thin_walls: true,
  fill_gaps: true,
  support_enable: oh >= 0.7 || bridge > p.maxBridge,
  support_overhang_deg: maxOh,
  seam_position: 'rear',
  recommended_print_axis: preferredAxis,
  notes: 'Hints only — verify in PrusaSlicer / Orca / Cura; not a vendor profile file.',
 }

 return {
  lineWidthMm: width,
  minWallMm: round(minWall, 3),
  bridgeSpanMm: round(bridge, 2),
  overhangRisk: oh,
  preferredAxis,
  orientations,
  issues,
  tips,
  slicerHints,
 }
}

/**
 * Fit-to-print + optional reorientation and light cell/rib tweaks
 * to reduce bridges / thin walls.
 */
export function optimizeLatticeForPrint(
 o: LighteningOptions,
 p: LatticePrintSettings,
 opts?: {reorient?: boolean; shrinkBridges?: boolean},
): {options: LighteningOptions; report: LatticePrintReport; changed: string[]} {
 const reorient = opts?.reorient ?? true
 const shrinkBridges = opts?.shrinkBridges ?? true
 const changed: string[] = []
 let next: LighteningOptions = {...o}

 // Rank before fit (fit forces channel axis to Z for FDM).
 if (reorient && !isSpatialPattern(next.pattern)) {
  const best = suggestPrintOrientation(next, p)[0]?.axis ?? 'z'
  if (next.axis !== best) {
   next = {...next, axis: best}
   changed.push(`reorient_channels_${best}`)
  }
 }

 if (isSpatialPattern(next.pattern) && p.openTop && !next.openTop) {
  next = {...next, openTop: true}
  changed.push('open_top')
 }

 next = fitLatticeToPrint(next, p)
 changed.push('fit_lines_layers')

 if (shrinkBridges) {
  const span = bridgeSpanMm(next)
  if (span > p.maxBridge) {
   const width = lineWidthOf(p)
   const targetCell = Math.max(next.rib * 2 + width, p.maxBridge + next.rib)
   if (targetCell < next.cell) {
    next = {...next, cell: round(targetCell, 3)}
    changed.push('shrink_cell_for_bridge')
   } else {
    const needRib = Math.max(next.rib, next.cell - p.maxBridge)
    const rib = Math.round(Math.ceil((needRib - 1e-8) / width) * width * 1e6) / 1e6
    if (rib > next.rib) {
     next = {...next, rib, cell: Math.max(next.cell, rib * 2 + width)}
     changed.push('thicken_rib_for_bridge')
    }
   }
   next = fitLatticeToPrint(next, p)
  }
 }

 return {options: next, report: analyzeLatticePrintability(next, p), changed}
}

export function formatLatticePrintReport(r: LatticePrintReport, locale: 'ru' | 'en'): string {
 const L = locale === 'ru'
 const lines = [
  L
   ? `Печать FDM: линия ${r.lineWidthMm} mm, мин. стенка ${r.minWallMm} mm, мост ≈${r.bridgeSpanMm} mm, свесы ${Math.round(r.overhangRisk * 100)}%, ось ${r.preferredAxis.toUpperCase()}`
   : `FDM print: line ${r.lineWidthMm} mm, min wall ${r.minWallMm} mm, bridge ≈${r.bridgeSpanMm} mm, overhangs ${Math.round(r.overhangRisk * 100)}%, axis ${r.preferredAxis.toUpperCase()}`,
 ]
 for (const issue of r.issues) {
  lines.push(`${issue.severity === 'critical' ? '✖' : issue.severity === 'warn' ? '⚠' : '•'} ${L ? issue.ru : issue.en}`)
 }
 if (r.tips.length) {
  lines.push(L ? 'Советы:' : 'Tips:')
  for (const tip of r.tips.slice(0, 6)) lines.push(`• ${L ? tip.ru : tip.en}`)
 }
 lines.push(L
  ? `Ориентации: ${r.orientations.map(o => `${o.axis.toUpperCase()}(${o.score})`).join(' › ')}`
  : `Orientations: ${r.orientations.map(o => `${o.axis.toUpperCase()}(${o.score})`).join(' › ')}`)
 return lines.join('\n')
}

/** JSON blob of slicer hints for copy/download (not a vendor profile). */
export function latticeSlicerHintsJson(r: LatticePrintReport): string {
 return JSON.stringify({generator: 'open-scad-viewer/lattice-print', hints: r.slicerHints}, null, 2)
}
