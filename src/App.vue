<script setup lang="ts">
import { modelGraphTextControls, isModelGraphText } from './services/modelGraphText'
import { editorBlocks, indentSelection, guideFitsIndent } from './services/editorBlocks'
import { formatCode } from './services/codeFormat'
import { highlightCode } from './services/codeHighlight'
import { computed, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import { flattenExportMeshes } from './services/meshExportAdapter'
import { exportMeshFormatCompressed, type MeshExportFormat } from './services/meshExportFormats'
import CommandPalette from './components/CommandPalette.vue'
import CustomizerPanel from './components/CustomizerPanel.vue'
import ExampleGallery from './components/ExampleGallery.vue'
import MechanicalGenerator from './features/MechanicalGenerator.vue'
import DirectModeler from './features/DirectModeler.vue'
import ScanPlanePanel from './features/ScanPlanePanel.vue'
import SvgPanel from './features/SvgPanel.vue'
import InspectPanel from './components/InspectPanel.vue'
import KeyboardShortcuts from './components/KeyboardShortcuts.vue'
import SceneOutliner from './components/SceneOutliner.vue'
import ViewCube from './components/ViewCube.vue'
import type {
  DistanceMeasurement as PanelMeasurement,
  InspectSelection,
  SceneMeshRow,
  SectionAxis,
  SourceProvenanceRow,
} from './components/cadPanels.types'
import type { GeometryQuality } from './core/build'
import { meshTransferables, type MeshData } from './core/mesh'
import { applyParameterPreset, captureParameterPreset, parameterTemplateHash, MAX_PARAMETER_PRESETS, type ParameterPreset } from './services/parameterPresets'
import { analyzeSurfaceArea, type GeometryComputeResult } from './services/geometryCompute'
import { AutoBuildScheduler } from './services/autoBuildScheduler'
import { BuildPerformanceHistory, type BuildPerformanceSample } from './services/buildPerformance'
import { EXAMPLE_CATALOG, EXAMPLES } from './data/examples'
import { assertExampleCatalog } from './services/exampleCatalog'
import type { CameraState } from './services/cameraHistory'
import {
  BuildCoordinator,
  type BuildCoordinatorState,
  type PublishedGeometryBuild,
} from './services/buildCoordinator'
import {
  buildPaletteDescriptors,
  buildShortcutHelpGroups,
  resolveKeyboardCommand,
  type CommandId,
  type CommandScope,
  type PaletteCommandId,
  type PaletteCommandRuntimeState,
} from './services/commandRegistry'
import { isPaletteCommandEnabled } from './services/commandSearch'
import { backendQuality } from './services/backendQuality'
import { clampEditorWidth, editorWidthBounds } from './services/layoutSizing'
import { nextRovingIndex, type RovingFocusKey } from './services/rovingFocus'
import {
  findLiteralMatches,
  replaceAllLiteral,
  replaceExpectedMatch,
  wrappedMatchIndex,
} from './services/textSearch'
import { buildBinaryStl, buildObj } from './services/meshExport'
import { inspectMesh } from './services/meshInspection'
import { RendererRecoveryGate } from './services/rendererRecoveryGate'
import { withSelectionSurfaces } from './services/meshSurfaceGroups'
import { SceneController } from './services/sceneController'
import { standardViewForCamera } from './services/viewportModel'
import { diagnosticFromBuildError, revealDiagnostic, type EditorDiagnostic } from './services/editorDiagnostics'
import {
  setStorageFailureHandler,
  storageGet,
  storageGetEnum,
  storageGetJSON,
  storageSet,
  storageSetJSON,
} from './services/safeStorage'
import { encodeCustomizerValue, extractCustomizerParameters, type CustomizerValue } from './services/scadCustomizer'
import { applySourceSplice } from './services/sourceSplice'
import { planScenePublication } from './services/scenePublication'
import {
  geometryExportEligibility,
  planGeometryPublication,
} from './services/buildPromotionPolicy'
import {
  MAX_WORKSPACE_FILE_NAME_LENGTH,
  MAX_WORKSPACE_SOURCE_LENGTH,
  updateWorkspaceDocument,
  workspaceDocumentsEqual,
  type WorkspaceDocumentSnapshot,
} from './services/workspaceDocument'
import type { BrowserWorkspacePersistence, WorkspaceRetryResult } from './services/workspacePersistence'
import { encodeWorkspaceShare } from './services/workspaceShare'
import { WebGPURenderer } from './services/webgpuRenderer'
import {
  assertThemeCatalog,
  resolveTheme,
  themeCanvasColor,
  THEME_CATALOG,
  THEME_SELECTIONS,
  type ThemeSelection,
} from './services/themePreferences'
import type {
  DisplayMode,
  DistanceMeasurement,
  PickHit,
  ProjectionMode,
  RendererLifecycleEvent,
  SelectionMode,
  StandardView,
} from './services/rendererContracts'

type Language = 'ru' | 'en'

assertExampleCatalog(EXAMPLE_CATALOG)
assertThemeCatalog(THEME_CATALOG)

const props = defineProps<{
  initialWorkspace: WorkspaceDocumentSnapshot
  initialWorkspaceDurable: boolean
  initialSharedImportPending: boolean
  workspacePersistence: BrowserWorkspacePersistence
}>()

const L: Record<Language, Record<string, string>> = {
  ru: {
    title: 'OpenSCAD Viewer',
    render: 'Собрать', auto: 'Авто', examples: 'Примеры',
    basic: 'Примитивы', csg: 'Настоящий CSG', house: 'Дом с модулями', tower: 'Параметрическая башня',
    open: 'Открыть', save: 'Сохранить', share: 'Поделиться',
    exportStl: 'Экспорт STL', exportObj: 'Экспорт OBJ', parameters: 'Параметры',
    noParameters: 'Добавьте верхнеуровневые переменные; диапазон слайдера: // [min:step:max]',
    openFile: 'Открыть файл OpenSCAD', saveFile: 'Сохранить исходник OpenSCAD', shareFile: 'Скопировать ссылку на модель',
    meshes: 'Объекты', triangles: 'Треугольники', volume: 'Объём', area: 'Площадь', time: 'Сборка',
    hint: 'Клик: выбрать · повторный клик: глубже · ЛКМ: вращение · ПКМ/Shift: панорама · колесо: масштаб · F: фокус',
    noGpu: 'WebGPU недоступен: редактор, сборка и экспорт работают, интерактивная 3D-сцена отключена.',
    backendWebGpu: 'WebGPU · интерактивный', backendHeadless: 'CPU · без 3D', transparencySorted: 'прозрачность по объектам',
    initializingViewport: 'Запускаем WebGPU…', retryRenderer: 'Повторить запуск 3D',
    find: 'Найти', replace: 'Заменить', replaceAll: 'Заменить все', matchCase: 'Учитывать регистр',
    previousMatch: 'Предыдущее совпадение', nextMatch: 'Следующее совпадение', closeSearch: 'Закрыть поиск',
    noMatches: 'Нет совпадений', tooManyMatches: 'Слишком много совпадений — уточните запрос',
    theme: 'Тема', systemTheme: 'Как в системе', darkTheme: 'Включить тёмную тему', lightTheme: 'Включить светлую тему',
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
    workerRestarted: 'Сборка не отвечала 30 секунд — Worker перезапущен',
    storageFailed: 'Не удалось сохранить данные: хранилище браузера недоступно или переполнено',
    format: 'Форматировать', saveBrowser: 'Сохранить в браузере', saveBrowserHelp: 'Сохранить текущий код в IndexedDB, заменив сохранённый черновик', savedBrowser: 'Сохранено в IndexedDB', failedBrowser: 'Не удалось сохранить в IndexedDB', unsavedDraft: 'Черновик не сохранён', retrySave: 'Повторить сохранение', savingDraft: 'Сохраняем…',
    storageConflict: 'Конфликт черновиков', useIndexed: 'Оставить версию IndexedDB', restoreDraft: 'Сохранить открытый черновик', exportDraft: 'Скачать открытый черновик',
    gpuLost: 'WebGPU перезапускается; восстанавливаем сцену…', gpuRecovered: 'Сцена WebGPU восстановлена',
    gpuRecoverFailed: 'Не удалось восстановить WebGPU после потери устройства', rendererError: 'Ошибка отрисовки',
    commands: 'Команды', commandHelp: 'Поиск действий', shortcuts: 'Горячие клавиши', display: 'Отображение',
    shaded: 'Заливка', edges: 'Рёбра', xray: 'Рентген',
    selected: 'Выбран', object: 'Объект', focus: 'Фокус', isolate: 'Изолировать',
    unisolate: 'Показать всё', deselect: 'Снять выбор',
    scene: 'Сцена', inspect: 'Инспектор', point: 'Точка', face: 'Грань', body: 'Тело',
    selectionMode: 'Режим выбора', sidebar: 'Боковая панель', closeSidebar: 'Закрыть боковую панель', measure: 'Измерить расстояние',
    cancelMeasure: 'Отменить измерение', flipSection: 'Перевернуть сечение', hideSelected: 'Скрыть выбранное', cycleSelectionMode: 'Следующий режим выбора',
    section: 'Плоскость сканирования', scanPlane: 'Срез', hidden: 'Скрыт',
    sourceStale: 'Сначала дождитесь сборки текущего исходника',
    needsModel: 'Сначала соберите модель', needsSelection: 'Сначала выберите объект',
    needsFullBuild: 'Нужна актуальная точная сборка', noPreviousView: 'История видов пока пуста',
    buildInProgress: 'Дождитесь завершения текущей сборки',
    depthCandidate: 'цель в глубине',
  },
  en: {
    title: 'OpenSCAD Viewer',
    render: 'Render', auto: 'Auto', examples: 'Examples',
    basic: 'Primitives', csg: 'Real CSG', house: 'Modular house', tower: 'Parametric tower',
    open: 'Open', save: 'Save', share: 'Share',
    exportStl: 'Export STL', exportObj: 'Export OBJ', parameters: 'Parameters',
    noParameters: 'Add top-level variables; slider metadata: // [min:step:max]',
    openFile: 'Open an OpenSCAD file', saveFile: 'Save OpenSCAD source', shareFile: 'Copy a link to this model',
    meshes: 'Objects', triangles: 'Triangles', volume: 'Volume', area: 'Surface', time: 'Build',
    hint: 'Click: select · repeat click: cycle deeper · LMB: orbit · RMB/Shift: pan · wheel: zoom · F: focus',
    noGpu: 'WebGPU is unavailable: editing, builds, and export still work; the interactive 3D viewport is disabled.',
    backendWebGpu: 'WebGPU · interactive', backendHeadless: 'CPU · no 3D', transparencySorted: 'object-sorted transparency',
    initializingViewport: 'Starting WebGPU…', retryRenderer: 'Retry 3D viewport',
    find: 'Find', replace: 'Replace', replaceAll: 'Replace all', matchCase: 'Match case',
    previousMatch: 'Previous match', nextMatch: 'Next match', closeSearch: 'Close search',
    noMatches: 'No matches', tooManyMatches: 'Too many matches — refine the query',
    theme: 'Theme', systemTheme: 'System', darkTheme: 'Use dark theme', lightTheme: 'Use light theme',
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
    workerRestarted: 'Build was unresponsive for 30 seconds — worker restarted',
    storageFailed: 'Could not save data: browser storage is unavailable or full',
    format: 'Format', saveBrowser: 'Save in browser', saveBrowserHelp: 'Save current code to IndexedDB, replacing the saved draft', savedBrowser: 'Saved to IndexedDB', failedBrowser: 'Could not save to IndexedDB', unsavedDraft: 'Draft is not saved', retrySave: 'Retry save', savingDraft: 'Saving…',
    storageConflict: 'Draft conflict', useIndexed: 'Keep IndexedDB version', restoreDraft: 'Save current draft', exportDraft: 'Download open draft',
    gpuLost: 'WebGPU restarted; restoring the scene…', gpuRecovered: 'WebGPU scene restored',
    gpuRecoverFailed: 'WebGPU could not recover after device loss', rendererError: 'Rendering failed',
    commands: 'Commands', commandHelp: 'Search actions', shortcuts: 'Keyboard shortcuts', display: 'Display',
    shaded: 'Shaded', edges: 'Edges', xray: 'X-ray',
    selected: 'Selected', object: 'Object', focus: 'Focus', isolate: 'Isolate',
    unisolate: 'Show all', deselect: 'Deselect',
    scene: 'Scene', inspect: 'Inspect', point: 'Point', face: 'Face', body: 'Body',
    selectionMode: 'Selection mode', sidebar: 'Sidebar', closeSidebar: 'Close sidebar', measure: 'Measure distance',
    cancelMeasure: 'Cancel measurement', flipSection: 'Flip section', hideSelected: 'Hide selected', cycleSelectionMode: 'Next selection mode',
    section: 'Scanning plane', scanPlane: 'Section', hidden: 'Hidden',
    sourceStale: 'Wait for the current source to finish building first',
    needsModel: 'Build a model first', needsSelection: 'Select an object first',
    needsFullBuild: 'An up-to-date full build is required', noPreviousView: 'View history is empty',
    buildInProgress: 'Wait for the current build to finish',
    depthCandidate: 'depth target',
  },
}

const lang = ref<Language>(storageGetEnum<Language>('scad-lang', ['ru', 'en'], 'ru'))
const legacyThemeRaw = storageGet('scad-theme')
const legacyTheme: ThemeSelection = legacyThemeRaw === 'dark' || legacyThemeRaw === 'light' ? legacyThemeRaw : 'system'
const preVersionedTheme = storageGetEnum<ThemeSelection>('scad-theme-selection', THEME_SELECTIONS, legacyTheme)
const themeSelection = ref(storageGetEnum<ThemeSelection>('scad-theme-v1', THEME_SELECTIONS, preVersionedTheme))
const themeMediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
const systemPrefersDark = ref(themeMediaQuery.matches)
const isDark = computed(() => resolveTheme(themeSelection.value, systemPrefersDark.value).scheme === 'dark')
const workspaceDocument = ref<WorkspaceDocumentSnapshot>(props.initialWorkspace)
const code = ref(props.initialWorkspace.source)
const autoRender = ref(storageGet('scad-auto') !== 'false')
const editorWidth = ref(clamp(Number(storageGet('scad-editor-width')) || 440, 300, 820))

const canvasRef = ref<HTMLCanvasElement | null>(null)
const editorRef = ref<HTMLTextAreaElement | null>(null)
const highlightRef = ref<HTMLPreElement | null>(null)
const selectedEditorName = ref('')
const blocks = computed(() => editorBlocks(code.value))
const foldedLines = ref(new Set<number>())
const blockColors = ['#7999e8', '#c792ea', '#d7a457', '#55bba4', '#d87d9d']
const editorRows = computed(() => {
  const highlighted = highlightCode(code.value, selectedEditorName.value).replace(/\n$/, '').split('\n')
  const lines = code.value.split('\n')
  const guideEnds = new Map(blocks.value.map(b => {
    let end = b.end
    while(end > b.start && !guideFitsIndent(lines[end] ?? '', b.column)) end--
    return [b.start, end]
  }))
  const byStart = new Map(blocks.value.map(b=>[b.start,b]))
  let active: typeof blocks.value = []
  return lines.map((text, line) => {
    active = active.filter(b=>b.end>=line)
    const row={line,text,html:highlighted[line]??'',block:byStart.get(line),guides:active.filter(b=>guideFitsIndent(text,b.column)).map(b=>({...b,last:line===guideEnds.get(b.start)}))}
    if(row.block)active.push(row.block)
    return row
  })
})
const foldedRows = computed(() => {
  let hiddenThrough=-1
  return editorRows.value.filter(row=>{
    if(row.line<=hiddenThrough)return false
    if(row.block && foldedLines.value.has(row.line))hiddenThrough=row.block.end
    return true
  })
})
function toggleFold(line: number) {
  const next = new Set(foldedLines.value); if(next.has(line)) next.delete(line); else next.add(line)
  foldedLines.value = next
}
async function editFoldedLine(line: number) {
  foldedLines.value = new Set(); await nextTick()
  const editor = editorRef.value; if(!editor) return
  const offset = code.value.split('\n').slice(0,line).reduce((n,s)=>n+s.length+1,0)
  editor.focus(); editor.setSelectionRange(offset,offset)
  editor.scrollTop = Math.max(0,line * parseFloat(getComputedStyle(editor).lineHeight)-40)
  syncHighlightScroll()
}
const highlightedCode = computed(() => highlightCode(code.value, selectedEditorName.value))
const lineNumbersRef = ref<HTMLDivElement | null>(null)
const guidesRef = ref<HTMLDivElement | null>(null)
const editorLineCount = computed(() => code.value.split('\n').length)
watch(code, () => { foldedLines.value = new Set(); void nextTick(syncHighlightScroll) }, { flush: 'post' })
function syncHighlightScroll() {
  if (!editorRef.value || !highlightRef.value) return
  const layer = highlightRef.value.firstElementChild as HTMLElement | null
  if(layer) layer.style.transform = `translate(${-editorRef.value.scrollLeft}px, ${-editorRef.value.scrollTop}px)`
  if (guidesRef.value) guidesRef.value.style.transform = `translate(${-editorRef.value.scrollLeft}px, ${-editorRef.value.scrollTop}px)`
  if (lineNumbersRef.value) lineNumbersRef.value.style.transform = `translateY(${-editorRef.value.scrollTop}px)`
}
const findInputRef = ref<HTMLInputElement | null>(null)
const fileInputRef = ref<HTMLInputElement | null>(null)
const mainRef = ref<HTMLElement | null>(null)
const error = ref('')
const editorDiagnostic = ref<EditorDiagnostic | null>(null)
const warnings = ref<string[]>([])
const meshCount = ref(0)
const triangleCount = ref(0)
const volume = ref(0)
const surfaceArea = ref(0)
const renderDuration = ref(0)
const performanceHistory = new BuildPerformanceHistory()
const performanceSamples = shallowRef<BuildPerformanceSample[]>([])
const performanceLast = computed(() => performanceSamples.value.at(-1))
const performanceMs = (value: number | null | undefined) => value == null ? '—' : `${value.toFixed(1)} ms`
const performanceRows = computed(() => {
  const sample = performanceSamples.value.at(-1)
  if (!sample) return []
  return [
    [lang.value === 'ru' ? 'Разбор исходника' : 'Parse', performanceMs(sample.phases.parseMs)],
    [lang.value === 'ru' ? 'Инициализация ядра' : 'Kernel initialization', performanceMs(sample.phases.initializeMs)],
    [lang.value === 'ru' ? 'Вычисление геометрии' : 'Geometry evaluation', performanceMs(sample.phases.evaluateMs)],
    [lang.value === 'ru' ? 'Анализ геометрии' : 'Geometry analysis', performanceMs(sample.phases.analyzeMs)],
    [lang.value === 'ru' ? 'Вся работа в Worker' : 'Worker total', performanceMs(sample.workerMs)],
    [lang.value === 'ru' ? 'Запрос → проверенный результат' : 'Request → validated result', performanceMs(sample.hostMs)],
    [lang.value === 'ru' ? 'Публикация сцены (CPU)' : 'Scene publication (CPU)', performanceMs(sample.publicationMs)],
    [lang.value === 'ru' ? 'Публикация → отправка кадра' : 'Publication → frame submission', performanceMs(sample.publicationToSubmitMs)],
    [lang.value === 'ru' ? 'Правка → отправка кадра' : 'Edit → frame submission', performanceMs(sample.editToSubmitMs)],
    [lang.value === 'ru' ? 'Переданные буферы' : 'Transferred buffers', `${(sample.transferBytes / 1024).toFixed(1)} KiB`],
    [lang.value === 'ru' ? 'Загрузка вершин и индексов на GPU' : 'Vertex/index GPU upload', sample.upload ? `${(sample.upload.geometryUploadBytes / 1024).toFixed(1)} KiB` : '—'],
    [lang.value === 'ru' ? 'Новые буферы вершин и индексов' : 'New vertex/index buffers', sample.upload?.geometryBuffersCreated ?? '—'],
    [lang.value === 'ru' ? 'Объекты с повторно использованной геометрией' : 'Entities reusing geometry', sample.upload?.reusedEntities ?? '—'],
  ]
})

let nextPerformanceFrameToken = 1
let previousBuildCounters = { builds: 0, superseded: 0, workerStarts: 0, hardRestarts: 0 }
let pendingPerformanceFrame: { frameToken: number; historyToken: number } | null = null
const buildCounters = shallowRef({ builds: 0, superseded: 0, workerStarts: 0, hardRestarts: 0 })
const gpuOk = ref(false)
const rendererInitializing = ref(false)
const rendererUnavailableMessage = ref('')
const currentBackendQuality = computed(() => backendQuality(gpuOk.value, displayMode.value))
const rendering = ref(false)
const renderingQuality = ref<GeometryQuality>('full')
const renderedQuality = ref<GeometryQuality>('full')
const renderedSource = ref('')
const notice = ref('')
const fileName = ref(props.initialWorkspace.fileName)
const workspacePersistenceStatus = ref<'saved' | 'saving' | 'error'>(
  props.initialWorkspaceDurable ? 'saved' : 'error',
)
const workspaceConflict = ref(props.workspacePersistence.hasConflict)
const exampleGalleryOpen = ref(false)
const mechanicalGeneratorOpen = ref(false)
const directModelerOpen = ref(false)
const projection = ref<ProjectionMode>('perspective')
const gridVisible = ref(true)
const standardView = ref<StandardView>('iso')
const activeView = ref<StandardView | 'custom'>('iso')
const cameraYaw = ref(Math.PI / 4)
const cameraPitch = ref(Math.atan(1 / Math.sqrt(2)))
/** Projection to restore once the camera leaves an orthographic face-view snap. */
const projectionBeforeFaceSnap = ref<ProjectionMode | null>(null)
const displayMode = ref<DisplayMode>('shaded')
const sceneController = new SceneController<PickHit>()
const sceneState = shallowRef(sceneController.state)
sceneController.subscribe(state => { sceneState.value = state })
const selectedMesh = computed({
  get: () => sceneState.value.selectedIndex,
  set: (selectedIndex: number | null) => { sceneController.update({ selectedIndex }) },
})
const selectedHit = computed({
  get: () => sceneState.value.selectedHit,
  set: (selectedHit: PickHit | null) => { sceneController.update({ selectedHit }) },
})
const hoveredHit = ref<PickHit | null>(null)
const isolated = computed({
  get: () => sceneState.value.isolated,
  set: (isolated: boolean) => { sceneController.update({ isolated }) },
})
const paletteOpen = ref(false)
const shortcutHelpOpen = ref(false)
const canPreviousView = ref(false)
const commandMru = ref<string[]>(readCommandMru())
const sceneMeshes = computed({
  get: () => sceneState.value.meshes,
  set: (meshes: MeshData[]) => { sceneController.update({ meshes }) },
})
const meshVisibility = computed({
  get: () => sceneState.value.visibility,
  set: (visibility: boolean[]) => { sceneController.update({ visibility }) },
})
const selectionMode = ref<SelectionMode>('face')
const measurement = ref<DistanceMeasurement | null>(null)
const measureActive = ref(false)
const sectionEnabled = ref(false)
const sectionAxis = ref<SectionAxis>('z')
const sectionOffset = ref(0)
const sectionFlip = ref(false)
const scanPanelOpen = ref(false)
const scanToggleRef = ref<HTMLButtonElement | null>(null)
let sectionInitialized = false
const dockTab = ref<'scene' | 'inspect' | 'parameters'>('scene')
const dockOpen = ref(true)
const findOpen = ref(false)
const replaceOpen = ref(false)
const findQuery = ref('')
const findReplacement = ref('')
const findCaseSensitive = ref(false)
const activeFindIndex = ref(-1)
const findResult = computed(() => findLiteralMatches(code.value, findQuery.value, {
  caseSensitive: findCaseSensitive.value,
}))
const findStatus = computed(() => {
  if (findResult.value.truncated) return t('tooManyMatches')
  const count = findResult.value.matches.length
  return count ? `${Math.max(0, activeFindIndex.value) + 1}/${count}` : t('noMatches')
})
watch([findQuery, findCaseSensitive, code], () => {
  activeFindIndex.value = findResult.value.matches.length ? 0 : -1
})
const dockToggleRef = ref<HTMLButtonElement | null>(null)
const editorMaxWidth = ref(820)

const dockTabs = ['scene', 'inspect', 'parameters'] as const
function handleDockTabKeydown(event: KeyboardEvent) {
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
  event.preventDefault()
  const current = dockTabs.indexOf(dockTab.value)
  const next = nextRovingIndex(current, dockTabs.length, event.key as RovingFocusKey)
  dockTab.value = dockTabs[next]
  void nextTick(() => document.getElementById(`dock-tab-${dockTab.value}`)?.focus())
}

function closeDock() {
  dockOpen.value = false
  void nextTick(() => dockToggleRef.value?.focus())
}

const computeResult = shallowRef<GeometryComputeResult | null>(null)
const computeBusy = ref(false)
const computeError = ref('')
let computeAbort: AbortController | null = null
function cancelGeometryAnalysis() {
  computeAbort?.abort()
  computeAbort = null
  computeBusy.value = false
  computeResult.value = null
  computeError.value = ''
}
watch([selectedMesh, sceneMeshes, code], cancelGeometryAnalysis, { flush: 'sync' })

async function runGeometryAnalysis() {
  const mesh = selectedMesh.value === null ? null : sceneMeshes.value[selectedMesh.value]
  if (!mesh || !canExport.value || computeBusy.value) return
  cancelGeometryAnalysis()
  const controller = new AbortController()
  computeAbort = controller
  computeBusy.value = true
  try {
    const result = await analyzeSurfaceArea(mesh, controller.signal)
    if (computeAbort === controller && !controller.signal.aborted) computeResult.value = result
  } catch {
    if (computeAbort === controller && !controller.signal.aborted) computeError.value = lang.value === 'ru' ? 'Не удалось проанализировать геометрию.' : 'Could not analyze geometry.'
  } finally {
    if (computeAbort === controller) { computeBusy.value = false; computeAbort = null }
  }
}

const compactControls = computed(() => isModelGraphText(code.value) ? modelGraphTextControls(code.value) : {parameters: [], errors: []})
const customizerParameters = computed(() => {
  if (!isModelGraphText(code.value)) return extractCustomizerParameters(code.value)
  return compactControls.value.parameters
})
const presetName = ref('')
const presetSelection = ref('')
const presetError = ref('')
const presetUndo = shallowRef<{ before: string; after: string } | null>(null)
const parameterPresets = computed(() => workspaceDocument.value.parameterPresets)
const selectedPreset = computed(() => parameterPresets.value.find(item => item.id === presetSelection.value) ?? parameterPresets.value[0])
const currentParameterTemplate = computed(() => {
  if (!parameterPresets.value.length) return null
  try { return parameterTemplateHash(code.value) } catch { return null }
})
const presetCompatible = computed(() => !!selectedPreset.value && selectedPreset.value.templateHash === currentParameterTemplate.value)

const stale = computed(() => renderedSource.value !== '' && (renderedSource.value !== code.value || renderedQuality.value === 'preview'))
const sourceMatchesEditor = computed(() => renderedSource.value !== '' && renderedSource.value === code.value)
const exportEligibility = computed(() => geometryExportEligibility({
  meshCount: sceneMeshes.value.length,
  rendering: rendering.value,
  hasError: Boolean(error.value),
  renderedQuality: renderedQuality.value,
  renderedSource: renderedSource.value,
  currentSource: code.value,
}))
const canExport = computed(() => exportEligibility.value.allowed)
const statusText = computed(() => rendering.value
  ? t('compiling')
  : error.value ? t('failed') : stale.value ? t('stale') : t('ready'))
const sceneRows = computed<SceneMeshRow[]>(() => sceneMeshes.value.map((mesh, index) => {
  const meshId = mesh.entityId ?? index
  const grouped = new Map<string | number, SourceProvenanceRow>()
  for (const run of mesh.provenance) {
    if (!run.source) continue
    const sourceKey = run.source.instanceId ?? run.source.operationId ?? run.source.id
    const existing = grouped.get(sourceKey)
    const triangleCount = run.triangleEnd - run.triangleStart
    if (existing) existing.triangleCount = (existing.triangleCount ?? 0) + triangleCount
    else {
      const location = lineAndColumn(renderedSource.value || code.value, run.source.start)
      grouped.set(sourceKey, {
        id: `${meshId}:${sourceKey}`,
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
    id: meshId,
    name: `${t('object')} ${index + 1}`,
    visible: meshVisibility.value[index] !== false,
    triangleCount: mesh.indices.length / 3,
    color: mesh.color,
    sources: [...grouped.values()],
  }
}))
const sceneMeshId = (index: number | null) => index === null
  ? null
  : sceneMeshes.value[index]?.entityId ?? index
const selectedSceneMeshId = computed(() => sceneMeshId(selectedMesh.value))
const hoveredMesh = computed(() => sceneMeshId(hoveredHit.value?.meshIndex ?? null))
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
      id: `${mesh.entityId ?? index}:${hit.source.instanceId ?? hit.source.operationId ?? hit.source.originalId}`,
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
    meshId: mesh.entityId ?? index,
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
function isSectionMeshVisible(index: number) {
  return meshVisibility.value[index] !== false && (!isolated.value || index === selectedMesh.value)
}
const sectionRange = computed(() => {
  const axis = sectionAxis.value === 'x' ? 0 : sectionAxis.value === 'y' ? 1 : 2
  let min = Infinity
  let max = -Infinity
  sceneMeshes.value.forEach((mesh, index) => {
    if (!isSectionMeshVisible(index)) return
    const bounds = inspectMesh(mesh, index).bounds
    if (!bounds) return
    min = Math.min(min, bounds.min[axis])
    max = Math.max(max, bounds.max[axis])
  })
  if (!Number.isFinite(min) || !Number.isFinite(max)) return { min: -100, max: 100, step: 0.1 }
  const span = Math.max(0.001, max - min)
  return { min, max, step: Math.max(0.001, span / 500) }
})
const sectionAvailable = computed(() => gpuOk.value && sceneMeshes.value.some((_, index) => isSectionMeshVisible(index)))
watch(sectionRange, syncSectionRange)
const paletteCommands = computed(() => {
  const hasVisibleModel = sceneMeshes.value.some((_, index) => meshVisibility.value[index] !== false)
  const hasSelection = selectedMesh.value !== null
  const state: Partial<Record<PaletteCommandId, PaletteCommandRuntimeState>> = {
    render: {
      enabled: !rendering.value,
      disabledReason: t('buildInProgress'),
    },
    'export-stl': {
      enabled: canExport.value,
      disabledReason: t('needsFullBuild'),
    },
    'export-obj': {
      enabled: canExport.value,
      disabledReason: t('needsFullBuild'),
    },
    focus: {
      enabled: hasSelection || hasVisibleModel,
      disabledReason: t('needsModel'),
    },
    isolate: {
      label: isolated.value ? t('unisolate') : t('isolate'),
      enabled: hasSelection,
      disabledReason: t('needsSelection'),
    },
    deselect: {
      enabled: hasSelection,
      disabledReason: t('needsSelection'),
    },
    fit: {
      enabled: hasVisibleModel,
      disabledReason: t('needsModel'),
    },
    'previous-view': {
      enabled: canPreviousView.value,
      disabledReason: t('noPreviousView'),
    },
    projection: {
      label: projection.value === 'perspective' ? t('orthographic') : t('perspective'),
    },
    measure: {
      enabled: hasVisibleModel || measureActive.value,
      disabledReason: t('needsModel'),
    },
    section: {
      enabled: sectionAvailable.value || sectionEnabled.value,
      disabledReason: t('needsModel'),
    },
  }
  return buildPaletteDescriptors({
    resolveLabel: key => t(key),
    aliasResolvers: [
      key => L.ru[key] ?? key,
      key => L.en[key] ?? key,
    ],
    state,
    mru: commandMru.value,
  })
})
const shortcutPlatform = /Mac|iPhone|iPad/.test(navigator.platform) ? 'mac' : 'windows-linux'
const shortcutHelpGroups = computed(() => buildShortcutHelpGroups(key => t(key), shortcutPlatform))

/** Last-resort watchdog: a building coordinator that stays completely silent
 * (no state change, no progress heartbeat) for this long is assumed wedged
 * inside one statement; the worker is replaced and the build retried once. */
const WATCHDOG_TIMEOUT_MS = 30_000
/** Rate limit for the storage-write-failure notice. */
const STORAGE_NOTICE_THROTTLE_MS = 10_000

let renderer: WebGPURenderer | null = null
let buildCoordinator: BuildCoordinator | null = null
let watchdogTimer: ReturnType<typeof setTimeout> | null = null
let watchdogToken = 0
let watchdogRetried = false
let lastStorageNotice = 0
const autoBuildScheduler = new AutoBuildScheduler({ build: quality => { if (componentActive && autoRender.value) doRender(quality) } })
let buildGeneration = 0
let storageDebounce: ReturnType<typeof setTimeout> | null = null
let workspaceEditGeneration = 0
let durableWorkspaceGeneration = props.initialWorkspaceDurable ? 0 : -1
let workspaceSourceValid = true
let workspaceFileNameValid = true
let beforeUnloadAttached = false
let sharedImportPending = props.initialSharedImportPending
let restoringWorkspace = false
let applyingWorkspaceReplacement = false
let componentActive = true
let noticeTimeout: ReturnType<typeof setTimeout> | null = null
let rendererRecoveryToken = 0
let activeRendererRecoveryToken: number | null = null
const rendererRecoveryGate = new RendererRecoveryGate()
let rendererErrorMessage = ''
let resizing = false
let layoutResizeObserver: ResizeObserver | null = null
let fitNextRender = false
const buildSources = new Map<number, string>()

const t = (key: string) => L[lang.value][key] ?? key
const formatNumber = (value: number, digits = 0) => value.toLocaleString(lang.value, { maximumFractionDigits: digits })

onMounted(async () => {
  setStorageFailureHandler(reportStorageFailure)
  themeMediaQuery.addEventListener('change', handleSystemThemeChange)
  applyPreferences()
  updateBeforeUnloadGuard()
  window.addEventListener('pagehide', flushWorkspacePersistence)
  window.addEventListener('pointerup', endParameterGesture)
  window.addEventListener('pointercancel', endParameterGesture)
  window.addEventListener('blur', endParameterGesture)
  document.addEventListener('visibilitychange', handleVisibilityChange)
  if (mainRef.value) {
    const syncEditorBounds = () => {
      const width = mainRef.value?.getBoundingClientRect().width ?? 0
      const bounds = editorWidthBounds(width)
      editorMaxWidth.value = bounds.max
      editorWidth.value = clampEditorWidth(editorWidth.value, width)
    }
    syncEditorBounds()
    layoutResizeObserver = new ResizeObserver(syncEditorBounds)
    layoutResizeObserver.observe(mainRef.value)
  }
  window.addEventListener('keydown', handleGlobalKey)
  try {
    startBuildCoordinator()
    doRender('full')
  } catch {
    error.value = t('workerError')
  }
  await initializeViewportRenderer()
})

async function initializeViewportRenderer() {
  const canvas = canvasRef.value
  if (!canvas || !componentActive || rendererInitializing.value) return
  rendererInitializing.value = true
  rendererUnavailableMessage.value = ''
  const nextRenderer = new WebGPURenderer()
  const ok = await nextRenderer.init(canvas)
  if (!componentActive) {
    nextRenderer.destroy()
    return
  }
  if (!ok) {
    const status = nextRenderer.currentStatus
    rendererUnavailableMessage.value = status.status === 'unavailable'
      ? status.message
      : status.status === 'error' ? status.error.message : t('noGpu')
    nextRenderer.destroy()
    renderer = null
    gpuOk.value = false
    rendererInitializing.value = false
    return
  }

  renderer?.destroy()
  renderer = nextRenderer
  bindRendererCallbacks(nextRenderer)
  nextRenderer.setDisplayMode(displayMode.value)
  nextRenderer.setBackgroundColor(themeCanvasColor(resolveTheme(themeSelection.value, systemPrefersDark.value)))
  nextRenderer.setSelectionMode(selectionMode.value)
  nextRenderer.setGridVisible(gridVisible.value)
  if (sceneMeshes.value.length) {
    nextRenderer.setMeshes(sceneMeshes.value)
    nextRenderer.setMeshVisibilityBatch(meshVisibility.value)
    if (selectedMesh.value !== null) nextRenderer.selectMesh(selectedMesh.value)
    applySection()
    syncSourceHighlightFromEditor()
    nextRenderer.fitView()
  }
  gpuOk.value = true
  rendererInitializing.value = false
}

function bindRendererCallbacks(instance: WebGPURenderer) {
  instance.onSelectionChange = (index, isIsolated, hit) => {
    if (activeRendererRecoveryToken !== null) return
    sceneController.applyRendererSelection(index, isIsolated, hit)
  }
  instance.onHoverChange = hit => {
    if (activeRendererRecoveryToken !== null) return
    hoveredHit.value = hit
  }
  instance.onMeasurementChange = (value, active) => {
    measurement.value = value
    measureActive.value = active
  }
  instance.onCameraHistoryChange = available => { canPreviousView.value = available }
  instance.onFrameSubmitted = (token, at) => {
    if (instance !== renderer) return
    if (!pendingPerformanceFrame || pendingPerformanceFrame.frameToken !== token) return
    if (performanceHistory.submitted(pendingPerformanceFrame.historyToken, at)) performanceSamples.value = performanceHistory.snapshot()
  }
  instance.onCameraChange = syncCameraState
  instance.onStatusChange = event => handleRendererStatus(instance, event)
  canPreviousView.value = instance.canGoToPreviousView
  syncCameraState(instance.getCameraState())
}

onUnmounted(() => {
  cancelGeometryAnalysis()
  layoutResizeObserver?.disconnect()
  layoutResizeObserver = null
  rendererRecoveryToken++
  activeRendererRecoveryToken = null
  rendererRecoveryGate.reset()
  autoBuildScheduler.cancel()
  if (storageDebounce) {
    clearTimeout(storageDebounce)
    void persistWorkspaceNow()
  }
  componentActive = false
  if (noticeTimeout) clearTimeout(noticeTimeout)
  disarmWatchdog()
  setStorageFailureHandler(null)
  buildCoordinator?.dispose()
  buildCoordinator = null
  window.removeEventListener('keydown', handleGlobalKey)
  themeMediaQuery.removeEventListener('change', handleSystemThemeChange)
  window.removeEventListener('pagehide', flushWorkspacePersistence)
  window.removeEventListener('pointerup', endParameterGesture)
  window.removeEventListener('pointercancel', endParameterGesture)
  window.removeEventListener('blur', endParameterGesture)
  detachBeforeUnloadGuard()
  document.removeEventListener('visibilitychange', handleVisibilityChange)
  if (renderer) {
    renderer.onSelectionChange = null
    renderer.onHoverChange = null
    renderer.onMeasurementChange = null
    renderer.onCameraHistoryChange = null
    renderer.onCameraChange = null
    renderer.onStatusChange = null
    renderer.onFrameSubmitted = null
  }
  renderer?.destroy()
  renderer = null
  void props.workspacePersistence.close()
})

function handleRendererStatus(instance: WebGPURenderer, event: RendererLifecycleEvent) {
  if (renderer !== instance) return
  if (event.status === 'device-lost') {
    showNotice(t('gpuLost'))
    if (rendererRecoveryGate.registerDeviceLoss() === 'start') void recoverRenderer(instance)
  } else if (event.status === 'error') {
    rendererErrorMessage = `${t('rendererError')}: ${event.error.message}`
    if (!rendering.value) error.value = rendererErrorMessage
  } else if (event.status === 'ready' && rendererErrorMessage && error.value === rendererErrorMessage) {
    error.value = ''
    rendererErrorMessage = ''
  }
}

interface RendererRecoveryNavigationSnapshot {
  camera: CameraState
  history: ReturnType<WebGPURenderer['getCameraHistorySnapshot']>
}

async function recoverRenderer(
  instance: WebGPURenderer,
  navigation: RendererRecoveryNavigationSnapshot = {
    camera: instance.getCameraState(),
    history: instance.getCameraHistorySnapshot(),
  },
) {
  const canvas = canvasRef.value
  if (!canvas || renderer !== instance) {
    rendererRecoveryGate.reset()
    return
  }
  const token = ++rendererRecoveryToken
  activeRendererRecoveryToken = token
  // Triangle identities and hover ownership are renderer-local. Object
  // selection can be restored by scene identity, but stale surface hits cannot.
  selectedHit.value = null
  hoveredHit.value = null
  let ready = false
  let failureMessage = t('gpuRecoverFailed')

  try {
    const recovered = await instance.init(canvas)
    if (token !== rendererRecoveryToken || renderer !== instance) return
    if (!recovered) return
    if (instance.currentStatus.status === 'device-lost' && !rendererRecoveryGate.mustDeferReady) {
      rendererRecoveryGate.registerDeviceLoss()
    }
    if (rendererRecoveryGate.mustDeferReady) return

    instance.setDisplayMode(displayMode.value)
    instance.setBackgroundColor(themeCanvasColor(resolveTheme(themeSelection.value, systemPrefersDark.value)))
    instance.setSelectionMode(selectionMode.value)
    instance.setGridVisible(gridVisible.value)
    // Builds can complete while adapter/device acquisition is pending. The CPU
    // scene and Vue state are authoritative, so re-read them after the await.
    const currentVisibility = [...meshVisibility.value]
    const selectionCandidate = selectedMesh.value
    const currentSelection = selectionCandidate !== null
      && Number.isInteger(selectionCandidate)
      && selectionCandidate >= 0
      && selectionCandidate < sceneMeshes.value.length
      && currentVisibility[selectionCandidate] !== false
      ? selectionCandidate
      : null
    const currentIsolation = currentSelection !== null && isolated.value
    const currentMeasurement = measurement.value
    const currentMeasureActive = measureActive.value
    selectedMesh.value = currentSelection
    isolated.value = currentIsolation
    if (sceneMeshes.value.length) {
      instance.setMeshes(sceneMeshes.value)
      instance.setMeshVisibilityBatch(currentVisibility)
      if (currentSelection !== null) {
        instance.selectMesh(currentSelection)
        if (currentIsolation) instance.toggleIsolateSelection()
      }
    }
    instance.restoreCameraState(navigation.camera)
    if (!instance.restoreCameraHistory(navigation.history)) throw new Error(t('gpuRecoverFailed'))
    instance.restoreMeasurement(currentMeasurement, currentMeasureActive)
    applySection()
    syncSourceHighlightFromEditor()
    if (instance.currentStatus.status === 'device-lost' && !rendererRecoveryGate.mustDeferReady) {
      rendererRecoveryGate.registerDeviceLoss()
    }
    if (!rendererRecoveryGate.mustDeferReady) ready = true
  } catch (caught) {
    failureMessage = caught instanceof Error ? caught.message : t('gpuRecoverFailed')
  } finally {
    if (activeRendererRecoveryToken === token) activeRendererRecoveryToken = null
    if (token !== rendererRecoveryToken || renderer !== instance) return

    const completion = rendererRecoveryGate.completeAttempt()
    if (completion === 'retry') {
      void recoverRenderer(instance, navigation)
      return
    }

    if (!ready || completion === 'exhausted' || instance.currentStatus.status === 'device-lost') {
      gpuOk.value = false
      error.value = failureMessage
      return
    }

    selectedHit.value = null
    hoveredHit.value = null
    gpuOk.value = true
    if (error.value === t('gpuRecoverFailed')) error.value = ''
    showNotice(t('gpuRecovered'))
  }
}

watch(code, value => {
  if (applyingWorkspaceReplacement) return
  editorDiagnostic.value = null
  // Persistence revisions are local to one WorkspaceDocument and can move
  // backwards when another document is restored. Geometry jobs instead use a
  // process-local monotonic generation, so old results can never alias a new
  // document that happens to have the same revision number.
  buildGeneration++
  performanceHistory.edited(buildGeneration, performance.now())
  // Provenance belongs to the last compiled source revision. Never retain a
  // reverse highlight while the editor has moved ahead of that revision.
  if (renderedSource.value !== value) renderer?.setSourceHighlight(null)
  // A #code= hash left by shareSource() goes stale when code diverges. An
  // imported hash that is still the only durable copy is retained until the
  // replacement draft reaches IDB or the synchronous recovery journal.
  if (!sharedImportPending && location.hash.startsWith('#code=')) {
    history.replaceState(null, '', location.pathname + location.search)
  }
  if (restoringWorkspace) {
    if (autoRender.value) scheduleRender(0)
    return
  }
  try {
    workspaceDocument.value = updateWorkspaceDocument(workspaceDocument.value, { source: value })
    workspaceSourceValid = true
  } catch {
    workspaceSourceValid = false
    workspaceEditGeneration++
    workspacePersistenceStatus.value = 'error'
    updateBeforeUnloadGuard()
    reportStorageFailure()
    return
  }
  workspaceEditGeneration++
  scheduleWorkspacePersistence()
  if (autoRender.value) scheduleRender()
})

watch(fileName, value => {
  if (applyingWorkspaceReplacement) return
  if (restoringWorkspace) return
  try {
    workspaceDocument.value = updateWorkspaceDocument(workspaceDocument.value, { fileName: value })
    workspaceFileNameValid = true
  } catch {
    workspaceFileNameValid = false
    workspaceEditGeneration++
    workspacePersistenceStatus.value = 'error'
    updateBeforeUnloadGuard()
    reportStorageFailure()
    return
  }
  workspaceEditGeneration++
  scheduleWorkspacePersistence()
})

watch(autoRender, enabled => {
  storageSet('scad-auto', String(enabled))
  if (enabled) scheduleRender(0)
  else autoBuildScheduler.cancel()
})

function scheduleRender(delay?: number) {
  autoBuildScheduler.edited(delay === 0)
}

function beginParameterGesture(event: PointerEvent) {
  if (autoRender.value && event.target instanceof HTMLInputElement && event.target.type === 'range') {
    autoBuildScheduler.beginGesture()
  }
}

async function endParameterGesture() {
  // Vue source watchers must advance the revision before submitting the final value.
  await nextTick()
  if (componentActive && autoRender.value) autoBuildScheduler.endGesture()
}

async function commitParameterControl(event: Event) {
  if (!(event.target instanceof HTMLInputElement) || event.target.type !== 'range') return
  await nextTick()
  if (componentActive && autoRender.value) autoBuildScheduler.flush()
}

function scheduleWorkspacePersistence() {
  if (storageDebounce) clearTimeout(storageDebounce)
  workspacePersistenceStatus.value = 'saving'
  updateBeforeUnloadGuard()
  storageDebounce = setTimeout(() => { void persistWorkspaceNow() }, 300)
}

async function persistWorkspaceNow(retryIndexedDb = false): Promise<boolean> {
  storageDebounce = null
  if (!workspaceSourceValid || !workspaceFileNameValid) {
    workspacePersistenceStatus.value = 'error'
    updateBeforeUnloadGuard()
    return false
  }
  const generation = workspaceEditGeneration
  const snapshot = workspaceDocument.value
  workspacePersistenceStatus.value = 'saving'
  updateBeforeUnloadGuard()
  let saved = false
  try {
    if (retryIndexedDb) {
      const result = await props.workspacePersistence.retry(snapshot)
      saved = result.saved
      if (result.restoredDocument) {
        const editorUnchanged = generation === workspaceEditGeneration
          && workspaceDocumentsEqual(workspaceDocument.value, snapshot)
          && code.value === snapshot.source
          && fileName.value === snapshot.fileName
        if (editorUnchanged) await restoreWorkspaceDocument(result.restoredDocument)
        else saved = false
      }
    } else {
      saved = await props.workspacePersistence.save(snapshot)
    }
  } catch {
    saved = false
  }
  workspaceConflict.value = props.workspacePersistence.hasConflict
  if (!componentActive) return saved
  if (saved) {
    durableWorkspaceGeneration = Math.max(durableWorkspaceGeneration, generation)
    if (sharedImportPending && location.hash.startsWith('#code=')) {
      if (props.workspacePersistence.backend === 'indexeddb') {
        history.replaceState(null, '', location.pathname + location.search)
        sharedImportPending = false
      } else {
        replaceSharedRecoveryHash(code.value)
      }
    }
    workspacePersistenceStatus.value = durableWorkspaceGeneration >= workspaceEditGeneration
      ? 'saved'
      : 'saving'
  } else {
    workspacePersistenceStatus.value = 'error'
    reportStorageFailure()
  }
  updateBeforeUnloadGuard()
  return saved
}

function flushWorkspacePersistence() {
  if (storageDebounce) clearTimeout(storageDebounce)
  if (!workspaceSourceValid || !workspaceFileNameValid) return
  if (sharedImportPending && location.hash.startsWith('#code=')) replaceSharedRecoveryHash(code.value)
  // localStorage is synchronous and therefore remains reliable during
  // pagehide/freeze where the browser may terminate an unfinished IDB task.
  if (!props.workspacePersistence.stageRecovery(workspaceDocument.value)) reportStorageFailure()
  void persistWorkspaceNow()
}

function handleVisibilityChange() {
  if (document.visibilityState === 'hidden' && storageDebounce) flushWorkspacePersistence()
}

function retryWorkspacePersistence() {
  if (storageDebounce) {
    clearTimeout(storageDebounce)
    storageDebounce = null
  }
  void persistWorkspaceNow(true)
}

const savingBrowser = ref(false)
const storageFailureDetail = ref('')
async function saveBrowserDraft() {
  if(savingBrowser.value) return
  savingBrowser.value = true
  if(storageDebounce) { clearTimeout(storageDebounce); storageDebounce = null }
  try {
    if(!workspaceSourceValid || !workspaceFileNameValid) { showNotice(t('failedBrowser')); return }
    await resolveWorkspaceConflict(true)
    showNotice(t(workspacePersistenceStatus.value === 'saved' && props.workspacePersistence.backend === 'indexeddb' ? 'savedBrowser' : 'failedBrowser'))
  } finally { savingBrowser.value = false }
}

async function resolveWorkspaceConflict(preferCurrentDraft: boolean) {
  const generation = workspaceEditGeneration
  const snapshot = workspaceDocument.value
  workspacePersistenceStatus.value = 'saving'
  let result: WorkspaceRetryResult
  try {
    result = preferCurrentDraft
      ? await props.workspacePersistence.saveCurrentDraft(snapshot)
      : await props.workspacePersistence.resolveConflict(false, snapshot)
  } catch {
    result = { saved: false }
  }
  workspaceConflict.value = props.workspacePersistence.hasConflict
  const editorUnchanged = generation === workspaceEditGeneration
    && workspaceDocumentsEqual(workspaceDocument.value, snapshot)
    && code.value === snapshot.source
    && fileName.value === snapshot.fileName
  if (result.restoredDocument && editorUnchanged) {
    await restoreWorkspaceDocument(result.restoredDocument)
  } else if (!editorUnchanged) {
    scheduleWorkspacePersistence()
    return
  }
  if (result.saved) {
    durableWorkspaceGeneration = workspaceEditGeneration
    workspacePersistenceStatus.value = 'saved'
    if (sharedImportPending && props.workspacePersistence.backend === 'indexeddb') {
      history.replaceState(null, '', location.pathname + location.search)
      sharedImportPending = false
    }
  } else {
    workspacePersistenceStatus.value = 'error'
    reportStorageFailure()
  }
  updateBeforeUnloadGuard()
}

function reportStorageFailure() {
  storageFailureDetail.value = props.workspacePersistence.failureReason || (lang.value === 'ru' ? 'Не удалось записать черновик. Нажмите «Сохранить в браузере» для повторной записи текущего кода.' : 'Could not write draft. Use Save in browser to retry the current code.')
  const now = Date.now()
  if (now - lastStorageNotice < STORAGE_NOTICE_THROTTLE_MS) return
  lastStorageNotice = now
  showNotice(t('storageFailed'))
}

function handleBeforeUnload(event: BeforeUnloadEvent) {
  if (durableWorkspaceGeneration >= workspaceEditGeneration
    && workspacePersistenceStatus.value !== 'error') return
  event.preventDefault()
  event.returnValue = ''
}

function updateBeforeUnloadGuard() {
  const needed = durableWorkspaceGeneration < workspaceEditGeneration
    || workspacePersistenceStatus.value === 'error'
  if (needed && !beforeUnloadAttached) {
    window.addEventListener('beforeunload', handleBeforeUnload)
    beforeUnloadAttached = true
  } else if (!needed && beforeUnloadAttached) {
    detachBeforeUnloadGuard()
  }
}

function detachBeforeUnloadGuard() {
  if (!beforeUnloadAttached) return
  window.removeEventListener('beforeunload', handleBeforeUnload)
  beforeUnloadAttached = false
}

async function restoreWorkspaceDocument(document: WorkspaceDocumentSnapshot) {
  restoringWorkspace = true
  workspaceSourceValid = true
  workspaceFileNameValid = true
  workspaceDocument.value = document
  code.value = document.source
  fileName.value = document.fileName
  exampleGalleryOpen.value = false
  renderer?.setSourceHighlight(null)
  await nextTick()
  restoringWorkspace = false
  props.workspacePersistence.stageRecovery(document)
  workspaceEditGeneration++
  durableWorkspaceGeneration = workspaceEditGeneration
  fitNextRender = true
}

function startBuildCoordinator() {
  if (buildCoordinator) return
  buildCoordinator = new BuildCoordinator({
    workerFactory: () => new Worker(new URL('./workers/geometry.worker.ts', import.meta.url), { type: 'module' }),
    // The parser cancels cooperatively at its yield points (every ~50ms), so a
    // superseded build normally reports `cancelled` well inside this grace
    // window and the warm worker — with its cached ~541 kB WASM — survives.
    // The grace timer stays as the hard boundary for statements that never
    // reach a yield point.
    supersedeGraceMs: 300,
    onPublish: handleGeometryResponse,
    onProgress: armWatchdog,
    onStateChange: handleBuildState,
  })
}

function doRender(quality: GeometryQuality = 'full') {
  if (!renderer) return
  autoBuildScheduler.cancelPending()
  try {
    startBuildCoordinator()
    const documentRevision = buildGeneration
    const source = code.value
    buildSources.clear()
    buildSources.set(documentRevision, source)
    buildCoordinator!.requestBuild({ documentRevision, source, quality })
  } catch (caught) {
    handleWorkerError(caught)
    return
  }
  error.value = ''
  editorDiagnostic.value = null
  warnings.value = []
}

function handleBuildState(state: BuildCoordinatorState) {
  if (buildCoordinator) {
    const current = buildCoordinator.diagnostics
    buildCounters.value = {
      builds: previousBuildCounters.builds + current.builds,
      superseded: previousBuildCounters.superseded + current.superseded,
      workerStarts: previousBuildCounters.workerStarts + current.workerStarts,
      hardRestarts: previousBuildCounters.hardRestarts + current.hardRestarts,
    }
  }
  rendering.value = state.status === 'building'
  autoBuildScheduler.setBuilding(rendering.value)
  if (state.requestedQuality) renderingQuality.value = state.requestedQuality
  if (!rendering.value && rendererErrorMessage && !error.value) error.value = rendererErrorMessage
  // The watchdog observes coordinator liveness: every state change (including
  // the worker's throttled mid-parse progress heartbeats) re-arms it, so it
  // only fires on genuine silence — a build wedged inside one statement.
  if (rendering.value) armWatchdog()
  else disarmWatchdog()
}

function armWatchdog() {
  if (watchdogTimer) clearTimeout(watchdogTimer)
  const token = ++watchdogToken
  watchdogTimer = setTimeout(() => handleWatchdogTimeout(token), WATCHDOG_TIMEOUT_MS)
}

function disarmWatchdog() {
  watchdogToken++
  if (watchdogTimer) { clearTimeout(watchdogTimer); watchdogTimer = null }
}

/**
 * Last-resort recovery. Cooperative cancellation is per-top-level-statement,
 * so a single wedged statement can hang the worker forever without ever
 * being superseded. If the coordinator reports no liveness within the
 * watchdog window, replace the worker through the coordinator's hard
 * cancellation boundary (accepting the cold-WASM reload) and retry the
 * current source once. A second consecutive timeout gives up with an error
 * instead of restarting in a loop.
 */
function handleWatchdogTimeout(token: number) {
  if (token !== watchdogToken) return
  watchdogToken++
  watchdogTimer = null
  if (!rendering.value || !buildCoordinator) return
  const quality = renderingQuality.value
  buildCoordinator.cancel('worker-restart')
  if (watchdogRetried) {
    watchdogRetried = false
    error.value = t('workerError')
    return
  }
  watchdogRetried = true
  showNotice(t('workerRestarted'))
  doRender(quality)
}

function handleGeometryResponse(response: PublishedGeometryBuild) {
  if (response.status === 'succeeded') autoBuildScheduler.observe(response.quality, response.durationMs)
  // The editor revision advances before its debounced preview is submitted.
  // Never publish an older build during that window, even if the coordinator
  // has not seen the replacement job yet.
  if (response.documentRevision !== buildGeneration) return
  watchdogRetried = false
  renderDuration.value = response.durationMs
  const source = buildSources.get(response.documentRevision) ?? code.value
  for (const revision of buildSources.keys()) {
    if (revision < response.documentRevision) buildSources.delete(revision)
  }

  if (response.status === 'failed') {
    error.value = response.error.message
    editorDiagnostic.value = diagnosticFromBuildError(source, response.error)
    return
  }

  const publicationPlan = planGeometryPublication({
    buildGeneration: response.documentRevision,
    source,
    quality: response.quality,
    reduced: response.reduced,
  }, { buildGeneration, source: code.value })
  if (!publicationPlan.publish) return
  const effectiveQuality = publicationPlan.effectiveQuality

  try {
    const publicationStartedAt = performance.now()
    const displayMeshes = response.meshes.map(withSelectionSurfaces)
    const sameSourceSnapshot = renderedSource.value !== '' && renderedSource.value === source
    const previousFaceHit = renderer?.currentHit
    const previousMeshes = sceneMeshes.value
    const previousVisibility = meshVisibility.value
    const previousMeasurement = measurement.value
    const previousMeasureActive = measureActive.value
    const publication = planScenePublication({
      previousMeshes,
      previousVisibility,
      previousSelectedIndex: selectedMesh.value,
      previousIsolated: isolated.value,
      nextMeshes: displayMeshes,
      sameSourceSnapshot,
    })

    const frameToken = nextPerformanceFrameToken++
    renderer?.setMeshes(displayMeshes, {
      frameToken,
      animate: renderedSource.value !== '' && !sameSourceSnapshot,
      preserveMeasurement: publication.measurementMayBePreserved,
    })
    sceneController.publish({
      meshes: displayMeshes,
      visibility: publication.nextVisibility,
      selectedIndex: publication.nextSelectedIndex,
      isolated: publication.nextIsolated,
    })
    renderer?.setMeshVisibilityBatch(meshVisibility.value)
    hoveredHit.value = null
    if (publication.nextSelectedIndex !== null) {
      const faceRestored = previousFaceHit && previousMeshes[previousFaceHit.meshIndex]
        && renderer?.restoreNativeFaceSelection(previousMeshes[previousFaceHit.meshIndex], previousFaceHit, publication.nextSelectedIndex)
      if (!faceRestored) renderer?.selectMesh(publication.nextSelectedIndex)
      if (publication.nextIsolated) renderer?.toggleIsolateSelection()
    } else {
      sceneController.applyRendererSelection(null, false, null)
    }
    if (publication.measurementMayBePreserved) {
      // The renderer may rebuild this overlay; retaining the UI value avoids a
      // preview → full flash and lets it restore the exact world-space points.
      measurement.value = previousMeasurement
      measureActive.value = previousMeasureActive
    } else {
      measurement.value = null
      measureActive.value = false
    }
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
    renderedQuality.value = effectiveQuality
    syncSourceHighlightFromEditor()
    syncSectionRange()
    const publishedAt = performance.now()
    const historyToken = performanceHistory.record({
      revision: response.documentRevision,
      quality: response.quality,
      phases: response.timings,
      workerMs: response.durationMs,
      hostMs: response.hostElapsedMs ?? null,
      publicationMs: publishedAt - publicationStartedAt,
      transferBytes: meshTransferables(response.meshes).reduce((sum, buffer) => sum + buffer.byteLength, 0),
      upload: renderer?.currentStatus.status === 'ready' ? renderer.sceneUploadMetrics : null,
    }, publishedAt)
    pendingPerformanceFrame = { frameToken, historyToken }
    performanceSamples.value = performanceHistory.snapshot()
    // Auto builds are sequential: request full only after preview proves it
    // used a reduced quality decision. Equivalent models therefore compile
    // exactly once and reduced models retain the warm Worker for their full
    // continuation.
    if (publicationPlan.requestFull && autoRender.value) autoBuildScheduler.promote()
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught)
  }
}

function handleWorkerError(caught?: unknown) {
  rendering.value = false
  disarmWatchdog()
  buildCoordinator?.dispose()
  previousBuildCounters = { ...buildCounters.value }
  buildCoordinator = null
  error.value = caught instanceof Error ? `${t('workerError')}: ${caught.message}` : t('workerError')
}

function toggleLang() {
  lang.value = lang.value === 'ru' ? 'en' : 'ru'
  storageSet('scad-lang', lang.value)
  document.documentElement.lang = lang.value
}

function toggleTheme() {
  themeSelection.value = isDark.value ? 'light' : 'dark'
  applyPreferences()
}

function handleSystemThemeChange(event: MediaQueryListEvent) {
  systemPrefersDark.value = event.matches
  if (themeSelection.value === 'system') applyPreferences()
}

function applyPreferences() {
  const theme = resolveTheme(themeSelection.value, systemPrefersDark.value)
  document.documentElement.dataset.theme = theme.scheme
  document.documentElement.dataset.themePreset = themeSelection.value
  document.documentElement.lang = lang.value
  document.documentElement.style.colorScheme = theme.scheme
  for (const [token, value] of Object.entries(theme.tokens)) {
    document.documentElement.style.setProperty(token, value)
  }
  renderer?.setBackgroundColor(themeCanvasColor(theme))
  storageSet('scad-theme-v1', themeSelection.value)
  storageSet('scad-theme', theme.scheme)
}

async function loadEditorDocument(example: string, nextFileName: string) {
  applyingWorkspaceReplacement = true
  buildGeneration++
  editorDiagnostic.value = null
  renderer?.setSourceHighlight(null)
  if (!sharedImportPending && location.hash.startsWith('#code=')) {
    history.replaceState(null, '', location.pathname + location.search)
  }
  workspaceDocument.value = updateWorkspaceDocument(workspaceDocument.value, {
    source: example,
    fileName: nextFileName,
  })
  workspaceSourceValid = true
  workspaceFileNameValid = true
  const editor = editorRef.value
  if (editor) {
    editor.setRangeText(example, 0, editor.value.length, 'select')
    code.value = editor.value
  } else {
    code.value = example
  }
  fileName.value = nextFileName
  workspaceEditGeneration++
  scheduleWorkspacePersistence()
  if (autoRender.value) scheduleRender()
  fitNextRender = true
  await nextTick()
  applyingWorkspaceReplacement = false
  editor?.focus({ preventScroll: true })
}

async function loadExample(id: string) {
  const example = EXAMPLES[id]
  if (example) await loadEditorDocument(example, `${id}.scad`)
}

async function loadMechanicalModel(source: string, name: string) {
  await loadEditorDocument(source, name)
  await doRender('full')
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
  if (file.size > MAX_WORKSPACE_SOURCE_LENGTH) {
    error.value = t('fileTooLarge')
    return
  }
  const source = await file.text()
  if (source.length > MAX_WORKSPACE_SOURCE_LENGTH) {
    error.value = t('fileTooLarge')
    return
  }
  code.value = source
  const requestedName = file.name.endsWith('.scad') ? file.name : `${file.name}.scad`
  fileName.value = requestedName.slice(0, MAX_WORKSPACE_FILE_NAME_LENGTH)
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

const additionalExportFormat = ref<MeshExportFormat>('3mf')
async function exportAdditionalMesh() {
  if (!canExport.value) return
  try {
    const artifact = await exportMeshFormatCompressed(flattenExportMeshes(sceneMeshes.value), additionalExportFormat.value)
    const buffer = new ArrayBuffer(artifact.data.byteLength)
    new Uint8Array(buffer).set(artifact.data)
    downloadBlob(new Blob([buffer], { type: artifact.mimeType }), sanitizeFileName(fileName.value).replace(/\.scad$/i, '.' + artifact.extension))
    showNotice(t('exported'))
  } catch (error) { showNotice(error instanceof Error ? error.message : 'Export failed') }
}

function exportStl() {
  if (!canExport.value) return
  const bytes = buildBinaryStl(sceneMeshes.value, fileName.value)
  const buffer = new ArrayBuffer(bytes.byteLength)
  new Uint8Array(buffer).set(bytes)
  downloadBlob(new Blob([buffer], { type: 'model/stl' }), sanitizeFileName(fileName.value).replace(/\.scad$/i, '.stl'))
  showNotice(t('exported'))
}

function exportPerformance() {
  downloadBlob(new Blob([JSON.stringify({
    schemaVersion: 1,
    recordedAt: new Date().toISOString(),
    frameBoundary: 'GPU queue submission, not display presentation or GPU completion',
    counters: buildCounters.value,
    samples: performanceSamples.value,
  }, null, 2)], { type: 'application/json' }), 'build-performance.json')
}

function exportObj() {
  if (!canExport.value) return
  downloadBlob(new Blob([buildObj(sceneMeshes.value)], { type: 'text/plain;charset=utf-8' }), sanitizeFileName(fileName.value).replace(/\.scad$/i, '.obj'))
  showNotice(t('exported'))
}

function savePresetList(presets: readonly ParameterPreset[]): boolean {
  try {
    workspaceDocument.value = updateWorkspaceDocument(workspaceDocument.value, { parameterPresets: presets })
    workspaceEditGeneration++
    scheduleWorkspacePersistence()
    return true
  } catch {
    presetError.value = lang.value === 'ru' ? 'Не удалось сохранить варианты: превышен допустимый объём.' : 'Could not save presets: storage size limit exceeded.'
    return false
  }
}

async function saveParameterPreset() {
  await nextTick()
  presetError.value = ''
  if (parameterPresets.value.some(item => item.name === presetName.value.trim())) {
    presetError.value = lang.value === 'ru' ? 'Вариант с таким именем уже есть. Выберите другое имя.' : 'This name already exists. Choose another name.'
    return
  }
  try {
    const preset = captureParameterPreset(code.value, presetName.value, crypto.randomUUID())
    if (savePresetList([...parameterPresets.value, preset])) {
      presetSelection.value = preset.id
      presetName.value = ''
    }
  } catch {
    presetError.value = lang.value === 'ru' ? 'Не удалось сохранить: нужны уникальные параметры (до 128) и имя до 80 символов.' : 'Could not save: requires unique parameters (up to 128) and a name of up to 80 characters.'
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
    presetError.value = lang.value === 'ru' ? 'Вариант не подходит к текущему исходнику.' : 'This preset does not match the current source.'
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
  if (savePresetList(parameterPresets.value.filter(item => item.id !== selectedPreset.value!.id))) presetSelection.value = ''
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

async function shareSource() {
  const url = new URL(window.location.href)
  url.hash = `code=${encodeWorkspaceShare(code.value)}`
  history.replaceState(null, '', url)
  try {
    await navigator.clipboard.writeText(url.toString())
    showNotice(t('copied'))
  } catch {
    showNotice(t('copyFailed'))
  }
}

function replaceSharedRecoveryHash(source: string) {
  const url = new URL(window.location.href)
  url.hash = `code=${encodeWorkspaceShare(source)}`
  history.replaceState(null, '', url)
}

function fitView() { renderer?.fitView() }
function resetView() {
  standardView.value = 'iso'
  activeView.value = 'iso'
  restoreProjectionAfterFaceSnap()
  renderer?.resetView()
}
function previousView() {
  const state = renderer?.previousView()
  if (!state) return
  // History restores an explicit projection; a pending face-snap restore is obsolete.
  projectionBeforeFaceSnap.value = null
  projection.value = state.projection
  const restoredView = standardViewForCamera(state)
  if (restoredView) {
    standardView.value = restoredView
    activeView.value = restoredView
  } else {
    activeView.value = 'custom'
  }
}

const FACE_VIEWS: readonly StandardView[] = ['front', 'back', 'left', 'right', 'top', 'bottom']

/** Mirrors the renderer camera into the UI so the view cube stays truthful. */
function syncCameraState(state: CameraState) {
  cameraYaw.value = state.yaw
  cameraPitch.value = state.pitch
  projection.value = state.projection
  const matched = standardViewForCamera(state)
  activeView.value = matched ?? 'custom'
  if (matched) standardView.value = matched
  // Orbiting away from a snapped face view (or landing on ISO) restores the
  // projection the user had before the orthographic snap.
  if (matched === null || matched === 'iso') restoreProjectionAfterFaceSnap()
}

function restoreProjectionAfterFaceSnap() {
  const previous = projectionBeforeFaceSnap.value
  projectionBeforeFaceSnap.value = null
  if (previous !== null && projection.value !== previous) {
    projection.value = previous
    renderer?.setProjection(previous)
  }
}

function focusSelection() {
  if (!renderer?.fitSelection()) renderer?.fitView()
}
function clearSelection() { renderer?.clearSelection() }
function toggleIsolate() {
  isolated.value = renderer?.toggleIsolateSelection() ?? false
}
function toggleProjection() {
  // An explicit projection choice cancels any pending face-snap restore.
  projectionBeforeFaceSnap.value = null
  projection.value = projection.value === 'perspective' ? 'orthographic' : 'perspective'
  renderer?.setProjection(projection.value)
}
function toggleGrid() {
  gridVisible.value = !gridVisible.value
  renderer?.setGridVisible(gridVisible.value)
}
function changeStandardView() {
  const view = standardView.value
  activeView.value = view
  if (FACE_VIEWS.includes(view)) {
    // Face views snap to orthographic; remember what to restore on orbit-away/ISO.
    if (projectionBeforeFaceSnap.value === null && projection.value === 'perspective') {
      projectionBeforeFaceSnap.value = 'perspective'
    }
    if (projection.value !== 'orthographic') {
      projection.value = 'orthographic'
    }
  } else {
    const previous = projectionBeforeFaceSnap.value
    projectionBeforeFaceSnap.value = null
    if (previous !== null) projection.value = previous
  }
  renderer?.setCameraPreset(view, projection.value)
}
function setStandardView(view: StandardView) {
  standardView.value = view
  changeStandardView()
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
  const entityIndex = typeof id === 'string'
    ? sceneMeshes.value.findIndex(mesh => mesh.entityId === id)
    : -1
  const index = entityIndex >= 0 ? entityIndex : typeof id === 'number' ? id : Number(id)
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
  sceneController.setVisibility(index, visible)
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
  const input = editorRef.value
  if(input) {
    const start=input.selectionStart, end=input.selectionEnd
    const selected=input.value.slice(start,end)
    selectedEditorName.value = /^\$?[A-Za-z_][\w$]*$/.test(selected)
      && !/[\w$]/.test(input.value[start-1] ?? '') && !/[\w$]/.test(input.value[end] ?? '') ? selected : ''
  }
  renderer?.setSourceHighlight(sourceIdAtEditorCaret())
}

function highlightSource(sourceId: number | null) {
  if (sourceId === null) syncSourceHighlightFromEditor()
  else renderer?.setSourceHighlight(sourceMatchesEditor.value ? sourceId : null)
}

function handleEditorInput() {
  editorDiagnostic.value = null
  // The v-model update precedes this handler, so this also clears stale
  // geometry immediately instead of waiting for the next Worker response.
  syncSourceHighlightFromEditor()
}

function openEditorFind(withReplace = false) {
  const editor = editorRef.value
  if (!findOpen.value && editor && editor.selectionStart !== editor.selectionEnd) {
    findQuery.value = editor.value.slice(editor.selectionStart, editor.selectionEnd)
  }
  findOpen.value = true
  replaceOpen.value = withReplace
  activeFindIndex.value = findResult.value.matches.length ? 0 : -1
  // The command palette restores its opener after closing. A second render
  // turn makes the explicit Find target win that documented focus handoff.
  void nextTick(() => nextTick(() => {
    findInputRef.value?.focus()
    findInputRef.value?.select()
    revealFindMatch()
  }))
}

function closeEditorFind() {
  findOpen.value = false
  replaceOpen.value = false
  void nextTick(() => editorRef.value?.focus())
}

function revealFindMatch() {
  const match = findResult.value.matches[activeFindIndex.value]
  if (!match || !editorRef.value) return
  editorRef.value.setSelectionRange(match.start, match.end)
}

function navigateFind(direction: 1 | -1) {
  activeFindIndex.value = wrappedMatchIndex(
    activeFindIndex.value,
    findResult.value.matches.length,
    direction,
  )
  revealFindMatch()
}

function handleFindKeydown(event: KeyboardEvent) {
  if (event.isComposing) return
  if (event.key === 'Escape') {
    event.preventDefault()
    closeEditorFind()
  } else if (event.key === 'Enter') {
    event.preventDefault()
    navigateFind(event.shiftKey ? -1 : 1)
  }
}

function replaceCurrentFindMatch() {
  const editor = editorRef.value
  const match = findResult.value.matches[activeFindIndex.value]
  if (!editor || !match) return
  if (replaceExpectedMatch(code.value, match, findReplacement.value) === null) {
    activeFindIndex.value = findResult.value.matches.length ? 0 : -1
    revealFindMatch()
    return
  }
  editor.setRangeText(findReplacement.value, match.start, match.end, 'select')
  code.value = editor.value
  handleEditorInput()
  activeFindIndex.value = findResult.value.matches.length
    ? Math.min(activeFindIndex.value, findResult.value.matches.length - 1)
    : -1
  revealFindMatch()
}

function replaceAllFindMatches() {
  const editor = editorRef.value
  if (!editor) return
  const result = replaceAllLiteral(code.value, findQuery.value, findReplacement.value, {
    caseSensitive: findCaseSensitive.value,
  })
  if (result.status === 'too-many' || result.status === 'too-large') {
    showNotice(t('tooManyMatches'))
    return
  }
  if (result.status !== 'replaced') return
  editor.setRangeText(result.source, 0, editor.value.length, 'preserve')
  code.value = editor.value
  handleEditorInput()
  activeFindIndex.value = findResult.value.matches.length ? 0 : -1
  revealFindMatch()
}

function revealCurrentDiagnostic() {
  if (editorRef.value && editorDiagnostic.value) revealDiagnostic(editorRef.value, editorDiagnostic.value)
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

function syncSectionRange() {
  const { min, max } = sectionRange.value
  if (sectionOffset.value < min || sectionOffset.value > max) {
    sectionOffset.value = (min + max) / 2
  }
  applySection()
}

function setSectionEnabled(enabled: boolean) {
  if (enabled && !sectionAvailable.value) return
  if (enabled && !sectionInitialized) {
    sectionOffset.value = (sectionRange.value.min + sectionRange.value.max) / 2
    sectionInitialized = true
  }
  sectionEnabled.value = enabled
  applySection()
}

function setSectionAxis(axis: SectionAxis) {
  sectionAxis.value = axis
  sectionOffset.value = (sectionRange.value.min + sectionRange.value.max) / 2
  sectionInitialized = true
  applySection()
}

function setSectionOffset(offset: number) {
  if (!Number.isFinite(offset)) return
  sectionOffset.value = clamp(offset, sectionRange.value.min, sectionRange.value.max)
  sectionInitialized = true
  applySection()
}

function setSectionFlip(flip: boolean) {
  sectionFlip.value = flip
  applySection()
}

function resetSection() {
  sectionOffset.value = (sectionRange.value.min + sectionRange.value.max) / 2
  sectionFlip.value = false
  sectionInitialized = true
  applySection()
}

function closeScanPanel() {
  scanPanelOpen.value = false
  void nextTick(() => scanToggleRef.value?.focus())
}

function lineAndColumn(source: string, offset: number) {
  const before = source.slice(0, clamp(offset, 0, source.length))
  const line = before.split('\n').length
  return { line, column: before.length - before.lastIndexOf('\n') }
}

function executeCommand(id: string) {
  const target = paletteCommands.value.find(candidate => candidate.id === id)
  if (target && !isPaletteCommandEnabled(target)) return
  const paletteWasOpen = paletteOpen.value
  paletteOpen.value = false
  if (target) recordCommandUsage(id)
  const views: StandardView[] = ['iso', 'front', 'back', 'left', 'right', 'top', 'bottom']
  if (views.includes(id as StandardView)) { setStandardView(id as StandardView); return }
  switch (id) {
    case 'render': doRender('full'); break
    case 'open': triggerOpen(); break
    case 'save': saveSource(); break
    case 'find': openEditorFind(false); break
    case 'replace': openEditorFind(true); break
    case 'shortcut-help': shortcutHelpOpen.value = true; break
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
      scanPanelOpen.value = true
      setSectionEnabled(!sectionEnabled.value)
      break
    case 'sidebar': dockOpen.value = !dockOpen.value; break
    case 'theme': toggleTheme(); break
    case 'command-palette': paletteOpen.value = !paletteWasOpen; break
    case 'cancel-measure': cancelMeasure(); break
    case 'flip-section': setSectionFlip(!sectionFlip.value); break
    case 'hide-selected':
      if (selectedMesh.value !== null) {
        setSceneMeshVisibility(selectedMesh.value, meshVisibility.value[selectedMesh.value] === false)
      }
      break
    case 'cycle-selection-mode': {
      const modes: SelectionMode[] = ['point', 'face', 'object']
      setSelectionMode(modes[(modes.indexOf(selectionMode.value) + 1) % modes.length])
      break
    }
  }
}

function recordCommandUsage(id: string) {
  const next = [id, ...commandMru.value.filter(candidate => candidate !== id)].slice(0, 12)
  commandMru.value = next
  storageSetJSON('scad-command-mru', next)
}

function formatEditor() {
  const formatted = formatCode(code.value)
  if(formatted === code.value) return
  const editor = editorRef.value
  const caret = editor?.selectionStart ?? 0
  if(editor) editor.setRangeText(formatted,0,editor.value.length,'preserve')
  code.value = formatted
  foldedLines.value = new Set()
  void nextTick(()=>{
    if(editor) { editor.focus(); editor.setSelectionRange(Math.min(caret,formatted.length),Math.min(caret,formatted.length)) }
    syncHighlightScroll()
  })
  handleEditorInput()
}

function handleEditorKey(event: KeyboardEvent) {
  if(event.altKey && event.shiftKey && event.code === 'KeyF') { event.preventDefault(); formatEditor(); return }
  if(event.key === 'Tab' && !event.ctrlKey && !event.metaKey && !event.altKey && !event.isComposing) {
    event.preventDefault()
    const editor=editorRef.value; if(!editor) return
    const edit=indentSelection(code.value,editor.selectionStart,editor.selectionEnd,event.shiftKey)
    editor.setRangeText(edit.text,edit.start,edit.end,'preserve')
    code.value=editor.value; editor.setSelectionRange(edit.selectionStart,edit.selectionEnd)
    handleEditorInput(); return
  }
  dispatchKeyboardCommand(event, 'editor')
}

function handleViewportKey(event: KeyboardEvent) {
  dispatchKeyboardCommand(event, 'viewport')
}

function handleGlobalKey(event: KeyboardEvent) {
  dispatchKeyboardCommand(event, 'global')
}

function dispatchKeyboardCommand(event: KeyboardEvent, scope: CommandScope) {
  const id = resolveKeyboardCommand(event, scope, { isEnabled: isCommandEnabled })
  if (!id) return
  if (shortcutHelpOpen.value || exampleGalleryOpen.value || mechanicalGeneratorOpen.value) return
  if (paletteOpen.value && id !== 'command-palette') return
  event.preventDefault()
  executeCommand(id)
}

function isCommandEnabled(id: CommandId): boolean {
  if (id === 'cancel-measure') return measureActive.value
  if (id === 'flip-section') return sectionEnabled.value
  if (id === 'hide-selected') return selectedMesh.value !== null
  const target = paletteCommands.value.find(candidate => candidate.id === id)
  return target ? isPaletteCommandEnabled(target) : true
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
  storageSet('scad-editor-width', String(Math.round(editorWidth.value)))
}
function resizeEditor(clientX: number) {
  const rect = mainRef.value?.getBoundingClientRect()
  if (!rect) return
  editorMaxWidth.value = editorWidthBounds(rect.width).max
  editorWidth.value = clampEditorWidth(clientX - rect.left, rect.width)
}
function resizeEditorWithKeyboard(event: KeyboardEvent) {
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
  event.preventDefault()
  const width = mainRef.value?.getBoundingClientRect().width ?? 0
  const bounds = editorWidthBounds(width)
  editorMaxWidth.value = bounds.max
  if (event.key === 'Home') editorWidth.value = bounds.min
  else if (event.key === 'End') editorWidth.value = bounds.max
  else editorWidth.value = clampEditorWidth(editorWidth.value + (event.key === 'ArrowRight' ? 20 : -20), width)
  storageSet('scad-editor-width', String(Math.round(editorWidth.value)))
}

function showNotice(message: string) {
  notice.value = message
  if (noticeTimeout) clearTimeout(noticeTimeout)
  noticeTimeout = setTimeout(() => { notice.value = '' }, 2200)
}

function readCommandMru(): string[] {
  const value = storageGetJSON<unknown[]>('scad-command-mru', [], Array.isArray)
  return [...new Set(value.filter((item): item is string => typeof item === 'string' && item.length > 0))].slice(0, 12)
}
function clamp(value: number, min: number, max: number) { return Math.max(min, Math.min(max, value)) }
function sanitizeFileName(name: string) { return (name.replace(/[^\w.() -]+/g, '_') || 'model.scad').replace(/\.scad.*$/i, '.scad') }

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
        <span class="kernel-badge" :title="currentBackendQuality.transparency === 'object-sorted-alpha' ? t('transparencySorted') : undefined">
          {{ currentBackendQuality.backend === 'webgpu-interactive' ? t('backendWebGpu') : t('backendHeadless') }}
        </span>
      </div>
      <div class="topbar-right">
        <div v-if="workspaceConflict" class="persistence-conflict" role="alert">
          <span>⚠ {{ t('storageConflict') }}</span>
          <button type="button" @click="saveSource">{{ t('exportDraft') }}</button>
          <button type="button" @click="resolveWorkspaceConflict(false)">{{ t('useIndexed') }}</button>
          <button type="button" @click="resolveWorkspaceConflict(true)">{{ t('restoreDraft') }}</button>
        </div>
        <button
          v-else-if="workspacePersistenceStatus === 'error'"
          class="persistence-status persistence-error"
          type="button"
          :title="t('retrySave')"
          @click="retryWorkspacePersistence"
        >⚠ {{ t('unsavedDraft') }}</button>
        <span v-if="workspacePersistenceStatus === 'error' && storageFailureDetail" class="persistence-error-detail" role="status">{{ storageFailureDetail }}</span>
        <span
          v-if="!workspaceConflict && workspacePersistenceStatus === 'saving'"
          class="persistence-status"
          role="status"
        >{{ t('savingDraft') }}</span>
        <button class="icon-btn command-btn" type="button" :title="t('commandHelp')" aria-keyshortcuts="Control+K Meta+K" @click="paletteOpen = true">
          <span aria-hidden="true">⌘</span> {{ t('commands') }} <kbd>Ctrl K</kbd>
        </button>
        <button
          class="icon-btn"
          type="button"
          :title="t('shortcuts')"
          aria-keyshortcuts="?"
          aria-haspopup="dialog"
          :aria-expanded="shortcutHelpOpen"
          @click="shortcutHelpOpen = true"
        >?</button>
        <button class="icon-btn lang-btn" type="button" :aria-label="t('language')" @click="toggleLang">
          {{ lang === 'ru' ? 'RU' : 'EN' }}
        </button>
        <label class="theme-picker">
          <span class="sr-only">{{ t('theme') }}</span>
          <select v-model="themeSelection" :aria-label="t('theme')" @change="applyPreferences">
            <option value="system">{{ t('systemTheme') }}</option>
            <option v-for="theme in THEME_CATALOG" :key="theme.id" :value="theme.id">
              {{ theme.name[lang] }}
            </option>
          </select>
        </label>
        <button class="icon-btn" type="button" :aria-label="isDark ? t('lightTheme') : t('darkTheme')" :aria-pressed="isDark" @click="toggleTheme">
          <span aria-hidden="true">{{ isDark ? '☾' : '☀' }}</span>
        </button>
      </div>
    </nav>

    <main ref="mainRef" class="main" :inert="directModelerOpen">
      <section class="editor-panel" :style="{ width: `${editorWidth}px` }" :aria-label="t('editor')">
        <div class="toolbar editor-toolbar">
          <button class="btn btn-primary" type="button" title="Ctrl/⌘+Enter" :disabled="rendering" @click="doRender('full')">
            <span class="play" aria-hidden="true">▶</span> {{ t('render') }}
          </button>
          <label class="auto-check"><input v-model="autoRender" type="checkbox"> {{ t('auto') }}</label>
          <span class="toolbar-divider" aria-hidden="true" />
          <button class="btn" type="button" @click="directModelerOpen = true">{{ lang === 'ru' ? 'Прямое моделирование' : 'Direct modeling' }}</button>
          <button class="btn" type="button" @click="mechanicalGeneratorOpen = true">⚙ {{ lang === 'ru' ? 'Генераторы' : 'Generators' }}</button>
          <button class="btn" type="button" @click="exampleGalleryOpen = true">▦ {{ t('examples') }}</button>
        </div>

        <div class="toolbar file-toolbar">
          <button class="btn" type="button" :title="t('openFile')" @click="triggerOpen">↥ {{ t('open') }}</button>
          <button class="btn" type="button" :title="t('saveFile')" @click="saveSource">↧ {{ t('save') }}</button>
          <button class="btn" type="button" title="Alt+Shift+F" aria-keyshortcuts="Alt+Shift+F" @click="formatEditor">{{ t('format') }}</button>
          <button class="btn" type="button" :disabled="savingBrowser" :title="t('saveBrowserHelp')" @click="saveBrowserDraft">{{ savingBrowser ? t('savingDraft') : t('saveBrowser') }}</button>
          <button class="btn" type="button" :title="t('shareFile')" @click="shareSource">⌁ {{ t('share') }}</button>
          <button class="btn export-btn" type="button" :disabled="!canExport" @click="exportStl">STL</button>
          <button class="btn export-btn" type="button" :disabled="!canExport" @click="exportObj">OBJ</button>
          <select v-model="additionalExportFormat" class="btn export-btn" :aria-label="lang === 'ru' ? 'Формат экспорта' : 'Export format'">
            <option value="3mf">3MF</option><option value="ply">PLY</option><option value="off">OFF</option><option value="amf">AMF</option>
          </select>
          <button class="btn export-btn" type="button" :disabled="!canExport" @click="exportAdditionalMesh">{{ lang === 'ru' ? 'Скачать' : 'Download' }}</button>
          <span class="file-name" :title="fileName">{{ fileName }}</span>
          <input ref="fileInputRef" class="sr-only" type="file" accept=".scad,text/plain" @change="openSelectedFile">
        </div>

        <div v-if="findOpen" class="find-bar" role="search" @keydown="handleFindKeydown">
          <input
            ref="findInputRef"
            v-model="findQuery"
            type="search"
            autocomplete="off"
            maxlength="4096"
            :aria-label="t('find')"
            :placeholder="t('find')"
          >
          <input
            v-if="replaceOpen"
            v-model="findReplacement"
            type="text"
            autocomplete="off"
            :maxlength="MAX_WORKSPACE_SOURCE_LENGTH"
            :aria-label="t('replace')"
            :placeholder="t('replace')"
          >
          <span class="find-status" role="status" aria-live="polite">{{ findStatus }}</span>
          <button type="button" :aria-label="t('previousMatch')" @click="navigateFind(-1)">↑</button>
          <button type="button" :aria-label="t('nextMatch')" @click="navigateFind(1)">↓</button>
          <label class="find-case"><input v-model="findCaseSensitive" type="checkbox"> {{ t('matchCase') }}</label>
          <button v-if="replaceOpen" type="button" :disabled="activeFindIndex < 0" @click="replaceCurrentFindMatch">{{ t('replace') }}</button>
          <button v-if="replaceOpen" type="button" :disabled="!findResult.matches.length || findResult.truncated" @click="replaceAllFindMatches">{{ t('replaceAll') }}</button>
          <button type="button" :aria-label="t('closeSearch')" @click="closeEditorFind">×</button>
        </div>

        <div class="code-editor" :style="{ '--line-number-digits': Math.max(2, String(editorLineCount).length) }">
        <div v-if="foldedLines.size" class="folded-editor code" aria-label="Свернутый код">
          <div v-for="row in foldedRows" :key="row.line" class="folded-row">
            <span class="folded-number">{{ row.line + 1 }}</span>
            <button v-if="row.block" class="fold-toggle" :aria-label="`${foldedLines.has(row.line) ? 'Развернуть' : 'Свернуть'} блок, строка ${row.line + 1}`" :aria-expanded="!foldedLines.has(row.line)" @click="toggleFold(row.line)">{{ foldedLines.has(row.line) ? '▸' : '▾' }}</button>
            <span v-else class="fold-spacer" />
            <span class="folded-text" @click="editFoldedLine(row.line)"><i v-for="guide in row.guides" :key="guide.start" class="folded-guide" aria-hidden="true" :style="{ left: `${guide.column}ch`, background: guide.last ? `linear-gradient(to bottom, ${blockColors[guide.depth % blockColors.length]} 75%, transparent 100%)` : blockColors[guide.depth % blockColors.length], bottom: '0' }" /><span v-html="row.html || ' '" /><span v-if="row.block && foldedLines.has(row.line)" class="fold-summary"> ⋯ {{ row.block.end - row.line }} строк</span></span>
          </div>
          <button class="unfold-all" @click="editFoldedLine(0)">Развернуть всё для редактирования</button>
        </div>
        <div v-show="!foldedLines.size" class="code-gutter"><div ref="lineNumbersRef" class="gutter-lines"><div v-for="row in editorRows" :key="row.line" class="gutter-row"><span aria-hidden="true">{{ row.line + 1 }}</span><button v-if="row.block" class="fold-toggle" :aria-label="`Свернуть блок, строка ${row.line + 1}`" aria-expanded="true" @click="toggleFold(row.line)">▾</button><span v-else class="fold-spacer" /></div></div></div>
        <div v-show="!foldedLines.size" class="code-content">
        <div class="code-guides" aria-hidden="true"><div ref="guidesRef" class="guide-lines"><div v-for="row in editorRows" :key="row.line" class="guide-row"><i v-for="guide in row.guides" :key="guide.start" :style="{ left: `${guide.column}ch`, background: guide.last ? `linear-gradient(to bottom, ${blockColors[guide.depth % blockColors.length]} 75%, transparent 100%)` : blockColors[guide.depth % blockColors.length], bottom: '0' }" /></div></div></div>
        <pre ref="highlightRef" class="code code-highlight" aria-hidden="true"><span class="highlight-content" v-html="highlightedCode" /></pre>
        <textarea
          ref="editorRef"
          v-model="code"
          class="code code-input"
          wrap="off"
          @scroll="syncHighlightScroll"
          :aria-label="t('editor')"
          spellcheck="false"
          autocomplete="off"
          autocorrect="off"
          autocapitalize="off"
          :maxlength="MAX_WORKSPACE_SOURCE_LENGTH"
          @input="handleEditorInput"
          @select="syncSourceHighlightFromEditor"
          @click="syncSourceHighlightFromEditor"
          @keyup="syncSourceHighlightFromEditor"
          @focus="syncSourceHighlightFromEditor"
          @keydown="handleEditorKey"
        />
        </div>
        </div>

        <div v-if="error" class="message error" role="alert" aria-live="assertive">
          <button v-if="editorDiagnostic" type="button" class="diagnostic-link" @click="revealCurrentDiagnostic">
            {{ error }} · {{ editorDiagnostic.line }}:{{ editorDiagnostic.column }}
          </button>
          <template v-else>{{ error }}</template>
        </div>
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
        <SvgPanel :meshes="sceneMeshes" :hit="selectedHit" :available="canExport" :locale="lang" @append="source => replacePresetSource(code + '\n\n' + source)" />
<details class="performance-panel">
    <summary>{{ lang === 'ru' ? 'Замеры сборки' : 'Build measurements' }}<span v-if="performanceLast"> · {{ performanceLast.quality }} · {{ performanceMs(performanceLast.hostMs) }}</span></summary>
    <div class="performance-content">
      <p>{{ lang === 'ru' ? 'Последние 60 успешных публикаций этой сессии. Отправка кадра не означает его показ на экране. Этапы вложены друг в друга: значения не нужно складывать.' : 'Last 60 successful publications in this session. Frame submission is not display presentation. Timings overlap; do not add them together.' }}</p>
      <p>{{ lang === 'ru' ? 'Сборки / вытеснены / запуски Worker / принудительные перезапуски' : 'Builds / superseded / Worker starts / forced restarts' }}: {{ buildCounters.builds }} / {{ buildCounters.superseded }} / {{ buildCounters.workerStarts }} / {{ buildCounters.hardRestarts }}</p>
      <table v-if="performanceLast">
        <caption>{{ lang === 'ru' ? 'Последняя публикация' : 'Latest publication' }} · {{ performanceLast.quality }} · #{{ performanceLast.revision }}</caption>
        <tbody><tr v-for="row in performanceRows" :key="row[0]"><th scope="row">{{ row[0] }}</th><td>{{ row[1] }}</td></tr></tbody>
      </table>
      <p v-else>{{ lang === 'ru' ? 'Замеры появятся после успешной сборки.' : 'Measurements will appear after a successful build.' }}</p>
      <p>{{ lang === 'ru' ? '«—»: этап не наблюдался. Объём загрузки учитывает только вершины и индексы, без рёбер и служебных данных.' : '“—”: boundary not observed. Upload bytes cover vertices and indices only, excluding edges and auxiliary data.' }}</p>
      <button type="button" :disabled="!performanceSamples.length" @click="exportPerformance">{{ lang === 'ru' ? 'Скачать отчёт JSON' : 'Download JSON report' }}</button>
    </div>
  </details>
      </section>

      <div
        class="splitter"
        role="separator"
        tabindex="0"
        aria-orientation="vertical"
        :aria-label="t('resize')"
        :aria-valuenow="Math.round(editorWidth)"
        aria-valuemin="300"
        :aria-valuemax="editorMaxWidth"
        @pointerdown="startResize"
        @pointermove="moveResize"
        @pointerup="stopResize"
        @pointercancel="stopResize"
        @lostpointercapture="stopResize"
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
          <button
            ref="scanToggleRef" class="view-btn scan-toggle" type="button"
            :class="{ active: sectionEnabled }" :aria-label="t('section')" :title="t('section')"
            :aria-expanded="scanPanelOpen" aria-controls="scan-plane-panel"
            :disabled="!sectionAvailable && !sectionEnabled"
            @click="scanPanelOpen = !scanPanelOpen"
          ><span aria-hidden="true">◩</span> {{ t('scanPlane') }}<i v-if="sectionEnabled" class="scan-active-dot" aria-hidden="true" /></button>
          <button ref="dockToggleRef" class="view-btn icon-only" type="button" :aria-label="t('sidebar')" :aria-expanded="dockOpen" aria-controls="cad-sidebar" @click="dockOpen = !dockOpen">▥</button>
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

        <ScanPlanePanel
          v-if="scanPanelOpen" id="scan-plane-panel"
          :enabled="sectionEnabled" :axis="sectionAxis" :offset="sectionOffset"
          :min="sectionRange.min" :max="sectionRange.max" :step="sectionRange.step"
          :flip="sectionFlip" :locale="lang" :available="sectionAvailable"
          @update:enabled="setSectionEnabled" @update:axis="setSectionAxis"
          @update:offset="setSectionOffset" @update:flip="setSectionFlip"
          @reset="resetSection" @close="closeScanPanel"
        />

        <div class="selection-modes" role="group" :aria-label="t('selectionMode')">
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
          role="application"
          tabindex="0"
          :aria-label="t('viewport')"
          @keydown="handleViewportKey"
        />
        <div v-if="!gpuOk" class="no-gpu" role="alert">
          <span>{{ rendererInitializing ? t('initializingViewport') : (rendererUnavailableMessage || t('noGpu')) }}</span>
          <button v-if="!rendererInitializing" class="btn" type="button" @click="initializeViewportRenderer">{{ t('retryRenderer') }}</button>
        </div>
        <div class="view-cube-wrap" :class="{ 'with-dock': dockOpen }">
          <ViewCube :active-view="activeView" :yaw="cameraYaw" :pitch="cameraPitch" @view="setStandardView" />
        </div>
        <div v-if="dockOpen" id="cad-sidebar" class="cad-dock">
          <div class="dock-tabs" role="tablist" :aria-label="t('sidebar')" @keydown="handleDockTabKeydown">
            <button id="dock-tab-scene" type="button" role="tab" aria-controls="dock-panel" :aria-selected="dockTab === 'scene'" :tabindex="dockTab === 'scene' ? 0 : -1" :class="{ active: dockTab === 'scene' }" @click="dockTab = 'scene'">{{ t('scene') }}</button>
            <button id="dock-tab-inspect" type="button" role="tab" aria-controls="dock-panel" :aria-selected="dockTab === 'inspect'" :tabindex="dockTab === 'inspect' ? 0 : -1" :class="{ active: dockTab === 'inspect' }" @click="dockTab = 'inspect'">{{ t('inspect') }}</button>
            <button id="dock-tab-parameters" type="button" role="tab" aria-controls="dock-panel" :aria-selected="dockTab === 'parameters'" :tabindex="dockTab === 'parameters' ? 0 : -1" :class="{ active: dockTab === 'parameters' }" @click="dockTab = 'parameters'">{{ t('parameters') }}</button>
            <button class="dock-close" type="button" :aria-label="t('closeSidebar')" @click="closeDock">×</button>
          </div>
          <div id="dock-panel" class="dock-panel" role="tabpanel" :aria-labelledby="`dock-tab-${dockTab}`">
          <SceneOutliner
            v-if="dockTab === 'scene'"
            :meshes="sceneRows"
            :selected-mesh-id="selectedSceneMeshId"
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
          <div v-else-if="dockTab === 'inspect'" class="compute-inspector">
          <section class="compute-panel" :aria-label="lang === 'ru' ? 'Вычисление геометрии' : 'Geometry computation'">
            <strong>{{ lang === 'ru' ? 'Вычисление геометрии' : 'Geometry computation' }}</strong>
            <button type="button" :disabled="selectedMesh === null || !canExport || computeBusy" @click="runGeometryAnalysis">{{ computeBusy ? (lang === 'ru' ? 'Вычисление…' : 'Computing…') : (lang === 'ru' ? 'Рассчитать площадь на GPU' : 'Compute surface area on GPU') }}</button>
            <button v-if="computeBusy" type="button" @click="cancelGeometryAnalysis">{{ lang === 'ru' ? 'Отменить' : 'Cancel' }}</button>
            <p>{{ lang === 'ru' ? 'Выберите деталь актуальной полной сборки. Эксперимент: WebGPU Compute со сверкой каждого результата на CPU.' : 'Select a part from a current full build. Experimental WebGPU Compute with CPU verification of every result.' }}</p>
            <div v-if="computeResult" role="status">
              <p>{{ lang === 'ru' ? 'Площадь поверхности' : 'Surface area' }}: <strong>{{ formatNumber(computeResult.area, 4) }}</strong></p>
              <p>{{ computeResult.backend === 'webgpu-compute' ? 'WebGPU Compute ✓' : (lang === 'ru' ? 'Результат CPU (запасной путь)' : 'CPU result (fallback)') }}</p>
              <p>CPU: {{ formatNumber(computeResult.cpuMs, 1) }} ms · GPU: {{ computeResult.gpuMs === null ? '—' : formatNumber(computeResult.gpuMs, 1) + ' ms' }}</p>
              <p v-if="computeResult.fallback">{{ lang === 'ru' ? 'GPU недоступен, завершился с ошибкой или не прошёл сверку. Показан расчёт CPU.' : 'GPU unavailable, failed, or did not pass verification. Showing CPU calculation.' }}</p>
              <p>{{ lang === 'ru' ? 'GPU-время включает запуск, передачу и чтение результата. Этот режим проверяет корректность, а не обещает ускорение.' : 'GPU time includes setup, upload and readback. This mode verifies correctness; it does not promise acceleration.' }}</p>
            </div>
            <p v-if="computeError" role="alert">{{ computeError }}</p>
          </section>
          <InspectPanel
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
          </div>
          <div v-else class="customizer-card" @pointerdown.capture="beginParameterGesture" @change.capture="commitParameterControl">
            <section class="parameter-presets" :aria-label="lang === 'ru' ? 'Варианты параметров' : 'Parameter presets'">
              <strong>{{ lang === 'ru' ? 'Варианты параметров' : 'Parameter presets' }}</strong>
              <form class="preset-save" @submit.prevent="saveParameterPreset">
                <input v-model="presetName" maxlength="80" :aria-label="lang === 'ru' ? 'Имя варианта' : 'Preset name'" :placeholder="lang === 'ru' ? 'Имя варианта' : 'Preset name'">
                <button type="submit" :disabled="!presetName.trim() || !customizerParameters.length || parameterPresets.length >= MAX_PARAMETER_PRESETS">{{ lang === 'ru' ? 'Сохранить вариант' : 'Save preset' }}</button>
              </form>
              <template v-if="parameterPresets.length">
                <select :value="selectedPreset?.id" :aria-label="lang === 'ru' ? 'Сохранённый вариант' : 'Saved preset'" @change="presetSelection = ($event.target as HTMLSelectElement).value">
                  <option v-for="preset in parameterPresets" :key="preset.id" :value="preset.id">{{ preset.name }}</option>
                </select>
                <div class="preset-actions">
                  <button type="button" :disabled="!presetCompatible" @click="restoreParameterPreset">{{ lang === 'ru' ? 'Применить' : 'Apply' }}</button>
                  <button type="button" @click="removeParameterPreset">{{ lang === 'ru' ? 'Удалить вариант' : 'Delete preset' }}</button>
                  <button v-if="presetUndo && code === presetUndo.after" type="button" @click="undoParameterPreset">{{ lang === 'ru' ? 'Отменить применение' : 'Undo apply' }}</button>
                </div>
                <p v-if="!presetCompatible">{{ lang === 'ru' ? 'Исходник изменился. Вариант доступен после восстановления той же структуры кода, включая комментарии.' : 'Source changed. Restore the same code structure, including comments, to apply this preset.' }}</p>
              </template>
              <p>{{ lang === 'ru' ? 'До 20 вариантов в локальной рабочей сессии. Файл SCAD и ссылка содержат только текущий исходник.' : 'Up to 20 presets in the local workspace. SCAD files and links contain only the current source.' }}</p>
              <p v-if="presetError" role="alert">{{ presetError }}</p>
            </section>
            <CustomizerPanel
              :parameters="customizerParameters"
              :errors="compactControls.errors"
              :title="t('parameters')"
              :empty-label="t('noParameters')"
              @change="updateCustomizer"
            />
          </div>
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
      :restore-focus="!shortcutHelpOpen"
      @close="paletteOpen = false"
      @execute="executeCommand"
    />
    <DirectModeler :open="directModelerOpen" :locale="lang" :can-append="!isModelGraphText(code)" :remaining-source="MAX_WORKSPACE_SOURCE_LENGTH - code.length - 2" @close="directModelerOpen = false" @append="source => { replacePresetSource(code + '\n\n' + source); nextTick(() => doRender('full')) }" />
    <MechanicalGenerator :open="mechanicalGeneratorOpen" :locale="lang" @close="mechanicalGeneratorOpen = false" @generate="loadMechanicalModel" @download-current="saveSource" />
    <ExampleGallery
      :open="exampleGalleryOpen"
      :examples="EXAMPLE_CATALOG"
      :locale="lang"
      @close="exampleGalleryOpen = false"
      @select="loadExample"
      @download-current="saveSource"
    />
    <KeyboardShortcuts
      :open="shortcutHelpOpen"
      :groups="shortcutHelpGroups"
      :locale="lang"
      @close="shortcutHelpOpen = false"
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
.performance-panel { flex: 0 0 auto; border-top: 1px solid var(--border); color: var(--text); font-size: .72rem; }
.performance-panel summary { cursor: pointer; padding: 8px 12px; }
.performance-panel summary span, .performance-panel p { color: var(--text-dim); }
.performance-content { padding: 0 12px 10px; max-height: 280px; overflow: auto; }
.performance-panel p { line-height: 1.45; }
.performance-panel table { width: 100%; border-collapse: collapse; }
.performance-panel caption { text-align: left; font-weight: 600; margin: 8px 0; }
.performance-panel th, .performance-panel td { padding: 4px 0; border-bottom: 1px solid var(--border); }
.performance-panel th { text-align: left; font-weight: 400; }
.performance-panel td { text-align: right; white-space: nowrap; font-variant-numeric: tabular-nums; }
.performance-panel button { padding: 6px 10px; background: var(--surface-raised); border: 1px solid var(--border); border-radius: 5px; color: var(--text); cursor: pointer; }
.performance-panel button:disabled { opacity: .5; cursor: default; }

.parameter-presets { padding: 10px; border-bottom: 1px solid var(--border); display: grid; gap: 8px; font-size: .72rem; }
.parameter-presets p { margin: 0; color: var(--text-dim); line-height: 1.45; }
.parameter-presets input, .parameter-presets select { width: 100%; min-width: 0; padding: 6px; border: 1px solid var(--border); border-radius: 5px; background: var(--surface-raised); color: var(--text); }
.preset-save { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 6px; }
.preset-actions { display: flex; gap: 6px; flex-wrap: wrap; }
.parameter-presets button { padding: 6px 8px; border: 1px solid var(--border); border-radius: 5px; background: var(--surface-raised); color: var(--text); cursor: pointer; }
.parameter-presets button:disabled { opacity: .5; cursor: default; }
.compute-inspector { flex: 1; min-height: 0; overflow: auto; display: flex; flex-direction: column; }
.compute-panel { display: grid; gap: 8px; padding: 10px; border: 1px solid var(--border); font-size: .72rem; }
.compute-panel p { margin: 0; line-height: 1.45; color: var(--text-dim); }
.compute-panel button { padding: 7px; color: var(--text); background: var(--surface-raised); border: 1px solid var(--border); border-radius: 5px; cursor: pointer; }
.compute-panel button:disabled { opacity: .5; cursor: default; }
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
.persistence-status {
  border: 0; background: transparent; color: var(--text-dim); font-size: .7rem; white-space: nowrap;
}
.persistence-error { color: var(--danger); cursor: pointer; }
.persistence-error:hover { text-decoration: underline; }
.persistence-conflict { display: flex; align-items: center; gap: 5px; color: var(--danger); font-size: .68rem; }
.persistence-conflict button {
  border: 1px solid var(--border); border-radius: 5px; background: var(--surface-raised);
  color: var(--text); padding: 3px 6px; cursor: pointer;
}
.icon-btn, .btn, .view-btn {
  min-height: 30px; border: 1px solid var(--border); border-radius: 7px; background: var(--surface-raised);
  color: var(--text); cursor: pointer; transition: background .12s, border-color .12s, transform .12s;
}
.icon-btn { min-width: 34px; padding: 4px 8px; }
.command-btn { display: flex; align-items: center; gap: 6px; padding-inline: 9px; font-size: .72rem; }
.theme-picker select { max-width: 116px; min-height: 30px; padding: 4px 7px; color: inherit; background: var(--surface-raised); border: 1px solid var(--border); border-radius: 6px; font-size: .7rem; }
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
.no-gpu { position: absolute; z-index: 8; inset: 0; display: flex; align-items: center; justify-content: center; flex-direction: column; gap: 14px; color: var(--danger); background: var(--canvas-bg); font-size: 1rem; padding: 40px; text-align: center; }
.main { flex: 1; min-height: 0; display: flex; overflow: hidden; }
.editor-panel {
  min-width: 300px; max-width: calc(100vw - 320px); display: flex; flex-direction: column;
  background: var(--surface); overflow: hidden;
}
.toolbar { display: flex; align-items: center; gap: 7px; padding: 7px 9px; border-bottom: 1px solid var(--border); }
.editor-toolbar { flex-wrap: wrap; }
.file-toolbar { flex-wrap: wrap; padding-block: 5px; background: color-mix(in srgb, var(--surface-raised) 55%, var(--surface)); }
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
.find-bar {
  display: flex; flex-wrap: wrap; align-items: center; gap: 5px; padding: 5px 8px;
  border-bottom: 1px solid var(--border); background: var(--surface-raised); font-size: .68rem;
}
.find-bar > input[type="search"], .find-bar > input[type="text"] {
  min-width: 90px; flex: 1 1 110px; height: 28px; padding: 3px 7px; border: 1px solid var(--border);
  border-radius: 5px; background: var(--bg); color: var(--text);
}
.find-bar button { min-width: 28px; min-height: 28px; border: 1px solid var(--border); border-radius: 5px; background: var(--surface); cursor: pointer; }
.find-bar button:disabled { opacity: .45; cursor: default; }
.find-status { min-width: 42px; color: var(--text-dim); text-align: center; }
.find-case { display: flex; align-items: center; gap: 4px; color: var(--text-dim); white-space: nowrap; }
.code {
  flex: 1; width: 100%; min-height: 120px; resize: none; border: 0; outline: 0; padding: 14px 15px;
  background: var(--bg); color: var(--text); caret-color: var(--accent);
  font-family: "JetBrains Mono", "SFMono-Regular", Consolas, monospace; font-size: .82rem; line-height: 1.58;
  font-weight: 400; font-style: normal; font-kerning: none; font-variant-ligatures: none; letter-spacing: 0; word-spacing: 0;
  tab-size: 2; white-space: pre; overflow: auto;
}
.code-editor { position: relative; display: flex; flex: 1; min-height: 120px; overflow: hidden; background: var(--bg); }
.code-content { position: relative; isolation: isolate; flex: 1; min-width: 0; }
.code-gutter { flex: 0 0 auto; width: calc(var(--line-number-digits) * 1ch + 38px); overflow: hidden; border-right: 1px solid var(--border); color: var(--text-dim); user-select: none; font: .82rem/1.58 "JetBrains Mono", "SFMono-Regular", Consolas, monospace; }
.code-gutter pre { margin: 0; padding: 14px 10px; text-align: right; font: inherit; white-space: pre; }
.code-editor .code { position: absolute; inset: 0; height: 100%; margin: 0; box-sizing: border-box; }
.code-highlight { z-index: 0; pointer-events: none; overflow: hidden; }
.highlight-content { display: block; width: max-content; min-width: 100%; transform-origin: top left; }
.code-input { z-index: 2; background: transparent; color: transparent; -webkit-text-fill-color: transparent; }
.code-input::selection { background: color-mix(in srgb, var(--accent) 35%, transparent); }
.code-highlight :deep(.syntax-comment) { color: #84929f; }
.code-highlight :deep(.syntax-keyword) { color: #c792ea; }
.code-highlight :deep(.syntax-string) { color: #9acb88; }
.code-highlight :deep(.syntax-number) { color: #e8ac76; }
.code-highlight :deep(.syntax-function) { color: #76c7df; }
.code-highlight :deep(.syntax-property) { color: #d5c288; }
.code-highlight :deep(.syntax-operator) { color: #b7bfea; }
@media (forced-colors: active) {
  .code-highlight { display: none; }
  .code-input { color: CanvasText; -webkit-text-fill-color: CanvasText; }
}
.message { margin: 7px 9px 0; padding: 8px 10px; border-radius: 7px; font: .74rem/1.45 ui-monospace, monospace; white-space: pre-wrap; overflow-wrap: anywhere; }
.diagnostic-link { all: unset; cursor: pointer; text-decoration: underline; text-underline-offset: 2px; }
.diagnostic-link:focus-visible { outline: 2px solid currentColor; outline-offset: 2px; border-radius: 2px; }
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
.splitter::before { content: ''; position: absolute; inset: 0 -9px; }
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
.scan-toggle { display: flex; align-items: center; gap: 5px; white-space: nowrap; }
.scan-toggle.active { background: color-mix(in srgb, var(--accent) 24%, transparent); border-color: var(--accent); }
.scan-active-dot { width: 5px; height: 5px; border-radius: 50%; background: var(--accent); }
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
.dock-panel { min-height: 0; display: flex; flex: 1; flex-direction: column; }
.dock-panel > :deep(.outliner), .dock-panel > :deep(.inspect-panel) { width: 100%; min-height: 0; flex: 1; border-radius: 0 0 9px 9px; }
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
  /* No downscale: badge hit targets must stay at least 24x24 CSS px here. */
  .view-cube-wrap { top: 55px; right: 8px; }
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
  .theme-picker select { max-width: 86px; }
  .view-cube-wrap { display: none; }
  .selection-modes button > span:not(.mode-point, .mode-face, .mode-body), .selection-modes kbd { display: none; }
  .selection-hud { max-width: calc(100% - 16px); }
  .canvas-hint { white-space: normal; }
}

@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { scroll-behavior: auto !important; transition-duration: .01ms !important; animation-duration: .01ms !important; animation-iteration-count: 1 !important; }
}

@media (forced-colors: active) {
  * { forced-color-adjust: auto; }
  button, select, input, textarea { border: 1px solid ButtonText !important; }
  :focus-visible { outline: 2px solid Highlight !important; outline-offset: 2px; }
}
.performance-panel { flex: 0 0 auto; border-top: 1px solid var(--border); color: var(--text); font-size: .72rem; }
.performance-panel summary { cursor: pointer; padding: 8px 12px; }
.performance-panel summary span, .performance-panel p { color: var(--text-dim); }
.performance-content { padding: 0 12px 10px; max-height: 280px; overflow: auto; }
.performance-panel p { line-height: 1.45; }
.performance-panel table { width: 100%; border-collapse: collapse; }
.performance-panel caption { text-align: left; font-weight: 600; margin: 8px 0; }
.performance-panel th, .performance-panel td { padding: 4px 0; border-bottom: 1px solid var(--border); }
.performance-panel th { text-align: left; font-weight: 400; }
.performance-panel td { text-align: right; white-space: nowrap; font-variant-numeric: tabular-nums; }
.performance-panel button { padding: 6px 10px; background: var(--surface-raised); border: 1px solid var(--border); border-radius: 5px; color: var(--text); cursor: pointer; }
.performance-panel button:disabled { opacity: .5; cursor: default; }

.parameter-presets { padding: 10px; border-bottom: 1px solid var(--border); display: grid; gap: 8px; font-size: .72rem; }
.parameter-presets p { margin: 0; color: var(--text-dim); line-height: 1.45; }
.parameter-presets input, .parameter-presets select { width: 100%; min-width: 0; padding: 6px; border: 1px solid var(--border); border-radius: 5px; background: var(--surface-raised); color: var(--text); }
.preset-save { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 6px; }
.preset-actions { display: flex; gap: 6px; flex-wrap: wrap; }
.parameter-presets button { padding: 6px 8px; border: 1px solid var(--border); border-radius: 5px; background: var(--surface-raised); color: var(--text); cursor: pointer; }
.parameter-presets button:disabled { opacity: .5; cursor: default; }
.compute-inspector { flex: 1; min-height: 0; overflow: auto; display: flex; flex-direction: column; }
.compute-panel { display: grid; gap: 8px; padding: 10px; border: 1px solid var(--border); font-size: .72rem; }
.compute-panel p { margin: 0; line-height: 1.45; color: var(--text-dim); }
.compute-panel button { padding: 7px; color: var(--text); background: var(--surface-raised); border: 1px solid var(--border); border-radius: 5px; cursor: pointer; }
.compute-panel button:disabled { opacity: .5; cursor: default; }
</style>

<style scoped>
.gutter-lines { padding: 14px 4px 14px 8px; }
.gutter-row { display: flex; justify-content: flex-end; height: 1.58em; align-items: center; }
.fold-toggle, .fold-spacer { display: inline-block; flex: 0 0 20px; width: 20px; }
.fold-toggle { padding: 0; border: 0; color: var(--text-dim); background: transparent; cursor: pointer; font: inherit; line-height: inherit; }
.fold-toggle:hover, .fold-toggle:focus-visible { color: var(--accent); background: var(--surface-raised); }
.code-guides { position: absolute; inset: 0; overflow: hidden; pointer-events: none; z-index: 1; font: .82rem/1.58 "JetBrains Mono", "SFMono-Regular", Consolas, monospace; }
.guide-lines { padding: 14px 15px; }
.guide-row { position: relative; height: 1.58em; }
.guide-row i { position: absolute; top: 0; bottom: 0; width: 1px; opacity: .5; }
.folded-editor { z-index: 3; }
.folded-row { display: flex; min-height: 1.58em; }
.folded-number { color: var(--text-dim); width: 4ch; text-align: right; flex: 0 0 4ch; }
.folded-text { position: relative; cursor: text; }
.folded-guide { position: absolute; top: 0; bottom: 0; width: 1px; opacity: .5; pointer-events: none; }
.fold-summary { color: var(--text-dim); background: var(--surface-raised); border-radius: 4px; }
.unfold-all { position: sticky; bottom: 0; left: 0; border: 1px solid var(--border); background: var(--surface); color: var(--text); border-radius: 4px; cursor: pointer; }
</style>

<style scoped>
.persistence-error-detail { max-width: 380px; color: var(--danger); font-size: .7rem; overflow-wrap: anywhere; }
</style>

<style scoped>
.code-editor :deep(.syntax-occurrence) { background: color-mix(in srgb, var(--accent) 23%, transparent); outline: 1px solid color-mix(in srgb, var(--accent) 65%, transparent); border-radius: 2px; }
</style>
