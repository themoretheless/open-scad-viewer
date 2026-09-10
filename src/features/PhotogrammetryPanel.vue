<script setup lang="ts">
import {MAX_CALIBRATION_FILE_BYTES, parsePhotoCalibration, photoCalibrationExampleJson, type PhotoCalibrationDocument} from '../services/photogrammetry/calibration'
import {computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch} from 'vue'
import {WebGPURenderer} from '../services/webgpuRenderer'
import {decodePhoto, PhotoCollection, PhotoInputError, validPhotoFocal, type ImportedPhoto} from '../services/photogrammetry/input'
import {downloadPhoto, photoCanAppend, PhotoExportError, photoPly, photoScadSource} from '../services/photogrammetry/export'
import {PhotoPreview, type PhotoPreviewMode} from '../services/photogrammetry/preview'
import {compilePhotogrammetryKernel} from '../services/photogrammetry/module'
import type {PhotoDensePreset, PhotoDiagnostics, PhotoPixels, PhotoReconstruction, PhotoSurface} from '../services/photogrammetry/kernel'
import type {PhotoTimings, PhotoWorkerEvent, PhotoWorkerRequest} from '../services/photogrammetry/workerProtocol'
import {photoRegistrationReason, photoReportJson, type PhotoReportInput} from '../services/photogrammetry/report'
import {MAX_WORKSPACE_SOURCE_LENGTH} from '../services/workspaceDocument'

const props = defineProps<{locale: 'ru' | 'en'; canAppend: boolean; remainingSource: number}>()
const emit = defineEmits<{append: [source: string]}>()
const ru = computed(() => props.locale === 'ru')
const collection = new PhotoCollection()
const photos = shallowRef<readonly ImportedPhoto[]>([])
const calibrationDocument = shallowRef<PhotoCalibrationDocument | null>(null)
const calibrationFiles = ref<HTMLInputElement | null>(null)
const allCalibration = ref('')
const calibrationGroups = computed(() => calibrationDocument.value?.groups ?? [])
const busy = ref(false), importing = ref(false), message = ref(''), warning = ref(''), stage = ref('')
const sparse = shallowRef<PhotoReconstruction | null>(null)
const surface = shallowRef<PhotoSurface | null>(null)
const result = computed(() => surface.value ?? sparse.value)
const failureDiagnostics = shallowRef<PhotoDiagnostics | null>(null)
const diagnostics = computed(() => sparse.value?.diagnostics ?? failureDiagnostics.value)
const imageReports = computed(() => new Map(diagnostics.value?.images.map(image => [image.image, image])))
const timings = shallowRef<PhotoTimings | null>(null)
const runInputs = shallowRef<PhotoReportInput[]>([])
let runSettings = {dense: true, resolution: 128, maxImageSide: 960, densePreset: 'baseline' as PhotoDensePreset}
const densePreset = ref<PhotoDensePreset>('baseline')
const dense = ref(true), resolution = ref(128), widthMm = ref<number | null>(null)
watch(densePreset, preset => { if (preset === 'dual-scale-volume') resolution.value = 128 })
const validWidth = computed(() => typeof widthMm.value === 'number' && Number.isFinite(widthMm.value) && widthMm.value > 0)
const validFocals = computed(() => photos.value.every(photo => validPhotoFocal(photo.equivalent)))
const solidCompatible = computed(() => {
  const candidate = surface.value?.documentSurface ?? surface.value
  return !!candidate && photoCanAppend(candidate)
})
const previewMode = ref<PhotoPreviewMode>('points')
const panel = ref<HTMLDetailsElement | null>(null)
const canvas = ref<HTMLCanvasElement | null>(null)
const viewportError = ref(''), files = ref<HTMLInputElement | null>(null)
let worker: Worker | null = null
let decodeAbort: AbortController | null = null
let renderer: WebGPURenderer | null = null
let preview: PhotoPreview | null = null
let previewResult: PhotoSurface | null = null
let generation = 0, viewGeneration = 0, disposed = false

const status = computed(() => ({
  decode: ru.value ? 'Подготовка фотографий' : 'Preparing photos',
  calibration: ru.value ? 'Исправление оптики по измеренной калибровке' : 'Applying measured calibration',
  cameras: ru.value ? 'Поиск деталей и восстановление камер' : 'Features and camera reconstruction',
  depth: ru.value ? 'Восстановление глубины и поверхности' : 'Depth and surface reconstruction',
}[stage.value] ?? stage.value))

onMounted(() => {
  if (location.hash === '#photogrammetry' && panel.value) {
    panel.value.open = true
    panel.value.scrollIntoView({block: 'start'})
  }
})

function inputMessage(error: unknown): string {
  if (ru.value && error instanceof PhotoInputError) {
    if (error.code === 'limit') return 'В этой версии можно добавить до 24 фотографий.'
    if (error.code === 'size') return `${error.filename}: файл больше 40 МБ`
    return 'Эквивалентное фокусное расстояние должно быть от 8 до 1000 мм.'
  }
  return error instanceof Error ? error.message : String(error)
}

function fitView() { renderer?.fitView() }
function clearResults() {
  viewportError.value = ''
  sparse.value = null
  surface.value = null
  failureDiagnostics.value = null
  timings.value = null
  runInputs.value = []
  message.value = ''
  warning.value = ''
  widthMm.value = null
  renderer?.destroy()
  renderer = null
  preview = null
  previewResult = null
  viewGeneration++
}

async function add(event: Event) {
  const input = event.target as HTMLInputElement
  const selected = [...(input.files ?? [])]
  input.value = ''
  if (busy.value || importing.value || !selected.length) return
  importing.value = true
  message.value = ''
  try {
    await collection.add(selected)
    if (disposed) return
    clearResults()
    photos.value = collection.entries
  } catch (error) {
    if (!disposed) message.value = inputMessage(error)
  } finally {
    importing.value = false
  }
}

function remove(index: number) {
  if (busy.value || importing.value) return
  collection.remove(index)
  photos.value = collection.entries
  clearResults()
}

function updateFocal(index: number, event: Event) {
  if (busy.value || importing.value) return
  const input = event.target as HTMLInputElement
  try {
    collection.setFocal(index, input.valueAsNumber)
    photos.value = collection.entries
    clearResults()
  } catch (error) {
    input.value = String(photos.value[index]?.equivalent ?? 50)
    message.value = inputMessage(error)
  }
}

function updateCalibration(index: number, event: Event) {
  if (busy.value || importing.value) return
  const id = (event.target as HTMLSelectElement).value
  if (id && !calibrationGroups.value.some(group => group.id === id)) return
  collection.setCalibration(index, id || undefined)
  photos.value = collection.entries
  clearResults()
}
function applyCalibrationToAll() {
  if (busy.value || importing.value) return
  const id = allCalibration.value
  if (id && !calibrationGroups.value.some(group => group.id === id)) return
  photos.value.forEach((_, index) => collection.setCalibration(index, id || undefined))
  photos.value = collection.entries
  clearResults()
}
async function importCalibration(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file || busy.value || importing.value) return
  importing.value = true
  try {
    if (file.size > MAX_CALIBRATION_FILE_BYTES) throw new Error(ru.value ? 'Файл калибровки больше 128 КБ.' : 'Calibration file exceeds 128 KiB.')
    const parsed = parsePhotoCalibration(await file.text())
    if (disposed) return
    // Replacing a manifest resets assignments; an id alone cannot identify unchanged measurements.
    photos.value.forEach((_, index) => collection.setCalibration(index))
    photos.value = collection.entries
    calibrationDocument.value = parsed
    allCalibration.value = ''
    clearResults()
  } catch (error) {
    if (!disposed) message.value = inputMessage(error)
  } finally { importing.value = false }
}
function saveCalibrationExample() {
  downloadPhoto('photo-calibration-example.json', 'application/json', photoCalibrationExampleJson())
}

function releaseWorker() {
  worker?.terminate()
  worker = null
  decodeAbort?.abort()
  decodeAbort = null
}
function finish() {
  busy.value = false
  stage.value = ''
  releaseWorker()
}
function stop() {
  generation++
  finish()
  warning.value = ru.value ? 'Обработка остановлена. Уже полученный результат сохранён.' : 'Stopped. Existing results remain available.'
}

async function run() {
  if (busy.value || importing.value || photos.value.length < 2 || !validFocals.value) return
  clearResults()
  busy.value = true
  stage.value = 'decode'
  runSettings = {dense: dense.value, resolution: Number(resolution.value), maxImageSide: 960,
    densePreset: densePreset.value}
  const current = ++generation
  const abort = new AbortController()
  decodeAbort = abort
  try {
    // Compilation overlaps decoding; the compiled module is cloned into the disposable Worker.
    const modulePromise = compilePhotogrammetryKernel()
    const calibrations = photos.value.map(photo => {
      const calibration = photo.calibrationGroupId
        ? calibrationGroups.value.find(group => group.id === photo.calibrationGroupId) : undefined
      if (photo.calibrationGroupId && !calibration) throw new Error('Assigned calibration group is missing')
      return calibration
    })
    const images: PhotoPixels[] = new Array(photos.value.length)
    let nextDecode = 0
    const decodeLane = async () => {
      while (nextDecode < photos.value.length) {
        const image = nextDecode++
        const photo = photos.value[image]!
        const pixels = await decodePhoto(photo.file, photo.equivalent, runSettings.maxImageSide, abort.signal, calibrations[image])
        if (current !== generation || disposed) return
        images[image] = pixels
        runInputs.value = [...runInputs.value, {image, name: photo.file.name, bytes: photo.file.size,
          equivalent: photo.equivalent, width: pixels.width, height: pixels.height, focalPixels: pixels.focal,
          ...(pixels.calibration ? {calibration: pixels.calibration} : {})}]
          .sort((a, b) => a.image - b.image)
      }
    }
    await Promise.all(Array.from({length: Math.min(4, images.length)}, decodeLane))
    if (current !== generation || disposed) return
    const kernelModule = await modulePromise
    if (current !== generation || disposed) return
    const activeWorker = new Worker(new URL('../workers/photogrammetry.worker.ts', import.meta.url), {type: 'module'})
    worker = activeWorker
    activeWorker.onerror = () => {
      if (current !== generation || disposed) return
      message.value = ru.value ? 'Ошибка вычислительного процесса. Попробуйте меньше снимков.' : 'Compute worker failed. Try fewer photos.'
      finish()
    }
    activeWorker.onmessage = (event: MessageEvent<PhotoWorkerEvent>) => {
      if (current !== generation || disposed) return
      const data = event.data
      if (data.type === 'stage') stage.value = data.stage
      if (data.type === 'sparse') sparse.value = data.result
      if (data.type === 'surface') surface.value = data.result
      if (data.type === 'warning') warning.value = data.message
      if (data.type === 'error') {
        message.value = data.message
        failureDiagnostics.value = data.diagnostics ?? null
      }
      if (data.type === 'done') timings.value = data.timings
      if (data.type === 'done' || data.type === 'error') finish()
    }
    const request: PhotoWorkerRequest = {module: kernelModule, images, dense: runSettings.dense, resolution: runSettings.resolution, densePreset: runSettings.densePreset, gpu: runSettings.densePreset === 'baseline'}
    activeWorker.postMessage(request, images.map(image => image.rgb.buffer))
  } catch (error) {
    if (current === generation && !disposed) {
      message.value = inputMessage(error)
      finish()
    }
  }
}

watch([result, previewMode], async ([value]) => {
  const version = ++viewGeneration
  if (!value) return
  await nextTick()
  if (!canvas.value || disposed || version !== viewGeneration) return
  const view = renderer ?? new WebGPURenderer()
  try {
    if (!renderer) {
      const ok = await view.init(canvas.value)
      if (disposed || version !== viewGeneration) { view.destroy(); return }
      if (!ok) throw new Error('WebGPU initialization failed')
      renderer = view
    }
    if (previewResult !== value) {
      preview = new PhotoPreview(value, sparse.value?.cameras[0])
      previewResult = value
    }
    view.setMeshes(preview!.meshes(previewMode.value))
    view.setGridVisible(false)
    const frame = preview!.frame(canvas.value.clientWidth / canvas.value.clientHeight)
    if (frame) {
      view.setPerspectiveFieldOfView(frame.fovY)
      view.restoreCameraState(frame.camera)
    } else view.fitView()
  } catch {
    view.destroy()
    if (renderer === view) renderer = null
    if (!disposed && version === viewGeneration) viewportError.value = ru.value
      ? 'Просмотр WebGPU недоступен. Результат можно скачать.'
      : 'WebGPU preview unavailable. Download remains available.'
  }
})

function save() {
  if (result.value) downloadPhoto(surface.value ? 'photo-surface.ply' : 'photo-cloud.ply', 'application/octet-stream', photoPly(result.value))
}
function saveReport() {
  downloadPhoto('photo-reconstruction-report.json', 'application/json', photoReportJson({
    settings: runSettings, inputs: runInputs.value, sparse: diagnostics.value,
    dense: surface.value?.denseDiagnostics ?? null, timings: timings.value,
  }))
}
function append() {
  if (!props.canAppend || !surface.value || !validWidth.value || busy.value) return
  try {
    emit('append', photoScadSource(surface.value, widthMm.value!, Math.min(MAX_WORKSPACE_SOURCE_LENGTH, props.remainingSource)))
  } catch (error) {
    message.value = ru.value && error instanceof PhotoExportError && error.code === 'budget'
      ? 'Поверхность слишком велика для текста документа. Скачайте PLY.' : inputMessage(error)
  }
}

onBeforeUnmount(() => {
  disposed = true
  generation++
  viewGeneration++
  releaseWorker()
  renderer?.destroy()
  collection.dispose()
})
</script>
<template>
<details id="photogrammetry" ref="panel" class="photo-panel">
 <summary>{{ru?'3D по фотографиям':'3D from photos'}} <span>Rust</span></summary>
 <div class="photo-content">
  <p>{{ru?'Неподвижный предмет, резкие перекрывающиеся ракурсы. Начните с 6–12 соседних снимков.':'A stationary object and sharp overlapping views. Start with 6–12 neighboring photos.'}}</p>
  <input ref="files" type="file" accept="image/jpeg,image/png,image/webp" multiple hidden :disabled="busy||importing" @change="add">
  <button type="button" :disabled="busy||importing" @click="files?.click()">{{ru?'Добавить фотографии':'Add photos'}} · {{photos.length}} / 24</button>
  <p class="muted">{{ru?'Фокусное расстояние — эквивалент для кадра 35 мм. Значение из EXIF является подсказкой; проверьте его для каждого объектива.':'Focal length is the 35 mm equivalent. EXIF is a hint; verify it for each lens.'}}</p>
  <div class="photo-calibration">
   <input ref="calibrationFiles" type="file" accept="application/json,.json" hidden :disabled="busy||importing" @change="importCalibration">
   <div class="photo-actions">
    <button type="button" :disabled="busy||importing" @click="calibrationFiles?.click()">{{ru?'Импорт измеренной калибровки JSON':'Import measured calibration JSON'}}</button>
    <button type="button" @click="saveCalibrationExample">{{ru?'Шаблон JSON':'JSON template'}}</button>
   </div>
   <p class="muted">{{ru?'Необязательно. Используйте измерения для своего объектива и режима съёмки. Размеры относятся к исходному снимку после поворота; исправление выполняется перед восстановлением. Данные шаблона синтетические, не для Canon или iPhone.':'Optional. Use measurements for your lens and capture settings. Dimensions refer to the original photo after orientation; correction runs before reconstruction. Template values are synthetic, not Canon or iPhone profiles.'}}</p>
   <div v-if="calibrationGroups.length" class="photo-actions">
    <select v-model="allCalibration" :disabled="busy||importing" :aria-label="ru?'Калибровка для всех снимков':'Calibration for all photos'"><option value="">{{ru?'Без измеренной калибровки':'No measured calibration'}}</option><option v-for="group in calibrationGroups" :key="group.id" :value="group.id">{{group.label}}</option></select>
    <button type="button" :disabled="busy||importing||!photos.length" @click="applyCalibrationToAll">{{ru?'Применить ко всем':'Apply to all'}}</button>
   </div>
  </div>
  <div class="photo-list">
   <article v-for="(p,i) in photos" :key="p.url"><img :src="p.url" :alt="p.file.name"><div><strong :title="p.file.name">{{p.file.name}}</strong><label>{{ru?'Экв. мм':'Equiv. mm'}} <input type="number" min="8" max="1000" :value="p.equivalent" :disabled="busy||importing||!!p.calibrationGroupId" :aria-label="`${p.file.name} focal equivalent`" @change="updateFocal(i,$event)"></label><label v-if="calibrationGroups.length" class="photo-calibration-assignment">{{ru?'Калибровка':'Calibration'}} <select :value="p.calibrationGroupId??''" :disabled="busy||importing" :aria-label="`${p.file.name} calibration`" @change="updateCalibration(i,$event)"><option value="">{{ru?'По фокусному расстоянию':'Focal hint'}}</option><option v-for="group in calibrationGroups" :key="group.id" :value="group.id">{{group.label}}</option></select></label><small>{{p.calibrationGroupId?(ru?'Измеренная калибровка':'Measured calibration'):p.hint?'EXIF':ru?'Проверьте значение':'Verify value'}}</small><small v-if="imageReports.get(i)&&!imageReports.get(i)!.registered">{{photoRegistrationReason(imageReports.get(i)!.reason,ru)}}</small><small v-else-if="sparse&&!sparse.cameras.some(c=>c.image===i)">{{ru?'Вне модели':'Not registered'}}</small></div><button type="button" :disabled="busy||importing" :aria-label="`${ru?'Удалить':'Remove'} ${p.file.name}`" @click="remove(i)">×</button></article>
  </div>
  <div class="photo-actions"><label><input v-model="dense" type="checkbox" :disabled="busy">{{ru?'Строить поверхность':'Build surface'}}</label><select v-model="resolution" :disabled="busy||!dense" :aria-label="ru?'Детализация':'Detail'"><option :value="128">{{ru?'Черновик':'Draft'}}</option><option :value="192" :disabled="densePreset==='dual-scale-volume'">{{ru?'Больше деталей':'More detail'}}</option></select><button v-if="!busy" type="button" :disabled="photos.length<2||importing||!validFocals" @click="run">{{ru?'Восстановить 3D':'Reconstruct 3D'}}</button><button v-else type="button" @click="stop">{{ru?'Остановить':'Stop'}}</button></div>
  <label>{{ru?'Режим поверхности':'Surface mode'}} <select v-model="densePreset" :disabled="busy||!dense"><option value="baseline">{{ru?'Обычный':'Standard'}}</option><option value="slanted-plane">{{ru?'Наклонные поверхности · эксперимент':'Slanted surfaces · experimental'}}</option><option value="dual-scale-volume">{{ru?'Общая поверхность · эксперимент':'Shared surface · experimental'}}</option></select></label>
  <p v-if="densePreset!=='baseline'&&dense" class="muted">{{ru?'Экспериментальный режим: обработка может занять больше времени; возможны ошибки на границах поверхности.':'Experimental mode: processing may take longer; errors can occur along surface boundaries.'}}</p>
  <p v-if="busy" role="status">{{status}}…</p><p v-if="message" class="photo-error" role="alert">{{message}}</p><p v-if="warning" role="status">{{warning}}</p>
  <template v-if="result"><p class="photo-stats">{{ru?'Связано снимков':'Registered photos'}}: {{sparse?.cameras.length}} / {{sparse?.inputImages}} · {{(result.positions.length/3).toLocaleString()}} {{ru?'точек':'points'}} · {{(result.triangles.length/3).toLocaleString()}} {{ru?'треугольников':'triangles'}}</p><p v-if="sparse" class="muted">{{ru?'Ошибка обратной проекции':'Reprojection error'}}: {{sparse.reprojectionRmse.toFixed(2)}} px. {{ru?'Это не оценка точности в миллиметрах.':'This does not measure accuracy in millimetres.'}}</p><select v-model="previewMode" :aria-label="ru?'Вид результата':'Result view'"><option value="points">{{ru?'Цветное облако · до 6 000 точек в просмотре':'Colored cloud · up to 6,000 preview points'}}</option><option value="surface" :disabled="!surface?.triangles.length">{{ru?'Поверхность без текстуры':'Untextured surface'}}</option></select><canvas ref="canvas" class="photo-viewport" :aria-label="ru?'Реконструкция 3D':'3D reconstruction'"></canvas><p v-if="viewportError">{{viewportError}}</p><div class="photo-actions"><button type="button" @click="fitView">{{ru?'Показать целиком':'Fit view'}}</button><button type="button" @click="save">{{ru?'Скачать цветной PLY':'Download colored PLY'}}</button></div><p class="muted">{{ru?'Цветные точки и поверхность доступны отдельно. PLY содержит все восстановленные точки. Невидимые стороны не восстановлены, масштаб относительный.':'Colored points and surface are separate views. PLY contains all reconstructed points. Hidden sides are not reconstructed; scale is relative.'}}</p><p v-if="surface?.triangles.length&&!solidCompatible" class="muted">{{ru?'Поверхность незамкнута или имеет некорректные соединения. Добавление в твердотельный CAD недоступно; используйте просмотр и PLY.':'Surface is open or has invalid connections. Solid CAD insertion is unavailable; use the preview and PLY export.'}}</p><template v-if="surface?.triangles.length&&solidCompatible"><label>{{ru?'Известная полная ширина результата по X, мм':'Known full width of result along X, mm'}} <input v-model.number="widthMm" type="number" min="0.001" step="any"></label><button type="button" :disabled="!canAppend||!validWidth||busy" @click="append">{{ru?'Добавить упрощённую поверхность':'Add simplified surface'}}</button></template></template>
  <details v-if="diagnostics||surface?.denseDiagnostics||runInputs.length" class="photo-diagnostics">
   <summary>{{ru?'Отчёт обработки':'Processing report'}}</summary>
   <template v-if="diagnostics?.calibrations?.some(entry=>entry.mode==='measured-brown')">
    <p class="muted">{{ru?'Оптика исправлена по импортированным измерениям. Края с недопустимыми координатами исключены сужением поля зрения; автоматическая калибровка не выполнялась.':'Optics corrected using imported measurements. Invalid borders are excluded by narrowing the field of view; no automatic calibration was performed.'}}</p>
    <ul><li v-for="entry in (diagnostics?.calibrations??[]).filter(entry=>entry.mode==='measured-brown')" :key="entry.image">{{photos[entry.image]?.file.name}} — {{entry.group?.label}}; {{ru?'масштаб поля зрения':'view zoom'}} ×{{entry.zoom?.toFixed(3)}}; {{entry.group?.source}}</li></ul>
   </template>
   <template v-if="diagnostics">
    <p>{{ru?'Проверено начальных пар':'Seed pairs tested'}}: {{diagnostics.seedPairsTested}}. {{ru?'Вычислено сопоставлений пар':'Computed pair matches'}}: {{diagnostics.computedPairs}} / {{diagnostics.matchingRequests}} {{ru?'запросов (повторные берутся из кэша)':'requests (repeated requests use the cache)'}}.</p>
    <ul><li v-for="entry in diagnostics.images" :key="entry.image">{{photos[entry.image]?.file.name??`#${entry.image+1}`}} — {{photoRegistrationReason(entry.reason,ru)}}; {{entry.features}} {{ru?'деталей':'features'}}, {{entry.acceptedObservations}} {{ru?'наблюдений':'observations'}}.</li></ul>
    <p v-if="diagnostics.bundleRuns.length" class="muted">{{ru?'Совместная оптимизация: значение функции ошибки до → после. Это не оценка точности в миллиметрах.':'Joint optimization: objective value before → after. This does not measure millimetre accuracy.'}}</p>
    <ul><li v-for="(bundle,i) in diagnostics.bundleRuns" :key="i">#{{i+1}}: {{bundle.initialCost.toFixed(2)}} → {{bundle.finalCost.toFixed(2)}}; {{bundle.acceptedSteps}} / {{bundle.iterations}} {{ru?'принятых шагов':'accepted steps'}}, {{bundle.observations}} {{ru?'наблюдений':'observations'}}.</li></ul>
    <p v-for="(entry,i) in diagnostics.warnings" :key="i" class="muted">{{entry}}</p>
   </template>
   <p v-if="surface?.denseDiagnostics">{{ru?'Карты глубины':'Depth maps'}}: {{surface.denseDiagnostics.estimatedMaps}}. {{ru?'Согласованные измерения':'Consistent samples'}}: {{surface.denseDiagnostics.consistentSamples}} / {{surface.denseDiagnostics.photometricSamples}}. {{ru?'Объединено измерений':'Merged samples'}}: {{surface.denseDiagnostics.fusedSamples}}.</p>
   <p v-if="surface?.denseDiagnostics?.preset">{{ru?'Режим поверхности':'Surface mode'}}: {{surface.denseDiagnostics.preset==='dual-scale-volume'?(ru?'общая поверхность, эксперимент':'shared surface, experimental'):surface.denseDiagnostics.preset==='slanted-plane'?(ru?'наклонные поверхности, эксперимент':'slanted surfaces, experimental'):(ru?'обычный':'standard')}}.</p>
   <p v-if="surface?.denseDiagnostics?.evaluatedHypotheses!==undefined" class="muted">{{ru?'Проверено вариантов глубины':'Depth hypotheses evaluated'}}: {{surface.denseDiagnostics.evaluatedHypotheses.toLocaleString()}} · {{ru?'сопоставлений участков':'source patches'}}: {{surface.denseDiagnostics.evaluatedSourcePatches?.toLocaleString()}} · {{ru?'выборок пикселей':'pixel samples'}}: {{surface.denseDiagnostics.sampledSourcePixels?.toLocaleString()}}.</p>
   <p v-if="timings?.preparationMs!==undefined">{{ru?'Передача и исправление снимков':'Upload and photo correction'}}: {{(timings.preparationMs/1000).toFixed(2)}} s.</p>
   <p v-if="timings">{{ru?'Камеры':'Cameras'}}: {{(timings.sparseMs/1000).toFixed(2)}} s · {{ru?'Глубина и поверхность':'Depth and surface'}}: {{(timings.denseMs/1000).toFixed(2)}} s.</p>
   <button type="button" @click="saveReport">{{ru?'Скачать отчёт JSON':'Download JSON report'}}</button>
  </details>
  <p class="muted">{{ru?'Экспериментальная реконструкция: возможны пропуски, фон и разрывы. Поддерживаются JPG, PNG и WebP; обработка остаётся на устройстве.':'Experimental reconstruction: gaps, background, and discontinuities are possible. JPG, PNG, and WebP; processing stays on device.'}}</p>
 </div>
</details>
</template>
<style scoped>
.photo-panel{border-top:1px solid var(--border,#333);font-size:12px}.photo-panel summary{cursor:pointer;padding:12px;font-weight:600;display:list-item}.photo-panel summary span{float:right;font-size:10px;opacity:.6}.photo-content{padding:0 12px 12px;display:grid;gap:10px}.photo-content p{margin:0;line-height:1.5}.muted{opacity:.65;font-size:11px}.photo-content button,.photo-content select,.photo-content input[type=number]{font:inherit;color:inherit;background:transparent;border:1px solid var(--border,#555);border-radius:5px;padding:6px}.photo-content button{cursor:pointer}.photo-content button:disabled{opacity:.4;cursor:default}.photo-list{display:grid;gap:6px;max-height:240px;overflow:auto}.photo-list article{display:flex;gap:8px;align-items:center}.photo-list img{width:64px;height:48px;object-fit:cover;border-radius:4px}.photo-list article>div{flex:1;min-width:0}.photo-list strong{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:10px}.photo-list input{width:65px;padding:3px!important}.photo-list small{opacity:.6;margin-left:5px;font-size:9px}.photo-actions{display:flex;gap:8px;align-items:center;flex-wrap:wrap}.photo-viewport{width:100%;height:320px;display:block;touch-action:none;background:#15181c}.photo-error{color:#e99}.photo-stats{font-weight:600}.photo-diagnostics{display:grid;gap:8px;overflow-wrap:anywhere}.photo-diagnostics summary{padding:4px 0}.photo-diagnostics ul{margin:0;padding-left:18px;display:grid;gap:4px}
.photo-calibration{display:grid;gap:8px}.photo-calibration select{max-width:100%}.photo-calibration-assignment{display:block;margin-top:4px}.photo-calibration-assignment select{max-width:100%;font-size:10px}
</style>
