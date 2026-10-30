import {createMeshInteraction} from './useMeshModeler/meshInteraction'
import {createMeshCommands} from './useMeshModeler/meshCommands'
import VrControls from '../../components/VrControls.vue'
import { prepareVrPolygons } from '../../services/vrScene'
import { computed, nextTick, onUnmounted, ref, shallowRef, watch } from 'vue'
import { stringifyMeshJson } from '../../services/meshJson'
import ModelingFloorGrid from '../../components/ModelingFloorGrid.vue'
import ModelingGridControls from '../../components/ModelingGridControls.vue'
import WorkspaceActionBar from '../../components/WorkspaceActionBar.vue'
import CommandPalette from '../../components/CommandPalette.vue'
import type { PaletteCommand } from '../../services/commandSearch'
import { SCULPT_FALLOFFS, SCULPT_KINDS, buildSculptBrush, isFractionalSculptKind, type SculptFalloff, type SculptKind } from '../../services/geometryEditing'
import { meshCentroid, sculptMesh } from '../../services/meshEditing'
import {
  MeshHistory,
  buildEdgeList,
  createBoxMesh,
  createUvSphereMesh,
  deleteFaces,
  duplicateObject,
  emptyMeshDocument,
  exportMeshObjectStl,
  extrudeSelectedFaces,
  flipFaces,
  insetSelectedFaces,
  joinMeshes,
  knifeSplitEdges,
  mergeByDistance,
  meshObjectStats,
  moveVertices,
  moveVerticesProportional,
  parseMeshDocument,
  separateFaces,
  subdivideFaces,
  symmetrizeMesh,
  transformMesh,
  twistMesh,
  type MeshSelectMode,
  type MeshWorkspaceDocument,
} from '../../services/meshEditing'
import { createSolidPreviewWorker } from '../../services/solidPreviewWorker'
import { useMainSolidTask } from '.././useMainSolidTask'
import { defaultDirectCamera, projectDirectPoint } from '../../services/directModelingTools'
import { projectMeshModelerObject,cachedMeshModelerStats } from '../../services/meshModelerProjection'
import { isGeometryKernelReady, warmGeometryKernel } from '../../services/geometry/kernel'
import { storageGet, storageSet } from '../../services/safeStorage'
import { MESH_IMPORT_ACCEPT, stripMeshExtension } from '../../services/meshImport'
import { manifoldPreferenceSnapshot } from '../../services/manifoldPreferences'
import ManifoldSettings from '../../components/ManifoldSettings.vue'
import { formatMeshImportNotice } from '../../services/meshImportReport'
import { formatMeshBooleanNotice } from '../../services/meshBooleanReport'
import { formatMeshRepairNotice } from '../../services/meshRepairReport'
import { meshManifoldDefectLabels } from '../../services/meshManifoldDefects'
import { MESH_EXPORT_FORMATS, MESH_FORMAT_LABELS, type MeshExportFormat } from '../../services/meshConvert'
import { downloadBytes } from '../../services/downloadArtifact'
import type { PolygonMesh } from '../../services/geometry/polygon'
import type { KernelManifoldReport } from '../../services/geometry/manifoldAnalysis'
export interface MeshModelerProps { open: boolean; locale: string; seedDocument?: MeshWorkspaceDocument | null; paletteRequest?: number }
export type MeshModelerEmit = { (event: 'close'): void; (event: 'exportToSolid', doc: MeshWorkspaceDocument): void }
export function useMeshModeler(props: MeshModelerProps, emit: MeshModelerEmit) {
  const {downloadJson, togglePick, getEdgeList, projected, screenDeltaToWorld, startElementDrag, moveElementDrag, endElementDrag, cancelElementDrag, onPointerDown, onPointerMove, onPointerUp, onClickCapture, resetView, onWheel, vrSnapshot} = createMeshInteraction({
    get document() { return document },
    get selectedVerts() { return selectedVerts },
    get selectedFaces() { return selectedFaces },
    get selectedEdges() { return selectedEdges },
    get edgeListCache() { return edgeListCache },
    get camera() { return camera },
    get selectMode() { return selectMode },
    get manifoldReports() { return manifoldReports },
    get view() { return view },
    get selection() { return selection },
    get elementDrag() { return elementDrag },
    set elementDrag(value: typeof elementDrag) { elementDrag = value },
    get dragSwallowsClick() { return dragSwallowsClick },
    set dragSwallowsClick(value: typeof dragSwallowsClick) { dragSwallowsClick = value },
    get proportionalEdit() { return proportionalEdit },
    get proportionalRadius() { return proportionalRadius },
    get history() { return history },
    get error() { return error },
    get run() { return run },
    get commit() { return commit },
    get orbit() { return orbit },
    set orbit(value: typeof orbit) { orbit = value },
    get center() { return center },
  })

  const {addPrimitive, transformSelected, applyExtrude, applyInset, applySubdivide, applyKnife, applyDeleteFaces, applyFlip, applyMerge, applyRepair, applySeparate, applyJoin, applySymmetrize, applyBrush, applyTwist, applyDuplicate, applyBoolean, removeSelected, importMesh, downloadStl, downloadMesh} = createMeshCommands({
    get run() { return run },
    get history() { return history },
    get commit() { return commit },
    get selection() { return selection },
    get selectMode() { return selectMode },
    get selected() { return selected },
    get selectedVerts() { return selectedVerts },
    get selectedEdges() { return selectedEdges },
    get getEdgeList() { return getEdgeList },
    get proportionalEdit() { return proportionalEdit },
    get dx() { return dx },
    get dy() { return dy },
    get dz() { return dz },
    get proportionalRadius() { return proportionalRadius },
    get angle() { return angle },
    get scale() { return scale },
    get selectedFaces() { return selectedFaces },
    get label() { return label },
    get extrudeDistance() { return extrudeDistance },
    get insetAmount() { return insetAmount },
    get mergeDistance() { return mergeDistance },
    get meshTaskPending() { return meshTaskPending },
    get error() { return error },
    get repairNotice() { return repairNotice },
    get importNotice() { return importNotice },
    get booleanNotice() { return booleanNotice },
    get document() { return document },
    get runMeshTask() { return runMeshTask },
    get ru() { return ru },
    get booleanTarget() { return booleanTarget },
    get brushKind() { return brushKind },
    get brushRadius() { return brushRadius },
    get brushStrength() { return brushStrength },
    get brushFalloff() { return brushFalloff },
    get brushMirror() { return brushMirror },
    get twistAmount() { return twistAmount },
    get booleanOp() { return booleanOp },
    get exportFormat() { return exportFormat },
    get props() { return props },
  })


const ru = computed(() => props.locale === 'ru')
const label = (a: string, b: string) => (ru.value ? a : b)
const storageKey = 'scad-mesh-modeler-v1'

let initial = emptyMeshDocument()
try {
  const stored = storageGet(storageKey)
  if (stored) initial = parseMeshDocument(stored)
} catch { /* start empty */ }

const history = new MeshHistory(initial)
const document = shallowRef(history.document)
const undoable = ref(false)
const redoable = ref(false)
const undoOperation = ref('')
const redoOperation = ref('')
const error = ref('')
const importNotice = ref('')
const booleanNotice = ref('')
const repairNotice = ref('')
const solidWorker = createSolidPreviewWorker()
const {
  pending: meshTaskPending,
  progress: meshTaskProgress,
  kind: meshTaskKind,
  run: runMeshTask,
  cancel: cancelMeshTask,
  dispose: disposeMeshTask,
} = useMainSolidTask(solidWorker)
const manifoldWorker = createSolidPreviewWorker()
const {
  pending: manifoldTaskPending,
  progress: manifoldTaskProgress,
  run: runManifoldTask,
  cancel: cancelManifoldTask,
  dispose: disposeManifoldTask,
} = useMainSolidTask(manifoldWorker)
const meshTaskStage = computed(() => {
  const progress = meshTaskProgress.value
  if (meshTaskKind.value === 'meshBoolean') return `${label('Булева операция manifold', 'Manifold Boolean operation')} · ${progress?.completed ?? 0}/${progress?.total ?? 1}`
  if (meshTaskKind.value === 'meshExport') {
    const phase = progress?.phase === 'export'
      ? label('Экспорт файла', 'Exporting file')
      : label('Анализ меша', 'Inspecting mesh')
    return `${phase} · ${progress?.completed ?? 0}/${progress?.total ?? 2}`
  }
  if (meshTaskKind.value === 'meshImport') {
    const phase = progress?.phase === 'decode'
      ? label('Разбор файла', 'Decoding file')
      : progress?.phase === 'finalize'
        ? label('Сварка и индексация', 'Welding and indexing')
        : progress?.phase === 'check'
          ? label('Проверка manifold', 'Checking manifold topology')
          : progress?.phase === 'repair'
            ? label('Исправление топологии', 'Repairing topology')
            : label('Повторная проверка', 'Verifying the result')
    return `${phase} · ${progress?.completed ?? 0}/${progress?.total ?? 5}`
  }
  const phase = progress?.phase === 'repair'
    ? label('Исправление топологии', 'Repairing topology')
    : progress?.phase === 'verify'
      ? label('Повторная проверка', 'Verifying the result')
      : label('Проверка исходного меша', 'Checking the source mesh')
  return `${phase} · ${progress?.completed ?? 0}/${progress?.total ?? 3}`
})
const meshTaskPresentation = computed(() => {
  if (meshTaskKind.value === 'meshBoolean') return {
    className: 'boolean-progress', total: 1,
    progressLabel: label('Прогресс булевой операции', 'Boolean operation progress'),
    cancelLabel: label('Отменить булеву операцию', 'Cancel Boolean'),
  }
  if (meshTaskKind.value === 'meshImport') return {
    className: 'import-progress', total: 5,
    progressLabel: label('Прогресс импорта', 'Mesh import progress'),
    cancelLabel: label('Отменить импорт', 'Cancel import'),
  }
  if (meshTaskKind.value === 'meshExport') return {
    className: 'export-progress', total: 2,
    progressLabel: label('Прогресс экспорта', 'Mesh export progress'),
    cancelLabel: label('Отменить экспорт', 'Cancel export'),
  }
  return {
    className: 'repair-progress', total: 3,
    progressLabel: label('Прогресс manifold repair', 'Manifold repair progress'),
    cancelLabel: label('Отменить repair', 'Cancel repair'),
  }
})
watch(() => props.open, open => { if (!open) cancelMeshTask() })
onUnmounted(() => { disposeMeshTask(); disposeManifoldTask() })
const kernelReady = ref(isGeometryKernelReady())
// Mesh can open before Solid or the source viewport has warmed the shared WASM.
// Publishing readiness invalidates projections that were requested while cold.
watch(() => props.open, async open => {
  if (!open || kernelReady.value) return
  try {
    await warmGeometryKernel()
    kernelReady.value = true
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught)
  }
}, { immediate: true })
const selection = ref('')
const selectMode = ref<MeshSelectMode>('object')
const selectedVerts = ref<number[]>([])
const selectedFaces = ref<number[]>([])
const selectedEdges = ref<number[]>([])
const dx = ref(0), dy = ref(0), dz = ref(0), angle = ref(0), scale = ref(1)
const proportionalEdit = ref(false)
const proportionalRadius = ref(5)
const extrudeDistance = ref(2)
const insetAmount = ref(0.5)
const mergeDistance = ref(0.01)
const brushRadius = ref(5)
const brushStrength = ref(1)
const brushKind = ref<SculptKind>('grab')
const brushFalloff = ref<SculptFalloff>('smooth')
const brushMirror = ref<[boolean, boolean, boolean]>([false, false, false])
const brushIsFraction = computed(() => isFractionalSculptKind(brushKind.value))
const twistAmount = ref(0.1)
const booleanOp = ref<'union' | 'difference' | 'intersection'>('union')
const booleanTarget = ref('')
const camera = ref(defaultDirectCamera())
const view = ref(140)
const center = ref<[number, number]>([0, 0])
const workspace = ref<HTMLElement>()
const fileInput = ref<HTMLInputElement>()
// LMB / RMB drag orbits, Shift or middle drag pans; a drag past the threshold swallows the following click so picking stays intact.
let orbit: { x: number; y: number; yaw: number; pitch: number; pan: boolean; center: [number, number]; moved: boolean } | null = null
let dragSwallowsClick = false

watch(() => props.open, async open => {
  if (open) {
    try {
      const stored = storageGet(storageKey)
      if (stored) {
        history.commit(parseMeshDocument(stored))
        document.value = history.document
        syncHistoryFlags()
      }
    } catch { /* keep current */ }
    await nextTick(() => workspace.value?.focus())
  }
}, { immediate: true })

watch(document, d => {
  if (elementDrag) return
  try { storageSet(storageKey, stringifyMeshJson(d)) } catch { /* ignore quota */ }
})

// A seed (the Code scene or a Solid document) replaces the workspace once; later opens keep the user's edits.
const appliedSeeds = new WeakSet<MeshWorkspaceDocument>()
watch([() => props.open, () => props.seedDocument], ([open, seed]) => {
  if (!open || !seed || appliedSeeds.has(seed)) return
  try {
    commit(parseMeshDocument(stringifyMeshJson(seed)))
    selection.value = history.document.objects[0]?.id ?? ''
    selectedVerts.value = []; selectedFaces.value = []; selectedEdges.value = []
    appliedSeeds.add(seed)
  } catch (e) { error.value = e instanceof Error ? e.message : String(e) }
}, { immediate: true })
const floorVisible = ref(true)
const selected = computed(() => document.value.objects.find(o => o.id === selection.value))
watch([document, selection], () => cancelMeshTask())
const stats=computed(()=>kernelReady.value&&selected.value?cachedMeshModelerStats(selected.value.mesh):null)
const manifoldReports = shallowRef<ReadonlyMap<string, KernelManifoldReport>>(new Map())
async function refreshManifoldReports() {
  cancelManifoldTask()
  manifoldReports.value = new Map()
  const objects = document.value.objects.map(({ id, mesh }) => ({ id, mesh }))
  if (!props.open || !kernelReady.value || !objects.length) return
  await runManifoldTask(
    { kind: 'meshManifold', objects },
    entries => { manifoldReports.value = new Map(entries.map(entry => [entry.id, entry.report])) },
    caught => { error.value = caught instanceof Error ? caught.message : String(caught) },
  )
}
watch([document, kernelReady, () => props.open], () => { void refreshManifoldReports() }, { immediate: true, flush: 'sync' })
const expandedManifoldObject = ref('')
const manifoldDefects = (objectId: string) => {
  const report = manifoldReports.value.get(objectId)
  return report ? meshManifoldDefectLabels(report, ru.value ? 'ru' : 'en') : []
}

function syncHistoryFlags() {
  undoable.value = history.canUndo
  redoable.value = history.canRedo
  undoOperation.value = history.undoOperation
  redoOperation.value = history.redoOperation
}

function commit(next: MeshWorkspaceDocument, operation = label('Редактирование меша', 'Edit mesh')) {
  history.commit(next, operation)
  document.value = history.document
  syncHistoryFlags()
  error.value = ''
}

function run(fn: () => void) {
  try { fn() } catch (e) { error.value = e instanceof Error ? e.message : String(e) }
}

function undo() { document.value = history.undo(); syncHistoryFlags() }
function redo() { document.value = history.redo(); syncHistoryFlags() }

const exportFormat = ref<MeshExportFormat>('3mf')

// Command palette: header actions, object tools and edit operations, filtered to what applies to the current selection.
const paletteOpen = ref(false)
const dockOpen = ref(storageGet('scad-mesh-dock') === 'true')
const dockTab = ref<'scene' | 'props'>('scene')
watch(dockOpen, open => storageSet('scad-mesh-dock', String(open)))
const propsOpen = computed({ get: () => dockOpen.value && dockTab.value === 'props', set: open => { if (open) { dockOpen.value = true; dockTab.value = 'props' } else dockTab.value = 'scene' } })
watch(() => props.paletteRequest, request => { if (request && props.open) paletteOpen.value = true })
type MeshCommand = PaletteCommand & { run: () => void }
const meshCommands = computed<MeshCommand[]>(() => {
  const cmd = (id: string, ru: string, en: string, run: () => void, extra: Partial<PaletteCommand> = {}): MeshCommand => ({ id, label: label(ru, en), aliases: [ru, en], run, ...extra })
  const hasObject = !!selected.value, hasFaces = selectedFaces.value.length > 0, hasEdges = selectedEdges.value.length > 0
  const needObject = label('Выберите объект', 'Select an object'), needFaces = label('Выберите грани', 'Select faces')
  const modes: Array<[MeshSelectMode, string, string, string]> = [['object', 'Режим: объект', 'Mode: object', '1'], ['vertex', 'Режим: вершина', 'Mode: vertex', '2'], ['edge', 'Режим: ребро', 'Mode: edge', '3'], ['face', 'Режим: грань', 'Mode: face', '4']]
  return [
    cmd('add-box', 'Куб', 'Cube', () => addPrimitive('box'), { detail: label('Добавить', 'Add') }),
    cmd('add-sphere', 'UV-сфера', 'UV Sphere', () => addPrimitive('sphere'), { detail: label('Добавить', 'Add') }),
    cmd('import', 'Импорт сетки', 'Import mesh', () => fileInput.value?.click(), { detail: 'STL / OBJ / PLY / OFF / AMF / 3MF' }),
    ...modes.map(([mode, ru, en, key]) => cmd(`mode-${mode}`, ru, en, () => { selectMode.value = mode }, { shortcut: key, enabled: selectMode.value !== mode })),
    cmd('props', propsOpen.value ? 'Скрыть панель свойств' : 'Панель свойств', propsOpen.value ? 'Hide properties panel' : 'Properties panel', () => { propsOpen.value = !propsOpen.value }, { detail: label('Вид', 'View') }),
    cmd('reset-view', 'Сбросить вид', 'Reset view', resetView, { detail: label('Вид', 'View') }),
    cmd('grid', floorVisible.value ? 'Скрыть сетку' : 'Показать сетку', floorVisible.value ? 'Hide grid' : 'Show grid', () => { floorVisible.value = !floorVisible.value }, { detail: label('Вид', 'View') }),
    cmd('undo', 'Отменить', 'Undo', undo, { shortcut: 'Ctrl Z', enabled: undoable.value }),
    cmd('redo', 'Повторить', 'Redo', redo, { shortcut: 'Ctrl Shift Z', enabled: redoable.value }),
    cmd('transform', 'Применить трансформ', 'Apply transform', transformSelected, { detail: label('Выбранное', 'Selection'), enabled: hasObject, disabledReason: needObject }),
    cmd('extrude', 'Выдавить грани', 'Extrude faces', applyExtrude, { detail: label('Грани', 'Faces'), shortcut: 'E', enabled: hasObject && hasFaces, disabledReason: needFaces }),
    cmd('inset', 'Inset', 'Inset', applyInset, { detail: label('Грани', 'Faces'), shortcut: 'I', enabled: hasObject && hasFaces, disabledReason: needFaces }),
    cmd('subdivide', 'Subdivide', 'Subdivide', applySubdivide, { detail: label('Грани', 'Faces'), enabled: hasObject, disabledReason: needObject }),
    cmd('delete-faces', 'Удалить грани', 'Delete faces', applyDeleteFaces, { detail: label('Грани', 'Faces'), enabled: hasObject && hasFaces, disabledReason: needFaces }),
    cmd('flip', 'Перевернуть нормали', 'Flip normals', applyFlip, { detail: label('Грани', 'Faces'), enabled: hasObject, disabledReason: needObject }),
    cmd('knife', 'Разрез по середине рёбер', 'Knife at edge midpoints', applyKnife, { detail: label('Рёбра', 'Edges'), shortcut: 'K', enabled: hasObject && hasEdges, disabledReason: label('Выберите рёбра', 'Select edges') }),
    cmd('merge', 'Слияние по расстоянию', 'Merge by distance', applyMerge, { detail: label('Топология', 'Topology'), enabled: hasObject, disabledReason: needObject }),
    cmd('repair', 'Починить меш', 'Repair mesh', applyRepair, { detail: 'Full · manifold', enabled: hasObject && !meshTaskPending.value, disabledReason: meshTaskPending.value ? label('Repair уже выполняется', 'Repair is already running') : needObject }),
    cmd('separate', 'Отделить грани', 'Separate faces', applySeparate, { detail: label('Топология', 'Topology'), enabled: hasObject && hasFaces, disabledReason: needFaces }),
    cmd('join', 'Объединить объекты', 'Join objects', applyJoin, { detail: label('Топология', 'Topology'), enabled: document.value.objects.length > 1, disabledReason: label('Нужно два объекта', 'Two objects are required') }),
    cmd('symmetrize', 'Симметрия по X', 'Symmetrize X', applySymmetrize, { detail: label('Топология', 'Topology'), enabled: hasObject, disabledReason: needObject }),
    cmd('twist', 'Twist deform', 'Twist deform', applyTwist, { detail: label('Деформация', 'Deform'), enabled: hasObject, disabledReason: needObject }),
    cmd('duplicate', 'Дублировать объект', 'Duplicate object', applyDuplicate, { enabled: hasObject, disabledReason: needObject }),
    cmd('delete-object', 'Удалить объект', 'Delete object', removeSelected, { enabled: hasObject, disabledReason: needObject }),
    cmd('to-solid', 'Открыть в Solid', 'Open in Solid', () => emit('exportToSolid', document.value), { detail: label('Файл', 'File'), enabled: document.value.objects.length > 0, disabledReason: label('Нет объектов', 'No objects') }),
    cmd('download-json', 'Скачать проект JSON', 'Download JSON project', downloadJson, { detail: label('Файл', 'File') }),
  ]
})
function executeMeshCommand(id: string) {
  paletteOpen.value = false
  const target = meshCommands.value.find(command => command.id === id)
  if (target && target.enabled !== false) run(target.run)
}

function onWorkspaceKey(event: KeyboardEvent) {
  if (paletteOpen.value) return
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') { event.preventDefault(); paletteOpen.value = true; return }
  if (event.key === 'Escape') {
    if (meshTaskPending.value) { cancelMeshTask(); return }
    if (manifoldTaskPending.value) { cancelManifoldTask(); return }
    if (elementDrag) { cancelElementDrag(); return }
    if (propsOpen.value) { propsOpen.value = false; return }
    emit('close')
  }
  if ((event.target as HTMLElement).matches('input,textarea,select')) return
  if (event.key === '\\' && !event.ctrlKey && !event.metaKey && !event.altKey) { event.preventDefault(); dockOpen.value = !dockOpen.value; return }
  const modes: Record<string, MeshSelectMode> = { '1': 'object', '2': 'vertex', '3': 'edge', '4': 'face' }
  if (modes[event.key]) { selectMode.value = modes[event.key]; return }
  if (event.key.toLowerCase() === 'f') resetView()
}

const edgeListCache = new WeakMap<PolygonMesh, Array<[number, number]>>()

// Dragging a vertex, edge or face moves the current selection in the screen plane (LMB, no modifiers);
// the working copy previews on every move and one history step is committed on release.
let elementDrag: { objectId: string; vertexIds: number[]; base: PolygonMesh; x: number; y: number; moved: boolean; svg: SVGSVGElement } | null = null
const scene = computed(() => !kernelReady.value ? [] : document.value.objects.filter(o => o.visible).map(o => ({ id: o.id, ...projected(o) })))
// SVG uses painter order: sort all faces together so nearer objects also cover farther ones.
const sceneFaces = computed(() => {
  const faces: Array<{ points: string; face: number; depth: number; objectId: string }> = []
  for (const object of scene.value) {
    const objectId = object.id
    for (const tri of object.tris) {
      faces.push({ points: tri.points, face: tri.face, depth: tri.depth, objectId })
    }
  }
  return faces.sort((a, b) => a.depth - b.depth)
})
return {
  ru, label, storageKey, initial, history,
  document, undoable, redoable, undoOperation, redoOperation,
  error, importNotice, booleanNotice, repairNotice, solidWorker,
  meshTaskPending, meshTaskProgress, meshTaskKind, runMeshTask, cancelMeshTask,
  disposeMeshTask, manifoldWorker, manifoldTaskPending, manifoldTaskProgress, runManifoldTask,
  cancelManifoldTask, disposeManifoldTask, meshTaskStage, meshTaskPresentation, kernelReady,
  selection, selectMode, selectedVerts, selectedFaces, selectedEdges,
  dx, dy, dz, angle, scale,
  proportionalEdit, proportionalRadius, extrudeDistance, insetAmount, mergeDistance,
  brushRadius, brushStrength, brushKind, brushFalloff, brushMirror,
  brushIsFraction, twistAmount, booleanOp, booleanTarget, camera,
  view, center, workspace, fileInput, orbit,
  dragSwallowsClick, appliedSeeds, floorVisible, selected, stats,
  manifoldReports, refreshManifoldReports, expandedManifoldObject, manifoldDefects, syncHistoryFlags,
  commit, run, undo, redo, addPrimitive,
  transformSelected, applyExtrude, applyInset, applySubdivide, applyKnife,
  applyDeleteFaces, applyFlip, applyMerge, applyRepair, applySeparate,
  applyJoin, applySymmetrize, applyBrush, applyTwist, applyDuplicate,
  applyBoolean, removeSelected, importMesh, downloadStl, exportFormat,
  downloadMesh, paletteOpen, dockOpen, dockTab, propsOpen,
  meshCommands, executeMeshCommand, onWorkspaceKey, downloadJson, togglePick,
  edgeListCache, getEdgeList, projected, elementDrag, screenDeltaToWorld,
  startElementDrag, moveElementDrag, endElementDrag, cancelElementDrag, onPointerDown,
  onPointerMove, onPointerUp, onClickCapture, resetView, onWheel,
  vrSnapshot, scene, sceneFaces
}
}
