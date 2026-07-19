<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import CommandPalette from './components/CommandPalette.vue'
import CustomizerPanel from './components/CustomizerPanel.vue'
import InspectPanel from './components/InspectPanel.vue'
import SceneOutliner from './components/SceneOutliner.vue'
import ViewCube from './components/ViewCube.vue'
import type {
  DistanceMeasurement as PanelMeasurement,
  InspectSelection,
  SceneMeshRow,
  SectionAxis,
  SourceProvenanceRow,
} from './components/cadPanels.types'
import { EXAMPLES } from './data/examples'
import type { CameraState } from './services/cameraHistory'
import type { GeometryResponse } from './services/geometryWorkerProtocol'
import { isPaletteCommandEnabled, type PaletteCommand } from './services/commandSearch'
import { buildBinaryStl, buildObj } from './services/meshExport'
import { inspectMesh, matchMeshesByProvenance } from './services/meshInspection'
import type { GeometryQuality, MeshData } from './services/openscadParser'
import { extractCustomizerParameters, replaceCustomizerValue, type CustomizerValue } from './services/scadCustomizer'
import {
  WebGPURenderer,
  type DisplayMode,
  type DistanceMeasurement,
  type PickHit,
  type ProjectionMode,
  type SelectionMode,
  type StandardView,
} from './services/webgpuRenderer'

type Language = 'ru' | 'en'

const L: Record<Language, Record<string, string>> = {
  ru: {
    title: 'OpenSCAD Viewer',
    render: 'Собрать', auto: 'Авто', examples: 'Пример',
    basic: 'Примитивы', csg: 'Настоящий CSG', house: 'Дом с модулями', tower: 'Параметрическая башня',
    open: 'Открыть', save: 'Сохранить', share: 'Поделиться',
    exportStl: 'Экспорт STL', exportObj: 'Экспорт OBJ', parameters: 'Параметры',
    noParameters: 'Добавьте верхнеуровневые переменные; диапазон слайдера: // [min:step:max]',
    openFile: 'Открыть файл OpenSCAD', saveFile: 'Сохранить исходник OpenSCAD', shareFile: 'Скопировать ссылку на модель',
    meshes: 'Объекты', triangles: 'Треугольники', volume: 'Объём', area: 'Площадь', time: 'Сборка',
    hint: 'Клик: выбрать · повторный клик: глубже · ЛКМ: вращение · ПКМ/Shift: панорама · колесо: масштаб · F: фокус',
    noGpu: 'WebGPU недоступен. Откройте приложение в актуальном Chrome, Edge, Firefox или Safari.',
    theme: 'Тема', darkTheme: 'Включить тёмную тему', lightTheme: 'Включить светлую тему',
    language: 'Переключить язык', editor: 'Редактор OpenSCAD', viewport: 'Трёхмерная сцена',
    fit: 'Вписать', reset: 'Сбросить вид', previousView: 'Предыдущий вид', perspective: 'Перспектива', orthographic: 'Ортографическая',
    grid: 'Сетка', view: 'Вид', iso: 'Изометрия', front: 'Спереди', back: 'Сзади',
    left: 'Слева', right: 'Справа', top: 'Сверху', bottom: 'Снизу',
    compiling: 'Собираем геометрию…', stale: 'Показан предыдущий результат', ready: 'Готово',
    failed: 'Ошибка сборки',
    preview: 'Быстрый preview', full: 'Точная сборка', exported: 'Геометрия экспортирована',
    copied: 'Ссылка скопирована', copyFailed: 'Ссылка добавлена в адресную строку',
    opened: 'Файл открыт', saved: 'Файл сохранён', fileTooLarge: 'Файл слишком большой (максимум 250 КБ)',
    workerError: 'Не удалось запустить геометрический Worker', resize: 'Изменить ширину редактора',
    commands: 'Команды', commandHelp: 'Поиск действий', display: 'Отображение',
    shaded: 'Заливка', edges: 'Рёбра', xray: 'Рентген',
    selected: 'Выбран', object: 'Объект', focus: 'Фокус', isolate: 'Изолировать',
    unisolate: 'Показать всё', deselect: 'Снять выбор',
    scene: 'Сцена', inspect: 'Инспектор', point: 'Точка', face: 'Грань', body: 'Тело',
    selectionMode: 'Режим выбора', sidebar: 'Боковая панель', measure: 'Измерить расстояние',
    section: 'Сечение', hidden: 'Скрыт',
    sourceStale: 'Сначала дождитесь сборки текущего исходника',
    needsModel: 'Сначала соберите модель', needsSelection: 'Сначала выберите объект',
    needsFullBuild: 'Нужна актуальная точная сборка', noPreviousView: 'История видов пока пуста',
    buildInProgress: 'Дождитесь завершения текущей сборки',
    depthCandidate: 'цель в глубине',
  },
  en: {
    title: 'OpenSCAD Viewer',
    render: 'Render', auto: 'Auto', examples: 'Example',
    basic: 'Primitives', csg: 'Real CSG', house: 'Modular house', tower: 'Parametric tower',
    open: 'Open', save: 'Save', share: 'Share',
    exportStl: 'Export STL', exportObj: 'Export OBJ', parameters: 'Parameters',
    noParameters: 'Add top-level variables; slider metadata: // [min:step:max]',
    openFile: 'Open an OpenSCAD file', saveFile: 'Save OpenSCAD source', shareFile: 'Copy a link to this model',
    meshes: 'Objects', triangles: 'Triangles', volume: 'Volume', area: 'Surface', time: 'Build',
    hint: 'Click: select · repeat click: cycle deeper · LMB: orbit · RMB/Shift: pan · wheel: zoom · F: focus',
    noGpu: 'WebGPU is unavailable. Open the app in a current Chrome, Edge, Firefox, or Safari.',
    theme: 'Theme', darkTheme: 'Use dark theme', lightTheme: 'Use light theme',
    language: 'Switch language', editor: 'OpenSCAD editor', viewport: '3D viewport',
    fit: 'Fit', reset: 'Reset view', previousView: 'Previous view', perspective: 'Perspective', orthographic: 'Orthographic',
    grid: 'Grid', view: 'View', iso: 'Isometric', front: 'Front', back: 'Back',
    left: 'Left', right: 'Right', top: 'Top', bottom: 'Bottom',
    compiling: 'Building geometry…', stale: 'Showing the previous result', ready: 'Ready',
    failed: 'Build failed',
    preview: 'Fast preview', full: 'Full build', exported: 'Geometry exported',
    copied: 'Link copied', copyFailed: 'Link added to the address bar',
    opened: 'File opened', saved: 'File saved', fileTooLarge: 'File is too large (250 KB maximum)',
    workerError: 'Could not start the geometry Worker', resize: 'Resize editor',
    commands: 'Commands', commandHelp: 'Search actions', display: 'Display',
    shaded: 'Shaded', edges: 'Edges', xray: 'X-ray',
    selected: 'Selected', object: 'Object', focus: 'Focus', isolate: 'Isolate',
    unisolate: 'Show all', deselect: 'Deselect',
    scene: 'Scene', inspect: 'Inspect', point: 'Point', face: 'Face', body: 'Body',
    selectionMode: 'Selection mode', sidebar: 'Sidebar', measure: 'Measure distance',
    section: 'Section', hidden: 'Hidden',
    sourceStale: 'Wait for the current source to finish building first',
    needsModel: 'Build a model first', needsSelection: 'Select an object first',
    needsFullBuild: 'An up-to-date full build is required', noPreviousView: 'View history is empty',
    buildInProgress: 'Wait for the current build to finish',
    depthCandidate: 'depth target',
  },
}

const COMMAND_ALIAS_KEYS: Record<string, readonly string[]> = {
  render: ['render'], open: ['open'], save: ['save'], share: ['share'],
  'export-stl': ['exportStl'], 'export-obj': ['exportObj'],
  fit: ['fit'], focus: ['focus'], reset: ['reset'], 'previous-view': ['previousView'],
  isolate: ['isolate', 'unisolate'], deselect: ['deselect'],
  projection: ['perspective', 'orthographic'], grid: ['grid'],
  shaded: ['shaded'], edges: ['edges'], xray: ['xray'],
  'select-point': ['point'], 'select-face': ['face'], 'select-object': ['body', 'object'],
  measure: ['measure'], section: ['section'], sidebar: ['sidebar'],
  iso: ['iso'], front: ['front'], right: ['right'], top: ['top'],
  back: ['back'], left: ['left'], bottom: ['bottom'], theme: ['theme'],
}

const lang = ref<Language>(readStorage('scad-lang') === 'en' ? 'en' : 'ru')
const isDark = ref(readStorage('scad-theme') !== 'light')
const sharedCode = readSharedCode()
const code = ref(sharedCode ?? readStorage('scad-code') ?? EXAMPLES.basic)
const autoRender = ref(readStorage('scad-auto') !== 'false')
const editorWidth = ref(clamp(Number(readStorage('scad-editor-width')) || 440, 300, 820))

const canvasRef = ref<HTMLCanvasElement | null>(null)
const editorRef = ref<HTMLTextAreaElement | null>(null)
const fileInputRef = ref<HTMLInputElement | null>(null)
const mainRef = ref<HTMLElement | null>(null)
const error = ref('')
const warnings = ref<string[]>([])
const meshCount = ref(0)
const triangleCount = ref(0)
const volume = ref(0)
const surfaceArea = ref(0)
const renderDuration = ref(0)
const gpuOk = ref(true)
const rendering = ref(false)
const renderingQuality = ref<GeometryQuality>('full')
const renderedQuality = ref<GeometryQuality>('full')
const renderedSource = ref('')
const notice = ref('')
const fileName = ref('model.scad')
const selectedExample = ref('')
const projection = ref<ProjectionMode>('perspective')
const gridVisible = ref(true)
const standardView = ref<StandardView>('iso')
const activeView = ref<StandardView | 'custom'>('iso')
const displayMode = ref<DisplayMode>('shaded')
const selectedMesh = ref<number | null>(null)
const selectedHit = ref<PickHit | null>(null)
const hoveredHit = ref<PickHit | null>(null)
const isolated = ref(false)
const paletteOpen = ref(false)
const canPreviousView = ref(false)
const commandMru = ref<string[]>(readCommandMru())
const sceneMeshes = ref<MeshData[]>([])
const meshVisibility = ref<boolean[]>([])
const selectionMode = ref<SelectionMode>('face')
const measurement = ref<DistanceMeasurement | null>(null)
const measureActive = ref(false)
const sectionEnabled = ref(false)
const sectionAxis = ref<SectionAxis>('z')
const sectionOffset = ref(0)
const sectionFlip = ref(false)
const dockTab = ref<'scene' | 'inspect' | 'parameters'>('scene')
const dockOpen = ref(true)

const customizerParameters = computed(() => extractCustomizerParameters(code.value))
const stale = computed(() => renderedSource.value !== '' && (renderedSource.value !== code.value || renderedQuality.value === 'preview'))
const sourceMatchesEditor = computed(() => renderedSource.value !== '' && renderedSource.value === code.value)
const canExport = computed(() => (
  sceneMeshes.value.length > 0
  && !rendering.value
  && renderedQuality.value === 'full'
  && renderedSource.value === code.value
))
const statusText = computed(() => rendering.value
  ? t('compiling')
  : error.value ? t('failed') : stale.value ? t('stale') : t('ready'))
const sceneRows = computed<SceneMeshRow[]>(() => sceneMeshes.value.map((mesh, index) => {
  const grouped = new Map<number, SourceProvenanceRow>()
  for (const run of mesh.provenance) {
    if (!run.source) continue
    const existing = grouped.get(run.source.id)
    const triangleCount = run.triangleEnd - run.triangleStart
    if (existing) existing.triangleCount = (existing.triangleCount ?? 0) + triangleCount
    else {
      const location = lineAndColumn(renderedSource.value || code.value, run.source.start)
      grouped.set(run.source.id, {
        id: `${index}:${run.source.id}`,
        sourceId: run.source.id,
        label: run.source.label,
        sourceStart: run.source.start,
        sourceEnd: run.source.end,
        line: location.line,
        column: location.column,
        originalId: run.source.originalId,
        triangleCount,
        color: mesh.color,
      })
    }
  }
  return {
    id: index,
    name: `${t('object')} ${index + 1}`,
    visible: meshVisibility.value[index] !== false,
    triangleCount: mesh.indices.length / 3,
    color: mesh.color,
    sources: [...grouped.values()],
  }
}))
const hoveredMesh = computed(() => hoveredHit.value?.meshIndex ?? null)
const currentInspection = computed<InspectSelection | null>(() => {
  const index = selectedMesh.value
  const mesh = index === null ? null : sceneMeshes.value[index]
  if (!mesh || index === null) return null
  const facts = inspectMesh(mesh, index)
  const hit = selectedHit.value?.meshIndex === index ? selectedHit.value : null
  let source: SourceProvenanceRow | undefined
  if (hit?.source) {
    const location = lineAndColumn(renderedSource.value || code.value, hit.source.start)
    source = {
      id: `${index}:${hit.source.originalId}`,
      sourceId: hit.source.id,
      label: hit.source.label,
      sourceStart: hit.source.start,
      sourceEnd: hit.source.end,
      line: location.line,
      column: location.column,
      originalId: hit.source.originalId,
      color: mesh.color,
    }
  }
  return {
    meshId: index,
    meshName: `${t('object')} ${index + 1}`,
    triangleCount: facts.triangles,
    surfaceArea: sceneMeshes.value.length === 1 ? surfaceArea.value : undefined,
    volume: sceneMeshes.value.length === 1 ? volume.value : undefined,
    position: hit?.point,
    normal: hit?.normal,
    bounds: facts.bounds ?? undefined,
    source,
  }
})
const panelMeasurement = computed<PanelMeasurement | null>(() => measurement.value ? {
  points: measurement.value.points,
  distance: measurement.value.distance ?? undefined,
} : null)
const sectionRange = computed(() => {
  const axis = sectionAxis.value === 'x' ? 0 : sectionAxis.value === 'y' ? 1 : 2
  let min = Infinity
  let max = -Infinity
  sceneMeshes.value.forEach((mesh, index) => {
    if (meshVisibility.value[index] === false) return
    const bounds = inspectMesh(mesh, index).bounds
    if (!bounds) return
    min = Math.min(min, bounds.min[axis])
    max = Math.max(max, bounds.max[axis])
  })
  if (!Number.isFinite(min) || !Number.isFinite(max)) return { min: -100, max: 100, step: 0.1 }
  const span = Math.max(0.001, max - min)
  return { min, max, step: Math.max(0.001, span / 500) }
})
const paletteCommands = computed(() => {
  const hasVisibleModel = sceneMeshes.value.some((_, index) => meshVisibility.value[index] !== false)
  const hasSelection = selectedMesh.value !== null
  const commands: PaletteCommand[] = [
    command('render', t('render'), 'Ctrl/⌘+Enter', 'build compile render', {
      enabled: !rendering.value,
      disabledReason: t('buildInProgress'),
    }),
    command('open', t('open'), 'Ctrl/⌘+O', 'file import'),
    command('save', t('save'), 'Ctrl/⌘+S', 'file export'),
    command('export-stl', t('exportStl'), '', 'mesh manufacturing print', {
      enabled: canExport.value,
      disabledReason: t('needsFullBuild'),
    }),
    command('export-obj', t('exportObj'), '', 'mesh interchange', {
      enabled: canExport.value,
      disabledReason: t('needsFullBuild'),
    }),
    command('share', t('share'), '', 'link copy'),
    command('focus', t('focus'), '/', 'selection frame', {
      enabled: hasSelection,
      disabledReason: t('needsSelection'),
    }),
    command('isolate', isolated.value ? t('unisolate') : t('isolate'), '.', 'selection visibility', {
      enabled: hasSelection,
      disabledReason: t('needsSelection'),
    }),
    command('deselect', t('deselect'), 'Esc', 'selection clear', {
      enabled: hasSelection,
      disabledReason: t('needsSelection'),
    }),
    command('fit', t('fit'), 'F', 'camera frame all', {
      enabled: hasVisibleModel,
      disabledReason: t('needsModel'),
    }),
    command('reset', t('reset'), '', 'camera home'),
    command('previous-view', t('previousView'), '[', 'camera history back', {
      enabled: canPreviousView.value,
      disabledReason: t('noPreviousView'),
    }),
    command('projection', projection.value === 'perspective' ? t('orthographic') : t('perspective'), '5', 'camera projection'),
    command('grid', t('grid'), 'G', 'overlay'),
    command('shaded', t('shaded'), '', 'shader display'),
    command('edges', t('edges'), 'E', 'wireframe mesh display'),
    command('xray', t('xray'), 'X', 'transparent display'),
    command('select-point', t('point'), '1', 'selection vertex snap'),
    command('select-face', t('face'), '3', 'selection surface'),
    command('select-object', t('body'), '4', 'selection body solid'),
    command('measure', t('measure'), 'Ctrl/⌘+=', 'inspect distance dimension', {
      enabled: hasVisibleModel || measureActive.value,
      disabledReason: t('needsModel'),
    }),
    command('section', t('section'), '', 'inspect slice clipping plane', {
      enabled: hasVisibleModel || sectionEnabled.value,
      disabledReason: t('needsModel'),
    }),
    command('sidebar', t('sidebar'), 'Ctrl/⌘+Shift+B', 'panel outliner inspect parameters'),
    command('iso', t('iso'), 'Num 0', 'camera view'),
    command('front', t('front'), 'Num 1', 'camera view'),
    command('right', t('right'), 'Num 3', 'camera view'),
    command('top', t('top'), 'Num 7', 'camera view'),
    command('back', t('back'), '', 'camera view'),
    command('left', t('left'), '', 'camera view'),
    command('bottom', t('bottom'), '', 'camera view'),
    command('theme', t('theme'), '', 'dark light'),
  ]
  return commands
})

let renderer: WebGPURenderer | null = null
let geometryWorker: Worker | null = null
let renderDebounce: ReturnType<typeof setTimeout> | null = null
let fullRenderDebounce: ReturnType<typeof setTimeout> | null = null
let storageDebounce: ReturnType<typeof setTimeout> | null = null
let noticeTimeout: ReturnType<typeof setTimeout> | null = null
let requestId = 0
let resizing = false
let fitNextRender = false
const requestSources = new Map<number, string>()

const t = (key: string) => L[lang.value][key] ?? key
const formatNumber = (value: number, digits = 0) => value.toLocaleString(lang.value, { maximumFractionDigits: digits })

onMounted(async () => {
  applyPreferences()
  if (!canvasRef.value) return

  renderer = new WebGPURenderer()
  const ok = await renderer.init(canvasRef.value)
  if (!ok) {
    gpuOk.value = false
    renderer = null
    return
  }

  renderer.onSelectionChange = (index, isIsolated, hit) => {
    selectedMesh.value = index
    isolated.value = isIsolated
    selectedHit.value = hit
  }
  renderer.onHoverChange = hit => { hoveredHit.value = hit }
  renderer.onMeasurementChange = (value, active) => {
    measurement.value = value
    measureActive.value = active
  }
  renderer.onCameraHistoryChange = available => { canPreviousView.value = available }
  canPreviousView.value = renderer.canGoToPreviousView
  renderer.setDisplayMode(displayMode.value)
  renderer.setSelectionMode(selectionMode.value)
  window.addEventListener('keydown', handleGlobalKey)

  try {
    startGeometryWorker()
    doRender('full')
  } catch {
    error.value = t('workerError')
  }
})

onUnmounted(() => {
  if (renderDebounce) clearTimeout(renderDebounce)
  if (fullRenderDebounce) clearTimeout(fullRenderDebounce)
  if (storageDebounce) clearTimeout(storageDebounce)
  if (noticeTimeout) clearTimeout(noticeTimeout)
  geometryWorker?.terminate()
  geometryWorker = null
  window.removeEventListener('keydown', handleGlobalKey)
  if (renderer) {
    renderer.onSelectionChange = null
    renderer.onHoverChange = null
    renderer.onMeasurementChange = null
    renderer.onCameraHistoryChange = null
  }
  renderer?.destroy()
  renderer = null
})

watch(code, value => {
  // Provenance belongs to the last compiled source revision. Never retain a
  // reverse highlight while the editor has moved ahead of that revision.
  if (renderedSource.value !== value) renderer?.setSourceHighlight(null)
  if (storageDebounce) clearTimeout(storageDebounce)
  storageDebounce = setTimeout(() => writeStorage('scad-code', value), 300)
  if (autoRender.value) scheduleRender()
})

watch(autoRender, enabled => {
  writeStorage('scad-auto', String(enabled))
  if (enabled) scheduleRender(0)
})

function scheduleRender(delay = 450) {
  if (renderDebounce) clearTimeout(renderDebounce)
  if (fullRenderDebounce) clearTimeout(fullRenderDebounce)
  renderDebounce = setTimeout(() => doRender('preview'), Math.min(delay, 180))
  fullRenderDebounce = setTimeout(() => doRender('full'), Math.max(delay + 500, 780))
}

function startGeometryWorker() {
  geometryWorker = new Worker(new URL('./workers/geometry.worker.ts', import.meta.url), { type: 'module' })
  geometryWorker.addEventListener('message', handleGeometryResponse)
  geometryWorker.addEventListener('error', handleWorkerError)
}

function restartGeometryWorker() {
  geometryWorker?.terminate()
  geometryWorker = null
  startGeometryWorker()
}

function doRender(quality: GeometryQuality = 'full') {
  if (!renderer) return
  if (renderDebounce) { clearTimeout(renderDebounce); renderDebounce = null }
  if (quality === 'full' && fullRenderDebounce) { clearTimeout(fullRenderDebounce); fullRenderDebounce = null }
  try {
    if (!geometryWorker) startGeometryWorker()
    else if (rendering.value) restartGeometryWorker()
  } catch {
    handleWorkerError()
    return
  }
  const id = ++requestId
  const source = code.value
  requestSources.set(id, source)
  rendering.value = true
  renderingQuality.value = quality
  error.value = ''
  warnings.value = []
  geometryWorker!.postMessage({ id, source, quality })
}

function handleGeometryResponse(event: MessageEvent<GeometryResponse>) {
  const response = event.data
  if (response.id !== requestId) return
  rendering.value = false
  renderDuration.value = response.durationMs
  const source = requestSources.get(response.id) ?? code.value
  requestSources.clear()

  if (!response.ok) {
    error.value = response.error.message
    return
  }

  try {
    const preserveInteraction = renderedSource.value !== '' && renderedSource.value === source
    const previousMeshes = sceneMeshes.value
    const previousVisibility = meshVisibility.value
    const previousSelection = selectedMesh.value
    const previousIsolation = isolated.value
    const previousMeasurement = measurement.value
    const previousMeasureActive = measureActive.value
    const previousIndices = preserveInteraction
      ? matchMeshesByProvenance(previousMeshes, response.meshes)
      : response.meshes.map(() => -1)

    renderer?.setMeshes(response.meshes, { preserveMeasurement: preserveInteraction })
    sceneMeshes.value = response.meshes
    meshVisibility.value = previousIndices.map(previousIndex => (
      previousIndex >= 0 ? previousVisibility[previousIndex] !== false : true
    ))
    meshVisibility.value.forEach((visible, index) => {
      if (!visible) renderer?.setMeshVisibility(index, false)
    })
    selectedHit.value = null
    hoveredHit.value = null
    if (preserveInteraction) {
      const replacementSelection = previousSelection === null
        ? -1
        : previousIndices.indexOf(previousSelection)
      if (replacementSelection >= 0) {
        renderer?.selectMesh(replacementSelection)
        if (previousIsolation) renderer?.toggleIsolateSelection()
      }
      // The renderer may rebuild this overlay; retaining the UI value avoids a
      // preview → full flash and lets it restore the exact world-space points.
      measurement.value = previousMeasurement
      measureActive.value = previousMeasureActive
    } else {
      measurement.value = null
    }
    selectedMesh.value = renderer?.selectedIndex ?? null
    isolated.value = renderer?.isIsolated ?? false
    if (fitNextRender) {
      renderer?.fitView()
      fitNextRender = false
    }
    meshCount.value = response.meshes.length
    triangleCount.value = response.meshes.reduce((sum, mesh) => sum + mesh.indices.length / 3, 0)
    volume.value = response.volume
    surfaceArea.value = response.surfaceArea
    warnings.value = response.warnings
    renderedSource.value = source
    renderedQuality.value = response.quality
    syncSourceHighlightFromEditor()
    sectionOffset.value = clamp(sectionOffset.value, sectionRange.value.min, sectionRange.value.max)
    applySection()
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught)
  }
}

function handleWorkerError() {
  rendering.value = false
  geometryWorker?.terminate()
  geometryWorker = null
  error.value = t('workerError')
}

function toggleLang() {
  lang.value = lang.value === 'ru' ? 'en' : 'ru'
  writeStorage('scad-lang', lang.value)
  document.documentElement.lang = lang.value
}

function toggleTheme() {
  isDark.value = !isDark.value
  applyPreferences()
}

function applyPreferences() {
  document.documentElement.dataset.theme = isDark.value ? 'dark' : 'light'
  document.documentElement.lang = lang.value
  document.documentElement.style.colorScheme = isDark.value ? 'dark' : 'light'
  writeStorage('scad-theme', isDark.value ? 'dark' : 'light')
}

function loadExample() {
  const example = EXAMPLES[selectedExample.value]
  if (!example) return
  code.value = example
  fileName.value = `${selectedExample.value}.scad`
  fitNextRender = true
}

function triggerOpen() { fileInputRef.value?.click() }

async function openSelectedFile(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (file) await openFile(file)
}

async function handleDrop(event: DragEvent) {
  const file = event.dataTransfer?.files?.[0]
  if (file) await openFile(file)
}

async function openFile(file: File) {
  if (file.size > 250_000) {
    error.value = t('fileTooLarge')
    return
  }
  code.value = await file.text()
  fileName.value = file.name.endsWith('.scad') ? file.name : `${file.name}.scad`
  fitNextRender = true
  showNotice(t('opened'))
}

function saveSource() {
  const blob = new Blob([code.value], { type: 'text/plain;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = sanitizeFileName(fileName.value)
  link.click()
  URL.revokeObjectURL(url)
  showNotice(t('saved'))
}

function downloadBlob(blob: Blob, name: string) {
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = name
  link.click()
  URL.revokeObjectURL(url)
}

function exportStl() {
  if (!canExport.value) return
  const bytes = buildBinaryStl(sceneMeshes.value, fileName.value)
  const buffer = new ArrayBuffer(bytes.byteLength)
  new Uint8Array(buffer).set(bytes)
  downloadBlob(new Blob([buffer], { type: 'model/stl' }), sanitizeFileName(fileName.value).replace(/\.scad$/i, '.stl'))
  showNotice(t('exported'))
}

function exportObj() {
  if (!canExport.value) return
  downloadBlob(new Blob([buildObj(sceneMeshes.value)], { type: 'text/plain;charset=utf-8' }), sanitizeFileName(fileName.value).replace(/\.scad$/i, '.obj'))
  showNotice(t('exported'))
}

function updateCustomizer(name: string, value: CustomizerValue) {
  const parameter = customizerParameters.value.find(candidate => candidate.name === name)
  if (!parameter) return
  code.value = replaceCustomizerValue(code.value, parameter, value)
  selectedExample.value = ''
}

async function shareSource() {
  const url = new URL(window.location.href)
  url.hash = `code=${encodeBase64(code.value)}`
  history.replaceState(null, '', url)
  try {
    await navigator.clipboard.writeText(url.toString())
    showNotice(t('copied'))
  } catch {
    showNotice(t('copyFailed'))
  }
}

function fitView() { renderer?.fitView() }
function resetView() {
  standardView.value = 'iso'
  activeView.value = 'iso'
  renderer?.resetView()
}
function previousView() {
  const state = renderer?.previousView()
  if (!state) return
  projection.value = state.projection
  const restoredView = standardViewForCamera(state)
  if (restoredView) {
    standardView.value = restoredView
    activeView.value = restoredView
  } else {
    activeView.value = 'custom'
  }
}

function standardViewForCamera(state: CameraState): StandardView | null {
  const orientations: Array<[StandardView, number, number]> = [
    ['iso', Math.PI / 4, Math.atan(1 / Math.sqrt(2))],
    ['front', 0, 0], ['back', Math.PI, 0],
    ['left', -Math.PI / 2, 0], ['right', Math.PI / 2, 0],
    ['top', 0, Math.PI / 2], ['bottom', 0, -Math.PI / 2],
  ]
  const angularDistance = (a: number, b: number) => Math.abs(Math.atan2(Math.sin(a - b), Math.cos(a - b)))
  return orientations.find(([, yaw, pitch]) => (
    angularDistance(state.yaw, yaw) <= 1e-7 && Math.abs(state.pitch - pitch) <= 1e-7
  ))?.[0] ?? null
}
function focusSelection() {
  if (!renderer?.fitSelection()) renderer?.fitView()
}
function clearSelection() { renderer?.clearSelection() }
function toggleIsolate() {
  isolated.value = renderer?.toggleIsolateSelection() ?? false
}
function toggleProjection() {
  projection.value = projection.value === 'perspective' ? 'orthographic' : 'perspective'
  renderer?.setProjection(projection.value)
}
function toggleGrid() {
  gridVisible.value = !gridVisible.value
  renderer?.setGridVisible(gridVisible.value)
}
function changeStandardView() {
  activeView.value = standardView.value
  renderer?.setView(standardView.value)
}
function setStandardView(view: StandardView) {
  standardView.value = view
  changeStandardView()
}
function markCustomView(event: PointerEvent) {
  if ((event.buttons & 1) !== 0 && !event.shiftKey) activeView.value = 'custom'
}
function setDisplayMode(mode: DisplayMode) {
  displayMode.value = mode
  renderer?.setDisplayMode(mode)
}
function changeDisplayMode() { renderer?.setDisplayMode(displayMode.value) }

function setSelectionMode(mode: SelectionMode) {
  selectionMode.value = mode
  renderer?.setSelectionMode(mode)
}

function meshIndex(id: number | string) {
  const index = typeof id === 'number' ? id : Number(id)
  return Number.isInteger(index) && index >= 0 && index < sceneMeshes.value.length ? index : null
}

function selectSceneMesh(id: number | string) {
  const index = meshIndex(id)
  if (index === null) return
  renderer?.selectMesh(index)
}

function setSceneMeshVisibility(id: number | string, visible: boolean) {
  const index = meshIndex(id)
  if (index === null) return
  meshVisibility.value = meshVisibility.value.map((value, candidate) => candidate === index ? visible : value)
  renderer?.setMeshVisibility(index, visible)
}

function preselectSceneMesh(id: number | string | null) {
  renderer?.setPreselection(id === null ? null : meshIndex(id))
}

function focusSceneMesh(id: number | string) {
  const index = meshIndex(id)
  if (index === null) return
  renderer?.selectMesh(index)
  renderer?.fitSelection()
}

function isolateSceneMesh(id: number | string) {
  const index = meshIndex(id)
  if (index === null) return
  if (selectedMesh.value !== index) renderer?.selectMesh(index)
  isolated.value = renderer?.toggleIsolateSelection() ?? false
}

function revealSource(source: SourceProvenanceRow) {
  if (!sourceMatchesEditor.value) {
    showNotice(t('sourceStale'))
    return
  }
  const editor = editorRef.value
  if (!editor) return
  const start = clamp(source.sourceStart, 0, code.value.length)
  const end = clamp(source.sourceEnd, start, code.value.length)
  editor.focus({ preventScroll: false })
  editor.setSelectionRange(start, end, 'forward')
  renderer?.setSourceHighlight(source.sourceId)
}

function sourceIdAtEditorCaret(): number | null {
  const editor = editorRef.value
  if (!editor || !sourceMatchesEditor.value) return null
  const caret = editor.selectionStart === editor.selectionEnd
    ? editor.selectionStart
    : editor.selectionDirection === 'backward'
      ? editor.selectionStart
      : Math.max(editor.selectionStart, editor.selectionEnd - 1)
  let sourceId: number | null = null
  let smallestSpan = Infinity
  let deepestStart = -1
  for (const mesh of sceneMeshes.value) {
    for (const run of mesh.provenance) {
      const source = run.source
      if (!source || caret < source.start || caret >= source.end) continue
      const span = source.end - source.start
      if (span < smallestSpan || (span === smallestSpan && source.start > deepestStart)) {
        sourceId = source.id
        smallestSpan = span
        deepestStart = source.start
      }
    }
  }
  return sourceId
}

function syncSourceHighlightFromEditor() {
  renderer?.setSourceHighlight(sourceIdAtEditorCaret())
}

function highlightSource(sourceId: number | null) {
  if (sourceId === null) syncSourceHighlightFromEditor()
  else renderer?.setSourceHighlight(sourceMatchesEditor.value ? sourceId : null)
}

function handleEditorInput() {
  selectedExample.value = ''
  // The v-model update precedes this handler, so this also clears stale
  // geometry immediately instead of waiting for the next Worker response.
  syncSourceHighlightFromEditor()
}

function startMeasure() {
  measureActive.value = true
  dockOpen.value = true
  dockTab.value = 'inspect'
  renderer?.setMeasureMode(true)
}

function cancelMeasure() {
  measureActive.value = false
  renderer?.setMeasureMode(false)
}

function clearMeasurement() { renderer?.clearMeasurement() }

async function copyMeasurement(value: PanelMeasurement) {
  if (value.points.length < 2) return
  const [a, b] = value.points
  const distance = value.distance ?? Math.hypot(b[0] - a[0], b[1] - a[1], b[2] - a[2])
  try {
    await navigator.clipboard.writeText(`A ${a.join(', ')}\nB ${b.join(', ')}\nDistance ${distance}`)
    showNotice(t('copied'))
  } catch { /* Clipboard may be unavailable outside a secure context. */ }
}

function applySection() {
  const base: [number, number, number] = sectionAxis.value === 'x' ? [1, 0, 0]
    : sectionAxis.value === 'y' ? [0, 1, 0]
      : [0, 0, 1]
  const sign = sectionFlip.value ? -1 : 1
  renderer?.setSection(
    sectionEnabled.value,
    [base[0] * sign, base[1] * sign, base[2] * sign],
    sectionOffset.value * sign,
  )
}

function setSectionEnabled(enabled: boolean) {
  sectionEnabled.value = enabled
  applySection()
}

function setSectionAxis(axis: SectionAxis) {
  sectionAxis.value = axis
  sectionOffset.value = clamp(sectionOffset.value, sectionRange.value.min, sectionRange.value.max)
  applySection()
}

function setSectionOffset(offset: number) {
  sectionOffset.value = clamp(offset, sectionRange.value.min, sectionRange.value.max)
  applySection()
}

function setSectionFlip(flip: boolean) {
  sectionFlip.value = flip
  applySection()
}

function resetSection() {
  sectionOffset.value = (sectionRange.value.min + sectionRange.value.max) / 2
  sectionFlip.value = false
  applySection()
}

function lineAndColumn(source: string, offset: number) {
  const before = source.slice(0, clamp(offset, 0, source.length))
  const line = before.split('\n').length
  return { line, column: before.length - before.lastIndexOf('\n') }
}

function command(
  id: string,
  label: string,
  shortcut = '',
  keywords = '',
  options: Partial<Pick<PaletteCommand, 'enabled' | 'disabledReason'>> = {},
): PaletteCommand {
  const aliases = (COMMAND_ALIAS_KEYS[id] ?? [])
    .flatMap(key => [L.ru[key], L.en[key]])
    .filter((value, index, values) => !!value && values.indexOf(value) === index)
  const mruRank = commandMru.value.indexOf(id)
  return {
    id,
    label,
    shortcut,
    keywords,
    aliases,
    ...options,
    ...(mruRank >= 0 ? { mruRank } : {}),
  }
}

function executeCommand(id: string) {
  const target = paletteCommands.value.find(candidate => candidate.id === id)
  if (target && !isPaletteCommandEnabled(target)) return
  paletteOpen.value = false
  recordCommandUsage(id)
  const views: StandardView[] = ['iso', 'front', 'back', 'left', 'right', 'top', 'bottom']
  if (views.includes(id as StandardView)) { setStandardView(id as StandardView); return }
  switch (id) {
    case 'render': doRender('full'); break
    case 'open': triggerOpen(); break
    case 'save': saveSource(); break
    case 'export-stl': exportStl(); break
    case 'export-obj': exportObj(); break
    case 'share': void shareSource(); break
    case 'fit': fitView(); break
    case 'focus': focusSelection(); break
    case 'reset': resetView(); break
    case 'previous-view': previousView(); break
    case 'isolate': toggleIsolate(); break
    case 'deselect': clearSelection(); break
    case 'projection': toggleProjection(); break
    case 'grid': toggleGrid(); break
    case 'shaded':
    case 'edges':
    case 'xray': setDisplayMode(id); break
    case 'select-point': setSelectionMode('point'); break
    case 'select-face': setSelectionMode('face'); break
    case 'select-object': setSelectionMode('object'); break
    case 'measure': measureActive.value ? cancelMeasure() : startMeasure(); break
    case 'section':
      dockOpen.value = true
      dockTab.value = 'inspect'
      setSectionEnabled(!sectionEnabled.value)
      break
    case 'sidebar': dockOpen.value = !dockOpen.value; break
    case 'theme': toggleTheme(); break
  }
}

function recordCommandUsage(id: string) {
  const next = [id, ...commandMru.value.filter(candidate => candidate !== id)].slice(0, 12)
  commandMru.value = next
  writeStorage('scad-command-mru', JSON.stringify(next))
}

function handleEditorKey(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
    event.preventDefault()
    doRender('full')
  }
}

function handleViewportKey(event: KeyboardEvent) {
  const numpadViews: Record<string, StandardView> = {
    Numpad0: 'iso', Numpad1: 'front', Numpad3: 'right', Numpad7: 'top',
  }
  const key = event.key.toLowerCase()
  const plainKey = !event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey
  if (plainKey && numpadViews[event.code]) {
    event.preventDefault()
    standardView.value = numpadViews[event.code]
    changeStandardView()
  }
  else if ((event.ctrlKey || event.metaKey) && event.key === '=') {
    event.preventDefault()
    measureActive.value ? cancelMeasure() : startMeasure()
  }
  else if (event.shiftKey && key === 'f' && sectionEnabled.value) { event.preventDefault(); setSectionFlip(!sectionFlip.value) }
  else if (plainKey && event.code === 'BracketLeft') { event.preventDefault(); previousView() }
  else if (key === 'f' || event.key === '/') { event.preventDefault(); focusSelection() }
  else if (event.key === '.') { event.preventDefault(); toggleIsolate() }
  else if (event.key === 'Escape') { event.preventDefault(); measureActive.value ? cancelMeasure() : clearSelection() }
  else if (key === 'e') { event.preventDefault(); setDisplayMode(displayMode.value === 'edges' ? 'shaded' : 'edges') }
  else if (key === 'x') { event.preventDefault(); setDisplayMode(displayMode.value === 'xray' ? 'shaded' : 'xray') }
  else if (key === 'g') { event.preventDefault(); toggleGrid() }
  else if (key === 'h' && selectedMesh.value !== null) {
    event.preventDefault()
    setSceneMeshVisibility(selectedMesh.value, meshVisibility.value[selectedMesh.value] === false)
  }
  else if (plainKey && event.code === 'Digit5') { event.preventDefault(); toggleProjection() }
  else if (plainKey && event.code === 'Digit1') { event.preventDefault(); setSelectionMode('point') }
  else if (plainKey && event.code === 'Digit3') { event.preventDefault(); setSelectionMode('face') }
  else if (plainKey && event.code === 'Digit4') { event.preventDefault(); setSelectionMode('object') }
  else if (event.shiftKey && event.code === 'KeyM') {
    event.preventDefault()
    const modes: SelectionMode[] = ['point', 'face', 'object']
    setSelectionMode(modes[(modes.indexOf(selectionMode.value) + 1) % modes.length])
  }
}

function handleGlobalKey(event: KeyboardEvent) {
  if (event.defaultPrevented) return
  if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === 'b') {
    event.preventDefault()
    dockOpen.value = !dockOpen.value
    return
  }
  if ((event.ctrlKey || event.metaKey) && event.key === '=') {
    event.preventDefault()
    measureActive.value ? cancelMeasure() : startMeasure()
    return
  }
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
    event.preventDefault()
    paletteOpen.value = true
    return
  }
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
    event.preventDefault()
    saveSource()
    return
  }
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'o') {
    event.preventDefault()
    triggerOpen()
  }
}

function startResize(event: PointerEvent) {
  if (window.matchMedia('(max-width: 800px)').matches) return
  resizing = true
  ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
  resizeEditor(event.clientX)
}
function moveResize(event: PointerEvent) { if (resizing) resizeEditor(event.clientX) }
function stopResize() {
  if (!resizing) return
  resizing = false
  writeStorage('scad-editor-width', String(Math.round(editorWidth.value)))
}
function resizeEditor(clientX: number) {
  const rect = mainRef.value?.getBoundingClientRect()
  if (!rect) return
  editorWidth.value = clamp(clientX - rect.left, 300, Math.max(300, Math.min(820, rect.width - 320)))
}
function resizeEditorWithKeyboard(event: KeyboardEvent) {
  if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
  event.preventDefault()
  editorWidth.value = clamp(editorWidth.value + (event.key === 'ArrowRight' ? 20 : -20), 300, 820)
  writeStorage('scad-editor-width', String(Math.round(editorWidth.value)))
}

function showNotice(message: string) {
  notice.value = message
  if (noticeTimeout) clearTimeout(noticeTimeout)
  noticeTimeout = setTimeout(() => { notice.value = '' }, 2200)
}

function readStorage(key: string): string | null {
  try { return typeof localStorage === 'undefined' ? null : localStorage.getItem(key) }
  catch { return null }
}
function readCommandMru(): string[] {
  try {
    const value: unknown = JSON.parse(readStorage('scad-command-mru') ?? '[]')
    if (!Array.isArray(value)) return []
    return [...new Set(value.filter((item): item is string => typeof item === 'string' && item.length > 0))].slice(0, 12)
  } catch { return [] }
}
function writeStorage(key: string, value: string) {
  try { localStorage.setItem(key, value) } catch { /* storage can be unavailable or full */ }
}
function clamp(value: number, min: number, max: number) { return Math.max(min, Math.min(max, value)) }
function sanitizeFileName(name: string) { return (name.replace(/[^\w.() -]+/g, '_') || 'model.scad').replace(/\.scad.*$/i, '.scad') }

function encodeBase64(value: string) {
  const bytes = new TextEncoder().encode(value)
  let binary = ''
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000))
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '')
}
function decodeBase64(value: string) {
  const normalized = value.replace(/-/g, '+').replace(/_/g, '/')
  const binary = atob(normalized)
  const bytes = Uint8Array.from(binary, character => character.charCodeAt(0))
  return new TextDecoder().decode(bytes)
}
function readSharedCode() {
  try {
    if (!location.hash.startsWith('#code=')) return null
    // Size-cap BEFORE decoding: a crafted multi-MB #code= link would
    // otherwise force large synchronous atob/TextDecoder allocations (and
    // an O(n^2) customizer parse) on page load. ~1.4M base64 chars ≈ 1 MB.
    if (location.hash.length > 1_400_000) return null
    return decodeBase64(location.hash.slice(6))
  } catch { return null }
}
</script>

<template>
  <div class="app" @dragover.prevent @drop.prevent="handleDrop">
    <nav class="topbar" aria-label="Application">
      <div class="topbar-left">
        <svg class="logo" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
          <path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/>
        </svg>
        <span class="brand">{{ t('title') }}</span>
        <span class="kernel-badge">Manifold</span>
      </div>
      <div class="topbar-right">
        <button class="icon-btn command-btn" type="button" :title="t('commandHelp')" aria-keyshortcuts="Control+K Meta+K" @click="paletteOpen = true">
          <span aria-hidden="true">⌘</span> {{ t('commands') }} <kbd>Ctrl K</kbd>
        </button>
        <button class="icon-btn lang-btn" type="button" :aria-label="t('language')" @click="toggleLang">
          {{ lang === 'ru' ? 'RU' : 'EN' }}
        </button>
        <button class="icon-btn" type="button" :aria-label="isDark ? t('lightTheme') : t('darkTheme')" :aria-pressed="isDark" @click="toggleTheme">
          <span aria-hidden="true">{{ isDark ? '☾' : '☀' }}</span>
        </button>
      </div>
    </nav>

    <main v-if="!gpuOk" class="no-gpu" role="alert">{{ t('noGpu') }}</main>

    <main v-else ref="mainRef" class="main">
      <section class="editor-panel" :style="{ width: `${editorWidth}px` }" :aria-label="t('editor')">
        <div class="toolbar editor-toolbar">
          <button class="btn btn-primary" type="button" title="Ctrl/⌘+Enter" :disabled="rendering" @click="doRender('full')">
            <span class="play" aria-hidden="true">▶</span> {{ t('render') }}
          </button>
          <label class="auto-check"><input v-model="autoRender" type="checkbox"> {{ t('auto') }}</label>
          <span class="toolbar-divider" aria-hidden="true" />
          <label class="select-label">
            <span class="sr-only">{{ t('examples') }}</span>
            <select v-model="selectedExample" class="select" :aria-label="t('examples')" @change="loadExample">
              <option disabled value="">{{ t('examples') }}</option>
              <option value="basic">{{ t('basic') }}</option>
              <option value="csg">{{ t('csg') }}</option>
              <option value="house">{{ t('house') }}</option>
              <option value="tower">{{ t('tower') }}</option>
            </select>
          </label>
        </div>

        <div class="toolbar file-toolbar">
          <button class="btn" type="button" :title="t('openFile')" @click="triggerOpen">↥ {{ t('open') }}</button>
          <button class="btn" type="button" :title="t('saveFile')" @click="saveSource">↧ {{ t('save') }}</button>
          <button class="btn" type="button" :title="t('shareFile')" @click="shareSource">⌁ {{ t('share') }}</button>
          <button class="btn export-btn" type="button" :disabled="!canExport" @click="exportStl">STL</button>
          <button class="btn export-btn" type="button" :disabled="!canExport" @click="exportObj">OBJ</button>
          <span class="file-name" :title="fileName">{{ fileName }}</span>
          <input ref="fileInputRef" class="sr-only" type="file" accept=".scad,text/plain" @change="openSelectedFile">
        </div>

        <textarea
          ref="editorRef"
          v-model="code"
          class="code"
          :aria-label="t('editor')"
          spellcheck="false"
          autocomplete="off"
          autocorrect="off"
          autocapitalize="off"
          @input="handleEditorInput"
          @select="syncSourceHighlightFromEditor"
          @click="syncSourceHighlightFromEditor"
          @keyup="syncSourceHighlightFromEditor"
          @focus="syncSourceHighlightFromEditor"
          @keydown="handleEditorKey"
        />

        <div v-if="error" class="message error" role="alert" aria-live="assertive">{{ error }}</div>
        <div v-if="warnings.length" class="message warning" role="status">
          <div v-for="warning in warnings" :key="warning">⚠ {{ warning }}</div>
        </div>

        <footer class="stats" aria-live="polite">
          <span>{{ t('meshes') }} <strong>{{ formatNumber(meshCount) }}</strong></span>
          <span>{{ t('triangles') }} <strong>{{ formatNumber(triangleCount) }}</strong></span>
          <span v-if="meshCount">{{ t('volume') }} <strong>{{ formatNumber(volume, 2) }}</strong></span>
          <span v-if="meshCount">{{ t('area') }} <strong>{{ formatNumber(surfaceArea, 2) }}</strong></span>
          <span class="status" :class="{ stale, busy: rendering, failed: !!error }">{{ statusText }} · {{ formatNumber(renderDuration, 0) }} ms</span>
        </footer>
      </section>

      <div
        class="splitter"
        role="separator"
        tabindex="0"
        aria-orientation="vertical"
        :aria-label="t('resize')"
        :aria-valuenow="Math.round(editorWidth)"
        aria-valuemin="300"
        aria-valuemax="820"
        @pointerdown="startResize"
        @pointermove="moveResize"
        @pointerup="stopResize"
        @pointercancel="stopResize"
        @keydown="resizeEditorWithKeyboard"
      ><span /></div>

      <section class="canvas-panel" :aria-label="t('viewport')">
        <div class="viewer-toolbar">
          <button class="view-btn" type="button" :title="t('fit')" @click="fitView">⌗ <span>{{ t('fit') }}</span></button>
          <button class="view-btn icon-only" type="button" :title="t('reset')" @click="resetView">↺</button>
          <button
            class="view-btn icon-only"
            type="button"
            :disabled="!canPreviousView"
            :aria-label="t('previousView')"
            :title="canPreviousView ? `${t('previousView')} · [` : t('noPreviousView')"
            @click="previousView"
          >←</button>
          <button class="view-btn" type="button" :aria-pressed="projection === 'orthographic'" @click="toggleProjection">
            {{ projection === 'perspective' ? t('perspective') : t('orthographic') }}
          </button>
          <button class="view-btn" type="button" :aria-pressed="gridVisible" @click="toggleGrid"># {{ t('grid') }}</button>
          <button class="view-btn icon-only" type="button" :aria-pressed="dockOpen" :title="t('sidebar')" @click="dockOpen = !dockOpen">▥</button>
          <label class="view-select-label">
            <span class="sr-only">{{ t('display') }}</span>
            <select v-model="displayMode" class="view-select display-select" :aria-label="t('display')" @change="changeDisplayMode">
              <option value="shaded">{{ t('shaded') }}</option>
              <option value="edges">{{ t('edges') }}</option>
              <option value="xray">{{ t('xray') }}</option>
            </select>
          </label>
          <label class="view-select-label">
            <span class="sr-only">{{ t('view') }}</span>
            <select v-model="standardView" class="view-select" :aria-label="t('view')" @change="changeStandardView">
              <option value="iso">{{ t('iso') }}</option>
              <option value="front">{{ t('front') }}</option>
              <option value="back">{{ t('back') }}</option>
              <option value="left">{{ t('left') }}</option>
              <option value="right">{{ t('right') }}</option>
              <option value="top">{{ t('top') }}</option>
              <option value="bottom">{{ t('bottom') }}</option>
            </select>
          </label>
        </div>

        <div class="selection-modes" :aria-label="t('selectionMode')">
          <button type="button" :class="{ active: selectionMode === 'point' }" :aria-pressed="selectionMode === 'point'" :title="`${t('point')} · 1`" @click="setSelectionMode('point')">
            <span class="mode-point" aria-hidden="true" /> <span>{{ t('point') }}</span><kbd>1</kbd>
          </button>
          <button type="button" :class="{ active: selectionMode === 'face' }" :aria-pressed="selectionMode === 'face'" :title="`${t('face')} · 3`" @click="setSelectionMode('face')">
            <span class="mode-face" aria-hidden="true" /> <span>{{ t('face') }}</span><kbd>3</kbd>
          </button>
          <button type="button" :class="{ active: selectionMode === 'object' }" :aria-pressed="selectionMode === 'object'" :title="`${t('body')} · 4`" @click="setSelectionMode('object')">
            <span class="mode-body" aria-hidden="true" /> <span>{{ t('body') }}</span><kbd>4</kbd>
          </button>
        </div>

        <canvas
          ref="canvasRef"
          class="gpu-canvas"
          role="img"
          tabindex="0"
          :aria-label="t('viewport')"
          @pointermove="markCustomView"
          @keydown="handleViewportKey"
        />
        <div class="view-cube-wrap" :class="{ 'with-dock': dockOpen }">
          <ViewCube :active-view="activeView" @view="setStandardView" />
        </div>
        <div v-if="dockOpen" class="cad-dock">
          <div class="dock-tabs" :aria-label="t('sidebar')">
            <button type="button" :aria-pressed="dockTab === 'scene'" :class="{ active: dockTab === 'scene' }" @click="dockTab = 'scene'">{{ t('scene') }}</button>
            <button type="button" :aria-pressed="dockTab === 'inspect'" :class="{ active: dockTab === 'inspect' }" @click="dockTab = 'inspect'">{{ t('inspect') }}</button>
            <button type="button" :aria-pressed="dockTab === 'parameters'" :class="{ active: dockTab === 'parameters' }" @click="dockTab = 'parameters'">{{ t('parameters') }}</button>
            <button class="dock-close" type="button" :aria-label="t('sidebar')" @click="dockOpen = false">×</button>
          </div>
          <SceneOutliner
            v-if="dockTab === 'scene'"
            :meshes="sceneRows"
            :selected-mesh-id="selectedMesh"
            :hovered-mesh-id="hoveredMesh"
            :locale="lang"
            :busy="rendering"
            @select="selectSceneMesh"
            @clear-selection="clearSelection"
            @toggle-visibility="setSceneMeshVisibility"
            @preselect="preselectSceneMesh"
            @focus="focusSceneMesh"
            @isolate="isolateSceneMesh"
            @reveal-source="revealSource"
            @highlight-source="highlightSource"
          />
          <InspectPanel
            v-else-if="dockTab === 'inspect'"
            :selection="currentInspection"
            :measurement="panelMeasurement"
            :measure-active="measureActive"
            :section-enabled="sectionEnabled"
            :section-axis="sectionAxis"
            :section-offset="sectionOffset"
            :section-min="sectionRange.min"
            :section-max="sectionRange.max"
            :section-step="sectionRange.step"
            :section-flip="sectionFlip"
            :source-reveal-enabled="sourceMatchesEditor"
            :locale="lang"
            @start-measure="startMeasure"
            @cancel-measure="cancelMeasure"
            @clear-measurement="clearMeasurement"
            @copy-measurement="copyMeasurement"
            @reveal-source="revealSource"
            @update:section-enabled="setSectionEnabled"
            @update:section-axis="setSectionAxis"
            @update:section-offset="setSectionOffset"
            @update:section-flip="setSectionFlip"
            @reset-section="resetSection"
          />
          <div v-else class="customizer-card">
            <CustomizerPanel
              :parameters="customizerParameters"
              :title="t('parameters')"
              :empty-label="t('noParameters')"
              @change="updateCustomizer"
            />
          </div>
        </div>
        <div v-if="selectedMesh !== null || isolated" class="selection-hud" role="status">
          <span v-if="selectedMesh !== null">
            <span class="selection-dot" aria-hidden="true" />
            {{ t('selected') }}: {{ t('object') }} {{ selectedMesh + 1 }} / {{ meshCount }}
            <span v-if="(selectedHit?.cycleCount ?? 0) > 1">
              · {{ t('depthCandidate') }} {{ (selectedHit?.cycleIndex ?? 0) + 1 }}/{{ selectedHit?.cycleCount }}
            </span>
          </span>
          <span v-else>{{ t('isolate') }}</span>
          <button v-if="selectedMesh !== null" type="button" :title="t('focus')" @click="focusSelection">/</button>
          <button type="button" :aria-pressed="isolated" :title="isolated ? t('unisolate') : t('isolate')" @click="toggleIsolate">.</button>
          <button v-if="selectedMesh !== null" type="button" :title="t('deselect')" @click="clearSelection">×</button>
        </div>
        <div v-if="rendering" class="rendering-badge" role="status"><span class="spinner" />{{ t('compiling') }} · {{ t(renderingQuality) }}</div>
        <div v-else-if="stale" class="stale-badge">{{ t('stale') }}</div>
        <div class="canvas-hint">{{ t('hint') }}</div>
      </section>
    </main>

    <div v-if="notice" class="toast" role="status">{{ notice }}</div>
    <CommandPalette
      :open="paletteOpen"
      :commands="paletteCommands"
      @close="paletteOpen = false"
      @execute="executeCommand"
    />
  </div>
</template>

<style>
:root {
  --bg: #111216;
  --surface: #1a1c22;
  --surface-raised: #22252d;
  --border: #30343e;
  --text: #eef0f5;
  --text-dim: #a4a9b5;
  --accent: #559dff;
  --accent-strong: #287eea;
  --hover: #2a2e38;
  --danger: #ff6b63;
  --warning: #f5bd55;
  --canvas-bg: #111318;
  --focus: #8ec1ff;
}

[data-theme="light"] {
  --bg: #f2f4f8;
  --surface: #ffffff;
  --surface-raised: #f7f8fa;
  --border: #d3d8e2;
  --text: #171a21;
  --text-dim: #596273;
  --accent: #176fd1;
  --accent-strong: #0c5eb9;
  --hover: #e8edf5;
  --danger: #be302c;
  --warning: #8a5b00;
  --canvas-bg: #171a20;
  --focus: #176fd1;
}

*, *::before, *::after { box-sizing: border-box; }
html, body, #app { width: 100%; height: 100%; margin: 0; }
body {
  overflow: hidden;
  background: var(--bg);
  color: var(--text);
  font-family: Inter, ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}
button, select, textarea, input { font: inherit; }
button, select { color: inherit; }
.sr-only {
  position: absolute !important; width: 1px; height: 1px; padding: 0; margin: -1px;
  overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0;
}
</style>

<style scoped>
.app { display: flex; flex-direction: column; min-height: 100vh; height: 100dvh; background: var(--bg); }
.topbar {
  z-index: 10; min-height: 44px; display: flex; align-items: center; justify-content: space-between;
  padding: 6px 12px; background: color-mix(in srgb, var(--surface) 94%, transparent);
  border-bottom: 1px solid var(--border); flex-shrink: 0;
}
.topbar-left, .topbar-right { display: flex; align-items: center; gap: 9px; }
.logo { color: var(--accent); filter: drop-shadow(0 0 8px color-mix(in srgb, var(--accent) 35%, transparent)); }
.brand { font-weight: 720; letter-spacing: -0.02em; }
.kernel-badge {
  padding: 2px 7px; border: 1px solid color-mix(in srgb, var(--accent) 35%, var(--border));
  border-radius: 999px; color: var(--accent); font-size: 0.68rem; font-weight: 650;
}
.icon-btn, .btn, .view-btn {
  min-height: 30px; border: 1px solid var(--border); border-radius: 7px; background: var(--surface-raised);
  color: var(--text); cursor: pointer; transition: background .12s, border-color .12s, transform .12s;
}
.icon-btn { min-width: 34px; padding: 4px 8px; }
.command-btn { display: flex; align-items: center; gap: 6px; padding-inline: 9px; font-size: .72rem; }
.command-btn kbd {
  padding: 1px 5px; border: 1px solid var(--border); border-radius: 4px;
  color: var(--text-dim); background: var(--bg); font: .62rem ui-monospace, monospace;
}
.lang-btn { font-size: .76rem; font-weight: 700; }
.icon-btn:hover, .btn:hover, .view-btn:hover { background: var(--hover); border-color: color-mix(in srgb, var(--accent) 45%, var(--border)); }
.icon-btn:focus-visible, .btn:focus-visible, .view-btn:focus-visible, .select:focus-visible,
.view-select:focus-visible, .splitter:focus-visible, .gpu-canvas:focus-visible, .code:focus-visible {
  outline: 2px solid var(--focus); outline-offset: 2px;
}
.no-gpu { flex: 1; display: grid; place-items: center; color: var(--danger); font-size: 1rem; padding: 40px; text-align: center; }
.main { flex: 1; min-height: 0; display: flex; overflow: hidden; }
.editor-panel {
  min-width: 300px; max-width: calc(100vw - 320px); display: flex; flex-direction: column;
  background: var(--surface); overflow: hidden;
}
.toolbar { display: flex; align-items: center; gap: 7px; padding: 7px 9px; border-bottom: 1px solid var(--border); }
.editor-toolbar { flex-wrap: wrap; }
.file-toolbar { padding-block: 5px; background: color-mix(in srgb, var(--surface-raised) 55%, var(--surface)); }
.btn { padding: 4px 10px; font-size: .76rem; white-space: nowrap; }
.btn:disabled { opacity: .55; cursor: progress; }
.btn-primary { background: var(--accent-strong); border-color: var(--accent-strong); color: white; font-weight: 700; }
.btn-primary:hover { background: var(--accent); }
.play { font-size: .65rem; margin-right: 2px; }
.auto-check { display: flex; align-items: center; gap: 5px; color: var(--text-dim); font-size: .75rem; cursor: pointer; }
.auto-check input { accent-color: var(--accent); }
.toolbar-divider { width: 1px; align-self: stretch; background: var(--border); margin-inline: 2px; }
.select, .view-select {
  height: 30px; max-width: 155px; border: 1px solid var(--border); border-radius: 7px;
  padding: 3px 25px 3px 8px; background: var(--surface-raised); font-size: .74rem;
}
.file-name { min-width: 0; margin-left: auto; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-dim); font-size: .7rem; }
.export-btn { padding-inline: 7px; color: var(--text-dim); font-size: .65rem; font-weight: 720; letter-spacing: .04em; }
.code {
  flex: 1; width: 100%; min-height: 120px; resize: none; border: 0; outline: 0; padding: 14px 15px;
  background: var(--bg); color: var(--text); caret-color: var(--accent);
  font-family: "JetBrains Mono", "SFMono-Regular", Consolas, monospace; font-size: .82rem; line-height: 1.58;
  tab-size: 2; white-space: pre; overflow: auto;
}
.message { margin: 7px 9px 0; padding: 8px 10px; border-radius: 7px; font: .74rem/1.45 ui-monospace, monospace; white-space: pre-wrap; overflow-wrap: anywhere; }
.error { color: var(--danger); background: color-mix(in srgb, var(--danger) 10%, transparent); border: 1px solid color-mix(in srgb, var(--danger) 35%, transparent); }
.warning { max-height: 88px; overflow: auto; color: var(--warning); background: color-mix(in srgb, var(--warning) 9%, transparent); border: 1px solid color-mix(in srgb, var(--warning) 28%, transparent); }
.stats { display: flex; flex-wrap: wrap; align-items: center; gap: 5px 11px; min-height: 31px; padding: 5px 10px; border-top: 1px solid var(--border); color: var(--text-dim); font-size: .68rem; }
.stats strong { color: var(--text); font-weight: 650; }
.status { margin-left: auto; }
.status.busy { color: var(--accent); }
.status.stale { color: var(--warning); }
.status.failed { color: var(--danger); }
.splitter {
  position: relative; z-index: 4; width: 7px; flex: 0 0 7px; cursor: col-resize;
  background: var(--surface); border-inline: 1px solid var(--border); touch-action: none;
}
.splitter span { position: absolute; width: 2px; height: 34px; inset: 50% auto auto 50%; transform: translate(-50%, -50%); border-radius: 2px; background: var(--border); }
.splitter:hover span, .splitter:focus-visible span { background: var(--accent); }
.canvas-panel { flex: 1; min-width: 0; position: relative; overflow: hidden; background: var(--canvas-bg); }
.gpu-canvas { width: 100%; height: 100%; display: block; touch-action: none; outline: 0; }
.viewer-toolbar {
  position: absolute; z-index: 3; top: 9px; left: 50%; transform: translateX(-50%);
  display: flex; align-items: center; gap: 5px; max-width: calc(100% - 18px); padding: 4px;
  border: 1px solid color-mix(in srgb, var(--border) 76%, transparent); border-radius: 9px;
  background: color-mix(in srgb, #171920 82%, transparent); backdrop-filter: blur(9px); color: #f2f4f8;
}
.view-btn { min-height: 28px; padding: 3px 8px; background: transparent; border-color: transparent; color: inherit; font-size: .7rem; }
.view-btn[aria-pressed="true"] { background: color-mix(in srgb, var(--accent) 24%, transparent); border-color: color-mix(in srgb, var(--accent) 42%, transparent); }
.view-btn.icon-only { min-width: 28px; padding-inline: 6px; font-size: .9rem; }
.view-select { max-width: 110px; height: 28px; color: #f2f4f8; background: #252932; border-color: #3b414d; }
.display-select { max-width: 90px; }
.view-cube-wrap { position: absolute; z-index: 2; top: 58px; right: 13px; }
.view-cube-wrap.with-dock { right: 326px; }
.selection-modes {
  position: absolute; z-index: 3; top: 58px; left: 12px; display: flex; gap: 3px; padding: 3px;
  border: 1px solid rgba(255,255,255,.12); border-radius: 8px; background: rgba(18,21,27,.84);
  color: #f2f4f8; box-shadow: 0 6px 24px rgba(0,0,0,.18); backdrop-filter: blur(8px);
}
.selection-modes button {
  min-height: 26px; display: flex; align-items: center; gap: 5px; padding: 3px 7px; border: 1px solid transparent;
  border-radius: 5px; background: transparent; color: inherit; cursor: pointer; font-size: .65rem;
}
.selection-modes button:hover { background: rgba(255,255,255,.07); }
.selection-modes button.active { background: rgba(85,157,255,.24); border-color: rgba(101,176,255,.48); }
.selection-modes kbd { color: rgba(255,255,255,.48); font: .56rem ui-monospace, monospace; }
.mode-point { width: 6px; height: 6px; border-radius: 50%; background: #6bc3ff; box-shadow: 0 0 5px #6bc3ff; }
.mode-face { width: 9px; height: 9px; border: 1px solid #6bc3ff; background: rgba(107,195,255,.2); transform: skewY(-18deg); }
.mode-body { width: 9px; height: 9px; border: 1px solid #6bc3ff; box-shadow: inset 2px -2px rgba(107,195,255,.28); }
.cad-dock {
  position: absolute; z-index: 4; top: 57px; right: 9px; bottom: 34px; width: min(304px, calc(100% - 18px));
  min-height: 0; display: flex; flex-direction: column; border-radius: 9px; background: var(--surface);
  box-shadow: 0 16px 44px rgba(0,0,0,.32);
}
.dock-tabs {
  flex: 0 0 32px; display: grid; grid-template-columns: 1fr 1fr 1.15fr 28px; gap: 2px; padding: 3px;
  border: 1px solid var(--border); border-bottom: 0; border-radius: 9px 9px 0 0; background: var(--surface-raised);
}
.dock-tabs button { border: 0; border-radius: 5px; background: transparent; color: var(--text-dim); cursor: pointer; font-size: .63rem; }
.dock-tabs button:hover { color: var(--text); background: var(--hover); }
.dock-tabs button.active { color: var(--text); background: color-mix(in srgb, var(--accent) 20%, var(--surface)); }
.dock-tabs .dock-close { font-size: 1rem; }
.cad-dock > :deep(.outliner), .cad-dock > :deep(.inspect-panel) { width: 100%; min-height: 0; flex: 1; border-radius: 0 0 9px 9px; }
.customizer-card { min-height: 0; flex: 1; overflow: auto; border: 1px solid var(--border); border-radius: 0 0 9px 9px; background: var(--surface); }
.selection-hud {
  position: absolute; z-index: 3; top: 98px; left: 12px; display: flex; align-items: center; gap: 7px;
  min-height: 30px; padding: 4px 6px 4px 9px; border: 1px solid rgba(255,255,255,.13);
  border-radius: 8px; background: rgba(18,21,27,.82); color: #f2f4f8;
  box-shadow: 0 6px 24px rgba(0,0,0,.18); backdrop-filter: blur(8px); font-size: .7rem;
}
.selection-hud > span { display: flex; align-items: center; gap: 6px; }
.selection-dot { width: 7px; height: 7px; border-radius: 50%; background: #65b0ff; box-shadow: 0 0 8px #65b0ff; }
.selection-hud button {
  width: 24px; height: 22px; padding: 0; border: 1px solid rgba(255,255,255,.14);
  border-radius: 5px; background: rgba(255,255,255,.06); color: inherit; cursor: pointer;
}
.selection-hud button:hover, .selection-hud button[aria-pressed="true"] { background: rgba(85,157,255,.28); border-color: rgba(101,176,255,.55); }
.rendering-badge, .stale-badge {
  position: absolute; z-index: 2; top: 55px; left: 50%; transform: translateX(-50%);
  display: flex; align-items: center; gap: 7px; padding: 6px 10px; border-radius: 999px;
  background: rgba(18, 21, 27, .82); color: #f2f4f8; border: 1px solid rgba(255,255,255,.12);
  backdrop-filter: blur(8px); font-size: .7rem; pointer-events: none;
}
.stale-badge { color: #ffd37c; }
.spinner { width: 11px; height: 11px; border: 2px solid rgba(255,255,255,.25); border-top-color: var(--accent); border-radius: 50%; animation: spin .7s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.canvas-hint {
  position: absolute; z-index: 2; bottom: 9px; left: 50%; transform: translateX(-50%);
  max-width: calc(100% - 18px); padding: 4px 8px; border-radius: 5px; color: rgba(255,255,255,.68);
  background: rgba(9,11,15,.48); font-size: .68rem; text-align: center; pointer-events: none;
}
.toast {
  position: fixed; z-index: 30; left: 50%; bottom: 22px; transform: translateX(-50%);
  padding: 8px 13px; border: 1px solid var(--border); border-radius: 8px;
  background: var(--surface-raised); box-shadow: 0 8px 32px rgba(0,0,0,.28); font-size: .78rem;
}

@media (max-width: 800px) {
  .kernel-badge { display: none; }
  .main { flex-direction: column; overflow: auto; }
  .editor-panel { width: 100% !important; min-width: 0; max-width: none; height: 46dvh; flex: 0 0 46dvh; }
  .splitter { display: none; }
  .canvas-panel { min-height: 46dvh; flex: 1 0 46dvh; border-top: 1px solid var(--border); }
  .view-btn span { display: none; }
  .viewer-toolbar { left: 8px; right: 8px; transform: none; justify-content: center; }
  .view-cube-wrap { top: 55px; right: 8px; transform: scale(.86); transform-origin: top right; }
  .view-cube-wrap.with-dock { display: none; }
  .selection-modes { top: 53px; left: 8px; }
  .selection-hud { top: 91px; left: 8px; }
  .cad-dock { top: 91px; right: 8px; bottom: 32px; }
  .stats { font-size: .64rem; }
  .status { width: 100%; margin-left: 0; }
}

@media (max-width: 480px) {
  .topbar { min-height: 40px; }
  .file-toolbar .btn { padding-inline: 7px; }
  .select { max-width: 128px; }
  .view-btn { padding-inline: 5px; }
  .view-select { max-width: 78px; }
  .display-select { max-width: 66px; }
  .command-btn kbd, .command-btn > span { display: none; }
  .command-btn { padding-inline: 7px; }
  .view-cube-wrap { display: none; }
  .selection-modes button > span:not(.mode-point, .mode-face, .mode-body), .selection-modes kbd { display: none; }
  .selection-hud { max-width: calc(100% - 16px); }
  .canvas-hint { white-space: normal; }
}

@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { scroll-behavior: auto !important; transition-duration: .01ms !important; animation-duration: .01ms !important; animation-iteration-count: 1 !important; }
}
</style>
