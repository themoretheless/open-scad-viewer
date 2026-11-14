import { computed, ref, shallowRef } from 'svet'
import { DirectHistory, emptyDirectDocument } from '../src/app/directModeling'
import { useNurbsMatching, type NurbsMatchingOptions } from '../src/features/directModeler/useNurbsMatching'

/**
 * Localized refusal text exactly as the direct modeler shows it: the same
 * `bodyCalculationFailure` composable function, created with plain state.
 */
export function controlFailureMessage(
  error: unknown,
  locale: 'ru' | 'en',
  state: { bodyName?: string; edges?: number[] } = {},
): string {
  const label = (ru: string, en: string) => (locale === 'ru' ? ru : en)
  const body = computed(() =>
    state.bodyName
      ? { id: 'body', name: state.bodyName, mesh: { positions: new Float64Array(), indices: new Uint32Array() } }
      : undefined,
  )
  const options = {
    advanced: ref({}),
    advancedOp: ref(null),
    bodyEditError: () => ref(''),
    document: shallowRef(emptyDirectDocument()),
    edgeIndex: ref(-1),
    edgeIndexes: ref(state.edges ?? []),
    history: new DirectHistory(),
    label,
    pointTrimPick: shallowRef(null),
    project: () => [0, 0],
    selectedBody: body,
    selectedNurbsCurve: computed(() => undefined),
    selectedNurbsSurface: computed(() => undefined),
    selection: ref(''),
    surfaceInputLabel: () => '',
    surfaceInputs: ref([]),
  } as unknown as NurbsMatchingOptions
  return useNurbsMatching(options).bodyCalculationFailure(error, false)
}
