<script setup lang="ts">
import type { CustomizerParameter, CustomizerValue } from '../services/scadCustomizer'

defineProps<{
  parameters: CustomizerParameter[]
  title: string
  emptyLabel: string
}>()

const emit = defineEmits<{
  change: [name: string, value: CustomizerValue]
}>()

function emitNumber(name: string, event: Event) {
  const input = event.target as HTMLInputElement
  if (input.value.trim() === '' || !Number.isFinite(input.valueAsNumber)) return
  emit('change', name, input.valueAsNumber)
}
function optionValue(parameter: CustomizerParameter, event: Event): CustomizerValue {
  const index = Number((event.target as HTMLSelectElement).value)
  return parameter.options?.[index] ?? parameter.value
}
</script>

<template>
  <section class="customizer" :aria-label="title">
    <header><span>{{ title }}</span><span class="count">{{ parameters.length }}</span></header>
    <p v-if="parameters.length === 0" class="empty">{{ emptyLabel }}</p>
    <div v-for="parameter in parameters" :key="parameter.name" class="parameter">
      <label :for="`customizer-${parameter.name}`">
        <span>{{ parameter.label }}</span><code>{{ parameter.name }}</code>
      </label>

      <template v-if="parameter.options?.length">
        <select
          :id="`customizer-${parameter.name}`"
          :value="parameter.options.findIndex(value => value === parameter.value)"
          @change="emit('change', parameter.name, optionValue(parameter, $event))"
        >
          <option v-for="(option, index) in parameter.options" :key="`${option}`" :value="index">{{ option }}</option>
        </select>
      </template>

      <template v-else-if="typeof parameter.value === 'boolean'">
        <input
          :id="`customizer-${parameter.name}`"
          type="checkbox"
          :checked="parameter.value"
          @change="emit('change', parameter.name, ($event.target as HTMLInputElement).checked)"
        >
      </template>

      <div v-else-if="typeof parameter.value === 'number'" class="numeric">
        <input
          v-if="parameter.min !== undefined && parameter.max !== undefined"
          :id="`customizer-${parameter.name}`"
          type="range"
          :min="parameter.min"
          :max="parameter.max"
          :step="parameter.step ?? 'any'"
          :value="parameter.value"
          @input="emitNumber(parameter.name, $event)"
        >
        <input
          :id="parameter.min === undefined ? `customizer-${parameter.name}` : undefined"
          class="number"
          type="number"
          :min="parameter.min"
          :max="parameter.max"
          :step="parameter.step ?? 'any'"
          :value="parameter.value"
          @input="emitNumber(parameter.name, $event)"
        >
      </div>

      <input
        v-else
        :id="`customizer-${parameter.name}`"
        type="text"
        :value="parameter.value"
        @input="emit('change', parameter.name, ($event.target as HTMLInputElement).value)"
      >
    </div>
  </section>
</template>

<style scoped>
.customizer { min-width: 0; color: var(--text); }
header { display: flex; align-items: center; justify-content: space-between; min-height: 34px; padding: 0 10px; border-bottom: 1px solid var(--border); font-size: .72rem; font-weight: 650; letter-spacing: .02em; }
.count { min-width: 19px; padding: 1px 5px; border-radius: 999px; background: var(--hover); color: var(--text-dim); text-align: center; font-size: .62rem; }
.empty { margin: 0; padding: 14px 10px; color: var(--text-dim); font-size: .69rem; line-height: 1.4; }
.parameter { display: grid; grid-template-columns: minmax(75px, .8fr) minmax(90px, 1.2fr); gap: 8px; align-items: center; padding: 7px 10px; border-bottom: 1px solid color-mix(in srgb, var(--border) 55%, transparent); }
label { min-width: 0; display: flex; flex-direction: column; gap: 2px; font-size: .69rem; }
label span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
code { color: var(--text-dim); font-size: .58rem; }
select, input[type="text"], input[type="number"] { width: 100%; min-width: 0; height: 26px; padding: 3px 6px; border: 1px solid var(--border); border-radius: 5px; background: var(--surface-raised); color: var(--text); font-size: .68rem; }
.numeric { min-width: 0; display: grid; grid-template-columns: 1fr 58px; gap: 6px; align-items: center; }
.numeric > .number:only-child { grid-column: 1 / -1; }
input[type="range"] { width: 100%; min-width: 0; accent-color: var(--accent); }
input[type="checkbox"] { width: 15px; height: 15px; accent-color: var(--accent); }
:is(input, select):focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
</style>
