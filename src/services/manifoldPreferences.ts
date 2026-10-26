import {ref} from 'vue'
import type {MeshImportRepairMode} from './meshImport'
import {storageGet, storageSet} from './safeStorage'

export const MANIFOLD_PREFERENCES_STORAGE_KEY = 'scad-manifold-preferences-v1'
export const MANIFOLD_WELD_EPSILON_RANGE = Object.freeze({min: 0, max: 1})
export const MANIFOLD_BOOLEAN_TOLERANCE_RANGE = Object.freeze({min: 1e-12, max: 1e-5})

export interface ManifoldPreferences {
  readonly version: 1
  readonly importWeldEpsilonMm: number
  readonly importRepair: MeshImportRepairMode
  readonly booleanWeldEpsilonMm: number
  readonly booleanRelativeTolerance: number
}

export const DEFAULT_MANIFOLD_PREFERENCES: ManifoldPreferences = Object.freeze({
  version: 1,
  importWeldEpsilonMm: 1e-4,
  importRepair: 'conservative',
  booleanWeldEpsilonMm: 0,
  booleanRelativeTolerance: 1e-9,
})

const repairModes: readonly MeshImportRepairMode[] = ['none', 'conservative', 'full']
const inRange = (value: unknown, min: number, max: number): value is number =>
  typeof value === 'number' && Number.isFinite(value) && value >= min && value <= max
const validWeld = (value: unknown) => inRange(value, MANIFOLD_WELD_EPSILON_RANGE.min, MANIFOLD_WELD_EPSILON_RANGE.max)
const validTolerance = (value: unknown) => inRange(value, MANIFOLD_BOOLEAN_TOLERANCE_RANGE.min, MANIFOLD_BOOLEAN_TOLERANCE_RANGE.max)

export function isManifoldPreferences(value: unknown): value is ManifoldPreferences {
  if (!value || typeof value !== 'object') return false
  const candidate = value as Partial<ManifoldPreferences>
  return candidate.version === 1 && validWeld(candidate.importWeldEpsilonMm)
    && repairModes.includes(candidate.importRepair as MeshImportRepairMode)
    && validWeld(candidate.booleanWeldEpsilonMm) && validTolerance(candidate.booleanRelativeTolerance)
}

/** Sanitize each field independently so one damaged value does not discard the others. */
export function normalizeManifoldPreferences(value: unknown): ManifoldPreferences {
  const candidate = value && typeof value === 'object' ? value as Partial<ManifoldPreferences> : {}
  return {
    version: 1,
    importWeldEpsilonMm: validWeld(candidate.importWeldEpsilonMm)
      ? candidate.importWeldEpsilonMm : DEFAULT_MANIFOLD_PREFERENCES.importWeldEpsilonMm,
    importRepair: repairModes.includes(candidate.importRepair as MeshImportRepairMode)
      ? candidate.importRepair as MeshImportRepairMode : DEFAULT_MANIFOLD_PREFERENCES.importRepair,
    booleanWeldEpsilonMm: validWeld(candidate.booleanWeldEpsilonMm)
      ? candidate.booleanWeldEpsilonMm : DEFAULT_MANIFOLD_PREFERENCES.booleanWeldEpsilonMm,
    booleanRelativeTolerance: validTolerance(candidate.booleanRelativeTolerance)
      ? candidate.booleanRelativeTolerance : DEFAULT_MANIFOLD_PREFERENCES.booleanRelativeTolerance,
  }
}

/** Load and canonicalize damaged storage without allowing storage exceptions to escape. */
export function loadManifoldPreferences(): ManifoldPreferences {
  const raw = storageGet(MANIFOLD_PREFERENCES_STORAGE_KEY)
  if (raw === null) return {...DEFAULT_MANIFOLD_PREFERENCES}
  let stored: unknown = null
  try { stored = JSON.parse(raw) } catch { /* canonicalize below */ }
  const normalized = normalizeManifoldPreferences(stored)
  const canonical = JSON.stringify(normalized)
  if (raw !== canonical) storageSet(MANIFOLD_PREFERENCES_STORAGE_KEY, canonical)
  return normalized
}

const state = ref<ManifoldPreferences>(loadManifoldPreferences())

/** A detached operation snapshot: later UI changes cannot alter a queued worker job. */
export function manifoldPreferenceSnapshot(): ManifoldPreferences {
  return {...state.value}
}

export function updateManifoldPreferences(patch: Partial<Omit<ManifoldPreferences, 'version'>>): ManifoldPreferences {
  const next = {...state.value, ...patch, version: 1 as const}
  if (!isManifoldPreferences(next)) throw new RangeError('Invalid manifold preferences.')
  state.value = next
  storageSet(MANIFOLD_PREFERENCES_STORAGE_KEY, JSON.stringify(next))
  return manifoldPreferenceSnapshot()
}

export function resetManifoldPreferences(): ManifoldPreferences {
  return updateManifoldPreferences(DEFAULT_MANIFOLD_PREFERENCES)
}

/** Re-read external/corrupted storage and update every open settings surface. */
export function reloadManifoldPreferences(): ManifoldPreferences {
  state.value = loadManifoldPreferences()
  return manifoldPreferenceSnapshot()
}

export function useManifoldPreferences() {
  return {manifoldPreferences: state}
}
