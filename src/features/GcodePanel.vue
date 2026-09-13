<script setup lang="ts">
import { computed, onUnmounted, reactive, ref, shallowRef, watch } from 'vue'
import type { MeshData } from '../core/mesh'
import type { ToolpathSettingsInput } from '../services/geometry/polygon'
import { downloadCad } from '../services/cadDrawing'
import { drawGcodeLayer, gcodeLayerRange, gcodeMeshBounds } from '../services/gcodePreviewGeometry'
import { GCODE_PREVIEW_MAX_BYTES, type GcodePreviewDocument } from '../services/gcodePreviewProtocol'
import { createGcodePreviewWorker } from '../services/gcodePreviewWorker'

const props = defineProps<{ meshes: MeshData[]; selection: number[]; source: string; ready: boolean; locale: string }>()
const label = (ru: string, en: string) => props.locale === 'ru' ? ru : en
const worker = createGcodePreviewWorker()
const settings = reactive<Required<ToolpathSettingsInput>>({
  layerHeightMm: 0.2, lineWidthMm: 0.4, wallCount: 2, infillSpacingMm: 2,
  feedrateMmS: 50, travelFeedrateMmS: 120, filamentDiameterMm: 1.75,
})
const fields: { key: keyof ToolpathSettingsInput; ru: string; en: string; min: number; step: number; max?: number }[] = [
  { key: 'layerHeightMm', ru: 'Высота слоя, мм', en: 'Layer height, mm', min: 0.01, step: 0.05 },
  { key: 'lineWidthMm', ru: 'Ширина линии, мм', en: 'Line width, mm', min: 0.01, step: 0.05 },
  { key: 'wallCount', ru: 'Линий стенки', en: 'Wall count', min: 1, max: 8, step: 1 },
  { key: 'infillSpacingMm', ru: 'Шаг заполнения, мм', en: 'Infill spacing, mm', min: 0.01, step: 0.1 },
  { key: 'feedrateMmS', ru: 'Скорость печати, мм/с', en: 'Print speed, mm/s', min: 0.01, step: 1 },
  { key: 'travelFeedrateMmS', ru: 'Холостой ход, мм/с', en: 'Travel speed, mm/s', min: 0.01, step: 1 },
  { key: 'filamentDiameterMm', ru: 'Диаметр филамента, мм', en: 'Filament diameter, mm', min: 0.01, step: 0.05 },
]
const zMin = ref(0), zMax = ref(0), busy = ref(false), error = ref(''), message = ref('')
const result = shallowRef<GcodePreviewDocument | null>(null)
const filename = ref('preview.gcode'), layer = ref(0), showTravel = ref(true), canvas = ref<HTMLCanvasElement | null>(null)
const selectedMesh = computed(() => props.selection.length === 1 ? props.meshes[props.selection[0]] ?? null : null)
const selectedBounds = computed(() => {
  if (!selectedMesh.value) return null
  try {
    if (selectedMesh.value.indices.length > 300000) return null
    return gcodeMeshBounds(selectedMesh.value)
  } catch { return null }
})
const unavailable = computed(() => !props.ready
  ? label('Сначала соберите текущий код.', 'Build the current source first.')
  : !selectedMesh.value ? label('Выберите ровно одно тело в сцене.', 'Select exactly one body in the scene.')
  : !selectedBounds.value ? label('Нужна корректная сетка до 100000 треугольников.', 'A valid mesh with at most 100000 triangles is required.')
  : '')
const preview = computed(() => result.value?.preview ?? null)
const layerRange = computed(() => preview.value ? gcodeLayerRange(preview.value, layer.value) : null)
let revision = 0
function invalidate(note = '') {
  revision++
  worker.cancel()
  busy.value = false; result.value = null; error.value = ''; message.value = note
}
function resetRange() {
  const bounds = selectedBounds.value
  if (bounds) { zMin.value = bounds.min[2]; zMax.value = bounds.max[2] }
}
watch([() => props.source, () => props.ready, selectedMesh, selectedBounds, () => props.selection.join(',')], () => {
  const stale = result.value !== null || busy.value
  resetRange()
  invalidate(stale ? label('Модель изменилась. Постройте траектории заново.', 'Model changed. Generate the toolpaths again.') : '')
}, { immediate: true, flush: 'sync' })
watch([settings, zMin, zMax], () => {
  invalidate(result.value || busy.value ? label('Настройки изменились. Постройте траектории заново.', 'Settings changed. Generate the toolpaths again.') : '')
}, { deep: true, flush: 'sync' })
watch([canvas, preview, layer, showTravel], () => {
  if (canvas.value && preview.value) drawGcodeLayer(canvas.value, preview.value, layer.value, showTravel.value)
}, { flush: 'post' })
onUnmounted(() => { revision++; worker.dispose() })

async function generate() {
  invalidate()
  if (unavailable.value || !selectedMesh.value) { error.value = unavailable.value; return }
  const ticket = revision
  busy.value = true
  try {
    const mesh = selectedMesh.value
    const document = await worker.run({ kind: 'slice', mesh: { vertices: mesh.vertices, indices: mesh.indices, transform: mesh.transform }, zMin: zMin.value, zMax: zMax.value, settings: { ...settings } })
    if (ticket !== revision) return
    result.value = document; layer.value = 0; filename.value = `body-${props.selection[0] + 1}-preview.gcode`
  } catch (reason) {
    if (ticket === revision && !(reason instanceof Error && reason.name === 'AbortError')) error.value = reason instanceof Error ? reason.message : String(reason)
  } finally { if (ticket === revision) busy.value = false }
}
function cancel() { invalidate(label('Расчёт отменён.', 'Processing cancelled.')) }
function toggle(event: Event) { if (!(event.target as HTMLDetailsElement).open && busy.value) cancel() }
async function openPreview(event: Event) {
  const input = event.target as HTMLInputElement, file = input.files?.[0]
  input.value = ''
  if (!file) return
  invalidate()
  const ticket = revision
  busy.value = true
  try {
    if (file.size > GCODE_PREVIEW_MAX_BYTES) throw new Error(label('Файл превышает 4 МиБ.', 'File exceeds 4 MiB.'))
    const gcode = await file.text()
    if (ticket !== revision) return
    const document = await worker.run({ kind: 'parse', gcode })
    if (ticket !== revision) return
    result.value = document; layer.value = 0; filename.value = file.name
  } catch (reason) {
    if (ticket === revision && !(reason instanceof Error && reason.name === 'AbortError')) error.value = reason instanceof Error ? reason.message : String(reason)
  } finally { if (ticket === revision) busy.value = false }
}
function download() {
  if (!result.value || busy.value) return
  try { downloadCad(result.value.gcode, 'text/plain;charset=utf-8', filename.value) }
  catch (reason) { error.value = reason instanceof Error ? reason.message : String(reason) }
}
const number = (value: number, digits = 2) => value.toLocaleString(props.locale === 'ru' ? 'ru-RU' : 'en-US', { maximumFractionDigits: digits })
</script>

<template>
  <details class="gcode-panel" @toggle="toggle">
    <summary>{{ label('Траектории и G-code', 'Toolpaths and G-code') }}</summary>
    <p class="gcode-note">{{ label('G-code для предпросмотра: стенки и линейное заполнение. Без нагрева, парковки осей, ретракта, поддержек и профиля принтера. Для печати используйте слайсер с профилем вашего принтера.', 'Preview G-code: walls and line infill. No heating, homing, retraction, supports, or printer profile. For printing, use a slicer with your printer profile.') }}</p>
    <p v-if="unavailable">{{ unavailable }}</p>
    <p v-else>{{ label('Тело', 'Body') }} {{ selection[0] + 1 }} · {{ number(selectedMesh!.indices.length / 3, 0) }} {{ label('треугольников', 'triangles') }}<br>
      {{ label('Координаты сцены, мм', 'Scene coordinates, mm') }}: Z {{ number(selectedBounds!.min[2]) }} … {{ number(selectedBounds!.max[2]) }}
    </p>
    <div class="gcode-fields">
      <label>Z min, mm<input v-model.number="zMin" aria-label="Z min, mm" type="number" step="0.1"></label>
      <label>Z max, mm<input v-model.number="zMax" aria-label="Z max, mm" type="number" step="0.1"></label>
    </div>
    <button type="button" :disabled="!selectedBounds" @click="resetRange">{{ label('Весь диапазон тела', 'Use full body range') }}</button>
    <div class="gcode-fields">
      <label v-for="field in fields" :key="field.key">{{ label(field.ru, field.en) }}
        <input v-model.number="settings[field.key]" :aria-label="label(field.ru, field.en)" type="number" :min="field.min" :max="field.max" :step="field.step">
      </label>
    </div>
    <div class="gcode-actions">
      <button type="button" :disabled="busy || !!unavailable" @click="generate">{{ label('Построить траектории', 'Generate toolpaths') }}</button>
      <button v-if="busy" type="button" @click="cancel">{{ label('Отменить расчёт', 'Cancel processing') }}</button>
    </div>
    <label class="gcode-file">{{ label('Открыть G-code предпросмотра', 'Open preview G-code') }}
      <input type="file" accept=".gcode,.gco,text/plain" :aria-label="label('Открыть G-code предпросмотра', 'Open preview G-code')" @change="openPreview">
    </label>
    <small>{{ label('Только диалект предпросмотра этого приложения, до 4 МиБ.', 'Only this app’s preview dialect, up to 4 MiB.') }}</small>
    <p v-if="busy" role="status">{{ label('Расчёт траекторий…', 'Processing toolpaths…') }}</p>
    <p v-if="message" role="status">{{ message }}</p>
    <p v-if="error" role="alert">{{ error }}</p>
    <section v-if="preview && result" class="gcode-result" :aria-label="label('Предпросмотр G-code', 'G-code preview')">
      <p class="gcode-filename">{{ filename }} · {{ result.dialect }}</p>
      <template v-if="preview.layers && preview.moves.length">
        <label>{{ label('Слой', 'Layer') }} {{ layer + 1 }} / {{ preview.layers }}
          <input v-model.number="layer" :aria-label="label('Слой предпросмотра', 'Preview layer')" type="range" min="0" :max="preview.layers - 1" step="1">
        </label>
        <p v-if="layerRange">Z {{ layerRange.z === null ? '—' : number(layerRange.z, 3) }} mm · {{ layerRange.end - layerRange.start }} {{ label('перемещений', 'moves') }}</p>
        <canvas ref="canvas" class="gcode-canvas" width="600" height="600" role="img" :aria-label="label(`Траектории слоя ${layer + 1}, вид сверху XY.`, `Layer ${layer + 1} toolpaths, XY top view.`)"></canvas>
        <div class="gcode-legend"><span class="gcode-extrusion">━ {{ label('Экструзия', 'Extrusion') }}</span><span class="gcode-travel">┄ {{ label('Холостой ход', 'Travel') }}</span></div>
        <label><input v-model="showTravel" type="checkbox">{{ label('Показывать холостой ход', 'Show travel moves') }}</label>
      </template>
      <p v-else>{{ label('В указанном диапазоне нет траекторий.', 'No toolpaths in this range.') }}</p>
      <dl class="gcode-stats">
        <dt>{{ label('Всего слоёв / перемещений', 'Total layers / moves') }}</dt><dd>{{ number(preview.layers, 0) }} / {{ number(preview.moves.length, 0) }}</dd>
        <dt>{{ label('Филамент, мм', 'Filament, mm') }}</dt><dd>{{ number(preview.extrusionMm) }}</dd>
        <dt>{{ label('Объём, мм³', 'Volume, mm³') }}</dt><dd>{{ number(preview.depositedVolumeMm3) }}</dd>
        <dt>{{ label('Путь печати, мм', 'Print distance, mm') }}</dt><dd>{{ number(preview.printDistanceMm) }}</dd>
        <dt>{{ label('Холостой путь, мм', 'Travel distance, mm') }}</dt><dd>{{ number(preview.travelDistanceMm) }}</dd>
        <dt>{{ label('Расчётное время движения, мин', 'Estimated motion time, min') }}</dt><dd>{{ number(preview.estimatedTimeS / 60) }}</dd>
      </dl>
      <small>{{ label('Время при постоянной скорости; без начального позиционирования, ускорений и операций принтера.', 'Time at constant speed; excludes initial positioning, acceleration, and printer operations.') }}</small>
      <button type="button" :disabled="busy || !preview.moves.length" @click="download">{{ label('Скачать G-code предпросмотра', 'Download preview G-code') }}</button>
    </section>
  </details>
</template>

<style scoped>
.gcode-panel{margin:10px 0;border-top:1px solid var(--border);padding-top:8px}
.gcode-panel summary{cursor:pointer;font-weight:600}.gcode-panel p{line-height:1.45}
.gcode-panel label{display:flex;align-items:center;gap:6px;margin:6px 0}.gcode-panel button,.gcode-panel input{font:inherit;background:var(--surface-raised);color:var(--text);border:1px solid var(--border);padding:5px;border-radius:4px}
.gcode-fields{display:grid;grid-template-columns:1fr 1fr;gap:0 8px}.gcode-fields label{display:flex;flex-direction:column;align-items:stretch;min-width:0}.gcode-fields input[type=number]{width:100%;box-sizing:border-box}
.gcode-actions{display:flex;flex-wrap:wrap;gap:6px;margin-top:8px}.gcode-panel button:disabled{opacity:.5;cursor:default}.gcode-file{flex-direction:column;align-items:stretch!important}.gcode-file input{width:100%;box-sizing:border-box}
.gcode-result{border-top:1px solid var(--border);margin-top:10px;padding-top:6px}.gcode-filename{overflow-wrap:anywhere;font-size:11px}.gcode-canvas{display:block;width:100%;height:auto;border:1px solid var(--border);box-sizing:border-box;border-radius:4px}
.gcode-result input[type=range]{min-width:0;flex:1}.gcode-legend{display:flex;gap:14px;background:#101922;padding:6px;border-radius:4px}.gcode-extrusion{color:#5bcbff}.gcode-travel{color:#e8ad61}
.gcode-stats{display:grid;grid-template-columns:1fr auto;gap:5px 10px}.gcode-stats dt{margin:0}.gcode-stats dd{margin:0;text-align:right;font-variant-numeric:tabular-nums}.gcode-result>button{display:block;margin-top:10px}.gcode-panel [role=alert]{color:var(--danger)}.gcode-panel small{line-height:1.45;display:block}.gcode-note{font-size:11px}
</style>
