/**
 * Display-quality preset of one viewer (low / medium / high), persisted per
 * viewer key. Choosing a preset writes its byte budget and target error into
 * the shared display budget setting (keeping the box proxy choice) and applies
 * that budget to the renderer's LOD scheduler.
 */
import { ref, watch, type Ref } from 'svet'
import { storageGet, storageSet } from '../core/safeStorage'
import {
  DISPLAY_BUDGET_PRESETS,
  isDisplayQualityPreset,
  type DisplayBudget,
  type DisplayQualityPreset,
} from '../render/displayScheduler'
import { boxProxiesOn } from '../render/displayScheduler'
import { getDisplayBudget, setDisplayBudget as storeDisplayBudget } from '../render/displayBudgetSetting'
import { registerDisplayLevelProvider, WorkerDisplayLevelProvider } from '../render/onDemandLevels'

/** Finer certified B-rep levels are produced on demand by a dedicated worker. */
export function registerDisplayLevelWorker() {
  if (typeof Worker === 'undefined') return
  registerDisplayLevelProvider(
    () =>
      new WorkerDisplayLevelProvider(
        () => new Worker(new URL('../workers/displayLevel.worker.ts', import.meta.url), { type: 'module' }),
      ),
  )
}

export const displayQualityStorageKey = (viewer: string) => `osv.displayQuality.${viewer}`

export function readDisplayQuality(viewer: string): DisplayQualityPreset {
  const stored = storageGet(displayQualityStorageKey(viewer))
  return isDisplayQualityPreset(stored) ? stored : 'medium'
}

export interface DisplayBudgetTarget {
  setDisplayBudget(budget: DisplayBudget): void
}

export function useDisplayQuality(options: {
  readonly viewer: string
  readonly getRenderer: () => DisplayBudgetTarget | null | undefined
  /** Renderer becomes available or is recreated; the preset is re-applied. */
  readonly rendererReady?: Ref<boolean>
}) {
  registerDisplayLevelWorker()
  const preset = ref<DisplayQualityPreset>(readDisplayQuality(options.viewer))
  const apply = () => options.getRenderer()?.setDisplayBudget(getDisplayBudget())
  function setDisplayQuality(next: DisplayQualityPreset) {
    if (!isDisplayQualityPreset(next)) return
    preset.value = next
    storageSet(displayQualityStorageKey(options.viewer), next)
    const { budgetBytes, targetErrorPixels } = DISPLAY_BUDGET_PRESETS[next]
    storeDisplayBudget({ budgetBytes, targetErrorPixels, boundingBoxProxies: boxProxiesOn(getDisplayBudget()) })
    apply()
  }
  if (options.rendererReady) watch(options.rendererReady, ready => ready && apply(), { immediate: true })
  return { displayQuality: preset, setDisplayQuality, applyDisplayQuality: apply }
}
