<script setup lang="ts">
import { GRID_UNITS, useModelingGrid, type GridUnit } from '../services/modelingGrid'
const props = defineProps<{ locale: string; snapping?: boolean }>()
const settings = useModelingGrid()
const { unit, size, enabled, grid, geometry, guides } = settings
const label = (ru: string, en: string) => props.locale === 'ru' ? ru : en
function changeSize(event: Event) {
  const input = event.target as HTMLInputElement
  settings.setSize(input.valueAsNumber)
  input.value = String(size.value)
}
</script>
<template>
  <div class="modeling-grid-controls">
    <label>{{ label('Клетка', 'Cell') }} <input :value="size" type="number" min="0" step="any" :aria-label="label('Размер клетки', 'Cell size')" @change="changeSize"></label>
    <select :value="unit" :aria-label="label('Единицы клетки', 'Cell units')" @change="settings.setUnit(($event.target as HTMLSelectElement).value as GridUnit)">
      <option v-for="(_, value) in GRID_UNITS" :key="value" :value="value">{{ value }}</option>
    </select>
    <template v-if="snapping">
      <label :title="label('Alt — временно без привязок', 'Alt temporarily bypasses snapping')"><input v-model="enabled" type="checkbox" @change="settings.saveSnaps">{{ label('Привязки', 'Snaps') }}</label>
      <details><summary :aria-label="label('Настройки привязок', 'Snap settings')">▾</summary><div>
        <label><input v-model="grid" type="checkbox" @change="settings.saveSnaps">{{ label('Сетка', 'Grid') }}</label>
        <label><input v-model="geometry" type="checkbox" @change="settings.saveSnaps">{{ label('Вершины, середины, центры, рёбра', 'Vertices, midpoints, centers, edges') }}</label>
        <label><input v-model="guides" type="checkbox" @change="settings.saveSnaps">{{ label('Выравнивание по осям', 'Axis alignment') }}</label>
      </div></details>
    </template>
  </div>
</template>
<style scoped>
.modeling-grid-controls{display:flex;align-items:center;gap:5px;font-size:11px;flex-wrap:wrap}.modeling-grid-controls label{display:flex;align-items:center;gap:4px}.modeling-grid-controls input[type=number]{width:66px}.modeling-grid-controls input,.modeling-grid-controls select,.modeling-grid-controls summary{color:var(--text);background:var(--surface);border:1px solid var(--border);border-radius:4px;padding:4px;font:inherit}.modeling-grid-controls details{position:relative}.modeling-grid-controls summary{cursor:pointer}.modeling-grid-controls details>div{position:absolute;right:0;top:100%;z-index:30;min-width:230px;display:grid;gap:9px;padding:12px;background:var(--surface);border:1px solid var(--border);border-radius:6px}
</style>
