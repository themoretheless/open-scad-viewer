import {expect, it} from 'vitest'
import {GeometryKernelError} from '../src/services/geometry/kernel'
import {listStructuralProfiles, lookupStructuralProfile} from '../src/services/structuralProfiles'

it('computes IPE 200 properties within the fillet allowance through real WASM', () => {
  // EN 10365 tabulated: A = 28.5 cm², Ix = 1943 cm⁴, Iy = 142.4 cm⁴,
  // mass 22.4 kg/m; computed values exclude fillets, so sit a few % below.
  const r = lookupStructuralProfile('ipe200')
  expect(r.designation).toBe('IPE 200')
  expect(r.family).toBe('ipe')
  expect(r.heightMm).toBe(200)
  expect(r.areaMm2).toBeGreaterThan(2850*0.93)
  expect(r.areaMm2).toBeLessThanOrEqual(2850)
  expect(r.iStrongMm4/1e4).toBeGreaterThan(1943*0.93)
  expect(r.iStrongMm4/1e4).toBeLessThanOrEqual(1943)
  expect(r.iWeakMm4/1e4).toBeGreaterThan(142.4*0.93)
  expect(r.iWeakMm4/1e4).toBeLessThanOrEqual(142.4)
  expect(r.massKgM).toBeGreaterThan(22.4*0.93)
  expect(r.massKgM).toBeLessThanOrEqual(22.4)
  expect(r.wStrongMm3).toBeCloseTo(r.iStrongMm4/100, 9)
  expect(r.jMm4).toBeGreaterThan(0)
  expect(r.cwMm6).toBeGreaterThan(0)
})

it('normalizes W, GOST and STO designations through real WASM', () => {
  const w = lookupStructuralProfile('W250×32.7')
  expect(w.designation).toBe('W250X32.7')
  // W10x22: A = 41.7 cm², Ix = 4900 cm⁴, mass 32.7 kg/m by designation.
  expect(w.massKgM).toBeCloseTo(32.7, 0)
  expect(w.iStrongMm4/1e4).toBeGreaterThan(4900*0.93)
  expect(w.iStrongMm4/1e4).toBeLessThanOrEqual(4900)

  const gost = lookupStructuralProfile('И20')
  expect(gost.designation).toBe('I20')
  expect(gost.family).toBe('gost-8239')
  expect(gost.areaMm2).toBeCloseTo(2632.64, 6)

  const sto = lookupStructuralProfile('20б1')
  expect(sto.designation).toBe('20B1')
  expect(sto.family).toBe('sto-aschm-b')
  expect(sto.iWeakMm4/1e4).toBeCloseTo(133.9, 0)

  const heb = lookupStructuralProfile('heb 300')
  expect(heb.designation).toBe('HEB 300')
  expect(heb.flangeWidthMm).toBe(300)
})

it('lists the catalogue and rejects unknown designations through real WASM', () => {
  const rows = listStructuralProfiles()
  expect(rows.length).toBeGreaterThanOrEqual(50)
  expect(new Set(rows.map(r => r.designation)).size).toBe(rows.length)
  for (const family of ['ipe', 'heb', 'w', 'gost-8239', 'sto-aschm-b'])
    expect(rows.some(r => r.family === family)).toBe(true)
  // Every listed designation round-trips through lookup.
  for (const row of rows) {
    const r = lookupStructuralProfile(row.designation)
    expect(r.areaMm2).toBeGreaterThan(0)
    expect(r.iStrongMm4).toBeGreaterThan(r.iWeakMm4)
  }
  expect(() => lookupStructuralProfile('IPE 999')).toThrowError(GeometryKernelError)
  try { lookupStructuralProfile('banana') } catch (error) {
    expect((error as GeometryKernelError).code).toBe('PROFILE_UNKNOWN')
  }
})
