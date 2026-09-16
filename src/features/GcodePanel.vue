<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, shallowRef, watch } from 'vue'
import type { MeshData } from '../core/mesh'
import { GCODE_FLAVORS, type GcodeFlavor, type JobSettingsInput, type ToolpathSettingsInput } from '../services/geometry/polygon'
import { downloadCad } from '../services/cadDrawing'
import { drawGcodeLayer, gcodeLayerRange, gcodeMeshBounds } from '../services/gcodePreviewGeometry'
import { GCODE_PREVIEW_MAX_BYTES, type GcodePreviewDocument } from '../services/gcodePreviewProtocol'
import { createGcodePreviewWorker } from '../services/gcodePreviewWorker'
import {
  companionDiscover,
  companionHealth,
  companionSend,
  PRINTER_COMPANION_DEFAULT_URL,
  utf8ToBase64,
  type DiscoveredCompanionPrinter,
  type PrinterVendor,
} from '../services/printerCompanion'

const props = defineProps<{ meshes: MeshData[]; selection: number[]; source: string; ready: boolean; locale: string }>()
const label = (ru: string, en: string) => props.locale === 'ru' ? ru : en
const worker = createGcodePreviewWorker()
const settings = reactive<Required<ToolpathSettingsInput>>({
  layerHeightMm: 0.2, lineWidthMm: 0.4, wallCount: 2, infillSpacingMm: 2,
  feedrateMmS: 50, travelFeedrateMmS: 120, filamentDiameterMm: 1.75,
})
const jobSettings = reactive({
  nozzleTempC: 210,
  bedTempC: 60,
  retractLengthMm: 0.8,
  retractFeedrateMmS: 40,
  unretractFeedrateMmS: 40,
  retractMinTravelMm: 2,
  fanSpeed: 255,
  homeAxes: true,
  flavor: 'marlin' as GcodeFlavor,
})
const flavorOptions: { value: GcodeFlavor; ru: string; en: string }[] = [
  { value: 'marlin', ru: 'Marlin (Creality, Prusa Buddy, Ender…)', en: 'Marlin (Creality, Prusa Buddy, Ender…)' },
  { value: 'klipper', ru: 'Klipper', en: 'Klipper' },
  { value: 'reprapfirmware', ru: 'RepRapFirmware (Duet)', en: 'RepRapFirmware (Duet)' },
]
function setFlavor(event: Event) {
  const value = (event.target as HTMLSelectElement).value
  if ((GCODE_FLAVORS as readonly string[]).includes(value)) jobSettings.flavor = value as GcodeFlavor
}
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
const companionOnline = ref(false)
const companionBusy = ref(false)
const discovered = shallowRef<DiscoveredCompanionPrinter[]>([])
const printer = reactive({
  vendor: 'moonraker' as PrinterVendor,
  host: '',
  accessCode: '',
  serial: '',
  apiKey: '',
  token: '',
})
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
const isJob = computed(() => !!result.value?.gcode3mfBase64)
const foreign = computed(() => result.value?.native === false)
const detected = computed(() => {
  const doc = result.value
  if (!doc || doc.native !== false) return ''
  const parts = [doc.generator && doc.generator !== 'unknown' ? doc.generator : label('неизвестный слайсер', 'unknown slicer')]
  if (doc.flavor) parts.push(`${label('прошивка', 'firmware')}: ${doc.flavor}`)
  return parts.join(' · ')
})
const layerRange = computed(() => preview.value ? gcodeLayerRange(preview.value, layer.value) : null)
const sendDisabled = computed(() => companionBusy.value || busy.value || !companionOnline.value || !result.value || !printer.host.trim())
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
watch([settings, jobSettings, zMin, zMax], () => {
  invalidate(result.value || busy.value ? label('Настройки изменились. Постройте траектории заново.', 'Settings changed. Generate the toolpaths again.') : '')
}, { deep: true, flush: 'sync' })
watch([canvas, preview, layer, showTravel], () => {
  if (canvas.value && preview.value) drawGcodeLayer(canvas.value, preview.value, layer.value, showTravel.value)
}, { flush: 'post' })
onMounted(() => { void refreshCompanion() })
onUnmounted(() => { revision++; worker.dispose() })

async function refreshCompanion() {
  companionOnline.value = await companionHealth(PRINTER_COMPANION_DEFAULT_URL)
}

async function generate(kind: 'slice' | 'job') {
  invalidate()
  if (unavailable.value || !selectedMesh.value) { error.value = unavailable.value; return }
  const ticket = revision
  busy.value = true
  try {
    const mesh = selectedMesh.value
    const jobSettingsPayload: JobSettingsInput = { ...settings, ...jobSettings }
    const document = await worker.run(kind === 'job'
      ? { kind: 'job', mesh: { vertices: mesh.vertices, indices: mesh.indices, transform: mesh.transform }, zMin: zMin.value, zMax: zMax.value, settings: jobSettingsPayload }
      : { kind: 'slice', mesh: { vertices: mesh.vertices, indices: mesh.indices, transform: mesh.transform }, zMin: zMin.value, zMax: zMax.value, settings: { ...settings } })
    if (ticket !== revision) return
    result.value = document; layer.value = 0
    filename.value = kind === 'job'
      ? `body-${props.selection[0] + 1}-job.gcode`
      : `body-${props.selection[0] + 1}-preview.gcode`
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
function download3mf() {
  if (!result.value?.gcode3mfBase64 || busy.value) return
  try {
    const binary = atob(result.value.gcode3mfBase64)
    const bytes = new Uint8Array(binary.length)
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i)
    const name = filename.value.replace(/\.gcode$/i, '') + '.gcode.3mf'
    const url = URL.createObjectURL(new Blob([bytes], { type: 'application/vnd.ms-package.3dmanufacturing-3dmodel+xml' }))
    const a = document.createElement('a'); a.href = url; a.download = name; a.click()
    setTimeout(() => URL.revokeObjectURL(url), 1000)
  } catch (reason) { error.value = reason instanceof Error ? reason.message : String(reason) }
}
async function discoverPrinters() {
  companionBusy.value = true; error.value = ''; message.value = ''
  try {
    await refreshCompanion()
    if (!companionOnline.value) throw new Error(label('Companion недоступен. Запустите printer-cli serve.', 'Companion unreachable. Run printer-cli serve.'))
    discovered.value = await companionDiscover(PRINTER_COMPANION_DEFAULT_URL)
    message.value = discovered.value.length
      ? label(`Найдено: ${discovered.value.length}`, `Found: ${discovered.value.length}`)
      : label('Принтеры не найдены (только Bambu/Snapmaker).', 'No printers found (Bambu/Snapmaker only).')
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason)
  } finally { companionBusy.value = false }
}
function applyDiscovered(printerInfo: DiscoveredCompanionPrinter) {
  if (printerInfo.vendor === 'bambu' || printerInfo.vendor === 'snapmaker'
    || printerInfo.vendor === 'moonraker' || printerInfo.vendor === 'octoprint'
    || printerInfo.vendor === 'prusa' || printerInfo.vendor === 'creality') {
    printer.vendor = printerInfo.vendor
  }
  printer.host = printerInfo.port && printerInfo.vendor === 'snapmaker'
    ? `${printerInfo.host}:${printerInfo.port}`
    : printerInfo.host
  if (printerInfo.serial) printer.serial = printerInfo.serial
}
async function sendToPrinter() {
  if (sendDisabled.value || !result.value) return
  companionBusy.value = true; error.value = ''; message.value = ''
  try {
    await refreshCompanion()
    if (!companionOnline.value) throw new Error(label('Companion недоступен. Запустите printer-cli serve.', 'Companion unreachable. Run printer-cli serve.'))
    const vendor = printer.vendor.trim().toLowerCase() as PrinterVendor
    if (!['bambu', 'moonraker', 'octoprint', 'prusa', 'creality', 'snapmaker'].includes(vendor)) {
      throw new Error(label('Неизвестный вендор.', 'Unknown vendor.'))
    }
    const use3mf = vendor === 'bambu'
    if (use3mf && !result.value.gcode3mfBase64) {
      throw new Error(label('Для Bambu нужен print job (.gcode.3mf).', 'Bambu needs a print job (.gcode.3mf).'))
    }
    if (!use3mf && !result.value.gcode) throw new Error(label('Нет G-code для отправки.', 'No G-code to send.'))
    const fileName = use3mf
      ? filename.value.replace(/\.gcode$/i, '') + '.gcode.3mf'
      : filename.value.endsWith('.gcode') ? filename.value : `${filename.value}.gcode`
    const bytesBase64 = use3mf ? result.value.gcode3mfBase64! : utf8ToBase64(result.value.gcode)
    const outcome = await companionSend(vendor, {
      host: printer.host.trim(),
      access_code: printer.accessCode || undefined,
      serial: printer.serial || undefined,
      api_key: printer.apiKey || undefined,
      token: printer.token || undefined,
    }, fileName, bytesBase64, PRINTER_COMPANION_DEFAULT_URL)
    message.value = label(
      `Отправлено: ${outcome.remoteName}${outcome.verified ? ' (подтверждено)' : ''}`,
      `Sent: ${outcome.remoteName}${outcome.verified ? ' (verified)' : ''}`,
    )
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason)
  } finally { companionBusy.value = false }
}
const number = (value: number, digits = 2) => value.toLocaleString(props.locale === 'ru' ? 'ru-RU' : 'en-US', { maximumFractionDigits: digits })
</script>

<template>
  <details class="gcode-panel" @toggle="toggle">
    <summary>{{ label('Траектории и G-code', 'Toolpaths and G-code') }}</summary>
    <p class="gcode-note">{{ label('Предпросмотр: стенки и линейное заполнение. Print job добавляет нагрев, ретракт и .gcode.3mf. Отправка на принтер идёт через localhost companion (printer-cli serve), не из браузера.', 'Preview: walls and line infill. Print job adds heat, retract, and .gcode.3mf. Send-to-printer uses the localhost companion (printer-cli serve), not the browser.') }}</p>
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
    <div class="gcode-fields">
      <label>{{ label('Сопло, °C', 'Nozzle, °C') }}<input v-model.number="jobSettings.nozzleTempC" :aria-label="label('Сопло, °C', 'Nozzle, °C')" type="number" min="1" step="1"></label>
      <label>{{ label('Стол, °C', 'Bed, °C') }}<input v-model.number="jobSettings.bedTempC" :aria-label="label('Стол, °C', 'Bed, °C')" type="number" min="1" step="1"></label>
      <label>{{ label('Ретракт, мм', 'Retract, mm') }}<input v-model.number="jobSettings.retractLengthMm" :aria-label="label('Ретракт, мм', 'Retract, mm')" type="number" min="0.01" step="0.1"></label>
      <label>{{ label('Вентилятор 0–255', 'Fan 0–255') }}<input v-model.number="jobSettings.fanSpeed" :aria-label="label('Вентилятор 0–255', 'Fan 0–255')" type="number" min="0" max="255" step="1"></label>
    </div>
    <label><input v-model="jobSettings.homeAxes" type="checkbox">{{ label('Парковка осей (G28) в job', 'Home axes (G28) in job') }}</label>
    <label>{{ label('Диалект прошивки (job)', 'Firmware flavor (job)') }}
      <select :value="jobSettings.flavor" :aria-label="label('Диалект прошивки (job)', 'Firmware flavor (job)')" @change="setFlavor">
        <option v-for="option in flavorOptions" :key="option.value" :value="option.value">{{ label(option.ru, option.en) }}</option>
      </select>
    </label>
    <div class="gcode-actions">
      <button type="button" :disabled="busy || !!unavailable" @click="generate('slice')">{{ label('Построить траектории', 'Generate toolpaths') }}</button>
      <button type="button" :disabled="busy || !!unavailable" @click="generate('job')">{{ label('Собрать print job', 'Generate print job') }}</button>
      <button v-if="busy" type="button" @click="cancel">{{ label('Отменить расчёт', 'Cancel processing') }}</button>
    </div>
    <label class="gcode-file">{{ label('Открыть G-code', 'Open G-code') }}
      <input type="file" accept=".gcode,.gco,.g,.nc,text/plain" :aria-label="label('Открыть G-code предпросмотра', 'Open preview G-code')" @change="openPreview">
    </label>
    <small>{{ label('Собственные диалекты проверяются строго; файлы PrusaSlicer, Orca/Bambu, Cura, Klipper, RepRapFirmware и др. читаются в режиме предпросмотра (G0–G3, G90/G91, M82/M83, G92, G20/G21). До 4 МиБ.', 'Own dialects are validated strictly; PrusaSlicer, Orca/Bambu, Cura, Klipper, RepRapFirmware and other files are read in preview mode (G0–G3, G90/G91, M82/M83, G92, G20/G21). Up to 4 MiB.') }}</small>
    <p v-if="busy" role="status">{{ label('Расчёт траекторий…', 'Processing toolpaths…') }}</p>
    <p v-if="message" role="status">{{ message }}</p>
    <p v-if="error" role="alert">{{ error }}</p>
    <section v-if="preview && result" class="gcode-result" :aria-label="label('Предпросмотр G-code', 'G-code preview')">
      <p class="gcode-filename">{{ filename }} · {{ result.dialect }}<template v-if="isJob && result.flavor"> · {{ result.flavor }}</template></p>
      <p v-if="foreign" class="gcode-hint" role="note">{{ label('Сторонний файл', 'Foreign file') }}: {{ detected }}. {{ label('Неизвестные команды пропущены; статистика приблизительная, объём — по диаметру филамента из заголовка или 1.75 мм.', 'Unknown commands were skipped; statistics are approximate and volume uses the header filament diameter or 1.75 mm.') }}</p>
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
      <div class="gcode-actions">
        <button type="button" :disabled="busy || !preview.moves.length" @click="download">{{ isJob ? label('Скачать job .gcode', 'Download job .gcode') : foreign ? label('Скачать исходный G-code', 'Download original G-code') : label('Скачать G-code предпросмотра', 'Download preview G-code') }}</button>
        <button v-if="isJob" type="button" :disabled="busy" @click="download3mf">{{ label('Скачать .gcode.3mf', 'Download .gcode.3mf') }}</button>
      </div>
      <section class="gcode-printer" :aria-label="label('Принтер', 'Printer')">
        <h3>{{ label('Принтер (LAN via companion)', 'Printer (LAN via companion)') }}</h3>
        <p v-if="!companionOnline" class="gcode-hint">{{ label('Send отключён: запустите `printer-cli serve` на этом компьютере (порт 17890).', 'Send disabled: run `printer-cli serve` on this machine (port 17890).') }}</p>
        <p v-else class="gcode-hint">{{ label('Companion доступен на 127.0.0.1:17890.', 'Companion reachable at 127.0.0.1:17890.') }}</p>
        <div class="gcode-fields">
          <label>{{ label('Вендор', 'Vendor') }}
            <input v-model="printer.vendor" :aria-label="label('Вендор', 'Vendor')" type="text" list="none" autocomplete="off" placeholder="bambu|moonraker|octoprint|prusa|creality|snapmaker">
          </label>
          <label>{{ label('Хост / URL', 'Host / URL') }}<input v-model="printer.host" :aria-label="label('Хост / URL', 'Host / URL')" type="text" autocomplete="off"></label>
          <label v-if="printer.vendor === 'bambu'">{{ label('Access code', 'Access code') }}<input v-model="printer.accessCode" :aria-label="label('Access code', 'Access code')" type="password" autocomplete="off"></label>
          <label v-if="printer.vendor === 'bambu'">{{ label('Serial', 'Serial') }}<input v-model="printer.serial" :aria-label="label('Serial', 'Serial')" type="text" autocomplete="off"></label>
          <label v-if="printer.vendor === 'octoprint' || printer.vendor === 'prusa' || printer.vendor === 'moonraker' || printer.vendor === 'creality'">{{ label('API key', 'API key') }}<input v-model="printer.apiKey" :aria-label="label('API key', 'API key')" type="password" autocomplete="off"></label>
          <label v-if="printer.vendor === 'snapmaker'">{{ label('Token', 'Token') }}<input v-model="printer.token" :aria-label="label('Token', 'Token')" type="password" autocomplete="off"></label>
        </div>
        <div class="gcode-actions">
          <button type="button" :disabled="companionBusy" @click="refreshCompanion">{{ label('Проверить companion', 'Check companion') }}</button>
          <button type="button" :disabled="companionBusy || !companionOnline" @click="discoverPrinters">{{ label('Найти', 'Discover') }}</button>
          <button type="button" :disabled="sendDisabled" @click="sendToPrinter">{{ label('Отправить', 'Send') }}</button>
        </div>
        <ul v-if="discovered.length" class="gcode-discovered">
          <li v-for="(item, index) in discovered" :key="`${item.vendor}-${item.host}-${index}`">
            <button type="button" @click="applyDiscovered(item)">{{ item.vendor }} · {{ item.host }}{{ item.serial ? ` · ${item.serial}` : '' }}</button>
          </li>
        </ul>
      </section>
    </section>
  </details>
</template>

<style scoped>
.gcode-panel{margin:10px 0;border-top:1px solid var(--border);padding-top:8px}
.gcode-panel summary{cursor:pointer;font-weight:600}.gcode-panel p{line-height:1.45}
.gcode-panel label{display:flex;align-items:center;gap:6px;margin:6px 0}.gcode-panel button,.gcode-panel input,.gcode-panel select{font:inherit;background:var(--surface-raised);color:var(--text);border:1px solid var(--border);padding:5px;border-radius:4px}
.gcode-fields{display:grid;grid-template-columns:1fr 1fr;gap:0 8px}.gcode-fields label{display:flex;flex-direction:column;align-items:stretch;min-width:0}.gcode-fields input[type=number],.gcode-fields input[type=text],.gcode-fields input[type=password],.gcode-fields select{width:100%;box-sizing:border-box}
.gcode-actions{display:flex;flex-wrap:wrap;gap:6px;margin-top:8px}.gcode-panel button:disabled{opacity:.5;cursor:default}.gcode-file{flex-direction:column;align-items:stretch!important}.gcode-file input{width:100%;box-sizing:border-box}
.gcode-result{border-top:1px solid var(--border);margin-top:10px;padding-top:6px}.gcode-filename{overflow-wrap:anywhere;font-size:11px}.gcode-canvas{display:block;width:100%;height:auto;border:1px solid var(--border);box-sizing:border-box;border-radius:4px}
.gcode-result input[type=range]{min-width:0;flex:1}.gcode-legend{display:flex;gap:14px;background:#101922;padding:6px;border-radius:4px}.gcode-extrusion{color:#5bcbff}.gcode-travel{color:#e8ad61}
.gcode-stats{display:grid;grid-template-columns:1fr auto;gap:5px 10px}.gcode-stats dt{margin:0}.gcode-stats dd{margin:0;text-align:right;font-variant-numeric:tabular-nums}
.gcode-printer{border-top:1px solid var(--border);margin-top:12px;padding-top:8px}.gcode-printer h3{margin:0 0 6px;font-size:13px}.gcode-hint{font-size:11px}.gcode-discovered{list-style:none;padding:0;margin:8px 0 0}.gcode-discovered button{width:100%;text-align:left}
.gcode-panel [role=alert]{color:var(--danger)}.gcode-panel small{line-height:1.45;display:block}.gcode-note{font-size:11px}
</style>
