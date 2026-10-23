<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { MeshData } from '../core/mesh'
import type { PickHit } from '../services/rendererContracts'
import { SVG_MAX_BYTES, SVG_MAX_FONTS, SVG_MAX_FONT_BYTES, SVG_MAX_TOTAL_FONT_BYTES } from '../services/svgLimits'
import { createSvgWorkerClient } from '../services/svgWorkerClient'
import type { SvgGeometryResult } from '../services/svgWorkerProtocol'
import { svgDraftStore, type SvgDraft } from '../services/svgDraftStore'
import { MAX_WORKSPACE_SOURCE_LENGTH } from '../services/workspaceDocument'

const props = withDefaults(defineProps<{ meshes: MeshData[]; hit: PickHit | null; available: boolean; locale: string; canAppend?: boolean; remainingSource?: number; appendRevision?: string | number }>(), { canAppend: true, remainingSource: MAX_WORKSPACE_SOURCE_LENGTH })
const emit = defineEmits<{ append: [source: string] }>()
const ru = computed(() => props.locale === 'ru')
const canAppend = computed(() => props.canAppend !== false)
const appendNotice = computed(() => ru.value ? 'Для добавления экструзии нужен документ OpenSCAD (.scad).' : 'Adding an extrusion requires an OpenSCAD (.scad) document.')
const text = ref('<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="30mm" viewBox="0 0 40 30"><rect x="2" y="2" width="36" height="26" rx="4" fill="#3b82f6"/></svg>')
const filename = ref('profile.svg')
const height = ref<number | string>(5)
const axis = ref<'x' | 'y' | 'z'>('z')
const dpi = ref<number | string>(96)
const tolerance = ref<number | string>(0.05)
const geometryMode = ref<'vector' | 'silhouette'>('vector')
const rasterSize = ref<number | string>(512)
const alphaThreshold = ref<number | string>(0.5)
const fonts = ref<{ name: string; bytes: Uint8Array }[]>([])
const error = ref('')
const busy = ref(false)
const preview = ref('')
const dimensions = ref('')
const warnings = ref<string[]>([])
const status = ref('')
const persistenceError = ref('')
const opened = ref(false)
const lastExport = ref<{ result: SvgGeometryResult; name: string } | null>(null)
const worker = createSvgWorkerClient()
const drafts = svgDraftStore()
let controller: AbortController | null = null
let activePurpose: 'svg' | 'project' | 'extrude' = 'svg'
let restoring = false, dirty = false
let saveTimer: ReturnType<typeof setTimeout> | undefined
let revision = 0
let disposed = false

function clearPreview() {
  if (preview.value) URL.revokeObjectURL(preview.value)
  preview.value = ''
  dimensions.value = ''
  warnings.value = []
}
watch([text, dpi, tolerance, geometryMode, rasterSize, alphaThreshold, fonts], () => {
  revision++
  cancelOperation(false)
  clearPreview()
  error.value = ''
  status.value = ''
}, { flush: 'sync' })
watch([() => props.meshes, () => props.hit, () => props.available], () => {
  if (controller && activePurpose === 'project') { revision++; cancelOperation(false) }
})
watch(height, () => { if (controller && activePurpose === 'extrude') { revision++; cancelOperation(false) } })
watch(axis, () => { if (controller && activePurpose === 'project') { revision++; cancelOperation(false) } })
watch(() => props.appendRevision, () => { if (controller && activePurpose === 'extrude') { revision++; cancelOperation(false) } })
watch(() => props.canAppend, value => { if (!value && controller && activePurpose === 'extrude') { revision++; cancelOperation(false) } })
function captureDraft(): SvgDraft {
  return { version: 1, text: text.value, filename: filename.value, height: height.value, axis: axis.value, dpi: dpi.value, tolerance: tolerance.value, geometryMode: geometryMode.value, rasterSize: rasterSize.value, alphaThreshold: alphaThreshold.value, fonts: fonts.value.map(font => ({ name: font.name, bytes: font.bytes })), opened: opened.value }
}
function storageNotice(cause: unknown) {
  if (ru.value) {
    if (cause && typeof cause === 'object' && 'code' in cause && cause.code === 'conflict') return 'Черновик SVG изменён в другой вкладке. Скачайте текущий рисунок перед перезагрузкой.'
    return 'Не удалось сохранить SVG в браузере. Скачайте рисунок, прежде чем закрывать страницу.'
  }
  return cause instanceof Error ? cause.message : 'Could not save SVG in this browser. Download the artwork before closing the page.'
}
function persist() {
  clearTimeout(saveTimer)
  if (!dirty) return
  dirty = false
  void drafts.save(captureDraft()).then(() => { if (!disposed) persistenceError.value = '' }, cause => {
    if (!disposed) { dirty = true; persistenceError.value = storageNotice(cause) }
  })
}
watch([text, filename, height, axis, dpi, tolerance, geometryMode, rasterSize, alphaThreshold, fonts, opened], () => {
  if (restoring) return
  dirty = true; clearTimeout(saveTimer); saveTimer = setTimeout(persist, 500)
}, { flush: 'sync' })
onMounted(async () => {
  globalThis.addEventListener?.('pagehide', persist)
  const started = revision
  try {
    const saved = await drafts.load()
    if (!saved || disposed || revision !== started || dirty) return
    restoring = true
    text.value = saved.text; filename.value = saved.filename; height.value = saved.height; axis.value = saved.axis
    dpi.value = saved.dpi; tolerance.value = saved.tolerance; geometryMode.value = saved.geometryMode; rasterSize.value = saved.rasterSize; alphaThreshold.value = saved.alphaThreshold
    fonts.value = saved.fonts; opened.value = saved.opened
    // The persistence watcher is synchronous so restoring cannot enqueue a rewrite.
    restoring = false
  } catch (cause) { if (!disposed) persistenceError.value = storageNotice(cause) }
})
onBeforeUnmount(() => { persist(); disposed = true; revision++; cancelOperation(false); worker.dispose(); clearPreview(); globalThis.removeEventListener?.('pagehide', persist) })
function cancelOperation(notify = true) {
  const active = controller
  controller = null
  active?.abort(); worker.cancel(); busy.value = false
  if (notify) status.value = ru.value ? 'Обработка SVG отменена.' : 'SVG processing cancelled.'
}


function options() {
  return { dpi: Number(dpi.value), tolerance: Number(tolerance.value), fonts: fonts.value.map(font => font.bytes), geometryMode: geometryMode.value, rasterSize: Number(rasterSize.value), alphaThreshold: Number(alphaThreshold.value) }
}
/** An edited source must never receive a result, download, or extrusion from an earlier operation. */
async function run<T>(work: (signal: AbortSignal) => Promise<T>, commit: (value: T) => void, purpose: 'svg' | 'project' | 'extrude' = 'svg') {
  if (busy.value) return
  const started = ++revision, current = new AbortController()
  controller = current; activePurpose = purpose
  busy.value = true
  error.value = ''
  status.value = ''
  let committing = false
  try {
    const result = await work(current.signal)
    if (!disposed && revision === started && controller === current && !current.signal.aborted) { controller = null; committing = true; commit(result) }
  } catch (cause) {
    if (!disposed && (committing || revision === started) && !current.signal.aborted) error.value = cause instanceof Error ? cause.message : String(cause)
  } finally {
    if (!disposed && (controller === current || controller === null)) { controller = null; busy.value = false }
  }
}
function showDiagnostics(result: { widthMm: number; heightMm: number; warnings: string[] }) {
  dimensions.value = `${Number(result.widthMm.toFixed(3))} × ${Number(result.heightMm.toFixed(3))} mm`
  warnings.value = result.warnings
}
function showPreview(result: SvgGeometryResult) {
  clearPreview()
  preview.value = URL.createObjectURL(new Blob([result.svg], { type: 'image/svg+xml' }))
  showDiagnostics(result)
}
function exportName(suffix = '') {
  const stem = filename.value.replace(/\.svg$/i, '').replace(/[^\p{L}\p{N}._ -]/gu, '_') || 'profile'
  return `${stem}${suffix}.svg`
}
function download(svg: string, name: string) {
  const url = URL.createObjectURL(new Blob([svg], { type: 'image/svg+xml' }))
  const link = document.createElement('a')
  link.href = url
  link.download = name
  link.click()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
  status.value = ru.value ? `Скачивание: ${name}` : `Downloading ${name}`
}
function refreshPreview() {
  const source = text.value, config = options()
  return run(signal => worker.run({ kind: 'preview', svg: source, options: config }, { signal }), showPreview)
}
function downloadArtwork() {
  const source = text.value, config = options(), name = exportName()
  return run(signal => worker.run({ kind: 'preview', svg: source, options: config }, { signal }), result => { showPreview(result); download(result.svg, name) })
}
function downloadContours() {
  const source = text.value, config = options(), name = exportName('-contours')
  return run(signal => worker.run({ kind: 'contours', svg: source, options: config }, { signal }), result => { showDiagnostics(result); download(result.svg, name) })
}
function extrude() {
  if (!canAppend.value) return
  const source = text.value, config = options(), depth = Number(height.value), destination = props.appendRevision
  return run(signal => worker.run({ kind: 'extrude', svg: source, options: config, height: depth }, { signal }), result => {
    showDiagnostics(result)
    if (props.appendRevision !== destination) return
    if (!canAppend.value) { error.value = appendNotice.value; return }
    if (typeof result.source !== 'string') throw new Error('SVG extrusion returned no source.')
    if (!Number.isFinite(props.remainingSource) || result.source.length > Math.max(0, props.remainingSource)) {
      throw new Error(ru.value ? 'Экструзия не помещается в текущий документ. Увеличьте допуск или скачайте контуры SVG.' : 'The extrusion exceeds the remaining document space. Increase tolerance or download the SVG contours.')
    }
    emit('append', result.source)
    status.value = ru.value ? 'Экструзия добавлена в модель.' : 'Extrusion added to the model.'
  }, 'extrude')
}
function create(shape: 'rect' | 'circle' | 'artwork' | 'text') {
  const shapes = {
    rect: '<rect x="2" y="2" width="36" height="36" rx="4" fill="#3b82f6"/>',
    circle: '<circle cx="20" cy="20" r="18" fill="#14b8a6"/>',
    artwork: '<defs><linearGradient id="ink"><stop stop-color="#6366f1"/><stop offset="1" stop-color="#ec4899"/></linearGradient><clipPath id="round"><circle cx="20" cy="20" r="18"/></clipPath><path id="tile" d="M0 0h12v12H0z"/></defs><style>.tile{fill:url(#ink);stroke:#312e81;stroke-width:1.2}</style><g clip-path="url(#round)"><rect width="40" height="40" fill="#e0e7ff"/><g class="tile" transform="translate(4 4) rotate(12 16 16)"><use href="#tile"/><use href="#tile" x="18"/><use href="#tile" y="18"/><use href="#tile" x="18" y="18"/></g></g>',
    text: '<text x="20" y="25" text-anchor="middle" font-family="sans-serif" font-size="16" font-weight="bold" fill="#7c3aed">SVG</text>',
  }
  text.value = `<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="40mm" viewBox="0 0 40 40">${shapes[shape]}</svg>`
  filename.value = `${shape === 'rect' ? 'rectangle' : shape}.svg`
  void refreshPreview()
}
async function file(event: Event) {
  const input = event.target as HTMLInputElement, selected = input.files?.[0]
  input.value = ''
  if (!selected) return
  const config = options()
  await run(async signal => {
    if (selected.size > SVG_MAX_BYTES) throw new Error(ru.value ? `Размер SVG не должен превышать ${SVG_MAX_BYTES / 1024 / 1024} МиБ.` : `SVG must not exceed ${SVG_MAX_BYTES / 1024 / 1024} MiB.`)
    const source = await selected.text()
    const result = await worker.run({ kind: 'preview', svg: source, options: config }, { signal })
    return { source, result }
  }, ({ source, result }) => {
    text.value = source
    filename.value = selected.name
    showPreview(result)
  })
}
async function uploadFonts(event: Event) {
  const input = event.target as HTMLInputElement, selected = Array.from(input.files ?? [])
  input.value = ''
  if (!selected.length) return
  const previous = fonts.value, source = text.value, config = options()
  await run(async signal => {
    if (previous.length + selected.length > SVG_MAX_FONTS || selected.some(font => font.size > SVG_MAX_FONT_BYTES) || previous.reduce((sum, font) => sum + font.bytes.length, 0) + selected.reduce((sum, font) => sum + font.size, 0) > SVG_MAX_TOTAL_FONT_BYTES) {
      throw new Error(ru.value ? 'Допустимо до 16 шрифтов, до 4 МиБ каждый и до 8 МиБ суммарно.' : 'Use up to 16 fonts, at most 4 MiB each and 8 MiB in total.')
    }
    const next = [...previous, ...await Promise.all(selected.map(async font => ({ name: font.name, bytes: new Uint8Array(await font.arrayBuffer()) })))]
    const result = await worker.run({ kind: 'preview', svg: source, options: { ...config, fonts: next.map(font => font.bytes) } }, { signal })
    return { next, result }
  }, ({ next, result }) => { fonts.value = next; showPreview(result) })
}
function fromMesh(face: boolean) {
  const hit = props.hit
  if (!props.available || face && !hit) return
  const config = { axis: axis.value, ...(face && hit ? { face: { meshIndex: hit.meshIndex, triangleIndex: hit.triangleIndex } } : {}) }
  const name = face ? 'selected-face.svg' : `model-${axis.value}.svg`
  // A model projection is already in millimeters and does not depend on artwork fonts or DPI.
  const meshes = props.meshes, currentOptions = {}
  return run(signal => worker.run({ kind: 'project', meshes, ...config, options: currentOptions }, { signal }), result => {
    if (props.meshes !== meshes || !props.available || face && props.hit !== hit) return
    lastExport.value = { result, name }
    download(result.svg, name)
  }, 'project')
}
function openExport() {
  const exported = lastExport.value
  if (!exported) return
  text.value = exported.result.svg; filename.value = exported.name; showPreview(exported.result)
}

</script>

<template>
  <details class="svg-panel" :open="opened" @toggle="opened = ($event.currentTarget as HTMLDetailsElement).open">
    <summary>SVG ↔ 3D</summary>
    <div class="svg-content" :aria-busy="busy" role="region" :aria-label="ru ? 'Инструменты SVG' : 'SVG tools'" tabindex="0">
      <p>{{ ru ? 'Откройте SVG, вставьте разметку или начните с примера. Просматривайте рисунок, создавайте объём и сохраняйте SVG.' : 'Open SVG, paste markup or start with an example. Preview artwork, create a solid and save SVG.' }}</p>
      <div class="actions">
        <button :disabled="busy" @click="create('rect')">{{ ru ? 'Прямоугольник' : 'Rectangle' }}</button>
        <button :disabled="busy" @click="create('circle')">{{ ru ? 'Круг' : 'Circle' }}</button>
        <button :disabled="busy" @click="create('artwork')">{{ ru ? 'Стили и обрезка' : 'Styles and clipping' }}</button>
        <button :disabled="busy" @click="create('text')">{{ ru ? 'Текст' : 'Text' }}</button>
        <label>{{ ru ? 'Открыть SVG' : 'Open SVG' }} <input type="file" accept=".svg,image/svg+xml" :disabled="busy" :aria-label="ru ? 'Открыть SVG' : 'Open SVG'" @change="file"></label>
      </div>
      <div class="file-info"><span>{{ filename }}</span><span v-if="dimensions">{{ dimensions }}</span></div>
      <textarea v-model="text" aria-label="SVG" spellcheck="false" />
      <div class="actions">
        <label>DPI <input v-model.number="dpi" type="number" min="0.01" max="100000" step="1" aria-label="DPI" :disabled="busy"></label>
        <label>{{ ru ? 'Точность, мм' : 'Tolerance, mm' }} <input v-model.number="tolerance" type="number" min="0.0001" max="10" step="0.01" :aria-label="ru ? 'Точность, мм' : 'Tolerance, mm'" :disabled="busy"></label>
      </div>
      <p>{{ ru ? 'DPI задаёт размер px; единицы мм, см и дюймы сохраняют физический размер. Меньший допуск даёт более плавные контуры.' : 'DPI sets the size of pixels; mm, cm and inches keep their physical size. A smaller tolerance gives smoother contours.' }}</p>
      <details class="font-options">
        <summary>{{ ru ? 'Шрифты для текста' : 'Fonts for text' }}{{ fonts.length ? ` (${fonts.length})` : '' }}</summary>
        <p>{{ ru ? 'Текст превращается в контуры. Загрузите контурный шрифт TTF, OTF или TTC, указанный в SVG. Цветные глифы предварительно преобразуйте в SVG-контуры.' : 'Text becomes outlines. Load the outline TTF, OTF or TTC font named in the SVG. Convert colored glyphs to SVG paths first.' }}</p>
        <input type="file" accept=".ttf,.otf,.ttc" multiple :disabled="busy" :aria-label="ru ? 'Загрузить шрифты' : 'Load fonts'" @change="uploadFonts">
        <div v-if="fonts.length" class="actions"><span>{{ fonts.map(font => font.name).join(', ') }}</span><button :disabled="busy" @click="fonts = []">{{ ru ? 'Убрать шрифты' : 'Clear fonts' }}</button></div>
      </details>
      <div v-if="preview" class="preview"><img :src="preview" :alt="ru ? 'Предпросмотр рисунка SVG' : 'SVG artwork preview'"></div>
      <p v-else class="preview-hint">{{ ru ? 'Нажмите «Предпросмотр», чтобы увидеть текущий SVG.' : 'Choose Preview to see the current SVG.' }}</p>
      <div class="actions">
        <button :disabled="busy" @click="refreshPreview">{{ ru ? 'Предпросмотр' : 'Preview' }}</button>
        <button :disabled="busy" @click="downloadArtwork">{{ ru ? 'Скачать рисунок SVG' : 'Download artwork SVG' }}</button>
        <button :disabled="busy" @click="downloadContours">{{ ru ? 'Скачать контуры SVG' : 'Download contours SVG' }}</button>
      </div>
      <p>{{ ru ? 'Рисунок сохраняет цвета и эффекты. Контуры — одноцветный профиль в мм для изготовления.' : 'Artwork keeps colors and effects. Contours are a single-color profile in mm for fabrication.' }}</p>
      <div class="actions">
        <label>{{ ru ? 'Профиль для 3D' : '3D profile' }}
          <select v-model="geometryMode" :aria-label="ru ? 'Профиль для 3D' : '3D profile'" :disabled="busy">
            <option value="vector">{{ ru ? 'Векторные контуры' : 'Vector contours' }}</option>
            <option value="silhouette">{{ ru ? 'Видимый силуэт' : 'Visible silhouette' }}</option>
          </select>
        </label>
      </div>
      <template v-if="geometryMode === 'silhouette'">
        <p>{{ ru ? 'Силуэт учитывает маски, фильтры и встроенные изображения. Это приближение по пикселям; разрешение задаёт длину большей стороны.' : 'The silhouette includes masks, filters and embedded images. It is a pixel approximation; resolution sets the longer edge.' }}</p>
        <div class="actions">
          <label>{{ ru ? 'Разрешение, px' : 'Resolution, px' }} <input v-model.number="rasterSize" type="number" min="128" max="2048" step="128" :aria-label="ru ? 'Разрешение, px' : 'Resolution, px'" :disabled="busy"></label>
          <label>{{ ru ? 'Порог непрозрачности' : 'Opacity threshold' }} <input v-model.number="alphaThreshold" type="number" min="0.01" max="1" step="0.05" :aria-label="ru ? 'Порог непрозрачности' : 'Opacity threshold'" :disabled="busy"></label>
        </div>
      </template>
      <div class="actions">
        <label>{{ ru ? 'Высота, мм' : 'Height, mm' }} <input v-model.number="height" type="number" min="0.01" max="100000" step="1" :aria-label="ru ? 'Высота, мм' : 'Height, mm'" :disabled="busy"></label>
        <button :disabled="busy || !canAppend" @click="extrude">{{ ru ? 'Добавить экструзию в модель' : 'Add extrusion to model' }}</button>
      </div>
      <p v-if="!canAppend">{{ appendNotice }}</p>
      <div class="actions">
        <label>{{ ru ? 'Ось проекции' : 'Projection axis' }} <select v-model="axis" :aria-label="ru ? 'Ось проекции' : 'Projection axis'" :disabled="busy"><option>x</option><option>y</option><option>z</option></select></label>
        <button :disabled="busy || !available" @click="fromMesh(false)">{{ ru ? 'Проекция модели → SVG' : 'Model projection → SVG' }}</button>
        <button :disabled="busy || !available || !hit" @click="fromMesh(true)">{{ ru ? 'Выбранная грань → SVG' : 'Selected face → SVG' }}</button>
      </div>
      <p>{{ ru ? 'Проекция включает все тела, в том числе скрытые, без учёта режима сечения. Грань должна быть плоской; кривые поверхности проецируются без развёртки.' : 'Projection includes every body, including hidden bodies, and ignores the section view. The selected face must be planar; curved surfaces are projected, not unwrapped.' }}</p>
      <div v-if="busy" class="actions"><p role="status">{{ ru ? 'Обработка SVG…' : 'Processing SVG…' }}</p><button @click="cancelOperation()">{{ ru ? 'Отменить' : 'Cancel' }}</button></div>
      <div v-if="lastExport" class="actions"><span>{{ lastExport.name }}</span><button :disabled="busy" @click="openExport">{{ ru ? 'Открыть экспортированный SVG в редакторе' : 'Open exported SVG in editor' }}</button></div>
      <p v-if="persistenceError" role="status">{{ persistenceError }}</p>
      <ul v-if="warnings.length" class="warnings"><li v-for="warning in warnings" :key="warning">{{ warning }}</li></ul>
      <p v-if="status" role="status">{{ status }}</p>
      <p v-if="error" role="alert">{{ error }}</p>
    </div>
  </details>
</template>

<style scoped>
.svg-panel{border:1px solid var(--border);border-radius:8px;margin:0;flex:0 0 auto;background:var(--surface);font-size:12px}.svg-panel summary{padding:9px;cursor:pointer}.svg-content{min-height:0;max-height:min(55dvh,600px);overflow:auto;overscroll-behavior-y:contain;padding:0 10px 10px;display:grid;gap:8px}.svg-content:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}.svg-content p{margin:0;opacity:.8;line-height:1.45}.actions{display:flex;gap:6px;flex-wrap:wrap;align-items:center}.actions label{display:flex;gap:5px;align-items:center;flex-wrap:wrap}.file-info{display:flex;gap:8px;justify-content:space-between;overflow-wrap:anywhere}.file-info span:last-child{font-variant-numeric:tabular-nums}textarea{width:100%;min-height:100px;resize:vertical;box-sizing:border-box;font:11px/1.4 monospace}.preview{height:180px;display:flex;align-items:center;justify-content:center;overflow:hidden;border:1px solid var(--border);border-radius:4px;background-color:white;background-image:linear-gradient(45deg,#eee 25%,transparent 25%),linear-gradient(-45deg,#eee 25%,transparent 25%),linear-gradient(45deg,transparent 75%,#eee 75%),linear-gradient(-45deg,transparent 75%,#eee 75%);background-size:16px 16px;background-position:0 0,0 8px,8px -8px,-8px 0}.preview img{display:block;min-width:0;min-height:0;width:100%;height:100%;object-fit:contain}.preview-hint{padding:12px;border:1px dashed var(--border);border-radius:4px}input[type=number]{width:72px}input[type=file]{max-width:100%;width:195px}.font-options{border:1px solid var(--border);border-radius:4px;padding:0 7px 7px}.font-options summary{padding-left:0}.font-options p{margin-bottom:6px}.font-options .actions{margin-top:6px;overflow-wrap:anywhere}button,select,input,textarea{color:var(--text);background:var(--surface-raised);border:1px solid var(--border);border-radius:4px;padding:5px}button:disabled,input:disabled,select:disabled{opacity:.4}button{cursor:pointer}button:disabled{cursor:default}[role=alert]{color:#d65b4a}.warnings{margin:0;padding-left:18px;color:var(--text);line-height:1.45}
</style>
