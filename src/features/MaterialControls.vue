<script setup lang="ts">
import { THEME_PRESETS, type ShadingModel } from '../services/rendererContracts'

const props = defineProps<{
  locale: string
  shadingModel: ShadingModel
  themeId: string
  baseColor: string
  metallic: number
  roughness: number
}>()
const emit = defineEmits<{
  'update:shadingModel': [value: ShadingModel]
  'update:themeId': [value: string]
  'update:baseColor': [value: string]
  'update:metallic': [value: number]
  'update:roughness': [value: number]
}>()

const SHADING_MODELS: readonly ShadingModel[] = ['phong', 'pbr', 'matcap', 'toon', 'unlit']
const label = (ru: string, en: string) => props.locale === 'ru' ? ru : en
const sliderValue = (event: Event) => (event.target as HTMLInputElement).valueAsNumber
</script>

<template>
  <div class="material-controls">
    <label>{{ label('Шейдер', 'Shading') }}
      <select
        :value="shadingModel"
        :aria-label="label('Модель затенения', 'Shading model')"
        @change="emit('update:shadingModel', ($event.target as HTMLSelectElement).value as ShadingModel)"
      >
        <option v-for="model in SHADING_MODELS" :key="model" :value="model">{{ model }}</option>
      </select>
    </label>
    <label>{{ label('Тема', 'Theme') }}
      <select
        :value="themeId"
        :aria-label="label('Тема рендера', 'Render theme')"
        @change="emit('update:themeId', ($event.target as HTMLSelectElement).value)"
      >
        <option v-for="theme in THEME_PRESETS" :key="theme.id" :value="theme.id">{{ theme.name }}</option>
      </select>
    </label>
    <label :title="label('Базовый цвет материала сцены', 'Scene default material base color')">{{ label('Цвет', 'Color') }}
      <input
        type="color"
        :value="baseColor"
        :aria-label="label('Базовый цвет', 'Base color')"
        @input="emit('update:baseColor', ($event.target as HTMLInputElement).value)"
      >
    </label>
    <label :title="label('Металличность материала сцены', 'Scene default material metallic')">{{ label('Металл', 'Metal') }}
      <input
        type="range" min="0" max="1" step="0.01"
        :value="metallic"
        :aria-label="label('Металличность', 'Metallic')"
        @input="emit('update:metallic', sliderValue($event))"
      >
    </label>
    <label :title="label('Шероховатость материала сцены', 'Scene default material roughness')">{{ label('Шерох.', 'Rough') }}
      <input
        type="range" min="0" max="1" step="0.01"
        :value="roughness"
        :aria-label="label('Шероховатость', 'Roughness')"
        @input="emit('update:roughness', sliderValue($event))"
      >
    </label>
  </div>
</template>

<style scoped>
.material-controls{padding:5px 8px;display:flex;align-items:center;gap:5px;font-size:11px;flex-wrap:wrap}.material-controls label{display:flex;align-items:center;gap:4px}.material-controls input[type=range]{width:64px}.material-controls input[type=color]{width:28px;height:22px;padding:1px}.material-controls input,.material-controls select{color:var(--text);background:var(--surface);border:1px solid var(--border);border-radius:4px;padding:4px;font:inherit}
</style>
