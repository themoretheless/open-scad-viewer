import {describe, it, expect} from 'vitest'
import {
 LATTICE_MATERIALS,
 ashbyExponent,
 estimateRelativeDensity,
 analyzeLatticeStrength,
 findLatticeWeakSpots,
 formatLatticeStrengthReport,
 weakSpotHighlightMeshes,
 feaMemberHighlightMeshes,
 compareLatticeVariants,
} from '../src/services/latticeStrengthAnalysis'
import {
 DEFAULT_PRINT_SERVICE,
 defaultCompressionScenario,
 defaultStrutSection,
 effectiveMaterialScalars,
} from '../src/services/latticeStrengthScenario'
import {type LighteningOptions} from '../src/services/solidLightening'
import {extrudeDirectSketch} from '../src/services/directModeling'

const box = () => extrudeDirectSketch({id: 's', name: 'Box', closed: true, points: [[0, 0], [20, 0], [20, 12], [0, 12]]}, 10, '0')
const base: LighteningOptions = {
 pattern: 'octet', axis: 'z', cell: 12, rib: 3.2, rim: 0, bottom: 0, top: 0,
 seed: 42, jitter: 0, lineWidth: 0.45, perimeters: 3, skin: 0, step: 1.2, diagonals: true,
}
const pla = LATTICE_MATERIALS.find(m => m.id === 'pla')!

describe('lattice strength analysis', () => {
 it('ranks stretch lattices with lower Ashby exponents than grids', () => {
  expect(ashbyExponent('octet')).toBeLessThan(ashbyExponent('grid'))
  expect(ashbyExponent('isogrid')).toBe(1)
  expect(ashbyExponent('spatial', false)).toBeGreaterThan(ashbyExponent('spatial', true))
 })

 it('estimates lower relative density for larger cells and thinner ribs', () => {
  const dense = estimateRelativeDensity(box(), {...base, cell: 8, rib: 4})
  const light = estimateRelativeDensity(box(), {...base, cell: 16, rib: 2})
  expect(dense).toBeGreaterThan(light)
  expect(light).toBeGreaterThan(0.02)
  expect(dense).toBeLessThan(0.98)
 })

 it('reports stretch class, strut stress and Euler ratio for octet walls', () => {
  const report = analyzeLatticeStrength(
   box(),
   {...base, wallDepth: 3.6, keepCore: false},
   pla,
   {case: 'compression', forceN: 200, safety: 2},
   {beforeMm3: 2400, afterMm3: 720},
  )
  expect(report.loadClass).toBe('stretch')
  expect(report.relativeDensity).toBeCloseTo(0.3, 5)
  expect(report.volumeReductionPct).toBeCloseTo(70, 5)
  expect(report.ashbyExponent).toBe(1)
  expect(report.relativeStiffness).toBeGreaterThan(0)
  expect(report.strutStressMPa).toBeGreaterThan(0)
  expect(report.eulerBucklingN).toBeGreaterThan(0)
  expect(report.connectivity).toBeGreaterThan(3)
  expect(report.disclaimer.ru + report.disclaimer.en).toMatch(/Ashby|сопромат|strength/i)
 })

 it('flags low Euler margin for slender struts under high force', () => {
  const report = analyzeLatticeStrength(
   box(),
   {...base, cell: 16, rib: 1.2},
   pla,
   {case: 'compression', forceN: 5000, safety: 1.5},
  )
  expect(report.bucklingRatio).toBeGreaterThan(0.7)
  expect(report.warnings.some(w => /Euler|Эйлер/i.test(w.en + w.ru))).toBe(true)
 })

 it('gives isogrid higher specific stiffness than plain grid at the same density', () => {
  const load = {case: 'compression' as const, forceN: 100, safety: 2}
  const volumes = {beforeMm3: 2400, afterMm3: 1200}
  const iso = analyzeLatticeStrength(box(), {...base, pattern: 'isogrid', rib: 1.5, cell: 8, rim: 2, bottom: 0.6}, pla, load, volumes)
  const grid = analyzeLatticeStrength(box(), {...base, pattern: 'grid', rib: 1.5, cell: 8, rim: 2, bottom: 0.6}, pla, load, volumes)
  expect(iso.specificStiffness).toBeGreaterThan(grid.specificStiffness)
  expect(iso.ashbyExponent).toBeLessThan(grid.ashbyExponent)
 })

 it('formats a bilingual report and rejects invalid loads', () => {
  const report = analyzeLatticeStrength(box(), base, pla, {case: 'tension', forceN: 50, safety: 2})
  const ru = formatLatticeStrengthReport(report, pla, {case: 'tension', forceN: 50, safety: 2}, 'ru')
  const en = formatLatticeStrengthReport(report, pla, {case: 'tension', forceN: 50, safety: 2}, 'en')
  expect(ru).toContain('Сопромат')
  expect(en).toContain('Strength estimate')
  expect(en).toContain('Ashby')
  expect(ru).toMatch(/Слабые места|Weak spots/)
  expect(en).toMatch(/Weak spots/)
  expect(report.weakSpots.length).toBeGreaterThan(0)
  expect(report.weakSpots[0].at).toHaveLength(3)
  expect(() => analyzeLatticeStrength(box(), base, pla, {case: 'compression', forceN: 0, safety: 2})).toThrow(/force/i)
  expect(() => analyzeLatticeStrength(box(), base, pla, {case: 'compression', forceN: 10, safety: 0.5})).toThrow(/safety|factor/i)
 })

 it('flags slender struts and body-corner nodes on a coarse octet lattice', () => {
  const spots = findLatticeWeakSpots(
   box(),
   {...base, cell: 16, rib: 1.2},
   pla,
   {case: 'compression', forceN: 4000, safety: 1.5},
  )
  expect(spots.some(s => s.kind === 'slender_strut' || s.kind === 'long_span' || s.kind === 'bridge')).toBe(true)
  expect(spots.some(s => s.kind === 'body_corner' || s.kind === 'underconnected' || s.kind === 'hinge_node')).toBe(true)
  expect(spots[0].score).toBeGreaterThanOrEqual(spots.at(-1)!.score)
  expect(['critical', 'high', 'medium', 'low']).toContain(spots[0].severity)
 })

 it('flags frame corners and wide openings on channel grids', () => {
  const spots = findLatticeWeakSpots(
   box(),
   {...base, pattern: 'grid', cell: 10, rib: 1.2, rim: 1.5, bottom: 0.6, top: 0.6, axis: 'z'},
   pla,
   {case: 'bending', forceN: 80, safety: 2},
  )
  expect(spots.some(s => s.kind === 'body_corner')).toBe(true)
  expect(spots.some(s => s.kind === 'long_span' || s.kind === 'bridge')).toBe(true)
 })

 it('flags acute corners on irregular web cells', () => {
  const spots = findLatticeWeakSpots(
   box(),
   {...base, pattern: 'web', cell: 7, rib: 1.1, rim: 1, bottom: 0.4, top: 0, jitter: 0.9, seed: 7},
   pla,
   {case: 'tension', forceN: 60, safety: 2},
  )
  // Web jitter creates irregular polygons; acute corners or frame corners should appear.
  expect(spots.length).toBeGreaterThan(0)
  expect(spots.some(s => s.kind === 'sharp_corner' || s.kind === 'body_corner')).toBe(true)
 })
})

 it('weights weak spots by material: PLA elevates corners/bridges, steel elevates slender buckling', () => {
  const o = {...base, cell: 16, rib: 1.2}
  const load = {case: 'compression' as const, forceN: 3500, safety: 1.5}
  const pla = LATTICE_MATERIALS.find(m => m.id === 'pla')!
  const steel = LATTICE_MATERIALS.find(m => m.id === 'steel')!
  const nylon = LATTICE_MATERIALS.find(m => m.id === 'nylon')!
  const plaSpots = findLatticeWeakSpots(box(), o, pla, load)
  const steelSpots = findLatticeWeakSpots(box(), o, steel, load)
  const nylonSpots = findLatticeWeakSpots(box(), {...base, pattern: 'web', cell: 7, rib: 1.1, rim: 1, bottom: 0.4, top: 0, jitter: 0.9, seed: 7}, nylon, {case: 'bending', forceN: 80, safety: 2})
  const plaCorner = plaSpots.find(s => s.kind === 'body_corner' || s.kind === 'sharp_corner')
  const nylonCorner = nylonSpots.find(s => s.kind === 'body_corner' || s.kind === 'sharp_corner')
  expect(plaSpots[0].color).toHaveLength(4)
  expect(plaSpots[0].materialBias).toBeGreaterThan(0)
  // Brittle PLA corner bias exceeds ductile nylon when both see a corner.
  if (plaCorner && nylonCorner) expect(plaCorner.materialBias).toBeGreaterThan(nylonCorner.materialBias)
  const steelSlender = steelSpots.find(s => s.kind === 'slender_strut')
  const plaSlender = plaSpots.find(s => s.kind === 'slender_strut')
  if (steelSlender && plaSlender) {
   // Higher E lowers Euler risk multiplier for the same geometry.
   expect(steelSlender.materialBias).toBeLessThan(plaSlender.materialBias)
  }
  const meshes = weakSpotHighlightMeshes(plaSpots.slice(0, 3), 1.5)
  expect(meshes.length).toBe(Math.min(3, plaSpots.length))
  expect(meshes[0].color).toEqual(plaSpots[0].color)
  expect(meshes[0].indices.length).toBeGreaterThan(0)
 })

 it('lists different material-biased severities in the formatted report', () => {
  const load = {case: 'compression' as const, forceN: 4000, safety: 1.5}
  const pla = LATTICE_MATERIALS.find(m => m.id === 'pla')!
  const steel = LATTICE_MATERIALS.find(m => m.id === 'steel')!
  const plaReport = analyzeLatticeStrength(box(), {...base, cell: 16, rib: 1.2}, pla, load)
  const steelReport = analyzeLatticeStrength(box(), {...base, cell: 16, rib: 1.2}, steel, load)
  const plaText = formatLatticeStrengthReport(plaReport, pla, load, 'en')
  const steelText = formatLatticeStrengthReport(steelReport, steel, load, 'en')
  expect(plaText).toMatch(/Weak spots for PLA/)
  expect(steelText).toMatch(/Weak spots for Mild steel/)
  expect(plaReport.warnings.some(w => /FDM|layer/i.test(w.en))).toBe(true)
  expect(steelReport.warnings.some(w => /Metal|buckling/i.test(w.en))).toBe(true)
 })

 it('applies FDM orthotropy and humidity knock-down for nylon', () => {
  const nylon = LATTICE_MATERIALS.find(m => m.id === 'nylon')!
  const dry = effectiveMaterialScalars(nylon, {...DEFAULT_PRINT_SERVICE, humidityPct: 20, temperatureC: 23, infill: 1})
  const wet = effectiveMaterialScalars(nylon, {...DEFAULT_PRINT_SERVICE, humidityPct: 80, temperatureC: 50, infill: 0.4, perimeters: 2})
  expect(dry.EParallel).toBeGreaterThan(dry.EPerp)
  expect(wet.E).toBeLessThan(dry.E)
  expect(wet.envFactor).toBeLessThan(dry.envFactor)
 })

 it('runs truss FEA with supports/loads, deflection, advice and fea markers', () => {
  const pla = LATTICE_MATERIALS.find(m => m.id === 'pla')!
  const load = {case: 'compression' as const, forceN: 300, safety: 2}
  const scenario = defaultCompressionScenario(load.forceN, load.safety)
  scenario.section = {...defaultStrutSection(2.5), eccentricity: 0.3}
  scenario.service = {...DEFAULT_PRINT_SERVICE, printAxis: 'z', humidityPct: 40}
  const report = analyzeLatticeStrength(box(), {...base, pattern: 'octet', cell: 12, rib: 2.5, wallDepth: 3.6, keepCore: false}, pla, load, {beforeMm3: 2400, afterMm3: 900}, scenario)
  expect(report.scenarioId).toBe('compression')
  expect(report.maxDeflectionMm).toBeGreaterThan(0)
  expect(report.feaMembers?.length).toBeGreaterThan(0)
  expect(report.homogenized?.EStar).toBeGreaterThan(0)
  expect(report.advice?.length).toBeGreaterThan(0)
  expect(report.boundarySummary?.en).toMatch(/support|Fixed|force/i)
  const text = formatLatticeStrengthReport(report, pla, load, 'en')
  expect(text).toMatch(/FEA|deflection|δ_max|η/i)
  expect(text).toMatch(/Recommend|Increase|Reduce|diagonal|strut|cell|Advice|tip/i)
  const feaMeshes = feaMemberHighlightMeshes(report.feaMembers ?? [])
  expect(feaMeshes.length).toBe(report.feaMembers!.length)
  expect(feaMeshes[0].indices.length).toBeGreaterThan(0)
 })

 it('supports ULS combinations and A/B lattice comparison', () => {
  const pla = LATTICE_MATERIALS.find(m => m.id === 'pla')!
  const load = {case: 'compression' as const, forceN: 200, safety: 2}
  const scenario = defaultCompressionScenario(load.forceN, load.safety)
  scenario.activeId = 'uls'
  const uls = analyzeLatticeStrength(box(), {...base, pattern: 'octet', cell: 12, rib: 2.8, wallDepth: 3.6, keepCore: false}, pla, load, undefined, scenario)
  expect(uls.scenarioId).toBe('uls')
  expect(uls.maxDeflectionMm ?? 0).toBeGreaterThan(0)
  const octet = analyzeLatticeStrength(box(), {...base, pattern: 'octet', cell: 12, rib: 2.8}, pla, load, {beforeMm3: 2400, afterMm3: 1000})
  const bcc = analyzeLatticeStrength(box(), {...base, pattern: 'bcc', cell: 12, rib: 2.8}, pla, load, {beforeMm3: 2400, afterMm3: 1000})
  const cmp = compareLatticeVariants(
   {label: 'octet', massProxy: octet.relativeDensity, stiffnessProxy: octet.relativeStiffness, utilization: octet.utilization ?? 1, bucklingRatio: octet.bucklingRatio ?? 0.01},
   {label: 'bcc', massProxy: bcc.relativeDensity, stiffnessProxy: bcc.relativeStiffness, utilization: bcc.utilization ?? 1, bucklingRatio: bcc.bucklingRatio ?? 0.01},
  )
  expect(cmp.rows.length).toBeGreaterThanOrEqual(3)
  expect(['a', 'b', 'tie']).toContain(cmp.winner)
  expect(cmp.summary.en.length).toBeGreaterThan(0)
 })

 it('raises utilization when strut eccentricity adds bending', () => {
  const pla = LATTICE_MATERIALS.find(m => m.id === 'pla')!
  const load = {case: 'compression' as const, forceN: 250, safety: 2}
  const concentric = defaultCompressionScenario(load.forceN, load.safety)
  concentric.section = {...defaultStrutSection(2.4), eccentricity: 0}
  const eccentric = defaultCompressionScenario(load.forceN, load.safety)
  eccentric.section = {...defaultStrutSection(2.4), eccentricity: 0.8}
  const a = analyzeLatticeStrength(box(), {...base, pattern: 'octet', cell: 12, rib: 2.4}, pla, load, undefined, concentric)
  const b = analyzeLatticeStrength(box(), {...base, pattern: 'octet', cell: 12, rib: 2.4}, pla, load, undefined, eccentric)
  expect((b.combinedUtilization ?? b.utilization ?? 0)).toBeGreaterThanOrEqual((a.combinedUtilization ?? a.utilization ?? 0))
 })
