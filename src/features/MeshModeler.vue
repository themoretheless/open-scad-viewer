<script setup lang="ts">
import { computed, nextTick, ref, shallowRef, watch } from 'vue'
import { stringifyMeshJson } from '../services/meshJson'
import CommandPalette from '../components/CommandPalette.vue'
import type { PaletteCommand } from '../services/commandSearch'
import { SCULPT_FALLOFFS, SCULPT_KINDS, buildSculptBrush, isFractionalSculptKind, type SculptFalloff, type SculptKind } from '../services/geometryEditing'
import { meshCentroid, sculptMesh } from '../services/meshEditing'
import {
  MeshHistory,
  booleanMeshObjects,
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
} from '../services/meshEditing'
import { defaultDirectCamera, projectDirectPoint } from '../services/directModelingTools'
import { storageGet, storageSet } from '../services/safeStorage'
import { importMeshFromFile, MESH_IMPORT_ACCEPT, stripMeshExtension } from '../services/meshImport'
import { polygonMeshToExportMesh, MESH_EXPORT_FORMATS, MESH_FORMAT_LABELS, type MeshExportFormat } from '../services/meshConvert'
import { exportMeshFormatCompressed } from '../services/meshExportFormats'
import { downloadBytes } from '../services/downloadArtifact'
import type { PolygonMesh } from '../services/geometry/polygon'

const props = defineProps<{ open: boolean; locale: string; seedDocument?: MeshWorkspaceDocument | null; paletteRequest?: number }>()
const emit = defineEmits<{ close: []; exportToSolid: [doc: MeshWorkspaceDocument] }>()
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
const error = ref('')
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
  try { storageSet(storageKey, stringifyMeshJson(d)) } catch { /* ignore quota */ }
}, { deep: true })

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
const floorLines = computed(() => {
  const cam = camera.value
  // The SVG shows roughly 10000 / view world units across; keep about 20 grid cells in view.
  const step = Math.pow(10, Math.floor(Math.log10(Math.max(1, 10000 / view.value / 4))))
  const extent = step * 20
  const lines: string[] = []
  for (let i = -20; i <= 20; i++) {
    const n = i * step
    for (const line of [[[-extent, n, 0], [extent, n, 0]], [[n, -extent, 0], [n, extent, 0]]] as const) {
      lines.push(line.map(p => { const q = projectDirectPoint([p[0], p[1], p[2]], cam); return `${q[0]},${-q[1]}` }).join(' '))
    }
  }
  return lines
})

const selected = computed(() => document.value.objects.find(o => o.id === selection.value))
const stats = computed(() => {
  if (!selected.value) return null
  const base = meshObjectStats(selected.value.mesh)
  return { ...base, edges: buildEdgeList(selected.value.mesh).length }
})

function syncHistoryFlags() {
  undoable.value = history.canUndo
  redoable.value = history.canRedo
}

function commit(next: MeshWorkspaceDocument) {
  history.commit(next)
  document.value = history.document
  syncHistoryFlags()
  error.value = ''
}

function run(fn: () => void) {
  try { fn() } catch (e) { error.value = e instanceof Error ? e.message : String(e) }
}

function undo() { document.value = history.undo(); syncHistoryFlags() }
function redo() { document.value = history.redo(); syncHistoryFlags() }

function addPrimitive(kind: 'box' | 'sphere') {
  run(() => {
    const d = history.document
    const id = `mesh-${Date.now().toString(36)}`
    d.objects.push({
      id,
      name: kind === 'box' ? 'Cube' : 'UV Sphere',
      mesh: kind === 'box' ? createBoxMesh([20, 20, 20]) : createUvSphereMesh(10, 24, 16),
      visible: true,
    })
    commit(d)
    selection.value = id
    selectMode.value = 'object'
  })
}

function transformSelected() {
  run(() => {
    if (!selected.value) return
    const d = history.document
    const index = d.objects.findIndex(o => o.id === selection.value)
    if ((selectMode.value === 'vertex' && selectedVerts.value.length) ||
        (selectMode.value === 'edge' && selectedEdges.value.length)) {
      const vertexIds = selectMode.value === 'edge'
        ? [...new Set(selectedEdges.value.flatMap(edge => buildEdgeList(d.objects[index].mesh)[edge] ?? []))]
        : selectedVerts.value
      d.objects[index] = {
        ...d.objects[index],
        mesh: selectMode.value === 'vertex' && proportionalEdit.value
          ? moveVerticesProportional(
              d.objects[index].mesh,
              vertexIds,
              [dx.value, dy.value, dz.value],
              proportionalRadius.value,
            )
          : moveVertices(d.objects[index].mesh, vertexIds, [dx.value, dy.value, dz.value]),
      }
    } else {
      d.objects[index] = {
        ...d.objects[index],
        mesh: transformMesh(d.objects[index].mesh, [dx.value, dy.value, dz.value], angle.value, scale.value),
      }
    }
    commit(d)
  })
}

function applyExtrude() {
  run(() => {
    if (!selected.value || !selectedFaces.value.length) throw new Error(label('Выберите грани.', 'Select faces.'))
    const d = history.document
    const index = d.objects.findIndex(o => o.id === selection.value)
    d.objects[index] = {
      ...d.objects[index],
      mesh: extrudeSelectedFaces(d.objects[index].mesh, selectedFaces.value, extrudeDistance.value),
    }
    commit(d)
    selectedFaces.value = []
  })
}

function applyInset() {
  run(() => {
    if (!selected.value || !selectedFaces.value.length) throw new Error(label('Выберите грани.', 'Select faces.'))
    const d = history.document
    const index = d.objects.findIndex(o => o.id === selection.value)
    d.objects[index] = {
      ...d.objects[index],
      mesh: insetSelectedFaces(d.objects[index].mesh, selectedFaces.value, insetAmount.value),
    }
    commit(d)
  })
}

function applySubdivide() {
  run(() => {
    if (!selected.value) return
    const d = history.document
    const index = d.objects.findIndex(o => o.id === selection.value)
    const faces = selectedFaces.value.length ? selectedFaces.value : undefined
    d.objects[index] = { ...d.objects[index], mesh: subdivideFaces(d.objects[index].mesh, faces) }
    commit(d)
    selectedFaces.value = []
  })
}

function applyKnife() {
  run(() => {
    if (!selected.value || !selectedEdges.value.length) throw new Error(label('Выберите рёбра.', 'Select edges.'))
    const d = history.document
    const index = d.objects.findIndex(o => o.id === selection.value)
    d.objects[index] = { ...d.objects[index], mesh: knifeSplitEdges(d.objects[index].mesh, selectedEdges.value) }
    commit(d)
    selectedEdges.value = []
  })
}

function applyDeleteFaces() {
  run(() => {
    if (!selected.value || !selectedFaces.value.length) throw new Error(label('Выберите грани.', 'Select faces.'))
    const d = history.document
    const index = d.objects.findIndex(o => o.id === selection.value)
    d.objects[index] = { ...d.objects[index], mesh: deleteFaces(d.objects[index].mesh, selectedFaces.value) }
    commit(d)
    selectedFaces.value = []
  })
}

function applyFlip() {
  run(() => {
    if (!selected.value) return
    const d = history.document
    const index = d.objects.findIndex(o => o.id === selection.value)
    d.objects[index] = {
      ...d.objects[index],
      mesh: flipFaces(d.objects[index].mesh, selectedFaces.value.length ? selectedFaces.value : undefined),
    }
    commit(d)
  })
}

function applyMerge() {
  run(() => {
    if (!selected.value) return
    const d = history.document
    const index = d.objects.findIndex(o => o.id === selection.value)
    d.objects[index] = { ...d.objects[index], mesh: mergeByDistance(d.objects[index].mesh, mergeDistance.value) }
    commit(d)
    selectedVerts.value = []
    selectedFaces.value = []
  })
}

function applySeparate() {
  run(() => {
    if (!selected.value || !selectedFaces.value.length) throw new Error(label('Выберите грани.', 'Select faces.'))
    const d = history.document
    const index = d.objects.findIndex(o => o.id === selection.value)
    const { kept, separated } = separateFaces(d.objects[index].mesh, selectedFaces.value)
    d.objects[index] = { ...d.objects[index], mesh: kept }
    const id = `mesh-${Date.now().toString(36)}`
    d.objects.push({ id, name: `${d.objects[index].name} part`, mesh: separated, visible: true })
    commit(d)
    selectedFaces.value = []
    selection.value = id
  })
}

function applyJoin() {
  run(() => {
    if (!selected.value || !booleanTarget.value || booleanTarget.value === selection.value) {
      throw new Error(label('Выберите два разных объекта.', 'Pick two different objects.'))
    }
    const d = history.document
    const a = d.objects.find(o => o.id === selection.value)!
    const b = d.objects.find(o => o.id === booleanTarget.value)
    if (!b) throw new Error(label('Второй объект не найден.', 'Second object missing.'))
    a.mesh = joinMeshes([a.mesh, b.mesh])
    d.objects = d.objects.filter(o => o.id !== b.id)
    commit(d)
    booleanTarget.value = ''
  })
}

function applySymmetrize() {
  run(() => {
    if (!selected.value) return
    const d = history.document
    const index = d.objects.findIndex(o => o.id === selection.value)
    d.objects[index] = { ...d.objects[index], mesh: symmetrizeMesh(d.objects[index].mesh, 0) }
    commit(d)
  })
}

function applyBrush() {
  run(() => {
    if (!selected.value) return
    const mesh = selected.value.mesh
    // Selected vertices define the stroke center when present; otherwise the object centroid.
    const center = meshCentroid(mesh, selectedVerts.value)
    const brush = buildSculptBrush({ kind: brushKind.value, radius: brushRadius.value, strength: brushStrength.value, falloff: brushFalloff.value, mirror: brushMirror.value }, center)
    const d = history.document
    const index = d.objects.findIndex(o => o.id === selection.value)
    d.objects[index] = { ...d.objects[index], mesh: sculptMesh(mesh, brush) }
    commit(d)
  })
}

function applyTwist() {
  run(() => {
    if (!selected.value) return
    const d = history.document
    const index = d.objects.findIndex(o => o.id === selection.value)
    d.objects[index] = { ...d.objects[index], mesh: twistMesh(d.objects[index].mesh, twistAmount.value) }
    commit(d)
  })
}

function applyDuplicate() {
  run(() => {
    if (!selected.value) return
    const d = history.document
    const copy = duplicateObject(selected.value)
    d.objects.push(copy)
    commit(d)
    selection.value = copy.id
  })
}

function applyBoolean() {
  run(() => {
    if (!selected.value || !booleanTarget.value || booleanTarget.value === selection.value) {
      throw new Error(label('Выберите два разных объекта.', 'Pick two different objects.'))
    }
    const d = history.document
    const a = d.objects.find(o => o.id === selection.value)!
    const b = d.objects.find(o => o.id === booleanTarget.value)
    if (!b) throw new Error(label('Второй объект не найден.', 'Second object missing.'))
    const mesh = booleanMeshObjects(a.mesh, b.mesh, booleanOp.value)
    a.mesh = mesh
    d.objects = d.objects.filter(o => o.id !== b.id)
    commit(d)
    booleanTarget.value = ''
  })
}

function removeSelected() {
  run(() => {
    if (!selection.value) return
    const d = history.document
    d.objects = d.objects.filter(o => o.id !== selection.value)
    commit(d)
    selection.value = ''
  })
}

async function importMesh(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  try {
    const imported = await importMeshFromFile(file, { weld: 1e-4 })
    const mesh: PolygonMesh = { positions: imported.positions.slice(), indices: imported.indices.slice() }
    const d = history.document
    const id = `${imported.format}-${Date.now().toString(36)}`
    d.objects.push({ id, name: stripMeshExtension(file.name) || imported.format.toUpperCase(), mesh, visible: true })
    commit(d)
    selection.value = id
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    input.value = ''
  }
}

function downloadStl() {
  run(() => {
    if (!selected.value) throw new Error(label('Выберите объект.', 'Select an object.'))
    const blob = new Blob([exportMeshObjectStl(selected.value.mesh)], { type: 'model/stl' })
    const url = URL.createObjectURL(blob)
    const a = window.document.createElement('a')
    a.href = url
    a.download = `${selected.value.name || 'mesh'}.stl`
    a.click()
    URL.revokeObjectURL(url)
  })
}

const exportFormat = ref<MeshExportFormat>('3mf')
async function downloadMesh() {
  try {
    if (!selected.value) throw new Error(label('Выберите объект.', 'Select an object.'))
    const artifact = await exportMeshFormatCompressed(polygonMeshToExportMesh(selected.value.mesh), exportFormat.value)
    downloadBytes(artifact.data, artifact.mimeType, `${selected.value.name || 'mesh'}.${artifact.extension}`)
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  }
}

// Command palette: header actions, object tools and edit operations, filtered to what applies to the current selection.
const paletteOpen = ref(false)
const dockOpen = ref(storageGet('scad-mesh-dock') !== 'false')
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
defineExpose({ meshCommands, executeMeshCommand })
function onWorkspaceKey(event: KeyboardEvent) {
  if (paletteOpen.value) return
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') { event.preventDefault(); paletteOpen.value = true; return }
  if (event.key === 'Escape') { if (elementDrag) { cancelElementDrag(); return } if (propsOpen.value) { propsOpen.value = false; return } emit('close') }
  if ((event.target as HTMLElement).matches('input,textarea,select')) return
  const modes: Record<string, MeshSelectMode> = { '1': 'object', '2': 'vertex', '3': 'edge', '4': 'face' }
  if (modes[event.key]) { selectMode.value = modes[event.key]; return }
  if (event.key.toLowerCase() === 'f') resetView()
}
function downloadJson() {
  const blob = new Blob([stringifyMeshJson(document.value)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = window.document.createElement('a')
  a.href = url
  a.download = 'mesh-workspace.json'
  a.click()
  URL.revokeObjectURL(url)
}

function togglePick(kind: 'vertex' | 'face' | 'edge', id: number) {
  const list = kind === 'vertex' ? selectedVerts : kind === 'face' ? selectedFaces : selectedEdges
  const set = new Set(list.value)
  set.has(id) ? set.delete(id) : set.add(id)
  list.value = [...set]
}

function projected(objectId: string) {
  const object = document.value.objects.find(o => o.id === objectId)
  if (!object || !object.visible) return { tris: [] as Array<{ points: string; face: number }>, verts: [] as Array<{ x: number; y: number; id: number }>, edges: [] as Array<{ x1: number; y1: number; x2: number; y2: number; id: number }> }
  const cam = camera.value
  const tris: Array<{ points: string; face: number }> = []
  for (let f = 0; f < object.mesh.indices.length / 3; f++) {
    const pts = [0, 1, 2].map(k => {
      const i = object.mesh.indices[f * 3 + k]
      return projectDirectPoint([object.mesh.positions[i * 3], object.mesh.positions[i * 3 + 1], object.mesh.positions[i * 3 + 2]], cam)
    })
    tris.push({ points: pts.map(p => `${p[0]},${-p[1]}`).join(' '), face: f })
  }
  const verts: Array<{ x: number; y: number; id: number }> = []
  if (selectMode.value === 'vertex') {
    for (let i = 0; i < object.mesh.positions.length / 3; i++) {
      const p = projectDirectPoint([object.mesh.positions[i * 3], object.mesh.positions[i * 3 + 1], object.mesh.positions[i * 3 + 2]], cam)
      verts.push({ x: p[0], y: -p[1], id: i })
    }
  }
  const edges = selectMode.value === 'edge' ? buildEdgeList(object.mesh).map(([a, b], id) => {
    const point = (vertex: number) => projectDirectPoint([
      object.mesh.positions[vertex * 3],
      object.mesh.positions[vertex * 3 + 1],
      object.mesh.positions[vertex * 3 + 2],
    ], cam)
    const pa = point(a), pb = point(b)
    return { id, x1: pa[0], y1: -pa[1], x2: pb[0], y2: -pb[1] }
  }) : []
  return { tris, verts, edges }
}

// Dragging a vertex, edge or face moves the current selection in the screen plane (LMB, no modifiers);
// the working copy previews on every move and one history step is committed on release.
let elementDrag: { objectId: string; vertexIds: number[]; base: PolygonMesh; x: number; y: number; moved: boolean; svg: SVGSVGElement } | null = null
function screenDeltaToWorld(dxPx: number, dyPx: number, svg: SVGSVGElement): [number, number, number] {
  const unit = 200 / Math.max(1, Math.min(svg.clientWidth, svg.clientHeight)) / (view.value / 100)
  const sx = dxPx * unit, sy = -dyPx * unit
  // Inverse of projectDirectPoint's rotation applied to the screen-plane vector (sx, sy, 0).
  const cam = camera.value, cy = Math.cos(cam.yaw), sy0 = Math.sin(cam.yaw), cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch)
  return [sx * cy + sy * sy0 * sp, -sx * sy0 + sy * cy * sp, -sy * cp]
}
function startElementDrag(event: PointerEvent, objectId: string, kind: 'vertex' | 'edge' | 'face', id: number) {
  if (event.button !== 0 || event.shiftKey || event.altKey || selectMode.value === 'object') { onPointerDown(event); return }
  if (selectMode.value !== kind) { onPointerDown(event); return }
  event.stopPropagation()
  const object = document.value.objects.find(o => o.id === objectId)
  if (!object) return
  if (selection.value !== objectId) { selection.value = objectId; selectedVerts.value = []; selectedFaces.value = []; selectedEdges.value = [] }
  const list = kind === 'vertex' ? selectedVerts : kind === 'face' ? selectedFaces : selectedEdges
  if (!list.value.includes(id)) { list.value = [...list.value, id] }
  const mesh = object.mesh
  const vertexIds = kind === 'vertex' ? [...selectedVerts.value]
    : kind === 'edge' ? [...new Set(selectedEdges.value.flatMap(edge => buildEdgeList(mesh)[edge] ?? []))]
      : [...new Set(selectedFaces.value.flatMap(face => [mesh.indices[face * 3], mesh.indices[face * 3 + 1], mesh.indices[face * 3 + 2]]))]
  const target = event.currentTarget as SVGGraphicsElement
  const svg = target.ownerSVGElement ?? (target as unknown as SVGSVGElement)
  // Meshes may carry per-face duplicates of a corner; move every index that shares the position.
  const byPosition = new Map<string, number[]>()
  const keyOf = (i: number) => `${mesh.positions[i * 3].toFixed(5)},${mesh.positions[i * 3 + 1].toFixed(5)},${mesh.positions[i * 3 + 2].toFixed(5)}`
  for (let i = 0; i < mesh.positions.length / 3; i++) { const key = keyOf(i); const list = byPosition.get(key); if (list) list.push(i); else byPosition.set(key, [i]) }
  const expanded = [...new Set(vertexIds.flatMap(i => byPosition.get(keyOf(i)) ?? [i]))]
  elementDrag = { objectId, vertexIds: expanded, base: { positions: mesh.positions.slice(), indices: mesh.indices.slice() }, x: event.clientX, y: event.clientY, moved: false, svg }
  dragSwallowsClick = false
  ;(event.currentTarget as Element).setPointerCapture(event.pointerId)
}
function moveElementDrag(event: PointerEvent) {
  const drag = elementDrag
  if (!drag) return
  const dxPx = event.clientX - drag.x, dyPx = event.clientY - drag.y
  if (!drag.moved && Math.hypot(dxPx, dyPx) < 3) return
  drag.moved = true
  dragSwallowsClick = true
  const delta = screenDeltaToWorld(dxPx, dyPx, drag.svg)
  try {
    const mesh = selectMode.value === 'vertex' && proportionalEdit.value
      ? moveVerticesProportional(drag.base, drag.vertexIds, delta, proportionalRadius.value)
      : moveVertices(drag.base, drag.vertexIds, delta)
    const current = history.document
    document.value = { ...current, objects: current.objects.map(o => o.id === drag.objectId ? { ...o, mesh } : o) }
  } catch (e) { error.value = e instanceof Error ? e.message : String(e) }
}
function endElementDrag() {
  const drag = elementDrag
  if (!drag) return
  elementDrag = null
  if (!drag.moved) return
  const preview = document.value
  document.value = history.document
  run(() => commit(preview))
}
function cancelElementDrag() {
  if (!elementDrag) return
  elementDrag = null
  document.value = history.document
}
function onPointerDown(event: PointerEvent) {
  if (event.button > 2) return
  dragSwallowsClick = false
  orbit = {
    x: event.clientX, y: event.clientY, yaw: camera.value.yaw, pitch: camera.value.pitch,
    pan: event.button === 1 || event.shiftKey, center: [center.value[0], center.value[1]], moved: false,
  }
  ;(event.currentTarget as Element).setPointerCapture(event.pointerId)
}
function onPointerMove(event: PointerEvent) {
  if (elementDrag) { moveElementDrag(event); return }
  if (!orbit) return
  const dx = event.clientX - orbit.x, dy = event.clientY - orbit.y
  if (!orbit.moved && Math.hypot(dx, dy) < 4) return
  orbit.moved = true
  dragSwallowsClick = true
  if (orbit.pan) {
    const svg = event.currentTarget as SVGSVGElement
    const unit = 200 / Math.max(1, Math.min(svg.clientWidth, svg.clientHeight))
    center.value = [orbit.center[0] + dx * unit, orbit.center[1] + dy * unit]
    return
  }
  camera.value = {
    ...camera.value,
    yaw: orbit.yaw + dx * 0.01,
    pitch: Math.max(-1.4, Math.min(1.4, orbit.pitch + dy * 0.01)),
  }
}
function onPointerUp() { orbit = null; endElementDrag() }
function onClickCapture(event: MouseEvent) {
  if (!dragSwallowsClick) return
  dragSwallowsClick = false
  event.stopPropagation()
  event.preventDefault()
}
function resetView() { camera.value = defaultDirectCamera(); view.value = 140; center.value = [0, 0] }
function onWheel(event: WheelEvent) {
  event.preventDefault()
  view.value = Math.max(20, Math.min(800, view.value * (event.deltaY > 0 ? 0.9 : 1.1)))
}

const scene = computed(() => document.value.objects.filter(o => o.visible).map(o => ({ id: o.id, ...projected(o.id) })))
</script>

<template>
  <div v-if="open" ref="workspace" class="mesh-workspace" tabindex="0" @keydown="onWorkspaceKey">
    <header class="mesh-bar">
      <strong>{{ label('Mesh', 'Mesh') }}</strong>
      <button type="button" class="command-search" :title="label('Поиск команд · Ctrl/⌘ K', 'Search commands · Ctrl/⌘ K')" @click="paletteOpen = true"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg><span>{{ label('Команда…', 'Command…') }}</span><kbd>Ctrl K</kbd></button>
      <span class="hint"></span>
      <button type="button" class="icon" :disabled="!undoable" :title="label('Отменить · Ctrl/⌘ Z', 'Undo · Ctrl/⌘ Z')" :aria-label="label('Отменить', 'Undo')" @click="undo"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 14 4 9l5-5"/><path d="M4 9h11a5 5 0 0 1 0 10h-3"/></svg></button>
      <details class="file-menu"><summary :title="label('Файл', 'File')"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" aria-hidden="true"><path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><path d="M14 3v6h6"/></svg><span>{{ label('Файл', 'File') }}</span><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg></summary><div>
        <button type="button" @click="fileInput?.click()">{{ label('Импорт STL / OBJ / PLY / OFF / AMF / 3MF', 'Import STL / OBJ / PLY / OFF / AMF / 3MF') }}</button>
        <button type="button" :disabled="!selected" @click="downloadStl">{{ label('Скачать STL выбранного', 'Download selected as STL') }}</button>
        <label class="file-row"><select v-model="exportFormat" :aria-label="label('Формат экспорта', 'Export format')"><option v-for="format in MESH_EXPORT_FORMATS" :key="format" :value="format">{{ MESH_FORMAT_LABELS[format] }}</option></select><button type="button" :disabled="!selected" @click="downloadMesh">{{ label('Скачать', 'Download') }}</button></label>
        <button type="button" @click="downloadJson">{{ label('Скачать проект JSON', 'Download JSON project') }}</button>
        <button type="button" :disabled="!document.objects.length" @click="emit('exportToSolid', document)">{{ label('Открыть в Solid', 'Open in Solid') }}</button>
      </div></details>
      <button type="button" class="icon" :disabled="!redoable" :title="label('Повторить · Ctrl/⌘ Shift Z', 'Redo · Ctrl/⌘ Shift Z')" :aria-label="label('Повторить', 'Redo')" @click="redo"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m15 14 5-5-5-5"/><path d="M20 9H9a5 5 0 0 0 0 10h3"/></svg></button>
    </header>
    <CommandPalette v-if="paletteOpen" :open="paletteOpen" :commands="meshCommands.filter(command => command.enabled !== false)" @close="paletteOpen = false" @execute="executeMeshCommand" />

    <div class="mesh-body">
      <input ref="fileInput" type="file" :accept="MESH_IMPORT_ACCEPT" hidden @change="importMesh" />
      <div class="pane-tools" role="toolbar" :aria-label="label('Инструменты Mesh', 'Mesh tools')">
        <button type="button" class="tool-icon" :title="label('Куб', 'Cube')" :aria-label="label('Куб', 'Cube')" @click="addPrimitive('box')"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7M12 11v10"/></svg></button>
        <button type="button" class="tool-icon" :title="label('UV-сфера', 'UV Sphere')" :aria-label="label('UV-сфера', 'UV Sphere')" @click="addPrimitive('sphere')"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM3 12h18M12 3c-3 2.5-3 15.5 0 18M12 3c3 2.5 3 15.5 0 18"/></svg></button>
        <button type="button" class="tool-icon" :title="label('Импорт сетки', 'Import mesh')" :aria-label="label('Импорт сетки', 'Import mesh')" @click="fileInput?.click()"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 21V9M6 15l6-6 6 6M4 5h16"/></svg></button>
        <span class="tool-divider" aria-hidden="true"></span>
        <button type="button" class="tool-icon" :title="label('Объект · 1', 'Object · 1')" :aria-label="label('Объект · 1', 'Object · 1')" :aria-pressed="selectMode==='object'" @click="selectMode='object'"><svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" fill-opacity="0.35" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7M12 11v10"/></svg></button>
        <button type="button" class="tool-icon" :title="label('Вершина · 2', 'Vertex · 2')" :aria-label="label('Вершина · 2', 'Vertex · 2')" :aria-pressed="selectMode==='vertex'" @click="selectMode='vertex'"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7"/><path d="M4 4.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5zM20 4.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5zM12 8.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5z" fill="currentColor"/></svg></button>
        <button type="button" class="tool-icon" :title="label('Ребро · 3', 'Edge · 3')" :aria-label="label('Ребро · 3', 'Edge · 3')" :aria-pressed="selectMode==='edge'" @click="selectMode='edge'"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7"/><path d="M12 11v10" stroke-width="3.5"/></svg></button>
        <button type="button" class="tool-icon" :title="label('Грань · 4', 'Face · 4')" :aria-label="label('Грань · 4', 'Face · 4')" :aria-pressed="selectMode==='face'" @click="selectMode='face'"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4z" fill="currentColor" fill-opacity="0.45"/><path d="M4 7v10l8 4 8-4V7M12 11v10"/></svg></button>
        <span class="tool-divider" aria-hidden="true"></span>
        <button type="button" class="tool-icon" :title="label('Сбросить вид', 'Reset view')" :aria-label="label('Сбросить вид', 'Reset view')" @click="resetView()"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 12a9 9 0 1 0 3-6.7M3 4v5h5"/></svg></button>
        <button type="button" class="tool-icon" :title="label('Сетка', 'Grid')" :aria-label="label('Сетка', 'Grid')" :aria-pressed="floorVisible" @click="floorVisible = !floorVisible"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 9h18M3 15h18M9 3v18M15 3v18M3 3h18v18H3z"/></svg></button>
        <span class="tool-divider" aria-hidden="true"></span>
        <button type="button" class="tool-icon" :title="label('Панель свойств', 'Properties panel')" :aria-label="label('Панель свойств', 'Properties panel')" :aria-pressed="propsOpen" @click="propsOpen = !propsOpen"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 8h10M18 8h2M4 16h4M12 16h8"/><circle cx="16" cy="8" r="2"/><circle cx="10" cy="16" r="2"/></svg></button>
        <span class="subtle">{{ stats ? `${stats.vertices}v · ${stats.edges}e · ${stats.faces}f · ${stats.closed ? label('замкнут', 'closed') : label('открыт', 'open')}` : label('ЛКМ: вращение · Shift: панорама · колесо: масштаб', 'LMB: orbit · Shift: pan · wheel: zoom') }}</span>
        <p v-if="error" class="error" role="alert">{{ error }}</p>
      </div>
      <div class="mesh-stage">
      <div class="mesh-stage-view">
      <div class="mesh-view-wrap">
      <div v-if="!scene.length" class="mesh-empty"><strong>{{ label('Объектов пока нет', 'No objects yet') }}</strong><span>{{ label('Добавьте куб или сферу, импортируйте сетку или соберите модель в Code: сцена переносится сюда автоматически.', 'Add a cube or sphere, import a mesh, or build a model in Code: the scene carries over here automatically.') }}</span></div>
      <svg
        class="mesh-view"
        viewBox="-100 -100 200 200"
        @pointerdown="onPointerDown"
        @pointermove="onPointerMove"
        @pointerup="onPointerUp"
        @pointercancel="onPointerUp"
        @click.capture="onClickCapture"
        @contextmenu.prevent
        @wheel.prevent="onWheel"
      >
        <g :transform="`translate(${center[0]},${center[1]}) scale(${view / 100})`">
          <g v-if="floorVisible" pointer-events="none"><polyline v-for="(line, i) in floorLines" :key="i" :points="line" fill="none" stroke="var(--border)" stroke-opacity=".45" stroke-width=".5" vector-effect="non-scaling-stroke" /></g>
          <g v-for="object in scene" :key="object.id" :opacity="object.id === selection ? 1 : 0.55">
            <polygon
              v-for="tri in object.tris"
              :key="`${object.id}-${tri.face}`"
              :points="tri.points"
              :class="{ face: true, selected: object.id === selection && selectedFaces.includes(tri.face) }"
              @pointerdown="startElementDrag($event, object.id, 'face', tri.face)"
              @click.stop="selection = object.id; selectMode === 'face' && togglePick('face', tri.face)"
            />
            <line
              v-for="edge in object.edges"
              :key="`${object.id}-e-${edge.id}`"
              :x1="edge.x1"
              :y1="edge.y1"
              :x2="edge.x2"
              :y2="edge.y2"
              :class="{ edge: true, selected: object.id === selection && selectedEdges.includes(edge.id) }"
              @pointerdown="startElementDrag($event, object.id, 'edge', edge.id)"
              @click.stop="selection = object.id; togglePick('edge', edge.id)"
            />
            <circle
              v-for="vert in object.verts"
              :key="`${object.id}-v-${vert.id}`"
              :cx="vert.x"
              :cy="vert.y"
              r="0.6"
              :class="{ vert: true, selected: selectedVerts.includes(vert.id) }"
              @pointerdown="startElementDrag($event, object.id, 'vertex', vert.id)"
              @click.stop="selection = object.id; togglePick('vertex', vert.id)"
            />
          </g>
        </g>
      </svg>
      </div>
      </div>
      </div>
      <aside class="side-dock" :class="{ collapsed: !dockOpen }" :aria-label="label('Панель', 'Panel')">
        <div class="dock-rail" role="tablist" aria-orientation="vertical">
          <button type="button" role="tab" :aria-selected="dockOpen && dockTab === 'scene'" :aria-label="label('Сцена', 'Scene')" :title="label('Сцена', 'Scene')" @click="dockTab = 'scene'; dockOpen = true"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h16"/></svg></button>
          <button type="button" role="tab" :aria-selected="dockOpen && dockTab === 'props'" :aria-label="label('Свойства', 'Properties')" :title="label('Свойства', 'Properties')" @click="dockTab = 'props'; dockOpen = true"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M4 8h10M18 8h2M4 16h4M12 16h8M16 6a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM10 14a2 2 0 1 0 0 4 2 2 0 0 0 0-4z"/></svg></button>
          <span class="dock-rail-spacer"></span>
          <button type="button" :aria-label="dockOpen ? label('Свернуть панель', 'Collapse panel') : label('Развернуть панель', 'Expand panel')" :title="dockOpen ? label('Свернуть панель', 'Collapse panel') : label('Развернуть панель', 'Expand panel')" @click="dockOpen = !dockOpen"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path :d="dockOpen ? 'M9 6l6 6-6 6' : 'M15 6l-6 6 6 6'"/></svg></button>
        </div>
        <div v-if="dockOpen" class="dock-body">
          <template v-if="dockTab === 'scene'">
            <div class="dock-heading">{{ label('Сцена', 'Scene') }} <span>{{ document.objects.length }}</span></div>
            <ul class="scene-list">
              <li v-for="object in document.objects" :key="object.id"><button type="button" :aria-label="object.name" :aria-pressed="selection === object.id" @click="selection = object.id; selectedVerts = []; selectedFaces = []; selectedEdges = []"><span class="dot body"></span>{{ object.name }}<small>{{ object.mesh.indices.length / 3 }} △</small></button></li>
              <li v-if="!document.objects.length" class="scene-empty">{{ label('Объектов нет. Добавьте куб или сферу, импортируйте сетку или соберите модель в Code.', 'No objects. Add a cube or sphere, import a mesh, or build a model in Code.') }}</li>
            </ul>
          </template>
          <template v-else>
            <div class="dock-heading">{{ selected?.name || label('Свойства', 'Properties') }}</div>
            <div class="dock-props">
        <section v-if="selected">
          <h3>{{ label('Трансформ', 'Transform') }} G / R / S</h3>
          <label>ΔX <input v-model.number="dx" type="number" step="0.1" /></label>
          <label>ΔY <input v-model.number="dy" type="number" step="0.1" /></label>
          <label>ΔZ <input v-model.number="dz" type="number" step="0.1" /></label>
          <label>{{ label('Угол', 'Angle') }} <input v-model.number="angle" type="number" step="1" /></label>
          <label>{{ label('Масштаб', 'Scale') }} <input v-model.number="scale" type="number" step="0.05" min="0.01" /></label>
          <template v-if="selectMode === 'vertex'">
            <label><span>O · {{ label('Пропорционально', 'Proportional') }}</span><input v-model="proportionalEdit" type="checkbox" /></label>
            <label v-if="proportionalEdit">{{ label('Радиус влияния', 'Influence radius') }} <input v-model.number="proportionalRadius" type="number" step="0.5" min="0.01" /></label>
            <p v-if="proportionalEdit" class="tool-hint">{{ label('Плавный спад по расстоянию в пространстве.', 'Smooth falloff by spatial distance.') }}</p>
          </template>
          <button type="button" @click="transformSelected">{{ label('Применить', 'Apply') }}</button>
        </section>
        <section v-if="selected && selectMode === 'face'">
          <h3>Edit · Face</h3>
          <label>E {{ label('Выдавить', 'Extrude') }} <input v-model.number="extrudeDistance" type="number" step="0.1" /></label>
          <button type="button" @click="applyExtrude">Extrude</button>
          <label>I {{ label('Inset', 'Inset') }} <input v-model.number="insetAmount" type="number" step="0.05" min="0.01" /></label>
          <button type="button" @click="applyInset">Inset</button>
          <button type="button" @click="applySubdivide">Subdivide</button>
          <button type="button" @click="applyDeleteFaces">Delete faces</button>
          <button type="button" @click="applyFlip">Flip normals</button>
        </section>
        <section v-if="selected && selectMode === 'edge'">
          <h3>Edit · Edge</h3>
          <p class="tool-hint">{{ label('Клик выбирает рёбра; Shift вращает вид.', 'Click edges to select; Shift orbits the view.') }}</p>
          <button type="button" :disabled="!selectedEdges.length" @click="applyKnife">K · {{ label('Разрез по середине', 'Knife at midpoint') }}</button>
        </section>
        <section v-if="selected">
          <h3>{{ label('Топология', 'Topology') }}</h3>
          <label>{{ label('Слияние', 'Merge by distance') }} <input v-model.number="mergeDistance" type="number" step="0.001" min="0.0001" /></label>
          <button type="button" @click="applyMerge">Merge</button>
          <button type="button" @click="applySeparate">Separate faces</button>
          <button type="button" @click="applyJoin">Join objects</button>
          <button type="button" @click="applySymmetrize">Symmetrize X</button>
          <h3>{{ label('Скульптинг', 'Sculpt') }}</h3>
          <label>{{ label('Кисть', 'Brush') }}
            <select v-model="brushKind">
              <option v-for="k in SCULPT_KINDS" :key="k" :value="k">{{ k }}</option>
            </select>
          </label>
          <label>Falloff
            <select v-model="brushFalloff">
              <option v-for="f in SCULPT_FALLOFFS" :key="f" :value="f">{{ f }}</option>
            </select>
          </label>
          <label>R <input v-model.number="brushRadius" type="number" step="0.5" min="0.1" /></label>
          <label>{{ brushIsFraction ? label('Доля', 'Amount') : label('Сила', 'Strength') }}
            <input v-model.number="brushStrength" type="number" :step="brushIsFraction ? 0.05 : 0.1" :min="brushIsFraction ? 0 : undefined" :max="brushIsFraction ? 1 : undefined" />
          </label>
          <label title="Mirror X"><input v-model="brushMirror[0]" type="checkbox" /> X</label>
          <label title="Mirror Y"><input v-model="brushMirror[1]" type="checkbox" /> Y</label>
          <label title="Mirror Z"><input v-model="brushMirror[2]" type="checkbox" /> Z</label>
          <button type="button" @click="applyBrush">{{ label('Мазок', 'Stroke') }}</button>
          <label>Twist <input v-model.number="twistAmount" type="number" step="0.05" /></label>
          <button type="button" @click="applyTwist">Twist deform</button>
          <button type="button" @click="applySubdivide">Subdivide all</button>
          <button type="button" @click="applyDuplicate">Duplicate</button>
          <button type="button" @click="removeSelected">Delete object</button>
        </section>
        <section v-if="document.objects.length > 1">
          <h3>Boolean</h3>
          <select v-model="booleanOp">
            <option value="union">Union</option>
            <option value="difference">Difference</option>
            <option value="intersection">Intersection</option>
          </select>
          <select v-model="booleanTarget">
            <option value="">{{ label('Второй объект…', 'Second object…') }}</option>
            <option v-for="object in document.objects.filter(o => o.id !== selection)" :key="object.id" :value="object.id">{{ object.name }}</option>
          </select>
          <button type="button" @click="applyBoolean">{{ label('Выполнить', 'Run') }}</button>
        </section>
        <p v-if="!selected" class="tool-hint">{{ label('Выберите объект в полосе снизу или кликом по холсту.', 'Select an object in the strip below or by clicking the canvas.') }}</p>
            </div>
          </template>
        </div>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.mesh-workspace {
  user-select: none;
  position: fixed;
  inset: 46px 0 28px;
  z-index: 20;
  display: flex;
  flex-direction: column;
  background: var(--bg);
  color: var(--text);
  font-size: 13px;
}
@media (max-width: 750px) { .mesh-workspace { inset: 0; } }
.mesh-bar {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
}
.mesh-bar .hint { margin-right: auto; }
.mesh-bar .command-search { display: inline-flex; align-items: center; gap: 8px; min-width: 190px; background: var(--bg); color: var(--text-dim); border-radius: 8px; padding: 6px 10px; }
.mesh-bar .command-search span { flex: 1; text-align: left; }
.mesh-bar .command-search kbd { font: 11px var(--font-mono, monospace); padding: 1px 5px; border: 1px solid var(--border); border-radius: 4px; }
.mesh-bar .icon { width: 32px; height: 32px; padding: 0; display: inline-flex; align-items: center; justify-content: center; }
.mesh-bar button, .dock-props button, .dock-props select, .dock-props input {
  background: var(--surface-raised);
  color: inherit;
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.3rem 0.55rem;
  font: inherit;
  cursor: pointer;
}
.mesh-bar button:hover:not(:disabled), .dock-props button:hover:not(:disabled) { background: var(--hover); }
.mesh-bar button:disabled, .dock-props button:disabled { opacity: .45; cursor: default; }
.mesh-bar button:focus-visible, .dock-props button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
.mesh-bar .close { margin-left: 0.25rem; }
.mesh-body { flex: 1; display: flex; flex-direction: column; min-height: 0; }
.pane-tools { min-height: 46px; padding: 7px 12px; display: flex; gap: 5px; align-items: center; flex-wrap: wrap; border-bottom: 1px solid var(--border); background: var(--surface); }
.pane-tools .subtle { flex: 1; color: var(--text-dim); font-size: 12px; }
.pane-tools .error { flex-basis: 100%; margin: 0; }
.tool-icon { width: 34px; height: 34px; padding: 0; display: inline-flex; align-items: center; justify-content: center; border-radius: 7px; background: var(--surface-raised); border: 1px solid var(--border); color: var(--text); cursor: pointer; }
.tool-icon:hover { background: var(--hover); }
.tool-icon[aria-pressed=true] { border-color: var(--accent); color: var(--accent); }
.tool-divider { width: 1px; height: 22px; background: var(--border); margin: 0 3px; }
.mesh-stage { flex: 1; min-height: 0; display: flex; }
.mesh-stage-view { flex: 1; min-width: 0; position: relative; }
.side-dock { flex: 0 0 344px; display: flex; min-height: 0; border-left: 1px solid var(--border); background: var(--bg); }
.side-dock.collapsed { flex-basis: 46px; }
.dock-rail { flex: 0 0 46px; display: flex; flex-direction: column; align-items: center; gap: 4px; padding: 8px 0; border-right: 1px solid var(--border); }
.dock-rail button { width: 34px; height: 34px; padding: 0; border: 0; border-radius: 8px; background: transparent; color: var(--text-dim); display: flex; align-items: center; justify-content: center; cursor: pointer; }
.dock-rail button:hover { color: var(--text); background: var(--hover); }
.dock-rail button[aria-selected=true] { color: var(--text); background: var(--surface-raised); }
.dock-rail-spacer { flex: 1; }
.dock-body { flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; overflow: auto; }
.dock-heading { height: 42px; flex-shrink: 0; display: flex; align-items: center; gap: 8px; padding: 0 12px; border-bottom: 1px solid var(--border); font-weight: 600; }
.dock-heading span { color: var(--text-dim); font-weight: 400; }
.scene-list { list-style: none; margin: 0; padding: 6px; display: flex; flex-direction: column; gap: 1px; }
.scene-list li > button { width: 100%; display: flex; align-items: center; gap: 8px; height: 32px; padding: 0 8px; border: 0; border-radius: 7px; background: transparent; color: var(--text); text-align: left; font-family: var(--font-mono, monospace); font-size: 12.5px; cursor: pointer; }
.scene-list li > button:hover { background: var(--hover); }
.scene-list li > button[aria-pressed=true] { background: color-mix(in srgb, var(--accent) 16%, var(--bg)); outline: 1px solid var(--accent); outline-offset: -1px; }
.scene-list small { margin-left: auto; font-size: 11px; color: var(--text-dim); font-family: var(--font-ui, sans-serif); }
.dot { width: 8px; height: 8px; border-radius: 2px; background: #c3b7a3; }
.scene-empty { padding: 16px 12px; color: var(--text-dim); font-size: 12px; line-height: 1.5; }
.dock-props { display: flex; flex-direction: column; gap: 0.75rem; padding: 12px 14px; }
@media (max-width: 750px) { .side-dock { display: none; } }
.file-menu { position: relative; }
.file-menu > summary { display: inline-flex; align-items: center; gap: 6px; list-style: none; cursor: pointer; padding: 0.3rem 0.55rem; background: var(--surface-raised); border: 1px solid var(--border); border-radius: 6px; }
.file-menu > summary::-webkit-details-marker { display: none; }
.file-menu > div { position: absolute; right: 0; top: 40px; z-index: 7; width: 270px; display: grid; gap: 6px; padding: 10px; background: var(--surface); border: 1px solid var(--border); border-radius: 8px; box-shadow: 0 8px 30px #0004; }
.file-menu button, .file-menu select { background: var(--surface-raised); border: 1px solid var(--border); border-radius: 6px; color: var(--text); padding: 0.3rem 0.55rem; font: inherit; cursor: pointer; text-align: left; }
.file-row { display: flex; gap: 6px; }
.file-row select { flex: 1; }
.dock-props h3 { margin: 0 0 0.35rem; font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-dim); }
.dock-props label { display: flex; justify-content: space-between; gap: 0.5rem; font-size: 0.85rem; margin: 0.2rem 0; }
.dock-props ul { list-style: none; padding: 0; margin: 0.35rem 0 0; }
.dock-props li button { width: 100%; text-align: left; margin-top: 0.2rem; }
.dock-props button.active, .row button.active { border-color: var(--accent); color: var(--accent); }
.row { display: flex; flex-wrap: wrap; gap: 0.25rem; }
.mesh-view { width: 100%; height: 100%; background: var(--canvas-bg); }
.mesh-view-wrap { position: relative; min-width: 0; min-height: 0; }
.mesh-empty { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; text-align: center; pointer-events: none; color: var(--text-dim); padding: 24px; }
.mesh-empty strong { font-size: 18px; font-weight: 500; }
.mesh-empty span { font-size: 12px; max-width: 300px; }
.mesh-view .face { fill: #3d5a80; stroke: #0b1220; stroke-width: 0.15; cursor: pointer; }
.mesh-view .face.selected { fill: var(--accent); }
.mesh-view .edge { stroke: #8ecaff; stroke-width: 0.8; vector-effect: non-scaling-stroke; cursor: crosshair; }
.mesh-view .edge.selected { stroke: #ffca6a; stroke-width: 2.5; }
.mesh-view .vert { fill: #eee; cursor: grab; }
.mesh-view .vert.selected { fill: #e76f51; }
.tool-hint { margin: 0.2rem 0 0.45rem; font-size: 0.78rem; color: var(--text-dim); }
.stats { font-size: 0.8rem; color: var(--text-dim); }
.error { color: var(--danger); font-size: 0.85rem; }
</style>
