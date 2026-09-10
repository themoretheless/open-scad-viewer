/** Advanced lattice SoM scenario: loads, supports, cases, FDM service conditions. */

export type Vec3 = [number, number, number]

export type LatticeSupportKind = 'fixed' | 'pinned' | 'roller'
export type LatticeActionKind = 'force' | 'pressure' | 'moment' | 'distributed'
export type StrutSectionKind = 'circle' | 'square' | 'rectangle' | 'tube'

export interface LatticeSupport {
 id: string
 kind: LatticeSupportKind
 normal: Vec3
 at?: Vec3
 ru: string
 en: string
}

export interface LatticeAction {
 id: string
 kind: LatticeActionKind
 direction: Vec3
 /** N / MPa / N·mm / N/mm depending on kind. */
 magnitude: number
 at?: Vec3
 faceNormal?: Vec3
 ru: string
 en: string
}

export interface LatticeLoadCaseDef {
 id: string
 ru: string
 en: string
 safety: number
 supports: LatticeSupport[]
 actions: LatticeAction[]
}

export interface LatticeLoadCombination {
 id: string
 ru: string
 en: string
 terms: {caseId: string; factor: number}[]
}

export interface StrutSection {
 kind: StrutSectionKind
 a: number
 b?: number
 eccentricity?: number
 jointStiffness?: number
}

export interface PrintServiceConditions {
 printAxis: 'x' | 'y' | 'z'
 temperatureC: number
 humidityPct: number
 infill: number
 perimeters: number
 minWallMm: number
 maxBridgeMm: number
}

export interface LatticeStrengthScenario {
 cases: LatticeLoadCaseDef[]
 combinations: LatticeLoadCombination[]
 activeId: string
 section: StrutSection
 service: PrintServiceConditions
}

export const DEFAULT_PRINT_SERVICE: PrintServiceConditions = {
 printAxis: 'z',
 temperatureC: 23,
 humidityPct: 50,
 infill: 1,
 perimeters: 3,
 minWallMm: 0.8,
 maxBridgeMm: 10,
}

export const defaultStrutSection = (ribMm: number): StrutSection => ({
 kind: 'circle', a: ribMm, eccentricity: 0, jointStiffness: 0.5,
})

export function defaultCompressionScenario(forceN: number, safety = 2): LatticeStrengthScenario {
 return {
  activeId: 'compression',
  section: defaultStrutSection(3.2),
  service: {...DEFAULT_PRINT_SERVICE},
  cases: [
   {
    id: 'compression', ru: 'Сжатие', en: 'Compression', safety,
    supports: [{id: 'base', kind: 'fixed', normal: [0, 0, -1], ru: 'Жёсткая заделка −Z', en: 'Fixed support −Z'}],
    actions: [{id: 'top-force', kind: 'force', direction: [0, 0, -1], magnitude: forceN, faceNormal: [0, 0, 1], ru: 'Сила на +Z', en: 'Force on +Z'}],
   },
   {
    id: 'tension', ru: 'Растяжение', en: 'Tension', safety,
    supports: [{id: 'base', kind: 'fixed', normal: [0, 0, -1], ru: 'Жёсткая заделка −Z', en: 'Fixed support −Z'}],
    actions: [{id: 'top-pull', kind: 'force', direction: [0, 0, 1], magnitude: forceN, faceNormal: [0, 0, 1], ru: 'Растягивающая сила на +Z', en: 'Tensile force on +Z'}],
   },
   {
    id: 'bending', ru: 'Изгиб', en: 'Bending', safety,
    supports: [{id: 'left', kind: 'pinned', normal: [-1, 0, 0], ru: 'Шарнир −X', en: 'Pinned −X'}],
    actions: [{id: 'tip-force', kind: 'force', direction: [0, 0, -1], magnitude: forceN, faceNormal: [1, 0, 0], ru: 'Поперечная сила у +X', en: 'Transverse force at +X'}],
   },
   {
    id: 'pressure', ru: 'Давление на грань', en: 'Face pressure', safety,
    supports: [{id: 'base', kind: 'fixed', normal: [0, 0, -1], ru: 'Жёсткая заделка −Z', en: 'Fixed support −Z'}],
    actions: [{
     id: 'top-pressure', kind: 'pressure', direction: [0, 0, -1],
     magnitude: Math.max(0.05, forceN / 400), faceNormal: [0, 0, 1],
     ru: 'Давление на +Z', en: 'Pressure on +Z',
    }],
   },
  ],
  combinations: [{
   id: 'uls',
   ru: 'ULS 1.2·сжатие + 1.5·изгиб',
   en: 'ULS 1.2·compression + 1.5·bending',
   terms: [{caseId: 'compression', factor: 1.2}, {caseId: 'bending', factor: 1.5}],
  }],
 }
}

export function actionToForceN(action: LatticeAction, faceAreaMm2: number): number {
 if (action.kind === 'force' || action.kind === 'distributed') return action.magnitude
 if (action.kind === 'pressure') return action.magnitude * Math.max(faceAreaMm2, 1)
 const lever = Math.max(1, 0.25 * Math.sqrt(Math.max(faceAreaMm2, 1)))
 return action.magnitude / lever
}

function averageDir(dirs: Vec3[]): Vec3 {
 if (!dirs.length) return [0, 0, -1]
 const s: Vec3 = [0, 0, 0]
 for (const d of dirs) {s[0] += d[0]; s[1] += d[1]; s[2] += d[2]}
 const L = Math.hypot(s[0], s[1], s[2]) || 1
 return [s[0] / L, s[1] / L, s[2] / L]
}

export function resolveActiveCase(scenario: LatticeStrengthScenario): {
 caseDef: LatticeLoadCaseDef
 forceN: number
 safety: number
 primaryKind: LatticeActionKind
 loadAxis: Vec3
 supports: LatticeSupport[]
 actions: LatticeAction[]
} {
 const combo = scenario.combinations.find(c => c.id === scenario.activeId)
 if (combo) {
  let forceN = 0, safety = 1
  const dirs: Vec3[] = []
  const supports: LatticeSupport[] = []
  const actions: LatticeAction[] = []
  let caseDef = scenario.cases[0]
  let primaryKind: LatticeActionKind = 'force'
  for (const term of combo.terms) {
   const c = scenario.cases.find(x => x.id === term.caseId)
   if (!c) continue
   caseDef = c
   safety = Math.max(safety, c.safety)
   supports.push(...c.supports)
   for (const a of c.actions) {
    const scaled: LatticeAction = {...a, magnitude: a.magnitude * term.factor}
    actions.push(scaled)
    forceN += Math.abs(actionToForceN(scaled, 240))
    dirs.push(a.direction)
    primaryKind = a.kind
   }
  }
  return {caseDef, forceN: Math.max(forceN, 1e-6), safety, primaryKind, loadAxis: averageDir(dirs), supports, actions}
 }
 const caseDef = scenario.cases.find(c => c.id === scenario.activeId) ?? scenario.cases[0]
 const forceN = caseDef.actions.reduce((s, a) => s + Math.abs(actionToForceN(a, 240)), 0)
 return {
  caseDef,
  forceN: Math.max(forceN, 1e-6),
  safety: caseDef.safety,
  primaryKind: caseDef.actions[0]?.kind ?? 'force',
  loadAxis: averageDir(caseDef.actions.map(a => a.direction)),
  supports: caseDef.supports,
  actions: caseDef.actions,
 }
}

export function sectionProperties(section: StrutSection): {area: number; I: number; W: number; label: string} {
 const a = Math.max(section.a, 1e-6)
 const r2 = (v: number, d = 2) => Math.round(v * 10 ** d) / 10 ** d
 if (section.kind === 'circle') {
  const r = a / 2
  const I = Math.PI * r ** 4 / 4
  return {area: Math.PI * r * r, I, W: I / r, label: `⌀${r2(a)}`}
 }
 if (section.kind === 'square') {
  const I = a ** 4 / 12
  return {area: a * a, I, W: I / (a / 2), label: `${r2(a)}×${r2(a)}`}
 }
 if (section.kind === 'tube') {
  const ro = a / 2, ri = Math.max(0, Math.min((section.b ?? a * 0.6) / 2, ro * 0.95))
  const I = Math.PI / 4 * (ro ** 4 - ri ** 4)
  return {area: Math.PI * (ro * ro - ri * ri), I, W: I / ro, label: `⌀${r2(a)}/⌀${r2(ri * 2)}`}
 }
 const b = Math.max(section.b ?? a, 1e-6)
 const I = b * a ** 3 / 12
 return {area: a * b, I, W: I / (a / 2), label: `${r2(a)}×${r2(b)}`}
}

export function effectiveMaterialScalars(
 base: {id: string; E: number; sigmaAllow: number; density: number; process: 'fdm' | 'metal'},
 service: PrintServiceConditions,
 opts?: {EParallel?: number; EPerp?: number; sigmaInterlayer?: number; sigmaYield?: number},
): {
 E: number
 EParallel: number
 EPerp: number
 sigmaAllow: number
 sigmaInterlayer: number
 sigmaYield: number
 envFactor: number
} {
 const EParallel = opts?.EParallel ?? base.E
 const EPerp = opts?.EPerp ?? (base.process === 'fdm' ? base.E * 0.55 : base.E)
 const sigmaInterlayer = opts?.sigmaInterlayer ?? (base.process === 'fdm' ? base.sigmaAllow * 0.45 : base.sigmaAllow)
 const sigmaYield = opts?.sigmaYield ?? base.sigmaAllow * 1.15
 let env = 1
 if (base.process === 'fdm') {
  if (service.temperatureC > 40) env *= Math.max(0.45, 1 - (service.temperatureC - 40) * 0.012)
  if (base.id === 'nylon' && service.humidityPct > 40) env *= Math.max(0.5, 1 - (service.humidityPct - 40) * 0.006)
  env *= 0.55 + 0.45 * Math.min(1, Math.max(0.15, service.infill))
  env *= Math.min(1, 0.7 + 0.1 * Math.max(1, service.perimeters))
 }
 const E = base.process === 'fdm' ? Math.sqrt(EParallel * EPerp) * env : base.E * env
 const sigmaAllow = Math.min(base.sigmaAllow, sigmaInterlayer * (base.process === 'fdm' ? 1.15 : 1)) * env
 return {
  E,
  EParallel: EParallel * env,
  EPerp: EPerp * env,
  sigmaAllow,
  sigmaInterlayer: sigmaInterlayer * env,
  sigmaYield: sigmaYield * env,
  envFactor: env,
 }
}
