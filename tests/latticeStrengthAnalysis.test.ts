import {describe, it, expect} from 'vitest'
import {
 LATTICE_MATERIALS,
 ashbyExponent,
 estimateRelativeDensity,
 analyzeLatticeStrength,
 formatLatticeStrengthReport,
} from '../src/services/latticeStrengthAnalysis'
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
  expect(() => analyzeLatticeStrength(box(), base, pla, {case: 'compression', forceN: 0, safety: 2})).toThrow(/force/i)
  expect(() => analyzeLatticeStrength(box(), base, pla, {case: 'compression', forceN: 10, safety: 0.5})).toThrow(/safety|factor/i)
 })
})
