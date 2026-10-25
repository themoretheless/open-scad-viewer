import { computed, nextTick, ref, shallowRef, type ComputedRef, type Ref, type ShallowRef } from 'vue'
import {
  applyParameterPreset,
  captureParameterPreset,
  parameterTemplateHash,
  type ParameterPreset,
} from '../services/parameterPresets'
import { type CustomizerParameter, type CustomizerValue, encodeCustomizerValue } from '../services/scadCustomizer'
import { applySourceSplice } from '../services/sourceSplice'
import {
  MAX_WORKSPACE_SOURCE_LENGTH,
  updateWorkspaceDocument,
  type WorkspaceDocumentSnapshot,
} from '../services/workspaceDocument'
import type { Language } from '../i18n/appStrings'

export interface UseParameterPresetsOptions {
  code: Ref<string>
  editorRef: Ref<HTMLTextAreaElement | null>
  workspaceDocument: Ref<WorkspaceDocumentSnapshot>
  lang: Ref<Language>
  customizerParameters: ComputedRef<readonly CustomizerParameter[]>
  bumpWorkspaceEditGeneration: () => void
  scheduleWorkspacePersistence: () => void
}

export interface UseParameterPresetsReturn {
  presetName: Ref<string>
  presetSelection: Ref<string>
  presetError: Ref<string>
  presetUndo: ShallowRef<{ before: string; after: string } | null>
  parameterPresets: ComputedRef<readonly ParameterPreset[]>
  selectedPreset: ComputedRef<ParameterPreset | undefined>
  currentParameterTemplate: ComputedRef<string | null>
  presetCompatible: ComputedRef<boolean>
  savePresetList: (presets: readonly ParameterPreset[]) => boolean
  saveParameterPreset: () => Promise<void>
  replacePresetSource: (source: string) => void
  restoreParameterPreset: () => void
  undoParameterPreset: () => void
  removeParameterPreset: () => void
  updateCustomizer: (name: string, value: CustomizerValue) => void
}

export function useParameterPresets(options: UseParameterPresetsOptions): UseParameterPresetsReturn {
  const {
    code,
    editorRef,
    workspaceDocument,
    lang,
    customizerParameters,
    bumpWorkspaceEditGeneration,
    scheduleWorkspacePersistence,
  } = options

  const presetName = ref('')
  const presetSelection = ref('')
  const presetError = ref('')
  const presetUndo = shallowRef<{ before: string; after: string } | null>(null)
  const parameterPresets = computed(() => workspaceDocument.value.parameterPresets)
  const selectedPreset = computed(
    () => parameterPresets.value.find(item => item.id === presetSelection.value) ?? parameterPresets.value[0],
  )
  const currentParameterTemplate = computed(() => {
    if (!parameterPresets.value.length) return null
    try {
      return parameterTemplateHash(code.value)
    } catch {
      return null
    }
  })
  const presetCompatible = computed(
    () => !!selectedPreset.value && selectedPreset.value.templateHash === currentParameterTemplate.value,
  )

  function savePresetList(presets: readonly ParameterPreset[]): boolean {
    try {
      workspaceDocument.value = updateWorkspaceDocument(workspaceDocument.value, { parameterPresets: presets })
      bumpWorkspaceEditGeneration()
      scheduleWorkspacePersistence()
      return true
    } catch {
      presetError.value =
        lang.value === 'ru'
          ? 'Не удалось сохранить варианты: превышен допустимый объём.'
          : 'Could not save presets: storage size limit exceeded.'
      return false
    }
  }

  async function saveParameterPreset() {
    await nextTick()
    presetError.value = ''
    if (parameterPresets.value.some(item => item.name === presetName.value.trim())) {
      presetError.value =
        lang.value === 'ru'
          ? 'Вариант с таким именем уже есть. Выберите другое имя.'
          : 'This name already exists. Choose another name.'
      return
    }
    try {
      const preset = captureParameterPreset(code.value, presetName.value, crypto.randomUUID())
      if (savePresetList([...parameterPresets.value, preset])) {
        presetSelection.value = preset.id
        presetName.value = ''
      }
    } catch {
      presetError.value =
        lang.value === 'ru'
          ? 'Не удалось сохранить: нужны уникальные параметры (до 128) и имя до 80 символов.'
          : 'Could not save: requires unique parameters (up to 128) and a name of up to 80 characters.'
    }
  }

  function replacePresetSource(source: string) {
    if (source.length > MAX_WORKSPACE_SOURCE_LENGTH) throw new Error('source limit')
    const editor = editorRef.value
    if (editor) {
      editor.setRangeText(source, 0, editor.value.length, 'preserve')
      code.value = editor.value
    } else code.value = source
  }

  function restoreParameterPreset() {
    presetError.value = ''
    if (!selectedPreset.value) return
    try {
      const before = code.value
      const after = applyParameterPreset(before, selectedPreset.value)
      if (before === after) return
      replacePresetSource(after)
      presetUndo.value = { before, after }
    } catch {
      presetError.value =
        lang.value === 'ru'
          ? 'Вариант не подходит к текущему исходнику.'
          : 'This preset does not match the current source.'
    }
  }

  function undoParameterPreset() {
    const undo = presetUndo.value
    if (!undo || code.value !== undo.after) return
    replacePresetSource(undo.before)
    presetUndo.value = null
  }

  function removeParameterPreset() {
    if (!selectedPreset.value) return
    presetError.value = ''
    if (savePresetList(parameterPresets.value.filter(item => item.id !== selectedPreset.value!.id))) {
      presetSelection.value = ''
    }
  }

  function updateCustomizer(name: string, value: CustomizerValue) {
    const parameter = customizerParameters.value.find(candidate => candidate.name === name)
    if (!parameter) return
    const encoded = encodeCustomizerValue(value)
    const applied = applySourceSplice(code.value, {
      from: parameter.valueStart,
      to: parameter.valueEnd,
      insert: encoded,
      expected: code.value.slice(parameter.valueStart, parameter.valueEnd),
      origin: 'customizer',
    })
    const editor = editorRef.value
    if (editor) {
      editor.setRangeText(encoded, parameter.valueStart, parameter.valueEnd, 'select')
      code.value = editor.value
    } else code.value = applied.source
  }

  return {
    presetName,
    presetSelection,
    presetError,
    presetUndo,
    parameterPresets,
    selectedPreset,
    currentParameterTemplate,
    presetCompatible,
    savePresetList,
    saveParameterPreset,
    replacePresetSource,
    restoreParameterPreset,
    undoParameterPreset,
    removeParameterPreset,
    updateCustomizer,
  }
}
