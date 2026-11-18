/**
 * Persisted display budget chosen in the viewer settings. Framework-free:
 * the display-quality preset and the material menu (megabytes, box proxies)
 * write it; the renderer lifecycle applies it to every renderer it creates and
 * to the live one on change.
 */
import { storageGet, storageSet } from '../core/safeStorage'
import {
  boxProxiesOn,
  DEFAULT_DISPLAY_BUDGET,
  displayBudgetFor,
  sameDisplayBudget,
  type DisplayBudget,
} from './displayScheduler'

export const DISPLAY_BUDGET_STORAGE_KEY = 'osv.displayBudget'

type Listener = (budget: DisplayBudget) => void

const listeners = new Set<Listener>()
let current: DisplayBudget | null = null

/** Parses a stored value; anything malformed falls back to the default budget. */
export function parseDisplayBudget(raw: string | null): DisplayBudget {
  if (!raw) return DEFAULT_DISPLAY_BUDGET
  try {
    const value: unknown = JSON.parse(raw)
    if (!value || typeof value !== 'object') return DEFAULT_DISPLAY_BUDGET
    const { megabytes, boundingBoxProxies, targetErrorPixels } = value as {
      megabytes?: unknown
      boundingBoxProxies?: unknown
      targetErrorPixels?: unknown
    }
    if (typeof megabytes !== 'number') return DEFAULT_DISPLAY_BUDGET
    const budget = displayBudgetFor(megabytes, boundingBoxProxies !== false)
    return typeof targetErrorPixels === 'number' && Number.isFinite(targetErrorPixels) && targetErrorPixels > 0
      ? { ...budget, targetErrorPixels }
      : budget
  } catch {
    return DEFAULT_DISPLAY_BUDGET
  }
}

export function getDisplayBudget(): DisplayBudget {
  return (current ??= parseDisplayBudget(storageGet(DISPLAY_BUDGET_STORAGE_KEY)))
}

export function setDisplayBudget(budget: DisplayBudget): void {
  if (sameDisplayBudget(getDisplayBudget(), budget)) return
  current = budget
  storageSet(
    DISPLAY_BUDGET_STORAGE_KEY,
    JSON.stringify({
      megabytes: budget.budgetBytes / (1024 * 1024),
      boundingBoxProxies: boxProxiesOn(budget),
      targetErrorPixels: budget.targetErrorPixels,
    }),
  )
  for (const listener of listeners) listener(budget)
}

export function subscribeDisplayBudget(listener: Listener): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/** Drops the cached value and listeners so the next read goes back to storage (tests). */
export function resetDisplayBudgetSetting(): void {
  current = null
  listeners.clear()
}
