<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, shallowRef, watch } from 'vue'
import { bodyPoints, DirectHistory, directBodiesScad, emptyDirectDocument, extrudeDirectSketch, parseDirectDocument, transformDirectPoints, type DirectDocument, type Point2 } from '../services/directModeling'
import { storageGet, storageSet } from '../services/safeStorage'
import { exportPolygonStl } from '../services/polygonKernel'
import { applyDirectExtrusion, circularDirectCopies, defaultDirectCamera, directExtrusionTool, directFaceShade, projectDirectPoint, snapDirectPoint, unprojectDirectXY } from '../services/directModelingTools'
const props = defineProps<{ open: boolean; locale: string; canAppend: boolean; remainingSource: number }>()
const emit = defineEmits<{ close: []; append: [source: string] }>()
const ru = computed(() => props.locale === 'ru')
const label = (a: string, b: string) => ru.value ? a : b
const key = 'scad-direct-modeler-v1'
const error = ref(''), saveError = ref(false)
let stored = storageGet(key)
let initial = emptyDirectDocument()
try { if (stored) initial = parseDirectDocument(stored) } catch { error.value = 'Saved direct document is invalid. Import a backup to recover.' }
const history = new DirectHistory(initial)
const document = shallowRef(history.document)
const undoable = ref(false), redoable = ref(false)
const selection = ref(''), mode = ref<'2d' | '3d'>('2d'), tool = ref<'select' | 'rectangle' | 'circle' | 'polyline'>('select')
const draft = ref<Point2[]>([]), height = ref(10), dx = ref(0), dy = ref(0), dz = ref(0), angle = ref(0), scale = ref(1)
type Pane = '2d' | '3d'
const workspace = ref<HTMLElement>(), splitArea = ref<HTMLElement>()
const split = ref(50)
const views = ref<Record<Pane, number>>({ '2d': 160, '3d': 160 })
const centers = ref<Record<Pane, Point2>>({ '2d': [0, 0], '3d': [0, 0] })
const panes: Pane[] = ['2d', '3d']
const camera = ref(defaultDirectCamera()), hovered = ref(''), snap = ref(true), grid = ref(1)
const snapMarker = ref<Point2 | null>(null)
const drawMeasure = ref(''), operation = ref<'extrude' | 'array' | null>(null)
const baseZ = ref(0), extrusionMode = ref<'new' | 'union' | 'difference'>('new'), targetBody = ref('')
const copyCount = ref(8), copySweep = ref(360), copyX = ref(0), copyY = ref(0)
const previewBody = shallowRef<ReturnType<typeof directExtrusionTool> | null>(null), previewError = ref('')
const movingBody = ref(false), showHelp = ref(false), floorVisible = ref(true)
let previewTimer: ReturnType<typeof setTimeout> | undefined
onUnmounted(() => { clearTimeout(previewTimer) })
let previousFocus: HTMLElement | null = null
watch(() => props.open, async open => {
  if (open) { previousFocus = window.document.activeElement as HTMLElement; await nextTick(); workspace.value?.focus() }
  else { cancelGesture(); operation.value = null; previousFocus?.focus() }
}, { immediate: true })
const selectedSketch = computed(() => document.value.sketches.find(s => s.id === selection.value))
const selectedBody = computed(() => document.value.bodies.find(s => s.id === selection.value))
function run(action: () => void) { error.value = ''; try { action() } catch (e) { error.value = e instanceof Error ? e.message : String(e) } }
function persist() {
  const current = storageGet(key)
  if (current !== stored) { saveError.value = true; error.value = label('Документ изменён в другой вкладке. Скачайте JSON, чтобы сохранить свои правки.', 'Document changed in another tab. Download JSON to keep your edits.'); return }
  const text = JSON.stringify(document.value)
  saveError.value = !storageSet(key, text)
  if (!saveError.value) stored = text
}
function sync() { document.value = history.document; undoable.value = history.canUndo; redoable.value = history.canRedo; persist() }
function commit(next: DirectDocument) { history.commit(next); sync() }
function undo(redo = false) { operation.value = null; cancelGesture(); redo ? history.redo() : history.undo(); sync() }
function addSketch(points: Point2[], closed: boolean) {
  const d = history.document, id = crypto.randomUUID()
  d.sketches.push({ id, name: label('Эскиз ', 'Sketch ') + (d.sketches.length + 1), points, closed })
  commit(d); selection.value = id
}
function finish(closed: boolean) { run(() => { if (draft.value.length < (closed ? 3 : 2)) return; addSketch(draft.value, closed); draft.value = []; tool.value = 'select' }) }
function beginExtrude() {
  if (!selectedSketch.value?.closed) return
  operation.value = 'extrude'; mode.value = '3d'; previewError.value = ''
  if (!document.value.bodies.some(b => b.id === targetBody.value)) targetBody.value = document.value.bodies[0]?.id ?? ''
  void nextTick(() => { fit('3d') })
}
function extrude() { run(() => {
  if (!selectedSketch.value || previewError.value || !previewBody.value) return
  const id = crypto.randomUUID()
  commit(applyDirectExtrusion(history.document, selectedSketch.value.id, height.value, baseZ.value, extrusionMode.value, targetBody.value, id))
  operation.value = null; mode.value = '3d'; selection.value = extrusionMode.value === 'new' ? id : targetBody.value; fit('3d')
}) }
const copyPreview = computed(() => {
  if (operation.value !== 'array' || !selectedSketch.value) return []
  try { let i = 0; return circularDirectCopies(selectedSketch.value, copyCount.value, [copyX.value, copyY.value], copySweep.value, () => 'preview-' + i++) } catch { return [] }
})
function applyCopies() { run(() => { if (!selectedSketch.value) return; const d = history.document; d.sketches.push(...circularDirectCopies(selectedSketch.value, copyCount.value, [copyX.value, copyY.value], copySweep.value, () => crypto.randomUUID())); commit(d); operation.value = null }) }
function duplicate() { run(() => {
  const d = history.document, id = crypto.randomUUID()
  if (selectedSketch.value) d.sketches.push({ ...selectedSketch.value, id, name: (selectedSketch.value.name + ' · copy').slice(0,100), points: transformDirectPoints(selectedSketch.value.points,[10,10,0],0,1) as Point2[] })
  else if (selectedBody.value) d.bodies.push({ ...selectedBody.value, id, name: (selectedBody.value.name + ' · copy').slice(0,100), mesh: { positions: transformDirectPoints(bodyPoints(selectedBody.value),[10,10,0],0,1).flat(), indices: [...selectedBody.value.mesh.indices] } })
  else return
  commit(d); selection.value = id
}) }
watch([operation, selectedSketch, height, baseZ], () => {
  clearTimeout(previewTimer); previewBody.value = null; previewError.value = ''
  if (operation.value !== 'extrude' || !selectedSketch.value) return
  previewTimer = setTimeout(() => {
    try { previewBody.value = directExtrusionTool(selectedSketch.value!,height.value,baseZ.value) }
    catch (e) { previewError.value = e instanceof Error ? e.message : String(e) }
  }, 60)
})
watch(selection, () => { operation.value = null; previewBody.value = null })
function remove() { run(() => { const d = history.document; d.sketches = d.sketches.filter(s => s.id !== selection.value); d.bodies = d.bodies.filter(b => b.id !== selection.value); commit(d); selection.value = '' }) }
function transform() { run(() => {
  const d = history.document, sketch = d.sketches.find(s => s.id === selection.value), body = d.bodies.find(b => b.id === selection.value)
  if (sketch) sketch.points = transformDirectPoints(sketch.points, [dx.value, dy.value, 0], angle.value, scale.value) as Point2[]
  if (body) body.mesh.positions = transformDirectPoints(bodyPoints(body), [dx.value, dy.value, dz.value], angle.value, scale.value).flat()
  commit(d); dx.value = dy.value = dz.value = angle.value = 0; scale.value = 1
}) }
function appendBodies() {
  run(() => {
    const source = directBodiesScad(document.value)
    if (!props.canAppend || source.length > props.remainingSource) throw new Error(label('Сохраните тела в SCAD: текущий документ не подходит для добавления.', 'Download SCAD: the current document cannot accept these bodies.'))
    emit('append', source); emit('close')
  })
}
function download(text: string, name: string) { const url = URL.createObjectURL(new Blob([text], { type: 'text/plain' })); const a = window.document.createElement('a'); a.href = url; a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(url), 1000) }
async function importFile(event: Event) {
  const input = event.target as HTMLInputElement, file = input.files?.[0]
  try { if (file) { if (file.size > 4_000_000) throw new Error('Document exceeds 4 MB.'); const text = await file.text(); run(() => { cancelGesture(); commit(parseDirectDocument(text)); selection.value = '' }) } }
  catch (e) { error.value = String(e) } finally { input.value = '' }
}
function project(p: number[], pane: Pane): Point2 { return pane === '2d' ? [p[0], -p[1]] : projectDirectPoint(p, camera.value).slice(0,2) as Point2 }
function viewBox(pane: Pane) { const size = views.value[pane], center = centers.value[pane]; return `${center[0] - size / 2} ${center[1] - size / 2} ${size} ${size}` }
function zoom(pane: Pane, factor: number) { views.value[pane] = Math.max(.1, Math.min(2e6, views.value[pane] * factor)) }
function fit(pane: Pane) {
  const points = pane === '2d' ? document.value.sketches.flatMap(s => s.points) : [...document.value.bodies, ...(previewBody.value ? [previewBody.value] : [])].flatMap(bodyPoints)
  if (!points.length) { views.value[pane] = 160; centers.value[pane] = [0, 0]; return }
  const projected = points.map(p => project(p, pane))
  const min = [Infinity, Infinity], max = [-Infinity, -Infinity]
  for (const p of projected) for (let axis = 0; axis < 2; axis++) { min[axis] = Math.min(min[axis], p[axis]); max[axis] = Math.max(max[axis], p[axis]) }
  centers.value[pane] = [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2]
  views.value[pane] = Math.max(10, max[0] - min[0], max[1] - min[1]) * 1.6
}
function resizeSplit(e: PointerEvent) {
  if (e.button !== 0) return
  const target = e.currentTarget as HTMLElement
  target.setPointerCapture(e.pointerId)
}
function moveSplit(e: PointerEvent) {
  if (!(e.currentTarget as HTMLElement).hasPointerCapture(e.pointerId)) return
  const rect = splitArea.value!.getBoundingClientRect()
  split.value = Math.max(25, Math.min(75, (e.clientX - rect.left) / rect.width * 100))
}
function splitKey(e: KeyboardEvent) {
  if (!['ArrowLeft', 'ArrowRight', 'Home'].includes(e.key)) return
  e.preventDefault(); split.value = e.key === 'Home' ? 50 : Math.max(25, Math.min(75, split.value + (e.key === 'ArrowLeft' ? -2 : 2)))
}
function meshPolygons(b: ReturnType<typeof directExtrusionTool>) {
  const points = bodyPoints(b)
  return Array.from({ length: b.mesh.indices.length / 3 }, (_, i) => {
    const face = b.mesh.indices.slice(i * 3, i * 3 + 3).map(j => points[j])
    return { id: b.id, key: b.id + ':' + i, points: face.map(p => project(p, '3d').join(',')).join(' '), shade: directFaceShade(b.mesh,i,camera.value), depth: face.reduce((n,p) => n + projectDirectPoint(p,camera.value)[2], 0) }
  })
}
const polygons = computed(() => document.value.bodies.flatMap(meshPolygons).sort((a,b)=>a.depth-b.depth))
const floorLines = computed(() => {
  const step = Math.pow(10, Math.floor(Math.log10(views.value['3d'] / 8))), extent = step * 20
  return Array.from({length:41},(_,i) => (i-20)*step).flatMap(n => [[[-extent,n,0],[extent,n,0]],[[n,-extent,0],[n,extent,0]]]).map(line => line.map(p=>project(p,'3d').join(',')).join(' '))
})
const ghostPolygons = computed(() => previewBody.value ? meshPolygons(previewBody.value).sort((a,b)=>a.depth-b.depth) : [])
const extrusionHandle = computed(() => {
  if (operation.value !== 'extrude' || !selectedSketch.value) return null
  const p = selectedSketch.value.points, center = [p.reduce((s,v)=>s+v[0],0)/p.length,p.reduce((s,v)=>s+v[1],0)/p.length]
  return { base: project([...center,baseZ.value],'3d'), top: project([...center,baseZ.value+height.value],'3d') }
})
let orbitDrag: { x: number; y: number; yaw: number; pitch: number; pointer: number; svg: SVGSVGElement } | null = null
let heightDrag: { y: number; height: number; pointer: number; svg: SVGSVGElement } | null = null
function dragHeight(e: PointerEvent) {
  e.preventDefault(); const svg = (e.target as SVGElement).ownerSVGElement!
  heightDrag = { y: e.clientY, height: height.value, pointer: e.pointerId, svg }; svg.setPointerCapture(e.pointerId)
}
function canvasOf(e: PointerEvent): SVGSVGElement { const target = e.target as SVGElement; return (target instanceof SVGSVGElement ? target : target.ownerSVGElement)! }
function snapped(p: Point2, e: PointerEvent): Point2 {
  if (!snap.value || e.altKey) { snapMarker.value = null; return p }
  const candidates = document.value.sketches.filter(s=>s.id!==gesture?.id).flatMap(s=>s.points)
  const result = snapDirectPoint(p,candidates,views.value['2d']/100,(Number.isFinite(grid.value) && grid.value > 0 && grid.value <= 1e6) ? grid.value : 0)
  snapMarker.value = result.kind === 'vertex' ? result.point : null
  return result.point
}
function position(e: PointerEvent): Point2 {
  const target = e.target as SVGElement
  const svg = gesture?.svg ?? (target instanceof SVGSVGElement ? target : target.ownerSVGElement)!
  const matrix = svg.getScreenCTM()
  if (!matrix) throw new Error('Canvas is not ready.')
  const point = new DOMPoint(e.clientX, e.clientY).matrixTransform(matrix.inverse())
  return [point.x, point.y]
}
function plane(p: Point2, pane: Pane): Point2 { return pane === '2d' ? [p[0], -p[1]] : unprojectDirectXY(p,camera.value) }
let gesture: { start: Point2; document: DirectDocument; vertex: number | null; id: string; pointer: number; pane: Pane; svg: SVGSVGElement; pan: boolean; center: Point2 } | null = null
function cancelGesture() { if (gesture) { document.value = gesture.document; gesture = null } if (heightDrag) height.value = heightDrag.height; heightDrag = null; orbitDrag = null; draft.value = []; drawMeasure.value = ''; snapMarker.value = null }
function down(e: PointerEvent, pane: Pane, id = '', vertex: number | null = null) {
  if (![0, 1, 2].includes(e.button) || gesture) return
  const pan = e.button === 1 || e.shiftKey || (pane === '2d' && e.button === 2)
  const target = e.target as SVGElement
  const svg = (target instanceof SVGSVGElement ? target : target.ownerSVGElement)!
  svg.focus(); mode.value = pane
  if (pane === '3d' && !pan && (e.button === 2 || (!movingBody.value && e.button === 0))) {
    if (id && !operation.value) selection.value = id
    orbitDrag = { x:e.clientX, y:e.clientY, yaw:camera.value.yaw, pitch:camera.value.pitch, pointer:e.pointerId, svg }; svg.setPointerCapture(e.pointerId); return
  }
  let p: Point2
  try { p = plane(position(e), pane) } catch (e) { error.value = String(e); return }
  if (pane === '2d' && !pan) p = snapped(p,e)
  if (pan) { gesture = { start: position(e), document: history.document, vertex: null, id: '', pointer: e.pointerId, pane, svg, pan: true, center: [...centers.value[pane]] }; svg.setPointerCapture(e.pointerId); return }
  if (pane === '2d' && operation.value) operation.value = null
  if (pane === '2d' && tool.value === 'polyline') { if (draft.value.length >= 3 && Math.hypot(p[0]-draft.value[0][0],p[1]-draft.value[0][1]) < views.value['2d']/100) { finish(true); return } draft.value = [...draft.value, p]; return }
  if (tool.value === 'select' || pane === '3d') { selection.value = id; if (!id) return }
  gesture = { start: p, document: history.document, vertex, id, pointer: e.pointerId, pane, svg, pan: false, center: [...centers.value[pane]] }
  svg.setPointerCapture(e.pointerId)
}
function move(e: PointerEvent) {
  if (heightDrag && heightDrag.pointer === e.pointerId) {
    const factor = views.value['3d'] / Math.min(heightDrag.svg.clientWidth,heightDrag.svg.clientHeight) / Math.max(.05,Math.cos(camera.value.pitch))
    const h = heightDrag.height - (e.clientY-heightDrag.y)*factor
    height.value = Math.abs(h)<.01 ? .01 : Math.round(h*100)/100; return
  }
  if (orbitDrag && orbitDrag.pointer === e.pointerId) {
    camera.value = { yaw: orbitDrag.yaw + (e.clientX-orbitDrag.x)*.007, pitch: Math.max(-1.5,Math.min(1.5,orbitDrag.pitch+(e.clientY-orbitDrag.y)*.007)) }; return
  }
  if (!gesture || gesture.pointer !== e.pointerId) return
  if (gesture.pan) {
    const p = position(e), center = centers.value[gesture.pane]
    centers.value[gesture.pane] = [center[0] + gesture.start[0] - p[0], center[1] + gesture.start[1] - p[1]]
    return
  }
  let p = plane(position(e), gesture.pane); const start = gesture.start
  if (gesture.pane === '2d') p = snapped(p,e)
  if (gesture.pane === '2d' && tool.value !== 'select') {
    draft.value = tool.value === 'rectangle' ? [start, [p[0], start[1]], p, [start[0], p[1]]] : Array.from({ length: 64 }, (_, i) => { const r = Math.hypot(p[0] - start[0], p[1] - start[1]), a = i * Math.PI / 32; return [start[0] + r * Math.cos(a), start[1] + r * Math.sin(a)] as Point2 })
    drawMeasure.value = tool.value === 'circle' ? `R ${Math.hypot(p[0]-start[0],p[1]-start[1]).toFixed(2)} mm` : `${Math.abs(p[0]-start[0]).toFixed(2)} × ${Math.abs(p[1]-start[1]).toFixed(2)} mm`
    return
  }
  const d: DirectDocument = JSON.parse(JSON.stringify(gesture.document)), delta = [p[0] - start[0], p[1] - start[1], 0]
  const sketch = d.sketches.find(s => s.id === gesture!.id), body = d.bodies.find(b => b.id === gesture!.id)
  if (sketch) { if (gesture.vertex !== null) sketch.points[gesture.vertex] = p; else sketch.points = transformDirectPoints(sketch.points, delta, 0, 1) as Point2[] }
  if (body) body.mesh.positions = transformDirectPoints(bodyPoints(body), delta, 0, 1).flat()
  document.value = d
}
function up(e: PointerEvent) {
  if (heightDrag) { heightDrag = null; return }
  if (orbitDrag) { orbitDrag = null; return }
  drawMeasure.value = ''; snapMarker.value = null
  if (!gesture || gesture.pointer !== e.pointerId) return
  move(e)
  const before = gesture.document, pane = gesture.pane, pan = gesture.pan; gesture = null
  if (pan) return
  run(() => {
    if (pane === '2d' && tool.value !== 'select') {
      const points = draft.value
      if (points.length >= 3 && Math.abs(points.reduce((sum, p, i) => { const q = points[(i + 1) % points.length]; return sum + p[0] * q[1] - q[0] * p[1] }, 0)) > 1e-6) addSketch(points, true)
      draft.value = []; tool.value = 'select'
    } else commit(document.value)
  })
  if (error.value && JSON.stringify(history.document) === JSON.stringify(before)) document.value = before
}
function keydown(e: KeyboardEvent) {
  if (e.key === 'Escape') { e.preventDefault(); cancelGesture(); operation.value = null; return }
  if (e.key === 'Enter' && operation.value) { e.preventDefault(); operation.value === 'extrude' ? extrude() : applyCopies(); return }
  if ((e.target as HTMLElement).matches('input,textarea,select')) return
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd') { e.preventDefault(); duplicate(); return }
  if (!e.ctrlKey && !e.metaKey && !e.altKey) {
    const k = e.key.toLowerCase(), tools = { v:'select', r:'rectangle', c:'circle', l:'polyline' } as const
    if (k in tools) { cancelGesture(); tool.value = tools[k as keyof typeof tools]; mode.value = '2d' }
    if (k === 'e') beginExtrude()
    if (k === 'f') fit(mode.value)
    if (k === 'g') movingBody.value = !movingBody.value
    if (k === '?') showHelp.value = !showHelp.value
  }
  if ((e.ctrlKey || e.metaKey) && (e.key.toLowerCase() === 'z' || e.key.toLowerCase() === 'y')) { e.preventDefault(); e.stopPropagation(); undo(e.shiftKey || e.key.toLowerCase() === 'y') }
  if (e.key === 'Delete' || e.key === 'Backspace') { e.preventDefault(); remove() }
  if (e.key === 'Escape' && (gesture || draft.value.length)) { e.preventDefault(); e.stopPropagation(); cancelGesture() }
}

</script>
<template>
  <section v-show="open" ref="workspace" class="direct-workspace" tabindex="-1" :aria-label="label('Прямое моделирование', 'Direct modeling')" @keydown.stop="keydown">
    <header class="workspace-bar">
      <button class="back" @click="emit('close')">← {{ label('Редактор', 'Editor') }}</button>
      <strong>{{ label('Прямое моделирование', 'Direct modeling') }}</strong>
      <div class="history-tools"><button :disabled="!undoable" @click="undo()" :title="label('Отменить · Ctrl/⌘ Z', 'Undo · Ctrl/⌘ Z')">↶</button><button :disabled="!redoable" @click="undo(true)" :title="label('Повторить · Ctrl/⌘ Shift Z', 'Redo · Ctrl/⌘ Shift Z')">↷</button></div>
      <button @click="showHelp = !showHelp" title="Keyboard shortcuts">?</button>
      <span class="save-status" role="status">{{ saveError ? label('Не сохранено', 'Unsaved') : label('Сохранено в браузере', 'Saved in browser') }}</span>
      <details class="file-menu"><summary>{{ label('Файл', 'File') }} ▾</summary><div>
        <button @click="download(JSON.stringify(document), 'direct-model.json')">{{ label('Скачать проект JSON', 'Download JSON project') }}</button>
        <label class="file-open">{{ label('Открыть проект', 'Open project') }}<input type="file" accept=".json" @change="importFile"></label>
        <button :disabled="!document.bodies.length" @click="download(directBodiesScad(document), 'direct-bodies.scad')">{{ label('Экспорт SCAD', 'Export SCAD') }}</button>
        <button :disabled="!document.bodies.length || !canAppend" @click="appendBodies">{{ label('Добавить в основную сцену', 'Add to main scene') }}</button>
      </div></details>
    </header>
    <div ref="splitArea" class="split-workspace" :style="{ '--split': split + '%' }">
      <template v-for="pane in panes" :key="pane">
        <div v-if="pane === '3d'" class="splitter" role="separator" tabindex="0" aria-orientation="vertical" :aria-label="label('Ширина 2D и 3D', '2D and 3D width')" :aria-valuenow="Math.round(split)" :aria-valuemin="25" :aria-valuemax="75" @pointerdown="resizeSplit" @pointermove="moveSplit" @pointerup="($event.currentTarget as HTMLElement).releasePointerCapture($event.pointerId)" @keydown="splitKey" @dblclick="split = 50"><span /></div>
        <section class="pane" :class="{ active: mode === pane }" :aria-label="pane === '2d' ? label('2D — эскизы', '2D — sketches') : label('3D — тела', '3D — bodies')">
          <header class="pane-heading"><strong>{{ pane === '2d' ? label('2D · Эскизы', '2D · Sketches') : label('3D · Тела', '3D · Bodies') }}</strong><span>{{ pane === '2d' ? label('Вид сверху · мм', 'Top view · mm') : label('Орбита · мм', 'Orbit · mm') }}</span><button @click="fit(pane)">{{ label('Вписать', 'Fit') }}</button></header>
          <div class="pane-tools">
            <template v-if="pane === '2d'">
              <button v-for="(name, value) in { select: label('↖ Выбор · V', '↖ Select · V'), rectangle: label('□ Прямоугольник · R', '□ Rectangle · R'), circle: label('○ Круг · C', '○ Circle · C'), polyline: label('⌁ Ломаная · L', '⌁ Polyline · L') }" :key="value" :aria-pressed="tool === value" @click="cancelGesture(); tool = value; mode = '2d'">{{ name }}</button>
              <label class="snap-toggle"><input v-model="snap" type="checkbox">{{ label('Привязка', 'Snap') }}</label><input v-if="snap" class="grid-input" v-model.number="grid" type="number" min=".01" step=".5" :aria-label="label('Шаг сетки', 'Grid step')">
            </template>
            <template v-else><button :aria-pressed="!movingBody" @click="movingBody = false">{{ label('↻ Обзор', '↻ Orbit') }}</button><button :aria-pressed="movingBody" @click="movingBody = true">{{ label('↔ Двигать · G', '↔ Move · G') }}</button><button @click="camera = defaultDirectCamera(); fit('3d')">ISO</button><button :aria-pressed="floorVisible" @click="floorVisible = !floorVisible" :aria-label="label('Сетка 3D', '3D grid')">#</button><span class="subtle">{{ label('ПКМ — вращать · Shift — панорама', 'Right drag to orbit · Shift to pan') }}</span><button :disabled="!selectedBody" @click="run(() => download(exportPolygonStl(selectedBody!.mesh), 'body.stl'))">↓ STL</button></template>
          </div>
          <div class="canvas-wrap">
            <svg :viewBox="viewBox(pane)" tabindex="0" :aria-label="pane === '2d' ? label('Холст эскизов 2D', '2D sketch canvas') : label('Холст тел 3D', '3D body canvas')" @contextmenu.prevent @wheel.prevent="zoom(pane, $event.deltaY > 0 ? 1.1 : 1/1.1)" @pointerdown="down($event, pane)" @pointermove="move" @pointerup="up" @pointercancel="cancelGesture" @lostpointercapture="cancelGesture">
              <defs><pattern :id="'direct-grid-' + pane" width="10" height="10" patternUnits="userSpaceOnUse"><path d="M 10 0 L 0 0 0 10" fill="none" stroke="var(--border)" stroke-opacity=".45" stroke-width=".5" vector-effect="non-scaling-stroke" /></pattern></defs>
              <rect v-if="pane === '2d'" x="-2000000" y="-2000000" width="4000000" height="4000000" :fill="'url(#direct-grid-' + pane + ')'" />
              <g v-if="pane === '3d' && floorVisible" pointer-events="none"><polyline v-for="(line,i) in floorLines" :key="i" :points="line" fill="none" stroke="var(--border)" stroke-opacity=".45" stroke-width=".5" vector-effect="non-scaling-stroke" /></g>
              <path v-if="pane === '2d'" d="M -2000000 0 H 2000000 M 0 -2000000 V 2000000" stroke="var(--border)" vector-effect="non-scaling-stroke" />
              <g v-if="pane === '2d'">
                <g v-for="s in document.sketches" :key="s.id">
                  <path :d="'M ' + s.points.map(p => project(p, '2d').join(',')).join(' L ') + (s.closed ? ' Z' : '')" :class="{ selected: selection === s.id, hovered: hovered === s.id }" @pointerenter="hovered = s.id" @pointerleave="hovered = ''" :fill="selection === s.id && s.closed ? 'var(--accent)' : 'none'" fill-opacity=".08" pointer-events="all" stroke="var(--accent)" stroke-width="2" vector-effect="non-scaling-stroke" @pointerdown.stop="down($event, pane, s.id)" />
                  <template v-if="selection === s.id && tool === 'select'"><circle v-for="(p, i) in s.points" :key="i" :cx="p[0]" :cy="-p[1]" :r="views['2d'] / 150" fill="var(--accent)" @pointerdown.stop="down($event, pane, s.id, i)" /></template>
                </g>
                <path v-for="s in copyPreview" :key="s.id" :d="'M '+s.points.map(p=>project(p,'2d').join(',')).join(' L ')+(s.closed?' Z':'')" fill="none" stroke="#b894ff" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" pointer-events="none" />
                <circle v-if="snapMarker" :cx="snapMarker[0]" :cy="-snapMarker[1]" :r="views['2d']/100" fill="none" stroke="#77eac5" vector-effect="non-scaling-stroke" pointer-events="none" />
                <polyline v-if="draft.length"
 :points="draft.map(p => project(p, '2d').join(',')).join(' ')" fill="none" stroke="var(--accent)" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" pointer-events="none" />
              </g>
              <g v-else><polygon v-for="p in polygons" :key="p.key" :points="p.points" :fill="`hsl(${selection === p.id ? 266 : hovered === p.id ? 190 : 220} 45% ${p.shade}%)`" :stroke="`hsl(${selection === p.id ? 266 : 220} 45% ${p.shade}%)`" stroke-width=".6" @pointerenter="hovered = p.id" @pointerleave="hovered = ''" vector-effect="non-scaling-stroke" @pointerdown.stop="down($event, pane, p.id)" /></g>
              <g v-if="pane === '3d' && operation === 'extrude'" pointer-events="none"><polygon v-for="p in ghostPolygons" :key="p.key" :points="p.points" :fill="extrusionMode === 'difference' ? '#ff647c' : '#75e4b8'" fill-opacity=".28" :stroke="extrusionMode === 'difference' ? '#ff647c' : '#75e4b8'" stroke-width=".7" vector-effect="non-scaling-stroke" /></g>
              <g v-if="pane === '3d' && extrusionHandle" class="height-handle" @pointerdown.stop="dragHeight">
                <line :x1="extrusionHandle.base[0]" :y1="extrusionHandle.base[1]" :x2="extrusionHandle.top[0]" :y2="extrusionHandle.top[1]" stroke="#77eac5" stroke-width="3" vector-effect="non-scaling-stroke" />
                <circle :cx="extrusionHandle.top[0]" :cy="extrusionHandle.top[1]" :r="views['3d']/55" fill="#77eac5" stroke="#18332d" stroke-width="2" vector-effect="non-scaling-stroke" />
                <text :x="extrusionHandle.top[0]+views['3d']/35" :y="extrusionHandle.top[1]" :font-size="views['3d']/45" fill="#77eac5">{{ height.toFixed(2) }} mm</text>
              </g>
            </svg>
            <div v-if="pane === '2d' && drawMeasure" class="live-measure">{{ drawMeasure }}</div>
            <div v-if="pane === '3d' && operation === 'extrude'" class="operation-card">
              <strong>{{ label('Выдавливание', 'Extrusion') }}</strong><small>{{ label('Тяните зелёную ручку или введите размер', 'Drag the green handle or enter a dimension') }}</small>
              <div class="segmented"><button v-for="(name,value) in {new:label('Новое','New'),union:label('Добавить','Add'),difference:label('Вычесть','Cut')}" :key="value" :aria-pressed="extrusionMode === value" @click="extrusionMode = value">{{ name }}</button></div>
              <label>{{ label('Высота, мм', 'Height, mm') }}<input v-model.number="height" type="number" step="1"></label>
              <label>{{ label('Начало Z, мм', 'Start Z, mm') }}<input v-model.number="baseZ" type="number" step="1"></label>
              <label v-if="extrusionMode !== 'new'">{{ label('Тело', 'Body') }}<select v-model="targetBody"><option v-for="b in document.bodies" :key="b.id" :value="b.id">{{ b.name }}</option></select></label>
              <small v-if="previewError" role="alert">{{ previewError }}</small>
              <div><button class="primary" :disabled="!previewBody || !!previewError || (extrusionMode !== 'new' && !targetBody)" @click="extrude">{{ label('Готово · Enter', 'Apply · Enter') }}</button><button @click="operation = null">Esc</button></div>
            </div>
            <div v-if="pane === '2d' && operation === 'array'" class="operation-card">
              <strong>{{ label('Круговые копии', 'Circular copies') }}</strong><small>{{ label('Количество включает исходный эскиз', 'Count includes the original sketch') }}</small>
              <label>{{ label('Количество', 'Count') }}<input v-model.number="copyCount" type="number" min="2" max="64"></label><label>{{ label('Угол, °', 'Angle, °') }}<input v-model.number="copySweep" type="number" min="-360" max="360"></label>
              <label>{{ label('Центр X', 'Center X') }}<input v-model.number="copyX" type="number"></label><label>{{ label('Центр Y', 'Center Y') }}<input v-model.number="copyY" type="number"></label>
              <div><button class="primary" :disabled="!copyPreview.length" @click="applyCopies">{{ label('Готово · Enter', 'Apply · Enter') }}</button><button @click="operation = null">Esc</button></div>
            </div>
            <div v-if="pane === '2d' && !document.sketches.length && !draft.length" class="empty-hint"><strong>{{ label('Начните с контура', 'Start with a contour') }}</strong><span>{{ label('Выберите фигуру сверху и нарисуйте её мышью', 'Choose a tool above and draw with the mouse') }}</span></div>
            <div v-if="pane === '3d' && !document.bodies.length && operation !== 'extrude'" class="empty-hint"><strong>{{ label('Здесь появится объём', 'Your solid appears here') }}</strong><span>{{ label('Выберите эскиз слева и нажмите «Выдавить»', 'Select a sketch on the left and press Extrude') }}</span></div>
            <div class="zoom-tools"><button :aria-label="label('Приблизить ', 'Zoom in ') + pane" @click="zoom(pane, .8)">+</button><button :aria-label="label('Отдалить ', 'Zoom out ') + pane" @click="zoom(pane, 1.25)">−</button></div>
          </div>
          <div class="object-strip"><span>{{ pane === '2d' ? label('Эскизы', 'Sketches') : label('Тела', 'Bodies') }}</span><button v-for="item in (pane === '2d' ? document.sketches : document.bodies)" :key="item.id" :aria-pressed="selection === item.id" @click="selection = item.id; mode = pane">{{ item.name }}</button></div>
        </section>
      </template>
    </div>
    <footer class="context-bar">
      <template v-if="tool === 'polyline' && draft.length"><span>{{ draft.length }} {{ label('точек', 'points') }}</span><button :disabled="draft.length < 3" @click="finish(true)">{{ label('Замкнуть контур', 'Close contour') }}</button><button :disabled="draft.length < 2" @click="finish(false)">{{ label('Завершить линию', 'Finish line') }}</button><button @click="cancelGesture">Esc</button></template>
      <template v-else-if="selectedSketch || selectedBody">
        <strong>{{ selectedSketch?.name || selectedBody?.name }}</strong>
        <button v-if="selectedSketch" class="primary" :disabled="!selectedSketch.closed" @click="beginExtrude">{{ label('Выдавить · E', 'Extrude · E') }}</button>
        <button v-if="selectedSketch" @click="operation = operation === 'array' ? null : 'array'">{{ label('Круговые копии', 'Circular copies') }}</button><button @click="duplicate">{{ label('Копия · ⌘/Ctrl D', 'Duplicate · ⌘/Ctrl D') }}</button>
        <details class="transform-menu"><summary>{{ label('Точные преобразования', 'Exact transforms') }}</summary><div>
          <label>X <input v-model.number="dx" type="number" aria-label="ΔX"></label><label>Y <input v-model.number="dy" type="number" aria-label="ΔY"></label><label v-if="selectedBody">Z <input v-model.number="dz" type="number" aria-label="ΔZ"></label>
          <label>↻ <input v-model.number="angle" type="number" :aria-label="label('Поворот Z', 'Z rotation')">°</label><label>× <input v-model.number="scale" type="number" min=".001" step=".1" :aria-label="label('Масштаб', 'Scale')"></label><button @click="transform">{{ label('Применить', 'Apply') }}</button>
        </div></details><button class="delete" @click="remove">{{ label('Удалить', 'Delete') }}</button>
      </template>
      <span v-else class="subtle">{{ label('R — прямоугольник · C — круг · L — ломаная · V — выбор · E — выдавить · F — вписать · ? — помощь', 'R rectangle · C circle · L polyline · V select · E extrude · F fit · ? help') }}</span>
    </footer>
    <div v-if="showHelp" class="help-card"><strong>{{ label('Управление', 'Controls') }}</strong><p>{{ label('2D: тяните фигуру или вершину. Alt временно отключает привязку. Ломаная замыкается кликом по первой точке.', '2D: drag shapes or vertices. Alt bypasses snapping. Close a polyline by clicking its first point.') }}</p><p>{{ label('3D: тяните для вращения; G включает перемещение тела. ПКМ всегда вращает. Shift или средняя кнопка — панорама. Колесо — масштаб.', '3D: drag to orbit; G enables body movement. Right drag always orbits. Shift or middle drag pans. Wheel zooms.') }}</p><p>{{ label('E — предпросмотр выдавливания; зелёная ручка меняет высоту. Enter подтверждает, Escape отменяет. Ctrl/⌘ Z — отмена, Ctrl/⌘ Shift Z — повтор.', 'E previews extrusion; the green handle changes height. Enter applies, Escape cancels. Ctrl/⌘ Z undoes; Ctrl/⌘ Shift Z redoes.') }}</p><button @click="showHelp = false">{{ label('Понятно', 'Got it') }}</button></div>
    <div v-if="error" class="error-bar" role="alert">{{ error }} <button @click="error = ''">×</button></div>
  </section>
</template>
<style scoped>
.direct-workspace{position:fixed;inset:44px 0 0;z-index:20;display:flex;flex-direction:column;min-height:0;background:var(--bg);color:var(--text);outline:none;font-size:13px}.workspace-bar{display:flex;align-items:center;gap:16px;padding:10px 16px;border-bottom:1px solid var(--border);background:var(--surface)}button,input,summary,.file-open{color:var(--text);background:var(--surface-raised);border:1px solid var(--border);border-radius:5px;padding:7px 10px;font:inherit}button,summary{cursor:pointer}button:disabled{opacity:.4;cursor:default}button:hover:not(:disabled){background:var(--hover)}button:focus-visible,summary:focus-visible{outline:2px solid var(--accent)}[aria-pressed=true]{border-color:var(--accent);color:var(--accent)}.back{background:transparent}.history-tools{display:flex;gap:4px}.history-tools button{font-size:20px;padding:2px 12px}.save-status{margin-left:auto;color:var(--text-dim);font-size:12px}.file-menu{position:relative}.file-menu>div{position:absolute;right:0;top:40px;z-index:5;width:250px;display:grid;gap:6px;padding:10px;background:var(--surface);border:1px solid var(--border);box-shadow:0 8px 30px #0004}.file-open input{display:block;width:100%;padding:4px;font-size:11px}.split-workspace{flex:1;min-height:0;display:grid;grid-template-columns:minmax(0,var(--split)) 7px minmax(0,1fr)}.pane{display:flex;flex-direction:column;min-width:0;min-height:0}.pane-heading{display:flex;align-items:center;gap:12px;padding:10px 14px;background:var(--surface);border-bottom:1px solid var(--border)}.pane-heading strong{font-size:14px}.pane-heading span{font-size:11px;color:var(--text-dim)}.pane-heading button{margin-left:auto;padding:4px 9px}.pane-tools{min-height:46px;padding:7px 12px;display:flex;gap:5px;align-items:center;flex-wrap:wrap;border-bottom:1px solid var(--border)}.pane-tools .subtle{flex:1}.pane-tools button{font-size:12px}.canvas-wrap{flex:1;min-height:120px;position:relative;overflow:hidden}.canvas-wrap svg{width:100%;height:100%;display:block;touch-action:none;outline:none}.canvas-wrap svg:focus-visible{box-shadow:inset 0 0 0 2px var(--accent)}.selected{stroke-width:3}.splitter{background:var(--surface-raised);cursor:col-resize;touch-action:none;display:flex;align-items:center;justify-content:center;border-inline:1px solid var(--border)}.splitter:hover,.splitter:focus-visible{background:var(--accent)}.splitter span{height:35px;width:2px;background:var(--text-dim);border-radius:2px}.object-strip{min-height:47px;max-height:90px;overflow:auto;display:flex;align-items:center;gap:6px;padding:8px 12px;flex-wrap:wrap;border-top:1px solid var(--border);background:var(--surface)}.object-strip>span{font-size:11px;color:var(--text-dim);margin-right:5px}.object-strip button{font-size:12px;padding:4px 8px}.context-bar{min-height:60px;display:flex;align-items:center;gap:12px;flex-wrap:wrap;padding:10px 16px;border-top:1px solid var(--border);background:var(--surface)}.context-bar label{display:flex;align-items:center;gap:5px;color:var(--text-dim)}.context-bar input{width:65px;padding:6px}.primary{background:var(--accent);color:var(--bg);font-weight:600}.delete{margin-left:auto}.subtle{color:var(--text-dim);font-size:12px}.empty-hint{position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;text-align:center;pointer-events:none;color:var(--text-dim);padding:25px}.empty-hint strong{font-size:18px;font-weight:500}.empty-hint span{font-size:12px;max-width:280px}.zoom-tools{position:absolute;right:14px;bottom:14px;display:flex;gap:4px}.zoom-tools button{font-size:18px}.error-bar{padding:10px 16px;color:var(--danger);background:var(--surface);display:flex;justify-content:space-between}@media(max-width:750px){.direct-workspace{inset:0}.workspace-bar{gap:8px;padding:8px}.workspace-bar>strong{font-size:12px}.save-status{display:none}.pane-heading{padding:8px;gap:5px}.pane-heading span{display:none}.pane-tools{padding:5px}.pane-tools button{padding:5px;font-size:11px}.context-bar{gap:7px;padding:8px}.context-bar input{width:52px}.empty-hint strong{font-size:14px}}
.hovered{stroke:#e1d4ff;stroke-width:3}.operation-card{position:absolute;right:14px;top:14px;width:245px;display:grid;gap:10px;padding:15px;background:var(--surface);border:1px solid var(--border);border-radius:9px;box-shadow:0 8px 24px #0003}.operation-card small{font-size:11px;color:var(--text-dim);line-height:1.5}.operation-card label{display:flex;justify-content:space-between;align-items:center;gap:8px}.operation-card input{width:90px}.operation-card select{max-width:145px;background:var(--surface-raised);color:var(--text);padding:5px;border:1px solid var(--border)}.operation-card>div{display:flex;gap:5px}.segmented button{padding:5px 8px;font-size:12px}.live-measure{position:absolute;left:14px;top:14px;padding:8px 12px;border-radius:5px;background:var(--surface);color:var(--accent);font:14px monospace;pointer-events:none}.height-handle{cursor:ns-resize}.snap-toggle{display:flex;align-items:center;gap:4px;font-size:11px;margin-left:auto}.grid-input{width:50px;padding:4px}.transform-menu{position:relative}.transform-menu>div{position:absolute;bottom:40px;left:0;width:270px;display:flex;flex-wrap:wrap;gap:10px;padding:14px;border:1px solid var(--border);background:var(--surface);border-radius:8px;box-shadow:0 8px 24px #0003}.help-card{position:absolute;right:18px;bottom:76px;width:min(360px,85vw);padding:20px;background:var(--surface);border:1px solid var(--border);border-radius:10px;box-shadow:0 8px 30px #0004;font-size:13px;line-height:1.6;z-index:5}@media(max-width:750px){.operation-card{width:195px;padding:10px;right:8px;top:8px}.pane-tools .subtle{display:none}.snap-toggle{margin-left:0}}
</style>
