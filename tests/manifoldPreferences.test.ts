import {afterEach, describe, expect, it, vi} from 'vitest'

async function loadModule() {
  vi.resetModules()
  return import('../src/services/manifoldPreferences')
}

function storage() {
  const values = new Map<string, string>()
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, String(value)),
    removeItem: (key: string) => values.delete(key),
  })
  return values
}

afterEach(() => vi.unstubAllGlobals())

describe('manifold preferences', () => {
  it('persists validated values and returns detached operation snapshots', async () => {
    const values = storage()
    const preferences = await loadModule()
    expect(preferences.manifoldPreferenceSnapshot()).toEqual(preferences.DEFAULT_MANIFOLD_PREFERENCES)
    preferences.updateManifoldPreferences({importWeldEpsilonMm: 0.002, importRepair: 'full', booleanWeldEpsilonMm: 0.001, booleanRelativeTolerance: 1e-7})
    const snapshot = preferences.manifoldPreferenceSnapshot()
    preferences.updateManifoldPreferences({importRepair: 'none'})
    expect(snapshot.importRepair).toBe('full')
    expect(JSON.parse(values.get(preferences.MANIFOLD_PREFERENCES_STORAGE_KEY)!)).toMatchObject({version: 1, importRepair: 'none', booleanRelativeTolerance: 1e-7})
    expect(preferences.reloadManifoldPreferences()).toMatchObject({importWeldEpsilonMm: 0.002, importRepair: 'none'})
  })

  it.each([NaN, Infinity, -1, 2])('rejects unsafe weld epsilon %s', async importWeldEpsilonMm => {
    storage()
    const preferences = await loadModule()
    expect(() => preferences.updateManifoldPreferences({importWeldEpsilonMm})).toThrow(RangeError)
    expect(preferences.manifoldPreferenceSnapshot().importWeldEpsilonMm).toBe(1e-4)
  })

  it.each([0, 1e-13, 1e-4, NaN])('rejects unsafe Boolean tolerance %s', async booleanRelativeTolerance => {
    storage()
    const preferences = await loadModule()
    expect(() => preferences.updateManifoldPreferences({booleanRelativeTolerance})).toThrow(RangeError)
  })

  it('normalizes malformed and out-of-range storage field by field', async () => {
    const values = storage()
    const key = 'scad-manifold-preferences-v1'
    values.set(key, JSON.stringify({version: 99, importWeldEpsilonMm: 0.003, importRepair: 'unknown', booleanWeldEpsilonMm: -2, booleanRelativeTolerance: 1e-8}))
    const preferences = await loadModule()
    expect(preferences.manifoldPreferenceSnapshot()).toEqual({version: 1, importWeldEpsilonMm: 0.003, importRepair: 'conservative', booleanWeldEpsilonMm: 0, booleanRelativeTolerance: 1e-8})
    expect(JSON.parse(values.get(key)!).version).toBe(1)
    values.set(key, '{broken')
    expect(preferences.reloadManifoldPreferences()).toEqual(preferences.DEFAULT_MANIFOLD_PREFERENCES)
    expect(JSON.parse(values.get(key)!)).toEqual(preferences.DEFAULT_MANIFOLD_PREFERENCES)
  })

  it('keeps safe defaults when storage is unavailable and supports reset', async () => {
    vi.stubGlobal('localStorage', {getItem: () => {throw new Error('blocked')}, setItem: () => {throw new Error('blocked')}})
    const preferences = await loadModule()
    expect(preferences.manifoldPreferenceSnapshot()).toEqual(preferences.DEFAULT_MANIFOLD_PREFERENCES)
    preferences.updateManifoldPreferences({importRepair: 'full'})
    expect(preferences.resetManifoldPreferences()).toEqual(preferences.DEFAULT_MANIFOLD_PREFERENCES)
  })
})
