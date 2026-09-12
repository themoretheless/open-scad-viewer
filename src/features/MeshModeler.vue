<script setup lang="ts">
import { computed, nextTick, ref, shallowRef, watch } from 'vue'
import {
  MeshHistory,
  booleanMeshObjects,
  brushDisplace,
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
  mergeByDistance,
  meshObjectStats,
  moveVertices,
  parseMeshDocument,
  separateFaces,
  stlBufferToPolygonMesh,
  subdivideFaces,
  symmetrizeMesh,
  transformMesh,
  twistMesh,
  type MeshSelectMode,
  type MeshWorkspaceDocument,
} from '../services/meshEditing'
import { defaultDirectCamera, projectDirectPoint } from '../services/directModelingTools'
import { storageGet, storageSet } from '../services/safeStorage'

const props = defineProps<{ open: boolean; locale: string }>()
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
const extrudeDistance = ref(2)
const insetAmount = ref(0.5)
const mergeDistance = ref(0.01)
const brushRadius = ref(5)
const brushStrength = ref(1)
const twistAmount = ref(0.1)
const booleanOp = ref<'union' | 'difference' | 'intersection'>('union')
const booleanTarget = ref('')
const camera = ref(defaultDirectCamera())
const view = ref(140)
const center = ref<[number, number]>([0, 0])
const workspace = ref<HTMLElement>()
const fileInput = ref<HTMLInputElement>()
let orbit: { x: number; y: number; yaw: number; pitch: number } | null = null

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
  try { storageSet(storageKey, JSON.stringify(d)) } catch { /* ignore quota */ }
}, { deep: true })

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
    if (selectMode.value === 'vertex' && selectedVerts.value.length) {
      d.objects[index] = {
        ...d.objects[index],
        mesh: moveVertices(d.objects[index].mesh, selectedVerts.value, [dx.value, dy.value, dz.value]),
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
    const count = mesh.positions.length / 3
    const center: [number, number, number] = [0, 0, 0]
    for (let i = 0; i < count; i++) {
      center[0] += mesh.positions[i * 3]
      center[1] += mesh.positions[i * 3 + 1]
      center[2] += mesh.positions[i * 3 + 2]
    }
    center[0] /= count; center[1] /= count; center[2] /= count
    const d = history.document
    const index = d.objects.findIndex(o => o.id === selection.value)
    d.objects[index] = {
      ...d.objects[index],
      mesh: brushDisplace(mesh, center, brushRadius.value, [0, 0, brushStrength.value]),
    }
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

async function importStl(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  try {
    if (file.size > 20_000_000) throw new Error('STL exceeds 20 MB.')
    const mesh = stlBufferToPolygonMesh(await file.arrayBuffer())
    const d = history.document
    const id = `stl-${Date.now().toString(36)}`
    d.objects.push({ id, name: file.name.replace(/\.stl$/i, '') || 'STL', mesh, visible: true })
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

function downloadJson() {
  const blob = new Blob([JSON.stringify(document.value)], { type: 'application/json' })
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
  if (!object || !object.visible) return { tris: [] as Array<{ points: string; face: number }>, verts: [] as Array<{ x: number; y: number; id: number }> }
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
  return { tris, verts }
}

function onPointerDown(event: PointerEvent) {
  if (event.button === 1 || event.button === 2 || event.shiftKey) {
    orbit = { x: event.clientX, y: event.clientY, yaw: camera.value.yaw, pitch: camera.value.pitch }
    ;(event.currentTarget as Element).setPointerCapture(event.pointerId)
  }
}
function onPointerMove(event: PointerEvent) {
  if (!orbit) return
  camera.value = {
    ...camera.value,
    yaw: orbit.yaw + (event.clientX - orbit.x) * 0.01,
    pitch: Math.max(-1.4, Math.min(1.4, orbit.pitch + (event.clientY - orbit.y) * 0.01)),
  }
}
function onPointerUp() { orbit = null }
function onWheel(event: WheelEvent) {
  event.preventDefault()
  view.value = Math.max(20, Math.min(800, view.value * (event.deltaY > 0 ? 0.9 : 1.1)))
}

const scene = computed(() => document.value.objects.filter(o => o.visible).map(o => ({ id: o.id, ...projected(o.id) })))
</script>

<template>
  <div v-if="open" ref="workspace" class="mesh-workspace" tabindex="0" @keydown.escape="emit('close')">
    <header class="mesh-bar">
      <strong>{{ label('Mesh', 'Mesh') }}</strong>
      <span class="hint">{{ label('Редактирование полигонов (Blender-like)', 'Polygon editing (Blender-like)') }}</span>
      <button type="button" :disabled="!undoable" @click="undo">Undo</button>
      <button type="button" :disabled="!redoable" @click="redo">Redo</button>
      <button type="button" @click="emit('exportToSolid', document)">{{ label('В Solid', 'To Solid') }}</button>
      <button type="button" class="close" @click="emit('close')">{{ label('Закрыть', 'Close') }}</button>
    </header>

    <div class="mesh-body">
      <aside class="mesh-side">
        <section>
          <h3>{{ label('Объекты', 'Objects') }}</h3>
          <button type="button" @click="addPrimitive('box')">+ Cube</button>
          <button type="button" @click="addPrimitive('sphere')">+ UV Sphere</button>
          <button type="button" @click="fileInput?.click()">{{ label('Импорт STL', 'Import STL') }}</button>
          <input ref="fileInput" type="file" accept=".stl,model/stl" hidden @change="importStl" />
          <ul>
            <li v-for="object in document.objects" :key="object.id">
              <button type="button" :class="{ active: selection === object.id }" @click="selection = object.id; selectedVerts = []; selectedFaces = []; selectedEdges = []">
                {{ object.name }}
              </button>
            </li>
          </ul>
        </section>

        <section>
          <h3>{{ label('Режим выбора', 'Select mode') }}</h3>
          <div class="row">
            <button v-for="mode in (['object', 'vertex', 'edge', 'face'] as const)" :key="mode" type="button" :class="{ active: selectMode === mode }" @click="selectMode = mode">{{ mode }}</button>
          </div>
        </section>

        <section v-if="selected">
          <h3>{{ label('Трансформ', 'Transform') }} G / R / S</h3>
          <label>ΔX <input v-model.number="dx" type="number" step="0.1" /></label>
          <label>ΔY <input v-model.number="dy" type="number" step="0.1" /></label>
          <label>ΔZ <input v-model.number="dz" type="number" step="0.1" /></label>
          <label>{{ label('Угол', 'Angle') }} <input v-model.number="angle" type="number" step="1" /></label>
          <label>{{ label('Масштаб', 'Scale') }} <input v-model.number="scale" type="number" step="0.05" min="0.01" /></label>
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

        <section v-if="selected">
          <h3>{{ label('Топология', 'Topology') }}</h3>
          <label>{{ label('Слияние', 'Merge by distance') }} <input v-model.number="mergeDistance" type="number" step="0.001" min="0.0001" /></label>
          <button type="button" @click="applyMerge">Merge</button>
          <button type="button" @click="applySeparate">Separate faces</button>
          <button type="button" @click="applyJoin">Join objects</button>
          <button type="button" @click="applySymmetrize">Symmetrize X</button>
          <label>Brush R <input v-model.number="brushRadius" type="number" step="0.5" min="0.1" /></label>
          <label>Brush Z <input v-model.number="brushStrength" type="number" step="0.1" /></label>
          <button type="button" @click="applyBrush">Sculpt brush</button>
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

        <section>
          <h3>{{ label('Экспорт', 'Export') }}</h3>
          <button type="button" :disabled="!selected" @click="downloadStl">STL</button>
          <button type="button" @click="downloadJson">JSON</button>
        </section>

        <p v-if="stats" class="stats">{{ stats.vertices }}v · {{ stats.edges }}e · {{ stats.faces }}f · {{ stats.closed ? 'closed' : 'open' }}</p>
        <p v-if="error" class="error">{{ error }}</p>
      </aside>

      <svg
        class="mesh-view"
        viewBox="-100 -100 200 200"
        @pointerdown="onPointerDown"
        @pointermove="onPointerMove"
        @pointerup="onPointerUp"
        @wheel.prevent="onWheel"
      >
        <g :transform="`translate(${center[0]},${center[1]}) scale(${view / 100})`">
          <g v-for="object in scene" :key="object.id" :opacity="object.id === selection ? 1 : 0.55">
            <polygon
              v-for="tri in object.tris"
              :key="`${object.id}-${tri.face}`"
              :points="tri.points"
              :class="{ face: true, selected: object.id === selection && selectedFaces.includes(tri.face) }"
              @click.stop="selection = object.id; selectMode === 'face' && togglePick('face', tri.face)"
            />
            <circle
              v-for="vert in object.verts"
              :key="`${object.id}-v-${vert.id}`"
              :cx="vert.x"
              :cy="vert.y"
              r="0.6"
              :class="{ vert: true, selected: selectedVerts.includes(vert.id) }"
              @click.stop="selection = object.id; togglePick('vertex', vert.id)"
            />
          </g>
        </g>
      </svg>
    </div>
  </div>
</template>

<style scoped>
.mesh-workspace {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: flex;
  flex-direction: column;
  background: #12151a;
  color: #e8eaed;
}
.mesh-bar {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid #2a313c;
}
.mesh-bar .hint { opacity: 0.7; font-size: 0.85rem; margin-right: auto; }
.mesh-bar button, .mesh-side button, .mesh-side select, .mesh-side input {
  background: #1c2330;
  color: inherit;
  border: 1px solid #334;
  border-radius: 4px;
  padding: 0.25rem 0.5rem;
}
.mesh-bar .close { margin-left: 0.25rem; }
.mesh-body { flex: 1; display: grid; grid-template-columns: 280px 1fr; min-height: 0; }
.mesh-side {
  overflow: auto;
  padding: 0.75rem;
  border-right: 1px solid #2a313c;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.mesh-side h3 { margin: 0 0 0.35rem; font-size: 0.85rem; text-transform: uppercase; letter-spacing: 0.04em; opacity: 0.75; }
.mesh-side label { display: flex; justify-content: space-between; gap: 0.5rem; font-size: 0.85rem; margin: 0.2rem 0; }
.mesh-side ul { list-style: none; padding: 0; margin: 0.35rem 0 0; }
.mesh-side li button { width: 100%; text-align: left; margin-top: 0.2rem; }
.mesh-side button.active, .row button.active { outline: 1px solid #6af; }
.row { display: flex; flex-wrap: wrap; gap: 0.25rem; }
.mesh-view { width: 100%; height: 100%; background: radial-gradient(circle at 30% 20%, #1b2433, #0d1016 70%); }
.mesh-view .face { fill: #3d5a80; stroke: #0b1220; stroke-width: 0.15; cursor: pointer; }
.mesh-view .face.selected { fill: #f4a261; }
.mesh-view .vert { fill: #eee; cursor: pointer; }
.mesh-view .vert.selected { fill: #e76f51; }
.stats { font-size: 0.8rem; opacity: 0.75; }
.error { color: #f88; font-size: 0.85rem; }
</style>
