import { computed, ref, shallowRef, watch, watchEffect, type Ref } from 'svet'
import { computeSpinnerParameterSuggestions } from '../app/spinnerGenerator'
import type { MeshData } from '../core/mesh'
import { analyzeSurfaceArea, type GeometryComputeResult } from '../kernel/geometryCompute'
import { isRushFrontend } from '../language/rushFrontendDetect'
import { extractCustomizerParameters, type CustomizerParameter } from '../language/scadCustomizer'

export interface UseGeometryAnalysisOptions {
  code: Ref<string>
  lang: Ref<'ru' | 'en'>
  selectedMesh: Ref<number | null>
  sceneMeshes: Ref<MeshData[]>
  canExport: () => boolean
}

/** On-demand surface analysis of the selected mesh; any scene or source change cancels it. */
export function useGeometryAnalysis(options: UseGeometryAnalysisOptions) {
  const computeResult = shallowRef<GeometryComputeResult | null>(null)
  const computeBusy = ref(false)
  const computeError = ref('')
  let computeAbort: AbortController | null = null

  function cancelGeometryAnalysis() {
    computeAbort?.abort()
    computeAbort = null
    computeBusy.value = false
    computeResult.value = null
    computeError.value = ''
  }
  watch([options.selectedMesh, options.sceneMeshes, options.code], cancelGeometryAnalysis, { flush: 'sync' })

  async function runGeometryAnalysis() {
    const index = options.selectedMesh.value
    const mesh = index === null ? null : options.sceneMeshes.value[index]
    if (!mesh || !options.canExport() || computeBusy.value) return
    cancelGeometryAnalysis()
    const controller = new AbortController()
    computeAbort = controller
    computeBusy.value = true
    const current = () => computeAbort === controller && !controller.signal.aborted
    try {
      const result = await analyzeSurfaceArea(mesh, controller.signal)
      if (current()) computeResult.value = result
    } catch {
      if (current())
        computeError.value =
          options.lang.value === 'ru' ? 'Не удалось проанализировать геометрию.' : 'Could not analyze geometry.'
    } finally {
      if (computeAbort === controller) {
        computeBusy.value = false
        computeAbort = null
      }
    }
  }

  return { computeResult, computeBusy, computeError, cancelGeometryAnalysis, runGeometryAnalysis }
}

const SUGGESTION_TRIGGERS = ['inner_radius', 'hole', 'outer_radius', 'inner_rim', 'outer_rim']

/** Customizer parameters of the editor source and the spinner's dependent-parameter suggestions. */
export function useCustomizerSource(code: Ref<string>) {
  // The rush compiler pulls the geometry kernel chunk; load it only when a
  // rush-frontend document is actually open.
  const compactControls = ref<{ parameters: CustomizerParameter[]; errors: string[] }>({ parameters: [], errors: [] })
  watchEffect(async () => {
    const source = code.value
    if (!isRushFrontend(source)) {
      compactControls.value = { parameters: [], errors: [] }
      return
    }
    const controls = (await import('../language/rushFrontend')).rushFrontendControls(source)
    if (code.value === source) compactControls.value = controls
  })
  const customizerParameters = computed(() =>
    isRushFrontend(code.value) ? compactControls.value.parameters : extractCustomizerParameters(code.value),
  )
  const editorParameterSuggestions = computed(() => {
    if (!customizerParameters.value.length) return []
    const map: Record<string, string | number | boolean> = {}
    for (const param of customizerParameters.value) map[param.name] = param.value
    for (const candidate of SUGGESTION_TRIGGERS) {
      const suggestions = computeSpinnerParameterSuggestions(candidate, map)
      if (suggestions.length > 0) return suggestions
    }
    return []
  })
  return { compactControls, customizerParameters, editorParameterSuggestions }
}
