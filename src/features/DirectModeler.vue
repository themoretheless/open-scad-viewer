<script setup lang="ts">
import {TransparentBsp} from '../services/transparentBsp'
import CpuOrbitCanvas from '../components/CpuOrbitCanvas.vue'
import VrControls from '../components/VrControls.vue'
import { prepareVrPolygons } from '../services/vrScene'
import type {SelfIntersection} from '../services/solidSelfIntersection'
import type {FaceContacts} from '../services/solidFaceContacts'
import type {BoundaryAgreement} from '../services/solidBoundaryAgreement'
import type {NurbsSurfaceDistance} from '../services/nurbsSurface'
import type {NurbsCurveDistance} from '../services/nurbsCurve'
import {SolidSketchSnapPreparation} from '../services/solidSketchSnapPreparation'
import {readSolidDraftHead,writeSolidDraftHead} from '../services/solidDraftHeadStore'
import type {MainSolidJob} from '../services/mainSolidProtocol'
import {collectSolidDraftSnapshots,withSolidDraftLock,solidDraftSnapshotId,readSolidDraftSnapshot,writeSolidDraftSnapshot,removeSolidDraftSnapshot} from '../services/solidDraftStore'
import {detachSolidInstances} from '../services/solidInstances'
import SceneObjectControls from '../components/SceneObjectControls.vue'
import SceneVirtualList from '../components/SceneVirtualList.vue'
import { connectedEdgeChain } from '../services/edgeSelection'
import CadQuantityInput from '../components/CadQuantityInput.vue'
import {type SurfaceBoundaryReport,type SurfaceBoundaryOptions} from '../services/solidSurfaceDiagnostics'
import { type inspectSolidDisplay, inspectSolidIntersections } from '../services/solidDiagnostics'
import SketchDimensionPanel from '../components/SketchDimensionPanel.vue'
import { sketchDimensions } from '../services/directDimensions'
import { defineAsyncComponent, computed, nextTick, onUnmounted, ref, shallowRef, watch, watchEffect, type ComponentPublicInstance } from 'vue'
import {type PointMeasurement,type CurveMeasurement,type FaceDistanceResult,type ShellDistanceResult,solidVertexCount} from '../services/solidMeasurements'
import type {solidBodyEdges} from '../services/solidBodyEdges'
import {isSolidProfileEdit,type SolidProfileEditOptions} from '../services/solidProfileEdit'
import {isSolidSceneEdit} from '../services/solidSceneEdit'
import {isSolidBodyEdit,type SolidBodyEditOptions} from '../services/solidBodyEdit'
import {assignSolidMaterial,solidMaterialRgb} from '../services/solidMaterial'
import { stringifyMeshJson } from '../services/meshJson'
import { SolidGpuLayer, isSolidGpuSupported, smoothTriangleList, type SolidGpuBody } from '../services/solidGpuView'
import { rayTriangleDistance } from '../services/math3d'
import ModelingFloorGrid from '../components/ModelingFloorGrid.vue'
import ModelingGridControls from '../components/ModelingGridControls.vue'
import {createSolidPreviewWorker} from '../services/solidPreviewWorker'
import { sameSketchPlane } from '../services/directExtrusion'
import { useModelingGrid } from '../services/modelingGrid'
import {SolidSnapPreparation} from '../services/solidSnapPreparation'
import { resolveModelingSnap, type SnapKind, type SnapGeometry } from '../services/modelingSnaps'
import CommandPalette from '../components/CommandPalette.vue'
import type { PaletteCommand } from '../services/commandSearch'
import {SolidCurveDisplayQueue} from '../services/solidCurveDisplayQueue'
import type {NurbsCurve,NurbsScreenProjection} from '../services/nurbsCurve'
import {SolidProfileDisplayQueue} from '../services/solidProfileDisplayQueue'
import {SolidSurfaceDisplayQueue} from '../services/solidSurfaceDisplayQueue'
import {SolidDisplayQueue} from '../services/solidDisplayQueue'
import {SolidDisplayCache,type DisplayMesh} from '../services/solidDisplayCache'
import { bodyPoints, DirectHistory, directBodiesScad, emptyDirectDocument, MAX_DOCUMENT_CHARACTERS, MAX_DRAFT_CHARACTERS, parseDirectDocument, serializeDirectDocument, type DirectBody, type DirectDocument, type DirectSketch, type Point2 } from '../services/directModeling'
import type {SolidSketchEditOptions} from '../services/solidSketchEdit'
import { validateSimpleSketch, slotSketch, sampleCurve, bakeSketch, trimSketch, worldPoint, xyPlane, unit3, cross3, type SketchPlane, type Vec3 } from '../services/directSketchGeometry'
import { transformSelection } from '../services/directSolidTools'
import { storageGet, storageSet } from '../services/safeStorage'
import { exportPolygonStl, type PolygonMesh } from '../services/geometry/polygon'
import type {SolidBooleanOptions} from '../services/solidBoolean'
import { createDirectProjector, defaultDirectCamera, directExtrusionTool, directFaceShade, projectDirectPoint, unprojectDirectXY, unprojectDirectPlane } from '../services/directModelingTools'
import { importMeshFromFile, MESH_IMPORT_ACCEPT, stripMeshExtension } from '../services/meshImport'
import { polygonMeshToExportMesh, MESH_EXPORT_FORMATS, MESH_FORMAT_LABELS, type MeshExportFormat } from '../services/meshConvert'
import { exportMeshFormatCompressed } from '../services/meshExportFormats'
import { downloadBytes } from '../services/downloadArtifact'
import { solidDocumentToMeshDocument } from '../services/solidBridge'
import {requirePolylineSketch} from '../services/retainedSketchProfile'
import type {prepareSolidProfile} from '../services/solidProfilePreparation'
import type {matchSolidCurve} from '../services/solidCurveMatching'
import type {matchSolidSurface,prepareSolidSurfaceSeams} from '../services/solidSurfaceMatching'
import type {SolidBrepToolOptions} from '../services/solidBrepTool'
import type {SolidNurbsEditOptions} from '../services/solidNurbsEdit'
import { createSolidNurbsCurve, createSolidNurbsSurface, type SolidNurbsSurface, type SolidNurbsCurve } from '../services/solidNurbs'
import {evaluateNurbsSurface} from '../services/nurbsSurface'
import type {FramedSweepResult,SurfaceJetBoundary} from '../services/nurbsConstructors'
import { evaluateNurbsCurve } from '../services/nurbsCurve'
import type {CadPairReport} from '../services/cadInspection'
import {createSolidClearanceWorker} from '../services/solidClearanceWorker'
import {isSolidNurbsRefit,type refitSolidNurbs} from '../services/solidCurveReduction'
import {isSolidSurfaceBuild,type buildSolidSurface} from '../services/solidSurfaceConstruction'
import { isGeometryKernelReady, warmGeometryKernel } from '../services/geometry/kernel'
import { type BrepMassProperties, type BrepBooleanOperation } from '../services/geometry/brep'
const CurvePointTrimControls=defineAsyncComponent(()=>import('../components/CurvePointTrimControls.vue'))
const CurveOffsetPreview=defineAsyncComponent(()=>import('../components/CurveOffsetPreview'))
const CurveOffsetConstructionInfo=defineAsyncComponent(()=>import('../components/CurveOffsetConstructionInfo'))
const CurveOffsetControls=defineAsyncComponent(()=>import('../components/CurveOffsetControls'))
const CoonsPreparationControls=defineAsyncComponent(()=>import('../components/CoonsPreparationControls'))
const props = defineProps<{ open: boolean; locale: string; canAppend: boolean; remainingSource: number; embedded?: boolean; initialDocument?: DirectDocument; initialSelection?: string; seedDocument?: DirectDocument | null; appendBodies?: { bodies: DirectBody[]; token: number; group?: { name: string; source: string; replaces: string | null } } | null; paletteRequest?: number }>()
const emit = defineEmits<{ backend: [gpu: boolean]; close: []; append: [source: string]; toMesh: []; 'edit-group': [request: { name: string; source: string; replaces: string | null }] }>()
const ru = computed(() => props.locale === 'ru')
const ShellDistanceSummary=defineAsyncComponent(()=>import('../components/ShellDistanceSummary.vue'))
const SolidVolumeWitness=defineAsyncComponent(()=>import('../components/SolidVolumeWitness.vue'))
const SolidVolumeDistance=defineAsyncComponent(()=>import('../components/SolidVolumeDistance.vue'))
const label = (a: string, b: string) => ru.value ? a : b
const key = props.embedded ? 'scad-main-modeler-v1' : 'scad-solid-modeler-v1'
const error = ref(''), saveError = ref(false), savePending=ref(false)
const draftConflict=ref(false),loadingLatestDraft=ref(false)
let persistedRevision:string|null=null
const durableReady=readSolidDraftHead(key).then(head=>{persistedRevision=head?.revision??null;return head})
void durableReady.catch(()=>{})
let saveGeneration=0
let persistedReference=storageGet(key)
let stored = persistedReference ?? storageGet(props.embedded ? 'scad-main-modeler-v1' : 'scad-direct-modeler-v1')
const restoringDraft=ref(!props.initialDocument&&(typeof indexedDB!=='undefined'||!!stored&&(!isGeometryKernelReady()||!!solidDraftSnapshotId(stored))))
let restoreDisposed=false
onUnmounted(()=>{restoreDisposed=true})
let initial = emptyDirectDocument()
try { if (!restoringDraft.value&&stored&&isGeometryKernelReady()&&!solidDraftSnapshotId(stored)&&!props.initialDocument) initial = parseDirectDocument(stored) } catch { saveError.value=true;error.value=label('Сохранённый JSON повреждён. Импортируйте резервный JSON через меню «Файл».','Saved JSON is corrupt. Import a JSON backup from the File menu.') }
if (props.initialDocument) initial = props.initialDocument
let history = new DirectHistory(initial)
const document = shallowRef(history.document)
// Committed render snapshot; gestures publish separate documents and never update this ref.
const snapDocument = shallowRef(document.value)
const stlInput = ref<HTMLInputElement>()
const svgInput=ref<HTMLInputElement>(),svgBusy=ref(false)
let svgController:AbortController|undefined
function cancelSvg(){svgController?.abort();svgController=undefined;svgBusy.value=false}
watch(()=>props.open,open=>{if(!open)cancelSvg()},{flush:'sync'})
onUnmounted(cancelSvg)
const stepInput = ref<HTMLInputElement>(), stepBusy = ref(false)
const undoable = ref(false), redoable = ref(false)
const selection = ref(props.initialSelection ?? ''), mode = ref<'2d' | '3d'>('2d'), tool = ref<'select' | 'rectangle' | 'circle' | 'arc' | 'polyline' | 'trim' | 'slot'>('select')
const draftCursor=ref<Point2|null>(null)
const slotWidth=ref(5), slotWidthValid=ref(true)
function undoDraftPoint(){draft.value=draft.value.slice(0,-1);draftCursor.value=null;snapMarker.value=null;snapGuide.value=null}
const draft = ref<Point2[]>([]), height = ref(10), dx = ref(0), dy = ref(0), dz = ref(0), angle = ref(0), scale = ref(1)
type Pane = '2d' | '3d'
const workspace = ref<HTMLElement>(), splitArea = ref<HTMLElement>()
const split = ref(34)
const views = ref<Record<Pane, number>>({ '2d': 160, '3d': 160 })
const centers = ref<Record<Pane, Point2>>({ '2d': [0, 0], '3d': [0, 0] })
const savedInputMode = storageGet('scad-input-mode')
const inputMode = ref<'mouse' | 'touch'>(savedInputMode === 'touch' || (savedInputMode !== 'mouse' && typeof window !== 'undefined' && window.matchMedia?.('(pointer: coarse)').matches) ? 'touch' : 'mouse')
const touchNavigate = ref(false)
const touchPointers = new Map<number, { x: number; y: number; pane: Pane; svg: SVGSVGElement }>()
let multiTouch = false
function resetInputGesture() {
  cancelGesture()
  const captured = [...touchPointers.entries()]
  touchPointers.clear(); multiTouch = false
  for (const [id, point] of captured) if (point.svg.hasPointerCapture(id)) point.svg.releasePointerCapture(id)
}
function toggleInputMode() {
  resetInputGesture()
  inputMode.value = inputMode.value === 'mouse' ? 'touch' : 'mouse'
  touchNavigate.value = false
  storageSet('scad-input-mode', inputMode.value)
}
function toggleTouchNavigation() { resetInputGesture(); touchNavigate.value = !touchNavigate.value }
function touchDown(e: PointerEvent, pane: Pane) {
  if (e.pointerType !== 'touch' || inputMode.value !== 'touch') return
  const svg = e.currentTarget as SVGSVGElement
  const first = touchPointers.values().next().value
  if (first && (first.pane !== pane || touchPointers.size >= 2)) { e.stopPropagation(); return }
  touchPointers.set(e.pointerId, { x: e.clientX, y: e.clientY, pane, svg })
  svg.setPointerCapture(e.pointerId)
  if (touchPointers.size === 2) {
    // A second finger cancels the edit preview before taking over the camera.
    cancelGesture(); multiTouch = true; cameraDragging.value = true
    e.stopPropagation(); e.preventDefault(); return
  }
  if (touchNavigate.value) {
    e.stopPropagation(); e.preventDefault(); svg.focus(); mode.value = pane
    cameraDragging.value = true
    if (pane === '3d') orbitDrag = { x:e.clientX, y:e.clientY, yaw:camera.value.yaw, pitch:camera.value.pitch, pointer:e.pointerId, svg }
    else gesture = { start:position(e), document:document.value, vertex:null, id:'', pointer:e.pointerId, pane, svg, pan:true, center:[...centers.value[pane]] }
  }
}
function touchMove(e: PointerEvent) {
  const point = touchPointers.get(e.pointerId)
  if (!point) return
  if (!multiTouch) { point.x = e.clientX; point.y = e.clientY; return }
  e.stopPropagation(); e.preventDefault()
  const other = [...touchPointers.entries()].find(([id]) => id !== e.pointerId)?.[1]
  if (other) {
    const distance = Math.hypot(point.x-other.x, point.y-other.y)
    const nextDistance = Math.hypot(e.clientX-other.x, e.clientY-other.y)
    const rect = point.svg.getBoundingClientRect()
    const pixels = Math.max(1, Math.min(rect.width, rect.height))
    const oldSize = views.value[point.pane]
    const midX = (point.x+other.x)/2 - rect.left - rect.width/2
    const midY = (point.y+other.y)/2 - rect.top - rect.height/2
    if (distance > 2 && nextDistance > 2) zoom(point.pane, distance / nextDistance)
    const size = views.value[point.pane], center = centers.value[point.pane]
    centers.value[point.pane] = [
      center[0] + midX*(oldSize-size)/pixels - (e.clientX-point.x)*size/(2*pixels),
      center[1] + midY*(oldSize-size)/pixels - (e.clientY-point.y)*size/(2*pixels),
    ]
  }
  point.x = e.clientX; point.y = e.clientY
}
function touchEnd(e: PointerEvent) {
  if (!touchPointers.has(e.pointerId)) return
  touchPointers.delete(e.pointerId)
  if (multiTouch) {
    e.stopPropagation()
    // Keep the remaining finger inert until lifted to avoid an accidental edit.
    if (!touchPointers.size) { multiTouch = false; cameraDragging.value = false }
  }
}
function cancelInputGesture() { resetInputGesture() }
const panes: Pane[] = ['2d', '3d']
const savedSketchPane = storageGet('scad-solid-sketch-pane')
const sketchPaneOpen = ref(savedSketchPane === null ? document.value.bodies.length === 0 : savedSketchPane === 'true')
function toggleSketchPane(open = !sketchPaneOpen.value) {
  sketchPaneOpen.value = open
  storageSet('scad-solid-sketch-pane', String(open))
  if (!open && mode.value === '2d') mode.value = '3d'
}
// Picking a sketch tool reveals the pane for this session without changing the saved preference.
watch([mode, tool], ([value]) => { if (value === '2d') sketchPaneOpen.value = true })
const camera = ref(defaultDirectCamera()), hovered = ref('')
const { step: grid, enabled: snap, grid: gridSnap, geometry: geometrySnap, guides: guideSnap, keypoints: keypointSnap, relations: relationSnap, radius: snapRadius } = useModelingGrid()
/**
 * True while a rotate or scale gizmo drag is in flight.
 *
 * Every preview step commits a new document, and the exact display path would
 * re-tessellate the moved B-rep through the kernel on each one. The working mesh is
 * shown for the moved bodies instead until the pointer is released.
 */
const previewingTransform = ref(false)
const snapMarker = ref<Point2 | null>(null)
const snapPane = ref<Pane>('2d'), snapKind = ref<SnapKind | null>(null)
const snapGuide = ref<[Point2, Point2] | null>(null)
const snapLabel = computed(() => { const names: Record<SnapKind, [string,string]> = { vertex:['Вершина','Vertex'], midpoint:['Середина','Midpoint'], center:['Центр','Center'], edge:['Контур','Edge'], intersection:['Пересечение','Intersection'], axis:['Ось','Axis'], grid:['Сетка','Grid'], quadrant:['Четверть окружности','Quadrant'],tangent:['Касательная','Tangent'],perpendicular:['Перпендикуляр','Perpendicular'],origin:['Начало координат','Origin'],'bounds-center':['Центр габаритов','Bounds center'] }; return snapKind.value ? label(...names[snapKind.value]) : '' })
const dragConstraint = ref('')
const drawMeasure = ref(''), operation = ref<'extrude' | 'revolve' | 'fillet' | 'dogear' | 'array' | null>(null)
const cornerVertex = ref(0), cornerRadius = ref(2)
const revolveAxis = ref<'x'|'y'>('y'), revolveOffset = ref(0), revolveAngle = ref(360), revolveSegments = ref(48)
const revolveOptions = () => ({ axis: revolveAxis.value, offset: revolveOffset.value, angle: revolveAngle.value, segments: revolveSegments.value })
const cornerActive = computed(() => operation.value === 'fillet' || operation.value === 'dogear')
const solidActive = computed(() => operation.value === 'extrude' || operation.value === 'revolve')
const sketchEditWorker=createSolidPreviewWorker(),sketchEditPending=ref(false),sketchEditResult=shallowRef<DirectDocument|null>(null),sketchEditError=ref('')
const sketchEditRevision=ref(0),sketchEditRetryVisible=ref(false)
const sketchEditFailure=computed(()=>{
 const messages:Record<string,string>={
  'Use 2–64 instances and an angle up to 360 degrees.':'Задайте 2–64 экземпляра и угол от 0,01° до 360° по модулю.',
  'Radius exceeds the adjacent edges. Use a smaller radius.':'Радиус превышает длину соседних рёбер. Уменьшите радиус.',
  'Radius creates a self-intersection. Use a smaller radius.':'Радиус создаёт самопересечение. Уменьшите радиус.',
  'DogEar requires a right-angle corner.':'Для DogEar выберите прямой угол.',
  'Select a non-collinear corner.':'Выберите вершину, в которой рёбра не лежат на одной прямой.',
  'Radius must be at least 0.01 mm.':'Радиус должен быть не меньше 0,01 мм.',
 }
 return ru.value?messages[sketchEditError.value]??sketchEditError.value:sketchEditError.value
})
let sketchEditGeneration=0
function cancelSketchEdit(){sketchEditGeneration++;sketchEditWorker.cancel();sketchEditPending.value=false;sketchEditResult.value=null;sketchEditError.value=''}
onUnmounted(()=>{cancelSketchEdit();sketchEditWorker.dispose()})
const cornerPreview = computed(() => ({sketch:cornerActive.value?sketchEditResult.value?.sketches.find(s=>s.id===selection.value)??null:null,error:sketchEditFailure.value}))
function beginCorner(kind: 'fillet'|'dogear') { boxSelect.value=false; if (!selectedSketch.value?.closed) return; cancelCommand(); tool.value = 'select'; operation.value = kind; mode.value = '2d' }
function applySketchEdit(){run(()=>{const next=sketchEditResult.value;if(!next||sketchEditPending.value)return;commit(next);operation.value=null})}
function applyCorner(){applySketchEdit()}
const profileIds = ref<string[]>([]), previewEmpty = ref(false)
const baseZ = ref(0), extrusionMode = ref<'new' | 'union' | 'difference'>('new'), targetBody = ref('')
const copyCount = ref(8), copySweep = ref(360), copyX = ref(0), copyY = ref(0)
const previewBody = shallowRef<ReturnType<typeof directExtrusionTool> | null>(null), previewError = ref('')
const solidPreviewRevision=ref(0),solidPreviewRetryVisible=ref(false)
watch(operation,()=>{solidPreviewRetryVisible.value=false;sketchEditRetryVisible.value=false},{flush:'sync'})
const movingBody = ref(false), showHelp = ref(false), floorVisible = ref(true)
let fitNextPreview = false
let previewTimer: ReturnType<typeof setTimeout> | undefined
const previewWorker=createSolidPreviewWorker()
let previewGeneration=0
const solidPreviewResult=shallowRef<DirectDocument|null>(null), previewPending=ref(false)
function invalidateSolidPreview() {
 clearTimeout(previewTimer);previewGeneration++;previewWorker.cancel()
 solidPreviewResult.value=null;previewPending.value=false
 previewBody.value=null;previewEmpty.value=false;previewError.value=''
}
onUnmounted(() => { invalidateSolidPreview();previewWorker.dispose() })
let orbitDrag: { x: number; y: number; yaw: number; pitch: number; pointer: number; svg: SVGSVGElement } | null = null
// While the camera is being dragged the view falls back to the working mesh so orbiting stays responsive.
const cameraDragging = ref(false)
let heightDrag: { y: number; height: number; pointer: number; svg: SVGSVGElement } | null = null
let gesture: { start: Point2; document: DirectDocument; vertex: number | null; id: string; pointer: number; pane: Pane; svg: SVGSVGElement; pan: boolean; center: Point2; sketch?: boolean; anchor?: Point2; anchor3?: Vec3; dragStart?: Point2; workerEdit?:boolean; inverse?:DOMMatrix|null; bodyDrag?: { ids: string[]; delta: Vec3 } | null } | null = null

/**
 * Elements offset directly while a body drag is in flight.
 *
 * The 3D view renders one SVG polygon per triangle, and this scene has thousands. Both
 * re-deriving the projection and re-rendering the list through Vue cost tens of
 * milliseconds per pointer move. An orthographic projection is affine, so translating a
 * body in the world is a constant screen offset: the drag sets that offset straight on
 * the dragged body's existing elements, leaving the reactive scene untouched until the
 * pointer is released.
 */
/**
 * Elements that must travel with dragged bodies: their hit polygons (when rendered) and
 * the SVG overlays drawn for the selection, such as vertex handles, feature edges and
 * the gizmo. Overlays are keyed by whitespace-separated body ids, so a gizmo shared by a
 * multi-selection matches any of its bodies. Without this the GPU surface moved while
 * the handles stayed behind, which reads as lag.
 */
function collectDragNodes(svg: SVGSVGElement | undefined, ids: readonly string[]): SVGElement[] {
  if (typeof svg?.querySelectorAll !== 'function') return []
  const seen = new Set<SVGElement>()
  for (const id of ids) {
    const escaped = id.replace(/["\\]/g, '\\$&')
    for (const node of svg.querySelectorAll<SVGElement>(`[data-body="${escaped}"], [data-body-overlay~="${escaped}"]`)) seen.add(node)
  }
  return [...seen]
}

let dragNodes: SVGElement[] = []
/**
 * Frame rate of the viewport, sampled over half-second windows.
 *
 * Measured from the browser's own frame callbacks, so it reflects what the eye sees
 * rather than the cost of any one handler.
 */
const fps = ref(0)
/** Median frame interval of the last window. A steady 33.3 ms means the frames are
 * being paced externally; uneven, longer intervals mean the work itself is too slow. */
const frameMs = ref(0)
/** GPU time of the latest draw, sampled alongside the frame rate. */
const drawMs = ref(0)
let fpsHandle = 0, fpsSince = 0, fpsPrevious = 0
let fpsIntervals: number[] = []
function fpsTick(now: number) {
  if (fpsPrevious) fpsIntervals.push(now - fpsPrevious)
  fpsPrevious = now
  const elapsed = now - fpsSince
  if (elapsed >= 500 && fpsIntervals.length) {
    fps.value = Math.round((fpsIntervals.length * 1000) / elapsed)
    const sorted = [...fpsIntervals].sort((a, b) => a - b)
    frameMs.value = Math.round(sorted[sorted.length >> 1] * 10) / 10
    drawMs.value = Math.round((gpuLayer?.drawMs ?? 0) * 10) / 10
    fpsIntervals = []
    fpsSince = now
  }
  fpsHandle = requestAnimationFrame(fpsTick)
}

let dragOffsetIds: string[] = []
function translationPreviewIds(ids:string[]){return [...new Set([...ids,...document.value.bodies.filter(b=>b.instance&&ids.includes(b.instance.sourceId)).map(b=>b.id)])]}
function clearDragPreview() {
  for (const node of dragNodes) node.removeAttribute?.('transform')
  dragNodes = []
  if (dragOffsetIds.length) { gpuLayer?.setDragOffset(dragOffsetIds, [0, 0, 0]); dragOffsetIds = [] }
}


const extraSelection=ref<string[]>([]),pickMode=ref<'body'|'face'|'edge'|'vertex'>('body'),vertexIndexes=ref<number[]>([]),faceIndex=ref(-1),edgeIndex=ref(-1),edgeIndexes=ref<number[]>([]),openingFaces=ref<number[]>([])
const workplaneOutline=ref<Point2[][]>([])
const workplaneBodyId=ref(''), choosingSketchFace=ref(false)
const faceDrawing = computed(()=>!!workplaneBodyId.value && tool.value!=='select' && tool.value!=='trim')
function profileBooleanOperation(id:string|null):'union'|'difference'|'intersection'|null {
 return id==='profile-union'?'union':id==='profile-difference'?'difference':id==='profile-intersection'?'intersection':null
}
function isProfileCommand(id:string|null){return id==='profile-prepare'||!!profileBooleanOperation(id)}
function setProfileTarget(id:string){
 const index=surfaceInputs.value.indexOf(id)
 if(index>0){const ids=[...surfaceInputs.value];[ids[0],ids[index]]=[ids[index],ids[0]];surfaceInputs.value=ids}
}
const pointTrimPick=shallowRef<{point:[number,number];matrix:NurbsScreenProjection;radius:number}|null>(null)
const activePlane=ref<SketchPlane>(xyPlane()),advancedOp=ref<'nurbs-offset'|'nurbs-point-trim'|'instance-transform'|'instance-create'|'instance-place'|'push'|'chamfer'|'edge-fillet'|'shell'|'split'|'offset'|'extend'|'curve'|'transform'|'loft'|'nurbs-loft'|'nurbs-sweep'|'nurbs-rebuild'|'nurbs-reduce'|'nurbs-surface-rebuild'|'nurbs-surface-reduce'|'nurbs-patch'|'nurbs-match'|'nurbs-prepare'|'nurbs-curve-match'|'profile-prepare'|'profile-union'|'profile-difference'|'profile-intersection'|null>(null)
const loftPreviewId=ref(''),surfaceInputs=ref<string[]>([]),surfaceReversed=ref<boolean[]>([])
const advanced=ref({offsetJoin:'smooth' as 'smooth'|'bevel'|'trim-nonzero'|'trim-evenodd',patchPrepare:false,patchError:1e-6,profileGap:.01,curveEndA:'end' as 'start'|'end',curveEndB:'start' as 'start'|'end',curveAngle:1e-6,prepareOpenPeriodic:false,matchOrder:1 as 1|2,matchBoundaryA:'uMax' as SurfaceJetBoundary,matchBoundaryB:'uMin' as SurfaceJetBoundary,matchScale:1,matchReverse:false,matchError:1e-6,sweepMode:'translation' as 'translation'|'framed',sweepSections:24,sweepDeviation:.01,sweepNormalX:0,sweepNormalY:0,sweepNormalZ:1,surfaceAxis:'u' as 'u'|'v',rebuildControls:6,reduceDegree:1,maxError:.01,distance:2,radius:2,endRadius:3,filletMode:'constant' as 'constant'|'variable'|'corner',axis:'z' as 'x'|'y'|'z',x:0,y:0,z:0,angle:0,scale:1,cx:0,cy:0,start:0,sweep:180,end:'end' as 'start'|'end'})
const brepSegments=ref(4),filletSegments=ref(12)
const revolveGeometry=ref<'faceted'|'exact'>('faceted')
function selectIndexKey(e:KeyboardEvent,current:number,last:number,min=0){
 if(!['ArrowUp','ArrowDown','Home','End'].includes(e.key))return null
 e.preventDefault();e.stopPropagation()
 return e.key==='Home'?min:e.key==='End'?last:Math.max(min,Math.min(last,current+(e.key==='ArrowDown'?1:-1)))
}
function revolveGeometryKey(e: KeyboardEvent){
 const index=selectIndexKey(e,revolveGeometry.value==='exact'?1:0,1)
 if(index!==null)revolveGeometry.value=index?'exact':'faceted'
}
function surfaceInputKey(e:KeyboardEvent,role:number){
 const curves=document.value.curves??[]
 if(!curves.length)return
 const index=selectIndexKey(e,curves.findIndex(c=>c.id===surfaceInputs.value[role]),curves.length-1)
 if(index!==null)surfaceInputs.value[role]=curves[index].id
}

const boxSelect=ref(false),selectionBox=ref<{start:Point2;end:Point2;pane:Pane}|null>(null),gizmoMode=ref<'move'|'rotate'|'scale'>('move')
// Vertex editing of polygon bodies: drag selected vertices in the screen plane, one undo step on release.
let vertexDrag:{svg:SVGSVGElement;pointer:number;start:Point2;ids:number[];before:DirectDocument;moved:boolean;inverse:DOMMatrix|null}|null=null
// Tessellated bodies duplicate vertices per face; group indices by position so a drag moves the whole corner.
function coincidentVertexGroups(positions:ArrayLike<number>):Map<string,number[]>{
 const groups=new Map<string,number[]>()
 for(let i=0;i<positions.length/3;i++){const key=`${positions[i*3].toFixed(5)},${positions[i*3+1].toFixed(5)},${positions[i*3+2].toFixed(5)}`;const list=groups.get(key);if(list)list.push(i);else groups.set(key,[i])}
 return groups
}
const bodyVertices=computed(()=>{
 if(pickMode.value!=='vertex'||!selectedBody.value)return []
 const points=bodyPoints(selectedBody.value)
 return [...coincidentVertexGroups(selectedBody.value.mesh.positions).values()].map(group=>{const p=project(points[group[0]],'3d');return {i:group[0],group,x:p[0],y:p[1]}})
})
function startVertexDrag(e:PointerEvent,index:number){
 if(!requireBodySnaps(e,document.value))return
 cancelNativeNurbs();cancelGizmoWorker();if(directTransformPending.value)cancelDirectTransform()
 e.stopPropagation()
 if(e.button!==0)return
 if(e.shiftKey||e.ctrlKey||e.metaKey){const set=new Set(vertexIndexes.value);set.has(index)?set.delete(index):set.add(index);vertexIndexes.value=[...set]}
 else if(!vertexIndexes.value.includes(index))vertexIndexes.value=[index]
 faceIndex.value=-1;edgeIndex.value=-1;edgeIndexes.value=[];advancedOp.value=null
 const body=selectedBody.value;if(!body)return
 const target=e.currentTarget as SVGGraphicsElement,svg=target.ownerSVGElement!;svg.focus()
 // Exact B-rep bodies keep their surfaces: dragging a corner moves the whole body instead of editing the mesh.
 if(body.brep){
  // The move path uses XY model coordinates, not projected SVG coordinates.
  gesture={start:plane(position(e),'3d'),document:history.document,vertex:null,id:body.id,pointer:e.pointerId,pane:'3d',svg,pan:false,center:[...centers.value['3d']],
   anchor3:Array.from(body.mesh.positions.slice(index*3,index*3+3)) as Vec3,dragStart:[e.clientX,e.clientY]}
  svg.setPointerCapture(e.pointerId);return
 }
 const groups=coincidentVertexGroups(body.mesh.positions),ids=[...new Set(vertexIndexes.value.flatMap(i=>{const key=`${body.mesh.positions[i*3].toFixed(5)},${body.mesh.positions[i*3+1].toFixed(5)},${body.mesh.positions[i*3+2].toFixed(5)}`;return groups.get(key)??[i]}))]
 vertexDrag={svg,pointer:e.pointerId,start:position(e),ids,before:history.document,moved:false,inverse:svg.getScreenCTM()?.inverse()??null}
 svg.setPointerCapture(e.pointerId)
}
const historyWorker=createSolidPreviewWorker(),historyPending=ref(false),historyMode=ref<'restore'|'import'|'shared'>('restore')
let historyGeneration=0
const nativeNurbsWorker=createSolidPreviewWorker(),nativeNurbsPending=ref(false)
const nativeBrepMode=ref<'mass'|'mesh'|null>(null)
let nativeNurbsGeneration=0
const gizmoWorker=createSolidPreviewWorker(),gizmoPending=ref(false)
let gizmoEpoch=0,gizmoRevision=0,gizmoRunning=false,gizmoPublishing=false,gizmoHistoryPending=false
let gizmoApplyRevision:number|null=null,gizmoBase:DirectDocument|null=null,gizmoPublished:DirectDocument|null=null
type DragWorkerJob=Extract<MainSolidJob,{kind:'sceneEdit'|'pointEdit'}>
const pointEditActive=ref(false)
let numericPointEdit=false
let gizmoQueued:{job:DragWorkerJob;revision:number}|null=null
let manipulatorDrag:{svg:SVGSVGElement;pointer:number;x:number;y:number;kind:'move'|'rotate'|'scale'|'push'|'split';axis:'x'|'y'|'z';direction:Point2;before:DirectDocument;initial:number;startPoint:Point2;center:Point2;delta?:Vec3}|null=null

let curveDrag:{id:string;kind:'center'|'radius'|'start'|'end';before:DirectDocument;pointer:number;inverse:DOMMatrix|null}|null=null
let cvDrag:{id:string;u:number;v:number;before:DirectDocument;point:number[];start:Point2;pointer:number;svg:SVGSVGElement;inverse:DOMMatrix|null}|null=null
const faceSketchWorker=createSolidPreviewWorker(),faceSketchPending=ref(false)
let faceSketchGeneration=0
const snapPreparationWorker=createSolidPreviewWorker(),snapPreparation=new SolidSnapPreparation(snapPreparationWorker)
const snapPreparationPending=ref(false),snapPreparationVersion=ref(0),snapPreparationErrors=ref<{id:string;message:string}[]>([])
let snapPreparationGeneration=0
function cancelSnapPreparation(){snapPreparationGeneration++;snapPreparation.cancel();snapPreparationPending.value=false}
onUnmounted(()=>{cancelSnapPreparation();snapPreparationWorker.dispose();snapPreparation.clear()})
const sketchSnapWorker=createSolidPreviewWorker(),sketchSnapPreparation=new SolidSketchSnapPreparation(sketchSnapWorker)
const sketchSnapPending=ref(false),sketchSnapVersion=ref(0),sketchSnapErrors=ref<{id:string;message:string}[]>([])
let sketchSnapGeneration=0
function cancelSketchSnaps(){sketchSnapGeneration++;sketchSnapPreparation.cancel();sketchSnapPending.value=false}
onUnmounted(()=>{cancelSketchSnaps();sketchSnapWorker.dispose();sketchSnapPreparation.clear()})
let previousFocus: HTMLElement | null = null
watch(() => props.open, async open => {
  if (open) {
    previousFocus = window.document.activeElement as HTMLElement
    await nextTick(); workspace.value?.focus()
  }
  else { cancelGesture(); operation.value = null; previousFocus?.focus() }
}, { immediate: true })

const selectedSketch = computed(() => document.value.sketches.find(s => s.id === selection.value))
function changeBodyMaterial(event:Event) {
 const color=(event.target as HTMLInputElement).value
 run(()=>commit(assignSolidMaterial(document.value,selectedIds.value,{...selectedBody.value?.material,name:'Custom',color})))
}
function changeMaterialParameter(key:'metallic'|'roughness'|'opacity',event:Event){
 const value=Number((event.target as HTMLInputElement).value)
 run(()=>commit(assignSolidMaterial(document.value,selectedIds.value,{name:'Custom',color:'#7094ba',...selectedBody.value?.material,[key]:value})))
}
function resetBodyMaterial(){run(()=>commit(assignSolidMaterial(document.value,selectedIds.value)))}
const selectedBody = computed(() => document.value.bodies.find(s => s.id === selection.value))
const selectedNurbsCurve = computed(() => document.value.curves?.find(s => s.id === selection.value))
const selectedNurbsSurface = computed(() => document.value.surfaces?.find(s => s.id === selection.value))
const selectedNurbs = computed(() => selectedNurbsCurve.value ?? selectedNurbsSurface.value)
const cvU = ref(0), cvV = ref(0), cvX = ref(0), cvY = ref(0), cvZ = ref(0), cvWeight = ref(1), knotValue = ref(.5)
const cvInvalidField=ref('')
const cvInputError=computed(()=>cvInvalidField.value==='weight'?label('Введите положительный вес контрольной точки.','Enter a positive control point weight.'):cvInvalidField.value?label('Введите конечное число в поле ','Enter a finite number in field ')+cvInvalidField.value.toUpperCase()+'.':'')
watch(()=>[selection.value,cvU.value,cvV.value,cvX.value,cvY.value,cvZ.value,cvWeight.value],()=>{cvInvalidField.value=''})
const trimBounds = ref<[number,number,number,number]>([0,1,0,1])
watch(selectedNurbsSurface, item => {
  if(!item)return
  const s=item.surface
  trimBounds.value=[s.knotsU[s.degreeU],s.knotsU[s.controlPoints.length],s.knotsV[s.degreeV],s.knotsV[s.controlPoints[0].length]]
})
const selectedCvPoint = computed(() => selectedNurbsCurve.value?.curve.controlPoints[cvU.value] ??
  selectedNurbsSurface.value?.surface.controlPoints[cvU.value]?.[cvV.value])
watch(selectedCvPoint, point => {
  if (point) {
    [cvX.value, cvY.value, cvZ.value] = [point[0] ?? 0, point[1] ?? 0, point[2] ?? 0]
    cvWeight.value = selectedNurbsCurve.value?.curve.weights[cvU.value] ??
      selectedNurbsSurface.value?.surface.weights[cvU.value]?.[cvV.value] ?? 1
  }
}, { immediate: true })
// Workspace visibility and locks do not alter geometry or its undo history.
const hiddenIds = ref<string[]>([]), lockedIds = ref<string[]>([]), activeGroup = ref('')
const documentObjects = (d:DirectDocument) => [...d.bodies,...d.sketches,...d.curves??[],...d.surfaces??[]]
const objectInView = (id:string) => objectVisible(id)&&(!isolatedBodyIds.value.length||isolatedBodyIds.value.includes(id))
const objectVisible = (id:string) => !hiddenIds.value.includes(id)
const objectSelectable = (id:string) => objectVisible(id) && !lockedIds.value.includes(id)
function toggleObjectState(id:string, kind:'hidden'|'locked') {
  cancelCommandState()
  const state=kind==='hidden'?hiddenIds:lockedIds
  state.value=state.value.includes(id)?state.value.filter(value=>value!==id):[...state.value,id]
  const ids=[selection.value,...extraSelection.value].filter(objectSelectable)
  selection.value=ids[0]??'';extraSelection.value=ids.slice(1);hovered.value=''
  faceIndex.value=edgeIndex.value=-1;edgeIndexes.value=[];openingFaces.value=[]
}
function moveSelectionToGroup() {
  cancelCommand()
  const next=history.document
  for(const object of documentObjects(next))if(selectedIds.value.includes(object.id)&&objectSelectable(object.id)) {
    if(activeGroup.value)object.group=activeGroup.value;else delete object.group
  }
  commit(next)
}
function addEmptyGroup() {
  const names=new Set([...documentObjects(document.value).map(b=>b.group),...(document.value.groups??[]).map(g=>g.name)])
  let i=1;while(names.has(label('Группа ','Group ')+i))i++
  const name=label('Группа ','Group ')+i,next=history.document
  next.groups=[...(next.groups??[]),{name,source:''}];commit(next);activeGroup.value=name
}
function selectEdgeChain() {
  if(!selectedBody.value || edgeIndex.value<0)return
  edgeIndexes.value=connectedEdgeChain(authoredEdges.value,edgeIndexes.value.length?edgeIndexes.value:[edgeIndex.value])
}
const selectedIds = computed(() => [...new Set([selection.value,...extraSelection.value].filter(Boolean))])
const selectedBrepBodies = computed(() => selectedIds.value.map(id=>document.value.bodies.find(b=>b.id===id)).filter(b=>b?.brep))
const twoSelectedBodies = computed(() => selectedIds.value.length === 2 && selectedIds.value.every(id => document.value.bodies.some(body => body.id === id)))
const selectedCurvePair = computed(() => selectedIds.value.length === 2 && selectedIds.value.every(id => document.value.curves?.some(item => item.id === id)) ? selectedIds.value : null)
const selectedSurfacePair = computed(() => selectedIds.value.length === 2 && selectedIds.value.every(id => document.value.surfaces?.some(item => item.id === id)) ? selectedIds.value : null)
type SolidTopology=import('../services/mainSolidProtocol').MainSolidResults['topology']
const topology=shallowRef<SolidTopology>({faces:[],edges:[]}),topologyPending=ref(false)
const topologyWorker=createSolidPreviewWorker()
let topologyPick=0,topologyGeneration=0,topologyBody:DirectBody|undefined,topologyRequest:Promise<SolidTopology|null>|null=null
function cancelTopology(clear=false){topologyPick++;topologyGeneration++;topologyWorker.cancel();if(clear||topologyPending.value){topologyBody=undefined;topologyRequest=null;topology.value={faces:[],edges:[]}}topologyPending.value=false}
function prepareTopology(body:DirectBody):Promise<SolidTopology|null>{
 if(topologyBody===body&&topologyRequest)return topologyRequest
 cancelTopology(true);const generation=topologyGeneration,source=document.value
 topologyBody=body;topologyPending.value=true
 topologyRequest=topologyWorker.run({kind:'topology',mesh:body.mesh}).then(result=>{
  if(generation!==topologyGeneration||document.value!==source||selectedBody.value!==body)return null
  topology.value=result;return result
 }).catch(e=>{if(generation===topologyGeneration)error.value=e instanceof Error?e.message:String(e);return null}).finally(()=>{if(generation===topologyGeneration)topologyPending.value=false})
 return topologyRequest
}
onUnmounted(()=>{cancelTopology();topologyWorker.dispose()})
function faceSelectionKey(e: KeyboardEvent) {
 if(topologyPending.value)return
 const index=selectIndexKey(e,faceIndex.value,topology.value.faces.length-1,-1)
 if(index===null)return
 faceIndex.value=index
 edgeIndex.value=-1;edgeIndexes.value=[];openingFaces.value=[]
}
const selectedFace = computed(() => topology.value.faces[faceIndex.value])
const selectedFaceTriangles=computed(()=>new Set((openingFaces.value.length?openingFaces.value:[faceIndex.value]).flatMap(i=>topology.value.faces[i]?.triangles??[])))
const samePlane = sameSketchPlane
const surfaceDistanceOpen=ref(false),surfaceDistanceBudget=ref(10000),surfaceDistancePending=ref(false)
const surfaceDistanceWorker=createSolidPreviewWorker()
const surfaceDistance=shallowRef<{value:NurbsSurfaceDistance|null;error:string}|null>(null)
onUnmounted(()=>surfaceDistanceWorker.dispose())
const boundaryInspection=ref(false)
const boundaryOptions=ref<SurfaceBoundaryOptions>({boundaryA:'uMax',boundaryB:'uMin',reverse:false,samples:65,toleranceMm:.01,angleToleranceDeg:1})
const boundaryWorker=createSolidPreviewWorker(),boundaryPending=ref(false)
const boundaryReport=shallowRef<{value:SurfaceBoundaryReport|null;error:string}|null>(null)
onUnmounted(()=>boundaryWorker.dispose())
const curveDisplayWorker=createSolidPreviewWorker(),curveDisplayQueue=new SolidCurveDisplayQueue(curveDisplayWorker)
const curveDisplayPending=ref(false),curveDisplayVersion=ref(0),curveDisplayErrors=ref<{id:string;message:string}[]>([])
let curveDisplayGeneration=0
function cancelCurveDisplay(){curveDisplayGeneration++;curveDisplayQueue.cancel();curveDisplayPending.value=false}
onUnmounted(()=>{cancelCurveDisplay();curveDisplayWorker.dispose();curveDisplayQueue.clear()})
function curvePoints(curve:NurbsCurve):number[][]{void curveDisplayVersion.value;return curveDisplayQueue.get({id:'',curve})??[]}
const profileDisplayWorker=createSolidPreviewWorker(),profileDisplayQueue=new SolidProfileDisplayQueue(profileDisplayWorker)
const profileDisplayPending=ref(false),profileDisplayVersion=ref(0),profileDisplayErrors=ref<{id:string;message:string}[]>([])
let profileDisplayGeneration=0
function cancelProfileDisplay(){profileDisplayGeneration++;profileDisplayQueue.cancel();profileDisplayPending.value=false}
onUnmounted(()=>{cancelProfileDisplay();profileDisplayWorker.dispose();profileDisplayQueue.clear()})
function profileLoops(sketch:DirectSketch):Point2[][]{
 void profileDisplayVersion.value
 return sketch.retainedProfile?profileDisplayQueue.get({id:sketch.id,profile:sketch.retainedProfile})??[]:[sketch.points]
}
const retainedDisplay=computed(()=>new Map(document.value.sketches.filter(s=>s.retainedProfile).map(s=>[s.id,profileLoops(s)])))
function sketchPath(sketch:DirectSketch,pane:Pane,loops:Point2[][]=retainedDisplay.value.get(sketch.id)??[sketch.points]){return loops.map(loop=>'M '+loop.map(p=>project(pane==='2d'?p:worldPoint(p,sketch.plane),pane).join(',')).join(' L ')+(sketch.closed?' Z':'')).join(' ')}
const visibleSketches = computed(() => document.value.sketches.filter(s=>objectInView(s.id)&&samePlane(s.plane,activePlane.value)))
const sceneList=ref<{reveal(key:string,focus?:boolean):Promise<boolean>}|null>(null)
watch(sceneList,async list=>{await nextTick();if(list&&selection.value)await list.reveal('object:'+selection.value)})
watch(selection, async () => {
  await nextTick()
  if(selection.value)await sceneList.value?.reveal('object:'+selection.value)
  workspace.value?.querySelector?.('.object-row>button[aria-pressed="true"]')?.scrollIntoView?.({block:'nearest'})
})
function pickObject(id:string,pane:Pane,add=false) {
 if(id && !objectSelectable(id))return
 if (isolatedBodyIds.value.length && !isolatedBodyIds.value.includes(id)) isolatedBodyIds.value = []

 if(subtract.value&&id&&document.value.bodies.some(b=>b.id===id)){subtractPick(id,add);mode.value=pane;return}
 if(id!==selection.value){faceIndex.value=edgeIndex.value=-1;edgeIndexes.value=[];openingFaces.value=[]}
 if(add){const ids=new Set(selectedIds.value);ids.has(id)?ids.delete(id):ids.add(id);const all=[...ids];selection.value=all[0]??'';extraSelection.value=all.slice(1)}
 else { selection.value=id;extraSelection.value=[] }
 mode.value=pane
 const sketch=document.value.sketches.find(s=>s.id===id);if(sketch){if(!samePlane(sketch.plane,activePlane.value))workplaneOutline.value=[];activePlane.value=sketch.plane??xyPlane();workplaneBodyId.value=sketch.supportBodyId??''}
}
function surfaceInputLabel(i:number) {
 if(advancedOp.value==='nurbs-patch')return [label('Низ · vMin','Bottom · vMin'),label('Верх · vMax','Top · vMax'),label('Слева · uMin','Left · uMin'),label('Справа · uMax','Right · uMax')][i]
 return advancedOp.value==='nurbs-sweep'?(i===0?label('Профиль','Profile'):label('Путь','Path')):label('Сечение ','Section ')+(i+1)
}
function beginAdvanced(kind:typeof advancedOp.value) {
 if(['push','chamfer','edge-fillet','shell','split'].includes(kind??'')) {
  const instance=document.value.bodies.find(body=>body.instance&&selectedIds.value.includes(body.id))
  if(instance)throw Error(label('Измените источник или отсоедините экземпляр: ','Edit the source or detach the instance: ')+instance.name)
 }
 boxSelect.value=false
 cancelCommand(); advancedOp.value=kind
 if(kind==='nurbs-offset'){loftPreviewId.value=crypto.randomUUID();mode.value='3d'}
 if(kind==='nurbs-point-trim'){
  pointTrimPick.value=null
  const points=selectedNurbsCurve.value?curvePoints(selectedNurbsCurve.value.curve):[],point=points[Math.floor(points.length/2)]??[0,0,0]
  advanced.value={...advanced.value,x:point[0],y:point[1],z:point[2]??0,end:'start',profileGap:.01};mode.value='3d'
 }
 if(isProfileCommand(kind)||kind==='offset'){surfaceInputs.value=[...selectedIds.value];activePlane.value=structuredClone(selectedSketch.value?.plane??xyPlane());sketchPaneOpen.value=true;mode.value='2d'}
 if(kind==='nurbs-match'||kind==='nurbs-prepare'||kind==='nurbs-curve-match'){surfaceInputs.value=[...selectedIds.value];mode.value='3d'}
 if(kind==='instance-transform'){advanced.value={...advanced.value,x:0,y:0,z:0,angle:0,scale:1,axis:'z'};mode.value='3d'}
 if(kind==='loft'||kind==='nurbs-loft'||kind==='nurbs-sweep'||kind==='nurbs-patch'){loftPreviewId.value=crypto.randomUUID();mode.value='3d';surfaceInputs.value=[...selectedIds.value];surfaceReversed.value=surfaceInputs.value.map(()=>false)}
 if(kind==='instance-create'){loftPreviewId.value=crypto.randomUUID();advanced.value={...advanced.value,x:10,y:0,z:0};mode.value='3d'}
 if(kind==='instance-place'&&selectedBody.value?.instance){const m=selectedBody.value.instance.matrix;advanced.value={...advanced.value,x:m[0][3],y:m[1][3],z:m[2][3]};mode.value='3d'}
 if(kind==='curve'&&selectedSketch.value?.analytic){const a=selectedSketch.value.analytic;advanced.value={...advanced.value,cx:a.center[0],cy:a.center[1],radius:a.radius,start:a.start,sweep:a.sweep}}
}
function cancelFaceSketch(){faceSketchGeneration++;faceSketchWorker.cancel();faceSketchPending.value=false}
onUnmounted(()=>{cancelFaceSketch();faceSketchWorker.dispose()})
async function faceSketch(){
 if(!selectedBody.value||!selectedFace.value)return
 cancelGesture();operation.value=null;error.value=''
 const body=selectedBody.value,source=document.value,face=faceIndex.value,generation=faceSketchGeneration
 faceSketchPending.value=true
 try{
  const {plane,outline,normal}=await faceSketchWorker.run({kind:'faceSketch',body,face})
  if(generation!==faceSketchGeneration||document.value!==source||selectedBody.value!==body||faceIndex.value!==face)return
  workplaneBodyId.value=body.id;choosingSketchFace.value=false
  const projectedNormal=projectDirectPoint(normal,camera.value)
  if(Math.abs(projectedNormal[2])<.08)camera.value={yaw:Math.atan2(normal[0],normal[1]),pitch:Math.asin(normal[2])}
  workplaneOutline.value=outline
  activePlane.value=plane;extraSelection.value=[];selection.value='';tool.value='rectangle';mode.value='3d';sketchPaneOpen.value=true;advancedOp.value=null
  const uv=outline.flat(),xs=uv.map(p=>p[0]),ys=uv.map(p=>p[1]);centers.value['2d']=[(Math.min(...xs)+Math.max(...xs))/2,-(Math.min(...ys)+Math.max(...ys))/2];views.value['2d']=Math.max(10,Math.max(Math.max(...xs)-Math.min(...xs),Math.max(...ys)-Math.min(...ys))*1.4)
 }catch(e){if(generation===faceSketchGeneration)error.value=e instanceof Error?e.message:String(e)}
 finally{if(generation===faceSketchGeneration)faceSketchPending.value=false}
}
function chooseSketchFace() {
  if(selectedBody.value&&selectedFace.value){faceSketch();return}
  cancelGesture();operation.value=null;advancedOp.value=null;tool.value='select'
  choosingSketchFace.value=true;pickMode.value='face';mode.value='3d'
}
function resetWorkplane() { cancelGesture();activePlane.value=xyPlane();workplaneOutline.value=[];workplaneBodyId.value='';choosingSketchFace.value=false;selection.value='';extraSelection.value=[] }
const volumeDistanceOpen=ref(false),volumeContact=shallowRef<Vec3[]|null>(null)
const measurementOpen=ref(false),measureTarget=ref(''),measureA=ref(1),measureB=ref(2),curveParameter=ref(.5)
const formatMeasurement=(value:number)=>value!==0&&Math.abs(value)<1e-6?value.toExponential(3):value.toFixed(6)
const clearanceOpen=ref(false)
const measurementTarget=computed(()=>measureTarget.value?document.value.bodies.find(body=>body.id===measureTarget.value):selectedBody.value)
const clearanceWorker=createSolidClearanceWorker()
const clearanceMeasurement=shallowRef<{value:CadPairReport|null;error:string}|null>(null)
const clearancePending=ref(false)
onUnmounted(()=>clearanceWorker.dispose())
watchEffect(onCleanup=>{
 const enabled=props.open&&measurementOpen.value&&clearanceOpen.value
 const source=selectedBody.value,target=measurementTarget.value,snapshot=document.value
 const sourceId=selection.value,targetId=measureTarget.value
 let current=true
 onCleanup(()=>{current=false;clearanceWorker.cancel()})
 clearanceMeasurement.value=null;clearancePending.value=false
 if(!enabled||!source)return
 if(!target||target.id===source.id){clearanceMeasurement.value={value:null,error:label('Выберите другое тело B.','Choose a different body B.')};return}
 clearancePending.value=true
 const valid=()=>current&&props.open&&document.value===snapshot&&selection.value===sourceId&&measureTarget.value===targetId&&measurementOpen.value&&clearanceOpen.value
 void clearanceWorker.run({kind:'inspect',bodies:[source,target].map(({id,name,mesh})=>({id,name,mesh}))}).then(reports=>{
  if(valid())clearanceMeasurement.value={value:reports[0],error:''}
 }).catch(e=>{if(valid())clearanceMeasurement.value={value:null,error:e instanceof Error?e.message:String(e)}})
 .finally(()=>{if(valid())clearancePending.value=false})
})
const measurementWorker=createSolidPreviewWorker(),curvatureWorker=createSolidPreviewWorker()
const measurementPending=ref(false),curvaturePending=ref(false)
const measurement=shallowRef<{value:PointMeasurement|null;error:string}|null>(null)
const curvatureMeasurement=shallowRef<{value:CurveMeasurement|null;error:string}|null>(null)
onUnmounted(()=>{measurementWorker.dispose();curvatureWorker.dispose()})
watchEffect(onCleanup=>{
 const enabled=props.open&&measurementOpen.value&&!clearanceOpen.value&&!volumeDistanceOpen.value
 const source=selectedBody.value,target=measurementTarget.value,a=measureA.value-1,b=measureB.value-1
 let current=true
 onCleanup(()=>{current=false;measurementWorker.cancel()})
 measurement.value=null;measurementPending.value=false
 if(!enabled||!source)return
 if(!target){measurement.value={value:null,error:label('Выберите существующее тело B.','Choose an existing target body.')};return}
 measurementPending.value=true
 void measurementWorker.run({kind:'measureVertices',a:source,indexA:a,b:target,indexB:b}).then(value=>{
  if(current)measurement.value={value,error:''}
 }).catch(e=>{if(current)measurement.value={value:null,error:e instanceof Error?e.message:String(e)}})
 .finally(()=>{if(current)measurementPending.value=false})
})
watchEffect(onCleanup=>{
 const enabled=props.open&&measurementOpen.value,body=selectedBody.value,edge=edgeIndex.value,parameter=curveParameter.value
 let current=true
 onCleanup(()=>{current=false;curvatureWorker.cancel()})
 curvatureMeasurement.value=null;curvaturePending.value=false
 if(!enabled||!body?.brep||edge<0)return
 curvaturePending.value=true
 void curvatureWorker.run({kind:'measureEdge',body,edge,parameter}).then(value=>{
  if(current)curvatureMeasurement.value={value,error:''}
 }).catch(e=>{if(current)curvatureMeasurement.value={value:null,error:e instanceof Error?e.message:String(e)}})
 .finally(()=>{if(current)curvaturePending.value=false})
})
const edgeDistanceOpen=ref(false),edgeDistanceA=ref(1),edgeDistanceB=ref(2),edgeDistanceBudget=ref(10000)
const edgeDistanceWorker=createSolidPreviewWorker(),edgeDistancePending=ref(false)
const edgeDistance=shallowRef<{value:NurbsCurveDistance|null;error:string}|null>(null)
onUnmounted(()=>edgeDistanceWorker.dispose())
watchEffect(onCleanup=>{
 const enabled=props.open&&measurementOpen.value&&edgeDistanceOpen.value
 const source=selectedBody.value,target=measurementTarget.value,a=edgeDistanceA.value-1,b=edgeDistanceB.value-1,maxCells=edgeDistanceBudget.value
 let current=true
 onCleanup(()=>{current=false;edgeDistanceWorker.cancel()})
 edgeDistance.value=null;edgeDistancePending.value=false
 if(!enabled||!source)return
 if(!source.brep||!target?.brep){edgeDistance.value={value:null,error:label('Выберите два тела с B-rep геометрией.','Choose two bodies with B-rep geometry.')};return}
 if(!Number.isInteger(a)||!source.brep.edges[a]){edgeDistance.value={value:null,error:label('Укажите существующее ребро A.','Choose an existing edge A.')};return}
 if(!Number.isInteger(b)||!target.brep.edges[b]){edgeDistance.value={value:null,error:label('Укажите существующее ребро B.','Choose an existing edge B.')};return}
 edgeDistancePending.value=true
 void edgeDistanceWorker.run({kind:'curveDistance',a:source.brep.edges[a].curve,b:target.brep.edges[b].curve,toleranceMm:.001,maxCells}).then(value=>{
  if(current)edgeDistance.value={value,error:''}
 }).catch(()=>{if(current)edgeDistance.value={value:null,error:label('Не удалось измерить выбранные рёбра. Проверьте геометрию или увеличьте объём расчёта.','Could not measure these edges. Check their geometry or increase the calculation budget.')}})
 .finally(()=>{if(current)edgeDistancePending.value=false})
})
const faceDistanceOpen=ref(false),faceDistanceA=ref(1),faceDistanceB=ref(2),faceDistanceBudget=ref(10000)
const faceDistanceWorker=createSolidPreviewWorker(),faceDistancePending=ref(false)
const faceDistance=shallowRef<{value:FaceDistanceResult|null;error:string}|null>(null)
onUnmounted(()=>faceDistanceWorker.dispose())
watchEffect(onCleanup=>{
 const enabled=props.open&&measurementOpen.value&&faceDistanceOpen.value
 const source=selectedBody.value,target=measurementTarget.value,a=faceDistanceA.value-1,b=faceDistanceB.value-1,maxCells=faceDistanceBudget.value
 let current=true
 onCleanup(()=>{current=false;faceDistanceWorker.cancel()})
 faceDistance.value=null;faceDistancePending.value=false
 if(!enabled||!source)return
 if(!source.brep||!target?.brep){faceDistance.value={value:null,error:label('Выберите два тела с B-rep геометрией.','Choose two bodies with B-rep geometry.')};return}
 if(!Number.isInteger(a)||!source.brep.faces[a]){faceDistance.value={value:null,error:label('Укажите существующую грань A.','Choose an existing face A.')};return}
 if(!Number.isInteger(b)||!target.brep.faces[b]){faceDistance.value={value:null,error:label('Укажите существующую грань B.','Choose an existing face B.')};return}
 faceDistancePending.value=true
 void faceDistanceWorker.run({kind:'faceDistance',options:{a:source.brep,b:target.brep,faceA:a,faceB:b,toleranceMm:.001,toleranceUv:1e-7,maxCells,maxDomainCells:maxCells===10000?1000000:8000000}}).then(value=>{
  if(current)faceDistance.value={value,error:''}
 }).catch(()=>{if(current)faceDistance.value={value:null,error:label('Не удалось измерить выбранные грани. Проверьте геометрию или увеличьте объём расчёта.','Could not measure these faces. Check their geometry or increase the calculation budget.')}})
 .finally(()=>{if(current)faceDistancePending.value=false})
})
const shellDistanceOpen=ref(false),shellDistanceBudget=ref(100000)
const shellDistanceWorker=createSolidPreviewWorker(),shellDistancePending=ref(false)
const shellDistance=shallowRef<{value:ShellDistanceResult|null;error:string}|null>(null)
onUnmounted(()=>shellDistanceWorker.dispose())
watchEffect(onCleanup=>{
 const enabled=props.open&&measurementOpen.value&&shellDistanceOpen.value
 const source=selectedBody.value,target=measurementTarget.value,maxCells=shellDistanceBudget.value
 let current=true
 onCleanup(()=>{current=false;shellDistanceWorker.cancel()})
 shellDistance.value=null;shellDistancePending.value=false
 if(!enabled||!source)return
 if(!source.brep||!target?.brep){shellDistance.value={value:null,error:label('Выберите два тела с B-rep геометрией.','Choose two bodies with B-rep geometry.')};return}
 if(source.id===target.id){shellDistance.value={value:null,error:label('Выберите другое тело B.','Choose a different body B.')};return}
 shellDistancePending.value=true
 void shellDistanceWorker.run({kind:'shellDistance',options:{a:source.brep,b:target.brep,toleranceMm:.001,toleranceUv:1e-7,maxCells,maxDomainCells:maxCells===100000?1000000:8000000}}).then(value=>{
  if(current)shellDistance.value={value,error:''}
 }).catch(()=>{if(current)shellDistance.value={value:null,error:label('Не удалось измерить оболочки тел. Проверьте геометрию или увеличьте объём расчёта.','Could not measure these boundary shells. Check their geometry or increase the calculation budget.')}})
 .finally(()=>{if(current)shellDistancePending.value=false})
})
const diagnosticPanel=ref<HTMLElement>()
const diagnosticsOpen=ref(false),sectionZ=ref(5),sectionNormal=ref([0,0,1]),diagnosticsRevision=ref(0)
const boundaryOpen=ref(false),boundaryBudget=ref(10000),boundaryRevision=ref(0),boundaryAgreementPending=ref(false),boundaryIndex=ref(0)
const boundaryResult=shallowRef<BoundaryAgreement|null>(null),boundaryError=ref('')
const boundaryAgreementWorker=createSolidPreviewWorker()
onUnmounted(()=>boundaryAgreementWorker.dispose())
const boundaryBudgetInvalid=computed(()=>!Number.isInteger(boundaryBudget.value)||boundaryBudget.value<1||boundaryBudget.value>1000000)
watchEffect(onCleanup=>{
 void boundaryRevision.value
 const enabled=props.open&&diagnosticsOpen.value&&boundaryOpen.value,body=selectedBody.value,snapshot=document.value,maxCells=boundaryBudget.value,invalid=boundaryBudgetInvalid.value
 let current=true
 onCleanup(()=>{current=false;boundaryAgreementWorker.cancel()})
 boundaryResult.value=null;boundaryError.value='';boundaryAgreementPending.value=false;boundaryIndex.value=0
 if(!enabled||!body)return
 if(invalid){boundaryError.value=label('Введите целый лимит от 1 до 1000000.','Enter an integer limit from 1 to 1000000.');return}
 if(!body.brep){boundaryError.value=label('Выберите тело с исходной B-rep геометрией.','Choose a body with original B-rep geometry.');return}
 boundaryAgreementPending.value=true
 const valid=()=>current&&document.value===snapshot&&selectedBody.value?.id===body.id
 void boundaryAgreementWorker.run({kind:'boundaryAgreement',model:body.brep,maxCells}).then(value=>{if(valid())boundaryResult.value=value})
 .catch(()=>{if(valid())boundaryError.value=label('Не удалось проверить границы этого тела. Повторите проверку; при повторном отказе проверьте структуру модели.','Could not inspect this body’s boundaries. Retry; if it fails again, inspect the model structure.')})
 .finally(()=>{if(valid())boundaryAgreementPending.value=false})
})
const boundaryDefects=computed(()=>boundaryResult.value?.uses.filter(u=>u.status==='mismatch')??[])
const boundarySelected=computed(()=>boundaryDefects.value[boundaryIndex.value])
const boundaryLine=computed(()=>boundaryResult.value?.lines.find(line=>line.edge===boundarySelected.value?.edge))

const faceContactsOpen=ref(false),faceContactsBudget=ref(10000),faceContactsRevision=ref(0),faceContactsPending=ref(false),faceContactIndex=ref(0)
const inspectWithinFaces=ref(false)
const faceContactsResult=shallowRef<FaceContacts|SelfIntersection|null>(null),faceContactsError=ref('')
const faceContactsWorker=createSolidPreviewWorker()
onUnmounted(()=>faceContactsWorker.dispose())
const faceContactsBudgetInvalid=computed(()=>!Number.isInteger(faceContactsBudget.value)||faceContactsBudget.value<1||faceContactsBudget.value>1000000)
watchEffect(onCleanup=>{
 void faceContactsRevision.value
 const enabled=props.open&&diagnosticsOpen.value&&faceContactsOpen.value,body=selectedBody.value,snapshot=document.value,maxCells=faceContactsBudget.value,invalid=faceContactsBudgetInvalid.value
 let current=true
 onCleanup(()=>{current=false;faceContactsWorker.cancel()})
 faceContactsResult.value=null;faceContactsError.value='';faceContactsPending.value=false;faceContactIndex.value=0
 if(!enabled||!body)return
 if(invalid){faceContactsError.value=label('Введите целый лимит от 1 до 1000000.','Enter an integer limit from 1 to 1000000.');return}
 if(!body.brep){faceContactsError.value=label('Выберите тело с исходной B-rep геометрией.','Choose a body with original B-rep geometry.');return}
 faceContactsPending.value=true
 const valid=()=>current&&document.value===snapshot&&selectedBody.value?.id===body.id
 void faceContactsWorker.run({...inspectWithinFaces.value?{kind:'selfIntersection' as const,maxSpans:Math.min(maxCells,100000)}:{kind:'faceContacts' as const},model:body.brep,toleranceUv:1e-8,limits:{maxPairs:10000,maxCells,maxDomainCells:Math.min(8000000,maxCells*100),cellsPerPair:Math.min(100000,Math.max(1,Math.floor(maxCells/20))),domainCellsPerPair:Math.min(1000000,Math.max(1,maxCells*5)),maxBoxes:64}})
 .then(value=>{if(valid())faceContactsResult.value=value})
 .catch(()=>{if(valid())faceContactsError.value=label('Не удалось проверить контакты граней. Повторите проверку; при повторном отказе проверьте структуру модели.','Could not inspect face contacts. Retry; if it fails again, inspect the model structure.')})
 .finally(()=>{if(valid())faceContactsPending.value=false})
})
const faceContactDefects=computed(()=>faceContactsResult.value?.pairs.filter(p=>p.witness!==null)??[])
const faceContactSelected=computed(()=>faceContactDefects.value[faceContactIndex.value])
const faceContactPoint=computed(()=>faceContactSelected.value?.witness?.pointIntervalMm.map(d=>d[0]*.5+d[1]*.5))

const sectionInputErrors=ref<Record<string,boolean>>({})
function sectionValidity(key:string,valid:boolean){if(valid)delete sectionInputErrors.value[key];else sectionInputErrors.value[key]=true}
const sectionInputInvalid=computed(()=>Object.keys(sectionInputErrors.value).length>0)
function sectionErrorMessage(message:string){
 if(message.includes('Section normal'))return label('Нормаль плоскости нулевая или вне допустимого диапазона. Измените X, Y или Z либо выберите плоскость XY.','The plane normal is zero or outside the numeric range. Change X, Y or Z, or choose the XY plane.')
 if(message.includes('Section offset'))return label('Задайте конечное смещение плоскости.','Enter a finite plane offset.')
 return label('Не удалось построить сечение. Измените плоскость или исправьте выделенные дефекты сетки.','Could not build the section. Change the plane or repair the highlighted mesh defects.')
}
const displayInspectionWorker=createSolidPreviewWorker(),diagnosticsPending=ref(false)
const diagnostics=shallowRef<{value:ReturnType<typeof inspectSolidDisplay>|null;error:string}|null>(null)
onUnmounted(()=>displayInspectionWorker.dispose())
watchEffect(onCleanup=>{
 void diagnosticsRevision.value
 const enabled=props.open&&diagnosticsOpen.value&&!sectionInputInvalid.value,body=selectedBody.value,snapshot=document.value,offset=sectionZ.value,normal=[...sectionNormal.value]
 let current=true
 onCleanup(()=>{current=false;displayInspectionWorker.cancel()})
 diagnostics.value=null;diagnosticsPending.value=false
 if(!enabled||!body)return
 diagnosticsPending.value=true
 const valid=()=>current&&props.open&&diagnosticsOpen.value&&document.value===snapshot&&selectedBody.value?.id===body.id
 void displayInspectionWorker.run({kind:'brepTool',document:snapshot,options:{kind:'display',id:body.id,offset,normal}}).then(result=>{
  if(valid()&&result.kind==='display')diagnostics.value={value:result.diagnostics,error:''}
 }).catch(e=>{if(valid())diagnostics.value={value:null,error:e instanceof Error?e.message:String(e)}})
 .finally(()=>{if(valid())diagnosticsPending.value=false})
})
const intersectionWorker=createSolidClearanceWorker()
const intersectionInspection=shallowRef<{value:ReturnType<typeof inspectSolidIntersections>|null;error:string}|null>(null)
const intersectionPending=ref(false),intersectionWork=ref(200_000),intersectionIndex=ref(0)
const selectedIntersection=computed(()=>intersectionInspection.value?.value?.contacts[intersectionIndex.value]??null)
const selectedIntersectionLines=computed(()=>{
 const report=intersectionInspection.value?.value,contact=selectedIntersection.value
 return report&&contact?contact.triangles.map(t=>report.lines[report.triangleIds.indexOf(t)]):[]
})
onUnmounted(()=>intersectionWorker.dispose())
watchEffect(onCleanup=>{
 void diagnosticsRevision.value
 const enabled=props.open&&diagnosticsOpen.value,body=selectedBody.value,snapshot=document.value,maxWork=intersectionWork.value
 let current=true
 onCleanup(()=>{current=false;intersectionWorker.cancel()})
 intersectionInspection.value=null;intersectionPending.value=false;intersectionIndex.value=0
 if(!enabled||!body)return
 intersectionPending.value=true
 const valid=()=>current&&props.open&&diagnosticsOpen.value&&document.value===snapshot&&selectedBody.value?.id===body.id
 void intersectionWorker.run({kind:'meshContacts',mesh:body.mesh,maxWork,maxContacts:maxWork>=8_000_000?100_000:maxWork>=1_000_000?50_000:10_000}).then(value=>{
  if(valid())intersectionInspection.value={value,error:''}
 }).catch(e=>{if(valid())intersectionInspection.value={value:null,error:e instanceof Error?e.message:String(e)}})
 .finally(()=>{if(valid())intersectionPending.value=false})
})
const collapsedSectionPoints=computed(()=>{
 const section=diagnostics.value?.value?.section
 if(!section)return []
 const sources=new Set(section.collapsedSegmentTriangles)
 return section.contours.flatMap(contour=>contour.sourceTriangles.flatMap((source,i)=>sources.has(source)?[contour.points[i]]:[]))
})
const brepMass=shallowRef<BrepMassProperties|null>(null)
watch(selectedBody,()=>{brepMass.value=null})
function measureSelectedBrep(){void runNativeBrep({kind:'mass',id:selection.value})}
function retessellateSelectedBrep(){void runNativeBrep({kind:'mesh',id:selection.value,segments:brepSegments.value})}
async function runNativeBrep(options:Exclude<SolidBrepToolOptions,{kind:'display'}>){
 cancelCommand();error.value=''
 const generation=++nativeNurbsGeneration
 nativeBrepMode.value=options.kind;nativeNurbsPending.value=true
 try{
  const result=await nativeNurbsWorker.run({kind:'brepTool',document:history.document,options})
  if(generation!==nativeNurbsGeneration||!props.open)return
  nativeNurbsPending.value=false;nativeBrepMode.value=null
  if(result.kind==='mass')brepMass.value=result.mass
  else if(result.kind==='mesh')commit(result.document)
 }catch(e){if(generation===nativeNurbsGeneration)error.value=e instanceof Error?e.message:String(e)}
 finally{if(generation===nativeNurbsGeneration){nativeNurbsPending.value=false;nativeBrepMode.value=null}}
}
const FALLBACK_NOTICE=()=>label('Одно из тел не имело точной топологии, результат построен по сетке и не является точным B-rep.','A body had no exact topology; the result was built from the mesh and is not an exact B-rep.')
const EMPTY_NOTICE=()=>label('Результат пуст: тела не пересекаются так, как требует операция.','The result is empty: the bodies do not overlap the way this operation needs.')
const formatMm=(mm:number)=>mm>=1e-3?mm.toPrecision(2):mm.toExponential(1)
const TOLERANT_NOTICE=(mm:number)=>label(`Точного решения для этой пары нет; кривая пересечения построена численно с допуском ${formatMm(mm)} мм.`,`No exact solution exists for this pair; the intersection curve was traced numerically within ${formatMm(mm)} mm.`)
const booleanWorker=createSolidPreviewWorker(),booleanPending=ref(false)
let booleanGeneration=0
function cancelBoolean(){booleanGeneration++;booleanWorker.cancel();booleanPending.value=false}
async function runBoolean(options:SolidBooleanOptions,guided=false){
 if(booleanPending.value)return
 cancelBoolean()
 const generation=booleanGeneration,snapshot=guided?captureCommand():null
 error.value='';notice.value='';booleanPending.value=true
 try{
  const result=await booleanWorker.run({kind:'boolean',document:document.value,options})
  if(generation!==booleanGeneration)return
  // Commit triggers the document watcher, so clear pending before publishing the transaction.
  booleanPending.value=false
  commit(result.document);selection.value=result.resultId??'';extraSelection.value=[]
  if(guided){subtract.value=null;if(snapshot)lastCommand.value=snapshot}
  if(result.resultId===null)notice.value=EMPTY_NOTICE()
  else if(!result.exact)notice.value=FALLBACK_NOTICE()
  else if(result.toleranceMm>0)notice.value=TOLERANT_NOTICE(result.toleranceMm)
 }catch(e){if(generation===booleanGeneration)error.value=bodyCalculationFailure(e,false)}
 finally{if(generation===booleanGeneration)booleanPending.value=false}
}
function applyBrepBoolean(operation:BrepBooleanOperation){run(()=>{
 if(booleanPending.value)return
 cancelCommand()
 const ids=selectedIds.value.filter(id=>document.value.bodies.some(b=>b.id===id))
 if(ids.length!==2)throw Error(label('Выберите ровно два тела: первое выбранное — A.','Select exactly two bodies; the first selected body is A.'))
 void runBoolean({operation,a:[ids[0]],b:[ids[1]],segments:brepSegments.value,ru:ru.value})
})}

/**
 * Guided subtraction: field A collects the bodies to keep, field B the bodies to remove.
 *
 * Bodies clicked in the viewport or the scene list go into the active field, Shift adds
 * or removes one. Each field is unioned first, then A minus B runs once, so several
 * cutters can be removed from several parts in a single step.
 */
const subtract=ref<{a:string[];b:string[];active:'a'|'b'}|null>(null)
function beginSubtract(){
 cancelCommand()
 const bodyIds=new Set(snapDocument.value.bodies.map(body=>body.id))
 const ids=selectedIds.value.filter(id=>bodyIds.has(id))
 subtract.value={a:ids.slice(0,1),b:ids.slice(1),active:ids.length?'b':'a'}
}
function subtractPick(id:string,add:boolean){
 const s=subtract.value;if(!s)return
 const field=s.active,other=field==='a'?'b':'a'
 s[field]=add?(s[field].includes(id)?s[field].filter(x=>x!==id):[...s[field],id]):[id]
 s[other]=s[other].filter(x=>!s[field].includes(x))
 const all=[...s.a,...s.b];selection.value=all[0]??'';extraSelection.value=all.slice(1)
}
function subtractNames(ids:string[]){
 const names=ids.map(id=>document.value.bodies.find(b=>b.id===id)?.name).filter((n):n is string=>!!n)
 return names.length?names.join(', '):label('пусто','empty')
}
function applySubtract(){
 const s=subtract.value;if(!s)return
 void runBoolean({operation:'difference',a:[...s.a],b:[...s.b],segments:brepSegments.value,ru:ru.value},true)
}
watch(()=>[props.open,document.value,JSON.stringify(selectedIds.value),JSON.stringify(subtract.value),brepSegments.value,ru.value],()=>{
 if(booleanPending.value)cancelBoolean()
},{flush:'sync'})
onUnmounted(()=>{cancelBoolean();booleanWorker.dispose()})
const rebuildPeriodic=computed(()=>advancedOp.value==='nurbs-rebuild'?!!selectedNurbsCurve.value?.curve.periodic:advancedOp.value==='nurbs-surface-rebuild'?!!(advanced.value.surfaceAxis==='u'?selectedNurbsSurface.value?.surface.periodicU:selectedNurbsSurface.value?.surface.periodicV):false)
const nurbsRefitResult=shallowRef<ReturnType<typeof refitSolidNurbs>|null>(null)
const nurbsReduction=computed(()=>isSolidNurbsRefit(advancedOp.value)?{value:nurbsRefitResult.value,error:bodyEditError.value}:null)
const curveMatchResult=shallowRef<ReturnType<typeof matchSolidCurve>|null>(null),surfaceMatchResult=shallowRef<ReturnType<typeof matchSolidSurface>|null>(null),seamPrepareResult=shallowRef<ReturnType<typeof prepareSolidSurfaceSeams>|null>(null)
const isNurbsMatch=(op:unknown)=>['nurbs-curve-match','nurbs-match','nurbs-prepare'].includes(String(op))
const surfacePreparation=computed(()=>{
 if(advancedOp.value!=='nurbs-prepare')return null
 const message=bodyEditError.value
 return {value:seamPrepareResult.value,error:message.includes('matching periodicity')?label('Для смешанной пары включите «Снять периодическую связь». Форма сохранится в допуске, концы станут независимыми.','For a mixed pair, enable “Remove periodic linkage”. Shape stays within tolerance; the endpoints become independent.'):message}
})
function preparationError(reason:string){return reason==='unproven-parameter-normalization'?label('Не подтверждена точность преобразования параметров. Автоматическая подготовка этих узлов пока недоступна.','Parameter conversion accuracy is unconfirmed. Automatic preparation of these knots is currently unavailable.'):label('Отклонение подготовки превышает допуск. Увеличьте допуск или измените входы.','Preparation deviation exceeds the tolerance. Increase the tolerance or change the inputs.')}
function profilePreparationError(reason:string,kind?:string){
 if(kind==='intersection')return label('Отмеченные сегменты пересекаются. Обрежьте их или измените точки, затем повторите сборку.','The marked segments intersect. Trim them or edit the points, then prepare the profile again.')
 if(kind==='overlap')return label('Отмеченные сегменты перекрываются. Удалите дублирующий участок или измените точки.','The marked segments overlap. Remove the duplicate section or edit the points.')
 if(kind==='unproven')return label('Проверка отмеченных сегментов не завершена. Упростите контур или разделите его на части.','The marked segments could not be verified. Simplify the contour or split it into parts.')
 if(reason==='endpoint-topology')return label('Есть разрывы или неоднозначные соединения. Проверьте отмеченные концы и допуск.','There are gaps or ambiguous connections. Check the marked endpoints and tolerance.')
 if(reason==='disconnected')return label('Выбрано несколько отдельных циклов. Соберите каждый профиль отдельно.','Multiple separate cycles are selected. Prepare each profile separately.')
 return label('Не удалось подтвердить корректность контура. Проверьте пересечения и короткие соединения.','Could not validate the contour. Check intersections and short connectors.')
}
const preparedProfileResult=shallowRef<ReturnType<typeof prepareSolidProfile>|null>(null)
const profilePreparation=computed(()=>{
 if(advancedOp.value!=='profile-prepare')return null
 const message=bodyEditError.value
 return {value:preparedProfileResult.value,error:message.includes('same sketch plane')?label('Все линии должны использовать одну плоскость эскиза. Перенесите их в общую плоскость.','All lines must use one sketch plane. Move them into a common plane.'):message.includes('open polylines')?label('Выберите открытые ломаные или дуги в одной плоскости.','Select open polylines or arcs in one plane.'):message}
})
function curveMatchError(reason:string){
 if(reason==='unproven-endpoint-regularity')return label('Касательная вырождена. Измените соседнюю управляющую точку.','The tangent is degenerate. Move the adjacent control point.')
 if(reason==='unproven-tangent-orientation')return label('Направление касательной не подтверждено. Выберите другой конец.','Tangent orientation is unconfirmed. Choose another endpoint.')
 return label('Угловая ошибка превышает допуск. Увеличьте допуск или измените входы.','Angular error exceeds tolerance. Increase the tolerance or change the inputs.')
}
const curveMatch=computed(()=>{
 if(advancedOp.value!=='nurbs-curve-match')return null
 const message=bodyEditError.value
 return {value:curveMatchResult.value,error:message.includes('non-periodic')?label('Выберите конец непериодической кривой. У периодической кривой нет независимого конца.','Choose a non-periodic curve endpoint. Periodic curves have no independent endpoint.'):message.includes('exactly clamped')?label('Конец должен быть зажат узлами. Подготовьте или обрежьте кривую.','The endpoint must be clamped. Prepare or trim the curve.'):message.includes('degenerate')?curveMatchError('unproven-endpoint-regularity'):message}
})
const curveMatchGuides=computed(()=>{
 if(advancedOp.value!=='nurbs-curve-match')return []
 const result=curveMatch.value?.value
 return surfaceInputs.value.slice(0,2).flatMap((id,i)=>{
  const curve=(result?.report.accepted?result.document:document.value).curves?.find(c=>c.id===id)?.curve
  if(!curve)return []
  const end=i===0?advanced.value.curveEndA:advanced.value.curveEndB,n=curve.controlPoints.length,k=end==='start'?0:n-1,j=end==='start'?1:n-2
  const point=(index:number)=>{const p=curve.controlPoints[index];return project([p[0],p[1],p[2]??0],'3d')}
  const joint=point(k),handle=point(j)
  return [{id:i,x:joint[0],y:joint[1],points:[joint,handle].map(p=>p.join(',')).join(' '),color:curveMatch.value?.error||result&&!result.report.accepted?'#ff647c':i===0?'#89baff':'#ffc977'}]
 })
})
const surfaceMatch=computed(()=>{
 if(advancedOp.value!=='nurbs-match')return null
 return {value:surfaceMatchResult.value,error:bodyEditError.value}
})
const surfaceMatchLines=computed(()=>{
 if(advancedOp.value!=='nurbs-match'&&advancedOp.value!=='nurbs-prepare')return []
 const result=surfaceMatch.value?.value,p=advanced.value
 return surfaceInputs.value.slice(0,2).flatMap((id,i)=>{
  const prepared=surfacePreparation.value?.value
  const source=(prepared?.report.accepted?prepared.document:document.value).surfaces?.find(s=>s.id===id)?.surface
  const surface=i===1&&result?.report.accepted?result.candidate:source
  if(!surface)return []
  const boundary=i===0?p.matchBoundaryA:p.matchBoundaryB,crossU=boundary[0]==='u',atMax=boundary.endsWith('Max')
  const ku=surface.knotsU,kv=surface.knotsV,nu=surface.controlPoints.length,nv=surface.controlPoints[0].length
  const start=crossU?kv[surface.degreeV]:ku[surface.degreeU],end=crossU?kv[nv]:ku[nu]
  const cross=crossU?ku[atMax?nu:surface.degreeU]:kv[atMax?nv:surface.degreeV]
  const line=(lo:number,hi:number)=>Array.from({length:33},(_,j)=>{const t=lo+(hi-lo)*j/32;return project(evaluateNurbsSurface(surface,crossU?cross:t,crossU?t:cross).point,'3d').join(',')}).join(' ')
  try{
   const regularity=i===0?result?.report.referenceRegularity:result?.report.editedRegularity
   // Edited unresolved intervals describe the candidate, which is not displayed on refusal.
   const intervals=i===0?regularity?.unresolvedIntervals??[]:[]
   return [{key:String(i),points:line(start,end),color:i===0?'#89baff':'#ffc977'},...intervals.map(([a,b],j)=>({key:`${i}-${j}`,points:line(a,b),color:'#ff647c'}))]
  }catch{return []}
 })
})
function surfaceMatchError(reason:string){
 if(reason==='unproven-regularity')return label('Регулярность шва не подтверждена. Выберите другую границу или исправьте вырождение.','Seam regularity is unconfirmed. Choose another boundary or repair the singularity.')
 if(reason==='unproven-tangential-smoothness')return label('Гладкость вдоль шва недостаточна. Снизьте порядок или подготовьте границу.','Insufficient smoothness along the seam. Lower the order or prepare the boundary.')
 return label('Ошибка производных превышает допуск. Измените масштаб или допуск.','Derivative error exceeds the tolerance. Change the scale or tolerance.')
}
function resultAdvanced(onSweepReport?:(report:FramedSweepResult['report'])=>void):DirectDocument {
 const d=history.document,p=advanced.value,id=selection.value,b=d.bodies.find(b=>b.id===id),s=d.sketches.find(s=>s.id===id)
 switch(advancedOp.value){
  case 'profile-prepare': {const r=profilePreparation.value;if(!r?.value)throw Error(r?.error||'Profile preparation unavailable.');if(!r.value.report.accepted)throw Error(profilePreparationError(r.value.report.reason,r.value.report.segmentDefect?.kind));return r.value.document}
  case 'nurbs-curve-match': {const r=curveMatch.value;if(!r?.value)throw Error(r?.error||'Curve matching unavailable.');if(!r.value.report.accepted)throw Error(curveMatchError(r.value.report.reason));return r.value.document}
  case 'nurbs-prepare': {const r=surfacePreparation.value;if(!r?.value)throw Error(r?.error||'Seam preparation unavailable.');if(!r.value.report.accepted)throw Error(preparationError(r.value.report.reason));return r.value.document}
  case 'nurbs-match': {const r=surfaceMatch.value;if(!r?.value)throw Error(r?.error||'Surface matching unavailable.');if(!r.value.report.accepted)throw Error(surfaceMatchError(r.value.report.reason));return r.value.document}
  case 'nurbs-surface-rebuild': case 'nurbs-rebuild': case 'nurbs-reduce': case 'nurbs-surface-reduce': {
   const r=nurbsReduction.value
   if(!r?.value)throw Error(r?.error||'Reduction unavailable.')
   if(!r.value.certificate.accepted||r.value.certificate.rolledBack)throw Error(label('Отклонение превышает допуск. Измените параметры или увеличьте допуск.','Deviation exceeds the tolerance. Adjust the parameters or increase the tolerance.'))
   return r.value.document
  }

 }
 return parseDirectDocument(stringifyMeshJson(d))
}
function bodyCalculationFailure(error:unknown,hasRetryButton=true):string {
 const code=error && typeof error==='object' && 'code' in error?error.code:undefined
 if(code==='BREP_LAYERED_FILLET_REFUSED'||code==='BREP_ANNULAR_FILLET_REFUSED'||code==='BREP_IMPRINT_PIPELINE_REFUSED'||code==='BREP_EXACT_FILLET_REFUSED'||code==='BREP_EXACT_CHAMFER_REFUSED'||code==='BREP_VARIABLE_RADIUS_FILLET_REFUSED'||code==='BREP_VALENCE3_CORNER_BLEND_REFUSED'){
  const message=error instanceof Error?error.message:String(error)
  const edges=(edgeIndexes.value.length?edgeIndexes.value:[edgeIndex.value]).filter(i=>i>=0).map(i=>i+1).join(', ')
  const context=`${selectedBody.value?.name??''} · ${label('рёбра','edges')} ${edges}: `
  if(message.includes('complete corner chain'))return context+label('Выберите всю цепочку рёбер с Shift.','Select the complete edge chain with Shift.')
  if(message.includes('cavity or missing material'))return context+label('Скругление прорезает стенку. Уменьшите радиус.','Fillet crosses the wall. Reduce the radius.')
  if(message.includes('Partial circular rim'))return context+label('Выберите полное кольцо: добавьте остальные дуги с Shift.','Select a complete rim: add the remaining arcs with Shift.')
  return context+(/collid|consum|radius.*(large|fit)|distance.*(large|fit)|too (small|short) for/i.test(message)
   ?label('Размер сопряжения не помещается. Уменьшите радиус или размер фаски.','The feature does not fit. Reduce the radius or chamfer size.')
   :label('Сопряжение не подтверждено для выбранной геометрии. Проверьте условия режима ниже и выберите подходящие рёбра.','The feature is unproven for this geometry. Check the mode requirements below and select suitable edges.'))
 }
 if(code==='CAD_CRASH')return label('Вычисление прервано из-за сбоя. Модель не изменена. ','The calculation stopped unexpectedly. The model is unchanged. ')+(hasRetryButton?label('Нажмите «Повторить вычисление».','Choose Retry calculation.'):label('Повторите операцию с выбранными телами.','Run the operation again with the selected bodies.'))
 if(code==='CAD_TIMEOUT')return label('Истекло время расчёта. Модель не изменена. Упростите геометрию или повторите расчёт.','The calculation timed out. The model is unchanged. Simplify geometry or retry.')
 if(code==='CAD_TRANSPORT'||code==='CAD_PROTOCOL')return label('Не удалось получить корректный результат вычисления. Модель не изменена. Повторите вычисление.','A valid calculation result could not be received. The model is unchanged. Retry the calculation.')
 return error instanceof Error?error.message:String(error)
}
function surfaceConstructionError(error:unknown):string {
 const message=error instanceof Error?error.message:String(error)
 if(advancedOp.value==='nurbs-point-trim'){
  if(message.includes('ambiguous or unresolved'))return pointTrimPick.value?label('Попадание в проекции неоднозначно. Поверните вид или выберите другую точку.','The projected pick is ambiguous. Rotate the view or choose another point.'):label('Ближайшая точка неоднозначна. Выберите другую точку разреза.','The nearest point is ambiguous. Choose another cut point.')
  if(message.includes('capture distance'))return label('Точка вне допуска. Приблизьте её к кривой или увеличьте допуск.','Outside capture distance. Move closer to the curve or increase the distance.')
  if(message.includes('strictly inside'))return label('Это конец кривой. Выберите внутреннюю точку.','Curve endpoint. Choose an interior point.')
  if(message.includes('open curve'))return label('Нужна открытая непериодическая кривая с разными концами.','An open non-periodic curve with distinct endpoints is required.')
 }
 if(advancedOp.value==='offset'&&message.includes('contains no material'))return label('Смещение удаляет профиль. Уменьшите модуль расстояния.','Offset removes the profile. Reduce the distance magnitude.')
 if(advancedOp.value==='offset'&&message.includes('below coordinate resolution'))return label('Смещение слишком мало для точности этих координат. Увеличьте расстояние.','The offset is below coordinate resolution. Increase the distance.')
 if(advancedOp.value==='offset'&&message.includes('cusp below resolution'))return label('Радиус после смещения слишком мал для проверки. Измените расстояние.','The resulting radius is below resolution. Change the distance.')
 if(profileBooleanOperation(advancedOp.value)&&message.includes('contains no material'))return label('Пустой результат. Измените входные профили или основной профиль.','Empty result. Change profiles or target.')
 if(profileBooleanOperation(advancedOp.value)&&message.includes('common sketch plane'))return label('Профили в разных плоскостях. Перенесите их в общую плоскость.','Move the profiles into one sketch plane.')
 if(advancedOp.value==='nurbs-sweep'&&message.includes('Closed sweep requires'))return label('Для замкнутого пути задайте минимум 4 сечения.','Use at least 4 sections for a closed path.')
 if(advancedOp.value==='nurbs-sweep'&&message.includes('Sweep sampled deviation'))return label('Измеренное отклонение превышает предел. Увеличьте число сечений или измените предел. ','Sampled deviation exceeds the budget. Increase sections or change the budget. ')+message
 if(advancedOp.value==='nurbs-sweep'){
  if(message.includes('zero or nonfinite direction'))return label('Начальное направление нулевое или вдоль касательной. Измените X, Y, Z.','Initial direction is zero or along the tangent. Change X, Y, Z.')
  if(message.includes('tangent discontinuities'))return label('У пути есть разрыв касательной. Разбейте путь в месте излома.','The path has a tangent discontinuity. Split it at the corner.')
 }
 if(message==='Choose distinct curves.')return label('Для каждой роли выберите отдельную кривую.','Choose a distinct curve for each role.')
 if(advancedOp.value!=='nurbs-patch')return message
 if(message.includes('PATCH_BUDGET')){
  const role=message.match(/PATCH_BUDGET:([0-3])/)
  return (role?surfaceInputLabel(Number(role[1]))+': ':'')+label('Подготовка границ превышает допуск. Увеличьте допуск или измените границы.','Boundary preparation exceeds the budget. Increase the budget or change the boundaries.')
 }
 if(message.includes('corner weights are incompatible'))return label(
  'Несовместимые угловые веса. Включите согласование весов границ и задайте допуск.',
  'Incompatible corner weights. Enable boundary weight matching and set a budget.')
 if(message.includes('positive finite weights'))return label(
  'Положительные веса patch не подтверждены. Разбейте patch или измените границы.',
  'Positive patch weights are unproven. Split the patch or change its boundaries.')
 const corner=message.match(/Coons corner ([1-4]) does not coincide/)
 if(corner){
  const pairs=[label('низ и слева','bottom and left'),label('низ и справа','bottom and right'),label('верх и слева','top and left'),label('верх и справа','top and right')]
  return label(`Угол ${corner[1]}: границы «${pairs[Number(corner[1])-1]}» не соединены. Проверьте роли и направление кривых; соедините их концы.`,
   `Corner ${corner[1]}: ${pairs[Number(corner[1])-1]} boundaries do not meet. Check curve roles and direction, then connect their endpoints.`)
 }
 return message
}
const surfaceBuildResult=shallowRef<ReturnType<typeof buildSolidSurface>|null>(null)
const invalidQuantities = ref<Record<string, boolean>>({})
const bodyEditRevision=ref(0),bodyEditRetryVisible=ref(false)
watch(advancedOp,()=>{bodyEditRetryVisible.value=false},{flush:'sync'})
watch(()=>props.open,open=>{if(!open)advancedOp.value=null},{flush:'sync'})
const bodyEditWorker=createSolidPreviewWorker()
let bodyEditGeneration=0,bodyEditApplyGeneration:number|null=null
const chainProject=(point:[number,number,number])=>project(point,'3d')
const currentChain=shallowRef<{curves:import('../services/solidNurbs').SolidNurbsCurve[];diagnostics:import('../services/curveOffsetDiagnostics').CurveOffsetDiagnostics}|null>(null)
const curveOffsetState=shallowRef<Awaited<ReturnType<typeof import('../services/previewSolidCurveOffset').previewSolidCurveOffset>>|null>(null)
const bodyEditPending=ref(false),bodyEditResult=shallowRef<DirectDocument|null>(null),bodyEditError=ref('')
function cancelBodyEdit(){curveOffsetState.value=null;bodyEditApplyGeneration=null;bodyEditGeneration++;bodyEditWorker.cancel();bodyEditPending.value=false;bodyEditResult.value=null;preparedProfileResult.value=null;nurbsRefitResult.value=null;surfaceBuildResult.value=null;curveMatchResult.value=null;surfaceMatchResult.value=null;seamPrepareResult.value=null;bodyEditError.value=''}
onUnmounted(()=>{cancelBodyEdit();bodyEditWorker.dispose()})
watch(()=>[props.open,bodyEditRevision.value,advancedOp.value,document.value,selection.value,faceIndex.value,edgeIndex.value,JSON.stringify(edgeIndexes.value),JSON.stringify(openingFaces.value),JSON.stringify(surfaceInputs.value),JSON.stringify(selectedIds.value),JSON.stringify(surfaceReversed.value),loftPreviewId.value,activeGroup.value,JSON.stringify(advanced.value),JSON.stringify(pointTrimPick.value),filletSegments.value,JSON.stringify(invalidQuantities.value)],()=>{
 cancelBodyEdit()
 if(!props.open||Object.keys(invalidQuantities.value).length||!(advancedOp.value==='nurbs-offset'||advancedOp.value==='nurbs-point-trim'||isSolidBodyEdit(advancedOp.value)||isSolidProfileEdit(advancedOp.value)||advancedOp.value==='profile-prepare'||isSolidNurbsRefit(advancedOp.value)||isSolidSurfaceBuild(advancedOp.value)||isNurbsMatch(advancedOp.value)||isSolidSceneEdit(advancedOp.value)))return
 const generation=bodyEditGeneration
 const operation=advancedOp.value
 const screenPick=pointTrimPick.value
 const options={...advanced.value,operation,id:selection.value,inputs:[...surfaceInputs.value],face:faceIndex.value,edges:edgeIndexes.value.length?[...edgeIndexes.value]:[edgeIndex.value],openings:openingFaces.value.length?[...openingFaces.value]:[faceIndex.value],segments:filletSegments.value}
 bodyEditPending.value=true
 // Coalesce synchronous command setup; invalidate immediately before rendering.
 queueMicrotask(async()=>{
  if(generation!==bodyEditGeneration)return
  try {
   // postMessage snapshots this shallow-ref document; avoid a JSON roundtrip on the UI thread.
   const source=document.value
   if(operation==='nurbs-offset'){
    const result=await (await import('../services/previewSolidCurveOffset')).previewSolidCurveOffset(bodyEditWorker,source,{...options,createdId:loftPreviewId.value,locale:props.locale})
    if(generation===bodyEditGeneration){bodyEditResult.value=result.document;curveOffsetState.value=result}
    return
   }
   if(operation==='nurbs-point-trim'){
    const dimension=source.curves?.find((c:SolidNurbsCurve)=>c.id===options.id)?.curve.controlPoints[0]?.length
    const result=await bodyEditWorker.run({kind:'nurbsEdit',document:source,options:screenPick?{kind:'trim-screen-point',id:options.id,...screenPick,keep:options.end}:{kind:'trim-point',id:options.id,point:dimension===2?[options.x,options.y]:[options.x,options.y,options.z],keep:options.end,maxDistance:options.profileGap}})
    if(generation===bodyEditGeneration)bodyEditResult.value=result
    return
   }
   if(isSolidSceneEdit(operation)){
    const result=await bodyEditWorker.run({kind:'sceneEdit',document:source,options:{operation,id:options.id,ids:[...selectedIds.value],createdId:loftPreviewId.value,x:options.x,y:options.y,z:options.z,axis:options.axis,angle:options.angle,scale:options.scale}})
    if(generation===bodyEditGeneration)bodyEditResult.value=result
    return
   }
   if(isNurbsMatch(operation)){
    const [a,b]=options.inputs
    if(operation==='nurbs-curve-match'){
     const result=await bodyEditWorker.run({kind:'curveMatch',args:[source,a??'',b??'',options.curveEndA,options.curveEndB,options.curveAngle]})
     if(generation===bodyEditGeneration)curveMatchResult.value=result
    }else{
     const parameters={referenceBoundary:options.matchBoundaryA,editedBoundary:options.matchBoundaryB,reverse:options.matchReverse,maxError:options.matchError}
     if(operation==='nurbs-match'){
      const result=await bodyEditWorker.run({kind:'surfaceMatch',args:[source,a??'',b??'',{...parameters,order:options.matchOrder,scale:options.matchScale}]})
      if(generation===bodyEditGeneration)surfaceMatchResult.value=result
     }else{
      const result=await bodyEditWorker.run({kind:'seamPrepare',args:[source,a??'',b??'',{...parameters,openPeriodic:options.prepareOpenPeriodic}]})
      if(generation===bodyEditGeneration)seamPrepareResult.value=result
     }
    }
    return
   }
   if(isSolidSurfaceBuild(operation)){
    const result=await bodyEditWorker.run({kind:'surfaceBuild',document:source,options:{kind:operation,ids:options.inputs,id:loftPreviewId.value,group:activeGroup.value||undefined,reversed:[...surfaceReversed.value],patchPreparation:{enabled:options.patchPrepare,maxError:options.patchError},sweep:{mode:options.sweepMode,normal:[options.sweepNormalX,options.sweepNormalY,options.sweepNormalZ],sections:options.sweepSections,maxDeviation:options.sweepDeviation}}})
    if(generation===bodyEditGeneration)surfaceBuildResult.value=result
    return
   }
   if(isSolidNurbsRefit(operation)){
    const result=await bodyEditWorker.run({kind:'nurbsRefit',document:source,options:{operation,id:options.id,axis:options.surfaceAxis,degree:options.reduceDegree,controlCount:options.rebuildControls,maxError:options.maxError}})
    if(generation===bodyEditGeneration)nurbsRefitResult.value=result
    return
   }
   if(operation==='profile-prepare'){
    const result=await bodyEditWorker.run({kind:'profilePrepare',document:source,ids:options.inputs,tolerance:options.profileGap})
    if(generation===bodyEditGeneration)preparedProfileResult.value=result
    return
   }
   const result=isSolidProfileEdit(operation)
    ?await bodyEditWorker.run({kind:'profileEdit',document:source,options:options as SolidProfileEditOptions})
    :await bodyEditWorker.run({kind:'bodyEdit',document:source,options:options as SolidBodyEditOptions})
   if(generation===bodyEditGeneration)bodyEditResult.value=result
  }catch(e){if(generation===bodyEditGeneration){bodyEditError.value=bodyCalculationFailure(e);bodyEditRetryVisible.value=true}}
  finally{if(generation===bodyEditGeneration){bodyEditPending.value=false;if(bodyEditApplyGeneration===generation){bodyEditApplyGeneration=null;applyCommand()}}}
 })
},{flush:'sync'})
const advancedPreview = computed(() => {
 if(!advancedOp.value||(advancedOp.value==='profile-prepare'||isSolidNurbsRefit(advancedOp.value)||isNurbsMatch(advancedOp.value))&&bodyEditPending.value)return {document:null,error:'',rawError:'',sweepReport:null}
 if(isSolidSurfaceBuild(advancedOp.value)){const r=surfaceBuildResult.value,error=r?.error||bodyEditError.value;return {document:r?.document??null,error:error?surfaceConstructionError(error):'',rawError:error,sweepReport:r?.report??null}}
 if(advancedOp.value==='nurbs-offset'||advancedOp.value==='nurbs-point-trim'||isSolidBodyEdit(advancedOp.value)||isSolidProfileEdit(advancedOp.value)||isSolidSceneEdit(advancedOp.value))return {document:bodyEditResult.value,error:bodyEditError.value?surfaceConstructionError(bodyEditError.value):'',rawError:bodyEditError.value,sweepReport:null}
 // Explicit reactive dependencies: history itself intentionally is not reactive.
 void document.value;void selection.value;void faceIndex.value;void edgeIndex.value;void edgeIndexes.value;void openingFaces.value;void advanced.value
 const report:{value:FramedSweepResult['report']|null}={value:null}
 try{const result=resultAdvanced(value=>report.value=value);return {document:result,error:'',rawError:'',sweepReport:report.value}}catch(e){return {document:null,error:surfaceConstructionError(e),rawError:e instanceof Error?e.message:String(e),sweepReport:report.value}}
})
const pointTrimPreviewPoints=computed(()=>{
 if(advancedOp.value!=='nurbs-point-trim')return []
 const curve=advancedPreview.value.document?.curves?.find(c=>c.id===selection.value)?.curve
 return curve?curvePoints(curve).map(p=>[p[0],p[1],p[2]??0] as Vec3):[]
})
const pointTrimCut=computed(()=>{
 if(advancedOp.value!=='nurbs-point-trim')return undefined
 const curve=advancedPreview.value.document?.curves?.find(c=>c.id===selection.value)?.curve
 // Native trim inserts the cut knot, making this endpoint a control point.
 const p=advanced.value.end==='start'?curve?.controlPoints.at(-1):curve?.controlPoints[0]
 return p?[p[0],p[1],p[2]??0]:undefined
})
const patchBudgetCurve=computed(()=>{
 const role=advancedOp.value==='nurbs-patch'?advancedPreview.value.rawError.match(/PATCH_BUDGET:([0-3])/):null
 return role?surfaceInputs.value[Number(role[1])]:undefined
})
const patchGapPoints=computed(()=>{
 if(advancedOp.value!=='nurbs-patch')return []
 const match=advancedPreview.value.rawError.match(/Coons corner ([1-4]) does not coincide/)
 if(!match)return []
 const pairs=[[[0,0],[2,0]],[[0,1],[3,0]],[[1,0],[2,1]],[[1,1],[3,1]]]
 try{return pairs[Number(match[1])-1].map(([role,end])=>{
  const curve=document.value.curves!.find(c=>c.id===surfaceInputs.value[role])!.curve
  const last=surfaceReversed.value[role]?!end:!!end
  return evaluateNurbsCurve(curve,curve.knots[last?curve.controlPoints.length:curve.degree]).point as Vec3
 })}catch{return []}
})
function applyAdvanced(){run(()=>{
 if(!advancedPreview.value.document)return
 const d=structuredClone(advancedPreview.value.document),splitBody=d.bodies.find(b=>b.id==='preview-split');if(splitBody)splitBody.id=crypto.randomUUID()
 const prepared=advancedOp.value==='nurbs-prepare'
 const profileId=(isProfileCommand(advancedOp.value))?surfaceInputs.value[0]:''
 const created=(advancedOp.value==='instance-create'||advancedOp.value==='nurbs-offset')?loftPreviewId.value:''
 commit(d);advancedOp.value=null;if(profileId){selection.value=profileId;extraSelection.value=[]}if(prepared)advanced.value.matchReverse=false;if(created)pickObject(created,'3d');faceIndex.value=edgeIndex.value=-1;edgeIndexes.value=[];openingFaces.value=[]
})}
const axisVector=(axis:string):Vec3=>axis==='x'?[1,0,0]:axis==='y'?[0,1,0]:[0,0,1]
const advancedBodyIds=computed(()=>new Set([...selectedIds.value,...sceneBodies.value.filter(b=>b.instance&&selectedIds.value.includes(b.instance.sourceId)).map(b=>b.id),'preview-split',loftPreviewId.value]))
const advancedPolygons=computed(()=>advancedPreview.value.document?.bodies.filter(b=>advancedBodyIds.value.has(b.id)).flatMap(b=>meshPolygons(b,1)).sort((a,b)=>a.depth-b.depth)??[])
const advancedSurfaceIds=computed(()=>advancedOp.value==='nurbs-prepare'?surfaceInputs.value:advancedOp.value==='nurbs-match'?[surfaceInputs.value[1]]:advancedOp.value==='nurbs-surface-reduce'||advancedOp.value==='nurbs-surface-rebuild'?[selection.value]:[loftPreviewId.value])
const advancedSurfacePolygons=computed(()=>advancedPreview.value.document?.surfaces?.filter(item=>advancedSurfaceIds.value.includes(item.id)).flatMap(item=>surfacePolygons(item)).sort((a,b)=>a.depth-b.depth)??[])
const advancedSketches=computed(()=>advancedPreview.value.document?.sketches.filter(s=>selectedIds.value.includes(s.id)&&samePlane(s.plane,activePlane.value))??[])
const advancedSketchDisplay=computed(()=>new Map(advancedSketches.value.map(s=>[s.id,profileLoops(s)])))
const authoredEdges=shallowRef<ReturnType<typeof solidBodyEdges>>([]),bodyEdgesPending=ref(false)
const bodyEdgesWorker=createSolidPreviewWorker()
let bodyEdgesGeneration=0
function cancelBodyEdges(){bodyEdgesGeneration++;bodyEdgesWorker.cancel();bodyEdgesPending.value=false}
onUnmounted(()=>{cancelBodyEdges();bodyEdgesWorker.dispose()})
async function refreshBodyEdges(){
 cancelBodyEdges();authoredEdges.value=[]
 const body=selectedBody.value,source=document.value,generation=bodyEdgesGeneration
 if(!props.open||!kernelReady.value||!body||pickMode.value!=='edge')return
 bodyEdgesPending.value=true
 try{
  const edges=await bodyEdgesWorker.run({kind:'bodyEdges',body})
  if(generation===bodyEdgesGeneration&&document.value===source&&selectedBody.value===body)authoredEdges.value=edges
 }catch(e){if(generation===bodyEdgesGeneration)error.value=e instanceof Error?e.message:String(e)}
 finally{if(generation===bodyEdgesGeneration)bodyEdgesPending.value=false}
}
const featureEdges=computed(()=>authoredEdges.value.map(edge=>({i:edge.index,id:edge.id,points:edge.points.map(point=>project(point,'3d').join(',')).join(' ')})))
const splitPlanePoints=computed(()=>{
 if(advancedOp.value!=='split'||!selectedBody.value)return ''
 const n=axisVector(advanced.value.axis),u=unit3(cross3(n,n[0]?[0,1,0]:[1,0,0])),v=cross3(n,u),p=bodyPoints(selectedBody.value),size=Math.max(...p.map(p=>Math.hypot(...p)))+10
 return [[-size,-size],[size,-size],[size,size],[-size,size]].map(q=>project(worldPoint(q,{origin:n.map(x=>x*advanced.value.distance) as Vec3,u,v}),'3d').join(',')).join(' ')
})
/** Axis-aligned bounds of the selection: the gizmo is centred on them and sized by them. */
const gizmoBounds=computed(()=>{
 const p=[...document.value.bodies.filter(b=>selectedIds.value.includes(b.id)).flatMap(bodyPoints),...document.value.sketches.filter(s=>selectedIds.value.includes(s.id)).flatMap(s=>s.points.map(p=>worldPoint(p,s.plane)))]
 if(!p.length)return null
 const min=[0,1,2].map(k=>Math.min(...p.map(q=>q[k]))),max=[0,1,2].map(k=>Math.max(...p.map(q=>q[k])))
 return {center:[0,1,2].map(k=>(min[k]+max[k])/2) as Vec3, extent:Math.max(...[0,1,2].map(k=>max[k]-min[k]))}
})
const gizmoCenter=computed(()=>gizmoBounds.value?.center??null)
/**
 * Gizmo arm length in world units.
 *
 * Proportional to the selection so the gizmo reads as attached to the body and shrinks
 * and grows with it under zoom, clamped to a screen-size band so it neither vanishes on
 * a tiny part nor covers the viewport on a large one.
 */
const gizmoLength=computed(()=>{
 // The floor only keeps the three tips from collapsing into each other; the tips
 // themselves stay a fixed screen size, so the arms may get short before that.
 const view=views.value['3d'],extent=gizmoBounds.value?.extent??0
 return Math.min(view/3,Math.max(view/40,extent*.7))
})
const gizmoAxes=computed(()=>gizmoCenter.value?(['x','y','z'] as const).map((axis,i)=>{
 const n=axisVector(axis),c=gizmoCenter.value!,length=gizmoLength.value
 const u=unit3(cross3(n,n[0]?[0,1,0]:[1,0,0])),v=cross3(n,u)
 return {axis,color:['#ff7777','#77df9d','#77baff'][i],base:project(c,'3d'),tip:project(c.map((x,k)=>x+n[k]*length),'3d'),ring:Array.from({length:65},(_,i)=>{const a=i*Math.PI/32;return project(c.map((x,k)=>x+length*.7*(u[k]*Math.cos(a)+v[k]*Math.sin(a))),'3d').join(',')}).join(' ')}
}):[])
function startGizmo(e:PointerEvent,kind:'move'|'rotate'|'scale'|'push'|'split',axis:'x'|'y'|'z') {
 if(kind==='move'&&!requireBodySnaps(e,document.value))return
 cancelNativeNurbs();cancelGizmoWorker()
 if(directTransformPending.value)cancelDirectTransform()
 if(kind==='rotate'||kind==='scale')previewingTransform.value=true
 e.preventDefault();e.stopPropagation();const svg=canvasOf(e),center=kind==='push'?selectedFace.value?.center:gizmoCenter.value;if(!center)return
 svg.focus()
 const n=kind==='push'?selectedFace.value!.normal:axisVector(axis),a=project(center,'3d'),b=project(center.map((x,i)=>x+n[i]),'3d')
 if(kind==='push'){advanced.value.distance=0;beginAdvanced('push')}
 dragConstraint.value=kind==='push'?label('Нормаль грани','Face normal'):axis.toUpperCase()
 manipulatorDrag={svg,pointer:e.pointerId,x:e.clientX,y:e.clientY,kind,axis,direction:[b[0]-a[0],b[1]-a[1]],before:history.document,initial:advanced.value.distance,startPoint:position(e),center:project(center,'3d')};svg.setPointerCapture(e.pointerId)
}
function startCurve(e:PointerEvent,kind:'center'|'radius'|'start'|'end'){
 if(!requireSketchSnaps(e))return
 cancelNativeNurbs();cancelGizmoWorker();if(directTransformPending.value)cancelDirectTransform()
 if(!selectedSketch.value?.analytic)return
 e.preventDefault();e.stopPropagation();advancedOp.value=null;const svg=canvasOf(e);svg.focus();curveDrag={id:selection.value,kind,before:history.document,pointer:e.pointerId,inverse:svg.getScreenCTM()?.inverse()??null};svg.setPointerCapture(e.pointerId)
}
function startCv(e:PointerEvent,u:number,v:number) {
 if(!requireBodySnaps(e,document.value))return
 cancelNativeNurbs();cancelGizmoWorker();if(directTransformPending.value)cancelDirectTransform()
 const point=selectedNurbsCurve.value?.curve.controlPoints[u]??selectedNurbsSurface.value?.surface.controlPoints[u]?.[v]
 if(!selectedNurbs.value||!point)return
 e.preventDefault();e.stopPropagation();cvU.value=u;cvV.value=v
 const svg=canvasOf(e);svg.focus();cvDrag={id:selectedNurbs.value.id,u,v,before:history.document,point:[...point],start:position(e),pointer:e.pointerId,svg,inverse:svg.getScreenCTM()?.inverse()??null};svg.setPointerCapture(e.pointerId)
}
function pickEdge(e:PointerEvent|KeyboardEvent,index:number){
 extraSelection.value=[];e.stopPropagation()
 if(e.shiftKey||e.ctrlKey||e.metaKey){const set=new Set(edgeIndexes.value.length?edgeIndexes.value:edgeIndex.value>=0?[edgeIndex.value]:[]);set.has(index)?set.delete(index):set.add(index);edgeIndexes.value=[...set];edgeIndex.value=edgeIndexes.value.at(-1)??-1}
 else {edgeIndex.value=index;edgeIndexes.value=[index]}
 faceIndex.value=-1;advancedOp.value=null
}
function trimAt(id:string,p:Point2){run(()=>{
 const d=history.document,s=d.sketches.find(s=>s.id===id);if(!s)return
 let best=Infinity,index=0,t=0
 for(let i=0;i<(s.closed?s.points.length:s.points.length-1);i++){const a=s.points[i],b=s.points[(i+1)%s.points.length],vx=b[0]-a[0],vy=b[1]-a[1],f=Math.max(0,Math.min(1,((p[0]-a[0])*vx+(p[1]-a[1])*vy)/(vx*vx+vy*vy))),dist=Math.hypot(p[0]-a[0]-f*vx,p[1]-a[1]-f*vy);if(dist<best){best=dist;index=i;t=f}}
 const pieces=trimSketch(s,index,t,d.sketches.filter(o=>samePlane(o.plane,s.plane)));pieces.forEach((p,i)=>{if(i)p.id=crypto.randomUUID()});d.sketches=d.sketches.filter(o=>o.id!==id);d.sketches.push(...pieces);commit(d)
})}

function run(action: () => void) { error.value = ''; try { action() } catch (e) { error.value = e instanceof Error ? e.message : String(e) } }
function draftConflictMessage(){return label('Документ изменён в другой вкладке. Скачайте JSON своих правок или загрузите сохранённую версию; Undo вернёт локальную модель.', 'Document changed in another tab. Download your edits as JSON or load the saved version; Undo restores your local model.')}
function noteExternalDraft(){
 if(storageGet(key)!==persistedReference){draftConflict.value=true;saveError.value=true;error.value=draftConflictMessage()}
}
function onDraftStorage(event:StorageEvent){if(event.key===key||event.key===null)noteExternalDraft()}
window.addEventListener?.('storage',onDraftStorage)
onUnmounted(()=>window.removeEventListener?.('storage',onDraftStorage))
if(typeof indexedDB!=='undefined')void collectSolidDraftSnapshots().catch(()=>{})
async function loadLatestDraft(){
 if(loadingLatestDraft.value)return
 cancelCommand();loadingLatestDraft.value=true
 const generation=++historyGeneration,saveRevision=++saveGeneration,before=document.value,target=history
 const current=()=>!restoreDisposed&&props.open&&generation===historyGeneration&&saveRevision===saveGeneration&&document.value===before&&target===history
 savePending.value=false;historyMode.value='shared';historyPending.value=true;workspace.value?.focus()
 try{await withSolidDraftLock(key,async()=>{
  await durableReady
  if(!current())return
  const head=await readSolidDraftHead(key),reference=storageGet(key)
  if(!current())return
  if(!head&&!reference)throw Error(label('Сохранённая версия отсутствует. Скачайте JSON текущей модели.','No saved version exists. Download the current model as JSON.'))
  const text=head?.text??(solidDraftSnapshotId(reference)?await readSolidDraftSnapshot(reference!):reference!)
  if(!current())return
  // The shared draft lock keeps the canonical head stable until the validated
  // replacement is published. Local edits remain the preceding history entry.
  const applied=await target.commitAsync(async()=>{
   const next=await historyWorker.run({kind:'restoreDocument',text})
   if(!current()||storageGet(key)!==reference)throw Error(label('Документ изменился во время загрузки. Повторите загрузку.','Document changed while loading. Load it again.'))
   return next
  })
  if(!applied||!current())return
  persistedRevision=head?.revision??null;persistedReference=stored=reference;draftConflict.value=false;error.value='';saveError.value=false
  sync()
 })}catch(e){if(current()){saveError.value=true;error.value=e instanceof Error?e.message:String(e)}}
 finally{if(generation===historyGeneration){loadingLatestDraft.value=false;historyPending.value=false}}
}
function persist() {
 const generation=++saveGeneration,previous=persistedReference
 savePending.value=true
 // Serialization finishes before the asynchronous write. Only top-level empty
 // collections are omitted; cloning materialized instance geometry is unnecessary.
 const persisted={...document.value}
 if(!persisted.curves?.length)delete persisted.curves
 if(!persisted.surfaces?.length)delete persisted.surfaces
 const text=serializeDirectDocument(persisted)
 void withSolidDraftLock(key,async()=>{
  await durableReady
  if(generation!==saveGeneration||restoreDisposed)return
  if(storageGet(key)!==previous){draftConflict.value=true;throw Error(draftConflictMessage())}
  let marker:string|undefined
  try{
   if(text.length>MAX_DRAFT_CHARACTERS)marker=await writeSolidDraftSnapshot(text)
   if(generation!==saveGeneration||restoreDisposed){if(marker)await removeSolidDraftSnapshot(marker);return}
   const head=await writeSolidDraftHead(key,persistedRevision,text,()=>generation===saveGeneration&&!restoreDisposed)
   persistedRevision=head.revision
   if(generation!==saveGeneration||restoreDisposed){if(marker)await removeSolidDraftSnapshot(marker);return}
   const reference=marker??text
   if(storageSet(key,reference))stored=persistedReference=reference
   saveError.value=false;draftConflict.value=false
   await removeSolidDraftSnapshot(previous).catch(()=>{})
  }catch(e){
   if(marker&&stored!==marker)void removeSolidDraftSnapshot(marker).catch(()=>{})
   throw e
  }
 }).catch(e=>{
  if(generation===saveGeneration&&!restoreDisposed){saveError.value=true;const detail=e instanceof Error?e.message:String(e);if(detail==='DRAFT_CONFLICT'){draftConflict.value=true;error.value=draftConflictMessage()}else if(e instanceof Error&&e.name==='QuotaExceededError')error.value=label('Хранилище заполнено. Скачайте JSON, освободите место и повторите сохранение.','Storage is full. Download JSON, free space and retry saving.')
  else if(detail==='DRAFT_CORRUPT')error.value=label('Сохранённая версия повреждена. Скачайте локальный JSON через меню «Файл».','Saved version is corrupt. Download local JSON from File.')
  else error.value=label('Не удалось сохранить черновик: ','Could not save draft: ')+detail}
 }).finally(()=>{if(generation===saveGeneration)savePending.value=false})
}
/**
 * Rebuilding the thousands of hit-test polygons after a commit costs far more than a
 * frame, and doing it the instant a drag ends is felt as a stall. The dragged elements
 * already carry the exact translation that was committed, so they stay correct for both
 * display and picking; the rebuild is deferred until the pointer has settled.
 */
const settling = ref(false)
let settleHandle = 0
function settleAfterDrag() {
  // The committed document normally rebuilds the GPU buffer, but never rely on that: a
  // commit that changes nothing leaves the buffer, and with it a stale offset, in place.
  if (dragOffsetIds.length) { gpuLayer?.setDragOffset(dragOffsetIds, [0, 0, 0]); dragOffsetIds = [] }
  if (typeof setTimeout !== 'function') { clearDragPreview(); return }
  settling.value = true
  clearTimeout(settleHandle)
  settleHandle = setTimeout(() => { settling.value = false; clearDragPreview() }, 140) as unknown as number
}

function pickBodyAt(p: Point2): { id: string; triangle: number } | null {
  const { yaw, pitch } = camera.value
  const cy = Math.cos(yaw), sy = Math.sin(yaw), cp = Math.cos(pitch), sp = Math.sin(pitch)
  const right: Vec3 = [cy, -sy, 0], up: Vec3 = [sy * sp, cy * sp, -cp], toward: Vec3 = [sy * cp, cy * cp, sp]
  const far = 1e6
  const origin = [0, 1, 2].map(k => p[0] * right[k] + p[1] * up[k] + far * toward[k]) as Vec3
  const ray = { origin, direction: [-toward[0], -toward[1], -toward[2]] as Vec3 }
  let best: { id: string; triangle: number } | null = null, bestDistance = Infinity
  const candidates=[...sceneBodies.value,...(document.value.surfaces??[]).filter(s=>objectInView(s.id)).flatMap(s=>{const mesh=surfaceDisplayQueue.get(s);return mesh?[{id:s.id,mesh}]:[]})]
  for (const body of candidates.filter(b=>objectSelectable(b.id))) {
    const pos = body.mesh.positions, idx = body.mesh.indices
    for (let t = 0; t < idx.length / 3; t++) {
      const a = idx[t * 3] * 3, b = idx[t * 3 + 1] * 3, c = idx[t * 3 + 2] * 3
      const d = rayTriangleDistance(ray, [pos[a], pos[a + 1], pos[a + 2]], [pos[b], pos[b + 1], pos[b + 2]], [pos[c], pos[c + 1], pos[c + 2]])
      if (d !== null && d < bestDistance) { bestDistance = d; best = { id: body.id, triangle: t } }
    }
  }
  return best
}

/** Pointer down on the canvas itself: with the GPU layer active, resolve the surface by ray. */
function downAt(e: PointerEvent, pane: Pane) {
  // Shift stays in: with a body under the cursor `down` turns it into add-to-selection,
  // and only without one does it fall back to panning.
  if (pane === '3d' && gpuActive.value && e.button !== 1) {
    try {
      const hit = pickBodyAt(position(e))
      if (hit) {
        if((document.value.surfaces??[]).some(s=>s.id===hit.id)){
          if(e.button===0){pickObject(hit.id,pane,e.shiftKey);return}
        }else{down(e, pane, hit.id, null, hit.triangle);return}
      }
    } catch { /* canvas not ready: fall through to the plain handler */ }
  }
  down(e, pane)
}

let lastHoverPick = 0
/** Pointer move: the drag logic first, then hover by ray while nothing is being dragged. */
function moveAt(e: PointerEvent) {
  move(e)
  if (e.pointerType === 'touch' || !gpuActive.value || gesture || orbitDrag || manipulatorDrag || vertexDrag || curveDrag || cvDrag || selectionBox.value) return
  if (e.timeStamp - lastHoverPick < 16) return
  lastHoverPick = e.timeStamp
  try {
    const id = pickBodyAt(position(e))?.id ?? ''
    if (id !== hovered.value) hovered.value = id
  } catch { /* canvas not ready */ }
}

function sync(next:DirectDocument=history.document) { snapDocument.value=next; document.value=next; const ids=new Set(documentObjects(document.value).map(o=>o.id));if(selection.value&&!ids.has(selection.value))selection.value='';extraSelection.value=extraSelection.value.filter(id=>ids.has(id)); if (gpuActive.value) settleAfterDrag(); const s=document.value.sketches.find(s=>s.id===selection.value);if(s&&!samePlane(s.plane,activePlane.value)){activePlane.value=s.plane??xyPlane();workplaneOutline.value=[]} undoable.value = history.canUndo; redoable.value = history.canRedo; persist() }
function prepareCommit(next: DirectDocument, origin:'edit'|'file'='edit') {
  const previous=origin==='file'&&!lockedIds.value.length?undefined:snapDocument.value
  const existing=new Set(previous?documentObjects(previous).map(b=>b.id):history.objectIds)
  const sourceChanges=new Map<string,boolean>()
  // Imported caches are checked and rebuilt by the authoritative history parser.
  // The edit-only guard detects attempts to modify a live linked body directly.
  for(const body of previous?.bodies??[])if(origin==='edit'&&body.instance) {
    const sourceId=body.instance.sourceId
    if(!sourceChanges.has(sourceId)) {
      const oldSource=previous!.bodies.find(item=>item.id===sourceId),newSource=next.bodies.find(item=>item.id===sourceId)
      sourceChanges.set(sourceId,stringifyMeshJson({mesh:oldSource?.mesh,brep:oldSource?.brep})!==stringifyMeshJson({mesh:newSource?.mesh,brep:newSource?.brep}))
    }
    const candidate=next.bodies.find(item=>item.id===body.id)
    if(!sourceChanges.get(sourceId)&&candidate?.instance&&stringifyMeshJson(candidate.instance)===stringifyMeshJson(body.instance)
      &&stringifyMeshJson({mesh:candidate.mesh,brep:candidate.brep})!==stringifyMeshJson({mesh:body.mesh,brep:body.brep}))
      throw Error(label('Измените источник или отсоедините экземпляр: ','Edit the source or detach the instance: ')+body.name)
  }
  const lockedObjects=(previous?documentObjects(previous):[]).filter(object=>lockedIds.value.includes(object.id))
  const validateLocked=lockedObjects.length?(resolved:DirectDocument)=>{
    const nextObjects=new Map(documentObjects(resolved).map(o=>[o.id,o]))
    for(const object of lockedObjects){
      const replacement=nextObjects.get(object.id)
      if(!replacement||stringifyMeshJson(object)!==stringifyMeshJson(replacement))throw Error(label('Объект заблокирован: ','Object is locked: ')+object.name)
    }
  }:undefined
  for(const body of documentObjects(next))if(!existing.has(body.id)&&(origin==='file'?body.group===undefined:!body.group)&&activeGroup.value)body.group=activeGroup.value
  const added=documentObjects(next).filter(b=>!existing.has(b.id)).map(b=>b.id)
  return {validateLocked,added}
}
function finishCommit(added:string[],next?:DirectDocument){
 sync(next)
 if(isolatedBodyIds.value.length)isolatedBodyIds.value=[...new Set([...isolatedBodyIds.value,...added])]
}
function commit(next:DirectDocument){
 const {validateLocked,added}=prepareCommit(next)
 history.commit(next,validateLocked);finishCommit(added)
}
function cancelHistoryRestore(){historyGeneration++;historyWorker.cancel();history.cancelRestore();historyPending.value=false;loadingLatestDraft.value=false}
async function undo(redo = false) {
 cancelCommand();error.value=''
 const generation=++historyGeneration,target=history
 historyMode.value='restore';historyPending.value=true
 // The history button becomes disabled; keep Escape routed to this workspace.
 workspace.value?.focus()
 try{
  let restored:DirectDocument|undefined
  // History takes its own copy; the UI can own the original worker response.
  const applied=await target.restoreAsync(redo?'redo':'undo',async text=>restored=await historyWorker.run({kind:'restoreDocument',text}))
  if(generation!==historyGeneration||target!==history||!props.open)return
  historyPending.value=false
  if(applied&&restored)sync(restored)
 }catch(e){if(generation===historyGeneration)error.value=e instanceof Error?e.message:String(e)}
 finally{if(generation===historyGeneration)historyPending.value=false}
}
watch(()=>[props.open,document.value,lockedIds.value.join(','),activeGroup.value],()=>{if(historyPending.value)cancelHistoryRestore()},{flush:'sync'})
onUnmounted(()=>{cancelHistoryRestore();historyWorker.dispose()})
function addSketch(points: Point2[], closed: boolean, analytic?: import('../services/directSketchGeometry').AnalyticCurve) {
  validateSimpleSketch(points,closed)
  const d = history.document, id = crypto.randomUUID()
  d.sketches.push({ id, name: label('Эскиз ', 'Sketch ') + (d.sketches.length + 1), points, closed, analytic, plane: JSON.parse(JSON.stringify(activePlane.value)), ...(workplaneBodyId.value?{supportBodyId:workplaneBodyId.value}:{}) })
  commit(d); pickObject(id,'2d')
}
function beginSketch(value: typeof tool.value) {
  cancelGesture(); operation.value = null; advancedOp.value = null; boxSelect.value = false
  choosingSketchFace.value=false;tool.value = value; mode.value = workplaneBodyId.value ? '3d' : '2d'; sketchPaneOpen.value = true
}
const canExtrudeSketch = computed(() => !!selectedSketch.value?.closed && tool.value === 'select' && !draft.value.length)
function finish(closed: boolean) { run(() => { if (draft.value.length < (closed ? 3 : 2)) return; addSketch(draft.value, closed); draft.value = []; draftCursor.value=null; tool.value = 'select' }) }
function beginExtrude(kind: 'extrude'|'revolve' = 'extrude') {
 if(kind==='revolve'&&selectedSketch.value)requirePolylineSketch(selectedSketch.value)
  if (!canExtrudeSketch.value) return
  cancelCommand(); boxSelect.value=false
  profileIds.value=selectedIds.value.filter(id=>document.value.sketches.some(s=>s.id===id&&s.closed&&samePlane(s.plane,selectedSketch.value!.plane)))
  if(!profileIds.value.length)profileIds.value=[selectedSketch.value!.id]
  const support=document.value.bodies.find(b=>b.id===selectedSketch.value!.supportBodyId)
  if(support){targetBody.value=support.id;extrusionMode.value='union';height.value=Math.abs(height.value)||10;baseZ.value=0}
  else if(!document.value.bodies.some(b=>b.id===targetBody.value))extrusionMode.value='new'
  fitNextPreview = true; operation.value = kind; mode.value = '3d'; previewError.value = ''
  if (!document.value.bodies.some(b => b.id === targetBody.value)) targetBody.value = document.value.bodies[0]?.id ?? ''
}
const profileChoices=computed(()=>document.value.sketches.filter(s=>s.closed&&samePlane(s.plane,selectedSketch.value?.plane)))
function extrusionOptions(id='preview') { return {sketchIds:[...profileIds.value],height:height.value,offset:baseZ.value,operation:extrusionMode.value,targetId:targetBody.value,id,segments:brepSegments.value} }
function setExtrusionMode(value: 'new'|'union'|'difference') {
  extrusionMode.value=value
  if(selectedSketch.value?.supportBodyId && value!=='new')height.value=Math.abs(height.value)*(value==='difference'?-1:1)
}
function extrude() { run(() => {
  const next=solidPreviewResult.value
  if(!next||previewPending.value||previewError.value)return
  const id=previewBody.value?.id??'';commit(next)
  operation.value=null;mode.value='3d';selection.value=extrusionMode.value==='new'?id:next.bodies.some(b=>b.id===targetBody.value)?targetBody.value:'';fit('3d')
}) }
watch(()=>[props.open,operation.value,document.value,selection.value,activeGroup.value,cornerVertex.value,cornerRadius.value,copyCount.value,copySweep.value,copyX.value,copyY.value,sketchEditRevision.value,JSON.stringify(invalidQuantities.value)],()=>{
 cancelSketchEdit()
 const op=operation.value
 if(!props.open||Object.keys(invalidQuantities.value).length||!selectedSketch.value||(op!=='fillet'&&op!=='dogear'&&op!=='array'))return
 const generation=sketchEditGeneration
 const options:SolidSketchEditOptions={operation:op,id:selection.value,vertex:cornerVertex.value,radius:cornerRadius.value,count:copyCount.value,center:[copyX.value,copyY.value],sweep:copySweep.value,copyIds:Array.from({length:Number.isInteger(copyCount.value)&&copyCount.value>=2&&copyCount.value<=64?copyCount.value-1:0},()=>crypto.randomUUID())}
 sketchEditPending.value=true
 queueMicrotask(async()=>{
  if(generation!==sketchEditGeneration)return
  try{
   const result=await sketchEditWorker.run({kind:'sketchEdit',document:document.value,options})
   if(generation===sketchEditGeneration)sketchEditResult.value=result
  }catch(e){if(generation===sketchEditGeneration){sketchEditError.value=bodyCalculationFailure(e);sketchEditRetryVisible.value=true}}
  finally{if(generation===sketchEditGeneration)sketchEditPending.value=false}
 })
},{flush:'sync'})
const copyPreview = computed(() => operation.value==='array'?sketchEditResult.value?.sketches.filter(s=>!document.value.sketches.some(source=>source.id===s.id))??[]:[])
function applyCopies(){applySketchEdit()}
function duplicate() { run(() => {
 const d=history.document,moved=transformSelection(d,selectedIds.value,[10,10,0],[0,0,1],0,1),ids:string[]=[],copies=new Map(selectedIds.value.map(id=>[id,crypto.randomUUID()]))
 for(const s of moved.sketches)if(selectedIds.value.includes(s.id)){s.id=copies.get(s.id)!;s.name=(s.name+' · copy').slice(0,100);ids.push(s.id);d.sketches.push(s)}
 for(const b of moved.bodies)if(selectedIds.value.includes(b.id)){b.id=copies.get(b.id)!;if(b.instance&&copies.has(b.instance.sourceId))b.instance.sourceId=copies.get(b.instance.sourceId)!;b.name=(b.name+' · copy').slice(0,100);ids.push(b.id);d.bodies.push(b)}
 if(!ids.length)return
 commit(d);pickObject(ids[0],mode.value);extraSelection.value=ids.slice(1)
}) }
watch([document, () => props.open, operation, selectedSketch, profileIds, activeGroup, height, baseZ, brepSegments, revolveAxis, revolveOffset, revolveAngle, revolveSegments, revolveGeometry, extrusionMode, targetBody, solidPreviewRevision, () => JSON.stringify(invalidQuantities.value)], () => {
  invalidateSolidPreview()
  if (!props.open || Object.keys(invalidQuantities.value).length || !solidActive.value || !selectedSketch.value) return
  const generation=previewGeneration
  previewPending.value=true
  previewTimer = setTimeout(async () => {
    try {
      const id=crypto.randomUUID(),source=document.value
      const result=operation.value==='extrude'
        ? await previewWorker.run({kind:'extrusion',document:source,options:extrusionOptions(id)})
        : await previewWorker.run({kind:'revolve',document:source,options:{...revolveOptions(),sketchId:selectedSketch.value!.id,geometry:revolveGeometry.value,operation:extrusionMode.value,targetId:targetBody.value,id,tessellation:brepSegments.value,
            name:selectedSketch.value!.name+' · '+(revolveGeometry.value==='faceted'?label('гранёный B-rep','faceted B-rep'):label('точный B-rep','exact B-rep'))}})
      if(generation!==previewGeneration)return
      solidPreviewResult.value=result
      previewBody.value=result.bodies.find(b=>b.id===(extrusionMode.value==='new'?id:targetBody.value))??null
      previewEmpty.value=!previewBody.value
      if(fitNextPreview){fit('3d');fitNextPreview=false}
    }
    catch (e) { if(generation===previewGeneration){previewError.value=bodyCalculationFailure(e);solidPreviewRetryVisible.value=true} }
    finally {if(generation===previewGeneration)previewPending.value=false}
  }, 60)
}, {flush:'sync'})
watch(selection, () => { advancedOp.value=null; operation.value = null; previewBody.value = null; cornerVertex.value = 0 })
function remove() { run(() => {
  advancedOp.value=null
  const d = history.document
  d.sketches = d.sketches.filter(s => !selectedIds.value.includes(s.id))
  d.bodies = d.bodies.filter(b => !selectedIds.value.includes(b.id))
  d.curves = d.curves!.filter(c => !selectedIds.value.includes(c.id))
  d.surfaces = d.surfaces!.filter(s => !selectedIds.value.includes(s.id))
  commit(d); selection.value = ''; extraSelection.value=[]
}) }
function restoreGizmoPreview(){
 if(gizmoBase&&gizmoPublished===document.value){gizmoPublishing=true;document.value=gizmoBase;gizmoPublishing=false}
 gizmoPublished=null
}
function cancelGizmoWorker(){
 if(gizmoHistoryPending){history.cancelRestore();gizmoHistoryPending=false}
 gizmoEpoch++;gizmoWorker.cancel();gizmoQueued=null;gizmoRunning=false;gizmoPending.value=false;gizmoApplyRevision=null
 numericPointEdit=false;pointEditActive.value=false;restoreGizmoPreview();gizmoBase=null;previewingTransform.value=false
}
function requestDragJob(job:DragWorkerJob){
 pointEditActive.value=job.kind==='pointEdit'
 gizmoBase=job.document;gizmoPending.value=true;error.value=''
 gizmoQueued={job,revision:++gizmoRevision}
 if(!gizmoRunning)void drainGizmoTransforms()
}
function requestGizmoTransform(before:DirectDocument,axis:'x'|'y'|'z',rotation:number,factor:number){
 requestDragJob({kind:'sceneEdit',document:before,options:{operation:'transform',id:selection.value,ids:[...selectedIds.value],createdId:'',x:0,y:0,z:0,axis,angle:rotation,scale:factor}})
}
async function drainGizmoTransforms(){
 const epoch=gizmoEpoch;gizmoRunning=true
 try{
  while(gizmoQueued&&epoch===gizmoEpoch){
   const job=gizmoQueued;gizmoQueued=null
   try{
    const result=await gizmoWorker.run(job.job)
    if(epoch!==gizmoEpoch)return
    if(job.revision!==gizmoRevision)continue
    if(gizmoApplyRevision===job.revision){
     numericPointEdit=false;gizmoPublishing=true
     try{
      if(job.job.kind==='sceneEdit'){
       const {validateLocked,added}=prepareCommit(result)
       result.blenderProjectId ??= snapDocument.value.blenderProjectId
       gizmoHistoryPending=true
       const applied=await history.commitAsync(async()=>result,validateLocked)
       if(epoch!==gizmoEpoch||!applied)return
       previewingTransform.value=false;finishCommit(added,result)
      }else{previewingTransform.value=false;commit(result)}
     }finally{if(epoch===gizmoEpoch)gizmoHistoryPending=false;gizmoPublishing=false}
     gizmoBase=null;gizmoPublished=null;gizmoApplyRevision=null;gizmoPending.value=false;pointEditActive.value=false;settleAfterDrag()
    }else{
     gizmoPublishing=true;document.value=result;gizmoPublished=document.value;gizmoPublishing=false
    }
   }catch(e){
    if(epoch!==gizmoEpoch)return
    if(job.revision===gizmoRevision){restoreGizmoPreview();error.value=e instanceof Error?e.message:String(e);if(gizmoApplyRevision!==null)previewingTransform.value=false;gizmoApplyRevision=null}
   }
  }
 }finally{if(epoch===gizmoEpoch){gizmoRunning=false;gizmoPending.value=false}}
}
watch(()=>[props.open,document.value,JSON.stringify(selectedIds.value)],()=>{
 if(!gizmoPublishing&&(gizmoBase||gizmoPending.value)){cancelGizmoWorker();manipulatorDrag=null;cvDrag=null;vertexDrag=null;curveDrag=null;if(gesture?.workerEdit)gesture=null;previewingTransform.value=false}
},{flush:'sync'})
watch(()=>[cvU.value,cvV.value],()=>{if(pointEditActive.value&&(gizmoBase||gizmoPending.value)){cancelGizmoWorker();cvDrag=null}},{flush:'sync'})
watch(()=>[cvU.value,cvV.value,cvX.value,cvY.value,cvZ.value,cvWeight.value],()=>{if(numericPointEdit&&gizmoPending.value)cancelGizmoWorker()},{flush:'sync'})
onUnmounted(()=>{cancelGizmoWorker();gizmoWorker.dispose()})
const directTransformWorker=createSolidPreviewWorker(),directTransformPending=ref(false)
let directTransformGeneration=0
function cancelDirectTransform(){
 directTransformGeneration++;directTransformWorker.cancel()
 if(directTransformPending.value){history.cancelRestore();clearDragPreview()}
 directTransformPending.value=false
}
async function commitDirectTransform(before:DirectDocument,ids:string[],delta:Vec3,rotation=0,factor=1,localSketch=false,resetFields=false){
 cancelDirectTransform()
 const generation=directTransformGeneration
 directTransformPending.value=true;error.value=''
 try{
  const result=await directTransformWorker.run({kind:'sceneEdit',document:before,options:{operation:localSketch?'sketch-transform':'transform',id:ids[0],ids,createdId:'',x:delta[0],y:delta[1],z:delta[2],axis:'z',angle:rotation,scale:factor}})
  if(generation!==directTransformGeneration)return
  const {validateLocked,added}=prepareCommit(result)
  result.blenderProjectId ??= snapDocument.value.blenderProjectId
  const applied=await history.commitAsync(async()=>result,validateLocked)
  if(generation!==directTransformGeneration||!applied)return
  directTransformPending.value=false;finishCommit(added,result);settleAfterDrag()
  if(resetFields){dx.value=dy.value=dz.value=angle.value=0;scale.value=1}
 }catch(e){if(generation===directTransformGeneration){error.value=e instanceof Error?e.message:String(e);clearDragPreview()}}
 finally{if(generation===directTransformGeneration)directTransformPending.value=false}
}
watch(()=>[props.open,document.value,JSON.stringify(selectedIds.value),dx.value,dy.value,dz.value,angle.value,scale.value],()=>{if(directTransformPending.value)cancelDirectTransform()},{flush:'sync'})
onUnmounted(()=>{cancelDirectTransform();directTransformWorker.dispose()})
const transformInputErrors=ref<Record<string,boolean>>({})
const transformInputInvalid=computed(()=>Object.keys(transformInputErrors.value).length>0)
function transformValidity(key:string,valid:boolean){
 if(valid)delete transformInputErrors.value[key];else{transformInputErrors.value[key]=true;cancelDirectTransform()}
}
function transform(){
 if(directTransformPending.value||transformInputInvalid.value)return
 cancelCommand()
 void commitDirectTransform(history.document,selectedIds.value,[dx.value,dy.value,dz.value],angle.value,scale.value,!!selectedSketch.value&&selectedIds.value.length===1,true)
}
function appendBodies() {
  run(() => {
    const source = directBodiesScad(document.value)
    if (!props.canAppend || source.length > props.remainingSource) throw new Error(label('Сохраните тела в SCAD: текущий документ не подходит для добавления.', 'Download SCAD: the current document cannot accept these bodies.'))
    emit('append', source); emit('close')
  })
}
function sendToMesh() {
  run(() => {
    storageSet('scad-mesh-modeler-v1', stringifyMeshJson(solidDocumentToMeshDocument(document.value)))
    emit('toMesh')
  })
}
function closeFileMenu(e:KeyboardEvent) {
 const menu=e.currentTarget as HTMLDetailsElement
 if(!menu.open)return
 e.preventDefault();e.stopPropagation()
 menu.open=false
 menu.querySelector('summary')?.focus()
}
function downloadProject(){if(!restoringDraft.value)download(serializeDirectDocument(document.value),'solid-model.json')}
function download(text: string, name: string) { const url = URL.createObjectURL(new Blob([text], { type: 'text/plain' })); const a = window.document.createElement('a'); a.href = url; a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(url), 1000) }
async function importFile(event: Event) {
 const input=event.target as HTMLInputElement,file=input.files?.[0]
 if(!file)return
 cancelCommand();error.value=''
 const generation=++historyGeneration,target=history,before=document.value
 const current=()=>generation===historyGeneration&&target===history&&!restoreDisposed&&props.open&&document.value===before
 historyMode.value='import';historyPending.value=true;workspace.value?.focus()
 try{
  if(file.size>MAX_DOCUMENT_CHARACTERS)throw Error('Document exceeds 64 MB.')
  const text=await file.text()
  if(!current())return
  const parsed=JSON.parse(text)
  if(parsed?.language==='modelgraph/nurbs-1'){
   const next=await historyWorker.run({kind:'modelGraphImport',document:history.document,text,group:activeGroup.value||undefined})
   if(!current())return
   const {validateLocked,added}=prepareCommit(next)
   const applied=await target.commitAsync(async()=>next,validateLocked)
   if(!current()||!applied)return
   historyPending.value=false;finishCommit(added)
   const addedIds=new Set(added)
   selection.value=next.surfaces?.find(item=>addedIds.has(item.id))?.id??next.curves?.find(item=>addedIds.has(item.id))?.id??''
  }else{
   // Inspect only the collections needed for metadata. Geometry is validated
   // once in the worker, before the history transaction can be published.
   if(!parsed||parsed.version!==1||!Array.isArray(parsed.bodies)||!Array.isArray(parsed.sketches)
    ||[parsed.curves,parsed.surfaces].some(items=>items!=null&&!Array.isArray(items))
    ||[...parsed.bodies,...parsed.sketches,...parsed.curves??[],...parsed.surfaces??[]].some(item=>!item||typeof item!=='object'))throw Error('Invalid direct modeling document.')
   const imported=parsed as DirectDocument
   if(imported.blenderProjectId===undefined)imported.blenderProjectId=crypto.randomUUID()
   const {validateLocked,added}=prepareCommit(imported,'file')
   const payload=stringifyMeshJson(imported)
   const applied=await target.commitAsync(()=>historyWorker.run({kind:'restoreDocument',text:payload}),validateLocked)
   if(!current()||!applied)return
   historyPending.value=false;finishCommit(added);selection.value='';extraSelection.value=[]
  }
  fit('2d');fit('3d')
 }catch(e){if(generation===historyGeneration)error.value=String(e)}
 finally{if(generation===historyGeneration)historyPending.value=false;input.value=''}
}
const bodyExportFormat = ref<MeshExportFormat>('stl_binary')
async function downloadBody() {
  if(restoringDraft.value)return
  try {
    if (!selectedBody.value) throw new Error(label('Выберите тело.', 'Select a body.'))
    const body=structuredClone(selectedBody.value),format=bodyExportFormat.value
    const artifact = await exportMeshFormatCompressed(polygonMeshToExportMesh(body.mesh), format)
    downloadBytes(artifact.data, artifact.mimeType, `${body.name || 'body'}.${artifact.extension}`)
  } catch (e) { error.value = e instanceof Error ? e.message : String(e) }
}
async function importStl(event: Event) {
  const before=document.value
  const input = event.target as HTMLInputElement, file = input.files?.[0]
  try {
    if (!file) return
    const imported = await importMeshFromFile(file, { weld: 1e-4 })
    if(restoreDisposed || !props.open || document.value!==before)throw Error('The scene changed during mesh import. Import again to add the mesh.')
    const mesh = { positions: imported.positions.slice(), indices: imported.indices.slice() }
    run(() => {
      const d = history.document
      const id = crypto.randomUUID()
      d.bodies.push({ id, name: stripMeshExtension(file.name) || imported.format.toUpperCase(), mesh })
      commit(d)
      selection.value = id
      mode.value = '3d'
    })
  } catch (e) { error.value = e instanceof Error ? e.message : String(e) }
  finally { input.value = '' }
}
async function importStep(event: Event) {
  const input = event.target as HTMLInputElement, file = input.files?.[0]
  if (stepBusy.value) { input.value = ''; return }
  const before=document.value
  stepBusy.value = true; error.value = ''; notice.value = ''
  try {
    if (!file) return
    if (file.size > 16 * 1024 * 1024) throw Error('STEP file exceeds 16 MiB.')
    const text = await file.text()
    const {prepareSolidStepImport} = await import('../services/solidStepExchange')
    if(restoreDisposed || !props.open || document.value!==before)throw Error('The scene changed during STEP import. Import again to add its bodies.')
    cancelCommand()
    const imported = await prepareSolidStepImport(text, before)
    if (restoreDisposed || !props.open || document.value !== before) {
      throw Error(imported.report.retained
        ? 'AP242 original saved, but the scene changed during import. Import again to add its bodies.'
        : 'The scene changed during STEP import. Import again to add its bodies.')
    }
    commit(imported.document)
    operation.value = null; advancedOp.value = null; subtract.value = null; boxSelect.value = false
    pickObject(imported.selected, '3d'); fit('3d')
    notice.value = imported.report.retained
      ? label(`AP242: ${imported.report.occurrenceIdentities.length} экземпляров; оригинал сохранён.`, `AP242: ${imported.report.occurrenceIdentities.length} occurrence(s); original saved.`)
      : label('STEP: сетка импортирована.', 'STEP: mesh imported.')
  } catch (e) { error.value = e instanceof Error ? e.message : String(e) }
  finally { stepBusy.value = false; input.value = '' }
}
async function exportBlender() {
  if(restoringDraft.value)return
  error.value=''
  const before=document.value
  try {
    const {exportSolidBlenderSnapshot}=await import('../services/solidBlenderExchange')
    if(document.value!==before||restoreDisposed)throw Error(label('Сцена изменилась. Повторите экспорт.', 'The scene changed. Export again.'))
    // Validate support before assigning identity or persisting metadata.
    exportSolidBlenderSnapshot(before,before.blenderProjectId??'validate')
    const id=history.ensureBlenderProjectId()
    sync()
    download(exportSolidBlenderSnapshot(document.value,id),'solid-model.osv-blender.json')
  } catch(e){error.value=e instanceof Error?e.message:String(e)}
}
async function importSvg(event:Event) {
  const input=event.target as HTMLInputElement,file=input.files?.[0]
  if(!file||svgBusy.value){input.value='';return}
  cancelSvg();const controller=new AbortController();svgController=controller
  const before=document.value,generation=saveGeneration
  svgBusy.value=true;error.value=''
  let worker:import('../services/svgWorkerClient').SvgWorkerClient|undefined
  try {
    if(file.size>4*1024*1024)throw Error('SVG exceeds 4 MiB.')
    const {createSvgWorkerClient}=await import('../services/svgWorkerClient')
    const text=await file.text()
    if(controller.signal.aborted)return
    worker=createSvgWorkerClient()
    const result=await worker.run({kind:'contours',svg:text,options:{}},{signal:controller.signal})
    if(controller.signal.aborted)return
    if(restoreDisposed||!props.open||document.value!==before||saveGeneration!==generation)throw Error(label('Сцена изменилась. Повторите импорт SVG.','The scene changed. Import SVG again.'))
    if(!result.contours?.length)throw Error('SVG contains no closed contours.')
    if(result.contours.some(r=>r.length>512))throw Error(label('Контур SVG превышает 512 вершин. Упростите исходный профиль.','SVG contour exceeds 512 vertices. Simplify the source profile.'))
    const next=history.document
    const ids=result.contours.map(points=>{
      const id=crypto.randomUUID()
      next.sketches.push({id,name:`${file.name.slice(0,70)} · ${next.sketches.length+1}`,closed:true,points})
      return id
    })
    commit(parseDirectDocument(serializeDirectDocument(next)))
    operation.value=null;advancedOp.value=null;subtract.value=null
    activePlane.value=xyPlane();workplaneOutline.value=[]
    selection.value=ids[0];extraSelection.value=ids.slice(1);mode.value='2d';sketchPaneOpen.value=true;fit('2d')
    notice.value=label('SVG импортирован. Для отверстий выдавливайте выбранные контуры вместе.','SVG imported. Extrude the selected contours together to retain holes.')+(result.warnings.length?' '+result.warnings.join(' '):'')
  } catch(e){if(!controller.signal.aborted)error.value=e instanceof Error?e.message:String(e)}
  finally{worker?.dispose();if(svgController===controller){svgController=undefined;svgBusy.value=false}input.value=''}
}
async function exportBodySvg() {
  if(restoringDraft.value)return
  const body=selectedBody.value
  if(!body)return
  cancelSvg();const controller=new AbortController();svgController=controller
  const mesh=structuredClone(body.mesh)
  error.value=''
  let worker:import('../services/svgWorkerClient').SvgWorkerClient|undefined
  try {
    const {createSvgWorkerClient}=await import('../services/svgWorkerClient')
    if(controller.signal.aborted)return
    worker=createSvgWorkerClient()
    const vertices=new Float32Array(mesh.positions.length*2)
    for(let i=0;i<mesh.positions.length;i+=3)vertices.set(mesh.positions.slice(i,i+3),i*2)
    const result=await worker.run({kind:'project',axis:'z',options:{},meshes:[{vertices,indices:new Uint32Array(mesh.indices),faceIds:new Uint32Array(mesh.indices.length/3),transform:new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1])}]},{signal:controller.signal})
    if(!controller.signal.aborted&&!restoreDisposed&&props.open)download(result.svg,'body-xy.svg')
  } catch(e){if(!controller.signal.aborted)error.value=e instanceof Error?e.message:String(e)}
  finally{worker?.dispose();if(svgController===controller)svgController=undefined}
}
async function exportStepCurrent() {
  if(restoringDraft.value)return
  const body=selectedBody.value
  if(!body)return
  const snapshot=structuredClone(body)
  error.value=''
  try {
    const {exportSolidStepCurrent}=await import('../services/solidStepExchange')
    download(await exportSolidStepCurrent(snapshot),'edited-body.step')
  } catch(e){error.value=e instanceof Error?e.message:String(e)}
}
async function exportStepAssembly() {
  if(restoringDraft.value)return
  const snapshot=structuredClone(document.value)
  error.value=''
  try {
    const {exportSolidStepAssembly}=await import('../services/solidStepAssembly')
    download(await exportSolidStepAssembly(snapshot),'edited-scene.step')
  } catch(e){error.value=e instanceof Error?e.message:String(e)}
}
async function exportStepOriginal() {
  if(restoringDraft.value)return
  error.value = ''
  try {
    const {exportSolidStepOriginal} = await import('../services/solidStepExchange')
    download(await exportSolidStepOriginal(), 'retained-model.step')
  } catch (e) { error.value = e instanceof Error ? e.message : String(e) }
}
function project(p: ArrayLike<number>, pane: Pane): Point2 { return pane === '2d' ? [p[0], -p[1]] : projectDirectPoint([p[0],p[1],p[2]??0], camera.value).slice(0,2) as Point2 }
function viewBox(pane: Pane) { const size = views.value[pane], center = centers.value[pane]; return `${center[0] - size / 2} ${center[1] - size / 2} ${size} ${size}` }
function zoom(pane: Pane, factor: number) { views.value[pane] = Math.max(.1, Math.min(2e6, views.value[pane] * factor)) }
function wheelZoom(e: WheelEvent, pane: Pane) {
  if (!e.deltaY) return
  const svg = e.currentTarget as SVGSVGElement, rect = svg.getBoundingClientRect()
  const pixels = Math.max(1, Math.min(rect.width, rect.height))
  const oldSize = views.value[pane]
  const delta = e.deltaY * (e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? rect.height : 1)
  zoom(pane, Math.exp(Math.max(-1, Math.min(1, delta * .0015))))
  const change = (oldSize - views.value[pane]) / pixels
  centers.value[pane] = [centers.value[pane][0] + (e.clientX-rect.left-rect.width/2)*change, centers.value[pane][1] + (e.clientY-rect.top-rect.height/2)*change]
}
function fit(pane: Pane, selectedOnly=false) {
  const points = pane === '2d' ? visibleSketches.value.filter(s=>!selectedOnly||selectedIds.value.includes(s.id)).flatMap(s => (retainedDisplay.value.get(s.id)??[s.points]).flat()).concat(selectedOnly?[]:copyPreview.value.flatMap(s=>s.points)) : [
    ...[...sceneBodies.value.filter(b=>!selectedOnly||selectedIds.value.includes(b.id)), ...(previewBody.value ? [previewBody.value] : [])].flatMap(bodyPoints),
    ...document.value.sketches.filter(item=>objectInView(item.id)&&(!selectedOnly||selectedIds.value.includes(item.id))).flatMap(item=>(retainedDisplay.value.get(item.id)??[item.points]).flat().map(p=>worldPoint(p,item.plane))),
    ...(document.value.curves ?? []).filter(item=>objectInView(item.id)&&(!selectedOnly||selectedIds.value.includes(item.id))).flatMap(item => item.curve.controlPoints),
    ...(document.value.surfaces ?? []).filter(item=>objectInView(item.id)&&(!selectedOnly||selectedIds.value.includes(item.id))).flatMap(item => item.surface.controlPoints.flat()),
  ]
  if (!points.length) { views.value[pane] = 160; centers.value[pane] = [0, 0]; return }
  const projected = points.map(p => project(p, pane))
  const min = [Infinity, Infinity], max = [-Infinity, -Infinity]
  for (const p of projected) for (let axis = 0; axis < 2; axis++) { min[axis] = Math.min(min[axis], p[axis]); max[axis] = Math.max(max[axis], p[axis]) }
  centers.value[pane] = [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2]
  views.value[pane] = Math.max(.1, Math.max(max[0] - min[0], max[1] - min[1]) * 1.6)
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
// Display tessellation for exact B-rep bodies: the authored mesh (segments = brepSegments) stays the working
// mesh for picking, topology and export; the view draws a denser mesh with smoothed normals, and every display
// triangle points back at the nearest working triangle so face picking and highlighting keep working.
// Chrome refuses a synchronous WebAssembly.Module over 8 MB on the main thread, so the display
// tessellation waits for the kernel's asynchronous warm-up and shows the working mesh until then.
const kernelReady = ref(isGeometryKernelReady())
const draftRecoveryWorker = createSolidPreviewWorker()
let draftRecoveryGeneration = 0
function stopDraftRecovery() {
  draftRecoveryGeneration++
  draftRecoveryWorker.cancel()
  history.cancelRestore()
  restoringDraft.value = false
}
function cancelDraftRecovery() {
  stopDraftRecovery()
  saveError.value = true
  error.value = label('Восстановление отменено. Сохранённый черновик не изменён; перезагрузите страницу, чтобы восстановить его.','Recovery cancelled. The saved draft is unchanged; reload the page to recover it.')
}
onUnmounted(() => { stopDraftRecovery(); draftRecoveryWorker.dispose() })
async function restoreInitialDraft() {
  const generation = ++draftRecoveryGeneration, target = history
  const reference = stored, expectedReference = persistedReference, expectedGeneration = saveGeneration
  const current = () => !restoreDisposed && generation === draftRecoveryGeneration && target === history
    && saveGeneration === expectedGeneration && !history.canUndo && storageGet(key) === expectedReference
  try {
    await warmGeometryKernel()
    if (restoreDisposed) return
    kernelReady.value = true
    if (!restoringDraft.value || !current()) return
    const head = await durableReady
    const text = head?.text ?? (reference ? (solidDraftSnapshotId(reference) ? await readSolidDraftSnapshot(reference) : reference) : null)
    if (!current()) return
    if (text) {
      const applied = await target.resetAsync(async () => {
        const recovered = await draftRecoveryWorker.run({kind:'restoreDocument', text})
        if (!current()) throw Error('Draft recovery cancelled.')
        if (!head) await withSolidDraftLock(key, async () => {
          if (!current()) throw Error('Draft recovery cancelled.')
          const existing = await readSolidDraftHead(key)
          if (existing && existing.text !== text) { draftConflict.value=true; throw Error(draftConflictMessage()) }
          const migrated = existing ?? await writeSolidDraftHead(key, null, text, current)
          if (current()) persistedRevision = migrated.revision
        })
        if (!current()) throw Error('Draft recovery cancelled.')
        return recovered
      })
      if (!applied || !current()) return
      snapDocument.value=history.document; document.value=snapDocument.value; error.value = ''; saveError.value = false
    }
    await nextTick()
    if (current()) { fit('2d'); fit('3d') }
  } catch(e) {
    if (!current()) return
    saveError.value = true
    const detail = e instanceof Error ? e.message : String(e)
    error.value = e instanceof SyntaxError || (e instanceof Error && e.name === 'SyntaxError') ? label('Сохранённый JSON повреждён. Импортируйте резервный JSON через меню «Файл».','Saved JSON is corrupt. Import a JSON backup from the File menu.')
      : detail === 'Saved draft snapshot is missing or invalid. Import a JSON backup.' ? label('Сохранённый снимок отсутствует или повреждён. Импортируйте резервный JSON через меню «Файл».',detail)
      : label('Не удалось восстановить сохранённый документ: ','Could not restore the saved document: ') + detail
  } finally {
    if (generation === draftRecoveryGeneration) restoringDraft.value = false
  }
}
if (!kernelReady.value || restoringDraft.value) void restoreInitialDraft()
const smoothDisplay = ref(true)
// Exact geometry keys survive history clones and include retained B-rep changes.
const displayCache = new SolidDisplayCache()
function triangleNormals(mesh: PolygonMesh) {
  const count = mesh.indices.length / 3, normals: number[][] = []
  for (let t = 0; t < count; t++) {
    const [a, b, c] = [0, 1, 2].map(k => { const i = mesh.indices[t * 3 + k]; return mesh.positions.slice(i * 3, i * 3 + 3) })
    const u = b.map((v, k) => v - a[k]), w = c.map((v, k) => v - a[k])
    const n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]], len = Math.hypot(...n) || 1
    normals.push(n.map(v => v / len))
  }
  return normals
}
const displayWorker = createSolidPreviewWorker()
const displayQueue = new SolidDisplayQueue(displayWorker, displayCache)
const displayVersion = ref(0)
const displayPending = ref(false)
let displayGeneration = 0
function cancelDisplayPreparation() {
  displayGeneration++
  displayQueue.cancel()
  displayPending.value = false
}
onUnmounted(() => { cancelDisplayPreparation(); displayWorker.dispose() })
function displayMeshFor(b: ReturnType<typeof directExtrusionTool>): DisplayMesh {
  void displayVersion.value
  const transforming = previewingTransform.value && (selectedIds.value.includes(b.id) || !!b.instance && selectedIds.value.includes(b.instance.sourceId))
  if (!transforming && smoothDisplay.value && b.brep && kernelReady.value) {
    const cached = displayQueue.get(b)
    if (cached) return cached
  }
  return { mesh: b.mesh, map: null, normals: triangleNormals(b.mesh) }
}
function shadeFromNormal(n: number[],projectPoint?:(p:number[])=>[number,number,number]): number {
  const light = projectPoint?projectPoint(n):projectDirectPoint([n[0], n[1], n[2]], camera.value)
  return Math.max(24, Math.min(78, 48 + 20 * light[2] - 15 * light[1] + 8 * light[0]))
}
const COARSE_DISPLAY: Pick<DisplayMesh, 'map'> = { map: null }
// WebGPU layer under the 3D SVG: draws dense, per-pixel shaded surfaces with the same camera; the SVG then only
// keeps transparent working-mesh polygons for picking plus gizmos and previews.
const gpuActive = ref(false)
watch(gpuActive,value=>emit('backend',value),{immediate:true})
let gpuLayer: SolidGpuLayer | null = null
let gpuResize: ResizeObserver | null = null
let gpuInitStarted = false
function mountGpuCanvas(el: Element | ComponentPublicInstance | null) {
  if (typeof HTMLCanvasElement === 'undefined' || !(el instanceof HTMLCanvasElement) || gpuInitStarted || !isSolidGpuSupported()) return
  gpuInitStarted = true
  const layer = new SolidGpuLayer(el,()=>{gpuActive.value=false;notice.value=label('WebGPU отключён. Работа продолжена на CPU; можно сохранить проект.','WebGPU stopped. Work continues on CPU; you can save the project.')})
  void layer.init().then(ok => {
    if (!ok || !layer.ready) { layer.destroy(); return }
    gpuLayer = layer
    const wrap = el.parentElement
    if (wrap) {
      gpuResize = new ResizeObserver(() => layer.resize(wrap.clientWidth, wrap.clientHeight, window.devicePixelRatio || 1))
      gpuResize.observe(wrap)
      layer.resize(wrap.clientWidth, wrap.clientHeight, window.devicePixelRatio || 1)
    }
    gpuActive.value = true
  })
}
onUnmounted(() => { gpuResize?.disconnect(); gpuLayer?.destroy(); gpuLayer = null; if (fpsHandle) cancelAnimationFrame(fpsHandle); fpsHandle = 0; clearTimeout(settleHandle) })
watch(() => props.open, open => {
  // The DOM-less test renderer has no frame callbacks, and a hidden workspace need not count.
  if (typeof requestAnimationFrame !== 'function') return
  if (fpsHandle) { cancelAnimationFrame(fpsHandle); fpsHandle = 0 }
  if (!open) { fps.value = 0; frameMs.value = 0; return }
  fpsIntervals = []; fpsPrevious = 0; fpsSince = performance.now(); fpsHandle = requestAnimationFrame(fpsTick)
}, { immediate: true })
const previewReplacesTarget=computed(()=>solidActive.value&&extrusionMode.value!=='new'&&(!!previewBody.value||previewEmpty.value))
// Presentation state stays outside the persisted CAD document and undo history.
const compactWorkspace = ref(true)
const isolatedBodyIds = ref<string[]>([])
const workspaceKey=props.embedded?'scad-main-workspace-v1':'scad-solid-workspace-v1'
watch(restoringDraft,pending=>{
 if(pending)return
 try{
  const state=JSON.parse(storageGet(workspaceKey)??'null')
  if(!state || state.version!==1)return
  const ids=new Set(documentObjects(document.value).map(o=>o.id))
  const read=(value:unknown):string[]=>Array.isArray(value)?value.filter((id):id is string=>typeof id==='string'&&ids.has(id)).slice(0,1000):[]
  hiddenIds.value=read(state.hidden);lockedIds.value=read(state.locked);isolatedBodyIds.value=read(state.isolated)
  if(typeof state.group==='string' && ((document.value.groups??[]).some(g=>g.name===state.group)||documentObjects(document.value).some(o=>o.group===state.group)))activeGroup.value=state.group
 }catch{/* Invalid workspace preferences do not affect the CAD document. */}
},{immediate:true})
watch([hiddenIds,lockedIds,isolatedBodyIds,activeGroup],()=>{
 if(!restoringDraft.value)storageSet(workspaceKey,JSON.stringify({version:1,hidden:hiddenIds.value,locked:lockedIds.value,isolated:isolatedBodyIds.value,group:activeGroup.value}))
})
const sceneBodies = computed(() => document.value.bodies.filter(b => objectVisible(b.id) && (!isolatedBodyIds.value.length || isolatedBodyIds.value.includes(b.id))))
function toggleBodyIsolation() {
  cancelCommandState()
  if (isolatedBodyIds.value.length) { isolatedBodyIds.value = []; return }
  isolatedBodyIds.value = document.value.bodies.filter(b => selectedIds.value.includes(b.id)).map(b => b.id)
  hovered.value = ''
}
watch(() => document.value, d => {
  if (!isolatedBodyIds.value.length) return
  const ids = new Set(documentObjects(d).map(object => object.id))
  const retained = isolatedBodyIds.value.filter(id => ids.has(id))
  // Preview snapshots retain identity; only invalidate visibility when an isolated object disappears.
  if (retained.length !== isolatedBodyIds.value.length) isolatedBodyIds.value = retained
})
const commandFailure = computed(() => Object.keys(invalidQuantities.value).length ? label('Исправьте числовой ввод', 'Correct the numeric input') : advancedOp.value ? advancedPreview.value.error : cornerActive.value ? cornerPreview.value.error : solidActive.value ? previewError.value : operation.value==='array'?sketchEditFailure.value:'')
const commandActive = computed(() => !!(advancedOp.value || operation.value || subtract.value || booleanPending.value || directTransformPending.value || gizmoPending.value || nativeNurbsPending.value))
const commandHint = computed(() => {
  if(nativeNurbsPending.value&&nativeBrepMode.value)return label('Вычисляется B-rep. Esc — отменить.', 'Computing B-rep. Esc cancels.')
  if(nativeNurbsPending.value)return label('Вычисляется NURBS-команда. Esc — отменить.', 'Computing NURBS command. Esc cancels.')
  if(gizmoPending.value)return label('Вычисляется преобразование. Esc — отменить.', 'Computing transform. Esc cancels.')
  if(directTransformPending.value)return label('Вычисляется преобразование. Esc — отменить.', 'Computing transform. Esc cancels.')
  if(sketchEditPending.value)return label('Вычисляется предпросмотр. Esc — отменить.', 'Computing preview. Esc cancels.')
  if(booleanPending.value)return label('Вычисляется Boolean. Esc — отменить.', 'Computing Boolean. Esc cancels.')
  if (commandFailure.value) return label('Измените параметры или выбор. Исходная модель сохранена.', 'Change parameters or selection. The original model is preserved.')
  if (subtract.value) return subtract.value.active === 'a' ? label('Выберите тела A, из которых нужно вычесть.', 'Select target bodies A.') : label('Выберите режущие тела B.', 'Select cutting bodies B.')
  if (advancedOp.value === 'nurbs-prepare') return label('Подготовка меняет базис обеих поверхностей. Проверьте отклонение и подтвердите.', 'Preparation changes both surface bases. Check the deviation and confirm.')
  if (advancedOp.value === 'profile-difference') return label('Выберите основной профиль. Остальные вырезают из него области. Enter — применить, Esc — отменить.','Choose the target profile. The other profiles remove material from it. Enter applies, Esc cancels.')
  if (advancedOp.value === 'profile-intersection') return label('Оставьте общую область выбранных профилей. Enter — применить, Esc — отменить.','Keep the common region of the selected profiles. Enter applies, Esc cancels.')
  if (advancedOp.value === 'profile-union') return label('Объедините замкнутые профили. Линии и круговые дуги сохраняются. Enter — применить, Esc — отменить.','Union closed profiles. Lines and circular arcs are retained. Enter applies, Esc cancels.')
  if (advancedOp.value === 'profile-prepare') return label('Соберите один замкнутый профиль. Допуск добавляет прямые соединения; Enter — применить, Esc — отменить.','Prepare one closed profile. Tolerance adds straight connectors; Enter applies, Esc cancels.')
  if (advancedOp.value === 'nurbs-point-trim') return label('Задайте точку и сохраняемый конец. Зелёный — результат; Enter применяет, Esc отменяет.','Set the cut point and retained endpoint. Green is the result; Enter applies, Esc cancels.')
  if (advancedOp.value === 'nurbs-curve-match') return label('Выберите концы A и B, проверьте угловой допуск. Enter — применить, Esc — отменить.','Choose endpoints A and B and check the angular tolerance. Enter applies; Esc cancels.')
  if (advancedOp.value === 'nurbs-match') return label('A — опорная поверхность, B — изменяемая. Проверьте границы и допуск перед применением.', 'A is the reference surface; B is edited. Check the boundaries and tolerance before applying.')
  if (advancedOp.value === 'loft') return label('Выберите сечения по порядку, затем проверьте результат.', 'Select sections in order, then inspect the result.')
  if (cornerActive.value) return label('Выберите вершину и задайте радиус.', 'Choose a vertex and enter the radius.')
  if (solidActive.value) return label('Задайте размер и проверьте предпросмотр.', 'Enter a dimension and inspect the preview.')
  return label('Измените параметры и проверьте предпросмотр.', 'Adjust parameters and inspect the preview.')
})
/** Scene controls cancel tool work while keeping their native keyboard focus. */
function cancelCommandState() {
  cancelSurfaceDisplay()
  cancelProfileDisplay()
  cancelCurveDisplay()
  cancelTopology()
  cancelBodyEdges()
  cancelSnapPreparation();cancelSketchSnaps()
  surfaceDistanceOpen.value=false
  boundaryInspection.value=false
  measurementOpen.value=false
  cancelPrimitive()
  if (restoringDraft.value) cancelDraftRecovery()
  cancelDisplayPreparation()
  cancelHistoryRestore()
  cancelDirectTransform(); cancelSketchEdit(); cancelBoolean(); invalidateSolidPreview(); invalidQuantities.value = {}
  cancelGesture(); operation.value = null; advancedOp.value = null; subtract.value = null
  previewBody.value = null; previewEmpty.value = false; previewError.value = ''
}
function cancelCommand(){cancelCommandState();workspace.value?.focus()}

function quantityValidity(key: string, valid: boolean) { if (valid) delete invalidQuantities.value[key]; else invalidQuantities.value[key] = true }
const commandReady = computed(() => {
  if (nativeNurbsPending.value || gizmoPending.value || directTransformPending.value || sketchEditPending.value || booleanPending.value || Object.keys(invalidQuantities.value).length) return false
  if (subtract.value) return !!(subtract.value.a.length && subtract.value.b.length)
  if (advancedOp.value) return !!advancedPreview.value.document
  if (solidActive.value) return !previewPending.value && !previewError.value && !!(previewBody.value || previewEmpty.value) && (extrusionMode.value === 'new' || !!targetBody.value)
  if (cornerActive.value) return !!cornerPreview.value.sketch
  return !!operation.value && !!copyPreview.value.length
})
function applyCommand() {
  if (!commandReady.value) return
  const snapshot = captureCommand(), before = document.value
  if (subtract.value) applySubtract()
  else if (advancedOp.value) applyAdvanced()
  else if (solidActive.value) extrude()
  else if (cornerActive.value) applyCorner()
  else applyCopies()
  if (document.value !== before) { if(snapshot) lastCommand.value = snapshot; workspace.value?.focus() }
}
watch(()=>advancedOp.value??operation.value,async command=>{
  if(!command)return
  // Palette unmount also restores focus on nextTick; the command owns focus after that.
  await nextTick();await nextTick()
  if(!props.open||(advancedOp.value??operation.value)!==command)return
  const input=workspace.value?.querySelector?.<HTMLInputElement>('.operation-card input:not([disabled]):not([type=checkbox]):not([type=radio]), .operation-card select:not([disabled])')
  input?.focus();input?.select?.()
})
const commandAnchor = computed(() => {
  const point = solidActive.value && extrusionHandle.value ? extrusionHandle.value.top : selectedFace.value ? project(selectedFace.value.center, '3d') : gizmoCenter.value ? project(gizmoCenter.value, '3d') : null
  return point
})
const inlineDimension = computed({
  get: () => solidActive.value ? height.value : advancedOp.value === 'edge-fillet' || advancedOp.value === 'chamfer' ? advanced.value.radius : advanced.value.distance,
  set: (value: number) => {
    if (solidActive.value) height.value = value
    else if (advancedOp.value === 'edge-fillet' || advancedOp.value === 'chamfer') advanced.value.radius = value
    else advanced.value.distance = value
  },
})
const inlineDimensionVisible = computed(() => operation.value === 'extrude' || ['push','shell','split','edge-fillet','chamfer'].includes(advancedOp.value ?? ''))
watch(() => document.value, () => {
  if(activeGroup.value && !(document.value.groups??[]).some(g=>g.name===activeGroup.value) && !documentObjects(document.value).some(b=>b.group===activeGroup.value))activeGroup.value=''
})
const selectedGroup = computed(() => documentObjects(document.value).find(o=>o.id===selection.value)?.group ?? '')

const renderBodies=computed(()=>previewReplacesTarget.value?sceneBodies.value.flatMap(b=>b.id===targetBody.value?(previewBody.value?[previewBody.value]:[]):[b]):sceneBodies.value)
watch([smoothDisplay, kernelReady], () => {
  cancelDisplayPreparation()
  displayCache.clear()
  displayVersion.value++
}, { flush: 'sync' })
watch(() => [props.open, kernelReady.value, smoothDisplay.value, renderBodies.value,
  previewBody.value, advancedPreview.value.document, previewingTransform.value,
  previewingTransform.value ? selectedIds.value.join(',') : ''] as const, async () => {
  cancelDisplayPreparation()
  if (!props.open || !kernelReady.value) return
  const generation = displayGeneration
  const bodies = [...renderBodies.value, ...(previewBody.value ? [previewBody.value] : []), ...(advancedPreview.value.document?.bodies ?? [])]
    .filter(b => (smoothDisplay.value || b.brep?.shells.some(s=>s.closed) && b.brep.shells.some(s=>!s.closed)) && !(previewingTransform.value && (selectedIds.value.includes(b.id) || !!b.instance && selectedIds.value.includes(b.instance.sourceId))))
  displayPending.value = true
  try {
    const changed = await displayQueue.prepare(bodies)
    if (generation === displayGeneration && changed) displayVersion.value++
  } finally {
    if (generation === displayGeneration) displayPending.value = false
  }
}, { flush: 'pre', immediate: true })
function vrSnapshot() {
  return prepareVrPolygons(renderBodies.value.map(body => ({ ...displayMeshFor(body).mesh, color: body.material ? solidMaterialRgb(body.material.color) : undefined })))
}
let surfaceGpuCache=new Map<PolygonMesh,ReturnType<typeof smoothTriangleList>>()
onUnmounted(()=>surfaceGpuCache.clear())
const gpuBodies = computed<SolidGpuBody[]>(() => {
  if (!gpuActive.value) {surfaceGpuCache.clear();return []}
  const hideSelected = !!advancedPreview.value.document
  const items=[...renderBodies.value.filter(b=>!hideSelected||!advancedBodyIds.value.has(b.id)).map(b=>({b,preview:0})),...(advancedPreview.value.document?.bodies??[]).filter(b=>advancedBodyIds.value.has(b.id)).map(b=>({b,preview:1})),...(previewBody.value&&!previewReplacesTarget.value?[{b:previewBody.value,preview:2}]:[])]
  const bodies=items.flatMap(({b,preview}) => {
    const display = displayMeshFor(b)
    const flat = display.flat ??= smoothTriangleList(display.mesh.positions, display.mesh.indices)
    const count = display.mesh.indices.length / 3
    const bodyHue = subtract.value?.a.includes(b.id) ? 45 : subtract.value?.b.includes(b.id) ? 350 : selectedIds.value.includes(b.id) ? 266 : hovered.value === b.id ? 190 : b.material ? -1 : 220
    const faceHighlight = selectedBody.value?.id === b.id && pickMode.value === 'face'
    const hues = new Float32Array(count)
    for (let i = 0; i < count; i++) {
      const working = display.map ? display.map[i] : i
      hues[i] = preview ? -1 : faceHighlight && selectedFaceTriangles.value.has(working) ? 40 : bodyHue
    }
    return [{ id: b.id, positions: flat.positions, normals: flat.normals, hues, color:preview?solidMaterialRgb(preview===2?(extrusionMode.value==='difference'?'#ff647c':'#75e4b8'):b.id==='preview-split'?'#ffc977':'#77eac5'):b.material?solidMaterialRgb(b.material.color):undefined, metallic:preview?0:b.material?.metallic,roughness:b.material?.roughness,opacity:preview?(preview===2?.28:.45):b.material?.opacity }]
  })
  void surfaceDisplayVersion.value
  const retainedSurfaceMeshes=new Map<PolygonMesh,ReturnType<typeof smoothTriangleList>>()
  const surfaceItems=[...(document.value.surfaces??[]).filter(item=>objectInView(item.id)&&!(hideSelected&&advancedSurfaceIds.value.includes(item.id))).map(item=>({item,preview:false})),...(advancedPreview.value.document?.surfaces??[]).filter(item=>advancedSurfaceIds.value.includes(item.id)).map(item=>({item,preview:true}))]
  for(const {item,preview} of surfaceItems){
    const mesh=surfaceDisplayQueue.get(item)
    if(!mesh)continue
    const flat=surfaceGpuCache.get(mesh)??smoothTriangleList(mesh.positions,mesh.indices)
    retainedSurfaceMeshes.set(mesh,flat)
    bodies.push({id:item.id,positions:flat.positions,normals:flat.normals,hues:new Float32Array(mesh.indices.length/3).fill(!preview&&selectedIds.value.includes(item.id)?266:-1),color:preview?[.467,.918,.773]:[.192,.373,.447],metallic:undefined,roughness:undefined,opacity:preview ? .45 : .72})
  }
  surfaceGpuCache=retainedSurfaceMeshes
  return bodies
})
watch(gpuBodies, bodies => gpuLayer?.setBodies(bodies), { flush: 'post' })
watchEffect(() => {
  if (!gpuActive.value) return
  const size = views.value['3d'], center = centers.value['3d']
  gpuLayer?.setView({ camera: camera.value, viewBox: [center[0] - size / 2, center[1] - size / 2, size] })
})
watch(gpuActive, active => { if (active && gpuLayer) gpuLayer.setBodies(gpuBodies.value) })
function meshPolygons(b: ReturnType<typeof directExtrusionTool>,preview=0,projectPoint=createDirectProjector(camera.value),world=false) {
  const display: DisplayMesh = cameraDragging.value || gpuActive.value ? { ...COARSE_DISPLAY, mesh: b.mesh, normals: world?triangleNormals(b.mesh):[] } : displayMeshFor(b)
  const closed = display.closed ?? (b.brep?.shells.some(s=>!s.closed) ? displayQueue.get(b)?.workClosed : null)
  const allClosed = !!b.brep?.shells.length && b.brep.shells.every(s=>s.closed)
  const mesh = display.mesh, points = bodyPoints({ ...b, mesh }).map(projectPoint)
  return Array.from({ length: mesh.indices.length / 3 }, (_, i) => {
    const face = Array.from(mesh.indices.slice(i * 3, i * 3 + 3), j => points[j])
    if (!world && !preview && (b.material?.opacity??1)===1 && (closed?.[i] ?? allClosed) && (face[1][0]-face[0][0])*(face[2][1]-face[0][1])-(face[1][1]-face[0][1])*(face[2][0]-face[0][0])<=0) return null
    const triangle = display.map ? display.map[i] : i
    return { preview, surface:false, normal:display.normals[i], closed:closed?.[i]??allClosed, vertices:face, id: b.id, material:b.material, triangle, key: b.id + ':' + i, points: face.map(p => p.slice(0,2).join(',')).join(' '), shade: world ? 0 : display.map ? shadeFromNormal(display.normals[i],projectPoint) : directFaceShade(mesh, i, camera.value,projectPoint), depth: face.reduce((n,p) => n + p[2], 0) }
  }).filter(p=>p!==null)
}
/**
 * While the GPU layer draws the scene, these thousands of polygons are transparent hit
 * targets and nothing more. Re-projecting and re-rendering them mid-orbit costs more
 * than a frame and changes nothing anyone can see, so they are held still until the
 * camera settles, then rebuilt once for picking.
 */
const worldPointProjection=(p:number[]):[number,number,number]=>[p[0],p[1],p[2]??0]
const cpuTransparency=computed(()=>{
 if(gpuActive.value)return null
 const bodies=[...renderBodies.value.filter(b=>!advancedPreview.value.document||!advancedBodyIds.value.has(b.id)).map(b=>({b,preview:0})),...(advancedPreview.value.document?.bodies??[]).filter(b=>advancedBodyIds.value.has(b.id)).map(b=>({b,preview:1})),...(!previewReplacesTarget.value&&previewBody.value?[{b:previewBody.value,preview:2}]:[])]
 const surfaces=[...(document.value.surfaces??[]).filter(item=>objectInView(item.id)&&(!advancedPreview.value.document||!advancedSurfaceIds.value.includes(item.id))).map(item=>({item,preview:0})),...(advancedPreview.value.document?.surfaces??[]).filter(item=>advancedSurfaceIds.value.includes(item.id)).map(item=>({item,preview:1}))]
 if(!surfaces.length&&!bodies.some(({b,preview})=>preview||(b.material?.opacity??1)<1))return null
 const items=[...bodies.flatMap(({b,preview})=>meshPolygons(b,preview,worldPointProjection,true)),...surfaces.flatMap(({item,preview})=>{void surfaceDisplayVersion.value;const mesh=surfaceDisplayQueue.get(item);return mesh?meshPolygons({id:item.id,name:item.name,mesh},preview,worldPointProjection,true).map(p=>({...p,surface:true})):[]})]
 try{return {items,tree:new TransparentBsp(items.map(p=>({owner:p.key,triangle:p.vertices as unknown as [Vec3,Vec3,Vec3]})))}}catch{return {items,tree:null}}
})
let restingPolygons: ReturnType<typeof meshPolygons> = []
const polygonView = computed(() => {
  let limited=false
  if (gpuActive.value && (cameraDragging.value || settling.value) && restingPolygons.length) return {items:restingPolygons,limited}
  const projectPoint=createDirectProjector(camera.value)
  const transparent=cpuTransparency.value
  if(!transparent?.tree){
  restingPolygons = renderBodies.value.flatMap(b=>meshPolygons(b,0,projectPoint)).sort((a, b) => a.depth - b.depth)

  }
  if(transparent){
   limited=!transparent.tree
   {
    const source=new Map(transparent.items.map(p=>[p.key,p])),direction=[Math.sin(camera.value.yaw)*Math.cos(camera.value.pitch),Math.cos(camera.value.yaw)*Math.cos(camera.value.pitch),Math.sin(camera.value.pitch)] as Vec3
    let fragments
    try{fragments=transparent.tree?.ordered(direction)}catch{limited=true}
    restingPolygons=(fragments??transparent.items.map(p=>({owner:p.key,triangle:p.vertices}))).flatMap((fragment,i)=>{
     const original=source.get(fragment.owner)!,vertices=fragment.triangle.map(p=>projectPoint([...p])),a=vertices[0],b=vertices[1],c=vertices[2]
     if(original.closed&&!original.preview&&!original.surface&&(original.material?.opacity??1)===1&&(b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])<=0)return []

     return [{...original,key:original.key+':split:'+i,vertices,depth:vertices.reduce((sum,p)=>sum+p[2]/3,0),points:vertices.map(p=>p.slice(0,2).join(',')).join(' '),shade:shadeFromNormal(original.normal!,projectPoint)}]
    })
    if(limited)restingPolygons.sort((a,b)=>a.depth-b.depth)
   }
  }
  return {items:restingPolygons,limited}
})
const polygons=computed(()=>polygonView.value.items)
const cpuOrbitReady=ref(false)
const cpuOrbitActive=computed(()=>!gpuActive.value&&cameraDragging.value&&polygons.value.length>1200)
function polygonColor(p:typeof polygons.value[number]){return p.preview?(p.preview===2?(extrusionMode.value==='difference'?'#ff647c':'#75e4b8'):p.id==='preview-split'?'#ffc977':'#77eac5'):p.surface?(selectedIds.value.includes(p.id)?'#8061bd':'#315f72'):p.material&&!selectedIds.value.includes(p.id)&&hovered.value!==p.id&&!subtract.value?.a.includes(p.id)&&!subtract.value?.b.includes(p.id)?`rgb(${solidMaterialRgb(p.material.color).map(c=>c*255*p.shade/78).join(' ')})`:`hsl(${selectedBody.value?.id===p.id&&selectedFaceTriangles.value.has(p.triangle)&&pickMode.value==='face'?40:subtract.value?.a.includes(p.id)?45:subtract.value?.b.includes(p.id)?350:selectedIds.value.includes(p.id)?266:hovered.value===p.id?190:220} 45% ${p.shade}%)`}
const cpuOrbitTriangles=computed(()=>cpuOrbitActive.value?polygons.value.map(p=>({vertices:p.vertices,
 color:polygonColor(p),
 opacity:p.preview?(p.preview===2?.28:.45):p.surface?.72:p.material?.opacity??1,stroke:!p.preview&&!p.surface&&(p.material?.opacity??1)===1
})):[])


const surfaceDisplayWorker=createSolidPreviewWorker(),surfaceDisplayQueue=new SolidSurfaceDisplayQueue(surfaceDisplayWorker)
const surfaceDisplayPending=ref(false),surfaceDisplayVersion=ref(0),surfaceDisplayErrors=ref<{id:string;message:string}[]>([])
let surfaceDisplayGeneration=0
function cancelSurfaceDisplay(){surfaceDisplayGeneration++;surfaceDisplayQueue.cancel();surfaceDisplayPending.value=false}
const surfaceDisplayNeedsRetry=computed(()=>{
 void surfaceDisplayVersion.value
 return !surfaceDisplayPending.value&&[...(document.value.surfaces??[]),...(advancedPreview.value.document?.surfaces??[])].some(item=>!surfaceDisplayQueue.get(item))
})
onUnmounted(()=>{cancelSurfaceDisplay();surfaceDisplayWorker.dispose();surfaceDisplayQueue.clear()})
function surfacePolygons(item:SolidNurbsSurface){
 void surfaceDisplayVersion.value
 const mesh=surfaceDisplayQueue.get(item)
 return mesh?meshPolygons({id:item.id,name:item.name,mesh}):[]
}
let restingSurfacePolygons: ReturnType<typeof meshPolygons> = []
const nurbsSurfacePolygons = computed(() => {
  if (gpuActive.value && cameraDragging.value && restingSurfacePolygons.length) return restingSurfacePolygons
  restingSurfacePolygons = !kernelReady.value ? [] : (document.value.surfaces ?? []).filter(item=>objectInView(item.id)).flatMap(item => {
    return surfacePolygons(item).sort((a,b)=>a.depth-b.depth)
  })
  return restingSurfacePolygons
})
const nurbsCurvePaths = computed(() => (document.value.curves ?? []).filter(item=>objectInView(item.id)).map(item => ({
  id: item.id,
  points: curvePoints(item.curve).map(point => project([point[0], point[1], point[2] ?? 0], '3d').join(',')).join(' '),
})))
const nativeCage = computed(() => {
  if (selectedNurbsCurve.value) return selectedNurbsCurve.value.curve.controlPoints.map((point, u) => ({ point, u, v: 0 }))
  if (selectedNurbsSurface.value) return selectedNurbsSurface.value.surface.controlPoints.flatMap((row, u) => row.map((point, v) => ({ point, u, v })))
  return []
})
/**
 * Floor grid at two decades with a continuous cross-fade.
 *
 * A single step chosen as a power of ten snaps to a tenfold different spacing the moment
 * the zoom crosses a decade, which reads as the grid changing under the cursor. Instead
 * the fine step fades out as the view grows toward the next decade while the coarse step
 * stays, so the visible density is constant and nothing jumps.
 */
const sketchGridStep = computed(() => grid.value * Math.pow(10, Math.max(0, Math.ceil(Math.log10(views.value['2d'] / (100 * grid.value))))))
const ghostPolygons = computed(() => previewBody.value ? meshPolygons(previewBody.value,2).sort((a,b)=>a.depth-b.depth) : [])
const extrusionHandle = computed(() => {
  if (operation.value !== 'extrude' || !selectedSketch.value) return null
  const p = selectedSketch.value.points, center = [p.reduce((s,v)=>s+v[0],0)/p.length,p.reduce((s,v)=>s+v[1],0)/p.length]
  return { base: project(worldPoint([...center,baseZ.value],selectedSketch.value.plane),'3d'), top: project(worldPoint([...center,baseZ.value+height.value],selectedSketch.value.plane),'3d') }
})
function dragHeight(e: PointerEvent) {
  e.preventDefault(); const svg = (e.target as SVGElement).ownerSVGElement!
  heightDrag = { y: e.clientY, height: height.value, pointer: e.pointerId, svg }; svg.setPointerCapture(e.pointerId)
}
function canvasOf(e: PointerEvent): SVGSVGElement { const target = e.target as SVGElement; return (target instanceof SVGSVGElement ? target : target.ownerSVGElement)! }
function snapProjection(svg: SVGSVGElement, pane: Pane): (p: Vec3) => Point2 {
  const matrix = svg.getScreenCTM()
  const scale = Math.min(svg.clientWidth || 600, svg.clientHeight || 600) / views.value[pane]
  return p => {
    const projected = pane === '2d' ? [p[0], -p[1]] : project(p, '3d')
    if (matrix && Number.isFinite(matrix.a)) return [matrix.a*projected[0]+matrix.c*projected[1]+matrix.e,matrix.b*projected[0]+matrix.d*projected[1]+matrix.f]
    return [projected[0] * scale, projected[1] * scale]
  }
}
function pickNurbsCurve(e:PointerEvent,id:string){
 if(advancedOp.value!=='nurbs-point-trim'||selection.value!==id){pickObject(id,'3d');return}
 if(e.button!==0)return
 for(const key of ['pointTrimx','pointTrimy','pointTrimz','pointTrimDistance'])quantityValidity(key,true)
 const svg=canvasOf(e);svg.focus()
 const toScreen=snapProjection(svg,'3d'),origin=toScreen([0,0,0]),x=toScreen([1,0,0]),y=toScreen([0,1,0]),z=toScreen([0,0,1])
 pointTrimPick.value={point:[e.clientX,e.clientY],matrix:[0,1].map(i=>[x[i]-origin[i],y[i]-origin[i],z[i]-origin[i],origin[i]]) as NurbsScreenProjection,radius:snapRadius.value}
}
function pointTrimNumeric(){
 const p=pointTrimCut.value
 if(p)advanced.value={...advanced.value,x:p[0],y:p[1],z:p[2]??0}
 pointTrimPick.value=null
}
function showSnap(result: ReturnType<typeof resolveModelingSnap>, pane: Pane) {
  snapPane.value = pane; snapKind.value = result.kind
  snapMarker.value = result.kind ? (pane === '2d' ? [result.point[0], result.point[1]] : project(result.point, '3d')) : null
  snapGuide.value = result.guide ? result.guide.map(p => pane === '2d' ? [p[0], -p[1]] : project(p, '3d')) as [Point2,Point2] : null
}
function snapped(p: Point2, e: PointerEvent, anchor?: Point2, pane: Pane='2d'): Point2 {
  if (!snap.value || e.altKey) { snapMarker.value = null; snapGuide.value = null; return p }
  const excluded = curveDrag ? [curveDrag.id] : gesture?.id ? selectedIds.value : []
  const geometry:SnapGeometry={points:[],segments:[],circles:[]}
  for(const sketch of localSnapSketches().filter(s=>!excluded.includes(s.id))){
    const targets=sketchSnapPreparation.get(sketch);if(!targets)continue
    geometry.points.push(...targets.points);geometry.segments.push(...targets.segments);geometry.circles!.push(...targets.circles??[])
  }
  geometry.points.push({point:[0,0,0],kind:'origin'})
  const screenProjection=snapProjection(canvasOf(e),pane)
  const localProjection=(p:Vec3)=>screenProjection(pane==='3d'?worldPoint(p,activePlane.value):p)
  for (const point of draft.value) if (tool.value === 'polyline') geometry.points.push({point:[...point,0],kind:'vertex'})
  const result = resolveModelingSnap([...p,0], geometry, {
    project: localProjection, grid: gridSnap.value ? grid.value : 0,
    geometry: geometrySnap.value, keypoints:keypointSnap.value, relations:relationSnap.value,radius:snapRadius.value, guides: guideSnap.value, anchor: anchor ? [...anchor,0] : undefined, gridAxes:[0,1],
  })
  if(pane==='3d')showSnap({...result,point:worldPoint(result.point,activePlane.value),guide:result.guide?.map(p=>worldPoint(p,activePlane.value)) as [Vec3,Vec3]|undefined},pane)
  else showSnap(result,pane)
  return [result.point[0],result.point[1]]
}
let snapSceneCache: { document: DirectDocument; excluded: string; geometry: SnapGeometry } | undefined
function solidSnapGeometry(source: DirectDocument, excluded: string[]): SnapGeometry {
  const key = excluded.join('|')
  if (snapSceneCache?.document === source && snapSceneCache.excluded === key) return snapSceneCache.geometry
  const geometry: SnapGeometry = {points:[],segments:[]}
  for (const body of source.bodies) if (objectVisible(body.id) && !excluded.includes(body.id) && (!isolatedBodyIds.value.length || isolatedBodyIds.value.includes(body.id))) {
    const bodyGeometry=snapPreparation.get(body)
    if(!bodyGeometry)continue
    geometry.points.push(...bodyGeometry.points);geometry.segments.push(...bodyGeometry.segments)
  }
  for(const sketch of source.sketches)if(objectInView(sketch.id) && !excluded.includes(sketch.id)){
    const targets=sketchSnapPreparation.get(sketch),plane=sketch.plane??xyPlane()
    if(!targets)continue
    const place=(p:Vec3)=>plane.origin.map((v,i)=>v+p[0]*plane.u[i]+p[1]*plane.v[i]) as Vec3
    geometry.points.push(...targets.points.map(p=>({...p,point:place(p.point)})))
    const curves=new Map<NurbsCurve,NurbsCurve>()
    geometry.segments.push(...targets.segments.map(edge=>{
      const curve=edge.nurbs?.curve
      if(curve&&!curves.has(curve))curves.set(curve,{...curve,controlPoints:curve.controlPoints.map(p=>place(p as Vec3))})
      return {a:place(edge.a),b:place(edge.b),...(edge.nurbs?{nurbs:{...edge.nurbs,curve:curves.get(curve!)!}}:{})}
    }))
    if(sketch.analytic){const a=sketch.analytic,count=64,evaluate=(t:number)=>place([a.center[0]+a.radius*Math.cos((a.start+a.sweep*t)*Math.PI/180),a.center[1]+a.radius*Math.sin((a.start+a.sweep*t)*Math.PI/180),0]);for(let i=0;i<count;i++)geometry.segments.push({a:evaluate(i/count),b:evaluate((i+1)/count),arc:{plane,center:a.center,radius:a.radius,start:a.start+a.sweep*i/count,sweep:a.sweep/count}})}
  }
  geometry.points.push({point:[0,0,0],kind:'origin'})
  snapSceneCache = {document:source,excluded:key,geometry}; return geometry
}
const visibleKeypoints = computed(() => {
  void snapPreparationVersion.value;void sketchSnapVersion.value
  if(!kernelReady.value||!snap.value||!geometrySnap.value||!keypointSnap.value||solidActive.value)return []
  const points:{point:Vec3;local?:Point2;kind:SnapKind}[]=[]
  for(const sketch of visibleSketches.value)if(selectedIds.value.includes(sketch.id)||hovered.value===sketch.id){
    for(const target of (sketchSnapPreparation.get(sketch)?.points??[]))if(['center','bounds-center','quadrant'].includes(target.kind))points.push({point:worldPoint(target.point,sketch.plane),local:[target.point[0],target.point[1]],kind:target.kind})
  }
  for(const body of sceneBodies.value)if(selectedIds.value.includes(body.id)||hovered.value===body.id){
    for(const target of (snapPreparation.get(body)?.points??[]))if(['center','bounds-center','quadrant'].includes(target.kind))points.push(target)
  }
  return points
})
function requireBodySnaps(e:PointerEvent,source:DirectDocument){
 if(!snap.value||!geometrySnap.value||e.altKey)return true
 if(source.bodies.every(body=>!objectInView(body.id)||snapPreparation.get(body)))return requireSketchSnaps(e,source,false)
 error.value=label('Привязки к телам ещё не готовы. Дождитесь подготовки, повторите её или удерживайте Alt для работы без привязок.','Body snaps are not ready. Wait, retry preparation, or hold Alt to work without snapping.')
 return false
}
function localSnapSketches(source:DirectDocument=snapDocument.value){return [...source.sketches.filter(s=>objectInView(s.id)&&samePlane(s.plane,activePlane.value)),...supportSnapSketches.value]}
function requireSketchSnaps(e:PointerEvent,source:DirectDocument=snapDocument.value,local=true){
 if(!snap.value||!geometrySnap.value||e.altKey)return true
 const sketches=local?localSnapSketches(source):source.sketches.filter(s=>objectInView(s.id))
 if(sketches.every(s=>sketchSnapPreparation.get(s)))return true
 error.value=label('Привязки эскизов ещё не готовы. Дождитесь подготовки, повторите её или удерживайте Alt для работы без привязок.','Sketch snaps are not ready. Wait, retry preparation, or hold Alt to work without snapping.')
 return false
}
function requireLocalGestureSnaps(e:PointerEvent){return !(curveDrag||gesture&&!gesture.pan&&(gesture.pane==='2d'||gesture.sketch))||requireSketchSnaps(e,curveDrag?.before??gesture?.document)}
function gestureSnapSource(){return vertexDrag?.before??cvDrag?.before??(manipulatorDrag?.kind==='move'?manipulatorDrag.before:undefined)??(gesture?.pane==='3d'&&!gesture.pan?gesture.document:undefined)}
function snapped3(p: Vec3, e: PointerEvent, source: DirectDocument, excluded: string[], anchor?: Vec3, axis?: number, xyOnly=false): Vec3 {
  if (!snap.value || e.altKey) { snapMarker.value=null; snapGuide.value=null; return p }
  const result=resolveModelingSnap(p,solidSnapGeometry(source,excluded),{
    project:snapProjection(canvasOf(e),'3d'), grid:gridSnap.value?grid.value:0, geometry:geometrySnap.value,keypoints:keypointSnap.value,relations:relationSnap.value,radius:snapRadius.value,
    guides:guideSnap.value, anchor, gridAxes:axis!==undefined?[axis]:xyOnly?[0,1]:[0,1,2],
    constrain:axis!==undefined&&anchor?target=>anchor.map((v,i)=>i===axis?target[i]:v) as Vec3:xyOnly?target=>[target[0],target[1],p[2]]:undefined,
  });showSnap(result,'3d');return result.point
}
watch([isolatedBodyIds,hiddenIds], () => { snapSceneCache = undefined })
function snapDistance(value: number, e: PointerEvent) { return snap.value && gridSnap.value && !e.altKey ? Math.round(value/grid.value)*grid.value : value }
function position(e: PointerEvent): Point2 {
  const target = e.target as SVGElement
  const svg = gesture?.svg ?? (target instanceof SVGSVGElement ? target : target.ownerSVGElement)!
  const matrix = svg.getScreenCTM()
  if (!matrix) throw new Error('Canvas is not ready.')
  const point = new DOMPoint(e.clientX, e.clientY).matrixTransform(matrix.inverse())
  return [point.x, point.y]
}
function plane(p: Point2, pane: Pane): Point2 { return pane === '2d' ? [p[0], -p[1]] : unprojectDirectXY(p,camera.value) }
function cancelGesture() { cancelFaceSketch(); cancelNativeNurbs();cancelGizmoWorker();dragConstraint.value=''; draftCursor.value=null;choosingSketchFace.value=false; clearDragPreview(); previewingTransform.value=false; if(vertexDrag){document.value=vertexDrag.before;vertexDrag=null} if(cvDrag){document.value=cvDrag.before;cvDrag=null} if(curveDrag){document.value=curveDrag.before;curveDrag=null} if(manipulatorDrag){document.value=manipulatorDrag.before;if(manipulatorDrag.kind==='push')advancedOp.value=null;if(manipulatorDrag.kind==='split')advanced.value.distance=manipulatorDrag.initial;manipulatorDrag=null}selectionBox.value=null; if (gesture) { if(!gesture.pan)document.value = gesture.document; gesture = null } if (heightDrag) height.value = heightDrag.height; heightDrag = null; orbitDrag = null; draft.value = []; drawMeasure.value = ''; snapMarker.value = null; cameraDragging.value = false }
function down(e: PointerEvent, pane: Pane, id = '', vertex: number | null = null, triangle = -1) {
 cancelNativeNurbs();cancelGizmoWorker()
 if(directTransformPending.value)cancelDirectTransform()
  if(id && !objectSelectable(id)) return
  if (![0, 1, 2].includes(e.button) || gesture) return
  if(e.button===0&&id&&e.shiftKey&&pickMode.value==='body'){pickObject(id,pane,true);return}
  const pan = e.button === 1 || e.shiftKey || (pane === '2d' && e.button === 2)
  const target = e.target as SVGElement
  const svg = (target instanceof SVGSVGElement ? target : target.ownerSVGElement)!
  svg.focus(); mode.value = pane
  if(pane==='3d'&&e.button===0&&!pan&&!faceDrawing.value&&pickMode.value==='face'&&id&&triangle>=0){
    extraSelection.value=[]
    const body=document.value.bodies.find(b=>b.id===id)
    if(!body)return
    const ready=topologyBody===body&&!topologyPending.value&&topologyRequest!==null
    const choose=(result:SolidTopology,allowDrag:boolean)=>{
      const candidate=result.faces.findIndex(f=>f.triangles.includes(triangle))
      if(candidate<0)return
      if(choosingSketchFace.value){faceIndex.value=candidate;faceSketch();return}
      if(allowDrag&&faceIndex.value===candidate&&!e.ctrlKey&&!e.metaKey){startGizmo(e,'push','z');return}
      faceIndex.value=candidate;edgeIndex.value=-1;edgeIndexes.value=[]
      if(e.ctrlKey||e.metaKey){const set=new Set(openingFaces.value);set.has(candidate)?set.delete(candidate):set.add(candidate);openingFaces.value=[...set]}else openingFaces.value=[candidate]
      advancedOp.value=null
    }
    if(selection.value!==id)pickObject(id,pane)
    if(ready)choose(topology.value,true)
    else {const request=prepareTopology(body),pick=++topologyPick;void request.then(result=>{if(result&&pick===topologyPick&&selectedBody.value===body)choose(result,false)})}
    return
  }
  if(boxSelect.value&&e.button===0&&!pan){const p=position(e);selectionBox.value={start:p,end:p,pane};svg.setPointerCapture(e.pointerId);return}
  if (pane === '3d' && !pan && (e.button === 2 || (!faceDrawing.value && !movingBody.value && e.button === 0))) {
    if (id && !operation.value && !advancedOp.value) {if(!selectedIds.value.includes(id))pickObject(id,pane)}
    orbitDrag = { x:e.clientX, y:e.clientY, yaw:camera.value.yaw, pitch:camera.value.pitch, pointer:e.pointerId, svg }; cameraDragging.value = true; svg.setPointerCapture(e.pointerId); return
  }
  const drawing = pane==='2d'||(pane==='3d'&&faceDrawing.value)
  if(!pan&&!(drawing?requireSketchSnaps(e):requireBodySnaps(e,document.value)))return
  if (pan) { gesture = { start: position(e), document: document.value, vertex: null, id: '', pointer: e.pointerId, pane, svg, pan: true, center: [...centers.value[pane]] }; cameraDragging.value = true; svg.setPointerCapture(e.pointerId); return }
  let p: Point2
  try { p = drawing&&pane==='3d'?unprojectDirectPlane(position(e),activePlane.value,camera.value):plane(position(e), pane) } catch (e) { error.value = String(e); return }
  if (drawing && !pan && tool.value !== 'select') p = snapped(p,e,draft.value.at(-1),pane)
  if(pane==='2d'&&tool.value==='trim'){if(id)trimAt(id,p);return}
  if (pane === '2d' && cornerActive.value) { if (id === selection.value && vertex !== null) cornerVertex.value = vertex; return }
  if (pane === '2d' && operation.value) operation.value = null
  if (pane === '2d' && vertex !== null) cornerVertex.value = vertex
  if (drawing && tool.value === 'polyline') { if (draft.value.length >= 3 && Math.hypot(p[0]-draft.value[0][0],p[1]-draft.value[0][1]) < views.value['2d']/100) { finish(true); return } draft.value = [...draft.value, p]; return }
  if (tool.value === 'select' || (pane === '3d'&&!drawing)) { if(!selectedIds.value.includes(id))pickObject(id,pane); if (!id) return }
  gesture = { inverse:svg.getScreenCTM()?.inverse()??null,start: p, document: history.document, vertex, id, sketch:drawing&&tool.value!=='select', pointer: e.pointerId, pane, svg, pan: false, center: [...centers.value[pane]] }
  if (pane === '2d' && id && vertex === null) {
    const sketch = visibleSketches.value.find(s=>s.id===id)
    if(sketch?.points.length) gesture.anchor = [...sketch.points.reduce((best,q)=>Math.hypot(q[0]-p[0],q[1]-p[1])<Math.hypot(best[0]-p[0],best[1]-p[1])?q:best)]
  }
  if(pane==='3d'&&id&&!drawing){const body=gesture.document.bodies.find(b=>b.id===id);if(body){const mouse=[e.clientX,e.clientY],toScreen=snapProjection(svg,'3d');gesture.anchor3=(bodyPoints(body) as Vec3[]).reduce((best,q)=>{const a=toScreen(best),b=toScreen(q);return Math.hypot(b[0]-mouse[0],b[1]-mouse[1])<Math.hypot(a[0]-mouse[0],a[1]-mouse[1])?q:best})}}
  svg.setPointerCapture(e.pointerId)
}
function pointDragPosition(e:PointerEvent,inverse:DOMMatrix|null):Point2{
 if(!inverse)return position(e)
 const p=new DOMPoint(e.clientX,e.clientY).matrixTransform(inverse);return [p.x,p.y]
}
function move(e: PointerEvent) {
  if(!requireLocalGestureSnaps(e)){cancelGesture();return}
  const snapSource=gestureSnapSource();if(snapSource&&!requireBodySnaps(e,snapSource)){cancelGesture();return}
  if(vertexDrag&&vertexDrag.pointer===e.pointerId){const g=vertexDrag,p=pointDragPosition(e,vertexDrag.inverse),sx=p[0]-g.start[0],sy=p[1]-g.start[1]
    if(!g.moved&&Math.hypot(sx,sy)<.05)return
    g.moved=true
    const cy=Math.cos(camera.value.yaw),sn=Math.sin(camera.value.yaw),sp=Math.sin(camera.value.pitch),cp=Math.cos(camera.value.pitch),horizontal=sy*sp,delta=[sx*cy+horizontal*sn,-sx*sn+horizontal*cy,-sy*cp]
    const body=g.before.bodies.find(b=>b.id===selection.value);if(!body)return
    const anchor=Array.from(body.mesh.positions.slice(g.ids[0]*3,g.ids[0]*3+3)) as Vec3
    const target=snapped3(anchor.map((v,i)=>v+delta[i]) as Vec3,e,g.before,[selection.value],anchor)
    requestDragJob({kind:'pointEdit',document:g.before,options:{kind:'vertices',id:body.id,indices:g.ids,delta:target.map((v,k)=>v-anchor[k]) as Vec3}});return}
  if(cvDrag&&cvDrag.pointer===e.pointerId){const g=cvDrag,p=pointDragPosition(e,cvDrag.inverse),sx=p[0]-g.start[0],sy=p[1]-g.start[1],cy=Math.cos(camera.value.yaw),sn=Math.sin(camera.value.yaw),sp=Math.sin(camera.value.pitch),cp=Math.cos(camera.value.pitch)
    const horizontal=sy*sp,delta=[sx*cy+horizontal*sn,-sx*sn+horizontal*cy,-sy*cp]
    const target=snapped3(g.point.map((value,i)=>value+delta[i]) as Vec3,e,g.before,[g.id],g.point as Vec3)
    requestDragJob({kind:'pointEdit',document:g.before,options:{kind:'cv',id:g.id,u:g.u,v:g.v,point:target}});return
  }
  if(curveDrag){const g=curveDrag,p=snapped(plane(pointDragPosition(e,g.inverse),'2d'),e)
    requestDragJob({kind:'pointEdit',document:g.before,options:{kind:'analytic-handle',id:g.id,handle:g.kind,point:p}});return
  }
  if(selectionBox.value){selectionBox.value.end=position(e);return}
  if(manipulatorDrag){const g=manipulatorDrag,f=views.value['3d']/Math.min(g.svg.clientWidth,g.svg.clientHeight),x=(e.clientX-g.x)*f,y=(e.clientY-g.y)*f,l=g.direction[0]**2+g.direction[1]**2
    if(!x&&!y&&!gizmoBase)return
    let distance=l>.001?(x*g.direction[0]+y*g.direction[1])/l:-y
    if(g.kind!=='move')distance=snapDistance(distance,e)
    if(g.kind==='move'&&gizmoCenter.value){const anchor=gizmoCenter.value,axis=['x','y','z'].indexOf(g.axis),target=anchor.map((v,i)=>v+(i===axis?distance:0)) as Vec3;distance=snapped3(target,e,g.before,selectedIds.value,anchor,axis)[axis]-anchor[axis]}
    if(g.kind==='push'||g.kind==='split'){advanced.value.distance=Math.round((distance+(g.kind==='split'?g.initial:0))*100)/100;return}
    const p=position(e),rotation=(Math.atan2(p[1]-g.center[1],p[0]-g.center[0])-Math.atan2(g.startPoint[1]-g.center[1],g.startPoint[0]-g.center[0]))*180/Math.PI*(projectDirectPoint(axisVector(g.axis),camera.value)[2]>=0?-1:1)
    if(g.kind==='move'){
      // Pure translation along an axis previews as a constant screen offset, so it skips
      // both the kernel round trip and the reactive re-render of every polygon.
      const worldDelta=axisVector(g.axis).map(v=>v*distance) as Vec3
      g.delta=worldDelta
      if(!dragNodes.length)dragNodes=collectDragNodes(g.svg,translationPreviewIds(selectedIds.value))
      const origin=project([0,0,0],'3d'),shifted=project(worldDelta,'3d')
      const offset=`translate(${shifted[0]-origin[0]},${shifted[1]-origin[1]})`
      for(const node of dragNodes)node.setAttribute('transform',offset)
      dragOffsetIds=translationPreviewIds(selectedIds.value)
      gpuLayer?.setDragOffset(dragOffsetIds,worldDelta as [number,number,number])
      return
    }
    requestGizmoTransform(g.before,g.axis,g.kind==='rotate'?rotation:0,g.kind==='scale'?Math.max(.01,1+distance/(views.value['3d']/7)):1);return
  }
  if (heightDrag && heightDrag.pointer === e.pointerId) {
    const n=cross3((selectedSketch.value?.plane??xyPlane()).u,(selectedSketch.value?.plane??xyPlane()).v), projected=projectDirectPoint(n,camera.value)
    const factor = -views.value['3d'] / Math.min(heightDrag.svg.clientWidth,heightDrag.svg.clientHeight) / (Math.abs(projected[1])<.05 ? -.05 : projected[1])
    const h = snapDistance(heightDrag.height - (e.clientY-heightDrag.y)*factor,e)
    height.value = Math.abs(h)<.01 ? .01 : Math.round(h*100)/100; return
  }
  if (orbitDrag && orbitDrag.pointer === e.pointerId) {
    camera.value = { yaw: orbitDrag.yaw + (e.clientX-orbitDrag.x)*.007, pitch: Math.max(-1.5,Math.min(1.5,orbitDrag.pitch+(e.clientY-orbitDrag.y)*.007)) }; return
  }
  if (!gesture || gesture.pointer !== e.pointerId) {
    const in3d=canvasOf(e).getAttribute?.('aria-label')===label('Холст тел 3D','3D body canvas')
    if(tool.value!=='select'&&(!in3d||faceDrawing.value)){
      if(!requireSketchSnaps(e)){snapMarker.value=null;draftCursor.value=null;return}
      try{const pane:Pane=in3d?'3d':'2d',p=in3d?unprojectDirectPlane(position(e),activePlane.value,camera.value):plane(position(e),'2d');draftCursor.value=snapped(p,e,draft.value.at(-1),pane)}catch{snapMarker.value=null}
    }
    return
  }
  if (gesture.pan) {
    const p = position(e), center = centers.value[gesture.pane]
    centers.value[gesture.pane] = [center[0] + gesture.start[0] - p[0], center[1] + gesture.start[1] - p[1]]
    return
  }
  // Selection and small pointer jitter must not snap or translate an exact body.
  if (gesture.dragStart && !gesture.bodyDrag && Math.hypot(e.clientX-gesture.dragStart[0],e.clientY-gesture.dragStart[1]) < 3) return
  let p = gesture.sketch&&gesture.pane==='3d'?unprojectDirectPlane(pointDragPosition(e,gesture.inverse??null),activePlane.value,camera.value):plane(pointDragPosition(e,gesture.inverse??null), gesture.pane); const start = gesture.start
  if (gesture.pane === '2d'||gesture.sketch) {
    const anchor=gesture.anchor
    if(anchor){const target=snapped([anchor[0]+p[0]-start[0],anchor[1]+p[1]-start[1]],e,anchor);p=[start[0]+target[0]-anchor[0],start[1]+target[1]-anchor[1]]}
    else p = snapped(p,e,start,gesture.pane)
  } else { const anchor=gesture.anchor3??[start[0],start[1],0],target=snapped3([anchor[0]+p[0]-start[0],anchor[1]+p[1]-start[1],anchor[2]],e,gesture.document,selectedIds.value,anchor,undefined,true);p=[start[0]+target[0]-anchor[0],start[1]+target[1]-anchor[1]] }
  if (gesture.sketch) {
    try { draft.value = tool.value === 'slot' ? (slotWidthValid.value && Math.hypot(p[0]-start[0],p[1]-start[1])>1e-6 ? slotSketch(start,p,slotWidth.value) : []) : tool.value === 'rectangle' ? [start, [p[0], start[1]], p, [start[0], p[1]]] : Array.from({ length: tool.value==='arc'?33:64 }, (_, i) => { const r = Math.hypot(p[0] - start[0], p[1] - start[1]), a = i * Math.PI / 32; return [start[0] + r * Math.cos(a), start[1] + r * Math.sin(a)] as Point2 })
    } catch(cause) { draft.value=[];error.value=String(cause);return }
    drawMeasure.value = tool.value === 'slot' ? label('Паз: ширина ','Slot width: ')+slotWidth.value+' mm' : tool.value === 'circle' ? `R ${Math.hypot(p[0]-start[0],p[1]-start[1]).toFixed(2)} mm` : `${Math.abs(p[0]-start[0]).toFixed(2)} × ${Math.abs(p[1]-start[1]).toFixed(2)} mm`
    return
  }
  const delta = [p[0] - start[0], p[1] - start[1], 0]
  // Body translation previews in JS. Routing it through the kernel would send the whole
  // document, B-rep included, across the WASM boundary on every pointer move.
  const draggingBodies = gesture.document.bodies.some(b => b.id === gesture!.id)
  if (draggingBodies || (selectedIds.value.length > 1 && !gesture.document.sketches.some(s => selectedIds.value.includes(s.id)))) {
    const ids = selectedIds.value.length > 1 ? [...selectedIds.value] : [gesture.id]
    const plane = gesture.pane === '2d' ? activePlane.value : xyPlane()
    const worldDelta = (selectedIds.value.length > 1
      ? worldPoint(delta, { ...plane, origin: [0, 0, 0] })
      : delta) as Vec3
    gesture.bodyDrag = { ids, delta: worldDelta }
    // The DOM-less test renderer has no query API; the drag still commits on release.
    if (!dragNodes.length) dragNodes = collectDragNodes(gesture.svg, translationPreviewIds(ids))
    const origin = project([0, 0, 0], '3d'), shifted = project(worldDelta as unknown as Vec3, '3d')
    const offset = `translate(${shifted[0] - origin[0]},${shifted[1] - origin[1]})`
    for (const node of dragNodes) node.setAttribute('transform', offset)
    // The visible surface comes from the GPU layer; move it in place, not by rebuilding.
    dragOffsetIds = translationPreviewIds(ids)
    gpuLayer?.setDragOffset(dragOffsetIds, worldDelta as [number, number, number])
    return
  }
  gesture.workerEdit=true
  if(selectedIds.value.length>1){
    const plane=gesture.pane==='2d'?activePlane.value:xyPlane(),worldDelta=worldPoint(delta,{...plane,origin:[0,0,0]})
    requestDragJob({kind:'sceneEdit',document:gesture.document,options:{operation:'transform',id:gesture.id,ids:[...selectedIds.value],createdId:'',x:worldDelta[0],y:worldDelta[1],z:worldDelta[2],axis:'z',angle:0,scale:1}})
  }else if(gesture.vertex!==null){
    requestDragJob({kind:'pointEdit',document:gesture.document,options:{kind:'sketch-vertex',id:gesture.id,index:gesture.vertex,point:p}})
  }else{
    requestDragJob({kind:'sceneEdit',document:gesture.document,options:{operation:'sketch-transform',id:gesture.id,ids:[gesture.id],createdId:'',x:delta[0],y:delta[1],z:0,axis:'z',angle:0,scale:1}})
  }

}
function up(e: PointerEvent) {
  if(!requireLocalGestureSnaps(e)){cancelGesture();return}
  const snapSource=gestureSnapSource();if(snapSource&&!requireBodySnaps(e,snapSource)){cancelGesture();return}
  dragConstraint.value=''
  void nextTick(()=>{snapMarker.value=null;snapGuide.value=null})
  cameraDragging.value = false
  if(vertexDrag&&vertexDrag.pointer===e.pointerId){const g=vertexDrag;move(e);vertexDrag=null;if(g.moved)gizmoApplyRevision=gizmoRevision;return}
  if(cvDrag&&cvDrag.pointer===e.pointerId){move(e);cvDrag=null;gizmoApplyRevision=gizmoRevision;return}
  if(curveDrag){move(e);curveDrag=null;gizmoApplyRevision=gizmoRevision;return}
  if(selectionBox.value){const box=selectionBox.value,min=[Math.min(box.start[0],box.end[0]),Math.min(box.start[1],box.end[1])],max=[Math.max(box.start[0],box.end[0]),Math.max(box.start[1],box.end[1])]
    const items=box.pane==='2d'?visibleSketches.value.map(s=>({id:s.id,points:s.points.map(p=>project(p,'2d'))})):sceneBodies.value.map(b=>({id:b.id,points:bodyPoints(b).map(p=>project(p,'3d'))}))
    const ids=items.filter(o=>objectSelectable(o.id)&&o.points.every(p=>p[0]>=min[0]&&p[0]<=max[0]&&p[1]>=min[1]&&p[1]<=max[1])).map(o=>o.id);selection.value=ids[0]??'';extraSelection.value=ids.slice(1);selectionBox.value=null;return
  }
  if(manipulatorDrag){const g=manipulatorDrag;if(e.clientX===g.x&&e.clientY===g.y&&!gizmoBase){cancelGesture();return}move(e);manipulatorDrag=null
    if(g.kind==='push'){clearDragPreview();if(bodyEditPending.value)bodyEditApplyGeneration=bodyEditGeneration;else applyCommand();return}
    if(g.kind==='move'){if(g.delta)void commitDirectTransform(g.before,selectedIds.value,g.delta);return}
    if(g.kind!=='split')gizmoApplyRevision=gizmoRevision;return}

  if (heightDrag) { heightDrag = null; return }
  if (orbitDrag) { orbitDrag = null; return }
  drawMeasure.value = ''; snapMarker.value = null
  if (!gesture || gesture.pointer !== e.pointerId) return
  move(e)
  const start=gesture.start, before = gesture.document, pane = gesture.pane, drawing=gesture.sketch, pan = gesture.pan, bodyDrag = gesture.bodyDrag, selectionOnly = gesture.dragStart && !bodyDrag, workerEdit=gesture.workerEdit; gesture = null
  if (pan || selectionOnly) return
  if(workerEdit){gizmoApplyRevision=gizmoRevision;return}
  run(() => {
    // The exact translation, including B-rep, is applied once here rather than per move.
    if (bodyDrag) { void commitDirectTransform(before,bodyDrag.ids,bodyDrag.delta); return }
    if (drawing) {
      const points = draft.value
      if((tool.value==='circle'||tool.value==='arc')&&points.length){const radius=Math.hypot(points[0][0]-start[0],points[0][1]-start[1]);if(radius>=.01){const analytic={kind:tool.value,center:start,radius,start:0,sweep:tool.value==='circle'?360:180} as const;addSketch(sampleCurve(analytic),tool.value==='circle',analytic)}}
      else if (points.length >= 3 && Math.abs(points.reduce((sum, p, i) => { const q = points[(i + 1) % points.length]; return sum + p[0] * q[1] - q[0] * p[1] }, 0)) > 1e-6) addSketch(points, true)
      draft.value = []; draftCursor.value=null; tool.value = 'select'
    } else commit(document.value)
  })
  if (error.value && stringifyMeshJson(history.document) === stringifyMeshJson(before)) document.value = before
}

// Command palette: every tool, primitive and context action of the workspace, searchable by its Russian or English name.
const paletteOpen = ref(false)
const notice = ref('')
const dockOpen = ref(storageGet('scad-solid-dock') !== 'false')
const dockTab = ref<'scene' | 'props'>('scene')
watch(dockOpen, open => storageSet('scad-solid-dock', String(open)))
const exactCardOpen = computed({ get: () => dockOpen.value && dockTab.value === 'props', set: open => { if (open) { dockOpen.value = true; dockTab.value = 'props' } else dockTab.value = 'scene' } })
watch(() => props.paletteRequest, request => { if (request && props.open) paletteOpen.value = true })
type SolidCommand = PaletteCommand & { run: () => void }
const availableSolidCommands = computed<SolidCommand[]>(() => {
  const sketch = selectedSketch.value, body = selectedBody.value, nurbs = selectedNurbs.value
  const anySelection = !!(sketch || body || nurbs)
  const needSelection = label('Сначала выберите объект', 'Select an object first')
  const needClosed = label('Нужен замкнутый эскиз', 'A closed sketch is required')
  const needFace = label('Выберите грань тела', 'Select a body face')
  const needEdge = label('Выберите ребро тела', 'Select a body edge')
  const twoBodies = twoSelectedBodies.value
  const anyBody = document.value.bodies.length > 0
  const twoBrep = selectedBrepBodies.value.length === 2 && selectedIds.value.length === 2
  const booleanDetail = twoBrep ? label('Точный B-rep', 'Exact B-rep') : label('По сетке', 'Mesh boolean')
  const needTwo = label('Выберите два тела: Shift + клик', 'Select two bodies: Shift + click')
  const cmd = (id: string, ru: string, en: string, run: () => void, extra: Partial<PaletteCommand> = {}): SolidCommand =>
    ({ id, label: label(ru, en), aliases: [ru, en], run, ...extra })
  const toolCmd = (value: typeof tool.value, ru: string, en: string, shortcut?: string) =>
    cmd(`tool-${value}`, ru, en, () => beginSketch(value), { detail: label('Инструмент 2D', '2D tool'), shortcut })
  const list: SolidCommand[] = [
    cmd('focus-selection','Фокус на выбранном','Focus selection',()=>fit(mode.value,true),{enabled:anySelection,disabledReason:needSelection,shortcut:'F'}),
    cmd('edge-chain','Продолжить цепочку рёбер','Extend edge chain',selectEdgeChain,{enabled:!!body&&edgeIndex.value>=0,disabledReason:needEdge,detail:label('До разветвления','Stops at junctions')}),
    toolCmd('select', 'Выбор', 'Select', 'V'), toolCmd('rectangle', 'Прямоугольник', 'Rectangle', 'R'), toolCmd('circle', 'Круг', 'Circle', 'C'),
    toolCmd('slot', 'Паз', 'Slot'), toolCmd('arc', 'Дуга', 'Arc'), toolCmd('trim', 'Обрезать', 'Trim'), toolCmd('polyline', 'Ломаная', 'Polyline', 'L'),
    cmd('box-select', 'Рамка', 'Box select', () => { boxSelect.value = !boxSelect.value }, { detail: label('Выделение', 'Selection') }),
    ...primitiveKinds.map(kind => cmd(`add-${kind}`, primitiveLabel(kind), primitiveLabel(kind), () => addPrimitive(kind), { detail: label('Добавить примитив', 'Add primitive'), keywords: ['primitive', 'примитив', kind], enabled: kernelReady.value })),
    cmd('add-curve', 'NURBS-кривая', 'NURBS curve', () => addNurbs('curve'), { detail: label('Добавить', 'Add') }),
    cmd('nurbs-surface-reduce','Снизить степень поверхности','Reduce surface degree',()=>{beginAdvanced('nurbs-surface-reduce');advanced.value.surfaceAxis=(selectedNurbsSurface.value?.surface.degreeU??1)>1?'u':'v';mode.value='3d'},{enabled:!!selectedNurbsSurface.value&&(selectedNurbsSurface.value.surface.degreeU>1||selectedNurbsSurface.value.surface.degreeV>1),disabledReason:label('Выберите NURBS-поверхность степени выше 1','Select a NURBS surface with degree above 1')}),
    cmd('nurbs-prepare','Подготовить границы поверхностей','Prepare surface boundaries',()=>beginAdvanced('nurbs-prepare'),{enabled:!!selectedSurfacePair.value,disabledReason:label('Выберите две NURBS-поверхности','Select two NURBS surfaces')}),
    cmd('profile-union','Объединить точные профили','Union exact profiles',()=>beginAdvanced('profile-union'),{enabled:selectedIds.value.length>=2&&selectedIds.value.every(id=>document.value.sketches.some(s=>s.id===id&&s.closed)),disabledReason:label('Выберите два замкнутых профиля','Select two closed profiles')}),
    cmd('profile-difference','Вычесть области профилей','Subtract profile regions',()=>beginAdvanced('profile-difference'),{enabled:selectedIds.value.length>=2&&selectedIds.value.every(id=>document.value.sketches.some(s=>s.id===id&&s.closed)),disabledReason:label('Выберите два замкнутых профиля','Select two closed profiles')}),
    cmd('profile-intersection','Пересечь точные профили','Intersect exact profiles',()=>beginAdvanced('profile-intersection'),{enabled:selectedIds.value.length>=2&&selectedIds.value.every(id=>document.value.sketches.some(s=>s.id===id&&s.closed)),disabledReason:label('Выберите два замкнутых профиля','Select two closed profiles')}),
    cmd('profile-prepare','Собрать профиль','Prepare profile',()=>beginAdvanced('profile-prepare'),{enabled:!!selectedIds.value.length&&selectedIds.value.every(id=>document.value.sketches.some(s=>s.id===id&&!s.closed&&!s.retainedProfile&&s.analytic?.kind!=='circle')),disabledReason:label('Выберите открытые ломаные и дуги в одной плоскости','Select open polylines and arcs in one plane')}),
    cmd('nurbs-offset','Смещение NURBS-кривой','Offset NURBS curve',()=>beginAdvanced('nurbs-offset'),{enabled:!!selectedNurbsCurve.value,disabledReason:label('Выберите NURBS-кривую','Select a NURBS curve')}),
    cmd('nurbs-point-trim','Обрезать NURBS по точке','Trim NURBS at point',()=>beginAdvanced('nurbs-point-trim'),{enabled:!!selectedNurbsCurve.value,disabledReason:label('Выберите NURBS-кривую','Select a NURBS curve')}),
    cmd('nurbs-curve-match','Согласовать кривые G1','Match curves G1',()=>beginAdvanced('nurbs-curve-match'),{enabled:!!selectedCurvePair.value,disabledReason:label('Выберите две NURBS-кривые','Select two NURBS curves')}),
    cmd('nurbs-match','Согласовать поверхности G1/G2','Match surfaces G1/G2',()=>beginAdvanced('nurbs-match'),{enabled:!!selectedSurfacePair.value,disabledReason:label('Выберите две NURBS-поверхности','Select two NURBS surfaces')}),
    cmd('nurbs-surface-rebuild','Перестроить поверхность','Rebuild surface',()=>{beginAdvanced('nurbs-surface-rebuild');mode.value='3d'},{enabled:!!selectedNurbsSurface.value,disabledReason:label('Выберите NURBS-поверхность','Select a NURBS surface')}),
    cmd('nurbs-rebuild','Перестроить кривую','Rebuild curve',()=>{beginAdvanced('nurbs-rebuild');mode.value='3d'},{enabled:!!selectedNurbsCurve.value&&!selectedNurbsCurve.value.bridge,disabledReason:label('Выберите независимую NURBS-кривую','Select an independent NURBS curve')}),
    cmd('nurbs-reduce','Снизить степень кривой','Reduce curve degree',()=>{beginAdvanced('nurbs-reduce');mode.value='3d'},{enabled:!!selectedNurbsCurve.value&&selectedNurbsCurve.value.curve.degree>1&&!selectedNurbsCurve.value.bridge,disabledReason:label('Выберите независимую NURBS-кривую степени выше 1','Select an independent NURBS curve with degree above 1')}),
    cmd('nurbs-patch','Coons patch','Coons patch',()=>beginAdvanced('nurbs-patch'),{enabled:selectedIds.value.length===4&&selectedIds.value.every(id=>document.value.curves?.some(c=>c.id===id)),disabledReason:label('Выберите четыре граничные NURBS-кривые','Select four boundary NURBS curves')}),
    cmd('nurbs-sweep','Перенос профиля по пути','Sweep',()=>beginAdvanced('nurbs-sweep'),{enabled:!!selectedCurvePair.value,disabledReason:label('Выберите две NURBS-кривые: профиль, затем путь','Select two NURBS curves: profile, then path')}),
    cmd('nurbs-loft','Поверхность по сечениям','NURBS loft',()=>beginAdvanced('nurbs-loft'),{enabled:selectedIds.value.length>=2&&selectedIds.value.every(id=>document.value.curves?.some(c=>c.id===id)),disabledReason:label('Выберите NURBS-сечения по порядку','Select NURBS sections in order')}),
    cmd('add-surface', 'NURBS-поверхность', 'NURBS surface', () => addNurbs('surface'), { detail: label('Добавить', 'Add') }),
    cmd('pick-body', 'Выбирать тела', 'Pick bodies', () => { pickMode.value = 'body'; advancedOp.value = null; boxSelect.value = false }, { detail: label('Режим выбора 3D', '3D pick mode') }),
    cmd('pick-face', 'Выбирать грани', 'Pick faces', () => { pickMode.value = 'face'; advancedOp.value = null; boxSelect.value = false }, { detail: label('Режим выбора 3D', '3D pick mode') }),
    cmd('pick-edge', 'Выбирать рёбра', 'Pick edges', () => { pickMode.value = 'edge'; advancedOp.value = null; boxSelect.value = false }, { detail: label('Режим выбора 3D', '3D pick mode') }),
    cmd('pick-vertex', 'Выбирать вершины', 'Pick vertices', () => { pickMode.value = 'vertex'; advancedOp.value = null; boxSelect.value = false }, { detail: label('Режим выбора 3D', '3D pick mode') }),
    cmd('gizmo-move', 'Манипулятор: двигать', 'Gizmo: move', () => { gizmoMode.value = 'move' }),
    cmd('gizmo-rotate', 'Манипулятор: вращать', 'Gizmo: rotate', () => { gizmoMode.value = 'rotate' }),
    cmd('gizmo-scale', 'Манипулятор: масштаб', 'Gizmo: scale', () => { gizmoMode.value = 'scale' }),
    cmd('orbit', 'Обзор камерой', 'Orbit camera', () => { movingBody.value = false }, { detail: label('Вид', 'View') }),
    cmd('move-body', 'Двигать тело', 'Move body', () => { movingBody.value = true }, { detail: label('Вид', 'View'), shortcut: 'G' }),
    cmd('sketch-pane', sketchPaneOpen.value ? 'Скрыть панель эскизов 2D' : 'Показать панель эскизов 2D', sketchPaneOpen.value ? 'Hide 2D sketch pane' : 'Show 2D sketch pane', () => toggleSketchPane(), { detail: label('Вид', 'View') }),
    cmd('fit', 'Вписать', 'Fit', () => fit(mode.value), { detail: label('Вид', 'View'), shortcut: 'F' }),
    cmd('iso', 'Изометрия', 'Isometric view', () => { camera.value = defaultDirectCamera(); fit('3d') }, { detail: label('Вид', 'View') }),
    cmd('smooth', smoothDisplay.value ? 'Показывать B-rep гранёным' : 'Показывать B-rep гладким', smoothDisplay.value ? 'Show B-rep faceted' : 'Show B-rep smooth', () => { smoothDisplay.value = !smoothDisplay.value }, { detail: label('Вид', 'View') }),
    cmd('grid', floorVisible.value ? 'Скрыть сетку 3D' : 'Показать сетку 3D', floorVisible.value ? 'Hide 3D grid' : 'Show 3D grid', () => { floorVisible.value = !floorVisible.value }, { detail: label('Вид', 'View') }),
    cmd('undo', 'Отменить', 'Undo', () => undo(), { shortcut: 'Ctrl Z', enabled: undoable.value, disabledReason: label('Нечего отменять', 'Nothing to undo') }),
    cmd('redo', 'Повторить', 'Redo', () => undo(true), { shortcut: 'Ctrl Shift Z', enabled: redoable.value, disabledReason: label('Нечего повторять', 'Nothing to redo') }),
    cmd('brep-union', 'Объединить тела', 'Union bodies', () => applyBrepBoolean('union'), { detail: booleanDetail, aliases: ['Объединить тела', 'Union bodies', 'B-rep объединить', 'B-rep Union'], keywords: ['boolean', 'булев', 'union', 'merge', 'слить'], enabled: twoBodies, disabledReason: needTwo }),
    cmd('brep-difference', 'Вычесть: A − B', 'Subtract: A − B', () => beginSubtract(), { detail: label('Панель: A − B, тела кликом, Shift добавляет', 'Panel: A − B, click bodies, Shift adds'), aliases: ['Вычесть: A − B', 'Subtract: A − B', 'B-rep A − B'], keywords: ['boolean', 'булев', 'union', 'merge', 'слить'], enabled: anyBody, disabledReason: needSelection }),
    cmd('brep-intersection', 'Пересечение тел', 'Intersect bodies', () => applyBrepBoolean('intersection'), { detail: booleanDetail, aliases: ['Пересечение тел', 'Intersect bodies', 'B-rep пересечение', 'B-rep Intersection'], keywords: ['boolean', 'булев', 'union', 'merge', 'слить'], enabled: twoBodies, disabledReason: needTwo }),
    cmd('brep-xor', 'B-rep XOR', 'B-rep XOR', () => applyBrepBoolean('xor'), { detail: label('Два точных тела B-rep', 'Two exact B-rep bodies'), enabled: twoBrep, disabledReason: label('Нужны два точных тела B-rep', 'Two exact B-rep bodies are required') }),
    cmd('push', 'Push / Pull', 'Push / Pull', () => beginAdvanced('push'), { detail: label('Грань', 'Face'), enabled: !!(body && selectedFace.value), disabledReason: needFace }),
    cmd('face-sketch', 'Эскиз на грани', 'Sketch on face', faceSketch, { detail: label('Грань', 'Face'), enabled: !!(body && selectedFace.value), disabledReason: needFace }),
    cmd('shell', 'Shell', 'Shell', () => beginAdvanced('shell'), { detail: label('Грань', 'Face'), enabled: !!(body && selectedFace.value), disabledReason: needFace }),
    cmd('chamfer', 'Фаска 3D', 'Chamfer 3D', () => beginAdvanced('chamfer'), { detail: label('Ребро', 'Edge'), enabled: !!body && edgeIndex.value >= 0, disabledReason: needEdge }),
    cmd('edge-fillet', 'Скруглить 3D', 'Fillet 3D', () => beginAdvanced('edge-fillet'), { detail: label('Ребро', 'Edge'), enabled: !!body && edgeIndex.value >= 0, disabledReason: needEdge }),
    cmd('split', 'Разрезать', 'Split', () => beginAdvanced('split'), { detail: label('Тело', 'Body'), enabled: !!body, disabledReason: label('Выберите тело', 'Select a body') }),
    cmd('brep-properties', 'Свойства B-rep', 'B-rep properties', measureSelectedBrep, { detail: label('Тело', 'Body'), enabled: !!body?.brep, disabledReason: label('Выберите тело B-rep', 'Select a B-rep body') }),
    cmd('retessellate', 'Перестроить mesh', 'Retessellate', retessellateSelectedBrep, { detail: label('Тело', 'Body'), enabled: !!body?.brep, disabledReason: label('Выберите тело B-rep', 'Select a B-rep body') }),
    cmd('curve', 'Параметры кривой', 'Curve parameters', () => beginAdvanced('curve'), { detail: label('Эскиз', 'Sketch'), enabled: !!sketch?.analytic, disabledReason: label('Выберите круг или дугу', 'Select a circle or arc') }),
    cmd('offset', 'Offset', 'Offset', () => beginAdvanced('offset'), { detail: label('Эскиз', 'Sketch'), enabled: !!(sketch && (sketch.closed || sketch.analytic)), disabledReason: needClosed }),
    cmd('extend', 'Продлить', 'Extend', () => beginAdvanced('extend'), { detail: label('Эскиз', 'Sketch'), enabled: !!(sketch && !sketch.closed && !sketch.analytic), disabledReason: label('Выберите незамкнутую линию', 'Select an open line') }),
    cmd('instance-create','Создать связанный экземпляр','Create linked instance',()=>beginAdvanced('instance-create'),{enabled:!!body&&!body.instance,disabledReason:label('Выберите независимое тело-источник','Select an independent source body')}),
    cmd('instance-transform','Преобразовать экземпляр','Transform instance',()=>beginAdvanced('instance-transform'),{enabled:!!body?.instance,disabledReason:label('Выберите связанный экземпляр','Select a linked instance')}),
    cmd('instance-place','Разместить экземпляр','Place instance',()=>beginAdvanced('instance-place'),{enabled:!!body?.instance,disabledReason:label('Выберите связанный экземпляр','Select a linked instance')}),
    cmd('instance-detach','Сделать независимым','Make independent',()=>run(()=>commit(detachSolidInstances(history.document,selectedIds.value))),{enabled:!!body?.instance,disabledReason:label('Выберите связанный экземпляр','Select a linked instance')}),
    cmd('instance-source','Выбрать источник экземпляра','Select instance source',()=>{if(body?.instance)pickObject(body.instance.sourceId,'3d')},{enabled:!!body?.instance,disabledReason:label('Выберите связанный экземпляр','Select a linked instance')}),
    cmd('body-clearance','Зазор двух тел по сетке','Two-body mesh clearance',()=>{measureTarget.value=selectedIds.value.find(id=>id!==selection.value)??'';exactCardOpen.value=true;measurementOpen.value=true;clearanceOpen.value=true;workspace.value?.focus()},{enabled:!!body&&selectedIds.value.length===2&&selectedIds.value.every(id=>document.value.bodies.some(b=>b.id===id)),disabledReason:label('Выберите два тела с Shift','Shift-select two bodies')}),
    cmd('measurements','Измерить вершины / ребро','Measure vertices / edge',()=>{exactCardOpen.value=true;measurementOpen.value=true;workspace.value?.focus()},{enabled:!!body,disabledReason:needSelection}),
    cmd('diagnostics','Диагностика тела','Body diagnostics',()=>{exactCardOpen.value=true;diagnosticsOpen.value=true;void nextTick(()=>diagnosticPanel.value?.focus())},{enabled:!!body,disabledReason:needSelection}),
    cmd('transform', 'Преобразовать выбор', 'Transform selection', () => beginAdvanced('transform'), { enabled: anySelection && !nurbs, disabledReason: needSelection }),
    cmd('dimensions', 'Размеры эскиза', 'Sketch dimensions', () => { exactCardOpen.value = true }, { enabled: !!sketch, disabledReason: label('Выберите эскиз', 'Select a sketch') }),
    cmd('exact-transform', 'Точные преобразования', 'Exact transforms', () => { exactCardOpen.value = true }, { detail: label('Числовой ввод', 'Numeric input'), enabled: !!(sketch || body), disabledReason: needSelection }),
    cmd('brep-detail', 'Детализация B-rep', 'B-rep detail', () => { exactCardOpen.value = true }, { detail: label('Числовой ввод', 'Numeric input'), enabled: !!body?.brep, disabledReason: label('Выберите тело B-rep', 'Select a B-rep body') }),
    cmd('extrude', 'Выдавить', 'Extrude', () => beginExtrude(), { detail: label('Эскиз', 'Sketch'), shortcut: 'E', enabled: canExtrudeSketch.value, disabledReason: needClosed }),
    cmd('revolve', 'Вращение', 'Revolve', () => beginExtrude('revolve'), { detail: label('Эскиз', 'Sketch'), enabled: canExtrudeSketch.value, disabledReason: needClosed }),
    cmd('loft', 'B-rep loft', 'B-rep loft', () => beginAdvanced('loft'), { detail: label('Эскиз', 'Sketch'), enabled: !!sketch && selectedIds.value.length >= 2, disabledReason: label('Выберите два и более эскиза', 'Select two or more sketches') }),
    cmd('fillet', 'Скруглить', 'Fillet', () => beginCorner('fillet'), { detail: label('Эскиз', 'Sketch'), enabled: canExtrudeSketch.value, disabledReason: needClosed }),
    cmd('dogear', 'DogEar', 'DogEar', () => beginCorner('dogear'), { detail: label('Эскиз', 'Sketch'), enabled: canExtrudeSketch.value, disabledReason: needClosed }),
    cmd('array', 'Круговые копии', 'Circular copies', () => { const active = operation.value === 'array'; cancelCommand(); if (!active) operation.value = 'array' }, { detail: label('Эскиз', 'Sketch'), enabled: !!sketch, disabledReason: label('Выберите эскиз', 'Select a sketch') }),
    cmd('duplicate', 'Копия', 'Duplicate', duplicate, { shortcut: 'Ctrl D', enabled: anySelection && !nurbs, disabledReason: needSelection }),
    cmd('delete', 'Удалить', 'Delete', remove, { shortcut: 'Del', enabled: anySelection, disabledReason: needSelection }),
    cmd('bake', 'Bake в Code', 'Bake into Code', appendBodies, { detail: label('Файл', 'File'), enabled: document.value.bodies.length > 0 && props.canAppend, disabledReason: label('Нет тел для переноса', 'No bodies to bake') }),
    cmd('to-mesh', 'Открыть в Mesh', 'Open in Mesh', sendToMesh, { detail: label('Файл', 'File'), enabled: document.value.bodies.length > 0, disabledReason: label('Нет тел', 'No bodies') }),
    cmd('download-svg', 'SVG выбранного тела · проекция XY', 'Selected body SVG · XY projection', exportBodySvg, { detail: label('Файл', 'File'), enabled: !!selectedBody.value, disabledReason: label('Выберите тело', 'Select a body') }),
    cmd('download-blender', 'Экспорт для Blender', 'Export for Blender', exportBlender, { detail: label('Файл', 'File'), enabled: document.value.bodies.length > 0, disabledReason: label('Нет тел', 'No bodies') }),
    cmd('download-json', 'Скачать проект JSON', 'Download JSON project', downloadProject, { enabled:!restoringDraft.value, disabledReason:label('Дождитесь восстановления документа.','Wait for document recovery.'), detail: label('Файл', 'File') }),
    cmd('download-scad', 'Экспорт SCAD', 'Export SCAD', () => download(directBodiesScad(document.value), 'solid-bodies.scad'), { detail: label('Файл', 'File'), enabled: document.value.bodies.length > 0, disabledReason: label('Нет тел', 'No bodies') }),
    cmd('help', 'Горячие клавиши', 'Keyboard shortcuts', () => { showHelp.value = !showHelp.value }, { shortcut: '?' }),
  ]
  return list
})
function captureCommand() {
  const id = subtract.value ? 'brep-difference' : advancedOp.value ?? operation.value
  if (!id) return null
  return { id, surfaceInputs:[...surfaceInputs.value], inputSelection:[...selectedIds.value], surfaceReversed:[...surfaceReversed.value], advanced: { ...advanced.value,...(id==='nurbs-point-trim'&&pointTrimPick.value&&pointTrimCut.value?{x:pointTrimCut.value[0],y:pointTrimCut.value[1],z:pointTrimCut.value[2]??0}:{}) }, height: height.value, baseZ: baseZ.value,
    cornerRadius: cornerRadius.value, revolveAxis: revolveAxis.value, revolveOffset: revolveOffset.value,
    revolveAngle: revolveAngle.value, revolveSegments: revolveSegments.value, revolveGeometry: revolveGeometry.value,
    filletSegments: filletSegments.value, brepSegments: brepSegments.value, copyCount: copyCount.value, copySweep: copySweep.value,
    copyX: copyX.value, copyY: copyY.value, extrusionMode: extrusionMode.value }
}
const lastCommand = shallowRef<ReturnType<typeof captureCommand>>(null)
// A profile merge keeps its first identity. After undo, the other inputs exist
// again but the selection still contains only that first identity.
const repeatProfileInputs = computed(() => {
 const saved=lastCommand.value
 if(!saved||!isProfileCommand(saved.id)||selectedIds.value.length!==1||selectedIds.value[0]!==saved.surfaceInputs[0])return []
 return saved.surfaceInputs.every(id=>document.value.sketches.some(s=>s.id===id&&(profileBooleanOperation(saved.id)?s.closed:!s.closed&&!s.retainedProfile&&s.analytic?.kind!=='circle')))?saved.surfaceInputs:[]
})
const repeatEntry = computed(() => availableSolidCommands.value.find(c => c.id === lastCommand.value?.id))
const repeatReason = computed(() => {
  if (!lastCommand.value) return label('Сначала примените команду', 'Apply a command first')
  if (commandActive.value) return label('Завершите текущую команду', 'Finish the current command')
  if (!repeatEntry.value || (repeatEntry.value.enabled === false && !repeatProfileInputs.value.length)) return repeatEntry.value?.disabledReason ?? label('Нужен другой выбор', 'Choose compatible geometry')
  if (['extrude','revolve'].includes(lastCommand.value.id) && lastCommand.value.extrusionMode !== 'new' && !selectedSketch.value?.supportBodyId)
    return label('Выберите эскиз на целевом теле', 'Select a sketch on the target body')
  return ''
})
function repeatCommand() {
  if (repeatReason.value || !lastCommand.value || !repeatEntry.value) return
  const saved = lastCommand.value
  const restoredInputs=[...repeatProfileInputs.value]
  cancelCommand()
  if(restoredInputs.length){selection.value=restoredInputs[0];extraSelection.value=restoredInputs.slice(1)}
  repeatEntry.value.run()
  if((isProfileCommand(saved.id))&&saved.surfaceInputs.every(id=>document.value.sketches.some(s=>s.id===id))&&saved.inputSelection.every(id=>selectedIds.value.includes(id)))surfaceInputs.value=[...saved.surfaceInputs]
  if(saved.id==='nurbs-curve-match'&&saved.surfaceInputs.every(id=>document.value.curves?.some(c=>c.id===id))&&saved.inputSelection.every(id=>selectedIds.value.includes(id)))surfaceInputs.value=[...saved.surfaceInputs]
  if((saved.id==='nurbs-match'||saved.id==='nurbs-prepare')&&saved.surfaceInputs.every(id=>document.value.surfaces?.some(s=>s.id===id))&&saved.inputSelection.every(id=>selectedIds.value.includes(id)))surfaceInputs.value=[...saved.surfaceInputs]
  if(['nurbs-loft','nurbs-sweep','nurbs-patch'].includes(saved.id)) {
    const order=saved.surfaceInputs.map(id=>saved.inputSelection.indexOf(id))
    if(saved.inputSelection.length===selectedIds.value.length&&saved.inputSelection.every((id,i)=>id===selectedIds.value[i])&&saved.surfaceInputs.every(id=>document.value.curves?.some(c=>c.id===id)))surfaceInputs.value=[...saved.surfaceInputs]
    else if(order.length===surfaceInputs.value.length&&order.every(i=>i>=0&&i<surfaceInputs.value.length))surfaceInputs.value=order.map(i=>surfaceInputs.value[i])
  }
  surfaceReversed.value=surfaceInputs.value.map((_,i)=>saved.surfaceReversed[i]??false)
  advanced.value = { ...saved.advanced }; height.value = saved.height; baseZ.value = saved.baseZ
  cornerRadius.value = saved.cornerRadius; revolveAxis.value = saved.revolveAxis; revolveOffset.value = saved.revolveOffset
  revolveAngle.value = saved.revolveAngle; revolveSegments.value = saved.revolveSegments; revolveGeometry.value = saved.revolveGeometry
  filletSegments.value = saved.filletSegments; brepSegments.value = saved.brepSegments; copyCount.value = saved.copyCount; copySweep.value = saved.copySweep
  copyX.value = saved.copyX; copyY.value = saved.copyY; extrusionMode.value = saved.extrusionMode
}
const solidCommands = computed<SolidCommand[]>(() => [...availableSolidCommands.value, {
  id:'repeat', label:label('Повторить команду','Repeat command'), shortcut:'Shift R', run:repeatCommand,
  enabled:!repeatReason.value, disabledReason:repeatReason.value, detail:repeatEntry.value?.label,
}])

function executeSolidCommand(id: string) {
  paletteOpen.value = false
  const target = solidCommands.value.find(command => command.id === id)
  if (target && target.enabled !== false) run(target.run)
}
// Tests and the App shell drive the workspace through the same command list the palette shows.
defineExpose({ solidCommands, executeSolidCommand })

function keydown(e: KeyboardEvent) {
  if (restoringDraft.value) { if(e.key==='Escape'){e.preventDefault();cancelDraftRecovery()} return }
  if (paletteOpen.value) return
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); paletteOpen.value = true; return }
  if (e.isComposing) return
  if (e.key === 'Escape') { e.preventDefault(); const boundaryWasPending=boundaryAgreementPending.value||faceContactsPending.value; if(diagnosticsPending.value||intersectionPending.value||boundaryWasPending)diagnosticsOpen.value=false;exactCardOpen.value = false; cancelCommand(); if(boundaryWasPending)void nextTick(()=>workspace.value?.focus()); return }
  if (e.key === 'Enter' && commandActive.value) {
    // Native controls retain Enter, including Cancel, operand selection and the File menu.
    if ((e.target as HTMLElement).closest?.('button, summary, a[href], select')) return
    e.preventDefault(); applyCommand(); return
  }
  if ((e.target as HTMLElement).matches('input,textarea,select')) return
  if(tool.value==='polyline' && draft.value.length && (e.key==='Backspace'||((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='z'&&!e.shiftKey))){e.preventDefault();undoDraftPoint();return}
  if(tool.value==='polyline' && e.key==='Enter'){e.preventDefault();finish(false);return}
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd') { e.preventDefault(); duplicate(); return }
  if (e.shiftKey && !e.ctrlKey && !e.metaKey && !e.altKey && e.key.toLowerCase() === 'r') { e.preventDefault(); repeatCommand(); return }
  if (!e.ctrlKey && !e.metaKey && !e.altKey) {
    const k = e.key.toLowerCase(), tools = { v:'select', r:'rectangle', c:'circle', l:'polyline' } as const
    if (k in tools) beginSketch(tools[k as keyof typeof tools])
    if (k === 'e') beginExtrude()
    if (k === 'f') fit(mode.value,selectedIds.value.length>0)
    if (k === 'g') movingBody.value = !movingBody.value
    if (k === '?') showHelp.value = !showHelp.value
  }
  if ((e.ctrlKey || e.metaKey) && (e.key.toLowerCase() === 'z' || e.key.toLowerCase() === 'y')) { e.preventDefault(); e.stopPropagation(); undo(e.shiftKey || e.key.toLowerCase() === 'y') }
  if (e.key === 'Delete' || e.key === 'Backspace') { e.preventDefault(); remove() }
  if (e.key === 'Escape' && (gesture || draft.value.length)) { e.preventDefault(); e.stopPropagation(); cancelGesture() }
}

if (props.initialDocument) void nextTick(() => { fit('2d'); fit('3d') })
const primitiveKinds = ['box','wedge','cylinder','frustum','tube','cone','sphere','torus'] as const
const roundGeometry=ref<'exact'|'faceted'>('exact')
const primitiveSize = ref(20),primitiveTopRadius=ref(5),primitiveInnerRadius=ref(7)
// 24x24 stroke icons for the pane tool bars; labels stay in the tooltip and aria-label.
const TOOL_ICONS = {
  select: 'M5 3l14 8-7 1.5L8 20z',
  rectangle: 'M4 6h16v12H4z',
  slot: 'M8 6h8a6 6 0 0 1 0 12H8A6 6 0 0 1 8 6z',
  circle: 'M12 4a8 8 0 1 0 0 16 8 8 0 0 0 0-16z',
  arc: 'M4 18A10 10 0 0 1 20 10M4 18h.01M20 10h.01',
  trim: 'M6 3a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM6 15a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM20 4 8.5 15.5M20 20 8.5 8.5',
  polyline: 'M3 18l6-10 5 6 7-9',
  box: 'M4 4h4M10 4h4M16 4h4M4 20h4M10 20h4M16 20h4M4 8v4M4 14v4M20 8v4M20 14v4',
  body: 'M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7M12 11v10',
  face: 'M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7M12 11v10',
  edge: 'M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7M12 11v10',
  move: 'M12 2v20M2 12h20M12 2l-3 3M12 2l3 3M12 22l-3-3M12 22l3-3M2 12l3-3M2 12l3 3M22 12l-3-3M22 12l-3 3',
  rotate: 'M3 12a9 9 0 1 0 3-6.7M3 4v5h5',
  scale: 'M21 3l-7 7M21 3h-6M21 3v6M3 21l7-7M3 21h6M3 21v-6',
  orbit: 'M12 3a9 9 0 1 0 9 9M21 3v6h-6M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8z',
  grip: 'M5 9V7a2 2 0 0 1 4 0v2M9 9V5a2 2 0 0 1 4 0v4M13 9V6a2 2 0 0 1 4 0v5M17 11a2 2 0 0 1 4 0v4a7 7 0 0 1-7 7h-2a7 7 0 0 1-6-3.5L3 13a2 2 0 0 1 3-2l1 1.5',
  grid: 'M3 9h18M3 15h18M9 3v18M15 3v18M3 3h18v18H3z',
  download: 'M12 3v12M6 9l6 6 6-6M4 19h16',
} as const
// 24x24 stroke icons for the primitive bar; labels stay in the tooltip and aria-label.
const PRIMITIVE_ICONS: Record<typeof primitiveKinds[number] | 'curve' | 'surface', string> = {
  box: 'M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7M12 11v10',
  wedge: 'M4 17V9l16 8zM4 9l8-4 12 12-4 0M12 5v12',
  cylinder: 'M12 3c-4.4 0-8 1.3-8 3s3.6 3 8 3 8-1.3 8-3-3.6-3-8-3zM4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6',
  frustum: 'M9 4h6l5 14c0 1.7-3.6 3-8 3s-8-1.3-8-3zM9 4c0 1 1.3 1.5 3 1.5S15 5 15 4',
  tube: 'M12 3c-4.4 0-8 1.3-8 3s3.6 3 8 3 8-1.3 8-3-3.6-3-8-3zM4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6M12 4.5c-2 0-3.5.7-3.5 1.5s1.5 1.5 3.5 1.5 3.5-.7 3.5-1.5-1.5-1.5-3.5-1.5',
  cone: 'M12 3l8 15c0 1.7-3.6 3-8 3s-8-1.3-8-3zM4 18c0-1.7 3.6-3 8-3s8 1.3 8 3',
  sphere: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM3 12h18M12 3c-3 2.5-3 15.5 0 18M12 3c3 2.5 3 15.5 0 18',
  torus: 'M12 6c-5.5 0-10 2.7-10 6s4.5 6 10 6 10-2.7 10-6-4.5-6-10-6zM12 10c-2.2 0-4 .9-4 2s1.8 2 4 2 4-.9 4-2-1.8-2-4-2',
  curve: 'M3 18c4-12 8 12 12 0s3-6 6-6M3 18h.01M21 12h.01',
  surface: 'M3 8c3-3 6 3 9 0s6-3 9 0v9c-3 3-6-3-9 0s-6 3-9 0zM3 12.5c3-3 6 3 9 0s6-3 9 0',
}
function primitiveLabel(kind: typeof primitiveKinds[number]) { return ({box:label('Куб','Box'),wedge:label('Клин','Wedge'),cylinder:label('Цилиндр','Cylinder'),cone:label('Конус','Cone'),sphere:label('Сфера','Sphere'),frustum:label('Усечённый конус','Frustum'),tube:label('Труба','Tube'),torus:label('Тор','Torus')})[kind] }
const primitiveWorker=createSolidPreviewWorker(),primitivePending=ref(false)
let primitiveGeneration=0
function cancelPrimitive(){primitiveGeneration++;primitiveWorker.cancel();primitivePending.value=false}
onUnmounted(()=>{cancelPrimitive();primitiveWorker.dispose()})
watch(()=>[props.open,document.value,operation.value,advancedOp.value,tool.value,primitiveSize.value,primitiveTopRadius.value,primitiveInnerRadius.value,roundGeometry.value,brepSegments.value,activeGroup.value,lockedIds.value.join(',')],()=>{if(primitivePending.value)cancelPrimitive()},{flush:'sync'})
async function addPrimitive(kind: typeof primitiveKinds[number]) {
 cancelCommand();error.value=''
 const generation=++primitiveGeneration,before=document.value,target=history,id=crypto.randomUUID()
 const current=()=>generation===primitiveGeneration&&props.open&&!restoreDisposed&&before===document.value&&target===history
 let name=primitiveLabel(kind)
 if(['box','wedge','cylinder','frustum','tube'].includes(kind))name+=' · 3D'
 if(['cylinder','sphere','cone'].includes(kind)&&!(kind==='cone'&&roundGeometry.value==='faceted'))name+=` · ${roundGeometry.value==='exact'?label('точный B-rep','exact B-rep'):label('гранёный B-rep','faceted B-rep')}`
 if(['frustum','tube','torus'].includes(kind))name+=` · ${label('точный B-rep','exact B-rep')}`
 primitivePending.value=true;workspace.value?.focus()
 try{
  if(!Number.isFinite(primitiveSize.value)||primitiveSize.value<.1||primitiveSize.value>10000)throw Error(label('Размер от 0.1 до 10000 мм','Size must be 0.1–10000 mm'))
  const next=await primitiveWorker.run({kind:'primitive',document:history.document,options:{kind,id,name,size:primitiveSize.value,topRadius:primitiveTopRadius.value,innerRadius:primitiveInnerRadius.value,round:roundGeometry.value,segments:brepSegments.value,group:activeGroup.value||undefined}})
  if(!current())return
  const {validateLocked,added}=prepareCommit(next)
  const applied=await target.commitAsync(async()=>next,validateLocked)
  if(!current()||!applied)return
  primitivePending.value=false;finishCommit(added);pickObject(id,'3d');fit('3d')
 }catch(e){if(current())error.value=e instanceof Error?e.message:String(e)}
 finally{if(generation===primitiveGeneration)primitivePending.value=false}
}
const bridgeEndA=ref<'start'|'end'>('end'),bridgeEndB=ref<'start'|'end'>('start'),bridgeTension=ref(1)
const bridgeError=ref('')
const bridgeTensionInvalid=computed(()=>!Number.isFinite(bridgeTension.value)||bridgeTension.value<0.01||bridgeTension.value>10)
function addBridgeCurve(){
 if(bridgeTensionInvalid.value||nativeNurbsPending.value)return
 bridgeError.value=''
 const pair=selectedCurvePair.value;if(!pair)return
 void runNativeNurbs({kind:'bridge',id:pair[0],otherId:pair[1],endA:bridgeEndA.value,endB:bridgeEndB.value,tension:bridgeTension.value,createdId:crypto.randomUUID()})
}
function updateDimensionSketch(sketch:import('../services/directModeling').DirectSketch){run(()=>{
  const d=history.document,index=d.sketches.findIndex(s=>s.id===sketch.id)
  if(index>=0){validateSimpleSketch(sketch.points,sketch.closed);d.sketches[index]=sketch;commit(d)}
})}
const dimensionLabels=computed(()=>visibleSketches.value.flatMap(s=>{
  if(!s.dimensions?.length)return []
  return sketchDimensions(s).measurements.map((m,i)=>({key:s.id+'-'+i,point:m.label,lines:m.lines,text:(m.value===null?'—':m.value.toFixed(2))+(s.dimensions![i].kind==='angle'?'°':' mm')}))
}))
function addNurbs(kind: 'curve'|'surface') { run(() => {
  const d = history.document, item = kind === 'curve' ? createSolidNurbsCurve() : createSolidNurbsSurface()
  if (kind === 'curve') d.curves!.push(item as ReturnType<typeof createSolidNurbsCurve>)
  else d.surfaces!.push(item as ReturnType<typeof createSolidNurbsSurface>)
  commit(d); pickObject(item.id, '3d'); fit('3d')
}) }
function updateCv() { run(() => {
  const point = [cvX.value, cvY.value, cvZ.value]
  const invalid=point.findIndex(value=>!Number.isFinite(value))
  cvInvalidField.value=invalid>=0?['x','y','z'][invalid]:!Number.isFinite(cvWeight.value)||cvWeight.value<=0?'weight':''
  if(cvInvalidField.value){
    const field=cvInvalidField.value
    void nextTick(()=>workspace.value?.querySelector<HTMLInputElement>(`[data-cv-field="${field}"]`)?.focus())
    return
  }
  cancelCommand();numericPointEdit=true
  requestDragJob({kind:'pointEdit',document:history.document,options:{kind:'cv',id:selection.value,u:cvU.value,v:cvV.value,point:point as Vec3,weight:cvWeight.value}});gizmoApplyRevision=gizmoRevision
}) }
function cancelNativeNurbs(){nativeNurbsGeneration++;nativeNurbsWorker.cancel();nativeNurbsPending.value=false;nativeBrepMode.value=null}
async function runNativeNurbs(options:SolidNurbsEditOptions){
 const curveBake=options.kind==='bake'&&!!selectedNurbsCurve.value
 cancelCommand();error.value=''
 const generation=++nativeNurbsGeneration
 nativeNurbsPending.value=true
 try{
  const result=await nativeNurbsWorker.run({kind:'nurbsEdit',document:history.document,options})
  if(generation!==nativeNurbsGeneration||!props.open)return
  nativeNurbsPending.value=false;commit(result)
  if(options.kind==='trim')cvU.value=cvV.value=0
  if('createdId' in options){pickObject(options.createdId,curveBake?'2d':'3d');if(curveBake||options.kind==='extrude')fit(curveBake?'2d':'3d')}
 }catch(e){if(generation===nativeNurbsGeneration){const message=e instanceof Error?e.message:String(e);if(options.kind==='bridge')bridgeError.value=message;else error.value=message}}
 finally{if(generation===nativeNurbsGeneration)nativeNurbsPending.value=false}
}
watch(()=>[props.open,document.value,selection.value,extraSelection.value.join(','),activeGroup.value,knotValue.value,...trimBounds.value,bridgeEndA.value,bridgeEndB.value,bridgeTension.value,brepSegments.value],()=>{bridgeError.value='';if(nativeNurbsPending.value)cancelNativeNurbs()},{flush:'sync'})
onUnmounted(()=>{cancelNativeNurbs();nativeNurbsWorker.dispose()})
function insertNativeKnot(axis:'curve'|'u'|'v'){void runNativeNurbs({kind:'knot',id:selection.value,axis,value:knotValue.value})}
function elevateNative(axis:'curve'|'u'|'v'){void runNativeNurbs({kind:'elevate',id:selection.value,axis})}
function curveToSurface(){void runNativeNurbs({kind:'extrude',id:selection.value,createdId:crypto.randomUUID()})}
function extractIso(axis:'u'|'v'){void runNativeNurbs({kind:'iso',id:selection.value,axis,value:knotValue.value,createdId:crypto.randomUUID()})}
function trimNativeSurface(){void runNativeNurbs({kind:'trim',id:selection.value,bounds:[...trimBounds.value]})}
function matchSelectedG1() { run(() => {
  const pair = selectedCurvePair.value ?? selectedSurfacePair.value
  if (!pair) throw new Error('Select exactly two curves or two surfaces.')
  if(selectedSurfacePair.value){beginAdvanced('nurbs-match');return}
  beginAdvanced('nurbs-curve-match')
}) }
function bakeNurbs(){void runNativeNurbs({kind:'bake',id:selection.value,createdId:crypto.randomUUID()})}

// The async component may mount with a seed already present. Install this
// watcher after setup state is initialized, and consume each seed once so a
// later open does not overwrite edits or an Undo of the import.
const appliedSeeds = new WeakSet<DirectDocument>()
/** Bodies listed by group, ungrouped first, each group keeping its insertion order. */
const bodySections = computed(() => {
  const ungrouped = document.value.bodies.filter(body => body.group === undefined)
  const sections: { key: string; name: string | null; bodies: DirectBody[] }[] = []
  if (ungrouped.length) sections.push({ key: '', name: null, bodies: ungrouped })
  const named = new Map<string, DirectBody[]>((document.value.groups??[]).map(g=>[g.name,[]]))
  for (const object of documentObjects(document.value))if(object.group&&!named.has(object.group))named.set(object.group,[])
  for (const body of document.value.bodies) {
    if (body.group === undefined) continue
    const existing = named.get(body.group)
    if (existing) existing.push(body)
    else named.set(body.group, [body])
  }
  for (const [name, bodies] of named) sections.push({ key: name, name, bodies })
  return sections
})


interface SceneRow {key:string;height:number;kind:'object'|'group'|'label'|'empty';pane?:Pane;dot?:string;title?:string;section?:{name:string|null};item?:{id:string;name:string;group?:string;instance?:unknown;brep?:unknown;material?:{color:string}}}
const sceneRows=computed(()=>{
 const rows:SceneRow[]=[]
 const objects=(items:SceneRow['item'][],pane:Pane,dot:string)=>{for(const item of items)if(item)rows.push({key:'object:'+item.id,height:32,kind:'object',item,pane,dot})}
 if(document.value.sketches.length){rows.push({key:'label:sketch',height:28,kind:'label',title:label('Эскизы','Sketches')});objects(document.value.sketches,'2d','sketch')}
 for(const section of bodySections.value){rows.push({key:'group:'+section.key,height:44,kind:'group',section});objects(section.bodies,'3d','body')}
 const nurbs=[...document.value.curves??[],...document.value.surfaces??[]]
 if(nurbs.length){rows.push({key:'label:nurbs',height:28,kind:'label',title:'NURBS'});objects(nurbs,'3d','nurbs')}
 if(!rows.length)rows.push({key:'empty',height:80,kind:'empty'})
 return rows
})

function isolateGroup(name:string) {
  cancelCommandState()
  isolatedBodyIds.value=documentObjects(document.value).filter(o=>o.group===name).map(o=>o.id)
  const selected=selectedIds.value.filter(id=>isolatedBodyIds.value.includes(id)&&objectSelectable(id))
  selection.value=selected[0]??'';extraSelection.value=selected.slice(1);hovered.value='';fit('3d')
}

function removeGroup(name: string) {
  run(() => {
    if(documentObjects(document.value).some(b=>b.group===name&&lockedIds.value.includes(b.id)))throw Error(label('Сначала разблокируйте объекты группы','Unlock the group objects first'))
    const next = { ...history.document }
    next.bodies = next.bodies.filter(body => body.group !== name)
    next.sketches = next.sketches.filter(item=>item.group!==name)
    if(next.curves)next.curves=next.curves.filter(item=>item.group!==name)
    if(next.surfaces)next.surfaces=next.surfaces.filter(item=>item.group!==name)
    const remaining = (next.groups ?? []).filter(group => group.name !== name)
    next.groups = remaining.length ? remaining : undefined
    cancelGesture()
    commit(next)
    if (!next.bodies.some(body => body.id === selection.value)) selection.value = ''
    sync()
  })
}

const DEFAULT_GROUP_SOURCE = 'cube([20, 20, 20], center = true);\n'

/** The source lives in the host's left panel; this only chooses what to open there. */
function openGroupDialog(name: string | null) {
  if (name === null) {
    const taken = new Set(document.value.groups?.map(group => group.name) ?? [])
    let index = 1
    while (taken.has(label('Группа ', 'Group ') + index)) index++
    emit('edit-group', { name: label('Группа ', 'Group ') + index, source: DEFAULT_GROUP_SOURCE, replaces: null })
    return
  }
  const source = document.value.groups?.find(group => group.name === name)?.source ?? ''
  emit('edit-group', { name, source, replaces: name })
}

/** A rebuild replaces the group of the same name, so repeated builds do not pile up. */
watch(() => props.appendBodies, request => {
  if (!request) return
  run(() => {
    const built = request.group
    const replaced = new Set<string>(built ? [built.name] : [])
    // An edit that renamed the group must also drop the bodies filed under the old name.
    if (built?.replaces) replaced.add(built.replaces)
    const next = { ...history.document }
    if(next.bodies.some(b=>b.group&&replaced.has(b.group)&&lockedIds.value.includes(b.id)))throw Error(label('Сначала разблокируйте объекты группы','Unlock the group objects first'))
    next.bodies = [
      ...next.bodies.filter(body => body.group === undefined || !replaced.has(body.group)),
      ...request.bodies,
    ]
    if (built) {
      const groups = (next.groups ?? []).filter(group => !replaced.has(group.name))
      next.groups = [...groups, { name: built.name, source: built.source }]
    }
    cancelGesture()
    commit(parseDirectDocument(stringifyMeshJson(next)))
    selection.value = request.bodies[0]?.id ?? ''
    sync()
  })
})

watch(()=>[props.open,kernelReady.value,selectedBody.value,document.value],()=>{
 cancelTopology(true)
 if(props.open&&kernelReady.value&&selectedBody.value)void prepareTopology(selectedBody.value)
},{immediate:true,flush:'sync'})

watch(()=>[props.open,kernelReady.value,selectedBody.value,document.value,pickMode.value],()=>void refreshBodyEdges(),{immediate:true,flush:'post'})

watch(()=>[props.open,document.value,selectedBody.value,faceIndex.value],()=>cancelFaceSketch(),{flush:'sync'})

const supportSnapSketches=computed<DirectSketch[]>(()=>workplaneOutline.value.map((points,i)=>({id:`support-${i}`,name:'Support',closed:true,points:points.map(p=>[p[0],p[1]] as Point2)})))
const snapSketches=computed(()=>[...snapDocument.value.sketches.filter(s=>objectInView(s.id)),...supportSnapSketches.value])
const sketchSnapsNeedRetry=computed(()=>{void sketchSnapVersion.value;return props.open&&snap.value&&geometrySnap.value&&!sketchSnapPending.value&&snapSketches.value.some(s=>!sketchSnapPreparation.get(s))})
async function refreshSketchSnaps(){
 cancelSketchSnaps();sketchSnapErrors.value=[];snapSceneCache=undefined
 if(!props.open||!kernelReady.value||!snap.value||!geometrySnap.value)return
 const generation=sketchSnapGeneration;sketchSnapPending.value=snapSketches.value.length>0
 const result=await sketchSnapPreparation.prepare(snapSketches.value)
 if(generation!==sketchSnapGeneration)return
 sketchSnapVersion.value++;snapSceneCache=undefined;sketchSnapErrors.value=result.errors;sketchSnapPending.value=false
}
watch(()=>[props.open,kernelReady.value,snap.value,geometrySnap.value,snapSketches.value],()=>void refreshSketchSnaps(),{immediate:true,flush:'post'})
const snapBodies=computed(()=>snapDocument.value.bodies.filter(b=>objectInView(b.id)))
const snapPreparationNeedsRetry=computed(()=>{void snapPreparationVersion.value;return props.open&&snap.value&&geometrySnap.value&&!snapPreparationPending.value&&snapBodies.value.some(b=>!snapPreparation.get(b))})
async function refreshSnapPreparation(){
 cancelSnapPreparation();snapSceneCache=undefined;snapPreparationErrors.value=[]
 if(!props.open||!kernelReady.value||!snap.value||!geometrySnap.value)return
 const generation=snapPreparationGeneration
 snapPreparationPending.value=snapBodies.value.length>0
 const result=await snapPreparation.prepare(snapBodies.value)
 if(generation!==snapPreparationGeneration)return
 snapPreparationVersion.value++;snapSceneCache=undefined;snapPreparationErrors.value=result.errors;snapPreparationPending.value=false
}
watch(()=>[props.open,kernelReady.value,snap.value,geometrySnap.value,snapBodies.value],()=>void refreshSnapPreparation(),{immediate:true,flush:'post'})

const displayCurves=computed(()=>[...(document.value.curves??[]),...(advancedPreview.value.document?.curves??[])])
const curveDisplayNeedsRetry=computed(()=>{void curveDisplayVersion.value;return !curveDisplayPending.value&&displayCurves.value.some(item=>!curveDisplayQueue.get(item))})
async function refreshCurveDisplay(){
 cancelCurveDisplay();curveDisplayErrors.value=[]
 if(!props.open||!kernelReady.value)return
 const generation=curveDisplayGeneration
 curveDisplayPending.value=displayCurves.value.length>0
 const result=await curveDisplayQueue.prepare(displayCurves.value)
 if(generation!==curveDisplayGeneration)return
 if(result.changed)curveDisplayVersion.value++
 curveDisplayErrors.value=result.errors;curveDisplayPending.value=false
}
watch(()=>[props.open,kernelReady.value,displayCurves.value],()=>void refreshCurveDisplay(),{immediate:true,flush:'post'})

const displayProfiles=computed(()=>[...document.value.sketches,...(advancedPreview.value.document?.sketches??[])].flatMap(s=>s.retainedProfile?[{id:s.id,profile:s.retainedProfile}]:[]))
const profileDisplayNeedsRetry=computed(()=>{void profileDisplayVersion.value;return !profileDisplayPending.value&&displayProfiles.value.some(item=>!profileDisplayQueue.get(item))})
async function refreshProfileDisplay(){
 cancelProfileDisplay();profileDisplayErrors.value=[]
 if(!props.open||!kernelReady.value)return
 const generation=profileDisplayGeneration
 profileDisplayPending.value=displayProfiles.value.length>0
 const result=await profileDisplayQueue.prepare(displayProfiles.value)
 if(generation!==profileDisplayGeneration)return
 if(result.changed)profileDisplayVersion.value++
 profileDisplayErrors.value=result.errors;profileDisplayPending.value=false
}
watch(()=>[props.open,kernelReady.value,displayProfiles.value],()=>void refreshProfileDisplay(),{immediate:true,flush:'post'})

async function refreshSurfaceDisplay(){
 cancelSurfaceDisplay();surfaceDisplayErrors.value=[]
 if(!props.open||!kernelReady.value)return
 const generation=surfaceDisplayGeneration
 const items=[...(document.value.surfaces??[]),...(advancedPreview.value.document?.surfaces??[])]
 surfaceDisplayPending.value=items.length>0
 const result=await surfaceDisplayQueue.prepare(items)
 if(generation!==surfaceDisplayGeneration)return
 if(result.changed)surfaceDisplayVersion.value++
 surfaceDisplayErrors.value=result.errors;surfaceDisplayPending.value=false
}
watch(()=>[props.open,kernelReady.value,document.value.surfaces,advancedPreview.value.document?.surfaces],()=>void refreshSurfaceDisplay(),{immediate:true,flush:'post'})

watchEffect(onCleanup=>{
 const enabled=props.open&&surfaceDistanceOpen.value&&!commandActive.value,pair=selectedSurfacePair.value
 const surfaces=document.value.surfaces,maxCells=surfaceDistanceBudget.value
 let current=true
 onCleanup(()=>{current=false;surfaceDistanceWorker.cancel()})
 surfaceDistance.value=null;surfaceDistancePending.value=false
 if(!enabled||!pair)return
 const [a,b]=pair.map(id=>surfaces!.find(s=>s.id===id)!.surface)
 surfaceDistancePending.value=true
 void surfaceDistanceWorker.run({kind:'surfaceDistance',a,b,toleranceMm:.001,maxCells}).then(value=>{
  if(current)surfaceDistance.value={value,error:''}
 }).catch(()=>{if(current)surfaceDistance.value={value:null,error:label('Не удалось измерить поверхности. Проверьте геометрию или увеличьте объём расчёта.','Could not measure the surfaces. Check their geometry or increase the calculation budget.')}})
 .finally(()=>{if(current)surfaceDistancePending.value=false})
})
watchEffect(onCleanup=>{
 const enabled=props.open&&boundaryInspection.value&&!commandActive.value,pair=selectedSurfacePair.value
 const surfaces=document.value.surfaces,options={...boundaryOptions.value}
 let current=true
 onCleanup(()=>{current=false;boundaryWorker.cancel()})
 boundaryReport.value=null;boundaryPending.value=false
 if(!enabled||!pair)return
 const [a,b]=pair.map(id=>surfaces!.find(s=>s.id===id)!.surface)
 boundaryPending.value=true
 void boundaryWorker.run({kind:'surfaceBoundary',a,b,options}).then(value=>{
  if(current)boundaryReport.value={value,error:''}
 }).catch(e=>{if(current)boundaryReport.value={value:null,error:e instanceof Error?e.message:String(e)}})
 .finally(()=>{if(current)boundaryPending.value=false})
})

watch([() => props.open, () => props.seedDocument, restoringDraft], ([open, seed, restoring], previous) => {
  if (!open || !seed || appliedSeeds.has(seed)) return
  if (restoring) {
    // Initial seeds are edits over the recovered baseline. A later explicit seed
    // supersedes an in-flight recovery without publishing its delayed result.
    if (!previous.length || previous[1] === seed) return
    stopDraftRecovery()
  }
  run(() => {
    const next = parseDirectDocument(stringifyMeshJson(seed))
    next.blenderProjectId ??= crypto.randomUUID()
    cancelGesture()
    history.commit(next)
    selection.value = next.bodies[0]?.id ?? ''
    extraSelection.value = []; faceIndex.value = edgeIndex.value = -1
    edgeIndexes.value = []; openingFaces.value = []
    operation.value = null; advancedOp.value = null; previewBody.value = null
    sync()
    appliedSeeds.add(seed)
  })
}, {immediate: true})
</script>
<template>
  <section v-show="open" ref="workspace" class="direct-workspace" :class="{ embedded, 'touch-mode': inputMode === 'touch', 'compact-workspace': compactWorkspace }" tabindex="-1" :aria-label="label('Solid — CAD-лепка', 'Solid — CAD sculpt')" @keydown.stop="keydown" @dragstart.prevent>
    <div v-if="restoringDraft" class="restore-loading" role="status" aria-label="draft-recovery">{{ label('Восстанавливаю геометрию…', 'Restoring geometry…') }} <button @click="cancelDraftRecovery">{{ label('Отмена · Esc','Cancel · Esc') }}</button></div>
    <header class="workspace-bar">
      <button v-if="embedded" class="back" @click="emit('close')">← {{ label('Code', 'Code') }}</button>
      <strong>{{ label('Solid', 'Solid') }}</strong>
      <span class="subtle">{{ label('Plasticity-like CAD', 'Plasticity-like CAD') }}</span>
      <div class="history-tools"><button :disabled="!undoable || historyPending" @click="undo()" :title="label('Отменить · Ctrl/⌘ Z', 'Undo · Ctrl/⌘ Z')">↶</button><button :disabled="!redoable || historyPending" @click="undo(true)" :title="label('Повторить · Ctrl/⌘ Shift Z', 'Redo · Ctrl/⌘ Shift Z')">↷</button></div>
      <button class="input-mode-toggle" :aria-pressed="inputMode === 'touch'" :title="label('Переключить управление: тач / мышь', 'Switch controls: touch / mouse')" @click="toggleInputMode">{{ inputMode === 'touch' ? label('Тач', 'Touch') : label('Мышь', 'Mouse') }}</button>
      <button v-if="inputMode === 'touch'" :aria-pressed="touchNavigate" @click="toggleTouchNavigation">{{ label('Навигация', 'Navigate') }}</button>
      <button :aria-pressed="compactWorkspace" @click="compactWorkspace = !compactWorkspace">{{ compactWorkspace ? label('Инструменты', 'Tools') : label('Больше места', 'More space') }}</button>
      <button @click="showHelp = !showHelp" title="Keyboard shortcuts">?</button>
      <button class="command-search" :title="label('Поиск команд · Ctrl/⌘ K', 'Search commands · Ctrl/⌘ K')" @click="paletteOpen = true"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg><span>{{ label('Команда…', 'Command…') }}</span><kbd>Ctrl K</kbd></button>
      <span v-if="sketchSnapPending" role="status" aria-label="sketch-snap-preparation">{{ label('Готовлю привязки эскизов…','Preparing sketch snaps…') }} <button @click="cancelSketchSnaps">Esc</button></span>
      <button v-if="sketchSnapsNeedRetry" @click="refreshSketchSnaps">{{ label('Обновить привязки эскизов','Refresh sketch snaps') }}</button>
      <span v-if="sketchSnapErrors.length" role="alert">{{ label('Не готовы привязки эскизов: ','Sketch snaps unavailable: ')+sketchSnapErrors.map(e=>e.id).join(', ') }} {{ sketchSnapErrors[0].message }}</span>
      <span v-if="snapPreparationPending" role="status" aria-label="snap-preparation">{{ label('Готовлю привязки к телам…','Preparing body snaps…') }} <button @click="cancelSnapPreparation">Esc</button></span>
      <button v-if="snapPreparationNeedsRetry" @click="refreshSnapPreparation">{{ label('Обновить привязки','Refresh snaps') }}</button>
      <span v-if="snapPreparationErrors.length" role="alert">{{ label('Не готовы привязки: ','Snaps unavailable: ')+snapPreparationErrors.map(e=>e.id).join(', ') }} {{ snapPreparationErrors[0].message }}</span>
      <span v-if="faceSketchPending" role="status" aria-label="face-sketch-preparation">{{ label('Готовлю плоскость эскиза…','Preparing sketch plane…') }} <button @click="cancelFaceSketch">Esc</button></span>
      <span v-if="bodyEdgesPending" role="status" aria-label="body-edges">{{ label('Готовлю рёбра…','Preparing edges…') }} <button @click="cancelBodyEdges">Esc</button></span>
      <button v-if="pickMode==='edge' && selectedBody && !bodyEdgesPending && !authoredEdges.length" @click="refreshBodyEdges">{{ label('Обновить рёбра','Refresh edges') }}</button>
      <span v-if="topologyPending" role="status" aria-label="topology-preparation">{{ label('Готовлю выбор граней…','Preparing face selection…') }} <button @click="cancelTopology()">Esc</button></span>
      <span v-if="curveDisplayPending" role="status" aria-label="curve-display">{{ label('Строю отображение кривых…','Preparing curve display…') }} <button @click="cancelCurveDisplay">Esc</button></span>
      <button v-if="curveDisplayNeedsRetry" @click="refreshCurveDisplay">{{ label('Обновить кривые','Refresh curves') }}</button>
      <span v-if="curveDisplayErrors.length" role="alert">{{ label('Не удалось отобразить кривые: ','Could not display curves: ')+curveDisplayErrors.map(e=>e.id).join(', ') }} {{ curveDisplayErrors[0].message }}</span>
      <span v-if="profileDisplayPending" role="status" aria-label="profile-display">{{ label('Строю отображение профилей…','Preparing profile display…') }} <button @click="cancelProfileDisplay">Esc</button></span>
      <button v-if="profileDisplayNeedsRetry" @click="refreshProfileDisplay">{{ label('Обновить профили','Refresh profiles') }}</button>
      <span v-if="profileDisplayErrors.length" role="alert">{{ label('Не удалось отобразить профили: ','Could not display profiles: ')+profileDisplayErrors.map(e=>e.id).join(', ') }} {{ profileDisplayErrors[0].message }}</span>
      <span v-if="surfaceDisplayPending" role="status" aria-label="surface-display">{{ label('Строю отображение поверхностей…','Preparing surface display…') }} <button @click="cancelSurfaceDisplay">Esc</button></span>
      <button v-if="surfaceDisplayNeedsRetry" @click="refreshSurfaceDisplay">{{ label('Обновить поверхности','Refresh surfaces') }}</button>
      <span v-if="surfaceDisplayErrors.length" role="alert">{{ label('Не удалось отобразить поверхности: ','Could not display surfaces: ')+surfaceDisplayErrors.map(e=>e.id).join(', ') }} {{ surfaceDisplayErrors[0].message }}</span>
      <span v-if="primitivePending" role="status" aria-label="primitive-build">{{ label('Создаю тело…','Creating body…') }} <button @click="cancelPrimitive">Esc</button></span>
      <span v-if="displayPending" role="status" aria-label="display-refinement">{{ label('Обновляю отображение…','Updating display…') }} <button @click="cancelDisplayPreparation">Esc</button></span>
      <span v-if="historyPending" role="status" aria-label="history-restore">{{ historyMode==='import'?label('Проверяю импорт…','Checking import…'):historyMode==='shared'?label('Загружаю общую версию…','Loading shared version…'):label('Восстанавливаю историю…','Restoring history…') }} <button @click="cancelHistoryRestore">Esc</button></span>
      <span class="save-status" :class="{ error: saveError }" role="status" :title="savePending ? label('Сохраняю…','Saving…') : saveError ? label('Не сохранено', 'Unsaved') : label('Сохранено в браузере', 'Saved in browser')" :aria-label="savePending ? label('Сохраняю…','Saving…') : saveError ? label('Не сохранено', 'Unsaved') : label('Сохранено в браузере', 'Saved in browser')">
        <span v-if="savePending">…</span>
        <svg v-else-if="saveError" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 8v5M12 17h.01"/><path d="M10.3 3.9 2.5 18a2 2 0 0 0 1.7 3h15.6a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z"/></svg>
        <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M7 18a4.5 4.5 0 0 1-.6-9A6 6 0 0 1 18 8.5 3.8 3.8 0 0 1 17.5 18z"/><path d="m9 13 2 2 4-4"/></svg>
      </span>
      <details class="file-menu" @keydown.esc="closeFileMenu"><summary :title="label('Файл', 'File')"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" aria-hidden="true"><path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><path d="M14 3v6h6"/></svg><span>{{ label('Файл', 'File') }}</span><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg></summary><div>
        <button :disabled="restoringDraft" @click="downloadProject">{{ label('Скачать проект JSON', 'Download JSON project') }}</button>
        <label class="file-open">{{ label('Открыть Solid / ModelGraph NURBS', 'Open Solid / ModelGraph NURBS') }}<input type="file" accept=".json,application/json" @change="importFile"></label>
        <button type="button" @click="stlInput?.click()">{{ label('Импорт STL / OBJ / PLY / OFF / AMF / 3MF как тело', 'Import STL / OBJ / PLY / OFF / AMF / 3MF as body') }}</button>
        <input ref="stlInput" type="file" :accept="MESH_IMPORT_ACCEPT" hidden @change="importStl" />
        <button type="button" :disabled="svgBusy" @click="svgInput?.click()">{{ label('Импорт SVG как профили', 'Import SVG as profiles') }}</button>
        <button v-if="draftConflict" type="button" :disabled="loadingLatestDraft" @click="loadLatestDraft">{{ label('Загрузить сохранённую версию · локальная останется в Undo','Load saved version · keep local in Undo') }}</button>
        <button v-if="saveError && !draftConflict" type="button" :disabled="savePending" @click="persist">{{ label('Повторить сохранение черновика','Retry saving draft') }}</button>
        <input ref="svgInput" type="file" accept=".svg,image/svg+xml" hidden @change="importSvg" />
        <button type="button" :disabled="stepBusy" @click="stepInput?.click()">{{ label('Импорт STEP', 'Import STEP') }}</button>
        <input ref="stepInput" type="file" accept=".step,.stp" hidden @change="importStep" />
        <button type="button" :disabled="restoringDraft || !selectedBody" @click="exportBodySvg">{{ label('SVG выбранного тела · проекция XY', 'Selected body SVG · XY projection') }}</button>
        <button type="button" :disabled="restoringDraft || !document.bodies.length" @click="exportBlender">{{ label('Экспорт для Blender', 'Export for Blender') }}</button>
        <button type="button" :disabled="restoringDraft || !selectedBody?.brep" @click="exportStepCurrent">{{ label('STEP выбранного тела · текущая геометрия', 'Export current body STEP') }}</button>
        <button type="button" :disabled="restoringDraft || !document.bodies.length" @click="exportStepAssembly">{{ label('STEP сборки · тела и группы', 'Export bodies and groups STEP') }}</button>
        <button type="button" :disabled="restoringDraft || stepBusy" @click="exportStepOriginal">{{ label('Экспорт AP242-оригинала', 'Export AP242 original') }}</button>
        <button :disabled="!document.bodies.length" @click="download(directBodiesScad(document), 'solid-bodies.scad')">{{ label('Экспорт SCAD (bake)', 'Export SCAD (bake)') }}</button>
        <button :disabled="!document.bodies.length || !canAppend" @click="appendBodies">{{ embedded ? label('Bake в код', 'Bake into code') : label('Bake в Code (append)', 'Bake into Code (append)') }}</button>
        <button :disabled="!document.bodies.length" @click="sendToMesh">{{ label('Открыть в Mesh', 'Open in Mesh') }}</button>
      </div></details>
    </header>
    <div class="sketch-start-bar" :aria-label="label('От плоской фигуры к объёму', 'From a flat shape to a solid')">
      <strong>{{ label('Плоские фигуры', 'Flat shapes') }}</strong>
      <button v-for="item in [{ tool: 'rectangle' as const, ru: 'Прямоугольник', en: 'Rectangle', key: 'R' }, { tool: 'circle' as const, ru: 'Круг', en: 'Circle', key: 'C' }, { tool: 'polyline' as const, ru: 'Контур', en: 'Contour', key: 'L' }]" :key="item.tool" type="button" :aria-pressed="tool === item.tool" :title="`${label(item.ru,item.en)} · ${item.key}`" @click="beginSketch(item.tool)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path :d="TOOL_ICONS[item.tool]" /></svg>{{ label(item.ru,item.en) }} · {{ item.key }}
      </button>
      <button type="button" :disabled="!document.bodies.length" :aria-pressed="choosingSketchFace" @click="chooseSketchFace">{{ label('На грани', 'On face') }}</button>
      <button v-if="workplaneBodyId" type="button" @click="resetWorkplane()">XY</button>
      <span aria-hidden="true">→</span>
      <button type="button" class="extrude-sketch" :disabled="!canExtrudeSketch" :title="canExtrudeSketch ? label('Задайте высоту или тяните ручку в 3D', 'Set a height or drag the handle in 3D') : label('Нарисуйте и выберите замкнутый контур', 'Draw and select a closed contour')" @click="beginExtrude()">{{ label('Выдавить · E', 'Extrude · E') }}</button>
      <span class="subtle">{{ selectedSketch && !selectedSketch.closed ? label('Для объёма нужен замкнутый контур', 'Close the contour to create a solid') : label('Нарисуйте на плоскости → задайте высоту → Enter', 'Draw on a plane → set height → Enter') }}</span>
    </div>
    <div v-if="choosingSketchFace || (workplaneBodyId && tool!=='select')" class="workplane-hint" role="status">{{ choosingSketchFace ? label('Выберите плоскую грань тела в 3D. Esc — отмена.', 'Pick a planar body face in 3D. Esc cancels.') : label('Рисуйте на выделенной грани в 3D или в панели эскиза. Затем нажмите «Выдавить».', 'Draw on the highlighted face in 3D or in the sketch pane, then choose Extrude.') }}</div>
    <div class="primitive-bar">
      <strong>{{ label('Примитивы','Primitives') }}</strong>
      <button v-for="kind in primitiveKinds" :key="kind" class="primitive-icon" :disabled="!kernelReady" :title="primitiveLabel(kind)" :aria-label="primitiveLabel(kind)" @click="addPrimitive(kind)"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="PRIMITIVE_ICONS[kind]" /></svg></button>
      <span class="primitive-divider" aria-hidden="true"></span>
      <button class="primitive-icon" :title="label('NURBS-кривая','NURBS curve')" :aria-label="label('NURBS-кривая','NURBS curve')" @click="addNurbs('curve')"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="PRIMITIVE_ICONS.curve" /></svg></button>
      <button class="primitive-icon" :title="label('NURBS-поверхность','NURBS surface')" :aria-label="label('NURBS-поверхность','NURBS surface')" @click="addNurbs('surface')"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="PRIMITIVE_ICONS.surface" /></svg></button>
      <details><summary>{{ label('Параметры круглых тел','Round solid options') }}</summary>
      <label>{{ label('Верхний радиус конуса, мм','Frustum top radius, mm') }} <input v-model.number="primitiveTopRadius" type="number" min="0.00001" /></label>
      <label>{{ label('Внутренний радиус трубы, мм','Tube inner radius, mm') }} <input v-model.number="primitiveInnerRadius" type="number" min="0.00001" /></label>
      </details>
      <label>{{ label('Сфера / цилиндр / конус','Sphere / cylinder / cone') }} <select v-model="roundGeometry"><option value="exact">{{ label('Точные поверхности','Exact surfaces') }}</option><option value="faceted">{{ label('Гранёный · planar Boolean','Faceted · planar Boolean') }}</option></select></label>
      <label>{{ label('Размер, мм','Size, mm') }} <input v-model.number="primitiveSize" type="number" min="0.1" max="10000" /></label>
      <button type="button" :title="label('Импорт сетки как тело', 'Import mesh as body')" @click="stlInput?.click()">{{ label('Импорт сетки', 'Import mesh') }}</button>
      <button v-if="embedded" :disabled="!canAppend" @click="appendBodies">{{ label('Применить в код (bake)','Apply to code') }}</button>
      <span class="subtle">{{ label('Solid хранит свой документ; bake в .scad — только по кнопке.','Solid keeps its own document; bake to .scad is explicit.') }}</span>
    </div>
    <div class="workspace-state" aria-live="polite">
      <label>{{ label('Создавать в','Create in') }} <select v-model="activeGroup" :aria-label="label('Активная группа','Active group')"><option value="">{{ label('Без группы','Ungrouped') }}</option><option v-for="section in bodySections.filter(s=>s.name)" :key="section.key" :value="section.name!">{{ section.name }}</option></select></label>
      <button :disabled="!selectedIds.length" @click="moveSelectionToGroup">{{ label('Перенести выбор', 'Move selection to group') }}</button>
      <button @click="addEmptyGroup">{{ label('Новая группа','New empty group') }}</button>
      <button v-if="hiddenIds.length" @click="hiddenIds=[]">{{ label('Показать всё','Show all objects') }} ({{ hiddenIds.length }})</button>
      <span v-if="lockedIds.length">{{ label('Заблокировано','Locked') }}: {{ lockedIds.length }}</span>
      <span v-if="subtract">{{ label('A — основа · B — вырез', 'A — target · B — cutter') }}</span>
      <label v-if="pickMode==='edge' && selectedBody">{{ label('Ребро','Edge') }} <select :value="edgeIndex" :aria-label="label('Выбрать ребро','Select edge')" @change="edgeIndex=Number(($event.target as HTMLSelectElement).value);edgeIndexes=edgeIndex>=0?[edgeIndex]:[];advancedOp=null"><option :value="-1">—</option><option v-for="edge in featureEdges" :key="edge.i" :value="edge.i">{{ edge.i+1 }}</option></select></label>
      <span v-if="edgeIndexes.length">{{ label('Рёбра','Edges') }}: {{ edgeIndexes.length }}</span>
      <span v-if="dragConstraint">{{ label('Ограничение','Constraint') }}: {{ dragConstraint }} · Esc {{ label('отмена','cancel') }}</span>
      <span v-else-if="advancedOp==='split' || advancedOp==='transform'">{{ label('Ось','Axis') }}: {{ advanced.axis.toUpperCase() }}</span>
      <span v-else-if="operation==='revolve'">{{ label('Ось эскиза','Sketch axis') }}: {{ revolveAxis.toUpperCase() }}</span>
      <button :disabled="!!repeatReason" :title="repeatReason || repeatEntry?.label" @click="repeatCommand">{{ label('Повтор · Shift R', 'Repeat · Shift R') }}</button>
      <button :aria-pressed="snap" @click="snap = !snap">{{ label('Привязки', 'Snapping') }}: {{ snap ? label('вкл', 'on') : label('выкл', 'off') }}</button>
      <span>{{ label('Плоскость', 'Plane') }}: {{ workplaneBodyId ? label('на грани', 'on face') : samePlane(activePlane, xyPlane()) ? 'XY' : label('пользовательская', 'custom') }}</span>
      <button :disabled="!isolatedBodyIds.length && !selectedBody" :aria-pressed="!!isolatedBodyIds.length" @click="toggleBodyIsolation">{{ isolatedBodyIds.length ? label('Выйти из изоляции', 'Exit isolation') : label('Изолировать тела', 'Isolate bodies') }}</button>
      <span>{{ label('Выбрано', 'Selected') }}: {{ selectedIds.length }}</span>
      <span v-if="selectedGroup">{{ label('Группа выбора', 'Selection group') }}: {{ selectedGroup }}</span>
      <span class="state-legend"><i class="state-hover"></i>{{ label('Наведение','Hover') }} <i class="state-selected"></i>{{ label('Выбор','Selection') }} <i class="state-preview"></i>{{ label('Предпросмотр','Preview') }}</span>
    </div>
    <div v-if="commandActive" class="command-guidance" :class="{ failed: commandFailure }" role="status">
      <strong>{{ commandFailure ? label('! Ошибка построения', '! Build error') : booleanPending ? 'Boolean' : label('Предпросмотр операции', 'Operation preview') }}</strong>
      <span>{{ commandHint }}</span><span class="command-keys"><template v-if="commandReady"><kbd>Enter</kbd> {{ label('применить','apply') }} </template> <button @click="cancelCommand"><kbd>Esc</kbd> {{ label('отмена','cancel') }}</button></span>
    </div>
    <div class="workspace-row">
    <div ref="splitArea" class="split-workspace" :class="{ 'sketch-hidden': !sketchPaneOpen }" :style="{ '--split': split + '%' }">
      <template v-for="pane in panes" :key="pane">
        <div v-if="pane === '3d' && sketchPaneOpen" class="splitter" role="separator" tabindex="0" aria-orientation="vertical" :aria-label="label('Ширина 2D и 3D', '2D and 3D width')" :aria-valuenow="Math.round(split)" :aria-valuemin="25" :aria-valuemax="75" @pointerdown="resizeSplit" @pointermove="moveSplit" @pointerup="($event.currentTarget as HTMLElement).releasePointerCapture($event.pointerId)" @keydown="splitKey" @dblclick="split = 50"><span /></div>
        <section v-show="pane === '3d' || sketchPaneOpen" class="pane" :class="{ active: mode === pane, 'sketch-pane': pane === '2d' }" :aria-label="pane === '2d' ? label('2D — эскизы', '2D — sketches') : label('3D — тела', '3D — bodies')">
          <div class="pane-tools">
            <VrControls v-if="pane === '3d'" :get-scene="vrSnapshot" :available="renderBodies.some(body => body.mesh.indices.length > 0)" :locale="locale" />
            <template v-if="pane === '2d'">
              <button v-for="(name, value) in { select: label('Выбор · V', 'Select · V'), rectangle: label('Прямоугольник · R', 'Rectangle · R'), circle: label('Круг · C', 'Circle · C'), arc: label('Дуга', 'Arc'), slot: label('Паз', 'Slot'), trim: label('Обрезать','Trim'), polyline: label('Ломаная · L', 'Polyline · L') }" :key="value" class="tool-icon" :title="name" :aria-label="name" :aria-pressed="tool === value" @click="beginSketch(value)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="TOOL_ICONS[value]" /></svg></button>
              <button class="tool-icon" :title="label('Рамка','Box select')" :aria-label="label('Рамка','Box select')" :aria-pressed="boxSelect" @click="boxSelect=!boxSelect"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="TOOL_ICONS.box" /></svg></button>
              <button :title="label('Вернуться к плоскости XY','Back to the XY plane')" @click="resetWorkplane()">XY</button>

              <button class="tool-icon" :title="label('Вписать · F', 'Fit · F')" :aria-label="label('Вписать', 'Fit')" @click="fit('2d')"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5"/></svg></button>
              <button :disabled="!selectedSketch" @click="exactCardOpen=true">{{ label('Размеры','Dimensions') }}</button>
              <button class="tool-icon" :title="label('Скрыть панель эскизов', 'Hide sketch pane')" :aria-label="label('Скрыть панель эскизов', 'Hide sketch pane')" @click="toggleSketchPane(false)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M6 6l12 12M18 6 6 18"/></svg></button>
            </template>
            <template v-else>
              <button class="tool-icon" :title="label('Тела','Bodies')" :aria-label="label('Тела','Bodies')" :aria-pressed="pickMode==='body'" @click="pickMode='body';advancedOp=null;boxSelect=false"><svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" fill-opacity="0.35" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true"><path :d="TOOL_ICONS.body" /></svg></button>
              <button class="tool-icon" :title="label('Грани','Faces')" :aria-label="label('Грани','Faces')" :aria-pressed="pickMode==='face'" @click="pickMode='face';advancedOp=null;boxSelect=false"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4z" fill="currentColor" fill-opacity="0.45" /><path :d="TOOL_ICONS.face" /></svg></button>
              <select v-if="pickMode==='face' && selectedBody" v-model.number="faceIndex" @keydown="faceSelectionKey" :disabled="topologyPending" :aria-label="label('Выбрать грань','Select face')" style="max-width:12rem" @change="edgeIndex=-1;edgeIndexes=[];openingFaces=[]">
                <option :value="-1">{{ label('Выберите грань','Choose face') }}</option>
                <option v-for="(face,i) in topology.faces" :key="i" :value="i">{{ label('Грань','Face') }} {{ i+1 }} · {{ face.center.map(v=>Number(v.toFixed(2))).join(', ') }} mm</option>
              </select>
              <button class="tool-icon" :title="label('Вершины','Vertices')" :aria-label="label('Вершины','Vertices')" :aria-pressed="pickMode==='vertex'" @click="pickMode='vertex';advancedOp=null;boxSelect=false"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7"/><path d="M4 4.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5zM20 4.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5zM12 8.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5z" fill="currentColor"/></svg></button>
              <button class="tool-icon" :title="label('Рёбра','Edges')" :aria-label="label('Рёбра','Edges')" :aria-pressed="pickMode==='edge'" @click="pickMode='edge';advancedOp=null;boxSelect=false"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true"><path :d="TOOL_ICONS.edge" /><path d="M12 11v10" stroke-width="3.5" /></svg></button>
              <button class="tool-icon" :title="label('Рамка','Box select')" :aria-label="label('Рамка','Box select')" :aria-pressed="boxSelect" @click="boxSelect=!boxSelect"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="TOOL_ICONS.box" /></svg></button>
              <span v-if="selectedBody || twoSelectedBodies" class="tool-divider" aria-hidden="true"></span>
              <div v-if="selectedBody || twoSelectedBodies" class="tool-group" role="group" :aria-label="label('Булевы операции','Boolean operations')">
                <button class="tool-icon" :title="label('Объединить тела','Union bodies')" :aria-label="label('Объединить тела','Union bodies')" :disabled="!twoSelectedBodies" @click="applyBrepBoolean('union')"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true"><path d="M9 8a5 5 0 1 0 0 10 5 5 0 0 0 0-10zM15 8a5 5 0 1 0 0 10 5 5 0 0 0 0-10z" fill="currentColor" fill-opacity=".3"/></svg></button>
                <button class="tool-icon" :title="label('Вычесть: A − B','Subtract: A − B')" :aria-label="label('Вычесть: A − B','Subtract: A − B')" @click="beginSubtract()"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true"><path d="M9 8a5 5 0 1 0 0 10 5 5 0 0 0 0-10z" fill="currentColor" fill-opacity=".3"/><path d="M15 8a5 5 0 1 0 0 10 5 5 0 0 0 0-10z" stroke-dasharray="3 2"/></svg></button>
                <button class="tool-icon" :title="label('Пересечение тел','Intersect bodies')" :aria-label="label('Пересечение тел','Intersect bodies')" :disabled="!twoSelectedBodies" @click="applyBrepBoolean('intersection')"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true"><path d="M9 8a5 5 0 1 0 0 10 5 5 0 0 0 0-10zM15 8a5 5 0 1 0 0 10 5 5 0 0 0 0-10z"/><path d="M12 8.8a5 5 0 0 0 0 8.4 5 5 0 0 0 0-8.4z" fill="currentColor" fill-opacity=".45" stroke="none"/></svg></button>
              </div>
              <span class="tool-divider" aria-hidden="true"></span>
              <div class="tool-group" role="group" :aria-label="label('Манипулятор','Manipulator')">
                <button v-for="(name, value) in { move: label('Манипулятор: двигать','Gizmo: move'), rotate: label('Манипулятор: вращать','Gizmo: rotate'), scale: label('Манипулятор: масштаб','Gizmo: scale') }" :key="value" class="tool-icon" :title="name" :aria-label="name" :aria-pressed="gizmoMode===value" @click="gizmoMode=value"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="TOOL_ICONS[value]" /></svg></button>
              </div>
              <span class="tool-divider" aria-hidden="true"></span>
              <button class="tool-icon" :title="label('Обзор','Orbit')" :aria-label="label('Обзор','Orbit')" :aria-pressed="!movingBody" @click="movingBody = false"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="TOOL_ICONS.orbit" /></svg></button>
              <button class="tool-icon" :title="label('Двигать тело · G','Move body · G')" :aria-label="label('Двигать · G','Move · G')" :aria-pressed="movingBody" @click="movingBody = true"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="TOOL_ICONS.grip" /></svg></button>
              <button class="tool-icon" :aria-pressed="sketchPaneOpen" :title="label('Панель эскизов 2D', '2D sketch pane')" :aria-label="label('Панель эскизов 2D', '2D sketch pane')" @click="toggleSketchPane()"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M10 4v16"/></svg></button>
              <button class="tool-icon" :title="label('Вписать · F', 'Fit · F')" :aria-label="label('Вписать', 'Fit')" @click="fit('3d')"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5"/></svg></button>
              <button :title="label('Изометрия и вписать','Isometric view and fit')" @click="camera = defaultDirectCamera(); fit('3d')">ISO</button>
              <button class="tool-icon" :aria-pressed="smoothDisplay" :title="label('Гладкий показ B-rep', 'Smooth B-rep display')" :aria-label="label('Гладкий показ B-rep', 'Smooth B-rep display')" @click="smoothDisplay = !smoothDisplay"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path d="M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z"/><path d="M8 9c1 3 2 3 4 3s3 0 4-3" stroke-opacity=".5"/></svg></button>
              <button class="tool-icon" :aria-pressed="floorVisible" :title="label('Сетка 3D', '3D grid')" :aria-label="label('Сетка 3D', '3D grid')" @click="floorVisible = !floorVisible"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="TOOL_ICONS.grid" /></svg></button>
              <span class="subtle">{{ pickMode==='face'?label('Ctrl/⌘ + клик — несколько открытых граней','Ctrl/⌘ click — multiple openings'):label('ЛКМ/ПКМ — вращать · Shift — панорама', 'Drag to orbit · Shift to pan') }}</span>
              <button class="tool-icon" :disabled="!selectedBody" :title="label('Скачать STL выбранного тела','Download selected body as STL')" :aria-label="label('Скачать STL','Download STL')" @click="run(() => download(exportPolygonStl(selectedBody!.mesh), 'body.stl'))"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="TOOL_ICONS.download" /></svg><span class="tool-tag">STL</span></button>
              <select v-model="bodyExportFormat" :aria-label="label('Формат экспорта тела', 'Body export format')"><option v-for="format in MESH_EXPORT_FORMATS" :key="format" :value="format">{{ MESH_FORMAT_LABELS[format] }}</option></select>
              <button class="tool-icon" :disabled="!selectedBody" :title="label('Скачать выбранное тело в выбранном формате','Download the selected body in the chosen format')" :aria-label="label('Скачать', 'Download')" @click="downloadBody"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="TOOL_ICONS.download" /></svg></button></template>
          </div>
          <ModelingGridControls :locale="locale" snapping />
          <div class="canvas-wrap" :class="{'profile-preparation-pane':pane==='2d' && (isProfileCommand(advancedOp)||advancedOp==='offset')}">
            <div class="canvas-viewport">
            <canvas v-if="pane === '3d'" :ref="mountGpuCanvas" class="gpu-layer" :style="{visibility:gpuActive?'visible':'hidden'}" aria-hidden="true"></canvas>
            <svg @pointerdown.capture="touchDown($event, pane)" @pointermove.capture="touchMove" @pointerup.capture="touchEnd" :viewBox="viewBox(pane)" tabindex="0" :aria-label="pane === '2d' ? label('Холст эскизов 2D', '2D sketch canvas') : label('Холст тел 3D', '3D body canvas')" @contextmenu.prevent @wheel.prevent="wheelZoom($event, pane)" @pointerdown="downAt($event, pane)" @pointermove="moveAt" @pointerleave="gpuActive && (hovered = '')" @pointerup="up" @pointercancel="cancelInputGesture" @lostpointercapture="touchPointers.has($event.pointerId) ? cancelInputGesture() : !multiTouch && (gesture || manipulatorDrag || heightDrag || orbitDrag || curveDrag || cvDrag || vertexDrag || selectionBox) && cancelGesture()" @dragstart.prevent @selectstart.prevent @mousedown.prevent draggable="false">
              <defs><pattern :id="'direct-grid-' + pane" :width="sketchGridStep" :height="sketchGridStep" patternUnits="userSpaceOnUse"><path :d="`M ${sketchGridStep} 0 L 0 0 0 ${sketchGridStep}`" fill="none" stroke="var(--border)" stroke-opacity=".45" stroke-width=".5" vector-effect="non-scaling-stroke" /></pattern></defs>
              <rect v-if="pane === '2d'" x="-2000000" y="-2000000" width="4000000" height="4000000" :fill="'url(#direct-grid-' + pane + ')'" />
              <ModelingFloorGrid v-if="pane === '3d' && floorVisible" :camera="camera" :size="views['3d']" :center="centers['3d']" />
              <path v-if="pane === '2d'" d="M -2000000 0 H 2000000 M 0 -2000000 V 2000000" stroke="var(--border)" vector-effect="non-scaling-stroke" />
              <g v-if="pane === '2d'">
                <g pointer-events="none" fill="#ffc45b" :font-size="views['2d']/25">
                  <template v-for="d in dimensionLabels" :key="'lines-'+d.key"><polyline v-for="(line,i) in d.lines" :key="i" :points="line.map(p=>`${p[0]},${-p[1]}`).join(' ')" fill="none" stroke="#ffc45b" stroke-width="1" vector-effect="non-scaling-stroke" /></template>
                  <text v-for="d in dimensionLabels" :key="d.key" :x="d.point[0]" :y="-d.point[1]-views['2d']/35" text-anchor="middle">{{ d.text }}</text>
                  <template v-if="selectedSketch && exactCardOpen && !selectedSketch.analytic && !selectedSketch.retainedProfile"><text v-for="(p,i) in selectedSketch.points" :key="'point-'+i" v-show="selectedSketch.points.length<=16 || selectedSketch.dimensions?.some(d=>'a' in d && (d.a===i || d.b===i || (d.kind==='angle' && d.c===i)))" :x="p[0]+views['2d']/100" :y="-p[1]+views['2d']/75">{{ i }}</text></template>
                </g>
                <path v-for="(loop,i) in workplaneOutline" :key="'workplane-'+i" :d="'M '+loop.map(p=>project(p,'2d').join(',')).join(' L ')+' Z'" fill="var(--border)" fill-opacity=".2" stroke="var(--text-dim)" stroke-dasharray="3 3" vector-effect="non-scaling-stroke" pointer-events="none" />
                <g v-for="s in visibleSketches" v-show="!(profileBooleanOperation(advancedOp) && advancedPreview.document && surfaceInputs.includes(s.id))" :key="s.id" :pointer-events="objectSelectable(s.id)?undefined:'none'">
                  <path :d="sketchPath(s,'2d')" fill-rule="evenodd" :class="{ selected: selectedIds.includes(s.id), hovered: hovered === s.id }" @pointerenter="hovered = s.id" @pointerleave="hovered = ''" :fill="selectedIds.includes(s.id) && s.closed ? 'var(--accent)' : 'none'" fill-opacity=".08" pointer-events="all" stroke="var(--accent)" stroke-width="2" vector-effect="non-scaling-stroke" @pointerdown.stop="down($event, pane, s.id)" />
                  <template v-if="selection === s.id && tool === 'select' && !s.analytic && !s.retainedProfile"><circle v-for="(p, i) in s.points" :key="i" :cx="p[0]" :cy="-p[1]" :r="views['2d'] / 150" fill="var(--accent)" @pointerdown.stop="down($event, pane, s.id, i)" /></template>
                </g>
                <line v-if="operation === 'revolve'" :x1="revolveAxis === 'y' ? revolveOffset : -2000000" :x2="revolveAxis === 'y' ? revolveOffset : 2000000" :y1="revolveAxis === 'x' ? -revolveOffset : -2000000" :y2="revolveAxis === 'x' ? -revolveOffset : 2000000" stroke="#77eac5" stroke-dasharray="8 4" vector-effect="non-scaling-stroke" pointer-events="none" />
                <path v-if="cornerPreview.sketch" :d="'M '+cornerPreview.sketch.points.map(p=>project(p,'2d').join(',')).join(' L ')+' Z'" fill="#77eac5" fill-opacity=".1" stroke="#77eac5" stroke-width="3" vector-effect="non-scaling-stroke" pointer-events="none" />
                <circle v-if="cornerActive && selectedSketch" :cx="selectedSketch.points[cornerVertex]?.[0]" :cy="-(selectedSketch.points[cornerVertex]?.[1] ?? 0)" :r="views['2d']/80" fill="none" stroke="#ffcc77" stroke-width="2" vector-effect="non-scaling-stroke" pointer-events="none" />
                <g v-if="selectedSketch?.analytic">
                  <circle :cx="selectedSketch.analytic.center[0]" :cy="-selectedSketch.analytic.center[1]" :r="views['2d']/120" fill="#ffc977" style="cursor:move" @pointerdown.stop="startCurve($event,'center')" />
                  <circle :cx="selectedSketch.analytic.center[0]+selectedSketch.analytic.radius" :cy="-selectedSketch.analytic.center[1]" :r="views['2d']/120" fill="#77eac5" style="cursor:ew-resize" @pointerdown.stop="startCurve($event,'radius')" />
                  <circle v-for="(a,i) in (selectedSketch.analytic.kind==='arc'?[selectedSketch.analytic.start,selectedSketch.analytic.start+selectedSketch.analytic.sweep]:[])" :key="i" :cx="selectedSketch.analytic.center[0]+selectedSketch.analytic.radius*Math.cos(a*Math.PI/180)" :cy="-(selectedSketch.analytic.center[1]+selectedSketch.analytic.radius*Math.sin(a*Math.PI/180))" :r="views['2d']/110" fill="#ffb877" style="cursor:grab" @pointerdown.stop="startCurve($event,i===0?'start':'end')" />
                  <path v-for="a in [selectedSketch.analytic.start,selectedSketch.analytic.start+selectedSketch.analytic.sweep]" :key="a" :d="'M '+[selectedSketch.analytic.center[0]+selectedSketch.analytic.radius*Math.cos(a*Math.PI/180),-(selectedSketch.analytic.center[1]+selectedSketch.analytic.radius*Math.sin(a*Math.PI/180))].join(',')+' l '+[-Math.sin(a*Math.PI/180)*views['2d']/12,-Math.cos(a*Math.PI/180)*views['2d']/12].join(',')" stroke="#ffc977" stroke-dasharray="3 2" vector-effect="non-scaling-stroke" />
                </g>
                <path v-for="s in copyPreview" :key="s.id" :data-preview="advancedOp==='offset'?'offset-profile':undefined" :d="sketchPath(s,'2d',advancedSketchDisplay.get(s.id))" fill="none" stroke="#b894ff" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" pointer-events="none" />

                <polyline v-if="draft.length"
 :points="(tool==='polyline'&&draftCursor?[...draft,draftCursor]:draft).map(p => project(p, '2d').join(',')).join(' ')" fill="none" stroke="var(--accent)" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" pointer-events="none" />
              </g>
              <g v-else>
                <!-- Stateless SVG slots follow painter order; update geometry and hit identity without moving DOM nodes. -->
                <CpuOrbitCanvas :active="cpuOrbitActive" :triangles="cpuOrbitTriangles" :size="views['3d']" :left="centers['3d'][0]-views['3d']/2" :top="centers['3d'][1]-views['3d']/2" @ready="cpuOrbitReady=$event" /><template v-if="!gpuActive && !(cpuOrbitActive && cpuOrbitReady)"><template v-for="(p, slot) in polygons" :key="slot"><polygon v-if="p.preview" :data-preview-body="p.id" :points="p.points" :fill="p.preview===2?(extrusionMode==='difference'?'#ff647c':'#75e4b8'):p.id==='preview-split'?'#ffc977':'#77eac5'" :fill-opacity="p.preview===2?.28:.45" stroke="none" pointer-events="none"/><polygon v-else-if="p.surface" v-show="!advancedPreview.document || !advancedSurfaceIds.includes(p.id)" :data-surface="p.id" :points="p.points" :fill="selectedIds.includes(p.id)?'#8061bd':'#315f72'" fill-opacity=".72" stroke="none" :pointer-events="objectSelectable(p.id)?undefined:'none'" @pointerdown.stop="pickObject(p.id,'3d')"/><polygon v-else :opacity="p.material?.opacity??1" :data-body="p.id" :data-triangle="p.triangle" v-show="!advancedPreview.document || !advancedBodyIds.has(p.id)" :points="p.points" :style="{color:polygonColor(p)}" fill="currentColor" :stroke="(p.material?.opacity??1)<1?'none':'currentColor'" :pointer-events="objectSelectable(p.id)?undefined:'none'" stroke-width="1" stroke-linejoin="round" @pointerenter="hovered = p.id" @pointerleave="hovered = ''" vector-effect="non-scaling-stroke" @pointerdown.stop="down($event, pane, p.id, null, p.triangle)" /></template></template>
                <polygon v-show="!advancedPreview.document || !advancedSurfaceIds.includes(p.id)" pointer-events="none" v-for="p in gpuActive?nurbsSurfacePolygons:[]" :data-surface="p.id" :key="'surface-'+p.key" :points="p.points" fill="transparent" stroke="none" />
                <polyline :pointer-events="objectSelectable(curve.id)?undefined:'none'" v-for="curve in nurbsCurvePaths" v-show="!(advancedOp==='nurbs-curve-match' && advancedPreview.document && curve.id===surfaceInputs[1])" :key="'curve-'+curve.id" :data-diagnostic="curve.id===patchBudgetCurve?'patch-budget':undefined" :points="curve.points" fill="none" :stroke="curve.id===patchBudgetCurve?'#ff647c':selectedIds.includes(curve.id)?'#ffc977':'#77eac5'" stroke-width="3" vector-effect="non-scaling-stroke" @pointerdown.stop="pickNurbsCurve($event,curve.id)" />
                <g v-if="selectedNurbs" class="nurbs-cage">
                  <polyline v-if="selectedNurbsCurve" :points="selectedNurbsCurve.curve.controlPoints.map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#ffc977" stroke-dasharray="3 3" vector-effect="non-scaling-stroke" pointer-events="none" />
                  <template v-if="selectedNurbsSurface">
                    <polyline v-for="(_,u) in selectedNurbsSurface.surface.controlPoints" :key="'u-'+u" :points="selectedNurbsSurface.surface.controlPoints[u].map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#ffc977" stroke-dasharray="3 3" vector-effect="non-scaling-stroke" pointer-events="none" />
                    <polyline v-for="(_,v) in selectedNurbsSurface.surface.controlPoints[0]" :key="'v-'+v" :points="selectedNurbsSurface.surface.controlPoints.map(row=>project(row[v],'3d').join(',')).join(' ')" fill="none" stroke="#ffc977" stroke-dasharray="3 3" vector-effect="non-scaling-stroke" pointer-events="none" />
                  </template>
                  <circle v-for="cv in nativeCage" :key="cv.u+'-'+cv.v" :cx="project(cv.point,'3d')[0]" :cy="project(cv.point,'3d')[1]" :r="views['3d']/110" :fill="cvU===cv.u&&cvV===cv.v?'#ff8b77':'#ffc977'" stroke="#2a2114" vector-effect="non-scaling-stroke" @pointerdown.stop="startCv($event,cv.u,cv.v)" />
                </g>
              </g>
              <g v-if="pane==='3d' && workplaneBodyId" pointer-events="none" fill="none" stroke="#77eac5" vector-effect="non-scaling-stroke">
                <path v-for="(loop,i) in workplaneOutline" :key="i" :d="'M '+loop.map(p=>project(worldPoint(p,activePlane),'3d').join(',')).join(' L ')+' Z'" stroke-dasharray="5 3" stroke-width="1" vector-effect="non-scaling-stroke" />
                <polyline v-if="draft.length" :points="(tool==='polyline'&&draftCursor?[...draft,draftCursor]:draft).map(p=>project(worldPoint(p,activePlane),'3d').join(',')).join(' ')" stroke-width="2" />
              </g>
              <g v-if="pane==='3d' && (advancedOp==='nurbs-match'||advancedOp==='nurbs-prepare')" pointer-events="none" data-diagnostic="surface-match-boundaries"><polyline v-for="line in surfaceMatchLines" :key="line.key" :points="line.points" fill="none" :stroke="line.color" stroke-width="3" vector-effect="non-scaling-stroke" /></g>
              <g v-if="pane === '3d' && extrusionHandle" class="height-handle" @pointerdown.stop="dragHeight">
                <line :x1="extrusionHandle.base[0]" :y1="extrusionHandle.base[1]" :x2="extrusionHandle.top[0]" :y2="extrusionHandle.top[1]" stroke="#77eac5" stroke-width="3" vector-effect="non-scaling-stroke" />
                <circle :cx="extrusionHandle.top[0]" :cy="extrusionHandle.top[1]" :r="views['3d']/55" fill="#77eac5" stroke="#18332d" stroke-width="2" vector-effect="non-scaling-stroke" />
                <text :x="extrusionHandle.top[0]+views['3d']/35" :y="extrusionHandle.top[1]" :font-size="views['3d']/45" fill="#77eac5" pointer-events="none" style="user-select:none">{{ height.toFixed(2) }} mm</text>
              </g>

              <g v-if="pane==='3d' && commandFailure && selectedBody && !(edgeIndexes.length && ['edge-fillet','chamfer'].includes(advancedOp??''))" class="invalid-input-geometry" pointer-events="none" aria-label="Operation input with an error">
                <polygon v-for="p in polygons.filter(p=>p.id===selectedBody?.id)" :key="'invalid-'+p.key" :points="p.points" fill="none" stroke="var(--danger, #ff6978)" stroke-width="1.5" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" />
              </g>
              <g v-if="pane==='3d'" pointer-events="none">
                <path v-for="s in document.sketches.filter(s=>objectInView(s.id))" :key="'plane-'+s.id" :d="sketchPath(s,'3d')" fill="none" stroke="#77eac5" stroke-opacity=".6" vector-effect="non-scaling-stroke" />
                <polygon v-for="p in gpuActive?[...advancedPolygons,...advancedSurfacePolygons,...(!previewReplacesTarget?ghostPolygons:[])]:[]" :key="'advanced-'+p.key" :data-preview-body="p.id" :points="p.points" fill="transparent" stroke="none" />
                <polygon v-if="splitPlanePoints" :points="splitPlanePoints" fill="#ffc977" fill-opacity=".12" stroke="#ffc977" stroke-dasharray="5 3" vector-effect="non-scaling-stroke" />
              </g>
              <circle v-if="pane==='3d' && advancedOp==='split' && gizmoCenter" :cx="project(gizmoCenter.map((x,i)=>axisVector(advanced.axis)[i]?advanced.distance:x),'3d')[0]" :cy="project(gizmoCenter.map((x,i)=>axisVector(advanced.axis)[i]?advanced.distance:x),'3d')[1]" :r="views['3d']/65" fill="#ffc977" style="cursor:grab" @pointerdown.stop="startGizmo($event,'split',advanced.axis)" />
              <g v-if="pane==='3d' && pickMode==='vertex'" :data-body-overlay="selectedBody?.id">
                <circle v-for="v in bodyVertices" :key="'vertex-'+v.i" :cx="v.x" :cy="v.y" :r="views['3d']/(vertexIndexes.includes(v.i)?90:130)" :fill="vertexIndexes.includes(v.i)?'#ffc977':'#89baff'" stroke="#14120f" stroke-width=".5" vector-effect="non-scaling-stroke" style="cursor:grab" @pointerdown="startVertexDrag($event,v.i)" />
              </g>
              <g v-if="pane==='3d' && advancedOp==='edge-fillet' && advanced.filletMode==='variable' && selectedBody?.brep && edgeIndex>=0" pointer-events="none">
                <g v-for="(vertex,end) in selectedBody.brep.edges[edgeIndex]?.vertices??[]" :key="end" :transform="`translate(${project(selectedBody.brep.vertices[vertex].point,'3d').join(' ')})`"><circle :r="views['3d']/100" fill="#ffc977"/><text :font-size="views['3d']/35" fill="#ffc977" :x="views['3d']/80">{{ end===0?'A':'B' }}</text></g>
              </g>
              <g v-if="pane==='3d' && pickMode==='edge'" :data-body-overlay="selectedBody?.id">
                <polyline v-for="edge in featureEdges" :key="edge.id" :data-topology-edge="edge.id" role="button" tabindex="0" :aria-label="label('Ребро ','Edge ')+(edge.i+1)" :aria-pressed="edgeIndexes.includes(edge.i)" @keydown.enter.stop.prevent="pickEdge($event,edge.i)" @keydown.space.stop.prevent="pickEdge($event,edge.i)" :points="edge.points" fill="none" :stroke="edgeIndexes.includes(edge.i)?(advancedPreview.error && ['edge-fillet','chamfer'].includes(advancedOp??'')?'#f87171':'#ffc977'):'#89baff'" :stroke-width="edgeIndexes.includes(edge.i)?5:3" vector-effect="non-scaling-stroke" @pointerdown.stop="pickEdge($event,edge.i)" />
              </g>
              <g v-if="pane==='3d' && !advancedOp && pickMode==='body'" :data-body-overlay="selectedIds.join(' ')">
                <g v-for="axis in gizmoAxes" :key="axis.axis">
                  <polyline v-if="gizmoMode==='rotate'" :points="axis.ring" fill="none" :stroke="axis.color" stroke-width="3" vector-effect="non-scaling-stroke" style="cursor:grab" @pointerdown.stop="startGizmo($event,'rotate',axis.axis)" />
                  <g v-else style="cursor:grab" @pointerdown.stop="startGizmo($event,gizmoMode,axis.axis)">
                    <line :x1="axis.base[0]" :y1="axis.base[1]" :x2="axis.tip[0]" :y2="axis.tip[1]" :stroke="axis.color" stroke-width="3" vector-effect="non-scaling-stroke" />
                    <circle :cx="axis.tip[0]" :cy="axis.tip[1]" :r="views['3d']/95" :fill="axis.color" />
                    <text :x="axis.tip[0]" :y="axis.tip[1]-views['3d']/65" :font-size="views['3d']/55" :fill="axis.color" pointer-events="none" style="user-select:none">{{ axis.axis.toUpperCase() }}</text>
                  </g>
                </g>
              </g>
              <g v-if="pane==='3d' && selectedFace && pickMode==='face' && !advancedOp" style="cursor:ns-resize" @pointerdown.stop="startGizmo($event,'push','z')">
                <circle :cx="project(selectedFace.center,'3d')[0]" :cy="project(selectedFace.center,'3d')[1]" :r="views['3d']/70" fill="#ffc977" />
              </g>
              <g v-if="pane==='2d'" pointer-events="none"><path v-for="s in advancedSketches" :key="'adv-'+s.id" :data-preview="advancedOp==='offset'?'offset-profile':undefined" :d="sketchPath(s,'2d',advancedSketchDisplay.get(s.id))" fill="none" stroke="#77eac5" stroke-width="3" vector-effect="non-scaling-stroke" /></g>
              <rect v-if="selectionBox?.pane===pane" :x="Math.min(selectionBox.start[0],selectionBox.end[0])" :y="Math.min(selectionBox.start[1],selectionBox.end[1])" :width="Math.abs(selectionBox.end[0]-selectionBox.start[0])" :height="Math.abs(selectionBox.end[1]-selectionBox.start[1])" fill="#77baff" fill-opacity=".12" stroke="#77baff" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" pointer-events="none" />

              <g v-if="!gesture && !previewingTransform" pointer-events="none" stroke="#77eac5" fill="none" stroke-opacity=".65">
                <template v-for="(target,i) in visibleKeypoints" :key="i">
                  <path v-if="pane==='3d'||target.local" :d="(()=>{const p=pane==='2d'?project(target.local!,'2d'):project(target.point,'3d'),r=views[pane]/180;return `M ${p[0]-r} ${p[1]} H ${p[0]+r} M ${p[0]} ${p[1]-r} V ${p[1]+r}`})()" vector-effect="non-scaling-stroke" />
                </template>
              </g>
              <g v-if="pane==='3d' && !shellDistanceOpen && !volumeDistanceOpen && measurement?.value" pointer-events="none" data-measurement="distance">
                <polyline :points="[measurement.value.a,measurement.value.b].map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#ffda75" stroke-width="2" vector-effect="non-scaling-stroke"/>
                <g v-for="(point,i) in [measurement.value.a,measurement.value.b]" :key="i" :transform="`translate(${project(point,'3d').join(' ')})`"><circle :r="views['3d']/120" fill="#ffda75"/><text :font-size="views['3d']/40" fill="#ffda75" :x="views['3d']/90">{{ i===0?'A':'B' }}</text></g>
              </g>
              <SolidVolumeWitness v-if="pane==='3d' && volumeContact" :points="volumeContact" :project="project" :size="views['3d']" :ru="ru"/>
              <g v-if="pane==='3d' && clearanceMeasurement?.value?.closestPoints" pointer-events="none" data-measurement="clearance">
                <polyline :points="clearanceMeasurement.value.closestPoints.map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#77eac5" stroke-width="3" vector-effect="non-scaling-stroke"/>
                <circle v-for="(point,i) in clearanceMeasurement.value.closestPoints" :key="i" :cx="project(point,'3d')[0]" :cy="project(point,'3d')[1]" :r="views['3d']/100" fill="#77eac5"/>
              </g>
              <g v-if="pane==='3d' && edgeDistance?.value" pointer-events="none" data-measurement="edge-distance">
                <polyline :points="edgeDistance.value.points.map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#f4afee" stroke-width="2" vector-effect="non-scaling-stroke"/>
                <circle v-for="(point,i) in edgeDistance.value.points" :key="i" :cx="project(point,'3d')[0]" :cy="project(point,'3d')[1]" :r="views['3d']/90" fill="#f4afee"/>
              </g>
              <g v-if="pane==='3d' && faceDistance?.value?.points" pointer-events="none" data-measurement="face-distance">
                <polyline :points="faceDistance.value.points.map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#f4afee" stroke-width="2" vector-effect="non-scaling-stroke"/>
                <circle v-for="(point,i) in faceDistance.value.points" :key="i" :cx="project(point,'3d')[0]" :cy="project(point,'3d')[1]" :r="views['3d']/90" fill="#f4afee"/>
              </g>
              <g v-if="pane==='3d' && shellDistance?.value?.points" pointer-events="none" data-measurement="shell-distance">
                <polyline :points="shellDistance.value.points.map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#f4afee" stroke-width="2" vector-effect="non-scaling-stroke"/>
                <circle v-for="(point,i) in shellDistance.value.points" :key="i" :cx="project(point,'3d')[0]" :cy="project(point,'3d')[1]" :r="views['3d']/90" fill="#f4afee"/>
              </g>
              <circle v-if="pane==='3d' && curvatureMeasurement?.value" data-measurement="curvature" :cx="project(curvatureMeasurement.value.point,'3d')[0]" :cy="project(curvatureMeasurement.value.point,'3d')[1]" :r="views['3d']/90" fill="#77eac5" pointer-events="none"/>
              <g v-if="pane==='3d' && patchGapPoints.length" data-diagnostic="patch-gap" pointer-events="none" fill="none" stroke="#ff647c" stroke-width="2">
                <polyline :points="patchGapPoints.map(p=>project(p,'3d').join(',')).join(' ')" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" />
                <circle v-for="(point,i) in patchGapPoints" :key="i" :cx="project(point,'3d')[0]" :cy="project(point,'3d')[1]" :r="views['3d']/100" vector-effect="non-scaling-stroke" />
              </g>
              <path v-if="pane==='2d' && profileBooleanOperation(advancedOp) && advancedPreview.document" data-preview="retained-profile" :d="profileLoops(advancedPreview.document.sketches.find(s=>s.id===surfaceInputs[0])!).map(loop=>'M '+loop.map(p=>[p[0],-p[1]].join(',')).join(' L ')+' Z').join(' ')" fill="#77eac520" fill-rule="evenodd" stroke="#77eac5" stroke-width="3" vector-effect="non-scaling-stroke" pointer-events="none"/>
              <g v-if="pane==='2d' && advancedOp==='profile-prepare' && profilePreparation?.value" pointer-events="none" data-diagnostic="profile-preparation">
                <polygon v-if="profilePreparation.value.report.accepted" data-preview="prepared-profile" :points="profilePreparation.value.report.points.map(p=>[p[0],-p[1]].join(',')).join(' ')" fill="#77eac520" stroke="#77eac5" stroke-width="3" vector-effect="non-scaling-stroke"/>
                <polyline v-for="segment in profilePreparation.value.report.segmentDefect?.segments??[]" :key="'defect-edge'+segment.index" data-diagnostic="profile-segment" :data-segment-index="segment.index" :points="[segment.a,segment.b].map(p=>[p[0],-p[1]].join(',')).join(' ')" fill="none" stroke="#ff647c" stroke-width="5" vector-effect="non-scaling-stroke"/>
                <circle v-for="(defect,i) in profilePreparation.value.report.defects" :key="i" :cx="defect.point[0]" :cy="-defect.point[1]" :r="views['2d']/90" fill="#ff647c"/>
                <polyline v-for="(connector,i) in profilePreparation.value.report.connectors" :key="'link'+i" :points="[connector.a,connector.b].map(p=>[p[0],-p[1]].join(',')).join(' ')" fill="none" stroke="#ffc977" stroke-width="4" vector-effect="non-scaling-stroke"/>
              </g>
              <g v-if="pane==='3d' && advancedOp==='nurbs-curve-match'" data-diagnostic="curve-match-endpoints" pointer-events="none"><g v-for="guide in curveMatchGuides" :key="guide.id"><polyline :points="guide.points" fill="none" :stroke="guide.color" stroke-width="2" stroke-dasharray="5 3" vector-effect="non-scaling-stroke"/><circle :cx="guide.x" :cy="guide.y" :r="views['3d']/(guide.id===0?85:140)" :fill="guide.color"/><text :x="guide.x+(guide.id===0?-1:1)*views['3d']/35" :y="guide.y-views['3d']/45" :fill="guide.color" :font-size="views['3d']/40">{{ guide.id===0?'A':'B' }}</text></g></g>
              <g v-if="pane==='3d' && advancedOp==='nurbs-point-trim'" pointer-events="none">
                <polyline v-if="pointTrimPreviewPoints.length" data-preview="point-trim" :points="pointTrimPreviewPoints.map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#77eac5" stroke-width="4" vector-effect="non-scaling-stroke" />
                <circle v-if="pointTrimCut" data-diagnostic="point-trim-cut" :cx="project(pointTrimCut,'3d')[0]" :cy="project(pointTrimCut,'3d')[1]" :r="views['3d']/100" fill="#77eac5" />
              </g>
              <CurveOffsetPreview v-if="pane==='3d' && currentChain" v-bind="currentChain" :project="chainProject" />
              <CurveOffsetPreview v-if="pane==='3d' && advancedOp==='nurbs-offset'" :curves="advancedPreview.document?.curves??[]" :id="loftPreviewId" :report="curveOffsetState?.report" :project="chainProject" :sample="curvePoints" />
              <polyline v-if="pane==='3d' && advancedOp==='nurbs-curve-match' && advancedPreview.document" data-preview="curve-match" :points="curvePoints(advancedPreview.document.curves!.find(c=>c.id===surfaceInputs[1])!.curve).map(p=>project([p[0],p[1],p[2]??0],'3d').join(',')).join(' ')" fill="none" stroke="#77eac5" stroke-width="3" vector-effect="non-scaling-stroke" pointer-events="none" />
              <polyline v-if="pane==='3d' && (advancedOp==='nurbs-rebuild'||advancedOp==='nurbs-reduce') && advancedPreview.document" data-preview="curve-reduction" :points="curvePoints(advancedPreview.document.curves!.find(c=>c.id===selection)!.curve).map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#77eac5" stroke-width="3" vector-effect="non-scaling-stroke" pointer-events="none" />
              <g v-if="pane==='3d' && surfaceDistance?.value" pointer-events="none" data-measurement="surface-distance">
                <polyline :points="surfaceDistance.value.points.map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#f4afee" stroke-width="2" vector-effect="non-scaling-stroke"/>
                <circle v-for="(point,i) in surfaceDistance.value.points" :key="i" :cx="project(point,'3d')[0]" :cy="project(point,'3d')[1]" :r="views['3d']/90" fill="#f4afee"/>
              </g>
              <g v-if="pane==='3d' && boundaryReport?.value" pointer-events="none" fill="none" stroke-width="2">
                <polyline v-for="side in (['a','b'] as const)" :key="side" :data-boundary-inspection="side" :points="boundaryReport.value.samples.map(s=>project(s[side],'3d').join(',')).join(' ')" :stroke="side==='a'?'#ffc45b':'#77eac5'" vector-effect="non-scaling-stroke" />
                <line v-for="(s,i) in boundaryReport.value.samples" :key="i" :x1="project(s.a,'3d')[0]" :y1="project(s.a,'3d')[1]" :x2="project(s.b,'3d')[0]" :y2="project(s.b,'3d')[1]" stroke="#ff647c" stroke-width="1" vector-effect="non-scaling-stroke" />
              </g>
              <g v-if="pane==='3d' && faceContactPoint" data-diagnostic="face-contact" :data-faces="faceContactSelected?.faces.join(',')" pointer-events="none"><circle :cx="project(faceContactPoint,'3d')[0]" :cy="project(faceContactPoint,'3d')[1]" :r="views['3d']/100" fill="#ff647c" stroke="white" stroke-width="2" vector-effect="non-scaling-stroke" /></g>
              <g v-if="pane==='3d' && boundaryLine" data-diagnostic="boundary-agreement" :data-edge="boundaryLine.edge" pointer-events="none"><polyline :points="boundaryLine.points.map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#ff647c" stroke-width="5" vector-effect="non-scaling-stroke" /></g>
              <g v-if="pane==='3d' && diagnostics?.value" pointer-events="none" fill="none" stroke-width="2">
                <polyline v-for="(defect,i) in diagnostics.value.defectLines" :key="'defect-'+i" :data-diagnostic="defect.kind" :points="defect.points.map(p=>project(p,'3d').join(',')).join(' ')" :stroke="defect.kind==='orientation'?'#c18cff':defect.kind==='non-manifold'?'#ff8c42':'#ff647c'" vector-effect="non-scaling-stroke" />
                <circle v-for="(defect,i) in diagnostics.value.defectLines.filter(d=>d.kind==='degenerate')" :key="'degenerate-marker-'+i" data-diagnostic="degenerate-marker" :cx="project(defect.points[0],'3d')[0]" :cy="project(defect.points[0],'3d')[1]" :r="views['3d']/110" stroke="#ff647c" vector-effect="non-scaling-stroke" />
                <circle v-for="(point,i) in collapsedSectionPoints" :key="'collapsed-'+i" data-diagnostic="precision" :cx="project(point,'3d')[0]" :cy="project(point,'3d')[1]" :r="views['3d']/110" stroke="#ff647c" vector-effect="non-scaling-stroke" />
                <polyline v-for="(contour,i) in diagnostics.value.section.contours" :key="'section-'+i" data-diagnostic="section" :points="contour.points.map(p=>project(p,'3d').join(',')).join(' ')" stroke="#ffc45b" vector-effect="non-scaling-stroke" />

              </g>
              <g v-if="pane==='3d' && intersectionInspection?.value" pointer-events="none" fill="none" stroke-width="2">
                <polyline v-for="(points,i) in intersectionInspection.value.lines" :key="'intersection-'+i" data-diagnostic="intersection" :points="points.map(p=>project(p,'3d').join(',')).join(' ')" stroke="#ff38d1" vector-effect="non-scaling-stroke" />
                <polyline v-for="(points,i) in selectedIntersectionLines" :key="'selected-contact-'+i" data-diagnostic="intersection-selected" :points="points.map(p=>project(p,'3d').join(',')).join(' ')" stroke="#ffc45b" stroke-width="4" vector-effect="non-scaling-stroke" />
                <circle v-if="selectedIntersection" data-diagnostic="intersection-point" :cx="project(selectedIntersection.point,'3d')[0]" :cy="project(selectedIntersection.point,'3d')[1]" :r="views['3d']/100" fill="#ffc45b" />
              </g>
              <g v-if="snapMarker && snapPane === pane" pointer-events="none" stroke="#77eac5" fill="none">
                <polyline v-if="snapGuide" :points="snapGuide.map(p=>p.join(',')).join(' ')" stroke-dasharray="5 4" vector-effect="non-scaling-stroke" />
                <circle :cx="snapMarker[0]" :cy="pane==='2d'?-snapMarker[1]:snapMarker[1]" :r="views[pane]/100" vector-effect="non-scaling-stroke" />
                <text :x="snapMarker[0]+views[pane]/60" :y="(pane==='2d'?-snapMarker[1]:snapMarker[1])-views[pane]/60" :font-size="views[pane]/50" fill="#77eac5" stroke="none">{{ snapLabel }}</text>
              </g>
              <foreignObject v-if="pane==='3d' && inlineDimensionVisible && commandAnchor" :x="commandAnchor[0]+views['3d']/30" :y="commandAnchor[1]-views['3d']/24" :width="views['3d']/4" :height="views['3d']/12" @pointerdown.stop @wheel.stop>
                <label class="gizmo-dimension" :class="{ failed: commandFailure }" :style="{ fontSize: views['3d']/36+'px' }"><CadQuantityInput v-model="inlineDimension" :locale="locale" @validity="quantityValidity('inline', $event)" step="0.5" :aria-label="label('Размер у манипулятора, мм','Dimension at manipulator, mm')" /> mm</label>
              </foreignObject>
            </svg>
            <div class="zoom-tools"><button :aria-label="label('Приблизить ', 'Zoom in ') + pane" @click="zoom(pane, .8)">+</button><button :aria-label="label('Отдалить ', 'Zoom out ') + pane" @click="zoom(pane, 1.25)">−</button></div>
            <div v-if="pane === '3d'" class="fps-badge" :class="{ low: fps > 0 && fps < 30 }" role="status" :aria-label="label('Кадров в секунду', 'Frames per second')">{{ fps }} FPS · {{ frameMs }} ms<template v-if="gpuActive"> · draw {{ drawMs }} ms</template></div>
            </div>
            <div v-if="advancedOp && pane === (['offset','extend','curve','profile-prepare','profile-union','profile-difference','profile-intersection'].includes(advancedOp) ? '2d' : '3d')" class="operation-card">
              <strong>{{ ({'nurbs-offset':label('Смещение NURBS-кривой','Offset NURBS curve'),'nurbs-point-trim':label('Обрезать NURBS по точке','Trim NURBS at point'),'profile-difference':label('Вычесть области профилей','Subtract profile regions'),'profile-intersection':label('Пересечь точные профили','Intersect exact profiles'),'profile-union':label('Объединить точные профили','Union exact profiles'),'profile-prepare':label('Собрать профиль','Prepare profile'),'nurbs-curve-match':label('Согласовать кривые G1','Match curves G1'),'nurbs-prepare':label('Подготовить границы','Prepare boundaries'),'nurbs-match':label('Согласовать поверхности G1/G2','Match surfaces G1/G2'),'instance-transform':label('Преобразовать экземпляр','Transform instance'),'instance-create':label('Создать связанный экземпляр','Create linked instance'),'instance-place':label('Разместить экземпляр','Place instance'),'nurbs-surface-rebuild':label('Перестроить поверхность','Rebuild surface'),'nurbs-rebuild':label('Перестроить кривую','Rebuild curve'),'nurbs-patch':label('Coons patch','Coons patch'),'nurbs-surface-reduce':label('Снизить степень поверхности','Reduce surface degree'),'nurbs-reduce':label('Снизить степень кривой','Reduce curve degree'),'nurbs-loft':label('Поверхность по сечениям','NURBS loft'),'nurbs-sweep':label('Перенос профиля по пути','Sweep'),loft:label('Линейчатый B-rep loft','Ruled B-rep loft'),push:label('Сдвиг грани','Push / Pull'),chamfer:label('Фаска ребра','Edge chamfer'),'edge-fillet':label('Скругление ребра','Edge fillet'),shell:label('Полое тело','Shell'),split:label('Разрез плоскостью','Plane split'),offset:label('Отступ контура','Offset'),extend:label('Продлить линию','Extend'),curve:label('Окружность / дуга','Circle / arc'),transform:label('Преобразовать выбор','Transform selection')})[advancedOp] }}</strong>
              <small v-if="advancedPreview.error" role="alert">{{ advancedPreview.error }}</small>
              <small v-if="selectedBody?.brep && (advancedOp==='chamfer' || advancedOp==='edge-fillet' && advanced.filletMode==='constant')">{{ label('Точный B-rep: скругление — полные выпуклые продольные цепочки призмы/корпуса или круговые рёбра фланца; фаска — связанные прямые выпуклые рёбра. Shift добавляет рёбра. При отказе уменьшите радиус или измените набор рёбер.', 'Exact B-rep: fillet complete convex longitudinal chains of a prism/enclosure or circular annular rims; chamfer connected straight convex edges. Shift adds edges. If refused, reduce the radius or change the edge selection.') }}</small>
              <small v-else-if="advancedOp === 'push'">{{ label('Сдвиг плоской грани выпуклого тела или торца цилиндра.', 'Move a planar face of a convex solid or a cylinder end cap.') }}</small>
              <small v-else-if="!selectedBody?.brep && ['chamfer','edge-fillet','shell'].includes(advancedOp)">{{ label('Mesh-операция для выпуклых тел с плоскими гранями.', 'Mesh operation for convex solids with planar faces.') }}</small>
              <small v-if="advancedOp==='loft'">{{ label('Выберите с Shift параллельные выпуклые эскизы с одинаковым числом вершин. Порядок выбора задаёт порядок сечений.','Shift-select parallel convex sketches with matching vertex counts. Selection order defines section order.') }}</small>
              <template v-if="advancedOp==='nurbs-prepare'">
                <label v-for="(role,i) in [label('A · первая','A · first'),label('B · вторая','B · second')]" :key="role">{{ role }}<select v-model="surfaceInputs[i]" :aria-label="role"><option v-for="item in document.surfaces" :key="item.id" :value="item.id">{{ item.name }}</option></select></label>
                <label>{{ label('Граница A','Boundary A') }}<select v-model="advanced.matchBoundaryA" :aria-label="label('Граница A','Boundary A')"><option v-for="b in ['uMin','uMax','vMin','vMax']" :key="b">{{ b }}</option></select></label>
                <label>{{ label('Граница B','Boundary B') }}<select v-model="advanced.matchBoundaryB" :aria-label="label('Граница B','Boundary B')"><option v-for="b in ['uMin','uMax','vMin','vMax']" :key="b">{{ b }}</option></select></label>
                <label><input v-model="advanced.matchReverse" type="checkbox">{{ label('Обратить направление B','Reverse B direction') }}</label>
                <label><input v-model="advanced.prepareOpenPeriodic" type="checkbox" :aria-label="label('Снять периодическую связь','Remove periodic linkage')">{{ label('Снять периодическую связь','Remove periodic linkage') }}</label>
                <small v-if="advanced.prepareOpenPeriodic">{{ label('Форма замкнутого шва сохраняется в допуске. После применения его крайние ряды можно редактировать независимо.','The closed seam shape is preserved within tolerance. Its endpoint rows can be edited independently after applying.') }}</small>
                <label class="preparation-tolerance">{{ label('Допуск подготовки, мм','Preparation tolerance, mm') }}<CadQuantityInput v-model="advanced.matchError" :locale="locale" :min="0" style="width:110px" :aria-label="label('Допуск подготовки, мм','Preparation tolerance, mm')" @validity="quantityValidity('preparationError',$event)" /></label>
                <small>{{ label('Обе поверхности получают общий базис границ и диапазон параметра 0…1. Форма сохраняется в указанном допуске. После подготовки выполните G1/G2; повторное обращение B не требуется.','Both surfaces receive a common boundary basis and parameter range 0…1. Shape is preserved within the tolerance. Run G1/G2 after preparation; B needs no second reversal.') }}</small>
                <output v-if="surfacePreparation?.value" data-testid="surface-preparation-report"><template v-if="surfacePreparation.value.report.accepted">{{ label('Допуск подтверждён','Tolerance confirmed') }} · <span style="white-space:nowrap">A ≤ {{ surfacePreparation.value.report.referenceErrorUpper?.toExponential(3) }} mm</span> · <span style="white-space:nowrap">B ≤ {{ surfacePreparation.value.report.editedErrorUpper?.toExponential(3) }} mm</span></template><template v-else>{{ preparationError(surfacePreparation.value.report.reason) }}</template></output>
              </template>
              <template v-if="profileBooleanOperation(advancedOp)">
                <label v-if="advancedOp==='profile-difference'">{{ label('Основной профиль','Target profile') }}<select :value="surfaceInputs[0]" :aria-label="label('Основной профиль','Target profile')" @change="setProfileTarget(($event.target as HTMLSelectElement).value)"><option v-for="id in surfaceInputs" :key="id" :value="id">{{ document.sketches.find(s=>s.id===id)?.name }}</option></select></label>
                <small data-testid="profile-operands">{{ advancedOp==='profile-difference'?label('Вычитаемые: ','Cutters: '):label('Входы: ','Inputs: ') }}{{ (advancedOp==='profile-difference'?surfaceInputs.slice(1):surfaceInputs).map(id=>document.sketches.find(s=>s.id===id)?.name).join(', ') }}</small>
                <small>{{ label('Кривые и отверстия сохраняются. ID — первого профиля. Размеры удаляются; Undo возвращает входы.','Curves, holes and the first profile identity are retained. Dimensions are removed; Undo restores the inputs.') }}</small>
                <small>{{ label('Прямые и круговые дуги; неоднозначные пересечения отклоняются.','Lines and circular arcs; ambiguous intersections are refused.') }}</small>
              </template>
              <CurveOffsetControls v-if="advancedOp==='nurbs-offset'" v-model:join="advanced.offsetJoin" v-model:distance="advanced.distance" v-model:tolerance="advanced.maxError" :locale="locale" :state="curveOffsetState" :on-distance-validity="valid=>quantityValidity('distance',valid)" :on-tolerance-validity="valid=>quantityValidity('maxError',valid)" />
              <template v-if="advancedOp==='profile-prepare'">
                <small>{{ label('Первый выбранный профиль сохраняет ID и свойства. Дуги сохраняют кривизну. Остальные входят в его контур. Размеры удаляются; Undo восстанавливает входы.','The first selected profile keeps its identity and properties. Arcs retain their curvature. The others merge into its contour. Dimensions are removed; Undo restores the inputs.') }}</small>
                <label class="preparation-tolerance">{{ label('Допуск разрыва, мм','Gap tolerance, mm') }}<CadQuantityInput v-model="advanced.profileGap" :locale="locale" :min="0" :max="1000000" style="width:110px" :aria-label="label('Допуск разрыва, мм','Gap tolerance, mm')" @validity="quantityValidity('profileGap',$event)" /></label>
                <output v-if="profilePreparation?.value" data-testid="profile-preparation-report">{{ profilePreparation.value.report.accepted?label('Контур замкнут. Добавлено отрезков: ','Closed contour. Added connectors: ')+profilePreparation.value.report.connectors.length:profilePreparationError(profilePreparation.value.report.reason,profilePreparation.value.report.segmentDefect?.kind) }}</output>
              </template>
              <CurvePointTrimControls v-if="advancedOp==='nurbs-point-trim'" :advanced="advanced" @update:advanced="value=>advanced={...advanced,...value}" :locale="locale" :dimension="selectedNurbsCurve?.curve.controlPoints[0]?.length??3" :pick="pointTrimPick" :cut="pointTrimCut" @numeric="pointTrimNumeric" @validity="quantityValidity" />
              <template v-if="advancedOp==='nurbs-curve-match'">
                <label v-for="(role,i) in [label('A · опорная','A · reference'),label('B · изменяемая','B · edited')]" :key="role">{{ role }}<select v-model="surfaceInputs[i]" :aria-label="role"><option v-for="item in document.curves" :key="item.id" :value="item.id">{{ item.name }}</option></select></label>
                <label>{{ label('Конец A','Endpoint A') }}<select v-model="advanced.curveEndA" :aria-label="label('Конец A','Endpoint A')"><option value="start">{{ label('Начало','Start') }}</option><option value="end">{{ label('Конец','End') }}</option></select></label>
                <label>{{ label('Конец B','Endpoint B') }}<select v-model="advanced.curveEndB" :aria-label="label('Конец B','Endpoint B')"><option value="start">{{ label('Начало','Start') }}</option><option value="end">{{ label('Конец','End') }}</option></select></label>
                <label class="preparation-tolerance">{{ label('Угловой допуск, °','Angular tolerance, °') }}<CadQuantityInput v-model="advanced.curveAngle" kind="angle" :locale="locale" :min="0" :max="89.999999" style="width:110px" :aria-label="label('Угловой допуск, °','Angular tolerance, °')" @validity="quantityValidity('curveAngle',$event)" /></label>
                <small>{{ label('Изменяются конец B и соседняя управляющая точка. Контакт точный; веса и узлы сохраняются.','Edits endpoint B and its adjacent control point. Contact is exact; weights and knots are preserved.') }}</small>
                <output v-if="curveMatch?.value" data-testid="curve-match-report"><span style="white-space:nowrap">{{ label('Угол ≤ ','Angle ≤ ') }}{{ curveMatch.value.report.angleDegreesUpper?.toExponential(3) ?? '—' }}°</span> · {{ curveMatch.value.report.accepted?label('Допуск подтверждён','Tolerance confirmed'):curveMatchError(curveMatch.value.report.reason) }}</output>
              </template>
              <template v-if="advancedOp==='nurbs-match'">
                <label v-for="(role,i) in [label('A · опорная','A · reference'),label('B · изменяемая','B · edited')]" :key="role">{{ role }}<select v-model="surfaceInputs[i]" :aria-label="role"><option v-for="item in document.surfaces" :key="item.id" :value="item.id">{{ item.name }}</option></select></label>
                <label>{{ label('Граница A','Boundary A') }}<select v-model="advanced.matchBoundaryA" :aria-label="label('Граница A','Boundary A')"><option v-for="b in ['uMin','uMax','vMin','vMax']" :key="b">{{ b }}</option></select></label>
                <label>{{ label('Граница B','Boundary B') }}<select v-model="advanced.matchBoundaryB" :aria-label="label('Граница B','Boundary B')"><option v-for="b in ['uMin','uMax','vMin','vMax']" :key="b">{{ b }}</option></select></label>
                <label>{{ label('Порядок','Order') }}<select v-model.number="advanced.matchOrder" :aria-label="label('Порядок','Order')"><option :value="1">G1</option><option :value="2">G2</option></select></label>
                <label>{{ label('Масштаб производных','Derivative scale') }}<input v-model.number="advanced.matchScale" type="number" min="0.000001" step="0.1"></label>
                <label><input v-model="advanced.matchReverse" type="checkbox">{{ label('Обратить направление B','Reverse B direction') }}</label>
                <label>{{ label('Допуск производных, мм','Derivative tolerance, mm') }}<input class="surface-match-number" v-model.number="advanced.matchError" type="number" min="0" step="0.000001"></label>
                <small>{{ label('Производные измеряются в нормализованных параметрах. Требуются совместимые узлы и подтверждённая регулярность всего шва.','Derivatives use normalized parameters. Compatible knots and confirmed regularity of the whole seam are required.') }}</small>
                <small>{{ label('Изменяемых рядов B, включая границу: ','Edited boundary B and adjacent rows: ') }}{{ advanced.matchOrder+1 }}</small>
                <output v-if="surfaceMatch?.value" data-testid="surface-match-report">{{ label('Верхняя граница ошибки: ','Error upper bound: ') }}<span style="white-space:nowrap">{{ surfaceMatch.value.report.errorUpper.toExponential(3) }} mm</span> · {{ surfaceMatch.value.report.accepted?label('Допуск подтверждён','Tolerance confirmed'):surfaceMatchError(surfaceMatch.value.report.reason) }}</output>
              </template>
              <template v-if="advancedOp==='nurbs-surface-rebuild'||advancedOp==='nurbs-rebuild'||advancedOp==='nurbs-reduce'||advancedOp==='nurbs-surface-reduce'">
                <label v-if="advancedOp==='nurbs-surface-rebuild'||advancedOp==='nurbs-surface-reduce'">{{ label('Направление','Direction') }}<select v-model="advanced.surfaceAxis" :aria-label="label('Направление','Direction')"><option value="u">U</option><option value="v">V</option></select></label>
                <label>{{ label('Новая степень','Target degree') }}<input v-model.number="advanced.reduceDegree" type="number" min="1" :max="(advancedOp==='nurbs-rebuild'||advancedOp==='nurbs-surface-rebuild')?25:(selectedNurbsSurface?(advanced.surfaceAxis==='u'?selectedNurbsSurface.surface.degreeU:selectedNurbsSurface.surface.degreeV):selectedNurbsCurve?.curve.degree??2)-1" step="1"></label>
                <label v-if="advancedOp==='nurbs-rebuild'||advancedOp==='nurbs-surface-rebuild'">{{ label('Управляющих точек','Control points') }}<input v-model.number="advanced.rebuildControls" type="number" :min="advanced.reduceDegree*(rebuildPeriodic?2:1)+1" max="256" step="1"></label>
                <small v-if="advancedOp==='nurbs-surface-rebuild'">{{ label('Перестраивается выбранное направление U/V. Периодические швы сохраняются.','Rebuilds the chosen U/V direction. Periodic seams are preserved.') }}</small>
                <small v-if="advancedOp==='nurbs-rebuild'">{{ selectedNurbsCurve?.curve.periodic?label('Периодический шов сохраняется. Число точек включает повтор первых точек и должно быть больше удвоенной степени. Оценка допуска консервативна.','The periodic seam is preserved. Control count includes repeated points and must exceed twice the degree. The deviation bound is conservative.'):label('Равномерные узлы, непериодическая кривая. Оценка допуска консервативна.','Uniform knots, non-periodic curve. The deviation bound is conservative.') }}</small>
                <small v-if="advancedOp==='nurbs-surface-rebuild'&&rebuildPeriodic">{{ label('Число точек включает повтор на шве и должно быть больше удвоенной степени.','Control count includes the seam repeat and must exceed twice the degree.') }}</small>
                <label>{{ label('Допуск отклонения, мм','Deviation tolerance, mm') }}<CadQuantityInput :aria-label="label('Допуск отклонения, мм','Deviation tolerance, mm')" v-model="advanced.maxError" :locale="locale" :min="0" @validity="quantityValidity('reduceError',$event)" /></label>
                <output v-if="nurbsReduction?.value">{{ label('Верхняя граница отклонения: ','Deviation upper bound: ')+nurbsReduction.value.certificate.hausdorffErrorUpper }} mm</output>
                <small>{{ label('Применение доступно только при подтверждённом допуске. ID и исходная геометрия в истории сохраняются.','Apply requires an accepted tolerance certificate. Identity and the original geometry in history are preserved.') }}</small>
              </template>
              <template v-if="advancedOp==='nurbs-loft'||advancedOp==='nurbs-sweep'||advancedOp==='nurbs-patch'">
                <small v-if="advancedOp==='nurbs-patch'">{{ label('Низ и верх идут вдоль U, левая и правая — вдоль V. Соедините концы и согласуйте веса.','Bottom/top follow U; left/right follow V. Connect endpoints and match weights.') }}</small>
                <CoonsPreparationControls v-if="advancedOp==='nurbs-patch'" v-model:enabled="advanced.patchPrepare" v-model:budget="advanced.patchError" :locale="locale" :reports="surfaceBuildResult?.patchReports" @validity="quantityValidity('patchError',$event)" />
                <small v-if="advancedOp!=='nurbs-patch' &amp;&amp; (advancedOp!=='nurbs-sweep'||advanced.sweepMode==='translation')">{{ advancedOp==='nurbs-sweep'?label('Профиль переносится вдоль пути с неизменной ориентацией. Результат — открытая NURBS-поверхность.','The profile translates along the path with fixed orientation. The result is an open NURBS surface.'):label('Сечения соединяются линейчато в указанном порядке. Степени и узлы согласуются без изменения кривых.','Sections are joined with ruled spans in the given order. Degrees and knots are aligned without changing the curves.') }}</small>
                <template v-if="advancedOp==='nurbs-sweep'">
                  <label>{{ label('Ориентация профиля','Profile orientation') }}<select v-model="advanced.sweepMode" aria-label="Sweep orientation" @change="quantityValidity('sweepDeviation',true)"><option value="translation">{{ label('Постоянная','Fixed') }}</option><option value="framed">{{ label('По касательной пути','Follow path tangent') }}</option></select></label>
                  <template v-if="advanced.sweepMode==='framed'">
                    <label>{{ label('Число сечений','Sections') }}<input v-model.number="advanced.sweepSections" type="number" min="2" max="32" step="1" aria-label="Sweep sections"></label>
                    <label>{{ label('Предел отклонения, мм','Deviation budget, mm') }}<CadQuantityInput :aria-label="label('Предел отклонения, мм','Deviation budget, mm')" v-model="advanced.sweepDeviation" kind="length" :locale="locale" :min="0" @validity="quantityValidity('sweepDeviation',$event)" /></label>
                    <label>{{ label('Начальное направление X','Initial direction X') }}<input v-model.number="advanced.sweepNormalX" type="number" aria-label="Sweep normal X"></label>
                    <label>{{ label('Начальное направление Y','Initial direction Y') }}<input v-model.number="advanced.sweepNormalY" type="number" aria-label="Sweep normal Y"></label>
                    <label>{{ label('Начальное направление Z','Initial direction Z') }}<input v-model.number="advanced.sweepNormalZ" type="number" aria-label="Sweep normal Z"></label>
                    <small v-if="advancedPreview.sweepReport" data-diagnostic="sweep-refinement">{{ label('Измеренное отклонение: ','Sampled deviation: ') }}{{ advancedPreview.sweepReport.sampledControlDeviation.toPrecision(4) }} {{ label('мм; проверено сечений: ','mm; checked stations: ') }}{{ advancedPreview.sweepReport.stations }}<template v-if="advancedPreview.sweepReport.closedPath"> · {{ label('Замкнутый шов C0','Closed C0 seam') }}</template></small>
                    <small>{{ label('Направление не должно быть параллельно начальной касательной. Проверка сравнивает сечения с вчетверо более частым построением; непрерывный допуск не подтверждается. Замкнутый шов C0; разрывы касательной отклоняются.','The direction must not be parallel to the initial tangent. The check compares sections against fourfold refinement; it does not certify continuous tolerance. Closed seams are C0; tangent discontinuities are refused.') }}</small>
                  </template>
                </template>
                <label v-for="(_,i) in surfaceInputs" :key="i">{{ surfaceInputLabel(i) }}<select v-model="surfaceInputs[i]" :aria-label="surfaceInputLabel(i)" @keydown="surfaceInputKey($event,i)"><option v-for="curve in document.curves" :key="curve.id" :value="curve.id">{{ curve.name }}</option></select></label>
                <label v-for="(_,i) in surfaceInputs" :key="'reverse-'+i"><input type="checkbox" v-model="surfaceReversed[i]">{{ label('Развернуть: ','Reverse: ')+surfaceInputLabel(i) }}</label>
                <button v-if="advancedOp!=='nurbs-patch'" @click="surfaceInputs=[...surfaceInputs].reverse();surfaceReversed=[...surfaceReversed].reverse()">{{ label('Обратить порядок входов','Reverse input order') }}</button>
              </template>
              <small v-if="advancedOp==='offset' && selectedSketch?.retainedProfile">{{ label('Круглые сопряжения. Положительное значение добавляет материал, отрицательное удаляет. Отверстия изменяются вместе с областью.','Round joins. Positive values expand material; negative values shrink it. Holes follow the region offset.') }}</small>
              <label v-if="['push','shell','split','offset'].includes(advancedOp)">{{ advancedOp==='shell'?label('Толщина, мм','Thickness, mm'):label('Расстояние, мм','Distance, mm') }}<CadQuantityInput :aria-label="advancedOp==='shell'?label('Толщина, мм','Thickness, mm'):label('Расстояние, мм','Distance, mm')" v-model="advanced.distance" kind="length" :locale="locale" @validity="quantityValidity('advanced.distance', $event)" step=".5" /></label>
              <label v-if="advancedOp==='edge-fillet' && selectedBody?.brep">{{ label('Тип скругления','Fillet type') }}<select v-model="advanced.filletMode" :aria-label="label('Тип скругления','Fillet type')" @change="quantityValidity('endRadius',true)"><option value="constant">{{ label('Постоянный радиус','Constant radius') }}</option><option value="variable">{{ label('Радиус A → B','Radius A → B') }}</option><option value="corner">{{ label('Угол трёх рёбер','Three-edge corner') }}</option></select></label>
              <small v-if="advancedOp==='edge-fillet' && selectedBody?.brep && advanced.filletMode==='variable'">{{ label('Один вертикальный край осевого параллелепипеда. Радиус меняется линейно от A к B; оба радиуса положительны и различны.','One vertical edge of an axis-aligned cuboid. Radius varies linearly from A to B; both radii must be positive and different.') }}</small>
              <small v-if="advancedOp==='edge-fillet' && selectedBody?.brep && advanced.filletMode==='corner'">{{ label('Три ребра у вершины с максимальными X, Y, Z осевого параллелепипеда. Общий радиус; сферическое сопряжение.','Three edges at the maximum X, Y, Z corner of an axis-aligned cuboid. Equal radius with a spherical corner blend.') }}</small>
              <label v-if="advancedOp==='edge-fillet' && selectedBody?.brep && advanced.filletMode==='variable'">{{ label('Радиус B, мм','Radius B, mm') }}<CadQuantityInput :aria-label="label('Радиус B, мм','Radius B, mm')" v-model="advanced.endRadius" kind="length" :locale="locale" :min=".01" @validity="quantityValidity('endRadius',$event)" /></label>
              <label v-if="['edge-fillet','chamfer','curve'].includes(advancedOp)">{{ advancedOp==='edge-fillet' && selectedBody?.brep && advanced.filletMode==='variable'?label('Радиус A, мм','Radius A, mm'):label('Радиус / размер, мм','Radius / size, mm') }}<CadQuantityInput :aria-label="advancedOp==='edge-fillet' && selectedBody?.brep && advanced.filletMode==='variable'?label('Радиус A, мм','Radius A, mm'):label('Радиус / размер, мм','Radius / size, mm')" v-model="advanced.radius" kind="length" :locale="locale" @validity="quantityValidity('advanced.radius', $event)" :min=".01" step=".5" /></label>
              <label v-if="advancedOp==='edge-fillet' && !selectedBody?.brep">{{ label('Грани скругления','Fillet segments') }}<input v-model.number="filletSegments" type="number" min="2" max="32" step="1"></label>
              <label v-if="advancedOp==='split'||advancedOp==='transform'||advancedOp==='instance-transform'">{{ label('Ось','Axis') }}<select v-model="advanced.axis"><option>x</option><option>y</option><option>z</option></select></label>
              <small v-if="advancedOp==='split'">{{ label('Обе части сохраняются отдельными телами. Пунктир — плоскость разреза.', 'Both halves remain separate bodies. The dashed outline is the cutting plane.') }}</small>
              <small v-if="advancedOp==='shell'">{{ label('Открытые грани:','Open faces:') }} {{ openingFaces.map(i=>i+1).join(', ') }}</small>
              <template v-if="advancedOp==='curve'">
                <label>{{ label('Центр X','Center X') }}<CadQuantityInput :aria-label="label('Центр X','Center X')" v-model="advanced.cx" kind="length" :locale="locale" @validity="quantityValidity('advanced.cx', $event)" /></label><label>{{ label('Центр Y','Center Y') }}<CadQuantityInput :aria-label="label('Центр Y','Center Y')" v-model="advanced.cy" kind="length" :locale="locale" @validity="quantityValidity('advanced.cy', $event)" /></label>
                <label>{{ label('Начальный угол','Start angle') }}<CadQuantityInput :aria-label="label('Начальный угол','Start angle')" v-model="advanced.start" kind="angle" :locale="locale" @validity="quantityValidity('advanced.start', $event)" /></label>
                <label v-if="selectedSketch?.analytic?.kind==='arc'">{{ label('Угол дуги','Arc sweep') }}<CadQuantityInput :aria-label="label('Угол дуги','Arc sweep')" v-model="advanced.sweep" kind="angle" :locale="locale" @validity="quantityValidity('advanced.sweep', $event)" :min="-360" :max="360" /></label>
              </template>
              <label v-if="advancedOp==='extend'">{{ label('Конец','Endpoint') }}<select v-model="advanced.end"><option value="start">{{ label('Начало','Start') }}</option><option value="end">{{ label('Конец','End') }}</option></select></label>
              <template v-if="advancedOp==='instance-create'||advancedOp==='instance-place'">
                <small>{{ label('Смещение относительно источника. Правки источника обновляют экземпляр.','Offset from the source. Editing the source updates the instance.') }}</small>
                <label>X<CadQuantityInput aria-label="X" v-model="advanced.x" :locale="locale" @validity="quantityValidity('instance.x',$event)" /></label>
                <label>Y<CadQuantityInput aria-label="Y" v-model="advanced.y" :locale="locale" @validity="quantityValidity('instance.y',$event)" /></label>
                <label>Z<CadQuantityInput aria-label="Z" v-model="advanced.z" :locale="locale" @validity="quantityValidity('instance.z',$event)" /></label>
              </template>
              <template v-if="advancedOp==='transform'||advancedOp==='instance-transform'">
                <small v-if="advancedOp==='instance-transform'">{{ label('Относительное преобразование вокруг центра габаритов экземпляра. Связь с источником сохраняется.','Relative transform about the instance bounds center. The source link is preserved.') }}</small>
                <label>X<CadQuantityInput aria-label="X" v-model="advanced.x" kind="length" :locale="locale" @validity="quantityValidity('advanced.x', $event)" /></label><label>Y<CadQuantityInput aria-label="Y" v-model="advanced.y" kind="length" :locale="locale" @validity="quantityValidity('advanced.y', $event)" /></label><label>Z<CadQuantityInput aria-label="Z" v-model="advanced.z" kind="length" :locale="locale" @validity="quantityValidity('advanced.z', $event)" /></label>
                <label>{{ label('Поворот, °','Rotation, °') }}<CadQuantityInput :aria-label="label('Поворот, °','Rotation, °')" v-model="advanced.angle" kind="angle" :locale="locale" @validity="quantityValidity('advanced.angle', $event)" /></label><label>{{ label('Масштаб','Scale') }}<CadQuantityInput :aria-label="label('Масштаб','Scale')" v-model="advanced.scale" kind="scalar" :locale="locale" @validity="quantityValidity('advanced.scale', $event)" :min=".01" step=".1" /></label>
              </template>
              <small v-if="bodyEditPending" role="status">{{ label('Вычисление предпросмотра… Esc — отмена.', 'Calculating preview… Esc to cancel.') }}</small>
              <button v-if="bodyEditRetryVisible" type="button" :disabled="bodyEditPending || Object.keys(invalidQuantities).length>0" @click="workspace?.focus(); bodyEditRevision++">{{ label('Повторить вычисление','Retry calculation') }}</button>
              <div><button class="primary" :disabled="!commandReady" @click="applyCommand">{{ label('Готово · Enter','Apply · Enter') }}</button><button @click="cancelCommand">Esc</button></div>
            </div>

            <div v-if="pane==='3d' && selectedNurbs && (!commandActive || pointEditActive || nativeNurbsPending)" class="operation-card nurbs-card">
              <CurveOffsetConstructionInfo :value="selectedNurbsCurve?.offsetConstruction" :locale="locale" :curves="document.curves??[]" :ids="selectedIds" @result="currentChain=$event" />
              <button v-if="selectedCurvePair || selectedSurfacePair" class="primary" @click="matchSelectedG1">{{ selectedSurfacePair?label('G1/G2: A → B','G1/G2: A → B'):label('G1: вторую к первой','G1: match second to first') }}</button>
              <section v-if="selectedSurfacePair" aria-label="surface-distance" class="body-diagnostics">
                <button @click="surfaceDistanceOpen=!surfaceDistanceOpen" :aria-pressed="surfaceDistanceOpen">{{ label('Расстояние между поверхностями','Distance between surfaces') }}</button>
                <template v-if="surfaceDistanceOpen">
                  <label>{{ label('Объём расчёта','Calculation budget') }}<select v-model.number="surfaceDistanceBudget" :aria-label="label('Объём расчёта поверхностей','Surface calculation budget')"><option :value="10000">{{ label('Обычный','Standard') }}</option><option :value="100000">{{ label('Расширенный','Extended') }}</option></select></label>
                  <small v-if="surfaceDistancePending" role="status">{{ label('Измеряю расстояние…','Measuring distance…') }} <button @click="surfaceDistanceOpen=false">Esc</button></small>
                  <template v-if="surfaceDistance?.value">
                    <output data-surface-distance>{{ surfaceDistance.value.distanceIntervalMm.map(v=>Number(v.toPrecision(12)).toString()).join(' … ') }} mm</output>
                    <small v-if="surfaceDistance.value.converged">{{ label('Допуск расстояния достигнут: 0,001 мм.','Distance tolerance reached: 0.001 mm.') }}</small>
                    <p v-else role="status">{{ label('Расчёт не завершён: показаны нижняя и верхняя границы. Увеличьте объём расчёта или проверьте меньшие участки поверхностей.','Calculation incomplete: lower and upper bounds are shown. Increase the calculation budget or inspect smaller surface regions.') }}</p>
                    <small>{{ label('Измерены полные выбранные NURBS-поверхности.','The complete selected NURBS surfaces are measured.') }}</small>
                  </template>
                  <p v-if="surfaceDistance?.error" role="alert">{{ surfaceDistance.error }}</p>
                </template>
              </section>
              <section v-if="selectedSurfacePair" aria-label="Surface boundary inspection" class="body-diagnostics">
                <button @click="boundaryInspection=!boundaryInspection" :aria-pressed="boundaryInspection">{{ label('Проверить стык поверхностей','Inspect surface boundary') }}</button>
                <template v-if="boundaryInspection">
                  <small v-if="boundaryPending" role="status" aria-label="surface-boundary-inspection">{{ label('Проверяю стык…','Inspecting boundary…') }} <button @click="boundaryInspection=false">Esc</button></small>
                  <small>{{ selectedSurfacePair.map((id,i)=>(i?'B: ':'A: ')+document.surfaces?.find(s=>s.id===id)?.name).join(' · ') }}</small>
                  <label>{{ label('Граница A','Boundary A') }}<select v-model="boundaryOptions.boundaryA" :aria-label="label('Граница A','Boundary A')"><option v-for="b in ['uMin','uMax','vMin','vMax']" :key="b">{{ b }}</option></select></label>
                  <label>{{ label('Граница B','Boundary B') }}<select v-model="boundaryOptions.boundaryB" :aria-label="label('Граница B','Boundary B')"><option v-for="b in ['uMin','uMax','vMin','vMax']" :key="b">{{ b }}</option></select></label>
                  <label><input type="checkbox" v-model="boundaryOptions.reverse">{{ label('Обратное направление B','Reverse B direction') }}</label>
                  <label>{{ label('Допуск зазора, мм','Gap tolerance, mm') }}<input v-model.number="boundaryOptions.toleranceMm" type="number" min="0.000001" step=".01"></label>
                  <label>{{ label('Допуск угла, °','Angle tolerance, °') }}<input v-model.number="boundaryOptions.angleToleranceDeg" type="number" min="0" max="90" step=".1"></label>
                  <label>{{ label('Точек проверки','Sample count') }}<input v-model.number="boundaryOptions.samples" type="number" min="2" max="257" step="1"></label>
                  <template v-if="boundaryReport?.value">
                    <output>{{ label('Максимальный зазор: ','Maximum gap: ')+boundaryReport.value.maxGapMm.toFixed(6) }} mm</output>
                    <output>{{ label('Угол касательных плоскостей: ','Tangent-plane angle: ')+(boundaryReport.value.maxTangentPlaneAngleDeg?.toFixed(6)??label('не определён','undefined')) }}°</output>
                    <strong>{{ boundaryReport.value.sampledWithinTolerance?label('Проверенные точки в допуске','Sampled points within tolerance'):label('Допуск не подтверждён','Tolerance not confirmed') }}</strong>
                  </template>
                  <p v-if="boundaryReport?.error" role="alert">{{ boundaryReport.error }}</p>
                  <small>{{ label('Проверка по равномерной выборке, не гарантия между точками. Жёлтый — A, зелёный — B, красный — зазоры. Сравниваются касательные плоскости без учёта знака нормали.','Uniform samples do not prove agreement between points. Yellow: A; green: B; red: gaps. Tangent planes are compared without normal orientation.') }}</small>
                </template>
              </section>
              <template v-if="selectedCurvePair">
                <strong>{{ label('Переход между кривыми G2','G2 bridge curves') }}</strong>
                <label>{{ label('Конец первой','First endpoint') }}<select v-model="bridgeEndA"><option value="start">Start</option><option value="end">End</option></select></label>
                <label>{{ label('Конец второй','Second endpoint') }}<select v-model="bridgeEndB"><option value="start">Start</option><option value="end">End</option></select></label>
                <label>{{ label('Натяжение','Tension') }}<input v-model.number="bridgeTension" :aria-invalid="bridgeTensionInvalid" :aria-describedby="bridgeTensionInvalid?'bridge-tension-error':undefined" type="number" min=".01" max="10" step=".1"></label>
                <small v-if="bridgeTensionInvalid" id="bridge-tension-error" role="alert">{{ label('Введите натяжение от 0,01 до 10.','Enter tension from 0.01 to 10.') }}</small>
                <template v-if="bridgeError"><small role="alert">{{ label('Не удалось создать переход G2. Проверьте выбранные концы кривых и натяжение, затем повторите создание.','Could not create the G2 bridge. Check the selected curve endpoints and tension, then retry creation.') }}</small><details><summary>{{ label('Технические сведения','Technical details') }}</summary>{{ bridgeError }}</details></template>
                <button :aria-disabled="nativeNurbsPending||bridgeTensionInvalid" @click="addBridgeCurve">{{ label('Создать переход G2','Create G2 bridge') }}</button>
              </template>
              <label>U / CV <select v-model.number="cvU"><option v-for="(_,i) in (selectedNurbsCurve?.curve.controlPoints ?? selectedNurbsSurface?.surface.controlPoints ?? [])" :key="i" :value="i">{{ i }}</option></select></label>
              <label v-if="selectedNurbsSurface">V / CV <select v-model.number="cvV"><option v-for="(_,i) in selectedNurbsSurface.surface.controlPoints[cvU] ?? []" :key="i" :value="i">{{ i }}</option></select></label>
              <label>X <input data-cv-field="x" :aria-invalid="cvInvalidField==='x'" :aria-describedby="cvInvalidField==='x'?'cv-input-error':undefined" v-model.number="cvX" type="number" step=".5"></label>
              <label>Y <input data-cv-field="y" :aria-invalid="cvInvalidField==='y'" :aria-describedby="cvInvalidField==='y'?'cv-input-error':undefined" v-model.number="cvY" type="number" step=".5"></label>
              <label>Z <input data-cv-field="z" :aria-invalid="cvInvalidField==='z'" :aria-describedby="cvInvalidField==='z'?'cv-input-error':undefined" v-model.number="cvZ" type="number" step=".5"></label>
              <label>{{ label('Вес','Weight') }} <input data-cv-field="weight" :aria-invalid="cvInvalidField==='weight'" :aria-describedby="cvInvalidField==='weight'?'cv-input-error':undefined" v-model.number="cvWeight" type="number" min=".000001" step=".1"></label>
              <small v-if="cvInputError" id="cv-input-error" role="alert">{{ cvInputError }}</small>
              <button class="primary" :disabled="gizmoPending" @click="updateCv">{{ label('Применить CV','Apply CV') }}</button>
              <label>{{ label('Параметр узла / iso','Knot / iso parameter') }} <input v-model.number="knotValue" type="number" step=".1"></label>
              <div v-if="selectedNurbsCurve"><button :aria-disabled="nativeNurbsPending" @click="!nativeNurbsPending && insertNativeKnot('curve')">Insert knot</button><button :aria-disabled="nativeNurbsPending" @click="!nativeNurbsPending && elevateNative('curve')">Degree +1</button></div>
              <template v-if="selectedNurbsSurface">
                <div><button :aria-disabled="nativeNurbsPending" @click="!nativeNurbsPending && insertNativeKnot('u')">Knot U</button><button :aria-disabled="nativeNurbsPending" @click="!nativeNurbsPending && insertNativeKnot('v')">Knot V</button></div>
                <div><button :aria-disabled="nativeNurbsPending" @click="!nativeNurbsPending && elevateNative('u')">Degree U +1</button><button :aria-disabled="nativeNurbsPending" @click="!nativeNurbsPending && elevateNative('v')">Degree V +1</button></div>
                <div><button :aria-disabled="nativeNurbsPending" @click="!nativeNurbsPending && extractIso('u')">Iso U</button><button :aria-disabled="nativeNurbsPending" @click="!nativeNurbsPending && extractIso('v')">Iso V</button></div>
                <small>{{ label('Прямоугольная обрезка сохраняет точную NURBS-поверхность в новом диапазоне параметров.','Rectangular parameter trim keeps an exact NURBS surface over the new domain.') }}</small>
                <div class="trim-grid"><label>U min<input v-model.number="trimBounds[0]" type="number" step=".05"></label><label>U max<input v-model.number="trimBounds[1]" type="number" step=".05"></label><label>V min<input v-model.number="trimBounds[2]" type="number" step=".05"></label><label>V max<input v-model.number="trimBounds[3]" type="number" step=".05"></label></div>
                <button :aria-disabled="nativeNurbsPending" @click="!nativeNurbsPending && trimNativeSurface()">{{ label('Обрезать диапазон UV','Trim UV domain') }}</button>
                <label>U segments <input v-model.number="selectedNurbsSurface.segmentsU" type="number" min="2" max="64" @change="commit(document)"></label>
                <label>V segments <input v-model.number="selectedNurbsSurface.segmentsV" type="number" min="2" max="64" @change="commit(document)"></label>
              </template>
              <button :aria-disabled="nativeNurbsPending" v-if="selectedNurbsCurve" @click="!nativeNurbsPending && curveToSurface()">{{ label('Выдавить NURBS 10 мм','Extrude NURBS 10 mm') }}</button>
              <button :aria-disabled="nativeNurbsPending" @click="!nativeNurbsPending && bakeNurbs()">{{ selectedNurbsSurface ? label('Bake поверхности в mesh-тело','Bake surface to mesh body') : label('Копировать sampled-кривую в эскиз','Copy sampled curve to sketch') }}</button>
            </div>

            <div v-if="pane === '2d' && cornerActive" class="operation-card">
              <strong>{{ operation === 'fillet' ? label('Скругление', 'Fillet') : 'DogEar' }}</strong>
              <small>{{ label('Нажмите вершину контура для выбора угла.', 'Click a contour vertex to choose a corner.') }}</small>
              <small v-if="operation === 'dogear'">{{ label('Круглый выход в прямом углу. Радиус соответствует радиусу инструмента.', 'Circular relief at a right-angle corner. Radius is the tool radius.') }}</small>
              <label>{{ label('Вершина', 'Vertex') }}<select v-model.number="cornerVertex" :aria-label="label('Вершина', 'Vertex')"><option v-for="(_, i) in selectedSketch?.points" :key="i" :value="i">{{ i + 1 }}</option></select></label>
              <label>{{ label('Радиус, мм', 'Radius, mm') }}<CadQuantityInput :aria-label="label('Радиус, мм', 'Radius, mm')" v-model="cornerRadius" kind="length" :locale="locale" @validity="quantityValidity('cornerRadius', $event)" :min=".01" step=".5" /></label>
              <small v-if="sketchEditPending" role="status">{{ label('Вычисляется предпросмотр…','Computing preview…') }}</small>
              <small v-if="cornerPreview.error" role="alert">{{ cornerPreview.error }}</small>
              <button v-if="sketchEditRetryVisible" :disabled="sketchEditPending || Object.keys(invalidQuantities).length>0" @click="sketchEditRevision++">{{ label('Повторить вычисление','Retry calculation') }}</button>
              <div><button class="primary" :disabled="!commandReady" @click="applyCommand">{{ label('Готово · Enter', 'Apply · Enter') }}</button><button @click="cancelCommand">Esc</button></div>
            </div>
            <div v-if="pane === '2d' && drawMeasure" class="live-measure">{{ drawMeasure }}</div>
            <div v-if="pane === '3d' && solidActive" class="operation-card">
              <strong>{{ operation === 'revolve' ? label('Вращение профиля', 'Revolve profile') : label('Выдавливание', 'Extrusion') }}</strong><small v-if="operation === 'extrude'">{{ label('Тяните зелёную ручку или введите размер', 'Drag the green handle or enter a dimension') }}</small>
              <div class="segmented"><button v-for="(name,value) in {new:label('Новое','New'),union:label('Добавить','Add'),difference:label('Вычесть','Cut')}" :key="value" :aria-pressed="extrusionMode === value" @click="setExtrusionMode(value)">{{ name }}</button></div>
              <fieldset v-if="operation==='extrude' && profileChoices.length>1" class="profile-choices"><legend>{{ label('Контуры профиля', 'Profile contours') }}</legend><label v-for="sketch in profileChoices" :key="sketch.id"><input v-model="profileIds" type="checkbox" :value="sketch.id">{{ sketch.name }}</label><small>{{ label('Внутренние контуры образуют отверстия; вложенные островки сохраняются.', 'Inner contours form holes; nested islands remain solid.') }}</small></fieldset>
              <small v-if="selectedSketch?.supportBodyId">{{ label('Добавить — наружу от грани. Вычесть — внутрь тела.', 'Add extends outward from the face. Cut goes into the body.') }}</small>
              <small v-if="previewPending" role="status">{{ label('Вычисление предпросмотра… Esc — отмена.', 'Calculating preview… Esc to cancel.') }}</small>
              <small v-if="previewEmpty">{{ label('Вырез полностью удалит выбранное тело.', 'The cut will remove the entire target body.') }}</small>
              <label v-if="operation === 'extrude'">{{ label('Высота, мм', 'Height, mm') }}<CadQuantityInput :aria-label="label('Высота, мм', 'Height, mm')" v-model="height" kind="length" :locale="locale" @validity="quantityValidity('height', $event)" step="1" /></label>
              <label v-if="operation === 'extrude'">{{ label('Смещение от плоскости, мм', 'Plane offset, mm') }}<CadQuantityInput :aria-label="label('Смещение от плоскости, мм', 'Plane offset, mm')" v-model="baseZ" kind="length" :locale="locale" @validity="quantityValidity('baseZ', $event)" step="1" /></label>
              <template v-if="operation === 'revolve'">
                <label>{{ label('Ось в эскизе', 'Sketch axis') }}<select v-model="revolveAxis"><option value="y">Y · {{ label('вертикаль', 'vertical') }}</option><option value="x">X · {{ label('горизонталь', 'horizontal') }}</option></select></label>
                <label>{{ label('Смещение оси, мм', 'Axis offset, mm') }}<CadQuantityInput :aria-label="label('Смещение оси, мм', 'Axis offset, mm')" v-model="revolveOffset" kind="length" :locale="locale" @validity="quantityValidity('revolveOffset', $event)" step="1" /></label>
                <label>{{ label('Угол, °', 'Angle, °') }}<CadQuantityInput :aria-label="label('Угол, °', 'Angle, °')" v-model="revolveAngle" kind="angle" :locale="locale" @validity="quantityValidity('revolveAngle', $event)" :min="-360" :max="360" step="15" /></label>
            <label>{{ label('Поверхности вращения','Revolve surfaces') }}<select v-model="revolveGeometry" @keydown="revolveGeometryKey"><option value="faceted">{{ label('Гранёные','Faceted') }}</option><option value="exact">{{ label('Точные NURBS','Exact NURBS') }}</option></select></label>
                <label>{{ label('Сегменты', 'Segments') }}<input v-model.number="revolveSegments" type="number" min="8" max="128"></label>
                <small>{{ label('Пунктир слева — ось вращения. Контур должен лежать по одну сторону от неё.', 'The dashed line on the left is the rotation axis. Keep the profile on one side of it.') }}</small>
              </template>
              <label v-if="extrusionMode !== 'new'">{{ label('Тело', 'Body') }}<select v-model="targetBody"><option v-for="b in document.bodies" :key="b.id" :value="b.id">{{ b.name }}</option></select></label>
              <small v-if="previewError" role="alert">{{ previewError }}</small>
              <button v-if="solidPreviewRetryVisible" type="button" :disabled="previewPending || Object.keys(invalidQuantities).length>0" @click="solidPreviewRevision++">{{ label('Повторить вычисление','Retry calculation') }}</button>
              <div><button class="primary" :disabled="!commandReady" @click="applyCommand">{{ label('Готово · Enter', 'Apply · Enter') }}</button><button @click="cancelCommand">Esc</button></div>
            </div>
            <div v-if="pane === '2d' && operation === 'array'" class="operation-card">
              <strong>{{ label('Круговые копии', 'Circular copies') }}</strong><small>{{ label('Количество включает исходный эскиз', 'Count includes the original sketch') }}</small>
              <small v-if="sketchEditPending" role="status">{{ label('Вычисляется предпросмотр…','Computing preview…') }}</small>
              <small v-if="sketchEditError" role="alert">{{ sketchEditFailure }}</small>
              <button v-if="sketchEditRetryVisible" :disabled="sketchEditPending || Object.keys(invalidQuantities).length>0" @click="sketchEditRevision++">{{ label('Повторить вычисление','Retry calculation') }}</button>
              <label>{{ label('Количество', 'Count') }}<input v-model.number="copyCount" type="number" min="2" max="64"></label><label>{{ label('Угол, °', 'Angle, °') }}<CadQuantityInput :aria-label="label('Угол, °', 'Angle, °')" v-model="copySweep" kind="angle" :locale="locale" @validity="quantityValidity('copySweep', $event)" :min="-360" :max="360" /></label>
              <label>{{ label('Центр X', 'Center X') }}<CadQuantityInput :aria-label="label('Центр X', 'Center X')" v-model="copyX" kind="length" :locale="locale" @validity="quantityValidity('copyX', $event)" /></label><label>{{ label('Центр Y', 'Center Y') }}<CadQuantityInput :aria-label="label('Центр Y', 'Center Y')" v-model="copyY" kind="length" :locale="locale" @validity="quantityValidity('copyY', $event)" /></label>
              <div><button class="primary" :disabled="!commandReady" @click="applyCommand">{{ label('Готово · Enter', 'Apply · Enter') }}</button><button @click="cancelCommand">Esc</button></div>
            </div>
            <div v-if="pane === '2d' && !document.sketches.length && !draft.length" class="empty-hint"><strong>{{ label('Начните с контура', 'Start with a contour') }}</strong><span>{{ label('Выберите фигуру сверху и нарисуйте её мышью', 'Choose a tool above and draw with the mouse') }}</span></div>
            <div v-if="pane === '3d' && !document.bodies.length && !document.curves?.length && !document.surfaces?.length && !solidActive" class="empty-hint"><strong>{{ label('Из плоской фигуры — в объём', 'Turn a flat shape into a solid') }}</strong><span>{{ label('Выберите эскиз слева и нажмите «Выдавить»', 'Select a sketch on the left and press Extrude') }}</span></div>
            <div v-if="pane === '3d' && subtract" class="operation-card subtract-card" role="dialog" :aria-label="label('Вычитание', 'Subtraction')">
              <strong>{{ label('Вычесть: A − B', 'Subtract: A − B') }}</strong>
              <button type="button" class="subtract-field" :aria-pressed="subtract.active === 'a'" @click="subtract.active = 'a'">
                <span>{{ label('A · из чего вычитаем', 'A · subtract from') }}</span>
                <small>{{ subtractNames(subtract.a) }}</small>
              </button>
              <button type="button" class="subtract-field" :aria-pressed="subtract.active === 'b'" @click="subtract.active = 'b'">
                <span>{{ label('B · что вычитаем', 'B · subtract') }}</span>
                <small>{{ subtractNames(subtract.b) }}</small>
              </button>
              <small>{{ label('Кликайте по телам, Shift добавляет. Enter выполняет, Esc отменяет.', 'Click bodies, Shift adds. Enter runs, Esc cancels.') }}</small>
              <div>
                <button type="button" @click="cancelCommand">{{ label('Отмена', 'Cancel') }}</button>
                <button class="primary" type="button" :disabled="!commandReady" @click="applyCommand">OK</button>
              </div>
            </div>
          </div>
        </section>
      </template>
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
          <div class="dock-heading">
            {{ label('Сцена', 'Scene') }} <span>{{ document.bodies.length + document.sketches.length + (document.curves?.length ?? 0) + (document.surfaces?.length ?? 0) }}</span>
            <button
              type="button"
              class="dock-action"
              :aria-label="label('Новая группа из кода', 'New group from source')"
              :title="label('Собрать исходник в новую группу точных тел', 'Build the source into a new group of exact solids')"
              @click="openGroupDialog(null)"
            ><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg></button>
          </div>
          <SceneVirtualList ref="sceneList" :items="sceneRows" v-slot="{item:row,style,position,total}">
              <li v-if="row.kind==='group' && row.section" :data-scene-key="row.key" :aria-posinset="position" :aria-setsize="total" :style="style" class="scene-group" :class="{ 'selected-group': !!row.section.name && row.section.name === selectedGroup }">
                <button class="group-name" :aria-label="label('Активная группа: ','Active group: ')+(row.section.name??label('Без группы','Ungrouped'))" :aria-pressed="activeGroup===(row.section.name??'')" @click="activeGroup=row.section.name??''">{{ activeGroup===(row.section.name??'')?'● ':'' }}{{ row.section.name ?? label('Тела', 'Bodies') }}</button>
                <button v-if="row.section.name" class="group-remove" :aria-label="label('Изолировать группу: ','Isolate group: ')+row.section.name" @click="isolateGroup(row.section.name)">◎</button>
                <button
                  v-if="row.section.name"
                  type="button"
                  class="group-remove"
                  :aria-label="label('Изменить код группы', 'Edit group source') + ' ' + row.section.name"
                  :title="label('Изменить код группы', 'Edit group source')"
                  @click="openGroupDialog(row.section.name)"
                >✎</button>
                <button
                  v-if="row.section.name"
                  type="button"
                  class="group-remove"
                  :aria-label="label('Удалить группу', 'Delete group') + ' ' + row.section.name"
                  :title="label('Удалить группу', 'Delete group')"
                  @click="removeGroup(row.section.name)"
                >×</button>
              </li>
            <li v-else-if="row.kind==='object' && row.item" :data-scene-key="row.key" :aria-posinset="position" :aria-setsize="total" :style="style" class="object-row" :class="{muted:!objectVisible(row.item.id),locked:lockedIds.includes(row.item.id)}"><button type="button" :disabled="!objectSelectable(row.item.id)" :aria-label="row.item.name" :aria-pressed="selectedIds.includes(row.item.id)" @click="pickObject(row.item.id,row.pane!,$event.shiftKey)"><span class="dot" :class="row.dot" :style="row.item.material?{background:row.item.material.color}:undefined"></span>{{row.item.name}}<small v-if="row.item.instance">{{label('Экземпляр','Instance')}}</small><small v-if="row.item.brep">B-rep</small><small v-if="row.dot!=='body' && row.item.group">{{row.item.group}}</small></button><SceneObjectControls :name="row.item.name" :locale="locale" :hidden="!objectVisible(row.item.id)" :locked="lockedIds.includes(row.item.id)" @visibility="toggleObjectState(row.item.id,'hidden')" @lock="toggleObjectState(row.item.id,'locked')" /></li>
            <li v-else-if="row.kind==='label'" :data-scene-key="row.key" :aria-posinset="position" :aria-setsize="total" :style="style" class="scene-group">{{row.title}}</li>
            <li v-else :data-scene-key="row.key" :aria-posinset="position" :aria-setsize="total" :style="style" class="scene-empty">{{label('Сцена пуста. Добавьте примитив или нарисуйте эскиз.','The scene is empty. Add a primitive or draw a sketch.')}}</li>
          </SceneVirtualList>
        </template>
        <template v-else>
          <div class="dock-heading">{{ selectedSketch?.name || selectedBody?.name || label('Свойства', 'Properties') }}</div>
          <div v-if="selectedSketch || selectedBody" class="dock-props">
      <div v-if="!selectedBody?.instance" class="exact-grid">
        <label>X <CadQuantityInput v-model="dx" aria-label="ΔX" kind="length" :locale="locale" @validity="transformValidity('dx',$event)" /></label><label>Y <CadQuantityInput v-model="dy" aria-label="ΔY" kind="length" :locale="locale" @validity="transformValidity('dy',$event)" /></label><label v-if="selectedBody">Z <CadQuantityInput v-model="dz" aria-label="ΔZ" kind="length" :locale="locale" @validity="transformValidity('dz',$event)" /></label>
        <label>↻ <CadQuantityInput v-model="angle" kind="angle" :locale="locale" :aria-label="label('Поворот Z', 'Z rotation')" @validity="transformValidity('angle',$event)" />°</label><label>× <CadQuantityInput v-model="scale" kind="scalar" :locale="locale" :min=".001" :aria-label="label('Масштаб', 'Scale')" @validity="transformValidity('scale',$event)" /></label>
      </div>
      <div v-if="!selectedBody?.instance" class="exact-actions"><button class="primary" :disabled="directTransformPending||transformInputInvalid" @click="transform">{{ label('Применить', 'Apply') }}</button></div>
      <p v-if="selectedBody?.instance">{{ label('Связанный экземпляр · источник: ','Linked instance · source: ') + (document.bodies.find(b=>b.id===selectedBody?.instance?.sourceId)?.name??selectedBody.instance.sourceId) }}</p>
      <template v-if="selectedBody?.instance">
        <button @click="executeSolidCommand('instance-transform')">{{ label('Преобразовать экземпляр','Transform instance') }}</button>
        <button @click="executeSolidCommand('instance-place')">{{ label('Разместить экземпляр','Place instance') }}</button>
        <button @click="executeSolidCommand('instance-source')">{{ label('Выбрать источник экземпляра','Select instance source') }}</button>
        <button @click="executeSolidCommand('instance-detach')">{{ label('Сделать независимым','Make independent') }}</button>
      </template>
      <label v-if="selectedBody">{{ label('Цвет материала','Material color') }}<input type="color" :aria-label="label('Цвет материала','Material color')" :value="selectedBody.material?.color ?? '#7094ba'" @change="changeBodyMaterial"></label>
      <template v-if="selectedBody">
        <label>{{ label('Металлическость','Metallic') }}<input type="number" min="0" max="1" step=".1" :aria-label="label('Металлическость','Metallic')" :value="selectedBody.material?.metallic??0" @change="changeMaterialParameter('metallic',$event)"></label>
        <label>{{ label('Шероховатость','Roughness') }}<input type="number" min="0" max="1" step=".1" :aria-label="label('Шероховатость','Roughness')" :value="selectedBody.material?.roughness??.5" @change="changeMaterialParameter('roughness',$event)"></label>
        <label>{{ label('Непрозрачность','Opacity') }}<input type="number" min="0" max="1" step=".1" :aria-label="label('Непрозрачность','Opacity')" :value="selectedBody.material?.opacity??1" @change="changeMaterialParameter('opacity',$event)"></label>
        <small v-if="!gpuActive">{{ label('Металлическость и шероховатость видны в WebGPU; здесь показан базовый цвет.','Metallic and roughness require WebGPU; this view shows base color.') }}</small>
      </template>
      <button v-if="selectedBody?.material" @click="resetBodyMaterial">{{ label('Сбросить материал','Reset material') }}</button>
      <SketchDimensionPanel v-if="selectedSketch && !selectedSketch.retainedProfile" :key="selectedSketch.id" :sketch="selectedSketch" :locale="locale" @change="updateDimensionSketch" />
      <section v-if="selectedBody" class="body-diagnostics" :aria-label="label('Измерения','Measurements')">
        <button @click="measurementOpen=!measurementOpen" :aria-pressed="measurementOpen">{{ label('Измерения','Measurements') }}</button>
        <template v-if="measurementOpen">
          <label v-if="!clearanceOpen && !shellDistanceOpen && !volumeDistanceOpen">{{ label('Вершина A','Vertex A') }}<input v-model.number="measureA" type="number" min="1" :max="solidVertexCount(selectedBody)" :aria-label="label('Вершина A','Vertex A')"></label>
          <label>{{ label('Тело B','Body B') }}<select v-model="measureTarget" :aria-describedby="shellDistanceOpen?'shell-distance-error':undefined" :aria-invalid="shellDistanceOpen&&(!measurementTarget?.brep||measurementTarget.id===selectedBody.id)" :aria-label="label('Тело B','Body B')"><option value="">{{ label('Выбранное тело','Selected body') }}</option><option v-for="body in document.bodies" :key="body.id" :value="body.id">{{ body.name }}</option></select></label>
          <label v-if="!clearanceOpen && !shellDistanceOpen && !volumeDistanceOpen">{{ label('Вершина B','Vertex B') }}<input v-model.number="measureB" type="number" min="1" :max="measurementTarget?solidVertexCount(measurementTarget):1" :aria-label="label('Вершина B','Vertex B')"></label>
          <small v-if="measurementPending && !shellDistanceOpen && !volumeDistanceOpen" role="status" aria-label="vertex-measurement">{{ label('Измеряю вершины…','Measuring vertices…') }} <button @click="measurementOpen=false">Esc</button></small>
          <output v-if="measurement?.value && !shellDistanceOpen && !volumeDistanceOpen">{{ measurement.value.distanceMm.toFixed(6) }} mm · ΔXYZ [{{ measurement.value.deltaMm.map(v=>v.toFixed(6)).join(', ') }}]</output>
          <small v-if="measurement?.value && !shellDistanceOpen && !volumeDistanceOpen">A [{{ measurement.value.a.map(v=>v.toFixed(6)).join(', ') }}] · B [{{ measurement.value.b.map(v=>v.toFixed(6)).join(', ') }}] mm</small>
          <p v-if="measurement?.error && !shellDistanceOpen && !volumeDistanceOpen" role="alert">{{ measurement.error }}</p>
          <button v-if="selectedBody.brep" @click="edgeDistanceOpen=!edgeDistanceOpen" :aria-pressed="edgeDistanceOpen">{{ label('Расстояние между рёбрами','Distance between edges') }}</button>
          <fieldset v-if="edgeDistanceOpen" class="edge-distance-panel" aria-label="edge-distance">
            <legend>{{ label('Расстояние между исходными рёбрами','Distance between original edges') }}</legend>
            <label>{{ label('Ребро A','Edge A') }}<input v-model.number="edgeDistanceA" type="number" min="1" :max="selectedBody.brep?.edges.length" :aria-invalid="!Number.isInteger(edgeDistanceA)||!selectedBody.brep?.edges[edgeDistanceA-1]" aria-describedby="edge-distance-error" :aria-label="label('Ребро A','Edge A')"></label>
            <label>{{ label('Ребро B','Edge B') }}<input v-model.number="edgeDistanceB" type="number" min="1" :max="measurementTarget?.brep?.edges.length" :aria-invalid="!Number.isInteger(edgeDistanceB)||!measurementTarget?.brep?.edges[edgeDistanceB-1]" aria-describedby="edge-distance-error" :aria-label="label('Ребро B','Edge B')"></label>
            <label>{{ label('Объём расчёта','Calculation budget') }}<select v-model.number="edgeDistanceBudget" :aria-label="label('Объём расчёта расстояния','Distance calculation budget')"><option :value="10000">{{ label('Обычный','Standard') }}</option><option :value="100000">{{ label('Расширенный','Extended') }}</option></select></label>
            <small v-if="edgeDistancePending" role="status">{{ label('Измеряю расстояние…','Measuring distance…') }} <button @click="edgeDistanceOpen=false">Esc</button></small>
            <template v-if="edgeDistance?.value">
              <output data-edge-distance>{{ edgeDistance.value.distanceIntervalMm.map(v=>Number(v.toPrecision(12)).toString()).join(' … ') }} mm</output>
              <small v-if="edgeDistance.value.converged">{{ label('Допуск расстояния достигнут: 0,001 мм.','Distance tolerance reached: 0.001 mm.') }}</small>
              <p v-else role="status">{{ label('Расчёт не завершён: показаны нижняя и верхняя границы. Увеличьте объём расчёта; если лимит уже максимальный, проверьте более короткие участки рёбер.','Calculation incomplete: lower and upper bounds are shown. Increase the calculation budget; at the maximum budget, inspect shorter edge sections.') }}</p>
              <small>{{ label('Измерены только выбранные рёбра.','Only the selected edges are measured.') }}</small>
            </template>
            <p v-if="edgeDistance?.error" id="edge-distance-error" role="alert">{{ edgeDistance.error }}</p>
          </fieldset>
          <button v-if="selectedBody.brep" @click="faceDistanceOpen=!faceDistanceOpen" :aria-pressed="faceDistanceOpen">{{ label('Расстояние между гранями','Distance between faces') }}</button>
          <fieldset v-if="faceDistanceOpen" class="face-distance-panel" aria-label="face-distance">
            <legend>{{ label('Расстояние между исходными гранями','Distance between original faces') }}</legend>
            <label>{{ label('Грань A','Face A') }}<input v-model.number="faceDistanceA" type="number" min="1" :max="selectedBody.brep?.faces.length" :aria-invalid="!Number.isInteger(faceDistanceA)||!selectedBody.brep?.faces[faceDistanceA-1]" aria-describedby="face-distance-error" :aria-label="label('Грань A','Face A')"></label>
            <label>{{ label('Грань B','Face B') }}<input v-model.number="faceDistanceB" type="number" min="1" :max="measurementTarget?.brep?.faces.length" :aria-invalid="!Number.isInteger(faceDistanceB)||!measurementTarget?.brep?.faces[faceDistanceB-1]" aria-describedby="face-distance-error" :aria-label="label('Грань B','Face B')"></label>
            <label>{{ label('Объём расчёта','Calculation budget') }}<select v-model.number="faceDistanceBudget" :aria-label="label('Объём расчёта расстояния','Distance calculation budget')"><option :value="10000">{{ label('Обычный','Standard') }}</option><option :value="100000">{{ label('Расширенный','Extended') }}</option></select></label>
            <small v-if="faceDistancePending" role="status">{{ label('Измеряю расстояние…','Measuring distance…') }} <button @click="faceDistanceOpen=false">Esc</button></small>
            <template v-if="faceDistance?.value">
              <output data-face-distance>{{ faceDistance.value.distanceIntervalMm.map(v=>v===null?'?':Number(v.toPrecision(12)).toString()).join(' … ') }} mm</output>
              <small v-if="faceDistance.value.converged">{{ label('Допуск расстояния достигнут: 0,001 мм.','Distance tolerance reached: 0.001 mm.') }}</small>
              <p v-else-if="faceDistance.value.reason==='empty-domain'" role="status">{{ label('В области поверхности не найден материал грани. Проверьте контуры или выберите другую грань.','No face material lies in the surface domain. Check the trim loops or choose another face.') }}</p>
              <p v-else-if="faceDistance.value.distanceIntervalMm[1]===null" role="status">{{ label('Расчёт не завершён: допустимая пара точек ещё не найдена, верхняя граница неизвестна. Увеличьте объём расчёта.','Calculation incomplete: no admissible point pair was found yet, so the upper bound is unknown. Increase the calculation budget.') }}</p>
              <p v-else role="status">{{ label('Расчёт не завершён: показаны нижняя и верхняя границы. Увеличьте объём расчёта; при максимальном лимите результат остаётся неполным.','Calculation incomplete: lower and upper bounds are shown. Increase the calculation budget; at the maximum budget, the result remains incomplete.') }}</p>
              <small>{{ label('Измерены выбранные грани с их отверстиями.','The selected faces and their holes are measured.') }}</small>
            </template>
            <p v-if="faceDistance?.error" id="face-distance-error" role="alert">{{ faceDistance.error }}</p>
          </fieldset>
          <SolidVolumeDistance @state="(active,point)=>{volumeDistanceOpen=active;volumeContact=point}" :active="props.open && measurementOpen" :ru="ru" :a="selectedBody.brep" :b="measurementTarget?.brep" :same="selectedBody.id===measurementTarget?.id" :names="[selectedBody.name,measurementTarget?.name??'B']"/>
          <button v-if="selectedBody.brep" @click="shellDistanceOpen=!shellDistanceOpen" :aria-pressed="shellDistanceOpen">{{ label('Расстояние между оболочками','Distance between shells') }}</button>
          <fieldset v-if="shellDistanceOpen" class="shell-distance-panel" aria-label="shell-distance">
            <legend>{{ label('Расстояние между всеми гранями оболочек','Distance between all shell faces') }}</legend>
            <label>{{ label('Объём расчёта','Calculation budget') }}<select v-model.number="shellDistanceBudget" :aria-label="label('Объём расчёта оболочек','Shell calculation budget')"><option :value="100000">{{ label('Обычный','Standard') }}</option><option :value="1000000">{{ label('Расширенный','Extended') }}</option></select></label>
            <small v-if="shellDistancePending" role="status">{{ label('Измеряю расстояние…','Measuring distance…') }} <button @click="shellDistanceOpen=false">Esc</button></small>
            <ShellDistanceSummary v-if="shellDistance?.value" :value="shellDistance.value" :ru="ru"/>
            <p v-if="shellDistance?.error" id="shell-distance-error" role="alert">{{ shellDistance.error }}</p>
          </fieldset>
          <button @click="clearanceOpen=!clearanceOpen" :aria-pressed="clearanceOpen">{{ label('Зазор тел по сетке','Body mesh clearance') }}</button>
          <template v-if="clearanceOpen">
            <small>{{ label('Расчёт по сеткам тел. Для B-rep точность ограничена детализацией.','Measured on body meshes. B-rep accuracy is limited by tessellation.') }}</small>
            <p v-if="clearancePending" role="status">{{ label('Вычисляем зазор…','Computing clearance…') }}</p>
            <output v-if="clearanceMeasurement?.value">{{ label('Зазор','Clearance') }}: {{ formatMeasurement(clearanceMeasurement.value.gapMm) }} mm · {{ label('Перекрытие','Overlap') }}: {{ formatMeasurement(clearanceMeasurement.value.overlapMm3) }} mm³</output>
            <p v-if="clearanceMeasurement?.error" role="alert">{{ clearanceMeasurement.error }}</p>
          </template>
          <template v-if="selectedBody.brep && edgeIndex>=0">
            <small v-if="curvaturePending" role="status" aria-label="edge-measurement">{{ label('Измеряю кривизну…','Measuring curvature…') }} <button @click="measurementOpen=false">Esc</button></small>
            <label>{{ label('Параметр ребра (0…1)','Edge parameter (0…1)') }}<input v-model.number="curveParameter" type="number" min="0" max="1" step=".05" :aria-label="label('Параметр ребра','Edge parameter')"></label>
            <output v-if="curvatureMeasurement?.value">{{ label('Локальный радиус кривизны','Local curvature radius') }}: {{ curvatureMeasurement.value.radiusMm===null?'∞':curvatureMeasurement.value.radiusMm.toFixed(6)+' mm' }}</output>
            <p v-if="curvatureMeasurement?.error" role="alert">{{ curvatureMeasurement.error }}</p>
          </template>
          <small v-if="!clearanceOpen && !shellDistanceOpen && !volumeDistanceOpen">{{ label('Расстояние между указанными вершинами, не минимальное расстояние между телами. Для радиуса выберите ребро B-rep.','Distance between the specified vertices, not the minimum distance between bodies. Select a B-rep edge to measure curvature.') }}</small>
        </template>
      </section>
      <section v-if="selectedBody" ref="diagnosticPanel" tabindex="-1" aria-label="Body diagnostics" class="body-diagnostics">
        <button @click="diagnosticsOpen=!diagnosticsOpen" :aria-pressed="diagnosticsOpen">{{ label('Диагностика тела','Body diagnostics') }}</button>
        <template v-if="diagnosticsOpen">
          <strong data-testid="diagnostic-body" :data-body-id="selectedBody.id">{{ label('Тело: ','Body: ')+selectedBody.name }}</strong>
          <button v-if="selectedBody.brep" @click="faceContactsOpen=!faceContactsOpen" :aria-pressed="faceContactsOpen">{{ label('Проверить контакты граней','Inspect face contacts') }}</button>
          <fieldset v-if="faceContactsOpen" aria-label="face-contacts" class="boundary-agreement-panel">
            <legend>{{ label('Контакты B-rep-граней','B-rep face contacts') }}</legend>
            <label><input v-model="inspectWithinFaces" type="checkbox">{{ label('Проверять внутри граней','Inspect within faces') }}</label>
            <label>{{ label('Лимит проверки контактов','Contact inspection limit') }}<input v-model.number="faceContactsBudget" type="number" min="1" max="1000000" step="1" :aria-invalid="faceContactsBudgetInvalid" aria-describedby="face-contacts-error" :aria-label="label('Лимит проверки контактов','Contact inspection limit')"></label>
            <p v-if="faceContactsPending" role="status" aria-label="face-contacts-pending">{{ label('Проверяю контакты…','Inspecting contacts…') }} <button @click="faceContactsOpen=false">Esc</button></p>
            <template v-if="faceContactsResult">
              <template v-if="'absenceProven' in faceContactsResult">
                <p role="status">{{ faceContactsResult.absenceProven ? label('Отсутствие самопересечений подтверждено.','Absence of self-intersections is proven.') : label('Отсутствие самопересечений не доказано.','Absence of self-intersections is unproven.') }}</p>
                <p>{{ label('Не проверены или не доказаны грани: ','Unvisited or unproven faces: ')+faceContactsResult.faces.filter(f=>!f.result?.proven).map(f=>f.face+1).join(', ') }}</p>
              </template>
              <p data-testid="face-contacts-summary">{{ label('Контактов: ','Contacts: ')+faceContactsResult.contactPairCount+' · '+label('Общих границ: ','Shared boundaries: ')+faceContactsResult.sharedBoundaryPairCount+' · '+label('Не завершено пар: ','Unresolved pairs: ')+faceContactsResult.unresolvedPairCount+' · '+label('Не посещено: ','Unvisited: ')+faceContactsResult.unvisitedPairs }}</p>
              <p v-if="faceContactsResult.allPairsDisjoint">{{ label('Все разные пары граней разнесены.','All distinct face pairs are disjoint.') }}</p>
              <p v-else-if="faceContactsResult.allPairsClassified">{{ label('Пары граней разнесены либо имеют лишь подтверждённые общие границы.','Face pairs are disjoint or share only verified boundaries.') }}</p>
              <p v-else role="status">{{ label('Проверка неполна: границы и касания могут быть не определены. Увеличьте лимит или проверьте участки отдельно.','Incomplete: boundaries and tangencies may be unresolved. Increase the limit or inspect regions separately.') }}</p>
              <label v-if="faceContactDefects.length">{{ label('Найденный контакт','Detected contact') }}<select v-model.number="faceContactIndex" :aria-label="label('Найденный контакт','Detected contact')"><option v-for="(p,i) in faceContactDefects" :key="p.faces.join('-')" :value="i">{{ label('Грани ','Faces ')+p.faces.map(f=>f+1).join(' / ') }}</option></select></label>
              <output v-if="faceContactSelected?.witness">{{ label('Интервалы точки, мм: ','Point intervals, mm: ')+faceContactSelected.witness.pointIntervalMm.map((d,i)=>'XYZ'[i]+': ['+d[0]+', '+d[1]+']').join(' · ') }}</output>
              <small>{{ label('Красный маркер — найденная точка. Пригодность объёма требует отдельных проверок.','Red marker: detected point. Solid validity requires separate checks.') }}</small>
            </template>
            <p v-if="faceContactsError" id="face-contacts-error" role="alert">{{ faceContactsError }}</p>
            <button :disabled="faceContactsBudgetInvalid" :aria-disabled="faceContactsPending||faceContactsBudgetInvalid" @click="!faceContactsPending && faceContactsRevision++">{{ label('Повторить проверку контактов','Retry contact inspection') }}</button>
          </fieldset>
          <button v-if="selectedBody.brep" @click="boundaryOpen=!boundaryOpen" :aria-pressed="boundaryOpen">{{ label('Проверить границы B-rep','Inspect B-rep boundaries') }}</button>
          <fieldset v-if="boundaryOpen" aria-label="boundary-agreement" class="boundary-agreement-panel">
            <legend>{{ label('Соответствие рёбер поверхностям','Edge and surface agreement') }}</legend>
            <label>{{ label('Лимит проверки','Inspection limit') }}<input v-model.number="boundaryBudget" type="number" min="1" max="1000000" step="1" :aria-invalid="boundaryBudgetInvalid" aria-describedby="boundary-agreement-error" :aria-label="label('Лимит проверки границ','Boundary inspection limit')"></label>
            <p v-if="boundaryAgreementPending" role="status" aria-label="boundary-agreement-pending">{{ label('Проверяю границы…','Inspecting boundaries…') }} <button @click="boundaryOpen=false">Esc</button></p>
            <template v-if="boundaryResult">
              <p data-testid="boundary-summary">{{ label('Расхождения: ','Mismatches: ')+boundaryResult.mismatchCount+' · '+label('Не проверено: ','Unresolved: ')+boundaryResult.unresolvedCount }}</p>
              <p v-if="boundaryResult.allWithinTolerance">{{ label('Все границы совпадают с поверхностями в допуске модели.','All boundaries agree with their surfaces within the model tolerance.') }}</p>
              <p v-if="!boundaryResult.complete" role="status">{{ label('Проверка неполная. Увеличьте лимит и повторите; если результат не изменится, проверьте отдельные участки модели.','Inspection incomplete. Increase the limit and retry; if unchanged, inspect individual model sections.') }}</p>
              <template v-if="boundarySelected">
                <label>{{ label('Расхождение границы','Boundary mismatch') }}<select v-model.number="boundaryIndex" :aria-label="label('Расхождение границы','Boundary mismatch')"><option v-for="(item,i) in boundaryDefects" :key="i" :value="i">{{ label('Грань ','Face ')+(item.face+1)+' / '+label('ребро ','edge ')+(item.edge+1) }}</option></select></label>
                <output data-testid="boundary-distance">{{ boundarySelected.distanceIntervalMm?.map(v=>Number(v.toPrecision(9))).join(' … ') }} mm</output>
                <small>{{ label('Красным выделено проблемное ребро. Исправьте границу или поверхность и повторите проверку.','The affected edge is highlighted in red. Repair the boundary or surface and inspect again.') }}</small>
              </template>
              <small>{{ label('Проверено соответствие границ. Самопересечения и корректность объёма требуют отдельных проверок. Допуск, мм: ','Boundary agreement checked. Self-intersections and solid validity require separate checks. Tolerance, mm: ')+boundaryResult.toleranceMm }}</small>
            </template>
            <p v-if="boundaryError" id="boundary-agreement-error" role="alert">{{ boundaryError }}</p>
            <button :disabled="boundaryBudgetInvalid" :aria-disabled="boundaryAgreementPending||boundaryBudgetInvalid" @click="!boundaryAgreementPending && boundaryRevision++">{{ label('Повторить проверку границ','Retry boundary inspection') }}</button>
          </fieldset>

          <label>{{ label('Нормаль плоскости','Plane normal') }}<span role="group" :aria-label="label('Нормаль плоскости','Plane normal')" :aria-invalid="diagnostics?.value?.sectionError.includes('Section normal')||undefined" :aria-describedby="sectionInputInvalid||diagnostics?.value?.sectionError?'solid-section-error':undefined"><CadQuantityInput v-for="(axis,i) in ['X','Y','Z']" :key="axis" v-model="sectionNormal[i]" kind="scalar" :locale="locale" :aria-label="label('Нормаль ','Normal ')+axis" :aria-describedby="sectionInputInvalid||diagnostics?.value?.sectionError?'solid-section-error':undefined" style="width:70px" @validity="sectionValidity(axis,$event)" /></span></label>
          <small>{{ label('Нормаль задаёт направление плоскости; смещение измеряется от начала координат в миллиметрах.','The normal sets the plane direction; offset is measured from the origin in millimeters.') }}</small>
          <label>{{ label('Смещение сечения, мм','Section offset, mm') }}<CadQuantityInput v-model="sectionZ" :locale="locale" :aria-label="label('Смещение сечения, мм','Section offset, mm')" :aria-describedby="sectionInputInvalid||diagnostics?.value?.sectionError?'solid-section-error':undefined" @validity="sectionValidity('offset',$event)" /></label>
          <p v-if="diagnosticsPending" role="status">{{ label('Вычисляется сечение и диагностика сетки…','Computing section and mesh diagnostics…') }}</p>
          <template v-if="diagnostics?.value"><p>{{ label('Открытых рёбер: ','Open edges: ')+diagnostics.value.report.boundaryEdges }} · {{ label('Немногообразных: ','Non-manifold: ')+diagnostics.value.report.nonManifoldEdges }}</p>
          <p>{{ label('Конфликтов ориентации: ','Orientation conflicts: ')+diagnostics.value.report.orientationConflicts }} · {{ label('Вырожденных треугольников: ','Degenerate triangles: ')+diagnostics.value.report.degenerateTriangles }}</p>
          <small>{{ label('Жёлтый — сечение сетки. Красный — открытые границы. Проверка сетки не сертифицирует B-rep.','Yellow: mesh section. Red: open boundaries. Mesh checks do not certify B-rep.') }}</small>
          <p v-if="diagnostics.value.report.nonManifoldEdges">{{ label('Оранжевые рёбра принадлежат более чем двум треугольникам. Удалите лишние грани или разделите оболочки.','Orange edges belong to more than two triangles. Remove extra faces or separate shells.') }}</p>
          <p v-if="diagnostics.value.report.orientationConflicts">{{ label('Фиолетовые рёбра соединяют грани с несовместимой ориентацией. Согласуйте направление соседних граней.','Purple edges join inconsistently oriented faces. Align adjacent face winding.') }}</p>
          <p v-if="diagnostics.value.report.degenerateTriangles">{{ label('Красные контуры отмечают вырожденные треугольники. Удалите или перестройте их перед операциями с объёмом.','Red outlines mark degenerate triangles. Remove or rebuild them before solid operations.') }}</p>
          <p v-if="diagnostics.value.report.boundaryEdges">{{ label('Замкните выделенные границы перед операциями с объёмом.','Close the highlighted boundaries before solid operations.') }}</p></template>
          <p v-if="diagnostics?.value?.section.collapsedSegmentTriangles.length" role="status">{{ label('Сегментов с потерей точности: ','Section segments collapsed at floating-point precision: ')+diagnostics.value.section.collapsedSegmentTriangles.length }}. {{ label('Контур только для просмотра; измените смещение для точных измерений.','Display only; change the offset for precise measurements.') }}</p>
          <p v-if="sectionInputInvalid" id="solid-section-error" role="alert">{{ label('Сечение не проверено. Исправьте выделенные поля плоскости.','Section not checked. Correct the highlighted plane fields.') }}</p>
          <template v-else-if="diagnostics?.value?.sectionError"><p id="solid-section-error" role="alert">{{ sectionErrorMessage(diagnostics.value.sectionError) }}</p><button v-if="diagnostics.value.sectionError.includes('Section normal')" @click="sectionNormal=[0,0,1]">{{ label('Плоскость XY','XY plane') }}</button><details><summary>{{ label('Технические сведения','Technical details') }}</summary>{{ diagnostics.value.sectionError }}</details></template>
          <template v-if="diagnostics?.value?.boundaryError"><p role="alert">{{ label('Не удалось собрать открытые границы. Проверьте выделенные красные и оранжевые рёбра тела.','Could not assemble open boundaries. Inspect the highlighted red and orange body edges.') }}</p><details><summary>{{ label('Технические сведения','Technical details') }}</summary>{{ diagnostics.value.boundaryError }}</details></template>
          <p v-if="intersectionPending" role="status">{{ label('Проверяю контакты сетки…','Inspecting mesh contacts…') }}</p>
          <label>{{ label('Объём проверки контактов','Contact inspection effort') }}<select v-model.number="intersectionWork" :aria-label="label('Объём проверки контактов','Contact inspection effort')"><option :value="200000">{{ label('Обычный','Standard') }}</option><option :value="1000000">{{ label('Расширенный','Extended') }}</option><option :value="8000000">{{ label('Максимальный','Maximum') }}</option></select></label>
          <template v-if="intersectionInspection?.value">
            <p v-if="!intersectionInspection.value.complete" role="status" data-testid="intersection-incomplete">{{ intersectionInspection.value.stopReason==='work-limit'?label('Обход не завершён. Увеличьте объём проверки или проверьте меньшую часть модели.','Inspection is incomplete. Increase the effort or inspect a smaller part of the model.'):label('Достигнут лимит списка контактов. Увеличьте объём проверки либо исправьте выделенные дефекты и повторите проверку.','The contact list limit was reached. Increase the effort or repair the highlighted defects and inspect again.') }}</p>
            <p v-if="intersectionInspection.value.contacts.length" role="status">{{ label('Найдено контактов: ','Contacts found: ')+intersectionInspection.value.contacts.length }} · {{ intersectionInspection.value.complete?label('Обход завершён','Inspection complete'):label('Частичный результат','Partial result') }}</p>
            <p v-else-if="intersectionInspection.value.complete">{{ label('Недопустимых контактов сетки при относительном допуске 1e−9 не найдено.','No disallowed mesh contacts found at relative tolerance 1e−9.') }}</p>
            <template v-if="selectedIntersection"><p data-testid="intersection-current">{{ label('Недопустимый контакт треугольников: ','Disallowed triangle contact: ')+selectedIntersection.triangles.map(i=>i+1).join(' / ') }} · {{ intersectionIndex+1 }} / {{ intersectionInspection.value.contacts.length }}</p>
              <div><button :disabled="intersectionIndex===0" @click="intersectionIndex--">{{ label('Предыдущий контакт','Previous contact') }}</button><button :disabled="intersectionIndex+1>=intersectionInspection.value.contacts.length" @click="intersectionIndex++">{{ label('Следующий контакт','Next contact') }}</button></div>
              <small>{{ label('Розовым отмечены все найденные треугольники, жёлтым — выбранная пара и точка контакта. Исправьте грани и повторите проверку. Допуск, мм: ','Pink marks all detected triangles; yellow marks the selected pair and contact point. Repair the faces and inspect again. Tolerance, mm: ')+selectedIntersection.toleranceMm.toExponential(2) }}</small>
            </template>
          </template>
          <template v-if="intersectionInspection?.error"><p role="alert">{{ label('Самопересечения не проверены. Повторите проверку; при повторном отказе исправьте вырожденные грани или проверьте меньшую часть модели.','Self-intersection inspection incomplete. Retry; if it fails again, repair degenerate faces or inspect a smaller part of the model.') }}</p><details><summary>{{ label('Технические сведения','Technical details') }}</summary>{{ intersectionInspection.error }}</details></template>
          <template v-if="diagnostics?.error"><p role="alert">{{ label('Не удалось проверить сетку выбранного тела. Повторите проверку.','Could not inspect the selected body mesh. Retry the inspection.') }}</p><details><summary>{{ label('Технические сведения','Technical details') }}</summary>{{ diagnostics.error }}</details></template>
          <button :disabled="diagnosticsPending||intersectionPending||sectionInputInvalid" @click="diagnosticsRevision++">{{ label('Повторить диагностику','Retry diagnostics') }}</button>
        </template>
      </section>
      <template v-if="selectedBody?.brep">
        <label class="exact-detail">{{ label('Детализация B-rep', 'B-rep detail') }}<input v-model.number="brepSegments" type="number" min="1" max="32" step="1"></label>
        <div class="exact-actions"><button :aria-disabled="nativeNurbsPending" @click="!nativeNurbsPending && retessellateSelectedBrep()">{{ label('Перестроить mesh', 'Retessellate') }}</button><button :aria-disabled="nativeNurbsPending" @click="!nativeNurbsPending && measureSelectedBrep()">{{ label('Свойства B-rep', 'B-rep properties') }}</button></div>
        <small v-if="brepMass">V = {{ brepMass.signedVolumeMm3.toFixed(4) }} mm³ · A = {{ brepMass.surfaceAreaMm2.toFixed(4) }} mm² · {{ label('центр', 'centroid') }} [{{ brepMass.centroid.map(v=>v.toFixed(3)).join(', ') }}]</small>
        <small v-if="brepMass">{{ label('Численная оценка; замкнутость тела не сертифицирована. Оценка ошибки V:', 'Numerical estimate; solid validity is not certified. Estimated V error:') }} {{ brepMass.volumeErrorEstimateMm3.toExponential(2) }} mm³</small>
      </template>
          </div>
          <p v-else class="scene-empty">{{ label('Выберите эскиз или тело.', 'Select a sketch or a body.') }}</p>
        </template>
      </div>
    </aside>
    </div>
    <footer v-if="tool==='slot' || (tool === 'polyline' && draft.length)" class="context-bar">
      <span v-if="tool==='slot'">{{ label('Потяните от центра одного торца к другому. Esc — отмена.','Drag between the two end-cap centers. Esc cancels.') }}</span><label v-if="tool==='slot'">{{ label('Ширина паза, мм','Slot width, mm') }} <CadQuantityInput v-model="slotWidth" :locale="locale" :min="0.02" :max="1000000" :aria-label="label('Ширина паза, мм','Slot width, mm')" @validity="slotWidthValid=$event" /></label><template v-if="tool === 'polyline' && draft.length"><button @click="undoDraftPoint">{{ label('Убрать точку · Backspace','Undo point · Backspace') }}</button><span>{{ draft.length }} {{ label('точек', 'points') }}</span><button :disabled="draft.length < 3" @click="finish(true)">{{ label('Замкнуть контур', 'Close contour') }}</button><button :disabled="draft.length < 2" @click="finish(false)">{{ label('Завершить линию', 'Finish line') }}</button><button @click="cancelGesture">Esc</button></template>
    </footer>
    <CommandPalette v-if="paletteOpen" :open="paletteOpen" :commands="solidCommands" @close="paletteOpen = false" @execute="executeSolidCommand" />
    <div class="input-hint" role="status">{{ inputMode === 'touch' ? label('Два пальца: масштаб и перенос · Навигация: одним пальцем вращать 3D / двигать 2D', 'Two fingers: zoom and pan · Navigate: one finger orbits 3D / pans 2D') : label('ЛКМ: выбор / вращение 3D · СКМ или Shift: перенос · Колесо: масштаб', 'Left drag: select / orbit 3D · Middle drag or Shift: pan · Wheel: zoom') }}</div>
    <div v-if="showHelp" class="help-card"><p>{{ label('Грани: выберите поверхность, затем тяните её или жёлтую ручку. Ctrl/⌘ + клик выбирает несколько открытых граней для Shell. Для фаски и скругления включите «Рёбра».','Faces: select a surface, then drag it or its yellow handle. Ctrl/⌘ click selects multiple Shell openings. Switch to Edges for chamfers and fillets.') }}</p><p>{{ label('Shift + клик и «Рамка» выделяют несколько объектов. Манипулятор двигает, вращает и масштабирует весь выбор. У окружностей и дуг есть ручки центра, радиуса и концов дуги.','Shift click and Box select select multiple objects. The gizmo moves, rotates and scales the whole selection. Circles and arcs have center, radius and arc endpoint handles.') }}</p><strong>{{ label('Управление', 'Controls') }}</strong><p>{{ label('2D: тяните фигуру или вершину. Alt временно отключает привязку. Ломаная замыкается кликом по первой точке.', '2D: drag shapes or vertices. Alt bypasses snapping. Close a polyline by clicking its first point.') }}</p><p>{{ label('3D: тяните для вращения; G включает перемещение тела. ПКМ всегда вращает. Shift или средняя кнопка — панорама. Колесо — масштаб.', '3D: drag to orbit; G enables body movement. Right drag always orbits. Shift or middle drag pans. Wheel zooms.') }}</p><p>{{ label('E — предпросмотр выдавливания; зелёная ручка меняет высоту. Enter подтверждает, Escape отменяет. Ctrl/⌘ Z — отмена, Ctrl/⌘ Shift Z — повтор.', 'E previews extrusion; the green handle changes height. Enter applies, Escape cancels. Ctrl/⌘ Z undoes; Ctrl/⌘ Shift Z redoes.') }}</p><button @click="showHelp = false">{{ label('Понятно', 'Got it') }}</button></div>
    <div v-if="polygonView.limited" class="notice-bar" role="status" aria-label="transparency-limit">{{ label('Прозрачность приблизительная. Скройте часть сцены.','Transparency is approximate. Hide some objects.') }}</div>
    <div v-if="error" class="error-bar" role="alert">{{ error }} <button @click="error = ''">×</button></div>
    <div v-else-if="notice" class="notice-bar" role="status">{{ notice }} <button @click="notice = ''">×</button></div>
  </section>
</template>
<style scoped>
.body-diagnostics [role=group][aria-invalid=true]{outline:2px solid var(--danger,#ff6978);outline-offset:3px;border-radius:4px}
.edge-distance-panel,.face-distance-panel,.shell-distance-panel{display:grid;gap:8px;min-width:0}.edge-distance-panel output,.face-distance-panel output,.shell-distance-panel output{display:block;overflow-wrap:anywhere}.edge-distance-panel small,.face-distance-panel small,.shell-distance-panel small{display:block}
.boundary-agreement-panel{display:grid;gap:8px;min-width:0}.boundary-agreement-panel small,.boundary-agreement-panel output{display:block}.boundary-agreement-panel output{overflow-wrap:anywhere}
.body-diagnostics{display:grid;gap:8px;padding:8px 0}.body-diagnostics label{display:grid;gap:4px}.body-diagnostics p{margin:0}.body-diagnostics small{color:var(--text-dim)}
.restore-loading{position:absolute;inset:0;z-index:100;display:grid;place-items:center;background:var(--bg);color:var(--text-dim)}

.workplane-hint{padding:7px 12px;color:var(--accent);background:var(--surface);border-bottom:1px solid var(--border)}.profile-choices{display:grid;gap:5px;max-height:160px;overflow:auto;border:1px solid var(--border);padding:8px}.profile-choices input[type=checkbox]{width:auto}

.sketch-start-bar{display:flex;align-items:center;gap:8px;flex-wrap:wrap;padding:8px 12px;border-bottom:1px solid var(--border);background:var(--surface)}.sketch-start-bar>button{display:inline-flex;align-items:center;gap:6px}.extrude-sketch:not(:disabled){border-color:var(--accent);color:var(--accent)}

.direct-workspace{position:fixed;inset:var(--topbar-h,52px) 0 28px;z-index:20;display:flex;flex-direction:column;min-height:0;background:var(--bg);color:var(--text);outline:none;font-size:13px}.workspace-bar{display:flex;align-items:center;gap:16px;padding:10px 16px;border-bottom:1px solid var(--border);background:var(--surface)}button,input,summary,.file-open{color:var(--text);background:var(--surface-raised);border:1px solid var(--border);border-radius:5px;padding:7px 10px;font:inherit}button,summary{cursor:pointer}button:disabled{opacity:.4;cursor:default}button:hover:not(:disabled){background:var(--hover)}button:focus-visible,summary:focus-visible{outline:2px solid var(--accent)}[aria-pressed=true]{border-color:var(--accent);color:var(--accent)}.back{background:transparent}.command-search{display:inline-flex;align-items:center;gap:8px;min-width:190px;padding:6px 10px;background:var(--bg);color:var(--text-dim);border-radius:8px}.command-search span{flex:1;text-align:left}.command-search kbd{font:11px var(--font-mono,monospace);padding:1px 5px;border:1px solid var(--border);border-radius:4px}.history-tools{display:flex;gap:4px}.history-tools button{font-size:20px;padding:2px 12px}.save-status{margin-left:auto;display:inline-flex;align-items:center;justify-content:center;width:30px;height:30px;color:var(--text-dim)}.save-status.error{color:var(--danger)}.file-menu>summary{display:inline-flex;align-items:center;gap:6px;list-style:none}.file-menu>summary::-webkit-details-marker{display:none}.file-menu{position:relative}.file-menu>div{position:absolute;right:0;top:40px;z-index:5;width:250px;display:grid;gap:6px;padding:10px;background:var(--surface);border:1px solid var(--border);box-shadow:0 8px 30px #0004}.file-open input{display:block;width:100%;padding:4px;font-size:11px}.split-workspace{flex:1;min-height:0;display:grid;grid-template-columns:minmax(0,var(--split)) 7px minmax(0,1fr)}.split-workspace.sketch-hidden{grid-template-columns:minmax(0,1fr)}.pane-heading .pane-toggle{margin-left:auto;padding:4px 8px}.pane-heading .pane-toggle+button{margin-left:0}.pane{display:flex;flex-direction:column;min-width:0;min-height:0}.pane-heading{display:flex;align-items:center;gap:12px;padding:10px 14px;background:var(--surface);border-bottom:1px solid var(--border)}.pane-heading strong{font-size:14px}.pane-heading span{font-size:11px;color:var(--text-dim)}.pane-heading button{margin-left:auto;padding:4px 9px}.pane-tools{min-height:46px;padding:7px 12px;display:flex;gap:5px;align-items:center;flex-wrap:wrap;border-bottom:1px solid var(--border)}.pane-tools .subtle{flex:1}.pane-tools button{font-size:12px}.canvas-wrap{flex:1;min-height:120px;position:relative;overflow:hidden}.canvas-wrap svg{position:relative;width:100%;height:100%;display:block;touch-action:none;outline:none;user-select:none;-webkit-user-select:none;-webkit-user-drag:none}.canvas-wrap svg text{user-select:none;-webkit-user-select:none}.gpu-layer{position:absolute;inset:0;width:100%;height:100%;pointer-events:none}.canvas-wrap svg:focus-visible{box-shadow:inset 0 0 0 2px var(--accent)}.selected{stroke-width:3}.splitter{background:var(--surface-raised);cursor:col-resize;touch-action:none;display:flex;align-items:center;justify-content:center;border-inline:1px solid var(--border)}.splitter:hover,.splitter:focus-visible{background:var(--accent)}.splitter span{height:35px;width:2px;background:var(--text-dim);border-radius:2px}.context-bar{min-height:60px;display:flex;align-items:center;gap:12px;flex-wrap:wrap;padding:10px 16px;border-top:1px solid var(--border);background:var(--surface)}.context-bar label{display:flex;align-items:center;gap:5px;color:var(--text-dim)}.context-bar input{width:65px;padding:6px}.primary{background:var(--accent);color:var(--bg);font-weight:600}.primary:hover:not(:disabled){background:color-mix(in srgb,var(--accent) 88%,var(--text))}.delete{margin-left:auto}.subtle{color:var(--text-dim);font-size:12px}.empty-hint{position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;text-align:center;pointer-events:none;color:var(--text-dim);padding:25px}.empty-hint strong{font-size:18px;font-weight:500}.empty-hint span{font-size:12px;max-width:280px}.subtract-card{display:grid;gap:8px}.subtract-field{display:flex;flex-direction:column;align-items:flex-start;gap:2px;text-align:left;padding:8px 10px}.subtract-field[aria-pressed=true]{border-color:var(--accent);box-shadow:inset 0 0 0 1px var(--accent)}.subtract-field small{color:var(--text-dim);font:11px var(--font-mono,monospace);white-space:normal}.subtract-card>div{display:flex;justify-content:flex-end;gap:6px}.fps-badge{position:absolute;left:14px;bottom:14px;padding:3px 8px;border-radius:5px;background:var(--surface);color:var(--text-dim);font:11px var(--font-mono,monospace);pointer-events:none;opacity:.85}.fps-badge.low{color:var(--danger)}.zoom-tools{position:absolute;right:14px;bottom:14px;display:flex;gap:4px}.zoom-tools button{font-size:18px}.workspace-row{flex:1;min-height:0;display:flex}.side-dock{flex:0 0 344px;display:flex;min-height:0;border-left:1px solid var(--border);background:var(--bg)}.side-dock.collapsed{flex-basis:46px}.dock-rail{flex:0 0 46px;display:flex;flex-direction:column;align-items:center;gap:4px;padding:8px 0;border-right:1px solid var(--border)}.dock-rail button{width:34px;height:34px;padding:0;border:0;border-radius:8px;background:transparent;color:var(--text-dim);display:flex;align-items:center;justify-content:center}.dock-rail button:hover{color:var(--text);background:var(--hover)}.dock-rail button[aria-selected=true]{color:var(--text);background:var(--surface-raised)}.dock-rail-spacer{flex:1}.dock-body{flex:1;min-width:0;min-height:0;display:flex;flex-direction:column;overflow:auto}.dock-heading{height:42px;flex-shrink:0;display:flex;align-items:center;gap:8px;padding:0 12px;border-bottom:1px solid var(--border);font-weight:600}.dock-heading span{color:var(--text-dim);font-weight:400}.scene-list{list-style:none;margin:0;padding:6px;display:flex;flex-direction:column;gap:1px}.scene-group{display:flex;align-items:center;gap:4px;padding:8px 8px 4px;font-size:11px;font-weight:600;color:var(--text-dim);text-transform:uppercase;letter-spacing:.06em}.scene-group .group-name{flex:1;overflow:hidden;text-overflow:ellipsis}.scene-group .group-remove{width:auto;flex:0 0 auto;padding:0 6px;border:0;background:transparent;color:var(--text-dim);font-size:14px;line-height:1}.scene-group .group-remove:hover{color:var(--danger);background:transparent}.dock-heading .dock-action{margin-left:auto;width:28px;height:28px;padding:0;display:inline-flex;align-items:center;justify-content:center;border-radius:7px}.scene-list li>button{width:100%;display:flex;align-items:center;gap:8px;height:32px;padding:0 8px;border:0;border-radius:7px;background:transparent;color:var(--text);text-align:left;font-family:var(--font-mono,monospace);font-size:12.5px}.scene-list li>button:hover{background:var(--hover)}.scene-list li>button[aria-pressed=true]{background:color-mix(in srgb,var(--accent) 16%,var(--bg));outline:1px solid var(--accent);outline-offset:-1px;color:var(--text)}.scene-list small{margin-left:auto;font-size:11px;color:var(--text-dim);font-family:var(--font-ui,sans-serif)}.dot{width:8px;height:8px;border-radius:2px;background:#c3b7a3}.dot.sketch{background:var(--accent)}.dot.nurbs{background:#77eac5}.scene-empty{padding:16px 12px;color:var(--text-dim);font-size:12px;line-height:1.5}.dock-props{display:grid;gap:10px;padding:12px 14px}.exact-grid{display:flex;flex-wrap:wrap;gap:8px}.exact-grid label,.exact-detail{display:flex;align-items:center;gap:5px;color:var(--text-dim)}.exact-grid input,.exact-detail input{width:64px;padding:6px}.exact-actions{display:flex;gap:6px;flex-wrap:wrap}.dock-props small{color:var(--text-dim);line-height:1.5}.group-dialog-backdrop{position:absolute;inset:0;z-index:30;display:flex;align-items:center;justify-content:center;background:#0008}.group-dialog{width:min(560px,92vw);max-height:82%;display:flex;flex-direction:column;gap:10px;padding:16px;background:var(--surface);border:1px solid var(--border);border-radius:10px;box-shadow:0 12px 40px #0006}.group-dialog-head{display:flex;align-items:center}.group-dialog-head strong{flex:1;font-size:14px}.group-dialog-head button{padding:2px 9px;background:transparent;border:0;font-size:16px}.group-dialog-name{display:flex;align-items:center;gap:8px;color:var(--text-dim)}.group-dialog-name input{flex:1;padding:7px}.group-dialog-source{flex:1;min-height:200px;padding:10px;resize:vertical;background:var(--bg);color:var(--text);border:1px solid var(--border);border-radius:6px;font:12.5px/1.5 var(--font-mono,monospace)}.group-dialog small{color:var(--text-dim);line-height:1.5}.group-dialog-actions{display:flex;justify-content:flex-end;gap:8px}.error-bar{padding:10px 16px;color:var(--danger);background:var(--surface);display:flex;justify-content:space-between}.notice-bar{padding:10px 16px;color:var(--text-dim);background:var(--surface);display:flex;justify-content:space-between;gap:12px}@media(max-width:750px){.direct-workspace{inset:0}.side-dock{display:none}.workspace-bar{gap:8px;padding:8px}.workspace-bar>strong{font-size:12px}.save-status{display:none}.pane-heading{padding:8px;gap:5px}.pane-heading span{display:none}.pane-tools{padding:5px}.pane-tools button{padding:5px;font-size:11px}.context-bar{gap:7px;padding:8px}.context-bar input{width:52px}.empty-hint strong{font-size:14px}}
.hovered{stroke:#e1d4ff;stroke-width:3}.operation-card{max-height:calc(100% - 28px);overflow-y:auto;position:absolute;right:14px;top:14px;width:245px;display:grid;gap:10px;padding:15px;background:var(--surface);border:1px solid var(--border);border-radius:9px;box-shadow:0 8px 24px #0003}.operation-card :deep(small){font-size:11px;color:var(--text-dim);line-height:1.5}.operation-card :deep(label){min-width:0;display:flex;justify-content:space-between;align-items:center;gap:8px}.operation-card :deep(input){width:90px}.operation-card .preparation-tolerance :deep(.quantity-field){flex:0 0 110px}.operation-card input.surface-match-number{width:110px;flex-shrink:0}.operation-card :deep(input[type=checkbox]){width:auto}.operation-card :deep(select){max-width:145px;min-width:0;flex-shrink:1;background:var(--surface-raised);color:var(--text);padding:5px;border:1px solid var(--border)}.operation-card>div{display:flex;gap:5px}.segmented button{padding:5px 8px;font-size:12px}.live-measure{position:absolute;left:14px;top:14px;padding:8px 12px;border-radius:5px;background:var(--surface);color:var(--accent);font:14px monospace;pointer-events:none}.height-handle{cursor:ns-resize}.snap-toggle{display:flex;align-items:center;gap:4px;font-size:11px;margin-left:auto}.grid-input{width:50px;padding:4px}.transform-menu{position:relative}.transform-menu>div{position:absolute;bottom:40px;left:0;width:270px;display:flex;flex-wrap:wrap;gap:10px;padding:14px;border:1px solid var(--border);background:var(--surface);border-radius:8px;box-shadow:0 8px 24px #0003}.help-card{max-height:75vh;overflow:auto;position:absolute;right:18px;bottom:76px;width:min(360px,85vw);padding:20px;background:var(--surface);border:1px solid var(--border);border-radius:10px;box-shadow:0 8px 30px #0004;font-size:13px;line-height:1.6;z-index:5}@media(max-width:750px){.operation-card{width:195px;padding:10px;right:8px;top:8px}.pane-tools .subtle{display:none}.snap-toggle{margin-left:0}}
.nurbs-card{max-height:calc(100% - 28px);overflow:auto}.nurbs-cage circle{cursor:move}.trim-grid{display:grid!important;grid-template-columns:1fr 1fr;gap:5px!important}.trim-grid label{display:grid!important;gap:2px!important;font-size:11px}.trim-grid input{width:100%!important;box-sizing:border-box}

.compact-workspace .sketch-start-bar,.compact-workspace .primitive-bar{display:none}
.compact-workspace .pane-tools .subtle{display:none}
.compact-workspace .pane-tools{gap:4px;padding:6px;align-items:center}
.compact-workspace .pane-tools select{max-width:100px}
.compact-workspace .pane-tools .tool-icon{width:30px;height:30px;flex-shrink:0}
.workspace-state{display:flex;align-items:center;flex-wrap:wrap;gap:12px;padding:5px 12px;font-size:11px;color:var(--text-dim);border-bottom:1px solid var(--border)}
.workspace-state button{font-size:11px;padding:3px 7px}.state-legend{margin-left:auto;display:flex;align-items:center;gap:6px}.state-legend i{width:8px;height:8px;border-radius:2px}.state-hover{border:1px solid #89baff}.state-selected{background:#b496ff}.state-preview{background:#77eac5}
.command-guidance{display:flex;align-items:center;flex-wrap:wrap;gap:10px;padding:7px 12px;background:var(--surface);border-bottom:1px solid #77eac5;font-size:12px}.command-guidance.failed{border-color:var(--danger);color:var(--danger)}.command-keys{margin-left:auto;display:flex;align-items:center;gap:6px}.command-keys button{padding:3px 6px}.command-guidance kbd{border:1px solid var(--border);border-radius:3px;padding:1px 4px}
.gizmo-dimension{display:flex;align-items:center;gap:.3em;background:var(--bg, #252520);color:var(--text, #f5f5f0);border:1px solid #77eac5;border-radius:.3em;padding:.2em;box-sizing:border-box;width:100%;height:85%}.gizmo-dimension :deep(.quantity-field){width:100%}.gizmo-dimension :deep(input){font:inherit;min-width:0;width:100%;padding:0;border:0;background:transparent;color:inherit}.gizmo-dimension.failed{border-color:#f87171}.gizmo-dimension:focus-within{outline:2px solid var(--accent)}
.scene-list .selected-group{color:var(--accent);border-left:2px solid var(--accent)}
.operation-card small[role=alert]{color:var(--danger);border-left:2px solid var(--danger);padding-left:7px}
.operation-card>small[role=alert]{position:sticky;top:0;z-index:2;background:var(--surface)}
@media(max-width:750px){.state-legend{display:none}.command-guidance{font-size:11px}.workspace-state{gap:6px}}
.scene-list .object-row{display:flex;align-items:center;min-width:0}.scene-list .object-row>button{flex:1;min-width:0}.object-row.muted>button{opacity:.4}.object-row.locked>button{border-left:2px dashed var(--text-dim)}.scene-group .group-name{border:0;background:transparent;text-align:left;padding:2px;font:inherit;color:inherit}.scene-group .group-name[aria-pressed=true]{color:var(--accent)}

.pane.sketch-pane{container:sketch-pane / inline-size}
.canvas-wrap{display:flex}
.canvas-viewport{position:relative;flex:1;min-width:0;min-height:0;overflow:hidden}
.canvas-wrap.profile-preparation-pane{flex-direction:column}
.canvas-wrap.profile-preparation-pane>.operation-card{flex:0 1 auto;width:100%;max-height:60%;border-left:0;border-right:0;gap:8px;padding:12px}
.canvas-wrap.profile-preparation-pane>.operation-card>strong{position:sticky;top:0;z-index:1;background:var(--surface)}
.canvas-wrap>.operation-card{position:relative;inset:auto;flex:0 0 245px;box-sizing:border-box;max-height:100%;margin:0;border-radius:0;border-top:0;border-bottom:0;box-shadow:none}
.touch-mode .canvas-viewport>svg{touch-action:none}
@media(max-width:750px){.canvas-wrap{flex-direction:column}.canvas-wrap>.operation-card{flex:0 1 auto;width:100%;max-height:45%;border-left:0;border-right:0}.canvas-viewport{min-height:120px}}
@container sketch-pane (max-width:520px){
 .canvas-wrap{flex-direction:column}
 .canvas-wrap>.operation-card{flex:0 1 auto;width:100%;max-height:45%;border-left:0;border-right:0;gap:8px;padding:10px}
 .canvas-viewport{min-height:120px}
}
</style>

<style scoped>
.restore-loading{position:absolute;inset:0;z-index:100;display:grid;place-items:center;background:var(--bg);color:var(--text-dim)}

.workplane-hint{padding:7px 12px;color:var(--accent);background:var(--surface);border-bottom:1px solid var(--border)}.profile-choices{display:grid;gap:5px;max-height:160px;overflow:auto;border:1px solid var(--border);padding:8px}.profile-choices input[type=checkbox]{width:auto}

.sketch-start-bar{display:flex;align-items:center;gap:8px;flex-wrap:wrap;padding:8px 12px;border-bottom:1px solid var(--border);background:var(--surface)}.sketch-start-bar>button{display:inline-flex;align-items:center;gap:6px}.extrude-sketch:not(:disabled){border-color:var(--accent);color:var(--accent)}

@media(max-width:750px){
  .workspace-bar{flex-wrap:wrap}
  .workspace-bar>.file-menu{margin-left:auto}
  .command-search{flex:1 1 160px;min-width:0}
  .file-menu>div{max-width:calc(100vw - 16px);max-height:calc(100dvh - 140px);overflow:auto;box-sizing:border-box}
}
.direct-workspace.embedded{position:absolute;inset:0;z-index:9}.embedded .workspace-bar{flex-wrap:wrap;gap:8px;padding:6px}.primitive-bar{display:flex;flex-wrap:wrap;gap:6px;align-items:center;padding:8px;border-bottom:1px solid var(--border)}.primitive-bar input{width:75px}.primitive-icon{width:36px;height:36px;padding:0;display:inline-flex;align-items:center;justify-content:center;border-radius:8px}.primitive-icon:hover{color:var(--accent)}.tool-icon{width:34px;height:34px;padding:0;display:inline-flex;align-items:center;justify-content:center;gap:3px;border-radius:7px}.tool-icon .tool-tag{font-size:9px;font-weight:700;letter-spacing:.04em}.tool-icon:has(.tool-tag){width:auto;padding:0 7px}.tool-group{display:inline-flex;gap:3px}.tool-divider{width:1px;height:22px;background:var(--border);margin:0 3px}.primitive-divider{width:1px;height:22px;background:var(--border);margin:0 4px}.embedded .pane-tools{padding:5px}.embedded .save-status{display:none}
.input-hint{padding:5px 12px;color:var(--text-dim);font-size:11px;flex-shrink:0}
.touch-mode .canvas-wrap>svg{touch-action:none}
.scene-list .object-row{display:flex;align-items:center;min-width:0}.scene-list .object-row>button{flex:1;min-width:0}.object-row.muted>button{opacity:.4}.object-row.locked>button{border-left:2px dashed var(--text-dim)}.scene-group .group-name{border:0;background:transparent;text-align:left;padding:2px;font:inherit;color:inherit}.scene-group .group-name[aria-pressed=true]{color:var(--accent)}
@container sketch-pane (max-width:520px){
 .canvas-wrap{flex-direction:column}
 .canvas-wrap>.operation-card{flex:0 1 auto;width:100%;max-height:45%;border-left:0;border-right:0;gap:8px;padding:10px}
 .canvas-viewport{min-height:120px}
}
</style>
