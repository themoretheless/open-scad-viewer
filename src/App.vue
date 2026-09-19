<script setup lang="ts">
import { isModelGraphText, SOURCE_FILE_ACCEPT, SOURCE_FILE_EXTENSION, sourceFileExtension, withSourceExtension } from './services/modelGraphTextDetect'
import { editorBlocks, indentSelection, guideFitsIndent } from './services/editorBlocks'
import { formatCode } from './services/codeFormat'
import { highlightCode } from './services/codeHighlight'
import { computed, defineAsyncComponent, nextTick, onMounted, onUnmounted, ref, shallowRef, watch, watchEffect, type ComponentPublicInstance } from 'vue'
import { detectMeshImportFormat, MESH_EXPORT_FORMATS, MESH_FORMAT_LABELS, MESH_IMPORT_ACCEPT, type MeshExportFormat } from './services/meshFormats'
import CommandPalette from './components/CommandPalette.vue'
import CustomizerPanel from './components/CustomizerPanel.vue'
import ExampleGallery from './components/ExampleGallery.vue'
const MechanicalGenerator = defineAsyncComponent(() => import('./features/MechanicalGenerator.vue'))
import {restoreSourceHistory,boundedSourceHistory} from './services/mainSourceEditing'
import type { DirectDocument } from './services/directModeling'
import { emptyDirectDocument } from './services/directModeling'
import type { DirectBody } from './services/directModeling'
import type { WorkspaceMode } from './services/workspaceModes'
import { WORKSPACE_MODES, workspaceModeHint, workspaceModeLabel } from './services/workspaceModes'
import { sceneMeshesToSolidDocument, meshDocumentToSolidDocument, meshDataToPolygon, polygonToMeshObject } from './services/solidBridge'
import { emptyMeshDocument, type MeshWorkspaceDocument } from './services/meshEditing'
const DirectModeler = defineAsyncComponent(() => import('./features/DirectModeler.vue'))
const MeshModeler = defineAsyncComponent(() => import('./features/MeshModeler.vue'))
const MainModelingTools = defineAsyncComponent(() => import('./features/MainModelingTools.vue'))
const ScanPlanePanel = defineAsyncComponent(() => import('./features/ScanPlanePanel.vue'))
const SvgPanel = defineAsyncComponent(() => import('./features/SvgPanel.vue'))
const PhotogrammetryPanel = defineAsyncComponent(() => import('./features/PhotogrammetryPanel.vue'))
const InspectPanel = defineAsyncComponent(() => import('./components/InspectPanel.vue'))
const SceneOutliner = defineAsyncComponent(() => import('./components/SceneOutliner.vue'))
import KeyboardShortcuts from './components/KeyboardShortcuts.vue'
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
import { isPaletteCommandEnabled, type PaletteCommand } from './services/commandSearch'
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
import { ViewportController } from './services/viewportController'
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
    render: 'Собрать', auto: 'Авто', examples: 'Примеры', functionReference: 'Справочник функций',
    basic: 'Примитивы', csg: 'Настоящий CSG', house: 'Дом с модулями', tower: 'Параметрическая башня',
    open: 'Открыть', save: 'Сохранить', share: 'Поделиться',
    exportStl: 'Экспорт STL', exportObj: 'Экспорт OBJ', parameters: 'Параметры',
    convertFile: 'Конвертировать…', convertFileHelp: 'Конвертировать файл STL / OBJ / PLY / OFF / AMF / 3MF в выбранный формат', converted: 'Файл конвертирован', exportFormat: 'Формат экспорта',
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
    grid: 'Сетка', gridStep: 'Шаг сетки', view: 'Вид', iso: 'Изометрия', front: 'Спереди', back: 'Сзади',
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
    export: 'Экспорт', exportTitle: 'Экспорт модели', download: 'Скачать', cancel: 'Отмена', more: 'Ещё', close: 'Закрыть',
    svg: 'SVG ↔ 3D', photo: '3D по фотографиям', perf: 'Замеры сборки', modelTools: 'Инструменты моделирования',
    generators: 'Генераторы', codeToSolid: 'Перенести сцену в Solid', codeToMesh: 'Перенести сцену в Mesh', fileMenu: 'Файл', forPrinting: 'печать', forCad: 'CAD, Blender',
    savedDraftStatus: 'Черновик сохранён в браузере', exportScope: 'Экспортируется вся сцена одним телом', renderShortcut: 'Рендер',
  },
  en: {
    title: 'OpenSCAD Viewer',
    render: 'Render', auto: 'Auto', examples: 'Examples', functionReference: 'Function reference',
    basic: 'Primitives', csg: 'Real CSG', house: 'Modular house', tower: 'Parametric tower',
    open: 'Open', save: 'Save', share: 'Share',
    exportStl: 'Export STL', exportObj: 'Export OBJ', parameters: 'Parameters',
    convertFile: 'Convert…', convertFileHelp: 'Convert an STL / OBJ / PLY / OFF / AMF / 3MF file to the selected export format', converted: 'File converted', exportFormat: 'Export format',
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
    grid: 'Grid', gridStep: 'Grid step', view: 'View', iso: 'Isometric', front: 'Front', back: 'Back',
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
    export: 'Export', exportTitle: 'Export model', download: 'Download', cancel: 'Cancel', more: 'More', close: 'Close',
    svg: 'SVG ↔ 3D', photo: '3D from photos', perf: 'Build measurements', modelTools: 'Modeling tools',
    generators: 'Generators', codeToSolid: 'Bring scene into Solid', codeToMesh: 'Bring scene into Mesh', fileMenu: 'File', forPrinting: 'printing', forCad: 'CAD, Blender',
    savedDraftStatus: 'Draft saved in browser', exportScope: 'The whole scene is exported as one body', renderShortcut: 'Render',
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
const convertInputRef = ref<HTMLInputElement | null>(null)
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
const FunctionReference = defineAsyncComponent(() => import('./components/FunctionReference.vue'))
const functionReferenceOpen = ref(false)
const functionReferenceQuery = ref('')
const functionReferenceButton = ref<HTMLButtonElement | null>(null)
const mechanicalGeneratorOpen = ref(false)
const directModelerOpen = ref(false)
const meshModelerOpen = ref(false)
const solidSeedDocument = ref<DirectDocument | null>(null)
// Incremented to ask the Solid workspace to open its own command palette.
const solidPaletteRequest = ref(0)
const meshPaletteRequest = ref(0)
function openCommandPalette() {
  if (directModelerOpen.value) solidPaletteRequest.value++
  else if (meshModelerOpen.value) meshPaletteRequest.value++
  else paletteOpen.value = true
}
const meshSeedDocument = ref<MeshWorkspaceDocument | null>(null)
function sceneMeshesToMeshDocument(): MeshWorkspaceDocument {
  const doc = emptyMeshDocument()
  const prefix = lang.value === 'ru' ? 'Объект' : 'Object'
  sceneMeshes.value.forEach((mesh, index) => {
    const polygon = meshDataToPolygon(mesh)
    if (polygon) doc.objects.push(polygonToMeshObject(polygon, `${prefix} ${index + 1}`, `scene-${index + 1}-${Date.now().toString(36)}`))
  })
  return doc
}

const workspaceMode = computed<WorkspaceMode>(() => (meshModelerOpen.value ? 'mesh' : 'solid'))

/** Source is no longer a workspace of its own; it opens as a drawer over either one. */
const editorOpen = ref(false)
const solidBuilding = ref(false)
let solidBuildAbort: AbortController | null = null
function cancelSolidBuild() { solidBuildAbort?.abort() }
const solidAppendBodies = ref<{ bodies: DirectBody[]; token: number; group?: { name: string; source: string; replaces: string | null } } | null>(null)
/** Set while the left panel edits one scene group's source instead of the document. */
const groupEdit = ref<{ name: string; source: string; replaces: string | null } | null>(null)
const groupHighlight = computed(() => (groupEdit.value ? highlightCode(groupEdit.value.source, 'group.scad') : ''))

function openGroupEditor(request: { name: string; source: string; replaces: string | null }) {
  cancelSolidBuild()
  groupEdit.value = { ...request }
  editorOpen.value = true
}

function openWorkspaceMode(mode: WorkspaceMode) {
  if (mode === 'solid') {
    meshModelerOpen.value = false
    directModelerOpen.value = true
    return
  }
  directModelerOpen.value = false
  meshModelerOpen.value = true
}

// Solid is the resting workspace: with no Code tab there is nothing else to show.
if (!directModelerOpen.value && !meshModelerOpen.value) directModelerOpen.value = true

/**
 * Builds the source as exact solids and hands them to the Solid workspace.
 *
 * Evaluation and exact construction run in a disposable geometry worker.
 */
/** Builds one group's own source and hands the bodies back tagged with its name. */
async function buildSolidGroup(request: { name: string; source: string; replaces: string | null }) {
  if (solidBuilding.value) return false
  solidBuilding.value = true
  const abort = new AbortController()
  solidBuildAbort = abort
  error.value = ''
  try {
    const { buildExactSolidsInWorker } = await import('./services/solid/exactSolidClient')
    const built = await buildExactSolidsInWorker(request.source, abort.signal)
    if (abort.signal.aborted) return false
    if (built.length === 0) {
      error.value = lang.value === 'ru'
        ? 'Код группы не описывает ни одного тела.'
        : 'The group source describes no solids.'
      return false
    }
    const bodies = built
      .map(body => ({ ...body, group: request.name }))
    solidAppendBodies.value = {
      bodies,
      group: { name: request.name, source: request.source, replaces: request.replaces },
      token: (solidAppendBodies.value?.token ?? 0) + 1,
    }
    // Do not discard edits made while the requested snapshot was being built.
    if (groupEdit.value?.name === request.name && groupEdit.value.source === request.source) groupEdit.value = null
    return true
  } catch (caught) {
    if (!abort.signal.aborted) error.value = caught instanceof Error ? caught.message : String(caught)
    return false
  } finally {
    if (solidBuildAbort === abort) solidBuildAbort = null
    solidBuilding.value = false
  }
}

/**
 * The editor's document becomes one group in the Solid scene, named after the file
 * and carrying its source, so the dock lists it as code that can be edited and
 * rebuilt; building again replaces the group instead of stacking copies.
 */
async function buildSolidFromSource() {
  const name = fileName.value.replace(SOURCE_FILE_EXTENSION, '') || 'source'
  if (!await buildSolidGroup({ name, source: code.value, replaces: name })) return
  meshModelerOpen.value = false
  directModelerOpen.value = true
  editorOpen.value = false
}

function bringCodeToMesh() {
  if (!sceneMeshes.value.length) return
  try {
    meshSeedDocument.value = sceneMeshesToMeshDocument()
    solidSeedDocument.value = null
    directModelerOpen.value = false
    meshModelerOpen.value = true
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught)
  }
}

function bringCodeToSolid() {
  if (!sceneMeshes.value.length) return
  try {
    const next = sceneMeshesToSolidDocument(sceneMeshes.value, lang.value === 'ru' ? 'Тело' : 'Body')
    solidSeedDocument.value = next
    meshModelerOpen.value = false
    directModelerOpen.value = true
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught)
  }
}

function openMeshFromSolid() {
  solidSeedDocument.value = null
  meshSeedDocument.value = null
  directModelerOpen.value = false
  meshModelerOpen.value = true
}

function meshToSolid(doc: MeshWorkspaceDocument) {
  solidSeedDocument.value = meshDocumentToSolidDocument(doc)
  storageSet('scad-solid-modeler-v1', JSON.stringify(solidSeedDocument.value))
  meshModelerOpen.value = false
  directModelerOpen.value = true
}

function continueMainEditInSolid(document: DirectDocument) {
  solidSeedDocument.value = document
  storageSet('scad-solid-modeler-v1', JSON.stringify(document))
  meshModelerOpen.value = false
  directModelerOpen.value = true
}
const sourceHistoryKey=()=> 'scad-source-history-v1:'+fileName.value
const restoredMainHistory=restoreSourceHistory(storageGet(sourceHistoryKey()),code.value)
const mainEditPast = ref(restoredMainHistory.past)
const mainEditFuture = ref(restoredMainHistory.future)
watch(fileName,()=>{const d=restoreSourceHistory(storageGet(sourceHistoryKey()),code.value);mainEditPast.value=d.past;mainEditFuture.value=d.future},{flush:'post'})
watch([mainEditPast,mainEditFuture],()=>{const d=boundedSourceHistory({past:mainEditPast.value,future:mainEditFuture.value});if(!storageSet(sourceHistoryKey(),JSON.stringify(d)))notice.value='Could not persist modeling history.'},{deep:true,flush:'post'})
const mainCameraRevision=ref(0)
const mainSelectedIndices=ref<number[]>([])
const mainShiftSelection=ref(false)
const mainProject=(point:readonly number[])=>renderer?.projectWorldPoint(point)??null
const mainRay=(x:number,y:number)=>renderer?.worldRay(x,y)??null
let mainPreserveGroup=false
function mainSelectMany(indices:number[]){mainSelectedIndices.value=indices;mainPreserveGroup=true;try{renderer?.selectMesh(indices[0]??null)}finally{mainPreserveGroup=false}}
let mainPreviewActive = false
let mainEditSelection: {source:string;index:number}|null = null
function previewMainGeometry(meshes:MeshData[]|null) {
 if(meshes){mainPreviewActive=true;renderer?.setMeshes(meshes)}
 else if(mainPreviewActive){try{renderer?.setMeshes(sceneMeshes.value);renderer?.setMeshVisibilityBatch(meshVisibility.value);renderer?.selectMesh(selectedMesh.value)}finally{mainPreviewActive=false}}
}
function commitMainSource(source:string, selectIndex=selectedMesh.value) {
 try {
  if(source.length>MAX_WORKSPACE_SOURCE_LENGTH)throw Error('source limit')
  previewMainGeometry(null)
  if(source===code.value)return
  if(mainEditPast.value.at(-1)?.after!==code.value)mainEditPast.value=[]
  mainEditPast.value.push({before:code.value,after:source});while(mainEditPast.value.length>30||(mainEditPast.value.length>1&&mainEditPast.value.reduce((n,e)=>n+e.before.length+e.after.length,0)>8_000_000))mainEditPast.value.shift();mainEditFuture.value=[]
  mainEditSelection=selectIndex===null?null:{source,index:selectIndex}
  replacePresetSource(source);void nextTick(()=>doRender('full'))
 } catch(e){error.value=e instanceof Error?e.message:String(e)}
}
function appendMainPrimitive(source:string) {
 if(isModelGraphText(code.value)){error.value=lang.value==='ru'?'Примитивы доступны в документе OpenSCAD.':'Primitives require an OpenSCAD document.';return}
 commitMainSource(code.value+'\n'+source,-1)
}
function undoMainGeometry(redo=false){
 const from=redo?mainEditFuture:mainEditPast,to=redo?mainEditPast:mainEditFuture,entry=from.value.at(-1)
 if(!entry||code.value!==(redo?entry.before:entry.after))return
 previewMainGeometry(null);from.value.pop();to.value.push(entry);replacePresetSource(redo?entry.after:entry.before);void nextTick(()=>doRender('full'))
}

const viewportController = new ViewportController()
const viewportState = shallowRef(viewportController.state)
viewportController.subscribe(state => { viewportState.value = state })
const projection = computed(() => viewportState.value.camera.projection)
const gridVisible = ref(true)
/** Grid spacing presets in model units (OpenSCAD millimetres); 25.4 is one inch. */
const GRID_STEP_OPTIONS = [1, 2, 5, 10, 25, 25.4, 50, 100] as const
type GridStep = typeof GRID_STEP_OPTIONS[number]
function restoreGridStep(): GridStep {
  const stored = Number(storageGet('scad-grid-step'))
  return GRID_STEP_OPTIONS.find(step => step === stored) ?? 10
}
const gridStep = ref<GridStep>(restoreGridStep())
function gridStepLabel(step: GridStep) {
  if (step === 25.4) return '1 in'
  return step >= 10 && step % 10 === 0 ? `${step / 10} cm` : `${step} mm`
}
function changeGridStep() {
  renderer?.setGridStep(gridStep.value)
  storageSet('scad-grid-step', String(gridStep.value))
}
const standardView = computed({
  get: () => viewportState.value.standardView,
  set: (view: StandardView) => { viewportController.setStandardView(view) },
})
const activeView = computed(() => viewportState.value.activeView)
const cameraYaw = computed(() => viewportState.value.camera.yaw)
const cameraPitch = computed(() => viewportState.value.camera.pitch)
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
const hoveredHit = computed(() => sceneState.value.hoveredHit)
const isolated = computed({
  get: () => sceneState.value.isolated,
  set: (isolated: boolean) => { sceneController.update({ isolated }) },
})
const paletteOpen = ref(false)
const shortcutHelpOpen = ref(false)
const canPreviousView = computed(() => viewportState.value.canGoBack)
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
const measurement = computed(() => sceneState.value.measurement)
const measureActive = computed(() => sceneState.value.measureActive)
const sectionEnabled = computed(() => sceneState.value.section.enabled)
const sectionAxis = computed(() => sceneState.value.section.axis)
const sectionOffset = computed(() => sceneState.value.section.offset)
const sectionFlip = computed(() => sceneState.value.section.flip)
const scanPanelOpen = ref(false)
const scanToggleRef = ref<HTMLButtonElement | null>(null)
type DockTab = 'scene' | 'inspect' | 'parameters' | 'svg' | 'photo' | 'perf'
const dockTab = ref<DockTab>('scene')
// Direct-modeling actions of the Code viewport are reached through the command palette.
const mainToolsRef = ref<InstanceType<typeof MainModelingTools> | null>(null)
const MAIN_COMMAND_PREFIX = 'main:'
const visiblePaletteCommands = computed<PaletteCommand[]>(() => {
  const base = paletteCommands.value.filter(isPaletteCommandEnabled)
  // The source tools only make sense while the editor drawer is showing.
  if (!editorOpen.value) return base
  const tools = (mainToolsRef.value?.commands ?? []) as readonly PaletteCommand[]
  return [...base, ...tools.filter(command => command.enabled !== false).map(command => ({ ...command, id: MAIN_COMMAND_PREFIX + command.id }))]
})
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

const dockTabs: readonly DockTab[] = ['scene', 'inspect', 'parameters', 'svg', 'photo', 'perf']
const DOCK_ICONS: Record<DockTab, string> = {
  scene: 'M4 6h16M4 12h16M4 18h16',
  inspect: 'M12 3v3M12 18v3M3 12h3M18 12h3M12 7a5 5 0 1 0 0 10 5 5 0 0 0 0-10z',
  parameters: 'M4 8h10M18 8h2M4 16h4M12 16h8M16 6a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM10 14a2 2 0 1 0 0 4 2 2 0 0 0 0-4z',
  svg: 'M4 4h16v16H4zM4 15l5-5 4 4 3-3 4 4M15 8h.01',
  photo: 'M4 8h3l2-3h6l2 3h3v11H4zM12 17a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z',
  perf: 'M4 20V10M10 20V4M16 20v-8M22 20H2',
}
// Panels moved into the dock are <details> components; open them when their tab shows.
function revealDockDetails(el: Element | ComponentPublicInstance | null) {
  if (!(el instanceof HTMLElement)) return
  for (const details of el.querySelectorAll<HTMLDetailsElement>(':scope > details')) {
    if (!details.open) details.open = true
  }
}
const DOCK_KEY_MAP: Record<string, RovingFocusKey> = {
  ArrowLeft: 'ArrowLeft', ArrowUp: 'ArrowLeft', ArrowRight: 'ArrowRight', ArrowDown: 'ArrowRight', Home: 'Home', End: 'End',
}
const exportDialogOpen = ref(false)
const exportDialogRef = ref<HTMLElement | null>(null)
function openExportDialog() {
  exportDialogOpen.value = true
  void nextTick(() => {
    const dialog = exportDialogRef.value
    ;(dialog?.querySelector<HTMLElement>('input:checked') ?? dialog?.querySelector<HTMLElement>('button'))?.focus()
  })
}
async function confirmExport() {
  exportDialogOpen.value = false
  await exportAdditionalMesh()
}
const persistenceLabel = computed(() => workspaceConflict.value
  ? t('storageConflict')
  : workspacePersistenceStatus.value === 'saving' ? t('savingDraft')
    : workspacePersistenceStatus.value === 'error' ? t('unsavedDraft') : t('savedDraftStatus'))
function closeMenus() {
  for (const menu of document.querySelectorAll<HTMLDetailsElement>('.app details.menu[open]')) menu.open = false
}
function closeMenusOutside(event: Event) {
  const target = event.target as Element | null
  if (target?.closest('details.menu')) return
  closeMenus()
}
function handleDockTabKeydown(event: KeyboardEvent) {
  const key = DOCK_KEY_MAP[event.key]
  if (!key) return
  event.preventDefault()
  const current = dockTabs.indexOf(dockTab.value)
  const next = nextRovingIndex(current, dockTabs.length, key)
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

// The modelgraph compiler pulls the geometry kernel chunk; load it only when a
// modelgraph-text document is actually open.
const compactControls = ref<{parameters: import('./services/scadCustomizer').CustomizerParameter[]; errors: string[]}>({parameters: [], errors: []})
watchEffect(async () => {
  const source = code.value
  if (!isModelGraphText(source)) {
    compactControls.value = {parameters: [], errors: []}
    return
  }
  const controls = (await import('./services/modelGraphText')).modelGraphTextControls(source)
  if (code.value === source) compactControls.value = controls
})
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
  nextRenderer.setGridStep(gridStep.value)
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
    if (viewportController.isRecovering || mainPreviewActive) return
    if(mainShiftSelection.value&&index!==null){const ids=new Set(mainSelectedIndices.value);ids.has(index)?ids.delete(index):ids.add(index);mainSelectedIndices.value=[...ids];if(!ids.has(index)){index=mainSelectedIndices.value.at(-1)??null;hit=null}}
    else if(!mainPreserveGroup)mainSelectedIndices.value=index===null?[]:[index]
    mainShiftSelection.value=false
    sceneController.applyRendererSelection(index, isIsolated, hit)
  }
  instance.onHoverChange = hit => {
    if (viewportController.isRecovering || mainPreviewActive) return
    sceneController.applyRendererHover(hit)
  }
  instance.onMeasurementChange = (value, active) => {
    sceneController.applyRendererMeasurement(value, active)
  }
  instance.onCameraHistoryChange = available => { viewportController.applyHistoryAvailability(available) }
  instance.onFrameSubmitted = (token, at) => {
    if (instance !== renderer) return
    if (!pendingPerformanceFrame || pendingPerformanceFrame.frameToken !== token) return
    if (performanceHistory.submitted(pendingPerformanceFrame.historyToken, at)) performanceSamples.value = performanceHistory.snapshot()
  }
  instance.onCameraChange = syncCameraState
  instance.onStatusChange = event => handleRendererStatus(instance, event)
  viewportController.applyHistoryAvailability(instance.canGoToPreviousView)
  syncCameraState(instance.getCameraState())
}

onUnmounted(() => {
  solidBuildAbort?.abort()
  cancelGeometryAnalysis()
  layoutResizeObserver?.disconnect()
  layoutResizeObserver = null
  viewportController.invalidateRecovery()
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
  const token = viewportController.beginRecovery()
  // Triangle identities and hover ownership are renderer-local. Object
  // selection can be restored by scene identity, but stale surface hits cannot.
  selectedHit.value = null
  sceneController.applyRendererHover(null)
  let ready = false
  let failureMessage = t('gpuRecoverFailed')

  try {
    const recovered = await instance.init(canvas)
    if (!viewportController.isCurrentRecovery(token) || renderer !== instance) return
    if (!recovered) return
    if (instance.currentStatus.status === 'device-lost' && !rendererRecoveryGate.mustDeferReady) {
      rendererRecoveryGate.registerDeviceLoss()
    }
    if (rendererRecoveryGate.mustDeferReady) return

    instance.setDisplayMode(displayMode.value)
    instance.setBackgroundColor(themeCanvasColor(resolveTheme(themeSelection.value, systemPrefersDark.value)))
    instance.setSelectionMode(selectionMode.value)
    instance.setGridVisible(gridVisible.value)
    instance.setGridStep(gridStep.value)
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
    viewportController.completeRecovery(token)
    if (!viewportController.isCurrentRecovery(token) || renderer !== instance) return

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
    sceneController.applyRendererHover(null)
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
    if (!workspaceDocumentsEqual(result.restoredDocument, snapshot)) {
      await restoreWorkspaceDocument(result.restoredDocument)
    }
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
    // Real Worker events already own their buffers; skip the same-realm snapshot.
    snapshotEvents: false,
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
    mainSelectedIndices.value=[]
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
    sceneController.applyRendererHover(null)
    if (publication.nextSelectedIndex !== null) {
      const faceRestored = previousFaceHit && previousMeshes[previousFaceHit.meshIndex]
        && renderer?.restoreNativeFaceSelection(previousMeshes[previousFaceHit.meshIndex], previousFaceHit, publication.nextSelectedIndex)
      if (!faceRestored) renderer?.selectMesh(publication.nextSelectedIndex)
      if (publication.nextIsolated) renderer?.toggleIsolateSelection()
    } else {
      sceneController.applyRendererSelection(null, false, null)
    }
    if(mainEditSelection?.source===source){
      const index=mainEditSelection.index<0?displayMeshes.length-1:Math.min(mainEditSelection.index,displayMeshes.length-1)
      mainEditSelection=null
      if(index>=0)renderer?.selectMesh(index)
    }
    if (publication.measurementMayBePreserved) {
      // The renderer may rebuild this overlay; retaining the UI value avoids a
      // preview → full flash and lets it restore the exact world-space points.
      sceneController.setMeasurement(previousMeasurement, previousMeasureActive)
    } else {
      sceneController.setMeasurement(null, false)
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
  if (example) await loadEditorDocument(example, `${id}${sourceFileExtension(example)}`)
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
  if (!file) return
  if (detectMeshImportFormat(file.name)) await convertFile(file)
  else await openFile(file)
}

function triggerConvert() { convertInputRef.value?.click() }

async function convertSelectedFile(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (file) await convertFile(file)
}

/** Convert a dropped/picked mesh file to the selected export format and download it. */
async function convertFile(file: File) {
  try {
    const { convertMeshFile } = await import('./services/meshConvert')
    const result = await convertMeshFile(file.name, await file.arrayBuffer(), additionalExportFormat.value)
    downloadBlob(new Blob([result.data.slice().buffer as ArrayBuffer], { type: result.mimeType }), result.fileName)
    showNotice(`${t('converted')}: ${result.source.format.toUpperCase()} → ${result.format.toUpperCase()}`)
  } catch (error) { showNotice(error instanceof Error ? error.message : 'Conversion failed') }
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
  const requestedName = withSourceExtension(file.name, source)
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

const additionalExportFormat = ref<MeshExportFormat>('stl_binary')
async function exportAdditionalMesh() {
  if (!canExport.value) return
  try {
    const [{ exportMeshFormatCompressed }, { flattenExportMeshes }] = await Promise.all([
      import('./services/meshExportFormats'),
      import('./services/meshExportAdapter'),
    ])
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
  downloadBlob(new Blob([buffer], { type: 'model/stl' }), sanitizeFileName(fileName.value).replace(SOURCE_FILE_EXTENSION, '.stl'))
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
  viewportController.resetView()
  renderer?.resetView()
}
function previousView() {
  const state = renderer?.previousView()
  if (!state) return
  viewportController.applyRestoredCamera(state)
}

/** Mirrors the renderer camera into the UI so the view cube stays truthful. */
function syncCameraState(state: CameraState) {
  mainCameraRevision.value++
  viewportController.applyCamera(state)
  if (viewportController.state.camera.projection !== state.projection) {
    renderer?.setProjection(viewportController.state.camera.projection)
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
  renderer?.setProjection(viewportController.toggleProjection())
}
function toggleGrid() {
  gridVisible.value = !gridVisible.value
  renderer?.setGridVisible(gridVisible.value)
}
function changeStandardView() {
  const { view, projection: nextProjection } = viewportController.applyStandardView(viewportController.state.standardView)
  renderer?.setCameraPreset(view, nextProjection)
}
function setStandardView(view: StandardView) {
  viewportController.setStandardView(view)
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
  sceneController.setMeasurement(measurement.value, true)
  dockOpen.value = true
  dockTab.value = 'inspect'
  renderer?.setMeasureMode(true)
}

function cancelMeasure() {
  sceneController.setMeasurement(measurement.value, false)
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
    sceneController.setSection({ offset: (min + max) / 2 })
  }
  applySection()
}

function setSectionEnabled(enabled: boolean) {
  if (enabled && !sectionAvailable.value) return
  const patch = enabled && !sceneState.value.section.initialized
    ? { enabled, offset: (sectionRange.value.min + sectionRange.value.max) / 2, initialized: true }
    : { enabled }
  sceneController.setSection(patch)
  applySection()
}

function setSectionAxis(axis: SectionAxis) {
  sceneController.setSection({
    axis,
    offset: (sectionRange.value.min + sectionRange.value.max) / 2,
    initialized: true,
  })
  applySection()
}

function setSectionOffset(offset: number) {
  if (!Number.isFinite(offset)) return
  sceneController.setSection({
    offset: clamp(offset, sectionRange.value.min, sectionRange.value.max),
    initialized: true,
  })
  applySection()
}

function setSectionFlip(flip: boolean) {
  sceneController.setSection({ flip })
  applySection()
}

function resetSection() {
  sceneController.setSection({
    offset: (sectionRange.value.min + sectionRange.value.max) / 2,
    flip: false,
    initialized: true,
  })
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
  if (id.startsWith(MAIN_COMMAND_PREFIX)) { paletteOpen.value = false; mainToolsRef.value?.execute(id.slice(MAIN_COMMAND_PREFIX.length)); return }
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
    case 'function-reference':
      if (paletteWasOpen) functionReferenceButton.value?.focus()
      openFunctionReference(document.activeElement === editorRef.value ? selectedEditorName.value : '')
      break
    case 'export-stl': exportStl(); break
    case 'export-obj': exportObj(); break
    case 'convert-mesh': triggerConvert(); break
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

function openFunctionReference(query = '') {
  functionReferenceQuery.value = query
  functionReferenceOpen.value = true
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
  if (shortcutHelpOpen.value || exampleGalleryOpen.value || mechanicalGeneratorOpen.value || functionReferenceOpen.value) return
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
function sanitizeFileName(name: string) { return (name.replace(/[^\w.() -]+/g, '_') || 'model.scad').replace(/\.mg.*$/i, '.mg').replace(/\.scad.*$/i, '.scad') }

</script>

<template>
  <div class="app" @dragover.prevent @drop.prevent="handleDrop" @pointerdown.capture="closeMenusOutside">
    <nav class="topbar" aria-label="Application" :inert="functionReferenceOpen">
      <div class="topbar-left">
        <svg class="logo" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" aria-hidden="true">
          <path d="M12 3 3 8v8l9 5 9-5V8z"/><path d="M3 8l9 5 9-5M12 13v8"/>
        </svg>
        <span class="brand">{{ t('title') }}</span>
        <span class="topbar-divider" aria-hidden="true" />
        <details class="menu file-menu">
          <summary class="file-chip" :title="t('fileMenu')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><path d="M14 3v6h6"/></svg>
            <span class="file-chip-name">{{ fileName }}</span>
            <span v-if="workspaceConflict || workspacePersistenceStatus !== 'saved'" class="file-chip-dot" :class="workspaceConflict ? 'error' : workspacePersistenceStatus" aria-hidden="true" />
            <svg class="chevron" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>
          </summary>
          <div class="menu-list" @click="closeMenus">
            <button type="button" :title="t('openFile')" @click="triggerOpen">{{ t('open') }}</button>
            <button type="button" :title="t('saveFile')" @click="saveSource">{{ t('save') }}</button>
            <button type="button" :disabled="savingBrowser" :title="t('saveBrowserHelp')" @click="saveBrowserDraft">{{ savingBrowser ? t('savingDraft') : t('saveBrowser') }}</button>
            <button type="button" :title="t('shareFile')" @click="shareSource">{{ t('share') }}</button>
            <button type="button" :title="t('convertFileHelp')" @click="triggerConvert">{{ t('convertFile') }}</button>
            <span class="menu-status" role="status">{{ persistenceLabel }}</span>
          </div>
        </details>
        <input ref="fileInputRef" class="sr-only" type="file" :accept="SOURCE_FILE_ACCEPT" @change="openSelectedFile">
        <input ref="convertInputRef" class="sr-only" type="file" :accept="MESH_IMPORT_ACCEPT" @change="convertSelectedFile">
      </div>
      <div class="mode-switch" role="group" :aria-label="lang === 'ru' ? 'Режим работы' : 'Workspace mode'">
        <button
          v-for="mode in WORKSPACE_MODES"
          :key="mode"
          type="button"
          :class="{ active: workspaceMode === mode }"
          :aria-pressed="workspaceMode === mode"
          :title="workspaceModeHint(mode, lang)"
          @click="openWorkspaceMode(mode)"
        >{{ workspaceModeLabel(mode, lang) }}</button>
      </div>
      <button
        class="icon-btn source-toggle"
        type="button"
        :class="{ active: editorOpen }"
        :aria-pressed="editorOpen"
        :aria-label="lang === 'ru' ? 'Исходный код' : 'Source'"
        :title="lang === 'ru' ? 'Исходный код' : 'Source'"
        @click="editorOpen = !editorOpen"
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M8 6 3 12l5 6M16 6l5 6-5 6"/></svg>
      </button>
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
          :title="storageFailureDetail || t('retrySave')"
          @click="retryWorkspacePersistence"
        >⚠ {{ t('unsavedDraft') }}</button>
        <button class="icon-btn command-btn" type="button" :title="t('commandHelp')" aria-keyshortcuts="Control+K Meta+K" @click="openCommandPalette">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>
          <span>{{ t('commands') }}</span> <kbd>Ctrl K</kbd>
        </button>
        <button class="btn topbar-action" type="button" :title="t('exportTitle')" @click="openExportDialog">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M12 3v12M6 9l6 6 6-6"/><path d="M4 19h16"/></svg>
          <span>{{ t('export') }}</span>
        </button>
        <button class="btn topbar-action share-action" type="button" :title="t('shareFile')" @click="shareSource">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><path d="m8.6 13.5 6.8 4M15.4 6.5l-6.8 4"/></svg>
          <span>{{ t('share') }}</span>
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
          <svg v-if="isDark" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z"/></svg>
          <svg v-else width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></svg>
        </button>
      </div>
    </nav>

    <main
      ref="mainRef"
      class="main"
      :class="{ 'editor-drawer': editorOpen }"
      :inert="!editorOpen && (directModelerOpen || meshModelerOpen || functionReferenceOpen)"
    >
      <section
        v-if="groupEdit"
        class="editor-panel group-editor"
        :style="{ width: `${editorWidth}px` }"
        :aria-label="lang === 'ru' ? 'Код группы' : 'Group source'"
      >
        <div class="toolbar editor-toolbar">
          <input
            v-model="groupEdit.name"
            class="group-name-input"
            type="text"
            maxlength="100"
            :aria-label="lang === 'ru' ? 'Имя группы' : 'Group name'"
          >
          <span class="toolbar-spacer" aria-hidden="true" />
          <button class="btn" type="button" @click="cancelSolidBuild(); groupEdit = null">{{ lang === 'ru' ? 'Отмена' : 'Cancel' }}</button>
          <button
            class="btn btn-primary"
            type="button"
            :disabled="solidBuilding || !groupEdit.name.trim() || !groupEdit.source.trim()"
            @click="buildSolidGroup({ ...groupEdit })"
          >{{ solidBuilding ? '…' : (lang === 'ru' ? 'Построить' : 'Build') }}</button>
        </div>
        <div class="code-area group-code">
          <pre class="code code-highlight" aria-hidden="true"><span class="highlight-content" v-html="groupHighlight" /></pre>
          <textarea
            v-model="groupEdit.source"
            class="code code-input"
            wrap="off"
            spellcheck="false"
            autocomplete="off"
            :aria-label="lang === 'ru' ? 'Код группы' : 'Group source'"
            :maxlength="100000"
          />
        </div>
        <p class="group-hint">{{ lang === 'ru'
          ? 'Строится как точные тела. hull, projection, offset и polyhedron точной формы не имеют и будут отклонены.'
          : 'Built as exact solids. hull, projection, offset and polyhedron have no exact form and are refused.' }}</p>
      </section>
      <section v-else class="editor-panel" :style="{ width: `${editorWidth}px` }" :aria-label="t('editor')">
        <div class="toolbar editor-toolbar">
          <button class="btn btn-primary" type="button" title="Ctrl/⌘+Enter" :disabled="rendering" @click="doRender('full')">
            <svg class="play" width="11" height="11" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M6 4v16l14-8z"/></svg>
            {{ t('render') }}
          </button>
          <label class="auto-check"><input v-model="autoRender" type="checkbox"> {{ t('auto') }}</label>
          <button
            class="btn"
            type="button"
            :disabled="solidBuilding"
            :title="lang === 'ru' ? 'Собрать точные тела (NURBS) и открыть в Solid' : 'Build exact NURBS solids and open them in Solid'"
            @click="buildSolidFromSource()"
          >{{ solidBuilding ? '…' : (lang === 'ru' ? 'В Solid' : 'To Solid') }}</button>
          <button v-if="solidBuilding" class="btn" type="button" @click="cancelSolidBuild()">{{ lang === 'ru' ? 'Отмена' : 'Cancel' }}</button>
          <span class="toolbar-spacer" aria-hidden="true" />
          <button class="btn" type="button" @click="exampleGalleryOpen = true">{{ t('examples') }}</button>
          <button class="btn" type="button" @click="mechanicalGeneratorOpen = true">{{ t('generators') }}</button>
          <button
            ref="functionReferenceButton"
            class="icon-btn"
            type="button"
            :title="`${t('functionReference')} (F1)`"
            :aria-label="t('functionReference')"
            aria-keyshortcuts="F1"
            aria-haspopup="dialog"
            :aria-expanded="functionReferenceOpen"
            @click="openFunctionReference()"
          >
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.7.3-1 .8-1 1.5"/><path d="M12 17h.01"/></svg>
          </button>
          <details class="menu more-menu">
            <summary class="icon-btn" :title="t('more')" :aria-label="t('more')">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><circle cx="5" cy="12" r="2"/><circle cx="12" cy="12" r="2"/><circle cx="19" cy="12" r="2"/></svg>
            </summary>
            <div class="menu-list menu-right" @click="closeMenus">
              <button type="button" aria-keyshortcuts="Alt+Shift+F" @click="formatEditor">{{ t('format') }} <kbd>Alt Shift F</kbd></button>
              <button type="button" @click="openEditorFind(false)">{{ t('find') }}</button>
              <button type="button" @click="openEditorFind(true)">{{ t('replace') }}</button>
              <button type="button" :disabled="!sceneMeshes.length || rendering" @click="bringCodeToSolid">{{ t('codeToSolid') }}</button>
              <button type="button" :disabled="!sceneMeshes.length || rendering" @click="bringCodeToMesh">{{ t('codeToMesh') }}</button>
            </div>
          </details>
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

      <section class="canvas-panel" :class="{ 'with-dock': dockOpen }" :aria-label="t('viewport')" @pointerdown.capture="mainShiftSelection = $event.shiftKey">
        <MainModelingTools ref="mainToolsRef" :project="mainProject" :ray="mainRay" :camera-revision="mainCameraRevision" :selected-indices="mainSelectedIndices" @select-many="mainSelectMany" :meshes="sceneMeshes" :selected="selectedMesh" :hit="selectedHit" :source="code" :ready="!rendering && !stale && renderedSource === code" :locale="lang" :can-undo="!rendering && mainEditPast.at(-1)?.after === code" :can-redo="!rendering && mainEditFuture.at(-1)?.before === code" @append="appendMainPrimitive" @apply="commitMainSource" @solid="continueMainEditInSolid" @preview="previewMainGeometry" @undo="undoMainGeometry()" @redo="undoMainGeometry(true)" />
        <div class="viewer-toolbar" :class="{ 'with-dock': dockOpen }">
          <button class="view-btn icon-only" type="button" :title="t('fit')" :aria-label="t('fit')" @click="fitView">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5"/></svg>
          </button>
          <button class="view-btn icon-only" type="button" :title="t('reset')" :aria-label="t('reset')" @click="resetView">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M3 12a9 9 0 1 0 3-6.7"/><path d="M3 4v5h5"/></svg>
          </button>
          <button
            class="view-btn icon-only"
            type="button"
            :disabled="!canPreviousView"
            :aria-label="t('previousView')"
            :title="canPreviousView ? `${t('previousView')} · [` : t('noPreviousView')"
            @click="previousView"
          ><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M19 12H5M11 6l-6 6 6 6"/></svg></button>
          <button class="view-btn icon-only" type="button" :aria-pressed="projection === 'orthographic'" :aria-label="projection === 'perspective' ? t('perspective') : t('orthographic')" :title="`${projection === 'perspective' ? t('perspective') : t('orthographic')} → ${projection === 'perspective' ? t('orthographic') : t('perspective')}`" @click="toggleProjection">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path v-if="projection === 'perspective'" d="M4 20L9 4h6l5 16zM6.5 12h11M5 16h14"/><path v-else d="M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7M12 11v10"/></svg>
          </button>
          <button class="view-btn icon-only" type="button" :aria-pressed="gridVisible" :aria-label="t('grid')" :title="t('grid')" @click="toggleGrid">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 9h18M3 15h18M9 3v18M15 3v18"/><rect x="3" y="3" width="18" height="18" rx="2"/></svg>
          </button>
          <label class="view-select-label">
            <span class="sr-only">{{ t('gridStep') }}</span>
            <select v-model="gridStep" class="view-select grid-step-select" :aria-label="t('gridStep')" :title="t('gridStep')" :disabled="!gridVisible" @change="changeGridStep">
              <option v-for="step in GRID_STEP_OPTIONS" :key="step" :value="step">{{ gridStepLabel(step) }}</option>
            </select>
          </label>
          <button
            ref="scanToggleRef" class="view-btn icon-only scan-toggle" type="button"
            :class="{ active: sectionEnabled }" :aria-label="t('section')" :title="t('scanPlane')"
            :aria-expanded="scanPanelOpen" aria-controls="scan-plane-panel"
            :disabled="!sectionAvailable && !sectionEnabled"
            @click="scanPanelOpen = !scanPanelOpen"
          ><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4z"/><path d="M4 7v10l8 4 8-4V7"/><path d="M2 12h20" stroke-dasharray="3 2"/></svg><i v-if="sectionEnabled" class="scan-active-dot" aria-hidden="true" /></button>
          <button ref="dockToggleRef" class="view-btn icon-only" type="button" :aria-label="t('sidebar')" :aria-pressed="dockOpen" :aria-expanded="dockOpen" aria-controls="cad-sidebar" @click="dockOpen = !dockOpen">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/></svg>
          </button>
          <div class="display-modes" role="group" :aria-label="t('display')">
            <button class="view-btn icon-only" type="button" :aria-pressed="displayMode === 'shaded'" :aria-label="t('shaded')" :title="t('shaded')" @click="setDisplayMode('shaded')">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4z" fill="currentColor" fill-opacity="0.35"/><path d="M4 7v10l8 4 8-4V7M12 11v10"/></svg>
            </button>
            <button class="view-btn icon-only" type="button" :aria-pressed="displayMode === 'edges'" :aria-label="t('edges')" :title="t('edges')" @click="setDisplayMode('edges')">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7M12 11v10M4 7l8 14M20 7l-8 14"/></svg>
            </button>
            <button class="view-btn icon-only" type="button" :aria-pressed="displayMode === 'xray'" :aria-label="t('xray')" :title="t('xray')" @click="setDisplayMode('xray')">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7M12 11v10" stroke-opacity="0.45"/><path d="M4 17l8-4 8 4M12 3v10"/></svg>
            </button>
          </div>
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
          <div class="dock-rail" role="tablist" aria-orientation="vertical" :aria-label="t('sidebar')" @keydown="handleDockTabKeydown">
            <button
              v-for="tab in dockTabs"
              :id="`dock-tab-${tab}`"
              :key="tab"
              type="button"
              role="tab"
              aria-controls="dock-panel"
              :aria-selected="dockTab === tab"
              :aria-label="t(tab)"
              :title="t(tab)"
              :tabindex="dockTab === tab ? 0 : -1"
              :class="{ active: dockTab === tab }"
              @click="dockTab = tab"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path :d="DOCK_ICONS[tab]"/></svg>
            </button>
            <span class="dock-rail-spacer" aria-hidden="true" />
            <button class="dock-close" type="button" :aria-label="t('closeSidebar')" :title="t('closeSidebar')" @click="closeDock">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M9 6l6 6-6 6"/></svg>
            </button>
          </div>
          <div class="dock-body">
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
          <div v-else-if="dockTab === 'svg'" class="dock-scroll" :ref="revealDockDetails">
          <SvgPanel :meshes="sceneMeshes" :hit="selectedHit" :available="canExport" :locale="lang" :can-append="!isModelGraphText(code)" :remaining-source="MAX_WORKSPACE_SOURCE_LENGTH - code.length - 2" :append-revision="workspaceDocument.documentId + ':' + workspaceDocument.mutation" @append="source => { replacePresetSource(code + '\n\n' + source); nextTick(() => doRender('full')) }" />
          </div>
          <div v-else-if="dockTab === 'photo'" class="dock-scroll" :ref="revealDockDetails">
          <PhotogrammetryPanel :locale="lang" :can-append="!isModelGraphText(code)" :remaining-source="MAX_WORKSPACE_SOURCE_LENGTH - code.length - 2" @append="source => replacePresetSource(code + '\n\n' + source)" />
          </div>
          <div v-else-if="dockTab === 'perf'" class="dock-scroll" :ref="revealDockDetails">
<details class="performance-panel" open>
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
        <div class="canvas-hint" :class="{ 'with-dock': dockOpen }">{{ t('hint') }}</div>
      </section>
    </main>

    <footer class="statusbar" :aria-label="lang === 'ru' ? 'Состояние' : 'Status'">
      <span class="status-item"><i class="status-dot" :class="gpuOk ? 'ok' : 'off'" aria-hidden="true" />{{ currentBackendQuality.backend === 'webgpu-interactive' ? t('backendWebGpu') : t('backendHeadless') }}</span>
      <span class="status-item kernel-badge">Manifold</span>
      <span v-if="currentBackendQuality.transparency === 'object-sorted-alpha'" class="status-item">{{ t('transparencySorted') }}</span>
      <span class="status-item" role="status">{{ persistenceLabel }}</span>
    </footer>

    <div v-if="exportDialogOpen" class="dialog-backdrop" @click.self="exportDialogOpen = false">
      <div ref="exportDialogRef" class="dialog export-dialog" role="dialog" aria-modal="true" aria-labelledby="export-dialog-title" @keydown.escape.stop="exportDialogOpen = false">
        <header class="dialog-header">
          <div>
            <h2 id="export-dialog-title">{{ t('exportTitle') }}</h2>
            <p>{{ fileName }} · {{ t('meshes') }} {{ formatNumber(meshCount) }} · {{ t('triangles') }} {{ formatNumber(triangleCount) }}</p>
          </div>
          <button class="icon-btn" type="button" :aria-label="t('close')" @click="exportDialogOpen = false">×</button>
        </header>
        <fieldset class="format-grid">
          <legend>{{ t('exportFormat') }}</legend>
          <label v-for="format in MESH_EXPORT_FORMATS" :key="format" class="format-card" :class="{ active: additionalExportFormat === format }">
            <input v-model="additionalExportFormat" type="radio" name="export-format" :value="format">
            <span class="format-name">{{ MESH_FORMAT_LABELS[format] }}</span>
            <small>{{ format === 'obj' || format === 'ply' || format === 'off' ? t('forCad') : t('forPrinting') }}</small>
          </label>
        </fieldset>
        <p class="dialog-note" :class="{ warning: !canExport }">{{ canExport ? t('exportScope') : t('needsFullBuild') }}</p>
        <footer class="dialog-footer">
          <button class="btn" type="button" :title="t('convertFileHelp')" @click="exportDialogOpen = false; triggerConvert()">{{ t('convertFile') }}</button>
          <span class="statusbar-spacer" aria-hidden="true" />
          <button class="btn" type="button" @click="exportDialogOpen = false">{{ t('cancel') }}</button>
          <button class="btn btn-primary" type="button" :disabled="!canExport" @click="confirmExport">{{ t('download') }} {{ MESH_FORMAT_LABELS[additionalExportFormat] }}</button>
        </footer>
      </div>
    </div>

    <div v-if="notice" class="toast" role="status">{{ notice }}</div>
    <CommandPalette
      :open="paletteOpen"
      :commands="visiblePaletteCommands"
      :restore-focus="!shortcutHelpOpen && !functionReferenceOpen"
      @close="paletteOpen = false"
      @execute="executeCommand"
    />
    <DirectModeler
      :open="directModelerOpen"
      :locale="lang"
      :can-append="!isModelGraphText(code)"
      :remaining-source="MAX_WORKSPACE_SOURCE_LENGTH - code.length - 2"
      :seed-document="solidSeedDocument"
      :append-bodies="solidAppendBodies"
      @edit-group="openGroupEditor"
      :palette-request="solidPaletteRequest"
      @close="directModelerOpen = false; solidSeedDocument = null"
      @to-mesh="openMeshFromSolid"
      @append="source => { replacePresetSource(code + '\n\n' + source); nextTick(() => doRender('full')) }"
    />
    <MeshModeler
      :open="meshModelerOpen"
      :locale="lang"
      :seed-document="meshSeedDocument"
      :palette-request="meshPaletteRequest"
      @close="meshModelerOpen = false"
      @export-to-solid="meshToSolid"
    />
    <MechanicalGenerator :open="mechanicalGeneratorOpen" :locale="lang" @close="mechanicalGeneratorOpen = false" @generate="loadMechanicalModel" @download-current="saveSource" />
    <ExampleGallery
      :open="exampleGalleryOpen"
      :examples="EXAMPLE_CATALOG"
      :locale="lang"
      @close="exampleGalleryOpen = false"
      @select="loadExample"
      @download-current="saveSource"
    />
    <FunctionReference
      :open="functionReferenceOpen"
      :locale="lang"
      :language="isModelGraphText(code) ? 'modelgraph' : 'openscad'"
      :initial-query="functionReferenceQuery"
      @close="functionReferenceOpen = false"
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
  --bg: #1c1a17;
  --surface: #221f1b;
  --surface-raised: #2a2622;
  --border: #3a352e;
  --text: #f1ece3;
  --text-dim: #a8a094;
  --accent: #d97757;
  --accent-strong: #b5533a;
  --hover: #33302a;
  --danger: #ff8f80;
  --warning: #f0b458;
  --canvas-bg: #141210;
  --focus: #f0a488;
  --font-ui: "Inter Tight", ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --font-mono: "JetBrains Mono", "SFMono-Regular", Consolas, monospace;
  --radius: 8px;
}

[data-theme="light"] {
  --bg: #f4f1ea;
  --surface: #fbfaf7;
  --surface-raised: #efebe2;
  --border: #d5cfc2;
  --text: #1f1c18;
  --text-dim: #5e574d;
  --accent: #b8543a;
  --accent-strong: #a2472f;
  --hover: #ebe6db;
  --danger: #b3261e;
  --warning: #8a5b00;
  --canvas-bg: #e9e4da;
  --focus: #a2472f;
}

*, *::before, *::after { box-sizing: border-box; }
html, body, #app { width: 100%; height: 100%; margin: 0; }
body {
  overflow: hidden;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-ui);
  font-size: 13px;
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

/* Top bar */
.topbar {
  z-index: 10; height: 46px; display: grid; grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr); align-items: center; gap: 12px;
  padding: 0 12px 0 14px; background: var(--surface); border-bottom: 1px solid var(--border); flex-shrink: 0;
}
.topbar-left, .topbar-right { display: flex; align-items: center; gap: 8px; min-width: 0; }
.topbar-right { justify-content: flex-end; }
.logo { color: var(--accent); flex-shrink: 0; }
.brand { font-weight: 600; font-size: 14px; letter-spacing: -0.01em; white-space: nowrap; }
.topbar-divider { width: 1px; height: 20px; background: var(--border); margin-inline: 4px; }
.kernel-badge { font-family: var(--font-mono); }

.icon-btn, .btn, .view-btn {
  min-height: 30px; border: 1px solid var(--border); border-radius: var(--radius); background: transparent;
  color: var(--text); cursor: pointer; transition: background .12s, border-color .12s;
  display: inline-flex; align-items: center; justify-content: center; gap: 6px;
}
.icon-btn { min-width: 30px; padding: 0 7px; color: var(--text-dim); }
.icon-btn:hover, .btn:hover, .view-btn:hover { background: var(--hover); color: var(--text); }
.icon-btn:focus-visible, .btn:focus-visible, .view-btn:focus-visible, .select:focus-visible, summary:focus-visible,
.view-select:focus-visible, .splitter:focus-visible, .gpu-canvas:focus-visible, .code:focus-visible, .dock-rail button:focus-visible,
.selection-modes button:focus-visible, .format-card:focus-within, .mode-switch button:focus-visible {
  outline: 2px solid var(--focus); outline-offset: 2px;
}
.btn { padding: 0 10px; font-size: 13px; font-weight: 500; white-space: nowrap; }
.btn:disabled, .icon-btn:disabled { opacity: .5; cursor: default; }
.btn-primary { background: var(--accent); border-color: var(--accent); color: var(--bg); font-weight: 600; }
.btn-primary:hover { background: var(--accent-strong); border-color: var(--accent-strong); color: #fff; }
.btn-primary:disabled { cursor: progress; }
.topbar-action span { display: inline; }
.command-btn { min-width: 150px; justify-content: flex-start; padding-inline: 10px; background: var(--bg); color: var(--text-dim); font-weight: 400; }
.command-btn span { flex: 1; text-align: left; }
.command-btn kbd, .menu-list kbd, .status-hint kbd {
  padding: 1px 5px; border: 1px solid var(--border); border-radius: 4px; color: var(--text-dim);
  font: 11px var(--font-mono); white-space: nowrap;
}
.lang-btn { font-size: 12px; font-weight: 600; }
.theme-picker select {
  max-width: 100px; height: 30px; padding: 0 7px; color: inherit; background: transparent;
  border: 1px solid var(--border); border-radius: var(--radius); font-size: 12px;
}

/* Menus (details/summary) */
.menu { position: relative; }
.menu > summary { list-style: none; cursor: pointer; }
.menu > summary::-webkit-details-marker { display: none; }
.menu-list {
  position: absolute; z-index: 20; top: calc(100% + 6px); left: 0; min-width: 220px; display: flex; flex-direction: column; gap: 2px;
  padding: 6px; border: 1px solid var(--border); border-radius: 10px; background: var(--surface);
  box-shadow: 0 16px 44px rgba(0,0,0,.28);
}
.menu-list.menu-right { left: auto; right: 0; }
.more-menu { margin-left: auto; }
.menu-list button {
  display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 32px; padding: 0 10px;
  border: 0; border-radius: 6px; background: transparent; color: var(--text); cursor: pointer; text-align: left; font-size: 13px; white-space: nowrap;
}
.menu-list button:hover, .menu-list button:focus-visible { background: var(--hover); outline: none; }
.menu-list button:disabled { opacity: .45; cursor: default; }
.menu-status { padding: 8px 10px 4px; border-top: 1px solid var(--border); margin-top: 4px; color: var(--text-dim); font-size: 12px; }
.file-chip {
  display: inline-flex; align-items: center; gap: 8px; height: 30px; padding: 0 10px; border-radius: var(--radius);
  border: 1px solid transparent; color: var(--text); max-width: 320px;
}
.file-chip:hover, .menu[open] > .file-chip { background: var(--hover); border-color: var(--border); }
.file-chip svg { color: var(--text-dim); flex-shrink: 0; }
.file-chip-name { font-family: var(--font-mono); font-size: 12.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.file-chip-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--warning); flex-shrink: 0; }
.file-chip-dot.error { background: var(--danger); }
.persistence-status { border: 0; background: transparent; color: var(--text-dim); font-size: 12px; white-space: nowrap; }
.persistence-error { color: var(--danger); cursor: pointer; }
.persistence-error:hover { text-decoration: underline; }
.persistence-conflict { display: flex; align-items: center; gap: 5px; color: var(--danger); font-size: 12px; }
.persistence-conflict button {
  border: 1px solid var(--border); border-radius: 6px; background: var(--surface-raised);
  color: var(--text); padding: 3px 6px; cursor: pointer;
}

/* Mode switch */
.mode-switch { display: inline-flex; gap: 2px; padding: 3px; border: 1px solid var(--border); border-radius: 9px; justify-self: center; }
.mode-switch button {
  min-width: 64px; height: 26px; padding: 0 14px; border: 0; border-radius: 6px; background: transparent;
  color: var(--text-dim); font-weight: 500; cursor: pointer;
}
.mode-switch button:hover { color: var(--text); background: var(--hover); }
.mode-switch button.active { background: var(--text); color: var(--bg); font-weight: 600; }

/* Layout */
.no-gpu { position: absolute; z-index: 8; inset: 0; display: flex; align-items: center; justify-content: center; flex-direction: column; gap: 14px; color: var(--danger); background: var(--canvas-bg); font-size: 1rem; padding: 40px; text-align: center; }
.main { flex: 1; min-height: 0; display: flex; overflow: hidden; }
/* With no Code workspace the source opens over the active one. The viewport stays
   laid out off to the side: hiding it would resize its canvas to zero. */
.main.editor-drawer {
  position: fixed;
  inset: 46px auto 28px 0;
  z-index: 30;
  width: min(560px, 82vw);
  border-right: 1px solid var(--border);
  box-shadow: 0 0 40px #0006;
}
.main.editor-drawer > .splitter { display: none; }
.main.editor-drawer > section:not(.editor-panel) {
  position: absolute;
  inset: 0 auto 0 100%;
  width: 60vw;
  visibility: hidden;
  pointer-events: none;
}
.main.editor-drawer .editor-panel { width: 100% !important; max-width: none; }
.source-toggle.active { color: var(--accent); border-color: var(--accent); }
.group-editor { display: flex; flex-direction: column; min-height: 0; }
.group-name-input { flex: 1; min-width: 0; padding: 5px 8px; background: var(--surface-raised); color: var(--text); border: 1px solid var(--border); border-radius: 5px; font: inherit; }
.group-editor .code-area { position: relative; flex: 1; min-height: 0; overflow: auto; }
.group-editor .code-highlight { position: absolute; inset: 0; margin: 0; pointer-events: none; }
.group-editor .code-input { position: relative; width: 100%; height: 100%; background: transparent; color: transparent; caret-color: var(--text); border: 0; resize: none; outline: none; }
.group-hint { margin: 0; padding: 8px 12px; color: var(--text-dim); font-size: 11.5px; line-height: 1.5; border-top: 1px solid var(--border); }

.editor-panel {
  min-width: 300px; min-height: 0; max-width: calc(100vw - 320px); display: flex; flex-direction: column;
  background: var(--bg); border-right: 1px solid var(--border); overflow-x: hidden; overflow-y: auto;
}
.toolbar { display: flex; align-items: center; gap: 6px; padding: 6px 8px; border-bottom: 1px solid var(--border); }
.editor-toolbar { flex-wrap: wrap; min-height: 42px; }
.toolbar-spacer, .statusbar-spacer { flex: 1; }
.play { margin-right: 1px; }
.auto-check { display: flex; align-items: center; gap: 7px; color: var(--text-dim); font-size: 13px; cursor: pointer; padding-inline: 4px; }
.auto-check input { accent-color: var(--accent); margin: 0; }
.toolbar-divider { width: 1px; align-self: stretch; background: var(--border); margin-inline: 2px; }
.select, .view-select {
  height: 28px; max-width: 155px; border: 1px solid var(--border); border-radius: 6px;
  padding: 0 22px 0 8px; background: var(--bg); color: var(--text); font-size: 12px;
}

/* Find */
.find-bar {
  display: flex; flex-wrap: wrap; align-items: center; gap: 5px; padding: 6px 10px;
  border-bottom: 1px solid var(--border); background: var(--surface); font-size: 12px;
}
.find-bar > input[type="search"], .find-bar > input[type="text"] {
  min-width: 90px; flex: 1 1 110px; height: 28px; padding: 3px 8px; border: 1px solid var(--border);
  border-radius: 6px; background: var(--bg); color: var(--text);
}
.find-bar button { min-width: 28px; min-height: 28px; border: 1px solid var(--border); border-radius: 6px; background: transparent; color: var(--text); cursor: pointer; }
.find-bar button:hover { background: var(--hover); }
.find-bar button:disabled { opacity: .45; cursor: default; }
.find-status { min-width: 42px; color: var(--text-dim); text-align: center; }
.find-case { display: flex; align-items: center; gap: 4px; color: var(--text-dim); white-space: nowrap; }

/* Code */
.code {
  flex: 1; width: 100%; min-height: 120px; resize: none; border: 0; outline: 0; padding: 14px 15px;
  background: var(--bg); color: var(--text); caret-color: var(--accent);
  font-family: var(--font-mono); font-size: 12.5px; line-height: 1.6;
  font-weight: 400; font-style: normal; font-kerning: none; font-variant-ligatures: none; letter-spacing: 0; word-spacing: 0;
  tab-size: 2; white-space: pre; overflow: auto;
}
.code-editor { position: relative; display: flex; flex: 1; min-height: 120px; overflow: hidden; background: var(--bg); }
.code-content { position: relative; isolation: isolate; flex: 1; min-width: 0; }
.code-gutter { flex: 0 0 auto; width: calc(var(--line-number-digits) * 1ch + 38px); overflow: hidden; color: var(--text-dim); opacity: .7; user-select: none; font: 12.5px/1.6 var(--font-mono); }
.code-gutter pre { margin: 0; padding: 14px 10px; text-align: right; font: inherit; white-space: pre; }
.code-editor .code { position: absolute; inset: 0; height: 100%; margin: 0; box-sizing: border-box; }
.code-highlight { z-index: 0; pointer-events: none; overflow: hidden; }
.highlight-content { display: block; width: max-content; min-width: 100%; transform-origin: top left; }
.code-input { z-index: 2; background: transparent; color: transparent; -webkit-text-fill-color: transparent; }
.code-input::selection { background: color-mix(in srgb, var(--accent) 35%, transparent); }
.code-highlight :deep(.syntax-comment) { color: color-mix(in srgb, var(--text-dim) 80%, var(--bg)); }
.code-highlight :deep(.syntax-keyword) { color: #7ab8f5; }
.code-highlight :deep(.syntax-string) { color: #9fd0a8; }
.code-highlight :deep(.syntax-number) { color: #9fd0a8; }
.code-highlight :deep(.syntax-function) { color: var(--accent); }
.code-highlight :deep(.syntax-property) { color: var(--warning); }
.code-highlight :deep(.syntax-operator) { color: var(--text-dim); }
[data-theme="light"] .code-highlight :deep(.syntax-keyword) { color: #1f6fc2; }
[data-theme="light"] .code-highlight :deep(.syntax-string), [data-theme="light"] .code-highlight :deep(.syntax-number) { color: #2f7d3f; }
@media (forced-colors: active) {
  .code-highlight { display: none; }
  .code-input { color: CanvasText; -webkit-text-fill-color: CanvasText; }
}

/* Messages and stats */
.message { margin: 8px 10px 0; padding: 8px 10px; border-radius: var(--radius); font: 12px/1.45 var(--font-mono); white-space: pre-wrap; overflow-wrap: anywhere; }
.diagnostic-link { all: unset; cursor: pointer; text-decoration: underline; text-underline-offset: 2px; }
.diagnostic-link:focus-visible { outline: 2px solid currentColor; outline-offset: 2px; border-radius: 2px; }
.error { color: var(--danger); background: color-mix(in srgb, var(--danger) 12%, transparent); border: 1px solid color-mix(in srgb, var(--danger) 35%, transparent); }
.warning { max-height: 88px; overflow: auto; color: var(--warning); background: color-mix(in srgb, var(--warning) 10%, transparent); border: 1px solid color-mix(in srgb, var(--warning) 28%, transparent); }
.stats { display: flex; flex-wrap: wrap; align-items: center; gap: 5px 12px; min-height: 34px; padding: 5px 12px; border-top: 1px solid var(--border); color: var(--text-dim); font-size: 12px; }
.stats strong { color: var(--text); font-weight: 600; font-variant-numeric: tabular-nums; }
.status { margin-left: auto; display: flex; align-items: center; gap: 6px; color: #9fd0a8; }
.status::before { content: ''; width: 7px; height: 7px; border-radius: 50%; background: currentColor; }
.status.busy { color: var(--accent); }
.status.stale { color: var(--warning); }
.status.failed { color: var(--danger); }
[data-theme="light"] .status { color: #2f7d3f; }

/* Splitter */
.splitter {
  position: relative; z-index: 4; width: 6px; flex: 0 0 6px; cursor: col-resize;
  background: var(--surface); touch-action: none;
}
.splitter::before { content: ''; position: absolute; inset: 0 -9px; }
.splitter span { position: absolute; width: 2px; height: 34px; inset: 50% auto auto 50%; transform: translate(-50%, -50%); border-radius: 2px; background: var(--border); }
.splitter:hover span, .splitter:focus-visible span { background: var(--accent); }

/* Viewport */
.canvas-panel { flex: 1; min-width: 0; position: relative; overflow: hidden; background: var(--canvas-bg); }
.gpu-canvas { width: 100%; height: 100%; display: block; touch-action: none; outline: 0; }
.viewer-toolbar {
  position: absolute; z-index: 3; top: 12px; left: 50%; transform: translateX(-50%);
  display: flex; align-items: center; gap: 2px; max-width: calc(100% - 24px); padding: 4px;
  border: 1px solid var(--border); border-radius: 12px;
  background: color-mix(in srgb, var(--surface) 86%, transparent); backdrop-filter: blur(6px); color: var(--text);
}
.viewer-toolbar.with-dock { left: calc((100% - 344px) / 2); max-width: calc(100% - 368px); }
.view-btn { min-height: 32px; padding: 0 11px; border-color: transparent; color: var(--text-dim); font-size: 13px; font-weight: 500; }
.view-btn:hover { color: var(--text); }
.view-btn[aria-pressed="true"], .view-btn.active { background: var(--surface-raised); color: var(--text); }
.view-btn.icon-only { min-width: 32px; padding-inline: 0; }
.scan-toggle { display: flex; align-items: center; gap: 5px; white-space: nowrap; }
.scan-active-dot { width: 5px; height: 5px; border-radius: 50%; background: var(--accent); }
.view-select-label { display: flex; align-items: center; }
.view-select { max-width: 110px; }
.display-modes { display: flex; gap: 2px; }
.view-cube-wrap { position: absolute; z-index: 2; top: 60px; right: 14px; }
.view-cube-wrap.with-dock { right: 358px; }
.selection-modes {
  position: absolute; z-index: 3; top: 60px; left: 14px; display: flex; flex-direction: column; gap: 2px; padding: 3px;
  border: 1px solid var(--border); border-radius: 9px; background: color-mix(in srgb, var(--surface) 86%, transparent);
  color: var(--text); backdrop-filter: blur(5px);
}
.selection-modes button {
  min-height: 34px; min-width: 34px; display: flex; align-items: center; justify-content: center; gap: 6px; padding: 0 8px; border: 0;
  border-radius: 7px; background: transparent; color: var(--text-dim); cursor: pointer; font-size: 12px;
}
.selection-modes button > span:not(.mode-point, .mode-face, .mode-body) { display: none; }
.selection-modes button:hover { background: var(--hover); color: var(--text); }
.selection-modes button.active { background: var(--surface-raised); color: var(--text); }
.selection-modes kbd { display: none; }
.mode-point { width: 7px; height: 7px; border-radius: 50%; background: currentColor; }
.mode-face { width: 10px; height: 10px; border: 1.5px solid currentColor; background: color-mix(in srgb, currentColor 25%, transparent); transform: skewY(-18deg); }
.mode-body { width: 10px; height: 10px; border: 1.5px solid currentColor; box-shadow: inset 2px -2px color-mix(in srgb, currentColor 30%, transparent); }

/* Dock */
.cad-dock {
  position: absolute; z-index: 4; top: 0; right: 0; bottom: 0; width: min(344px, calc(100% - 16px));
  min-height: 0; display: flex; background: var(--bg); border-left: 1px solid var(--border);
}
.dock-rail {
  flex: 0 0 46px; display: flex; flex-direction: column; align-items: center; gap: 4px; padding: 8px 0;
  border-right: 1px solid var(--border);
}
.dock-rail button, .dock-close {
  width: 34px; height: 34px; border: 0; border-radius: var(--radius); background: transparent; color: var(--text-dim); cursor: pointer;
  display: flex; align-items: center; justify-content: center;
}
.dock-rail button:hover, .dock-close:hover { color: var(--text); background: var(--hover); }
.dock-rail button.active { color: var(--text); background: var(--surface-raised); }
.dock-rail-spacer { flex: 1; }
.dock-body { flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; }
.dock-panel { min-height: 0; display: flex; flex: 1; flex-direction: column; overflow: hidden; }
.dock-panel > :deep(.outliner), .dock-panel > :deep(.inspect-panel) { width: 100%; min-height: 0; flex: 1; border-radius: 0; border-width: 0; }
.customizer-card { min-height: 0; flex: 1; overflow: auto; background: var(--bg); }
.dock-scroll { min-height: 0; flex: 1; overflow: auto; overscroll-behavior: contain; }
.dock-scroll :deep(details.svg-panel), .dock-scroll :deep(details.photo-panel), .dock-scroll > .performance-panel { border: 0; border-radius: 0; background: transparent; }
.dock-scroll :deep(details.svg-panel > summary), .dock-scroll :deep(details.photo-panel > summary), .dock-scroll > .performance-panel > summary { display: none; }
.dock-scroll :deep(.svg-content) { max-height: none; padding: 12px; }
.dock-scroll :deep(.photo-content) { padding: 12px; }
.dock-scroll > .performance-panel { border-top: 0; }
.dock-scroll > .performance-panel .performance-content { max-height: none; padding: 12px; }
.canvas-panel :deep(.main-model-tools) {
  left: 14px; right: auto; bottom: 34px; max-width: calc(100% - 28px); padding: 6px 8px; border-radius: 12px;
  background: color-mix(in srgb, var(--surface) 90%, transparent); backdrop-filter: blur(6px);
}
.canvas-panel.with-dock :deep(.main-model-tools) { max-width: calc(100% - 372px); }


/* HUD and badges */
.selection-hud {
  position: absolute; z-index: 3; top: 12px; left: 14px; display: flex; align-items: center; gap: 8px;
  min-height: 30px; padding: 0 6px 0 10px; border: 1px solid var(--border);
  border-radius: var(--radius); background: color-mix(in srgb, var(--surface) 86%, transparent); color: var(--text);
  backdrop-filter: blur(5px); font-size: 12.5px;
}
.selection-hud > span { display: flex; align-items: center; gap: 6px; }
.selection-dot { width: 8px; height: 8px; border-radius: 2px; background: var(--accent); }
.selection-hud button {
  min-width: 24px; height: 22px; padding: 0 6px; border: 1px solid var(--border);
  border-radius: 5px; background: transparent; color: var(--text-dim); cursor: pointer;
}
.selection-hud button:hover, .selection-hud button[aria-pressed="true"] { background: var(--surface-raised); color: var(--text); }
.rendering-badge, .stale-badge {
  position: absolute; z-index: 2; top: 58px; left: 50%; transform: translateX(-50%);
  display: flex; align-items: center; gap: 7px; padding: 6px 12px; border-radius: 999px;
  background: color-mix(in srgb, var(--surface) 86%, transparent); color: var(--text); border: 1px solid var(--border);
  backdrop-filter: blur(5px); font-size: 12px; pointer-events: none;
}
.stale-badge { color: var(--warning); }
.spinner { width: 11px; height: 11px; border: 2px solid var(--border); border-top-color: var(--accent); border-radius: 50%; animation: spin .7s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.canvas-hint {
  position: absolute; z-index: 2; bottom: 10px; right: 14px;
  max-width: calc(100% - 28px); color: var(--text-dim); font-size: 11.5px; text-align: right; pointer-events: none;
}
.canvas-hint.with-dock { right: 358px; }

/* Status bar */
.statusbar {
  height: 28px; flex-shrink: 0; display: flex; align-items: center; gap: 16px; padding: 0 14px;
  background: var(--surface); border-top: 1px solid var(--border); color: var(--text-dim); font-size: 11.5px; white-space: nowrap; overflow: hidden;
}
.status-item { display: inline-flex; align-items: center; gap: 6px; }
.status-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--danger); }
.status-dot.ok { background: #9fd0a8; }
[data-theme="light"] .status-dot.ok { background: #2f7d3f; }
.status-hint kbd { font-size: 10.5px; padding: 0 4px; }

/* Dialogs */
.dialog-backdrop { position: fixed; z-index: 40; inset: 0; display: flex; align-items: center; justify-content: center; padding: 20px; background: rgba(0,0,0,.45); }
.dialog {
  width: min(620px, 100%); max-height: 100%; overflow: auto; display: flex; flex-direction: column;
  background: var(--surface); border: 1px solid var(--border); border-radius: 14px; box-shadow: 0 24px 64px rgba(0,0,0,.4);
}
.dialog-header { display: flex; align-items: flex-start; gap: 10px; padding: 18px 20px 14px; border-bottom: 1px solid var(--border); }
.dialog-header > div { flex: 1; }
.dialog-header h2 { margin: 0; font-size: 17px; font-weight: 600; letter-spacing: -0.01em; }
.dialog-header p { margin: 2px 0 0; color: var(--text-dim); }
.format-grid { margin: 0; padding: 18px 20px 8px; border: 0; display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px; }
.format-grid legend { padding: 0; margin-bottom: 8px; color: var(--text-dim); font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: .06em; }
.format-card {
  display: flex; flex-direction: column; gap: 3px; padding: 10px 12px; border-radius: 10px; border: 1px solid var(--border);
  background: var(--bg); cursor: pointer;
}
.format-card.active { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 14%, var(--bg)); }
.format-card input { position: absolute; opacity: 0; width: 1px; height: 1px; }
.format-name { font-weight: 600; }
.format-card small { color: var(--text-dim); font-size: 11.5px; }
.dialog-note { margin: 6px 20px 14px; padding: 10px 12px; border-radius: var(--radius); background: var(--bg); border: 1px solid var(--border); color: var(--text-dim); font-size: 12.5px; }
.dialog-note.warning { color: var(--warning); }
.dialog-footer { display: flex; align-items: center; gap: 8px; padding: 14px 20px; border-top: 1px solid var(--border); background: var(--bg); border-radius: 0 0 14px 14px; }
.toast {
  position: fixed; z-index: 50; left: 50%; bottom: 44px; transform: translateX(-50%);
  padding: 8px 13px; border: 1px solid var(--border); border-radius: var(--radius);
  background: var(--surface-raised); box-shadow: 0 8px 32px rgba(0,0,0,.28); font-size: 13px;
}

@media (max-width: 1360px) {
  .topbar-action.share-action span { display: none; }
}

@media (max-width: 1100px) {
  .command-btn { min-width: 0; }
  .command-btn span, .command-btn kbd { display: none; }
  .topbar-action span { display: none; }
  .theme-picker { display: none; }
}

@media (max-width: 800px) {
  .topbar { grid-template-columns: 1fr auto; row-gap: 0; height: auto; min-height: 44px; padding-block: 6px; }
  .mode-switch { grid-column: 1 / -1; justify-self: stretch; }
  .mode-switch button { flex: 1; }
  .main { flex-direction: column; overflow: auto; }
  .editor-panel { width: 100% !important; min-width: 0; max-width: none; height: 46dvh; flex: 0 0 46dvh; border-right: 0; }
  .splitter { display: none; }
  .canvas-panel { min-height: 46dvh; flex: 1 0 46dvh; border-top: 1px solid var(--border); }
  .viewer-toolbar, .viewer-toolbar.with-dock { left: 8px; right: 8px; max-width: none; transform: none; justify-content: center; flex-wrap: wrap; }
  .view-cube-wrap { top: 60px; right: 8px; }
  .view-cube-wrap.with-dock { display: none; }
  .selection-modes { top: 60px; left: 8px; }
  .selection-hud { top: 12px; left: 8px; }
  .cad-dock { width: min(344px, calc(100% - 8px)); }
  .canvas-panel :deep(.main-model-tools), .canvas-panel.with-dock :deep(.main-model-tools) { left: 8px; right: 8px; max-width: none; }
  .canvas-hint, .canvas-hint.with-dock { right: 8px; }
  .stats { font-size: 11.5px; }
  .status { width: 100%; margin-left: 0; }
  .statusbar { gap: 10px; }
  .status-hint { display: none; }
  .format-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}

@media (max-width: 480px) {
  .brand { display: none; }
  .file-chip { max-width: 160px; }
  .select { max-width: 128px; }
  .view-btn { padding-inline: 6px; }
  .view-select { max-width: 78px; }
  .view-cube-wrap { display: none; }
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
