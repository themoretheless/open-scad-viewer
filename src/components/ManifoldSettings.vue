<script setup lang="ts">
import {computed, reactive, ref, watch} from 'vue'
import {
  MANIFOLD_BOOLEAN_TOLERANCE_RANGE,
  MANIFOLD_WELD_EPSILON_RANGE,
  isManifoldPreferences,
  manifoldPreferenceSnapshot,
  resetManifoldPreferences,
  updateManifoldPreferences,
  useManifoldPreferences,
  type ManifoldPreferences,
} from '../services/manifoldPreferences'

type Draft = {-readonly [Key in keyof ManifoldPreferences]: ManifoldPreferences[Key]}
const props = defineProps<{locale: string}>()
const label = (ru: string, en: string) => props.locale === 'ru' ? ru : en
const {manifoldPreferences} = useManifoldPreferences()
const draft = reactive<Draft>({...manifoldPreferenceSnapshot()})
watch(manifoldPreferences, value => Object.assign(draft, value))
const saveError = ref('')
const valid = computed(() => isManifoldPreferences(draft))
function save() {
  saveError.value = ''
  if (!valid.value) {
    saveError.value = label('Проверьте допуски: weld 0…1 мм; Boolean 1e-12…1e-5.', 'Check tolerances: weld 0…1 mm; Boolean 1e-12…1e-5.')
    return
  }
  try { updateManifoldPreferences(draft) }
  catch (error) { saveError.value = error instanceof Error ? error.message : String(error) }
}
function restoreDefaults() {
  saveError.value = ''
  Object.assign(draft, resetManifoldPreferences())
}
</script>

<template>
  <fieldset class="manifold-settings">
    <legend>{{ label('Настройки manifold', 'Manifold settings') }}</legend>
    <label class="file-row">{{ label('Сварка при импорте, мм', 'Import weld epsilon, mm') }}
      <input v-model.number="draft.importWeldEpsilonMm" type="number" :min="MANIFOLD_WELD_EPSILON_RANGE.min" :max="MANIFOLD_WELD_EPSILON_RANGE.max" step="0.0001" :aria-label="label('Допуск сварки при импорте, мм', 'Import weld epsilon, mm')" :aria-invalid="!valid" @change="save">
    </label>
    <label class="file-row">{{ label('Проверка manifold при импорте', 'Manifold inspection on import') }}
      <select :model-value="draft.importRepair" :aria-label="label('Режим repair при импорте', 'Import repair mode')" @update:model-value="draft.importRepair = $event; save()">
        <option value="none">{{ label('Только проверка', 'Check only') }}</option>
        <option value="conservative">Conservative</option>
        <option value="full">Full · {{ label('зашивка отверстий', 'fill holes') }}</option>
      </select>
    </label>
    <label class="file-row">{{ label('Сварка repair для Boolean, мм', 'Boolean repair weld epsilon, mm') }}
      <input v-model.number="draft.booleanWeldEpsilonMm" type="number" :min="MANIFOLD_WELD_EPSILON_RANGE.min" :max="MANIFOLD_WELD_EPSILON_RANGE.max" step="0.0001" :aria-label="label('Допуск сварки Boolean, мм', 'Boolean weld epsilon, mm')" :aria-invalid="!valid" @change="save">
    </label>
    <label class="file-row">{{ label('Относительный допуск Boolean', 'Boolean relative tolerance') }}
      <input v-model.number="draft.booleanRelativeTolerance" type="number" :min="MANIFOLD_BOOLEAN_TOLERANCE_RANGE.min" :max="MANIFOLD_BOOLEAN_TOLERANCE_RANGE.max" step="0.000000001" :aria-label="label('Относительный допуск Boolean', 'Boolean relative tolerance')" :aria-invalid="!valid" @change="save">
    </label>
    <small>{{ label('Сохраняется локально. Допуск Boolean относится к наибольшей стороне общих габаритов.', 'Saved locally. Boolean tolerance is relative to the longest combined bounding-box side.') }}</small>
    <button type="button" @click="restoreDefaults">{{ label('Безопасные значения', 'Restore safe defaults') }}</button>
    <span v-if="saveError" role="alert">{{ saveError }}</span>
  </fieldset>
</template>
