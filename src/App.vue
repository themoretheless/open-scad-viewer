<script setup lang="ts">
import {useTransientNotice} from './composables/useTransientNotice'
import SweepBoundaryBadges from './components/SweepBoundaryBadges.vue'
import { useBuildAndRendererLifecycle, type SweepViewportProgressSnapshot } from './composables/useBuildAndRendererLifecycle'
import { useCodeEditorViewport } from './composables/useCodeEditorViewport'
import { useEditorFormatting } from './composables/useEditorFormatting'
import { useEditorResize } from './composables/useEditorResize'
import { useParameterPresets } from './composables/useParameterPresets'
import { useSectionPlaneControls } from './composables/useSectionPlaneControls'
import { useSourceProvenanceCaret } from './composables/useSourceProvenanceCaret'
import { useViewportDisplayControls } from './composables/useViewportDisplayControls'
import { useWorkspaceFileActions } from './composables/useWorkspaceFileActions'
import { useWorkspacePersistenceLifecycle } from './composables/useWorkspacePersistenceLifecycle'
import VrControls from './components/VrControls.vue'
import ModelingGridControls from './components/ModelingGridControls.vue'
import MaterialControls from './features/MaterialControls.vue'
import { clamp } from './services/math3d'
import { stringifyMeshJson } from './services/meshJson'
import { isRushFrontend, SOURCE_FILE_ACCEPT, SOURCE_FILE_EXTENSION, sourceFileExtension } from './services/rushFrontendDetect'
import {readSweepViewportEvidence,readSweepPatchViewportEvidence,readSweepBodyBoundaryViewportEvidence} from './services/sweepViewportEvidence'
import { highlightCode } from './services/codeHighlight'
import { computed, defineAsyncComponent, nextTick, onMounted, onUnmounted, ref, shallowRef, watch, watchEffect } from 'vue'
import { MESH_EXPORT_FORMATS, MESH_FORMAT_LABELS, MESH_IMPORT_ACCEPT } from './services/meshFormats'
import CommandPalette from './components/CommandPalette.vue'
import CustomizerPanel from './components/CustomizerPanel.vue'
const ExampleGallery = defineAsyncComponent(() => import('./components/ExampleGallery.vue'))
const MechanicalGenerator = defineAsyncComponent(() => import('./features/MechanicalGenerator.vue'))
import {restoreSourceHistory,boundedSourceHistory} from './services/mainSourceEditing'
import type { DirectDocument } from './services/directModeling'
import { APP_STRINGS as L, type Language } from './i18n/appStrings'
import { useAppearance } from './composables/useAppearance'
import { useEditorFind } from './composables/useEditorFind'
import { emptyDirectDocument } from './services/directModeling'
import type { DirectBody } from './services/directModeling'
import type { WorkspaceMode } from './services/workspaceModes'
import { WORKSPACE_MODES, workspaceModeHint, workspaceModeLabel } from './services/workspaceModes'
import { sceneMeshesToSolidDocument, meshDocumentToSolidDocument, meshDataToPolygon, meshDataToEditablePolygon, polygonToMeshObject } from './services/solidBridge'
import { emptyMeshDocument, type MeshWorkspaceDocument } from './services/meshEditing'
const DirectModeler = defineAsyncComponent(() => import('./features/DirectModeler.vue'))
const MeshModeler = defineAsyncComponent(() => import('./features/MeshModeler.vue'))
const MainModelingTools = defineAsyncComponent(() => import('./features/MainModelingTools.vue'))
const ScanPlanePanel = defineAsyncComponent(() => import('./features/ScanPlanePanel.vue'))
const SvgPanel = defineAsyncComponent(() => import('./features/SvgPanel.vue'))
const PhotogrammetryPanel = defineAsyncComponent(() => import('./features/PhotogrammetryPanel.vue'))
const InspectPanel = defineAsyncComponent(() => import('./components/InspectPanel.vue'))
const SceneOutliner = defineAsyncComponent(() => import('./components/SceneOutliner.vue'))
const KeyboardShortcuts = defineAsyncComponent(() => import('./components/KeyboardShortcuts.vue'))
import ViewCube from './components/ViewCube.vue'
import type { GeometryQuality } from './core/build'
import { type MeshData } from './core/mesh'
import { MAX_PARAMETER_PRESETS } from './services/parameterPresets'
import { analyzeSurfaceArea, type GeometryComputeResult } from './services/geometryCompute'
import { EXAMPLE_CATALOG, EXAMPLES } from './data/examples'
import { assertExampleCatalog } from './services/exampleCatalog'
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
import { nextRovingIndex } from './services/rovingFocus'
import { SceneController } from './services/sceneController'
import { ViewportController } from './services/viewportController'
import { revealDiagnostic, type EditorDiagnostic } from './services/editorDiagnostics'
import {
  setStorageFailureHandler,
  storageGet,
  storageSet,
  storageSetJSON,
} from './services/safeStorage'
import { extractCustomizerParameters } from './services/scadCustomizer'
import {
  geometryExportEligibility,
} from './services/buildPromotionPolicy'
import {
  MAX_WORKSPACE_SOURCE_LENGTH,
  updateWorkspaceDocument,
  type WorkspaceDocumentSnapshot,
} from './services/workspaceDocument'
import type { BrowserWorkspacePersistence } from './services/workspacePersistence'
import { assertThemeCatalog, THEME_CATALOG } from './services/themePreferences'
import type {
  PickHit,
  SelectionMode,
  StandardView,
} from './services/rendererContracts'
import { blockColors, MAIN_COMMAND_PREFIX, dockTabs, DOCK_ICONS, revealDockDetails, DOCK_KEY_MAP, closeMenus, closeMenusOutside, readCommandMru } from './appHelpers'
import type { DockTab } from './appHelpers'

assertExampleCatalog(EXAMPLE_CATALOG)
assertThemeCatalog(THEME_CATALOG)

const props = defineProps<{
  initialWorkspace: WorkspaceDocumentSnapshot
  initialWorkspaceDurable: boolean
  initialSharedImportPending: boolean
  workspacePersistence: BrowserWorkspacePersistence
}>()

const {
  lang, themeSelection, customAccent, customBase, effectiveTokens, isDark,
  setCustomColor, effectiveCanvasColor, resetCustomColors,
  toggleLang, toggleTheme, applyPreferences, attachSystemTheme, detachSystemTheme,
} = useAppearance({
  onApplied(canvasColor) {
    getRenderer()?.setBackgroundColor(canvasColor)
    // Re-apply the render theme so its backgroundColor (when set) stays authoritative.
    getRenderer()?.setTheme(renderThemeId.value)
  },
})
const workspaceDocument = ref<WorkspaceDocumentSnapshot>(props.initialWorkspace)
const code = ref(props.initialWorkspace.source)
const autoRender = ref(storageGet('scad-auto') !== 'false')
const editorWidth = ref(clamp(Number(storageGet('scad-editor-width')) || 440, 300, 820))

const canvasRef = ref<HTMLCanvasElement | null>(null)
const editorRef = ref<HTMLTextAreaElement | null>(null)
const highlightRef = ref<HTMLPreElement | null>(null)
const selectedEditorName = ref('')
const {
  foldedLines,
  highlightedCode,
  foldedRows,
  lineNumbersRef,
  guidesRef,
  editorLineCount,
  guideRowPx,
  visibleGutterRows,
  visibleGuideRows,
  gutterLinesHeight,
  toggleFold,
  editFoldedLine,
  syncHighlightScroll,
} = useCodeEditorViewport({
  code,
  selectedEditorName,
  editorRef,
  highlightRef,
})
watch(code, () => { foldedLines.value = new Set(); void nextTick(syncHighlightScroll) }, { flush: 'post' })
const findInputRef = ref<HTMLInputElement | null>(null)
const mainRef = ref<HTMLElement | null>(null)
const error = ref('')
const editorDiagnostic = ref<EditorDiagnostic | null>(null)
const warnings = ref<string[]>([])
const meshCount = ref(0)
const triangleCount = ref(0)
const volume = ref(0)
const surfaceArea = ref(0)
const renderDuration = ref(0)
const gpuOk = ref(false)
const rendererInitializing = ref(false)
const rendererUnavailableMessage = ref('')
const currentBackendQuality = computed(() => backendQuality(gpuOk.value, displayMode.value))
const rendering = ref(false)
const renderingQuality = ref<GeometryQuality>('full')
const renderedQuality = ref<GeometryQuality>('full')
const renderedSource = ref('')
const notice = ref('')
const showNotice=useTransientNotice(notice)
const fileName = ref(props.initialWorkspace.fileName)
const exampleGalleryOpen = ref(false)
let componentActive = true
const t = (key: string) => L[lang.value][key] ?? key
const formatNumber = (value: number, digits = 0) => value.toLocaleString(lang.value, { maximumFractionDigits: digits })

const {
  workspacePersistenceStatus,
  workspaceConflict,
  savingBrowser,
  storageFailureDetail,
  persistenceLabel,
  bumpWorkspaceEditGeneration,
  isRestoringWorkspace,
  isSharedImportPending,
  applySourceChange,
  applyFileNameChange,
  markDocumentReplaced,
  scheduleWorkspacePersistence,
  flushWorkspacePersistence,
  handleVisibilityChange,
  retryWorkspacePersistence,
  saveBrowserDraft,
  resolveWorkspaceConflict,
  reportStorageFailure,
  updateBeforeUnloadGuard,
  detachBeforeUnloadGuard,
   disposeWorkspacePersistenceTimer,
} = useWorkspacePersistenceLifecycle({
  initialWorkspaceDurable: props.initialWorkspaceDurable,
  initialSharedImportPending: props.initialSharedImportPending,
  workspacePersistence: props.workspacePersistence,
  workspaceDocument,
  code,
  fileName,
  lang,
  exampleGalleryOpen,
  getRenderer: () => getRenderer(),
  isComponentActive: () => componentActive,
  markFitNextRender: () => markFitNextRender(),
  showNotice,
  translate: t,
})
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
    const polygon = meshDataToEditablePolygon(mesh)
    if (polygon) doc.objects.push(polygonToMeshObject(polygon, `${prefix} ${index + 1}`, `scene-${index + 1}-${Date.now().toString(36)}`))
  })
  return doc
}

const solidGpuActive=ref(false)
const workspaceMode = computed<WorkspaceMode>(() => (meshModelerOpen.value ? 'mesh' : 'solid'))

/** Source is no longer a workspace of its own; it opens as a drawer over either one. */
const editorOpen = ref(false)
// The source viewport is visible only when the source drawer previews a sweep.
// Keep its GPU resources separate from the active Solid/Mesh viewport.
const sourceViewportVisible = computed(() =>
  (!directModelerOpen.value && !meshModelerOpen.value && !functionReferenceOpen.value) ||
  (editorOpen.value && isRushFrontend(code.value) &&
    (code.value.includes('progressive_sweep') || code.value.includes('miter_sweep'))))
watch(sourceViewportVisible, async visible => {
  if (!visible) {
    destroySourceViewportRenderer()
    return
  }
  await nextTick()
  const activeRenderer = getRenderer()
  if (!activeRenderer) await initializeViewportRenderer()
  else activeRenderer.resize()
})
const solidBuilding = ref(false)
let solidBuildAbort: AbortController | null = null
function cancelSolidBuild() { solidBuildAbort?.abort() }
const solidAppendBodies = ref<{ bodies: DirectBody[]; token: number; group?: { name: string; source: string; replaces: string | null } } | null>(null)
/** Set while the left panel edits one scene group's source instead of the document. */
const groupEdit = ref<{ name: string; source: string; replaces: string | null } | null>(null)
// A completed worker must not publish a superseded source snapshot.
watch([code, fileName, () => groupEdit.value?.source], cancelSolidBuild, { flush: 'sync' })
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
  storageSet('scad-solid-modeler-v1', stringifyMeshJson(solidSeedDocument.value))
  meshModelerOpen.value = false
  directModelerOpen.value = true
}

function continueMainEditInSolid(document: DirectDocument) {
  solidSeedDocument.value = document
  storageSet('scad-solid-modeler-v1', stringifyMeshJson(document))
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
const mainProject=(point:readonly number[])=>getRenderer()?.projectWorldPoint(point)??null
const mainRay=(x:number,y:number)=>getRenderer()?.worldRay(x,y)??null
const sweepViewportProgress=ref<SweepViewportProgressSnapshot|null>(null)
function commitMainSource(source:string, selectIndex=selectedMesh.value) {
 try {
  if(source.length>MAX_WORKSPACE_SOURCE_LENGTH)throw Error('source limit')
  previewMainGeometry(null)
  if(source===code.value)return
  if(mainEditPast.value.at(-1)?.after!==code.value)mainEditPast.value=[]
  mainEditPast.value.push({before:code.value,after:source});while(mainEditPast.value.length>30||(mainEditPast.value.length>1&&mainEditPast.value.reduce((n,e)=>n+e.before.length+e.after.length,0)>8_000_000))mainEditPast.value.shift();mainEditFuture.value=[]
  setMainEditSelection(selectIndex===null?null:{source,index:selectIndex})
  replacePresetSource(source);void nextTick(()=>doRender('full'))
 } catch(e){error.value=e instanceof Error?e.message:String(e)}
}
function appendMainPrimitive(source:string) {
 if(isRushFrontend(code.value)){error.value=lang.value==='ru'?'Примитивы доступны в документе OpenSCAD.':'Primitives require an OpenSCAD document.';return}
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
const sweepFinalEvidence=computed(()=>!rendering.value&&!stale.value&&renderedSource.value===code.value
 ? sceneMeshes.value.map(mesh=>readSweepViewportEvidence(mesh.nativeGeometry)).filter(evidence=>evidence!==null) : [])
const sweepPatchFinalEvidence=computed(()=>!rendering.value&&!stale.value&&renderedSource.value===code.value
 ? sceneMeshes.value.map(mesh=>readSweepPatchViewportEvidence(mesh.nativeGeometry)).filter(evidence=>evidence!==null) : [])
const sweepBodyFinalEvidence=computed(()=>!rendering.value&&!stale.value&&renderedSource.value===code.value
 ? sceneMeshes.value.map(mesh=>readSweepBodyBoundaryViewportEvidence(mesh.nativeGeometry)).filter(evidence=>evidence!==null) : [])
const sceneMeshes = computed({
  get: () => sceneState.value.meshes,
  set: (meshes: MeshData[]) => { sceneController.update({ meshes }) },
})
const meshVisibility = computed({
  get: () => sceneState.value.visibility,
  set: (visibility: boolean[]) => { sceneController.update({ visibility }) },
})
const measurement = computed(() => sceneState.value.measurement)
const measureActive = computed(() => sceneState.value.measureActive)
const dockTab = ref<DockTab>('scene')
const dockOpen = ref(true)
const {
  projection,
  gridVisible,
  gridStep,
  standardView,
  activeView,
  cameraYaw,
  cameraPitch,
  displayMode,
  shadingModel,
  matcapId,
  envId,
  shadowsEnabled,
  renderThemeId,
  defaultBaseColor,
  defaultMetallic,
  defaultRoughness,
  selectionMode,
  sceneRows,
  selectedSceneMeshId,
  hoveredMesh,
  currentInspection,
  panelMeasurement,
  fitView,
  resetView,
  previousView,
  syncCameraState,
  focusSelection,
  clearSelection,
  toggleIsolate,
  toggleProjection,
  toggleGrid,
  changeStandardView,
  setStandardView,
  setDisplayMode,
  setShadingModel,
  setRenderTheme,
  defaultMaterialState,
  setDefaultBaseColor,
  setDefaultMetallic,
  setDefaultRoughness,
  setMatcap,
  setEnv,
  setShadows,
  applyMaterialPreset,
  setSelectionMode,
  selectSceneMesh,
  setSceneMeshVisibility,
  preselectSceneMesh,
  focusSceneMesh,
  isolateSceneMesh,
  startMeasure,
  cancelMeasure,
  clearMeasurement,
  copyMeasurement,
} = useViewportDisplayControls({
  viewportController,
  viewportState,
  sceneController,
  sceneState,
  sceneMeshes,
  meshVisibility,
  selectedMesh,
  selectedHit,
  hoveredHit,
  isolated,
  measurement,
  measureActive,
  renderedSource,
  code,
  surfaceArea,
  volume,
  mainCameraRevision,
  dockOpen,
  dockTab,
  getRenderer: () => getRenderer(),
  showNotice,
  translate: t,
})
watch(gridStep, step => getRenderer()?.setGridStep(step))
const {
  sectionEnabled,
  sectionAxis,
  sectionOffset,
  sectionFlip,
  scanPanelOpen,
  scanToggleRef,
  sectionRange,
  sectionAvailable,
  applySection,
  syncSectionRange,
  setSectionEnabled,
  setSectionAxis,
  setSectionOffset,
  setSectionFlip,
  resetSection,
  closeScanPanel,
} = useSectionPlaneControls({
  gpuOk,
  sceneMeshes,
  meshVisibility,
  isolated,
  selectedMesh,
  sceneState,
  sceneController,
  getRenderer: () => getRenderer(),
})
// Direct-modeling actions of the Code viewport are reached through the command palette.
const mainToolsRef = ref<InstanceType<typeof MainModelingTools> | null>(null)
const visiblePaletteCommands = computed<PaletteCommand[]>(() => {
  const base = paletteCommands.value.filter(isPaletteCommandEnabled)
  // The source tools only make sense while the editor drawer is showing.
  if (!editorOpen.value) return base
  const tools = (mainToolsRef.value?.commands ?? []) as readonly PaletteCommand[]
  return [...base, ...tools.filter(command => command.enabled !== false).map(command => ({ ...command, id: MAIN_COMMAND_PREFIX + command.id }))]
})
const {
  findOpen, replaceOpen, findQuery, findReplacement, findCaseSensitive, activeFindIndex, findResult, findStatus,
  openEditorFind, closeEditorFind, navigateFind, handleFindKeydown,
  replaceCurrentFindMatch, replaceAllFindMatches,
} = useEditorFind({
  code, editor: editorRef, input: findInputRef,
  translate: key => t(key), notify: message => showNotice(message), onEdited: () => handleEditorInput(),
})
const dockToggleRef = ref<HTMLButtonElement | null>(null)
const editorMaxWidth = ref(820)
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

// The rush compiler pulls the geometry kernel chunk; load it only when a
// rush-frontend document is actually open.
const compactControls = ref<{parameters: import('./services/scadCustomizer').CustomizerParameter[]; errors: string[]}>({parameters: [], errors: []})
watchEffect(async () => {
  const source = code.value
  if (!isRushFrontend(source)) {
    compactControls.value = {parameters: [], errors: []}
    return
  }
  const controls = (await import('./services/rushFrontend')).rushFrontendControls(source)
  if (code.value === source) compactControls.value = controls
})
const customizerParameters = computed(() => {
  if (!isRushFrontend(code.value)) return extractCustomizerParameters(code.value)
  return compactControls.value.parameters
})
const {
  presetName,
  presetSelection,
  presetError,
  presetUndo,
  parameterPresets,
  selectedPreset,
  presetCompatible,
  saveParameterPreset,
  replacePresetSource,
  restoreParameterPreset,
  undoParameterPreset,
  removeParameterPreset,
  updateCustomizer,
} = useParameterPresets({
  code,
  editorRef,
  workspaceDocument,
  lang,
  customizerParameters,
  bumpWorkspaceEditGeneration,
  scheduleWorkspacePersistence,
})

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
const {
  syncSourceHighlightFromEditor,
  highlightSource,
  revealSource,
} = useSourceProvenanceCaret({
  code,
  editorRef,
  selectedEditorName,
  sceneMeshes,
  sourceMatchesEditor,
  getRenderer: () => getRenderer(),
  showNotice,
  translate: t,
})
const {
  getRenderer,
  performanceSamples,
  performanceLast,
  performanceMs,
  performanceRows,
  buildCounters,
  markFitNextRender,
  advanceBuildGeneration,
  setMainEditSelection,
  mainSelectMany,
  previewMainGeometry,
  clearSweepViewportPreview,
  scheduleRender,
  cancelAutoBuild,
  beginParameterGesture,
  endParameterGesture,
  commitParameterControl,
  initializeViewportRenderer,
  destroySourceViewportRenderer,
  disposeRendererAndBuildCoordinator,
  startBuildCoordinator,
  doRender,
  cancelGeometryBuild,
} = useBuildAndRendererLifecycle({
  canvasRef,
  code,
  lang,
  autoRender,
  sourceViewportVisible,
  viewportController,
  sceneController,
  sceneMeshes,
  meshVisibility,
  selectedMesh,
  selectedHit,
  isolated,
  measurement,
  measureActive,
  displayMode,
  shadingModel,
  matcapId,
  envId,
  shadowsEnabled,
  renderThemeId,
  selectionMode,
  gridVisible,
  gridStep,
  error,
  editorDiagnostic,
  warnings,
  meshCount,
  triangleCount,
  volume,
  surfaceArea,
  renderDuration,
  rendering,
  renderingQuality,
  renderedQuality,
  renderedSource,
  gpuOk,
  rendererInitializing,
  rendererUnavailableMessage,
  mainSelectedIndices,
  mainShiftSelection,
  sweepViewportProgress,
  defaultMaterialState,
  effectiveCanvasColor,
  applySection,
  syncSectionRange,
  syncSourceHighlightFromEditor,
  syncCameraState,
  isComponentActive: () => componentActive,
  showNotice,
  translate: t,
})
watch(code, clearSweepViewportPreview, { flush: 'sync' })
const {
  fileInputRef,
  convertInputRef,
  exportDialogOpen,
  exportDialogRef,
  additionalExportFormat,
  openExportDialog,
  confirmExport,
  triggerOpen,
  openSelectedFile,
  handleDrop,
  triggerConvert,
  convertSelectedFile,
  saveSource,
  exportStl,
  exportPerformance,
  exportObj,
  shareSource,
} = useWorkspaceFileActions({
  code,
  fileName,
  error,
  canExport,
  sceneMeshes,
  buildCounters,
  performanceSamples,
  markFitNextRender,
  showNotice,
  translate: t,
})
const statusText = computed(() => rendering.value
  ? t('compiling')
  : error.value ? t('failed') : stale.value ? t('stale') : t('ready'))
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

let applyingWorkspaceReplacement = false
const topbarRef = ref<HTMLElement | null>(null)
const topbarHeight = ref(52)
let topbarResizeObserver: ResizeObserver | null = null
let layoutResizeObserver: ResizeObserver | null = null
let editorResizeObserver: ResizeObserver | null = null

onMounted(async () => {
  if (topbarRef.value) {
    const syncTopbarHeight = () => { topbarHeight.value = topbarRef.value!.getBoundingClientRect().height }
    syncTopbarHeight()
    topbarResizeObserver = new ResizeObserver(syncTopbarHeight)
    topbarResizeObserver.observe(topbarRef.value)
  }
  setStorageFailureHandler(reportStorageFailure)
  attachSystemTheme()
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
  guideRowPx.value = 1.58 * 0.82 * (parseFloat(getComputedStyle(document.documentElement).fontSize) || 16)
  if (editorRef.value) {
    editorResizeObserver = new ResizeObserver(syncHighlightScroll)
    editorResizeObserver.observe(editorRef.value)
  }
  window.addEventListener('keydown', handleGlobalKey)
  try {
    startBuildCoordinator()
    doRender('full')
  } catch {
    error.value = t('workerError')
  }
  if (sourceViewportVisible.value) await initializeViewportRenderer()
})

onUnmounted(() => {
  topbarResizeObserver?.disconnect()
  topbarResizeObserver = null
  solidBuildAbort?.abort()
  cancelGeometryAnalysis()
  layoutResizeObserver?.disconnect()
  layoutResizeObserver = null
  editorResizeObserver?.disconnect()
  editorResizeObserver = null
  disposeWorkspacePersistenceTimer()
  componentActive = false
  setStorageFailureHandler(null)
  disposeRendererAndBuildCoordinator()
  window.removeEventListener('keydown', handleGlobalKey)
  detachSystemTheme()
  window.removeEventListener('pagehide', flushWorkspacePersistence)
  window.removeEventListener('pointerup', endParameterGesture)
  window.removeEventListener('pointercancel', endParameterGesture)
  window.removeEventListener('blur', endParameterGesture)
  detachBeforeUnloadGuard()
  document.removeEventListener('visibilitychange', handleVisibilityChange)
  void props.workspacePersistence.close()
})

watch(code, value => {
  if (applyingWorkspaceReplacement) return
  editorDiagnostic.value = null
  // Persistence revisions are local to one WorkspaceDocument and can move
  // backwards when another document is restored. Geometry jobs instead use a
  // process-local monotonic generation, so old results can never alias a new
  // document that happens to have the same revision number.
  advanceBuildGeneration(performance.now())
  // Provenance belongs to the last compiled source revision. Never retain a
  // reverse highlight while the editor has moved ahead of that revision.
  if (renderedSource.value !== value) getRenderer()?.setSourceHighlight(null)
  // A #code= hash left by shareSource() goes stale when code diverges. An
  // imported hash that is still the only durable copy is retained until the
  // replacement draft reaches IDB or the synchronous recovery journal.
  if (!isSharedImportPending() && location.hash.startsWith('#code=')) {
    history.replaceState(null, '', location.pathname + location.search)
  }
  if (isRestoringWorkspace()) {
    if (autoRender.value) scheduleRender(0)
    return
  }
  if (!applySourceChange(value)) return
  if (autoRender.value) scheduleRender()
})

watch(fileName, value => {
  if (applyingWorkspaceReplacement) return
  if (isRestoringWorkspace()) return
  applyFileNameChange(value)
})

watch(autoRender, enabled => {
  storageSet('scad-auto', String(enabled))
  if (enabled) scheduleRender(0)
  else cancelAutoBuild()
})

async function loadEditorDocument(example: string, nextFileName: string) {
  applyingWorkspaceReplacement = true
  advanceBuildGeneration()
  editorDiagnostic.value = null
  getRenderer()?.setSourceHighlight(null)
  if (!isSharedImportPending() && location.hash.startsWith('#code=')) {
    history.replaceState(null, '', location.pathname + location.search)
  }
  workspaceDocument.value = updateWorkspaceDocument(workspaceDocument.value, {
    source: example,
    fileName: nextFileName,
  })
  const editor = editorRef.value
  if (editor) {
    editor.setRangeText(example, 0, editor.value.length, 'select')
    code.value = editor.value
  } else {
    code.value = example
  }
  fileName.value = nextFileName
  markDocumentReplaced()
  if (autoRender.value) scheduleRender()
  markFitNextRender()
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

function handleEditorInput() {
  editorDiagnostic.value = null
  // The v-model update precedes this handler, so this also clears stale
  // geometry immediately instead of waiting for the next Worker response.
  syncSourceHighlightFromEditor()
}

function revealCurrentDiagnostic() {
  if (editorRef.value && editorDiagnostic.value) revealDiagnostic(editorRef.value, editorDiagnostic.value)
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
    case 'shading-phong': setShadingModel('phong'); break
    case 'shading-pbr': setShadingModel('pbr'); break
    case 'shading-matcap': setShadingModel('matcap'); break
    case 'shading-toon': setShadingModel('toon'); break
    case 'shading-unlit': setShadingModel('unlit'); break
    case 'render-theme-default': setRenderTheme('default'); break
    case 'render-theme-dark-contrast': setRenderTheme('dark-contrast'); break
    case 'render-theme-light': setRenderTheme('light'); break
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
    case 'command-palette':
      if (paletteWasOpen) paletteOpen.value = false
      else openCommandPalette()
      break
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

const { formatEditor, handleEditorKey } = useEditorFormatting({ code, editorRef, foldedLines, syncHighlightScroll, handleEditorInput, dispatchKeyboardCommand })

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

const { startResize, moveResize, stopResize, resizeEditorWithKeyboard } = useEditorResize({ mainRef, editorWidth, editorMaxWidth })



</script>

<template>
  <div class="app" :style="{ '--workspace-top': topbarHeight + 'px' }" @dragover.prevent @drop.prevent="handleDrop" @pointerdown.capture="closeMenusOutside">
    <nav ref="topbarRef" class="topbar" aria-label="Application" :inert="functionReferenceOpen">
      <div class="topbar-left">
        <svg class="logo" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linejoin="round" aria-hidden="true">
          <path d="M12 3 3 8v8l9 5 9-5V8z"/><path d="M3 8l9 5 9-5M12 13v8"/>
        </svg>
        <details class="menu file-menu">
          <summary class="file-chip" :title="t('fileMenu')">
            <span class="file-chip-name">{{ fileName }}</span>
            <span v-if="workspaceConflict || workspacePersistenceStatus !== 'saved'" class="file-chip-dot" :class="workspaceConflict ? 'error' : workspacePersistenceStatus" aria-hidden="true" />
            <svg class="chevron" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>
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
        <button
          class="icon-btn source-toggle"
          type="button"
          :class="{ active: editorOpen }"
          :aria-pressed="editorOpen"
          :aria-label="lang === 'ru' ? 'Исходный код' : 'Source'"
          :title="lang === 'ru' ? 'Исходный код' : 'Source'"
          @click="editorOpen = !editorOpen"
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M8 6 3 12l5 6M16 6l5 6-5 6"/></svg>
        </button>
        <button class="icon-btn command-btn" type="button" :title="t('commandHelp')" :aria-label="t('commands')" aria-keyshortcuts="Control+K Meta+K" @click="openCommandPalette">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>
          <span>{{ t('commandHelp') }}</span> <kbd>Ctrl K</kbd>
        </button>
        <button class="btn topbar-action" type="button" :title="t('exportTitle')" @click="openExportDialog">
          <span>{{ t('export') }}</span>
        </button>
        <details class="menu settings-menu">
          <summary class="icon-btn" :title="t('settings')" :aria-label="t('settings')">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><circle cx="12" cy="5" r="1.7"/><circle cx="12" cy="12" r="1.7"/><circle cx="12" cy="19" r="1.7"/></svg>
          </summary>
          <div class="menu-list menu-right">
            <div class="menu-row" @click.stop>
              <span>{{ t('theme') }}</span>
              <select v-model="themeSelection" :aria-label="t('theme')" @change="applyPreferences">
                <option value="system">{{ t('systemTheme') }}</option>
                <option v-for="theme in THEME_CATALOG" :key="theme.id" :value="theme.id">{{ theme.name[lang] }}</option>
              </select>
            </div>
            <label class="menu-row" @click.stop>
              <span>{{ t('customBase') }}</span>
              <input type="color" :value="customBase ?? effectiveTokens['--bg']" :aria-label="t('customBase')" @input="setCustomColor('base', $event)">
            </label>
            <label class="menu-row" @click.stop>
              <span>{{ t('customAccent') }}</span>
              <input type="color" :value="customAccent ?? effectiveTokens['--accent']" :aria-label="t('customAccent')" @input="setCustomColor('accent', $event)">
            </label>
            <button v-if="customAccent || customBase" type="button" @click.stop="resetCustomColors">{{ t('customReset') }}</button>
            <button type="button" :aria-label="t('language')" @click="toggleLang">
              <span>{{ t('language') }}</span><kbd>{{ lang === 'ru' ? 'RU' : 'EN' }}</kbd>
            </button>
            <button type="button" aria-keyshortcuts="?" aria-haspopup="dialog" @click="closeMenus(); shortcutHelpOpen = true">
              <span>{{ t('shortcuts') }}</span><kbd>?</kbd>
            </button>
          </div>
        </details>
      </div>
    </nav>

    <main
      ref="mainRef"
      class="main"
      :class="{ 'editor-drawer': editorOpen, 'sweep-source-preview': editorOpen && isRushFrontend(code) && (code.includes('progressive_sweep') || code.includes('miter_sweep')) }"
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
        <div v-if="error" class="message error" role="alert" aria-live="assertive">
          {{ error }}
        </div>
        <p class="group-hint">{{ lang === 'ru'
          ? 'Строится как точные тела. hull, projection, offset и polyhedron точной формы не имеют и будут отклонены.'
          : 'Built as exact solids. hull, projection, offset and polyhedron have no exact form and are refused.' }}</p>
      </section>
      <section v-else class="editor-panel" :style="{ width: `${editorWidth}px` }" :aria-label="t('editor')">
        <div class="toolbar editor-toolbar">
          <VrControls :meshes="sceneState.meshes" :visibility="sceneState.visibility" :isolated="sceneState.isolated" :selected-index="sceneState.selectedIndex" :locale="lang" />
          <button class="btn btn-primary" type="button" title="Ctrl/⌘+Enter" :disabled="rendering" @click="doRender('full')">
            <svg class="play" width="11" height="11" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M6 4v16l14-8z"/></svg>
            {{ t('render') }}
          </button>
          <button v-if="rendering" class="btn" type="button" @click="cancelGeometryBuild()">{{ t('cancelBuild') }}</button>
          <label class="auto-check"><input v-model="autoRender" type="checkbox"> {{ t('auto') }}</label>
          <button v-if="solidBuilding" class="btn" type="button" @click="cancelSolidBuild()">{{ lang === 'ru' ? 'Отмена' : 'Cancel' }}</button>
          <span class="toolbar-spacer" aria-hidden="true" />
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
              <button type="button" :disabled="solidBuilding" :title="lang === 'ru' ? 'Собрать точные тела (NURBS) и открыть в Solid' : 'Build exact NURBS solids and open them in Solid'" @click="buildSolidFromSource()">{{ solidBuilding ? '…' : (lang === 'ru' ? 'В Solid' : 'To Solid') }}</button>
              <button type="button" @click="exampleGalleryOpen = true">{{ t('examples') }}</button>
              <button type="button" @click="mechanicalGeneratorOpen = true">{{ t('generators') }}</button>
              <span class="menu-sep" aria-hidden="true" />
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
        <div v-show="!foldedLines.size" class="code-gutter"><div ref="lineNumbersRef" class="gutter-lines" :style="{ height: gutterLinesHeight }"><div v-for="row in visibleGutterRows" :key="row.line" class="gutter-row" :style="{ top: `${row.line * 1.58}em` }"><span aria-hidden="true">{{ row.line + 1 }}</span><button v-if="row.block" class="fold-toggle" :aria-label="`Свернуть блок, строка ${row.line + 1}`" aria-expanded="true" @click="toggleFold(row.line)">▾</button><span v-else class="fold-spacer" /></div></div></div>
        <div v-show="!foldedLines.size" class="code-content">
        <div class="code-guides" aria-hidden="true"><div ref="guidesRef" class="guide-lines" :style="{ height: gutterLinesHeight }"><div v-for="row in visibleGuideRows" :key="row.line" class="guide-row" :style="{ top: `${row.line * 1.58}em` }"><i v-for="guide in row.guides" :key="guide.start" :style="{ left: `${guide.column}ch`, background: guide.last ? `linear-gradient(to bottom, ${blockColors[guide.depth % blockColors.length]} 75%, transparent 100%)` : blockColors[guide.depth % blockColors.length], bottom: '0' }" /></div></div></div>
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
          <span class="vt-sep" aria-hidden="true" />
          <button class="view-btn icon-only" type="button" :aria-pressed="projection === 'orthographic'" :aria-label="projection === 'perspective' ? t('perspective') : t('orthographic')" :title="`${projection === 'perspective' ? t('perspective') : t('orthographic')} → ${projection === 'perspective' ? t('orthographic') : t('perspective')}`" @click="toggleProjection">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path v-if="projection === 'perspective'" d="M4 20L9 4h6l5 16zM6.5 12h11M5 16h14"/><path v-else d="M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7M12 11v10"/></svg>
          </button>
          <button class="view-btn icon-only" type="button" :aria-pressed="gridVisible" :aria-label="t('grid')" :title="t('grid')" @click="toggleGrid">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 9h18M3 15h18M9 3v18M15 3v18"/><rect x="3" y="3" width="18" height="18" rx="2"/></svg>
          </button>
          <ModelingGridControls :locale="lang" />
          <details class="viewport-material-menu">
            <summary :aria-label="lang === 'ru' ? 'Материал' : 'Material'">{{ lang === 'ru' ? 'Материал' : 'Material' }}</summary>
          <MaterialControls
            :locale="lang"
            :shading-model="shadingModel"
            :theme-id="renderThemeId"
            :base-color="defaultBaseColor"
            :metallic="defaultMetallic"
            :roughness="defaultRoughness"
            :matcap-id="matcapId"
            :env-id="envId"
            :shadows-enabled="shadowsEnabled"
            @update:shading-model="setShadingModel"
            @update:theme-id="setRenderTheme"
            @update:base-color="setDefaultBaseColor"
            @update:metallic="setDefaultMetallic"
            @update:roughness="setDefaultRoughness"
            @update:matcap-id="setMatcap"
            @update:env-id="setEnv"
            @update:shadows-enabled="setShadows"
            @apply-preset="applyMaterialPreset"
          />
          </details>
          <button
            ref="scanToggleRef" class="view-btn icon-only scan-toggle" type="button"
            :class="{ active: sectionEnabled }" :aria-label="t('section')" :title="t('scanPlane')"
            :aria-expanded="scanPanelOpen" aria-controls="scan-plane-panel"
            :disabled="!sectionAvailable && !sectionEnabled"
            @click="scanPanelOpen = !scanPanelOpen"
          ><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4z"/><path d="M4 7v10l8 4 8-4V7"/><path d="M2 12h20" stroke-dasharray="3 2"/></svg><i v-if="sectionEnabled" class="scan-active-dot" aria-hidden="true" /></button>
          <span class="vt-sep" aria-hidden="true" />
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
          <span class="vt-sep" aria-hidden="true" />
          <button ref="dockToggleRef" class="view-btn icon-only" type="button" :aria-label="t('sidebar')" :aria-pressed="dockOpen" :aria-expanded="dockOpen" aria-controls="cad-sidebar" @click="dockOpen = !dockOpen">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/></svg>
          </button>
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
        <div v-if="sweepBodyFinalEvidence.length" class="sweep-preview-status" data-testid="sweep-body-final-evidence" role="status" aria-live="polite">
          <div v-for="evidence in sweepBodyFinalEvidence" :key="evidence.nodeId">
            {{ lang === 'ru' ? 'Непрерывная ошибка границы sweep-тела' : 'Continuous sweep body boundary error' }}:
            {{ evidence.continuousBound ? (lang === 'ru' ? 'доказана' : 'certified') : (lang === 'ru' ? 'не доказана' : 'unproved') }}
            <span v-if="evidence.errorUpper !== null"> · {{ lang === 'ru' ? 'оценка' : 'bound' }} ≈ {{ evidence.errorUpper > 0 && evidence.errorUpper < 0.000001 ? evidence.errorUpper.toExponential(3) : formatNumber(evidence.errorUpper, 6) }} mm</span>
            <span v-if="evidence.withinBudget === false"> · {{ lang === 'ru' ? 'превышает допуск' : 'exceeds tolerance' }}</span>
            <SweepBoundaryBadges :lang="lang" :evidence="evidence" />
          </div>
        </div>
        <div v-if="sweepPatchFinalEvidence.length" class="sweep-preview-status" data-testid="sweep-patch-final-evidence" role="status" aria-live="polite">
          <div v-for="evidence in sweepPatchFinalEvidence" :key="evidence.nodeId">
            {{ lang === 'ru' ? 'Ошибка sweep-поверхностей относительно исходного переноса профиля' : 'Sweep patch error relative to original profile transport' }}:
            {{ evidence.continuousBound ? (lang === 'ru' ? 'доказана' : 'certified') : (lang === 'ru' ? 'не доказана' : 'unproved') }}
            <template v-if="evidence.errorUpper !== null"> · ≤{{ evidence.errorUpper.toPrecision(6) }} / {{ evidence.budget?.toPrecision(6) }} mm</template>
            <template v-if="evidence.sourceFrameC2Certified !== null && evidence.closedSourceFrameC2Certified === null">
              · {{ lang === 'ru' ? 'C2 исходного кадра' : 'Original frame C2' }}:
              {{ evidence.sourceFrameC2Certified ? (lang === 'ru' ? 'доказана' : 'certified') : (lang === 'ru' ? 'не доказана' : 'unproved') }}
            </template>
            <template v-if="evidence.closedSourceFrameC2Certified !== null">
              · {{ lang === 'ru' ? 'C2 замкнутого исходного кадра' : 'Closed source frame C2' }}:
              {{ evidence.closedSourceFrameC2Certified ? (lang === 'ru' ? 'доказана' : 'certified') : (lang === 'ru' ? 'не доказана' : 'unproved') }}
            </template>
            <template v-if="evidence.closedSourceFrameC1Certified !== null">
              · {{ lang === 'ru' ? 'C1 замкнутого исходного кадра' : 'Closed source frame C1' }}:
              {{ evidence.closedSourceFrameC1Certified ? (lang === 'ru' ? 'доказана' : 'certified') : (lang === 'ru' ? 'не доказана' : 'unproved') }}
            </template>
            <template v-if="evidence.decompositionG2Certified !== null">
              · {{ lang === 'ru' ? 'G2 между частями одного профиля' : 'G2 between parts of one profile' }}:
              {{ evidence.decompositionG2Certified ? (lang === 'ru' ? 'доказана' : 'certified') : (lang === 'ru' ? 'не доказана' : 'unproved') }}
              <template v-if="evidence.decompositionJoinCount !== null"> ({{ evidence.decompositionJoinCount }})</template>
            </template>
            <template v-if="evidence.decompositionG1Certified !== null && evidence.decompositionG2Certified !== true">
              · {{ lang === 'ru' ? 'G1 между частями одного профиля' : 'G1 between parts of one profile' }}:
              {{ evidence.decompositionG1Certified ? (lang === 'ru' ? 'доказана' : 'certified') : (lang === 'ru' ? 'не доказана' : 'unproved') }}
            </template>
            <template v-if="evidence.sourceFrameRegularityCertified !== null">
              · {{ lang === 'ru' ? 'Регулярность исходного кадра' : 'Original frame regularity' }}:
              {{ evidence.sourceFrameRegularityCertified ? (lang === 'ru' ? 'доказана' : 'certified') : (lang === 'ru' ? 'не доказана' : 'unproved') }}
            </template>
            · {{ lang === 'ru' ? 'Регулярность поверхностей' : 'Surface regularity' }}:
            {{ evidence.surfaceRegularityCertified ? (lang === 'ru' ? 'доказана' : 'certified') : (lang === 'ru' ? 'не доказана' : 'unproved') }}
            · {{ lang === 'ru' ? 'Область: сохранённые поверхности' : 'Scope: retained patches' }}
          </div>
        </div>
        <div v-if="sweepFinalEvidence.length" class="sweep-preview-status" data-testid="sweep-final-evidence" role="status" aria-live="polite">
          <div v-for="evidence in sweepFinalEvidence" :key="evidence.nodeId">
            {{ lang === 'ru' ? 'Геометрия тела' : 'Solid geometry' }}:
            {{ evidence.solidGeometryCertified ? (lang === 'ru' ? 'доказана' : 'certified') : (lang === 'ru' ? 'не доказана' : 'unproved') }} ·
            {{ lang === 'ru' ? 'Непрерывная ошибка границы' : 'Continuous boundary error' }}:
            {{ evidence.continuousBound ? (lang === 'ru' ? 'доказана' : 'certified') : (lang === 'ru' ? 'не доказана' : 'unproved') }} ·
            {{ lang === 'ru' ? 'Регулярность профиля / стен' : 'Profile / wall regularity' }}:
            {{ evidence.profileRegularityCertified ? '✓' : '?' }} / {{ evidence.wallRegularityCertified ? '✓' : '?' }}
            <template v-if="evidence.profileG2Certified !== undefined">
              · {{ lang === 'ru' ? 'G1 / G2 стыков профиля' : 'Profile joins G1 / G2' }}:
              {{ evidence.profileG1Certified ? '✓' : '?' }} / {{ evidence.profileG2Certified ? '✓' : '?' }} ({{ evidence.profileSeamCount }})
              · {{ lang === 'ru' ? 'Стыки вдоль пути' : 'Path station joins' }}: {{ evidence.stationContinuity ?? 'C0' }}
              <template v-if="evidence.capContinuity === 'C0'"> · {{ lang === 'ru' ? 'Стыки крышек' : 'Cap joins' }}: C0</template>
            </template>
            <template v-if="evidence.boundaryErrorUpper !== null">
              · {{ lang === 'ru' ? 'Оценка ошибки границы' : 'Boundary error bound' }}:
              ≈{{ evidence.boundaryErrorUpper.toPrecision(6) }} mm
              <template v-if="evidence.boundaryErrorBudget !== null">
                / {{ evidence.boundaryErrorBudget.toPrecision(6) }} mm
              </template>
              · {{ evidence.boundaryErrorWithinBudget === true ? (lang === 'ru' ? 'в допуске' : 'within budget') : evidence.boundaryErrorWithinBudget === false ? (lang === 'ru' ? 'выше допуска' : 'over budget') : (lang === 'ru' ? 'допуск не подтверждён' : 'budget unproved') }}
            </template>
          </div>
        </div>
        <div v-if="sweepViewportProgress" class="sweep-preview-status" role="status" aria-live="polite">
          {{ lang === 'ru' ? 'Предпросмотр стен' : 'Wall preview' }} ·
          {{ sweepViewportProgress.sections }} {{ lang === 'ru' ? 'сечений' : 'sections' }} ·
          {{ lang === 'ru' ? 'Контрольное отклонение' : 'Sampled deviation' }}
          {{ formatNumber(sweepViewportProgress.deviation, 6) }} /
          {{ formatNumber(sweepViewportProgress.budget, 6) }} mm
          <span v-if="sweepViewportProgress.certifiedErrorUpper !== undefined"> ·
            {{ sweepViewportProgress.certifiedErrorUpper === null
              ? (lang === 'ru' ? 'Граница ошибки интерполяции не доказана' : 'Section interpolation bound unproved')
              : (lang === 'ru' ? 'Доказанная граница ошибки интерполяции' : 'Certified section interpolation bound') }}
            <template v-if="sweepViewportProgress.certifiedErrorUpper !== null">:
              {{ formatNumber(sweepViewportProgress.certifiedErrorUpper, 6) }} mm
            </template>
          </span>
          <span v-else-if="sweepViewportProgress.continuousErrorUpper !== undefined"> ·
            {{ lang === 'ru' ? 'Непрерывная оценка' : 'Continuous estimate' }}:
            {{ formatNumber(sweepViewportProgress.continuousErrorUpper, 6) }} mm
            ({{ lang === 'ru' ? 'без сертификации округления' : 'rounding uncertified' }})
          </span>
          <span v-if="sweepViewportProgress.profileRegularityCertified !== undefined"> ·
            {{ sweepViewportProgress.profileRegularityCertified
              ? (lang === 'ru' ? 'Касательные профиля доказаны' : 'Profile tangents certified')
              : (lang === 'ru' ? 'Касательные профиля не доказаны' : 'Profile tangents unproved') }}
          </span>
          <span v-if="sweepViewportProgress.wallRegularityCertified !== undefined"> ·
            {{ sweepViewportProgress.wallRegularityCertified === null
              ? (lang === 'ru' ? 'Регулярность стен ожидает допуска ошибки' : 'Wall regularity awaits error admission')
              : sweepViewportProgress.wallRegularityCertified
                ? (lang === 'ru' ? 'Jacobian стен доказан' : 'Wall Jacobian certified')
                : (lang === 'ru' ? 'Jacobian стен не доказан' : 'Wall Jacobian unproved') }}
          </span>
          <span v-if="sweepViewportProgress.frameTransportCertified === false"> · {{ lang === 'ru' ? 'Не доказан перенос кадров' : 'Frame transport unproved' }}</span>
          <span v-if="sweepViewportProgress.phaseResolved === false"> · {{ lang === 'ru' ? 'Уточняется фаза twist' : 'Resolving twist phase' }}</span>
        </div>
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
          <SvgPanel :meshes="sceneMeshes" :hit="selectedHit" :available="canExport" :locale="lang" :can-append="!isRushFrontend(code)" :remaining-source="MAX_WORKSPACE_SOURCE_LENGTH - code.length - 2" :append-revision="workspaceDocument.documentId + ':' + workspaceDocument.mutation" @append="source => { replacePresetSource(code + '\n\n' + source); nextTick(() => doRender('full')) }" />
          </div>
          <div v-else-if="dockTab === 'photo'" class="dock-scroll" :ref="revealDockDetails">
          <PhotogrammetryPanel :locale="lang" :can-append="!isRushFrontend(code)" :remaining-source="MAX_WORKSPACE_SOURCE_LENGTH - code.length - 2" @append="source => replacePresetSource(code + '\n\n' + source)" />
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
      </section>
    </main>

    <footer class="statusbar" :aria-label="lang === 'ru' ? 'Состояние' : 'Status'">
      <span class="status-item"><i class="status-dot" :class="(directModelerOpen?solidGpuActive:gpuOk) ? 'ok' : 'off'" aria-hidden="true" />{{ directModelerOpen ? (solidGpuActive?t('backendWebGpu'):'CPU · SVG') : currentBackendQuality.backend === 'webgpu-interactive' ? t('backendWebGpu') : t('backendHeadless') }}</span>
      <span class="status-item kernel-badge">Manifold</span>
      <span v-if="currentBackendQuality.transparency === 'object-sorted-alpha'" class="status-item">{{ t('transparencySorted') }}</span>
      <span v-if="!directModelerOpen && !meshModelerOpen" class="status-item" role="status">{{ persistenceLabel }}</span>
      <span v-if="!directModelerOpen && !meshModelerOpen" class="status-item status-hint">{{ t('hint') }}</span>
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
      :style="{ top: topbarHeight + 'px' }"
      @backend="solidGpuActive = $event"
      :open="directModelerOpen"
      :locale="lang"
      :can-append="!isRushFrontend(code)"
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
      v-if="exampleGalleryOpen"
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
      :language="isRushFrontend(code) ? 'rush' : 'openscad'"
      :initial-query="functionReferenceQuery"
      @close="functionReferenceOpen = false"
    />
    <KeyboardShortcuts
      v-if="shortcutHelpOpen"
      :open="shortcutHelpOpen"
      :groups="shortcutHelpGroups"
      :locale="lang"
      @close="shortcutHelpOpen = false"
    />
  </div>
</template>

<style src="./styles/app-shell-global.css"></style>

<style scoped src="./styles/app-shell.css"></style>
