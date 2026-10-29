<script setup lang="ts">
import {useBooleanOperations} from './directModeler/useBooleanOperations'
import {prepareDocumentCommit} from './directModeler/prepareDocumentCommit'
import {primitiveCreationError} from '../services/primitiveCreationError'
import RoundSolidOptions from '../components/RoundSolidOptions.vue'
import {sourceBodyRecordInspectionOptions} from '../services/sourceBodyArchive'
const ExpressionShellCheck=defineAsyncComponent(()=>import('../components/ExpressionShellCheck.vue'))
const SurfacePairChecks=defineAsyncComponent(()=>import('../components/SurfacePairChecks.vue'))
const SurfaceDistanceCheck=defineAsyncComponent(()=>import('../components/SurfaceDistanceCheck.vue'))
const SourceBodyFailureNotice=defineAsyncComponent(()=>import('../components/SourceBodyFailureNotice.vue'))
import type {ExpressionDisplayReport} from '../services/expressionDisplay'
import type {BoundaryMeshReport} from '../services/expressionBoundaryMesh'
import type {ExpressionMeshReport} from '../services/expressionMesh'
import CpuOrbitCanvas from '../components/CpuOrbitCanvas.vue'
const CadDimensionOverlay=defineAsyncComponent(()=>import('../components/CadDimensionOverlay.vue'))
import {previewMeshes as manufacturingMeshes} from '../services/mainModeling'
const SolidGcodePanel=defineAsyncComponent(()=>import('./GcodePanel.vue'))
const SolidLaserPanel=defineAsyncComponent(()=>import('./LaserCamPanel.vue'))
const VrControls=defineAsyncComponent(()=>import('../components/VrControls.vue'))
import {faceAbsenceProven } from '../services/solidSelfIntersection'
import type {NurbsSurfaceDistance} from '../services/nurbsSurface'
import type {MainSolidJob} from '../services/mainSolidProtocol'
import SceneObjectControls from '../components/SceneObjectControls.vue'
const SceneVirtualList=defineAsyncComponent(()=>import('../components/SceneVirtualList.vue'))
import { connectedEdgeChain } from '../services/edgeSelection'
import CadQuantityInput from '../components/CadQuantityInput.vue'
import {numericRectangle,numericSlotSources} from '../services/numericSketchAuthoring'
import {type SurfaceBoundaryReport,type SurfaceBoundaryOptions} from '../services/solidSurfaceDiagnostics'
const SketchDimensionPanel=defineAsyncComponent(()=>import('../components/SketchDimensionPanel.vue'))
import { defineAsyncComponent, computed, nextTick, onUnmounted, ref, shallowRef, watch, watchEffect } from 'vue'
import {solidVertexCount} from '../services/solidMeasurements'
import {isSolidProfileEdit,type SolidProfileEditOptions} from '../services/solidProfileEdit'
import {isSolidSceneEdit} from '../services/solidSceneEdit'
import {isSolidBodyEdit,type SolidBodyEditOptions} from '../services/solidBodyEdit'
import {assignSolidMaterial} from '../services/solidMaterial'
import { stringifyMeshJson } from '../services/meshJson'
import { rayTriangleDistance } from '../services/math3d'
import ModelingFloorGrid from '../components/ModelingFloorGrid.vue'
import ModelingGridControls from '../components/ModelingGridControls.vue'
import WorkspaceActionBar from '../components/WorkspaceActionBar.vue'
import {createSolidPreviewWorker} from '../services/solidPreviewWorker'
import { sameSketchPlane } from '../services/directExtrusion'
import { useModelingGrid } from '../services/modelingGrid'
import { resolveModelingSnap, type SnapKind, type SnapGeometry } from '../services/modelingSnaps'
import CommandPalette from '../components/CommandPalette.vue'
import type {NurbsCurve,NurbsScreenProjection} from '../services/nurbsCurve'
import { bodyPoints, DirectHistory, directBodiesScad, parseDirectDocument, serializeDirectDocument, type DirectBody, type DirectDocument, type DirectSketch, type Point2 } from '../services/directModeling'
import type {SolidSketchEditOptions} from '../services/solidSketchEdit'
import { validateSimpleSketch, slotSketch, sampleCurve, bakeSketch, trimSketch, worldPoint, xyPlane, unit3, cross3, type SketchPlane, type Vec3 } from '../services/directSketchGeometry'
import { storageGet, storageSet } from '../services/safeStorage'
import type {SolidBooleanOptions} from '../services/solidBoolean'
import { defaultDirectCamera, directExtrusionTool, projectDirectPoint, unprojectDirectXY, unprojectDirectPlane } from '../services/directModelingTools'
import { MESH_IMPORT_ACCEPT } from '../services/meshImport'
import ManifoldSettings from '../components/ManifoldSettings.vue'
import { MESH_EXPORT_FORMATS, MESH_FORMAT_LABELS } from '../services/meshConvert'
import { solidDocumentToMeshDocument } from '../services/solidBridge'
import type {SolidBrepToolOptions} from '../services/solidBrepTool'
import type {SolidNurbsEditOptions} from '../services/solidNurbsEdit'
import { type SolidNurbsCurve } from '../services/solidNurbs'
import type { SurfaceJetBoundary} from '../services/nurbsConstructors'
import {isSolidNurbsRefit } from '../services/solidCurveReduction'
import {isSolidSurfaceBuild,type buildSolidSurface} from '../services/solidSurfaceConstruction'
import { type BrepMassProperties, type BrepBooleanOperation } from '../services/geometry/brep'
import { touchPointers, panes, collectDragNodes, profileBooleanOperation, isProfileCommand, selectIndexKey, download, resizeSplit, canvasOf, primitiveKinds, PRIMITIVE_ICONS } from './directModeler/helpers'
import type { Pane } from './directModeler/helpers'
import { usePrimitives } from './directModeler/usePrimitives'
import { useSceneTree } from './directModeler/useSceneTree'
import { useSourceDisplay } from './directModeler/useSourceDisplay'
import { useAdvancedPreview } from './directModeler/useAdvancedPreview'
import { useNurbsMatching } from './directModeler/useNurbsMatching'
import { useCommandPalette } from './directModeler/useCommandPalette'
import { useSourceBodyTools } from './directModeler/useSourceBodyTools'
import { useDiagnostics } from './directModeler/useDiagnostics'
import { useMeasurements } from './directModeler/useMeasurements'
import { useDraftPersistence } from './directModeler/useDraftPersistence'
import { useFileExchange } from './directModeler/useFileExchange'
import { useDisplayAndSnapQueues } from './directModeler/useDisplayAndSnapQueues'
import { useViewportLayerAndPolygons } from './directModeler/useViewportLayerAndPolygons'
const ProfileIntersectionPresentation=defineAsyncComponent(()=>import('../components/ProfileIntersectionPresentation.vue'))
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
const MaterialPathCheck=defineAsyncComponent(()=>import('../components/MaterialPathCheck.vue'))
const label = (a: string, b: string) => ru.value ? a : b
const key = props.embedded ? 'scad-main-modeler-v1' : 'scad-solid-modeler-v1'
const error = ref('')
const workspace = ref<HTMLElement>(), splitArea = ref<HTMLElement>()
const historyWorker=createSolidPreviewWorker(),historyPending=ref(false),historyMode=ref<'restore'|'import'|'export'|'shared'>('restore')
let historyGeneration=0
const {
  initial,
  saveError,
  savePending,
  draftConflict,
  loadingLatestDraft,
  restoringDraft,
  isRestoreDisposed,
  getSaveGeneration,
  loadLatestDraft,
  persist,
  kernelReady,
  kernelLoadError,
  kernelLoading,
  retryKernelLoad,
  stopDraftRecovery,
  cancelDraftRecovery,
  restoreInitialDraft,
} = useDraftPersistence({
  key,
  legacyKey: props.embedded ? 'scad-main-modeler-v1' : 'scad-direct-modeler-v1',
  props,
  label,
  error,
  workspace,
  historyWorker,
  historyPending,
  historyMode,
  bumpHistoryGeneration: () => ++historyGeneration,
  getHistoryGeneration: () => historyGeneration,
  getHistory: () => history,
  getDocument: () => document,
  getSnapDocument: () => snapDocument,
  getCancelCommand: () => cancelCommand,
  getSync: () => sync,
  getFit: () => fit,
})
let history = new DirectHistory(initial)
const document = shallowRef(history.document)
// Committed render snapshot; gestures publish separate documents and never update this ref.
const snapDocument = shallowRef(document.value)
const undoable = ref(false), redoable = ref(false)
const selection = ref(props.initialSelection ?? ''), mode = ref<'2d' | '3d'>('2d'), tool = ref<'select' | 'rectangle' | 'circle' | 'arc' | 'polyline' | 'trim' | 'slot'>('select')
const draftCursor=ref<Point2|null>(null)
const slotWidth=ref(5), slotWidthValid=ref(true)
const draftPoint=ref<Point2>([0,0]),draftPointValid=ref([true,true])
const draftRadius=ref(10),draftRadiusValid=ref(true),draftArcStart=ref(0),draftArcSweep=ref(180),draftArcValid=ref([true,true])
const exactRoundCurve=computed(()=>{
 if(!draftPointValid.value.every(Boolean)||!draftPoint.value.every(Number.isFinite)||!draftRadiusValid.value||!Number.isFinite(draftRadius.value)||draftRadius.value<.01||draftRadius.value>1000000)return null
 if(tool.value==='arc'&&(!draftArcValid.value.every(Boolean)||!Number.isFinite(draftArcStart.value)||!Number.isFinite(draftArcSweep.value)||Math.abs(draftArcSweep.value)<.1||Math.abs(draftArcSweep.value)>=360))return null
 return {kind:tool.value==='arc'?'arc' as const:'circle' as const,center:[...draftPoint.value] as Point2,radius:draftRadius.value,start:tool.value==='arc'?draftArcStart.value:0,sweep:tool.value==='arc'?draftArcSweep.value:360}
})
const exactRoundPoints=computed(()=>(tool.value==='circle'||tool.value==='arc')&&exactRoundCurve.value?sampleCurve(exactRoundCurve.value):[])
function createExactRoundCurve(){if((tool.value!=='circle'&&tool.value!=='arc')||!exactRoundCurve.value)return;run(()=>{addSketch(sampleCurve(exactRoundCurve.value!),exactRoundCurve.value!.kind==='circle',exactRoundCurve.value!);draft.value=[];draftCursor.value=null;tool.value='select'})}
const draftSize=ref<Point2>([20,10]),draftSizeValid=ref([true,true]),draftEnd=ref<Point2>([20,0]),draftEndValid=ref([true,true])
const exactRectangle=computed(()=>draftPointValid.value.every(Boolean)&&draftSizeValid.value.every(Boolean)?numericRectangle(draftPoint.value,draftSize.value):null)
function createExactRectangle(){if(tool.value!=='rectangle'||!exactRectangle.value)return;run(()=>{addSketch(exactRectangle.value!,true);draft.value=[];draftCursor.value=null;tool.value='select'})}
const numericSlotWorker=createSolidPreviewWorker(),numericSlotPending=ref(false),numericSlotResult=shallowRef<DirectSketch|null>(null),numericSlotError=ref(false),numericSlotRetry=ref(0)
let numericSlotGeneration=0,numericSlotAutoApply=false
function cancelNumericSlot(){numericSlotGeneration++;numericSlotWorker.cancel();numericSlotPending.value=false;numericSlotResult.value=null;numericSlotError.value=false;numericSlotAutoApply=false}
onUnmounted(()=>{cancelNumericSlot();numericSlotWorker.dispose()})
const numericSlotInput=computed(()=>draftPointValid.value.every(Boolean)&&draftEndValid.value.every(Boolean)&&slotWidthValid.value?numericSlotSources(draftPoint.value,draftEnd.value,slotWidth.value,activePlane.value):null)
function createExactSlot(){const sketch=numericSlotResult.value;if(tool.value!=='slot'||numericSlotPending.value||!sketch?.retainedProfile||!numericSlotInput.value)return;run(()=>{addSketch(sketch.points,true,undefined,sketch.retainedProfile);draft.value=[];draftCursor.value=null;tool.value='select'})}
function refreshNumericSlot(){
 const autoApply=numericSlotAutoApply;cancelNumericSlot()
 if(!props.open||tool.value!=='slot'||!numericSlotInput.value)return
 const generation=numericSlotGeneration,source=document.value,input=numericSlotInput.value
 numericSlotPending.value=true
 queueMicrotask(async()=>{
  if(generation!==numericSlotGeneration)return
  try{
   const result=await numericSlotWorker.run({kind:'profilePrepare',...input,tolerance:1e-7})
   if(generation!==numericSlotGeneration||tool.value!=='slot'||document.value!==source)return
   const sketch=result.document.sketches.find(s=>s.id===result.id)
   if(!result.report.accepted||!sketch?.retainedProfile)throw Error('Slot profile preparation failed')
   numericSlotResult.value=sketch;numericSlotPending.value=false
   if(autoApply)createExactSlot()
  }catch{if(generation===numericSlotGeneration)numericSlotError.value=true}
  finally{if(generation===numericSlotGeneration)numericSlotPending.value=false}
 })
}
const canAddDraftPoint=computed(()=>draftPointValid.value.every(Boolean)&&draftPoint.value.every(Number.isFinite)&&!draft.value.some(p=>p[0]===draftPoint.value[0]&&p[1]===draftPoint.value[1]))
function addExactDraftPoint(){if(tool.value!=='polyline'||!canAddDraftPoint.value)return;draft.value=[...draft.value,[...draftPoint.value]];draftCursor.value=null;snapMarker.value=null;snapGuide.value=null}
function undoDraftPoint(){draft.value=draft.value.slice(0,-1);draftCursor.value=null;snapMarker.value=null;snapGuide.value=null}
const draft = ref<Point2[]>([]), height = ref(10), dx = ref(0), dy = ref(0), dz = ref(0), angle = ref(0), scale = ref(1)
const split = ref(34)
const views = ref<Record<Pane, number>>({ '2d': 160, '3d': 160 })
const centers = ref<Record<Pane, Point2>>({ '2d': [0, 0], '3d': [0, 0] })
const savedInputMode = storageGet('scad-input-mode')
const inputMode = ref<'mouse' | 'touch'>(savedInputMode === 'touch' || (savedInputMode !== 'mouse' && typeof window !== 'undefined' && window.matchMedia?.('(pointer: coarse)').matches) ? 'touch' : 'mouse')
const touchNavigate = ref(false)
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
let previewGeneration=0,previewIdentity=''
const solidPreviewResult=shallowRef<DirectDocument|null>(null), previewPending=ref(false)
// keepPreview: a parameter edit of the same operation leaves the last preview on screen until its replacement arrives (no blinking).
function invalidateSolidPreview(keepPreview=false) {
 clearTimeout(previewTimer);previewGeneration++;previewWorker.cancel();previewPending.value=false
 if(!keepPreview){solidPreviewResult.value=null;previewBody.value=null;previewEmpty.value=false;previewError.value=''}
}
onUnmounted(() => { invalidateSolidPreview();previewWorker.dispose() })
let orbitDrag: { x: number; y: number; yaw: number; pitch: number; pointer: number; svg: SVGSVGElement } | null = null
// While the camera (or an extrusion height, for its preview) is dragged the view keeps the working mesh: responsive, no faceted/refined swaps.
const cameraDragging = ref(false), heightDragging = ref(false)
let heightDrag: { y: number; height: number; pointer: number; svg: SVGSVGElement } | null = null
let gesture: { start: Point2; document: DirectDocument; vertex: number | null; id: string; pointer: number; pane: Pane; svg: SVGSVGElement; pan: boolean; center: Point2; end?:Point2; sketch?: boolean; anchor?: Point2; anchor3?: Vec3; dragStart?: Point2; workerEdit?:boolean; inverse?:DOMMatrix|null; bodyDrag?: { ids: string[]; delta: Vec3 } | null } | null = null

function translationPreviewIds(ids:string[]){return [...new Set([...ids,...document.value.bodies.filter(b=>b.instance&&ids.includes(b.instance.sourceId)).map(b=>b.id)])]}


const extraSelection=ref<string[]>([]),pickMode=ref<'body'|'face'|'edge'|'vertex'>('body'),vertexIndexes=ref<number[]>([]),faceIndex=ref(-1),edgeIndex=ref(-1),edgeIndexes=ref<number[]>([]),openingFaces=ref<number[]>([])
const workplaneOutline=ref<Point2[][]>([])
const workplaneBodyId=ref(''), choosingSketchFace=ref(false)
const faceDrawing = computed(()=>!!workplaneBodyId.value && tool.value!=='select' && tool.value!=='trim')
function setProfileTarget(id:string){
 const index=surfaceInputs.value.indexOf(id)
 if(index>0){const ids=[...surfaceInputs.value];[ids[0],ids[index]]=[ids[index],ids[0]];surfaceInputs.value=ids}
}
const pointTrimPick=shallowRef<{point:[number,number];matrix:NurbsScreenProjection;radius:number}|null>(null)
const activePlane=ref<SketchPlane>(xyPlane()),advancedOp=ref<'nurbs-offset'|'nurbs-point-trim'|'instance-transform'|'instance-create'|'instance-place'|'push'|'chamfer'|'edge-fillet'|'shell'|'split'|'offset'|'extend'|'curve'|'transform'|'loft'|'nurbs-loft'|'nurbs-sweep'|'nurbs-rebuild'|'nurbs-reduce'|'nurbs-surface-rebuild'|'nurbs-surface-reduce'|'nurbs-patch'|'nurbs-runout'|'nurbs-match'|'nurbs-prepare'|'nurbs-curve-match'|'profile-prepare'|'profile-union'|'profile-difference'|'profile-intersection'|null>(null)
const loftPreviewId=ref(''),surfaceInputs=ref<string[]>([]),surfaceReversed=ref<boolean[]>([])
const advanced=ref({runoutPosition:90,runoutTangentLength:.1,runoutFlip:false,offsetJoin:'smooth' as 'smooth'|'bevel'|'trim-nonzero'|'trim-evenodd',patchPrepare:false,patchError:1e-6,profileGap:.01,curveEndA:'end' as 'start'|'end',curveEndB:'start' as 'start'|'end',curveAngle:1e-6,prepareOpenPeriodic:false,matchOrder:1 as 1|2,matchBoundaryA:'uMax' as SurfaceJetBoundary,matchBoundaryB:'uMin' as SurfaceJetBoundary,matchScale:1,matchReverse:false,matchError:1e-6,sweepMode:'translation' as 'translation'|'framed',sweepSections:24,sweepDeviation:.01,sweepNormalX:0,sweepNormalY:0,sweepNormalZ:1,surfaceAxis:'u' as 'u'|'v',rebuildControls:6,reduceDegree:1,maxError:.01,distance:2,radius:2,endRadius:3,filletMode:'constant' as 'constant'|'variable'|'corner'|'partial-preview',axis:'z' as 'x'|'y'|'z',x:0,y:0,z:0,angle:0,scale:1,cx:0,cy:0,start:0,sweep:180,end:'end' as 'start'|'end'})
const brepSegments=ref(4),filletSegments=ref(12)
const revolveGeometry=ref<'faceted'|'exact'>('faceted')
function setPickedEdge(index:number){edgeIndex.value=index;edgeIndexes.value=index>=0?[index]:[];advancedOp.value=null}
const additionalEdgeIndex=ref(-1)
function addPickedEdge(){
 const index=additionalEdgeIndex.value
 if(!featureEdges.value.some(edge=>edge.i===index))return
 edgeIndexes.value=[...new Set([...edgeIndexes.value,index])];edgeIndex.value=index;advancedOp.value=null;additionalEdgeIndex.value=-1
}
function edgeSelectionKey(e:KeyboardEvent){
 const indices=[-1,...featureEdges.value.map(edge=>edge.i)]
 const index=selectIndexKey(e,Math.max(0,indices.indexOf(edgeIndex.value)),indices.length-1)
 if(index!==null)setPickedEdge(indices[index])
}
function revolveGeometryKey(e: KeyboardEvent){
 const index=selectIndexKey(e,revolveGeometry.value==='exact'?1:0,1,selectedSketch.value?.retainedProfile?1:0)
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
const nativeNurbsWorker=createSolidPreviewWorker(),nativeNurbsPending=ref(false)
const nativeBrepMode=ref<'mass'|'mesh'|null>(null)
let nativeNurbsGeneration=0
const gizmoWorker=createSolidPreviewWorker(),gizmoPending=ref(false)
let gizmoEpoch=0,gizmoRevision=0,gizmoRunning=false,gizmoPublishing=false,gizmoHistoryPending=false
let gizmoApplyRevision:number|null=null,gizmoBase:DirectDocument|null=null,gizmoPublished:DirectDocument|null=null
type DragWorkerJob=Extract<MainSolidJob,{kind:'sceneEdit'|'pointEdit'}>&{document:DirectDocument}
const pointEditActive=ref(false)
let numericPointEdit=false
let gizmoQueued:{job:DragWorkerJob;revision:number}|null=null
let manipulatorDrag:{svg:SVGSVGElement;pointer:number;x:number;y:number;kind:'move'|'rotate'|'scale'|'push'|'split';axis:'x'|'y'|'z';direction:Point2;before:DirectDocument;initial:number;startPoint:Point2;center:Point2;delta?:Vec3}|null=null

let curveDrag:{id:string;kind:'center'|'radius'|'start'|'end';before:DirectDocument;pointer:number;inverse:DOMMatrix|null}|null=null
let cvDrag:{id:string;u:number;v:number;before:DirectDocument;point:number[];start:Point2;pointer:number;svg:SVGSVGElement;inverse:DOMMatrix|null}|null=null
const faceSketchWorker=createSolidPreviewWorker(),faceSketchPending=ref(false)
let faceSketchGeneration=0
let previousFocus: HTMLElement | null = null
watch(() => props.open, async open => {
  if (open) {
    previousFocus = window.document.activeElement as HTMLElement
    await nextTick(); workspace.value?.focus()
  }
  else { await nextTick(); cancelGesture(); operation.value = null; previousFocus?.focus() }
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
  if(!objectSelectable(id)&&sourceEdgeSelection.value.startsWith(id+':'))sourceEdgeSelection.value=''
  const ids=[selection.value,...extraSelection.value].filter(objectSelectable)
  selection.value=ids[0]??'';extraSelection.value=ids.slice(1);hovered.value=''
  faceIndex.value=edgeIndex.value=-1;edgeIndexes.value=[];openingFaces.value=[]
}
function moveSelectionToGroup() {
  cancelCommand()
  void commitDirectTransform(snapDocument.value,selectedIds.value.filter(objectSelectable),[0,0,0],0,1,false,false,false,{operation:'group-move',group:activeGroup.value})
}
async function addEmptyGroup() {
  if(directTransformPending.value)return
  cancelCommand()
  const names=new Set([...documentObjects(document.value).map(b=>b.group),...(document.value.groups??[]).map(g=>g.name)])
  let i=1;while(names.has(label('Группа ','Group ')+i))i++
  const name=label('Группа ','Group ')+i
  if(await commitDirectTransform(snapDocument.value,[],[0,0,0],0,1,false,false,false,{operation:'group-create',group:name}))activeGroup.value=name
}
function selectEdgeChain() {
  if(!selectedBody.value || edgeIndex.value<0)return
  edgeIndexes.value=connectedEdgeChain(authoredEdges.value,edgeIndexes.value.length?edgeIndexes.value:[edgeIndex.value])
}
const selectedIds = computed(() => [...new Set([selection.value,...extraSelection.value].filter(Boolean))])
const selectedBodies = computed(() => selectedIds.value.flatMap(id => {
  const body = document.value.bodies.find(body => body.id === id)
  return body ? [body] : []
}))
const manufacturingOpen=ref(false),manufacturingRevision=ref(0)
watch(document,()=>{manufacturingRevision.value++},{flush:'sync'})
const manufacturingSelection=computed(()=>selectedIds.value.length===1?document.value.bodies.filter(body=>body.id===selectedIds.value[0]):[])
const manufacturingScene=computed(()=>manufacturingOpen.value?manufacturingMeshes({version:1,sketches:[],bodies:manufacturingSelection.value}):[])

const selectedBrepBodies = computed(() => selectedBodies.value.filter(b=>b.brep))
const twoSelectedBodies = computed(() => selectedIds.value.length === 2 && selectedIds.value.every(id => document.value.bodies.some(body => body.id === id)))
const selectedCurvePair = computed(() => selectedIds.value.length === 2 && selectedIds.value.every(id => document.value.curves?.some(item => item.id === id)) ? selectedIds.value : null)
const selectedSurfacePair = computed(() => selectedIds.value.length === 2 && selectedIds.value.every(id => document.value.surfaces?.some(item => item.id === id)) ? selectedIds.value : null)
const profileBridgePanel=ref<InstanceType<typeof SurfacePairChecks>|null>(null)
const profileBridgeSurfaces=computed(()=>selectedSurfacePair.value?selectedSurfacePair.value.map(id=>document.value.surfaces!.find(s=>s.id===id)!.surface) as [import('../services/nurbsSurface').NurbsSurface,import('../services/nurbsSurface').NurbsSurface]:null)
type SolidTopology=import('../services/mainSolidProtocol').MainSolidResults['topology']
const {
  snapPreparation,
  snapPreparationPending,
  snapPreparationVersion,
  snapPreparationErrors,
  snapBodies,
  snapPreparationNeedsRetry,
  cancelSnapPreparation,
  refreshSnapPreparation,
  sketchSnapPreparation,
  sketchSnapPending,
  sketchSnapVersion,
  sketchSnapErrors,
  supportSnapSketches,
  snapSketches,
  sketchSnapsNeedRetry,
  cancelSketchSnaps,
  refreshSketchSnaps,
  topology,
  topologyPending,
  topologyRetryVisible,
  isTopologyReadyFor,
  nextTopologyPick,
  getTopologyPick,
  cancelTopology,
  prepareTopology,
  curveDisplayQueue,
  curveDisplayPending,
  curveDisplayVersion,
  curveDisplayErrors,
  displayCurves,
  curveDisplayNeedsRetry,
  cancelCurveDisplay,
  refreshCurveDisplay,
  curvePoints,
  profileDisplayQueue,
  profileDisplayPending,
  profileDisplayVersion,
  profileDisplayErrors,
  displayProfiles,
  profileDisplayNeedsRetry,
  cancelProfileDisplay,
  refreshProfileDisplay,
  profileLoops,
  surfaceDisplayQueue,
  surfaceDisplayPending,
  surfaceDisplayVersion,
  surfaceDisplayErrors,
  surfaceDisplayNeedsRetry,
  cancelSurfaceDisplay,
  refreshSurfaceDisplay,
} = useDisplayAndSnapQueues({
  props,
  kernelReady,
  document,
  snapDocument,
  selectedBody,
  workplaneOutline,
  snap,
  geometrySnap,
  objectInView,
  getAdvancedPreviewDocument: () => advancedPreview.value.document,
  onSnapCacheInvalidated: () => { snapSceneCache = undefined },
  label,
  error,
})
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
const surfaceDistanceRevision=ref(0),surfaceDistanceRetryVisible=ref(false)
const surfaceDistanceWorker=createSolidPreviewWorker()
const surfaceDistance=shallowRef<{value:NurbsSurfaceDistance|null;error:string}|null>(null)
onUnmounted(()=>surfaceDistanceWorker.dispose())
const boundaryInspection=ref(false),boundaryInspectionRevision=ref(0),boundaryRetryVisible=ref(false)
const boundaryOptions=ref<SurfaceBoundaryOptions>({boundaryA:'uMax',boundaryB:'uMin',reverse:false,samples:65,toleranceMm:.01,angleToleranceDeg:1})
const boundaryInputValid=computed(()=>({gap:Number.isFinite(boundaryOptions.value.toleranceMm)&&boundaryOptions.value.toleranceMm>=.000001,angle:Number.isFinite(boundaryOptions.value.angleToleranceDeg)&&boundaryOptions.value.angleToleranceDeg>=0&&boundaryOptions.value.angleToleranceDeg<=90,samples:Number.isInteger(boundaryOptions.value.samples)&&boundaryOptions.value.samples>=2&&boundaryOptions.value.samples<=257}))
const boundaryWorker=createSolidPreviewWorker(),boundaryPending=ref(false)
const boundaryReport=shallowRef<{value:SurfaceBoundaryReport|null;error:string}|null>(null)
onUnmounted(()=>boundaryWorker.dispose())
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
 if(kind==='nurbs-runout'){loftPreviewId.value=crypto.randomUUID();mode.value='3d';surfaceInputs.value=[...selectedIds.value.filter(id=>document.value.surfaces?.some(s=>s.id===id)),...selectedIds.value.filter(id=>document.value.curves?.some(c=>c.id===id))];advanced.value.matchBoundaryA='vMax';surfaceReversed.value=[]}
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
const { manifoldMetrics, manifoldMetricsPending, manifoldMetricsRevision, manifoldMetricsRetryVisible, manifoldMetricsExportPending, manifoldMetricsExportError, exportManifoldMetrics, volumeDistanceOpen, volumeContact, materialPathOpen, materialPathOverlay, measurementOpen, measureTarget, measureA, measureB, curveParameter, formatMeasurement, clearanceOpen, clearanceRevision, clearanceRetryVisible, measurementTarget, clearanceMeasurement, clearancePending, measurementPending, curvaturePending, vertexAValid, vertexBValid, curveParameterValid, measurementRevision, curvatureRevision, measurementRetryVisible, curvatureRetryVisible, measurement, curvatureMeasurement, edgeDistanceOpen, edgeDistanceA, edgeDistanceB, edgeDistanceBudget, edgeDistanceRevision, edgeDistanceRetryVisible, edgeDistancePending, edgeDistance, faceDistanceOpen, faceDistanceA, faceDistanceB, faceDistanceBudget, faceDistanceRevision, faceDistanceRetryVisible, faceDistancePending, faceDistance, shellDistanceOpen, shellDistanceBudget, shellDistanceRevision, shellDistanceRetryVisible, shellDistancePending, shellDistance } = useMeasurements({ document, edgeIndex, label, props, selectedBody, selectedBodies, selection })
const { diagnosticPanel, diagnosticsOpen, sectionZ, sectionNormal, diagnosticsRevision, boundaryOpen, boundaryBudget, boundaryRevision, boundaryAgreementPending, boundaryIndex, boundaryResult, boundaryError, boundaryBudgetInvalid, boundaryDefects, boundarySelected, boundaryLine, faceContactsOpen, faceContactsBudget, faceContactsRevision, faceContactsPending, faceContactIndex, inspectWithinFaces, inspectExactBoundary, faceContactsResult, faceContactsError, faceContactsBudgetInvalid, faceContactDefects, faceContactSelected, faceContactPoint, sectionValidity, sectionInputInvalid, sectionErrorMessage, diagnosticsPending, diagnostics, intersectionInspection, intersectionPending, intersectionWork, intersectionIndex, selectedIntersection, selectedIntersectionLines, collapsedSectionPoints } = useDiagnostics({ document, label, props, selectedBody })
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
const formatUpperMm=(mm:number)=>{if(mm===0)return '0';const [coefficient,power]=mm.toExponential(2).split('e');return ((Number(coefficient.replace('.',''))+1)/100).toFixed(2)+'e'+power}
const TOLERANT_NOTICE=(mm:number)=>label(`Точного решения для этой пары нет; кривая пересечения построена численно с допуском ${formatMm(mm)} мм.`,`No exact solution exists for this pair; the intersection curve was traced numerically within ${formatMm(mm)} mm.`)
const {booleanPending,booleanKind,minkowskiDeviation,minkowskiProof,cancelBoolean,runBoolean,applyMinkowski,downloadMinkowskiProof,applyBrepBoolean,subtract,beginSubtract,subtractPick,subtractNames,applySubtract}=useBooleanOperations({document,snapDocument,selectedIds,selection,extraSelection,ru,mode,brepSegments,error,props,label,commit,fit,cancelCommand,run,download,EMPTY_NOTICE,FALLBACK_NOTICE,TOLERANT_NOTICE,
 setNotice:message=>{notice.value=message},captureCommand:()=>captureCommand(),recordCommand:command=>{lastCommand.value=command},calculationFailure:(e,retry)=>bodyCalculationFailure(e,retry)})
const { rebuildPeriodic, nurbsRefitResult, nurbsReduction, curveMatchResult, surfaceMatchResult, seamPrepareResult, isNurbsMatch, surfacePreparation, preparationError, profilePreparationError, preparedProfileResult, profilePreparation, curveMatchError, curveMatch, curveMatchGuides, surfaceMatch, surfaceMatchLines, surfaceMatchError, resultAdvanced, bodyCalculationFailure, surfaceConstructionError } = useNurbsMatching({ advanced, advancedOp, bodyEditError: () => bodyEditError, document, edgeIndex, edgeIndexes, history, label, pointTrimPick, project, selectedBody, selectedNurbsCurve, selectedNurbsSurface, selection, surfaceInputLabel, surfaceInputs })
const surfaceBuildResult=shallowRef<ReturnType<typeof buildSolidSurface>|null>(null)
const invalidQuantities = ref<Record<string, boolean>>({})
watch(()=>[advancedOp.value,advanced.value.sweep,selectedSketch.value?.analytic?.kind],()=>quantityValidity('curve.sweep-range',advancedOp.value!=='curve'||selectedSketch.value?.analytic?.kind!=='arc'||Number.isFinite(advanced.value.sweep)&&Math.abs(advanced.value.sweep)>=.1&&Math.abs(advanced.value.sweep)<=360),{flush:'sync'})
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
const invalidQuantitiesKey=computed(()=>JSON.stringify(invalidQuantities.value))
const selectedIdsKey=computed(()=>selectedIds.value.join('\0'))
watch(()=>[props.open,bodyEditRevision.value,advancedOp.value,document.value,selection.value,faceIndex.value,edgeIndex.value,edgeIndexes.value.join(','),openingFaces.value.join(','),surfaceInputs.value.join('\0'),selectedIdsKey.value,surfaceReversed.value.join(','),loftPreviewId.value,activeGroup.value,JSON.stringify(advanced.value),JSON.stringify(pointTrimPick.value),filletSegments.value,invalidQuantitiesKey.value],()=>{
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
    const result=await bodyEditWorker.run({kind:'surfaceBuild',document:source,options:{kind:operation,ids:options.inputs,id:loftPreviewId.value,group:activeGroup.value||undefined,reversed:[...surfaceReversed.value],runout:{sourceBoundary:options.matchBoundaryA,sourcePosition:options.runoutPosition/100,tangentLengthMm:options.runoutTangentLength,flipSide:options.runoutFlip,reverse:options.matchReverse},patchPreparation:{enabled:options.patchPrepare,maxError:options.patchError},sweep:{mode:options.sweepMode,normal:[options.sweepNormalX,options.sweepNormalY,options.sweepNormalZ],sections:options.sweepSections,maxDeviation:options.sweepDeviation}}})
    if(generation===bodyEditGeneration){surfaceBuildResult.value=result;if(operation==='nurbs-runout'&&result.runoutReport?.complete===false)bodyEditRetryVisible.value=true}
    return
   }
   if(isSolidNurbsRefit(operation)){
    const result=await bodyEditWorker.run({kind:'nurbsRefit',document:source,options:{operation,id:options.id,axis:options.surfaceAxis,degree:options.reduceDegree,controlCount:options.rebuildControls,maxError:options.maxError}})
    if(generation===bodyEditGeneration)nurbsRefitResult.value=result
    return
   }
   if(operation==='profile-prepare'){
    const result=await bodyEditWorker.run({kind:'profilePrepare',document:source,ids:options.inputs,tolerance:options.profileGap})
    if(generation===bodyEditGeneration){preparedProfileResult.value=result;activePlane.value=structuredClone(result.plane)}
    return
   }
   if(operation==='edge-fillet' && options.filletMode==='partial-preview') {
    const body=source.bodies.find(body=>body.id===options.id)
    if(!body?.brep)throw Error('Select a B-rep annular body.')
    if(options.edges.length!==1)throw Error('Select one outer circular quarter-arc for preview.')
    const result=await bodyEditWorker.run({kind:'partialAnnularPreview',body,edge:options.edges[0],radius:options.radius})
    if(generation===bodyEditGeneration)bodyEditResult.value={...source,bodies:source.bodies.map(body=>body.id===result.body.id?result.body:body)}
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
  const ids=new Set([...documentObjects(document.value),...document.value.sourceBodies??[]].map(o=>o.id))
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
  const ids = new Set([...documentObjects(d),...d.sourceBodies??[]].map(object => object.id))
  const retained = isolatedBodyIds.value.filter(id => ids.has(id))
  // Preview snapshots retain identity; only invalidate visibility when an isolated object disappears.
  if (retained.length !== isolatedBodyIds.value.length) isolatedBodyIds.value = retained
})
const renderBodies=computed(()=>previewReplacesTarget.value?sceneBodies.value.flatMap(b=>b.id===targetBody.value?(previewBody.value?[previewBody.value]:[]):[b]):sceneBodies.value)
const {
  fps,
  frameMs,
  drawMs,
  settling,
  hasDragNodes,
  setDragNodes,
  applyDragPreviewTransform,
  clearDragPreview,
  settleAfterDrag,
  smoothDisplay,
  displayPending,
  cancelDisplayPreparation,
  refreshDisplayPreparation,
  vrSnapshot,
  gpuActive,
  gpuInitializing,
  gpuRetryVisible,
  gpuFailureNotice,
  stopGpuLayer,
  retryGpuLayer,
  mountGpuCanvas,
  meshPolygons,
  cpuTransparency,
  polygonView,
  polygons,
  cpuOrbitReady,
  cpuOrbitActive,
  polygonColor,
  cpuOrbitTriangles,
  surfacePolygons,
  nurbsSurfacePolygons,
  ghostPolygons,
} = useViewportLayerAndPolygons({
  props,
  emitBackend: value => emit('backend', value),
  kernelReady,
  document,
  camera,
  views,
  centers,
  cameraDragging,
  heightDragging,
  previewingTransform,
  selectedIds,
  selectedBody,
  selectedFaceTriangles,
  pickMode,
  hovered,
  extrusionMode,
  previewBody,
  previewReplacesTarget,
  renderBodies,
  surfaceDisplayQueue,
  surfaceDisplayVersion,
  objectInView,
  getAdvancedPreview: () => ({
    document: advancedPreview.value.document ?? null,
    bodyIds: advancedBodyIds.value,
    surfaceIds: advancedSurfaceIds.value,
  }),
  getSubtract: () => subtract.value,
  setNotice: message => { notice.value = message },
  getNotice: () => notice.value,
  label,
})
const { advancedPolygons, advancedSurfacePolygons, advancedPreview, pointTrimPreviewPoints, pointTrimCut, patchBudgetCurve, patchGapPoints, applyAdvanced, axisVector, advancedBodyIds, advancedSurfaceIds, advancedSketches, advancedSketchDisplay, authoredEdges, bodyEdgesPending, cancelBodyEdges, refreshBodyEdges } = useAdvancedPreview({ activePlane, advanced, advancedOp, bodyEditError, bodyEditPending, bodyEditResult, commit, curvePoints, document, edgeIndex, edgeIndexes, error, extraSelection, faceIndex, isNurbsMatch, kernelReady: () => kernelReady, loftPreviewId, meshPolygons, openingFaces, pickMode, pickObject, profileLoops, props, resultAdvanced, run, samePlane, sceneBodies: () => sceneBodies, selectedBody, selectedIds, selection, surfaceBuildResult, surfaceConstructionError, surfaceInputs, surfacePolygons, surfaceReversed })
watch(() => [props.open, kernelReady.value, smoothDisplay.value, renderBodies.value,
  previewBody.value, advancedPreview.value.document, previewingTransform.value,
  previewingTransform.value ? selectedIds.value.join(',') : ''] as const, () => {
  refreshDisplayPreparation(advancedPreview.value.document)
}, { flush: 'pre', immediate: true })
const runoutProblemCurve=computed(()=>advancedOp.value==='nurbs-runout'&&advancedPreview.value.rawError.includes('RUNOUT_TARGET')?surfaceInputs.value[2]:undefined)
const { sourceDisplayResults, sourceDisplayErrors, sourceDisplayPending, sourceEdgeSelection, sourceEdgeHover, cancelSourceDisplay, refreshSourceDisplay, pickSourceEdge, pickSourceBody, sourceDisplayEdges, sourceDisplayFaces, sourceUnresolvedCount, sourceUnresolvedBoxes, sourceDisplayEndpointBoxes } = useSourceDisplay({ camera, cancelCommandState, document, edgeIndex, edgeIndexes, extraSelection, faceIndex, hovered: () => hovered, kernelReady: () => kernelReady, mode, objectInView, objectSelectable, openingFaces, project, props, selection, tool })
const featureEdges=computed(()=>authoredEdges.value.map(edge=>({i:edge.index,id:edge.id,points:edge.points.map(point=>project(point,'3d').join(',')).join(' ')})))
const splitPlanePoints=computed(()=>{
 if(advancedOp.value!=='split'||!selectedBody.value)return ''
 const n=axisVector(advanced.value.axis),u=unit3(cross3(n,n[0]?[0,1,0]:[1,0,0])),v=cross3(n,u),pos=selectedBody.value.mesh.positions
 let maxR=0
 for(let i=0;i<pos.length;i+=3){const r=Math.hypot(pos[i],pos[i+1],pos[i+2]);if(r>maxR)maxR=r}
 const size=maxR+10
 return [[-size,-size],[size,-size],[size,size],[-size,size]].map(q=>project(worldPoint(q,{origin:n.map(x=>x*advanced.value.distance) as Vec3,u,v}),'3d').join(',')).join(' ')
})
/** Axis-aligned bounds of the selection: the gizmo is centred on them and sized by them. */
const gizmoBounds=computed(()=>{
 let min0=Infinity,max0=-Infinity,min1=Infinity,max1=-Infinity,min2=Infinity,max2=-Infinity,hasPoints=false
 for(const b of document.value.bodies){
  if(!selectedIds.value.includes(b.id))continue
  const pos=b.mesh.positions
  for(let i=0;i<pos.length;i+=3){
   hasPoints=true
   const x=pos[i],y=pos[i+1],z=pos[i+2]
   if(x<min0)min0=x;if(x>max0)max0=x
   if(y<min1)min1=y;if(y>max1)max1=y
   if(z<min2)min2=z;if(z>max2)max2=z
  }
 }
 for(const s of document.value.sketches){
  if(!selectedIds.value.includes(s.id))continue
  for(const pt of s.points){
   hasPoints=true
   const q=worldPoint(pt,s.plane),x=q[0],y=q[1],z=q[2]
   if(x<min0)min0=x;if(x>max0)max0=x
   if(y<min1)min1=y;if(y>max1)max1=y
   if(z<min2)min2=z;if(z>max2)max2=z
  }
 }
 if(!hasPoints)return null
 return {center:[(min0+max0)/2,(min1+max1)/2,(min2+max2)/2] as Vec3, extent:Math.max(max0-min0,max1-min1,max2-min2)}
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
 // Navigation must bubble to the canvas even when it starts on an edge.
 if('button' in e && e.button!==0)return
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

function sync(next:DirectDocument=history.document) { snapDocument.value=next; document.value=next; const ids=new Set([...documentObjects(document.value),...document.value.sourceBodies??[]].map(o=>o.id));if(selection.value&&!ids.has(selection.value))selection.value='';extraSelection.value=extraSelection.value.filter(id=>ids.has(id)); if (gpuActive.value) settleAfterDrag(); const s=document.value.sketches.find(s=>s.id===selection.value);if(s&&!samePlane(s.plane,activePlane.value)){activePlane.value=s.plane??xyPlane();workplaneOutline.value=[]} undoable.value = history.canUndo; redoable.value = history.canRedo; persist() }
function prepareCommit(next:DirectDocument,origin:'edit'|'file'='edit') {
 return prepareDocumentCommit(next,origin==='file'&&!lockedIds.value.length?undefined:snapDocument.value,history.objectIds,lockedIds.value,activeGroup.value,origin,label)
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
function addSketch(points: Point2[], closed: boolean, analytic?: import('../services/directSketchGeometry').AnalyticCurve,retainedProfile?:import('../services/geometry/brepProfile').BrepProfile) {
  validateSimpleSketch(points,closed)
  const d = history.document, id = crypto.randomUUID()
  d.sketches.push({ id, name: label('Эскиз ', 'Sketch ') + (d.sketches.length + 1), points, closed, analytic, ...(retainedProfile?{retainedProfile}:{}), plane: JSON.parse(JSON.stringify(activePlane.value)), ...(workplaneBodyId.value?{supportBodyId:workplaneBodyId.value}:{}) })
  commit(d); pickObject(id,'2d')
}
function beginSketch(value: typeof tool.value) {
  cancelGesture(); operation.value = null; advancedOp.value = null; boxSelect.value = false
  choosingSketchFace.value=false;draftPoint.value=[0,0];draftPointValid.value=[true,true];draftRadius.value=10;draftRadiusValid.value=true;draftArcStart.value=0;draftArcSweep.value=180;draftArcValid.value=[true,true];draftSize.value=[20,10];draftSizeValid.value=[true,true];draftEnd.value=[20,0];draftEndValid.value=[true,true];tool.value = value; mode.value = workplaneBodyId.value ? '3d' : '2d'; sketchPaneOpen.value = true
}
const canExtrudeSketch = computed(() => !!selectedSketch.value?.closed && tool.value === 'select' && !draft.value.length)
function finish(closed: boolean) { run(() => { if (draft.value.length < (closed ? 3 : 2)) return; addSketch(draft.value, closed); draft.value = []; draftCursor.value=null; tool.value = 'select' }) }
function beginExtrude(kind: 'extrude'|'revolve' = 'extrude') {
  if (!canExtrudeSketch.value) return
  cancelCommand(); boxSelect.value=false
  if(kind==='revolve'&&selectedSketch.value?.retainedProfile)revolveGeometry.value='exact'
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
watch(()=>[props.open,operation.value,document.value,selection.value,activeGroup.value,cornerVertex.value,cornerRadius.value,copyCount.value,copySweep.value,copyX.value,copyY.value,sketchEditRevision.value,invalidQuantitiesKey.value],()=>{
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
const canDuplicate=computed(()=>selectedIds.value.length>0&&selectedIds.value.every(id=>document.value.bodies.some(body=>body.id===id)||document.value.sketches.some(sketch=>sketch.id===id)))
async function duplicate() {
 if(directTransformPending.value)return
 if(commandActive.value){error.value=label('Завершите текущую команду','Finish the current command');return}
 if(!canDuplicate.value){error.value=label('Для копирования выберите только тела или эскизы','Select only bodies or sketches to duplicate');return}
 cancelDirectTransform()
 const generation=directTransformGeneration,before=history.document,ids=[...selectedIds.value],copyIds=ids.map(()=>crypto.randomUUID())
 directTransformPending.value=true;directTransformOperation.value='duplicate';error.value=''
 try{
  const result=await directTransformWorker.run({kind:'sceneEdit',document:before.bodies.some(body=>body.instance)?serializeDirectDocument(before):before,options:{operation:'duplicate',id:ids[0]??'',ids,copyIds,createdId:'',x:10,y:10,z:0,axis:'z',angle:0,scale:1}})
  if(generation!==directTransformGeneration)return
  const {validateLocked,added}=prepareCommit(result)
  const applied=await history.commitAsync(async()=>result,validateLocked)
  if(generation!==directTransformGeneration||!applied)return
  directTransformPending.value=false;finishCommit(added,result);pickObject(copyIds[0],mode.value);extraSelection.value=copyIds.slice(1)
 }catch(e){if(generation===directTransformGeneration)error.value=bodyCalculationFailure(e,false)}
 finally{if(generation===directTransformGeneration)directTransformPending.value=false}
}

watch([document, () => props.open, operation, selectedSketch, profileIds, activeGroup, height, baseZ, brepSegments, revolveAxis, revolveOffset, revolveAngle, revolveSegments, revolveGeometry, extrusionMode, targetBody, solidPreviewRevision, invalidQuantitiesKey], () => {
  if (!props.open || Object.keys(invalidQuantities.value).length || !solidActive.value || !selectedSketch.value) {invalidateSolidPreview();previewIdentity='';return}
  const identity=[operation.value,selectedSketch.value.id,profileIds.value.join(','),activeGroup.value,extrusionMode.value,targetBody.value,revolveGeometry.value].join('\n');invalidateSolidPreview(identity===previewIdentity&&!!previewBody.value);previewIdentity=identity
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
    catch (e) { if(generation===previewGeneration){solidPreviewResult.value=null;previewBody.value=null;previewError.value=bodyCalculationFailure(e);solidPreviewRetryVisible.value=true} }
    finally {if(generation===previewGeneration)previewPending.value=false}
  }, 60)
}, {flush:'sync'})
watch(selection, () => { advancedOp.value=null; operation.value = null; previewBody.value = null; cornerVertex.value = 0 })
function remove() { run(() => {
  const selectedSet = new Set(selectedIds.value)
  advancedOp.value=null
  const d = history.document
  d.sketches = d.sketches.filter(s => !selectedSet.has(s.id))
  d.bodies = d.bodies.filter(b => !selectedSet.has(b.id))
  d.curves = d.curves!.filter(c => !selectedSet.has(c.id))
  d.surfaces = d.surfaces!.filter(s => !selectedSet.has(s.id))
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
watch(()=>[props.open,document.value,selectedIdsKey.value],()=>{
 if(!gizmoPublishing&&(gizmoBase||gizmoPending.value)){cancelGizmoWorker();manipulatorDrag=null;cvDrag=null;vertexDrag=null;curveDrag=null;if(gesture?.workerEdit)gesture=null;previewingTransform.value=false}
},{flush:'sync'})
watch(()=>[cvU.value,cvV.value],()=>{if(pointEditActive.value&&(gizmoBase||gizmoPending.value)){cancelGizmoWorker();cvDrag=null}},{flush:'sync'})
watch(()=>[cvU.value,cvV.value,cvX.value,cvY.value,cvZ.value,cvWeight.value],()=>{if(numericPointEdit&&gizmoPending.value)cancelGizmoWorker()},{flush:'sync'})
onUnmounted(()=>{cancelGizmoWorker();gizmoWorker.dispose()})
const directTransformWorker=createSolidPreviewWorker(),directTransformPending=ref(false),directTransformOperation=ref('transform')
let directTransformGeneration=0
function cancelDirectTransform(){
 directTransformGeneration++;directTransformWorker.cancel()
 if(directTransformPending.value){history.cancelRestore();clearDragPreview()}
 directTransformPending.value=false
}
async function commitDirectTransform(before:DirectDocument,ids:string[],delta:Vec3,rotation=0,factor=1,localSketch=false,resetFields=false,detach=false,metadata?:{operation:'group-create'|'group-move';group:string}){
 cancelDirectTransform()
 const generation=directTransformGeneration
 directTransformPending.value=true;directTransformOperation.value=metadata?.operation??(detach?'instance-detach':'transform');error.value=''
 try{
  const result=await directTransformWorker.run({kind:'sceneEdit',document:before.bodies.some(body=>body.instance)?serializeDirectDocument(before):before,options:{operation:metadata?.operation??(detach?'instance-detach':localSketch?'sketch-transform':'transform'),group:metadata?.group,id:ids[0]??'',ids,createdId:'',x:delta[0],y:delta[1],z:delta[2],axis:'z',angle:rotation,scale:factor}})
  if(generation!==directTransformGeneration)return
  const {validateLocked,added}=prepareCommit(result)
  result.blenderProjectId ??= snapDocument.value.blenderProjectId
  const applied=await history.commitAsync(async()=>result,validateLocked)
  if(generation!==directTransformGeneration||!applied)return
  directTransformPending.value=false;finishCommit(added,result);settleAfterDrag()
  if(resetFields){dx.value=dy.value=dz.value=angle.value=0;scale.value=1}
  return true
 }catch(e){if(generation===directTransformGeneration){error.value=bodyCalculationFailure(e,false);clearDragPreview()}}
 finally{if(generation===directTransformGeneration)directTransformPending.value=false}
}
watch(()=>[props.open,document.value,selectedIdsKey.value,dx.value,dy.value,dz.value,angle.value,scale.value],()=>{if(directTransformPending.value)cancelDirectTransform()},{flush:'sync'})
onUnmounted(()=>{cancelDirectTransform();directTransformWorker.dispose()})
const transformInputErrors=ref<Record<string,boolean>>({})
const transformInputInvalid=computed(()=>Object.keys(transformInputErrors.value).length>0)
function transformValidity(key:string,valid:boolean){
 if(valid)delete transformInputErrors.value[key];else{transformInputErrors.value[key]=true;cancelDirectTransform()}
}
function transform(){
 if(directTransformPending.value||transformInputInvalid.value)return
 cancelCommand()
 void commitDirectTransform(snapDocument.value,selectedIds.value,[dx.value,dy.value,dz.value],angle.value,scale.value,!!selectedSketch.value&&selectedIds.value.length===1,true)
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
const { sourceStepPending, sourceStepError, sourceStepTolerance, selectedSourceBody, sourceSeamPending, sourceSeamError, sourceSeamResult, cancelSourceSeam, sourceSeamMessage, qualifySourceSeam, sourceWallPending, sourceWallError, sourceWallResult, sourceWallOpen, sourceWallMenuOpen, sourceWallCoverageResult, sourceWallCoverageMode, sourceWallScanResult, sourceWallScanMode, sourceWallGroups, sourceWallSide, sourceWallMinimum, sourceWallTolerance, cancelSourceWall, sourceWallFaces, sourceWallFileMenuToggle, toggleSourceWallFace, sourceWallFaceUncertain, sourceWallFaceFill, sourceWallMeasurement, sourceWallMessage, sourceWallInputError, qualifySourceWall, cancelSourceStep, exportSourceStep, exportStepCurrent, exportStepAssembly, exportStepOriginal } = useSourceBodyTools({ document, error, label, objectInView, objectSelectable, project, props, isRestoreDisposed, restoringDraft, selectedBody, sourceDisplayResults, sourceEdgeSelection, isExpressionFaceHighlighted: sourceExpressionFaceHighlighted })
const {
  stlInput,
  meshImportProgress,
  meshImportStatus,
  svgInput,
  svgBusy,
  stepInput,
  stepBusy,
  bodyExportFormat,
  closeFileMenu,
  downloadProject,
  importFile,
  downloadBody,
  importStl,
  importStep,
  exportBlender,
  importSvg,
  exportBodySvg,
} = useFileExchange({
  props,
  ru,
  label,
  document,
  history,
  historyWorker,
  historyPending,
  historyMode,
  bumpHistoryGeneration: () => ++historyGeneration,
  getHistoryGeneration: () => historyGeneration,
  getSaveGeneration,
  isRestoreDisposed,
  restoringDraft,
  error,
  setNotice: message => {
    notice.value = message
  },
  workspace,
  activeGroup,
  selection,
  extraSelection,
  mode,
  sketchPaneOpen,
  operation,
  advancedOp,
  subtract,
  boxSelect,
  activePlane,
  workplaneOutline,
  selectedBody,
  sourceWallPending,
  sourceSeamPending,
  sourceStepPending,
  cancelSourceWall,
  cancelSourceSeam,
  cancelSourceStep,
  cancelCommand,
  prepareCommit,
  finishCommit,
  commit,
  sync,
  run,
  fit,
  pickObject,
})
const selectedExpressionOptions=computed(()=>selectedSourceBody.value?sourceBodyRecordInspectionOptions(selectedSourceBody.value):undefined)
const sourceExpressionFaces=ref<number[]>([])
const sourceExpressionRadiusFaces=ref<number[]>([])
const sourceExpressionDisplay=shallowRef<ExpressionDisplayReport|null>(null)
const sourceExpressionBoundaryMesh=shallowRef<BoundaryMeshReport|null>(null)
const sourceExpressionMesh=shallowRef<ExpressionMeshReport|null>(null)
const sourceExpressionMeshRegion=shallowRef<ExpressionMeshReport['pending'][number]|null>(null)
watch(selectedSourceBody,()=>{sourceExpressionRadiusFaces.value=[];sourceExpressionFaces.value=[];sourceExpressionDisplay.value=null;sourceExpressionBoundaryMesh.value=null;sourceExpressionMesh.value=null;sourceExpressionMeshRegion.value=null},{flush:'sync'})
const sourceExpressionMeshRegionBox=computed(()=>{
 const region=sourceExpressionMeshRegion.value,box=region?.bounds
 if(!box||!selectedSourceBody.value||!objectInView(selectedSourceBody.value.id))return null
 const points=Array.from({length:8},(_,k)=>project([box[0][k&1],box[1][(k>>1)&1],box[2][(k>>2)&1]],'3d'))
 const x=Math.min(...points.map(p=>p[0])),y=Math.min(...points.map(p=>p[1]))
 return {x,y,width:Math.max(...points.map(p=>p[0]))-x,height:Math.max(...points.map(p=>p[1]))-y,face:region!.face,reason:region!.reason}
})
const sourceExpressionTriangles=computed(()=>!selectedSourceBody.value||!objectInView(selectedSourceBody.value.id)?[]:
 (sourceExpressionMesh.value?.tiles??[]).flatMap((tile,index)=>{
  const points=tile.corners.map(point=>project(point,'3d'))
  return sourceExpressionMesh.value!.trianglesPerTile.map((triangle,part)=>({key:index+':'+part,face:tile.face,
   points:triangle.map(vertex=>points[vertex].join(',')).join(' '),error:tile.deviationUpperMm}))
 }))
const sourceBoundaryTriangles=computed(()=>{
 if(!selectedSourceBody.value||!objectInView(selectedSourceBody.value.id))return []
 const mesh=sourceExpressionBoundaryMesh.value
 if(!mesh)return []
 const failed=new Set(mesh.refinement?.numericFailures.map(t=>JSON.stringify([t.face,t.vertices,t.uv]))??[])
 const triangles=mesh.refinement?[...mesh.refinement.accepted.map(t=>({...t,state:'qualified'})),...mesh.refinement.pending.map(t=>({...t,state:failed.has(JSON.stringify([t.face,t.vertices,t.uv]))?'error':'pending'}))]:mesh.candidateTriangles.map(t=>({...t,state:'candidate'}))
 return triangles.flatMap((t,index)=>{
  const vertices=t.vertices.map(id=>mesh.vertices[id].point)
  if(vertices.some(v=>!v))return []
  return [{key:index,face:t.face,state:t.state,points:vertices.map(v=>project(v!,'3d').join(',')).join(' ')}]
 })
})
const sourceExpressionBoxes=computed(()=>!selectedSourceBody.value||!objectInView(selectedSourceBody.value.id)?[]:
 (sourceExpressionDisplay.value?.cells??[]).flatMap((cell,index)=>{
  const box=cell.bounds
  if(!box||cell.location==='outside')return []
  const points=Array.from({length:8},(_,k)=>project([box[0][k&1],box[1][(k>>1)&1],box[2][(k>>2)&1]],'3d'))
  const x=Math.min(...points.map(p=>p[0])),y=Math.min(...points.map(p=>p[1]))
  return [{key:index,face:cell.face,location:cell.location,domain:cell.domain,x,y,
   width:Math.max(...points.map(p=>p[0]))-x,height:Math.max(...points.map(p=>p[1]))-y}]
 }))
const sourceSeamEndpointMarkers=computed(()=>{
 const body=selectedSourceBody.value
 if(!props.open || !body || !objectInView(body.id) || sourceSeamResult.value?.qualified)return []
 return (sourceSeamResult.value?.endpointLimits?.sides??[]).flatMap(side=>{
  const point=side.collapsedPoint
  if(!side.distinctTangentLimits || !point || !point.every(Number.isFinite))return []
  return [{key:`${side.side}:${side.boundaryEnd}`,body:body.id,point,screen:project(point,'3d')}]
 })
})
function sourceExpressionFaceHighlighted(body:string,face:number){return body===selectedSourceBody.value?.id&&sourceExpressionFaces.value.includes(face)}
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
    ...Object.entries(sourceDisplayResults.value).filter(([id])=>objectInView(id)&&(!selectedOnly||sourceEdgeSelection.value.startsWith(id+':'))).flatMap(([,r])=>[...r.edges.flatMap(e=>(e.displaySegments??[]).flatMap(segment=>segment[1])),...(r.displayFaces??[]).flatMap(face=>[...face.tiles.flatMap(tile=>tile.corners),...face.unresolvedBoxes.flatMap(box=>Array.from({length:8},(_,i)=>[box[0][i&1],box[1][(i>>1)&1],box[2][(i>>2)&1]]))])]),
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
if (!kernelReady.value || restoringDraft.value) void restoreInitialDraft()
const commandFailure = computed(() => Object.keys(invalidQuantities.value).length ? label('Исправьте числовой ввод', 'Correct the numeric input') : advancedOp.value ? advancedPreview.value.error : cornerActive.value ? cornerPreview.value.error : solidActive.value ? previewError.value : operation.value==='array'?sketchEditFailure.value:'')
const commandActive = computed(() => !!(advancedOp.value || operation.value || subtract.value || booleanPending.value || directTransformPending.value || gizmoPending.value || nativeNurbsPending.value))
const commandHint = computed(() => {
  if(nativeNurbsPending.value&&nativeBrepMode.value)return label('Вычисляется B-rep. Esc — отменить.', 'Computing B-rep. Esc cancels.')
  if(nativeNurbsPending.value)return label('Вычисляется NURBS-команда. Esc — отменить.', 'Computing NURBS command. Esc cancels.')
  if(gizmoPending.value)return label('Вычисляется преобразование. Esc — отменить.', 'Computing transform. Esc cancels.')
  if(directTransformPending.value)return directTransformOperation.value==='duplicate'?label('Создаётся копия. Esc — отменить.','Creating copy. Esc cancels.'):directTransformOperation.value.startsWith('group-')?label('Изменяется группа. Esc — отменить.','Updating group. Esc cancels.'):directTransformOperation.value==='instance-detach'?label('Создаётся независимое тело. Esc — отменить.','Making body independent. Esc cancels.'):label('Вычисляется преобразование. Esc — отменить.', 'Computing transform. Esc cancels.')
  if(sketchEditPending.value)return label('Вычисляется предпросмотр. Esc — отменить.', 'Computing preview. Esc cancels.')
  if(booleanPending.value)return booleanKind.value==='minkowski3d'?label('Вычисляется сумма Минковского. Esc — отменить.','Computing Minkowski sum. Esc cancels.'):label('Вычисляется Boolean. Esc — отменить.', 'Computing Boolean. Esc cancels.')
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
  cancelNumericSlot();cancelGesture(); operation.value = null; advancedOp.value = null; subtract.value = null
  previewBody.value = null; previewEmpty.value = false; previewError.value = ''
}
function cancelCommand(){cancelCommandState();tool.value='select';workspace.value?.focus()}

function quantityValidity(key: string, valid: boolean) { if (valid) delete invalidQuantities.value[key]; else invalidQuantities.value[key] = true }
const commandReady = computed(() => {
  if(advancedOp.value==='edge-fillet' && advanced.value.filletMode==='partial-preview')return false
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
  input?.focus({preventScroll:true});input?.select?.()
  const card=input?.closest<HTMLElement>('.operation-card');if(card)card.scrollTop=0
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
const extrusionHandle = computed(() => {
  if (operation.value !== 'extrude' || !selectedSketch.value) return null
  const p = selectedSketch.value.points, center = [p.reduce((s,v)=>s+v[0],0)/p.length,p.reduce((s,v)=>s+v[1],0)/p.length]
  return { base: project(worldPoint([...center,baseZ.value],selectedSketch.value.plane),'3d'), top: project(worldPoint([...center,baseZ.value+height.value],selectedSketch.value.plane),'3d') }
})
function dragHeight(e: PointerEvent) {
  e.preventDefault(); const svg = (e.target as SVGElement).ownerSVGElement!
  heightDrag = { y: e.clientY, height: height.value, pointer: e.pointerId, svg }; heightDragging.value = true; svg.setPointerCapture(e.pointerId)
}
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
function cancelGesture() { cancelFaceSketch(); cancelNativeNurbs();cancelGizmoWorker();dragConstraint.value=''; draftCursor.value=null;choosingSketchFace.value=false; clearDragPreview(); previewingTransform.value=false; if(vertexDrag){document.value=vertexDrag.before;vertexDrag=null} if(cvDrag){document.value=cvDrag.before;cvDrag=null} if(curveDrag){document.value=curveDrag.before;curveDrag=null} if(manipulatorDrag){document.value=manipulatorDrag.before;if(manipulatorDrag.kind==='push')advancedOp.value=null;if(manipulatorDrag.kind==='split')advanced.value.distance=manipulatorDrag.initial;manipulatorDrag=null}selectionBox.value=null; if (gesture) { if(!gesture.pan)document.value = gesture.document; gesture = null } if (heightDrag) height.value = heightDrag.height; heightDrag = null; heightDragging.value = false; orbitDrag = null; draft.value = []; drawMeasure.value = ''; snapMarker.value = null; cameraDragging.value = false }
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
    const ready=isTopologyReadyFor(body)
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
    else {const request=prepareTopology(body),pick=nextTopologyPick();void request.then(result=>{if(result&&pick===getTopologyPick()&&selectedBody.value===body)choose(result,false)})}
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
      const previewIds=translationPreviewIds(selectedIds.value)
      if(!hasDragNodes())setDragNodes(collectDragNodes(g.svg,previewIds))
      const origin=project([0,0,0],'3d'),shifted=project(worldDelta,'3d')
      const offset=`translate(${shifted[0]-origin[0]},${shifted[1]-origin[1]})`
      applyDragPreviewTransform(offset,previewIds,worldDelta as [number,number,number])
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
    gesture.end=[...p]
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
    const previewIds = translationPreviewIds(ids)
    if (!hasDragNodes()) setDragNodes(collectDragNodes(gesture.svg, previewIds))
    const origin = project([0, 0, 0], '3d'), shifted = project(worldDelta as unknown as Vec3, '3d')
    const offset = `translate(${shifted[0] - origin[0]},${shifted[1] - origin[1]})`
    // The visible surface comes from the GPU layer; move it in place, not by rebuilding.
    applyDragPreviewTransform(offset, previewIds, worldDelta as [number, number, number])
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

  if (heightDrag) { heightDrag = null; heightDragging.value = false; return }
  if (orbitDrag) { orbitDrag = null; return }
  drawMeasure.value = ''; snapMarker.value = null
  if (!gesture || gesture.pointer !== e.pointerId) return
  move(e)
  const start=gesture.start, before = gesture.document, pane = gesture.pane, drawing=gesture.sketch, pan = gesture.pan, bodyDrag = gesture.bodyDrag, end=gesture.end, selectionOnly = gesture.dragStart && !bodyDrag, workerEdit=gesture.workerEdit; gesture = null
  if (pan || selectionOnly) return
  if(workerEdit){gizmoApplyRevision=gizmoRevision;return}
  if(drawing&&tool.value==='slot'&&draft.value.length&&end){draftPoint.value=[...start];draftEnd.value=[...end];draftPointValid.value=[true,true];draftEndValid.value=[true,true];numericSlotAutoApply=true;draft.value=[];refreshNumericSlot();return}
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

const { paletteOpen, notice, createOpen, dockOpen, dockTab, exactCardOpen, captureCommand, lastCommand, repeatEntry, repeatReason, repeatCommand, solidCommands, executeSolidCommand } = useCommandPalette({ applyMinkowski, addNurbs: kind => addNurbs(kind), addPrimitive: kind => addPrimitive(kind), advanced, advancedOp, appendBodies, applyBrepBoolean, baseZ, beginAdvanced, beginCorner, beginExtrude, beginSketch, beginSubtract, boxSelect, brepSegments, camera, canExtrudeSketch, cancelCommand, clearanceOpen, commandActive, commitDirectTransform, copyCount, copySweep, copyX, copyY, cornerRadius, diagnosticPanel, diagnosticsOpen, document, downloadProject, duplicate, edgeIndex, exportBlender, exportBodySvg, extraSelection, extrusionMode, faceSketch, filletSegments, fit, floorVisible, gizmoMode, height, kernelReady, kernelLoadError, profileBridgePanel, label, measureSelectedBrep, measureTarget, measurementOpen, mode, movingBody, operation, pickMode, pickObject, pointTrimCut, pointTrimPick, primitiveLabel: kind => primitiveLabel(kind), props, redoable, remove, restoringDraft, retessellateSelectedBrep, revolveAngle, revolveAxis, revolveGeometry, revolveOffset, revolveSegments, run, selectEdgeChain, selectedBody, selectedBrepBodies, selectedCurvePair, selectedFace, selectedIds, selectedNurbs, selectedNurbsCurve, selectedNurbsSurface, selectedSketch, selectedSurfacePair, selection, sendToMesh, showHelp, sketchPaneOpen, smoothDisplay, snapDocument, subtract, surfaceInputs, surfaceReversed, toggleSketchPane, tool, twoSelectedBodies, undo, undoable, workspace })
// Tests and the App shell drive the workspace through the same command list the palette shows.
defineExpose({ solidCommands, executeSolidCommand })

function keydown(e: KeyboardEvent) {
  if (restoringDraft.value) { if(e.key==='Escape'){e.preventDefault();cancelDraftRecovery()} return }
  if (paletteOpen.value) return
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); paletteOpen.value = true; return }
  if (e.isComposing) return
  if (e.key === 'Escape') { e.preventDefault(); cancelSourceWall();cancelSourceSeam();cancelSourceDisplay();sourceEdgeSelection.value=''; const boundaryWasPending=boundaryAgreementPending.value||faceContactsPending.value; if(diagnosticsPending.value||intersectionPending.value||boundaryWasPending)diagnosticsOpen.value=false;exactCardOpen.value = false; cancelCommand(); if(boundaryWasPending)void nextTick(()=>workspace.value?.focus()); return }
  if (e.key === 'Enter' && commandActive.value) {
    // Native controls retain Enter, including Cancel, operand selection and the File menu.
    if ((e.target as HTMLElement).closest?.('button, summary, a[href], select')) return
    e.preventDefault(); applyCommand(); return
  }
  if ((e.target as HTMLElement).matches('input,textarea,select')) return
  if (e.key === '\\' && !e.ctrlKey && !e.metaKey && !e.altKey) { e.preventDefault(); dockOpen.value = !dockOpen.value; return }
  if(tool.value==='polyline' && draft.value.length && (e.key==='Backspace'||((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='z'&&!e.shiftKey))){e.preventDefault();undoDraftPoint();return}
  if(tool.value==='polyline' && e.key==='Enter'){if((e.target as HTMLElement).closest?.('button, summary, a[href], select'))return;e.preventDefault();finish(false);return}
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
const { primitiveG2Enabled, primitiveG2Size, primitiveG2Individual, primitiveG2Sizes, failedPrimitive, primitiveG2Invalid, roundGeometry, primitiveSize, primitiveTopRadius, primitiveInnerRadius, TOOL_ICONS, primitiveLabel, primitivePending, cancelPrimitive, addPrimitive, bridgeEndA, bridgeEndB, bridgeTension, bridgeError, bridgeTensionInvalid, addBridgeCurve, updateDimensionSketch, dimensionLabels, addNurbs } = usePrimitives({ activeGroup, advancedOp, brepSegments, cancelCommand, commit, document, error, finishCommit, fit, history, label, lockedIds, nativeNurbsPending, operation, pickObject, prepareCommit, props, isRestoreDisposed, run, runNativeNurbs, selectedCurvePair, tool, visibleSketches, workspace })
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
const { openNewGroupFromMenu, bodySections, sceneRows, isolateGroup, removeGroup, openGroupDialog } = useSceneTree({ cancelCommandState, cancelGesture, commit, document, documentObjects, emit, extraSelection, fit, history, hovered, isolatedBodyIds, label, lockedIds, objectSelectable, run, selectedIds, selection, sync })

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
watch(()=>[props.open,kernelReady.value,snap.value,geometrySnap.value,snapSketches.value],()=>void refreshSketchSnaps(),{immediate:true,flush:'post'})
watch(()=>[props.open,kernelReady.value,snap.value,geometrySnap.value,snapBodies.value],()=>void refreshSnapPreparation(),{immediate:true,flush:'post'})
watch(()=>[props.open,tool.value,document.value,JSON.stringify(activePlane.value),JSON.stringify(draftPoint.value),JSON.stringify(draftPointValid.value),JSON.stringify(draftEnd.value),JSON.stringify(draftEndValid.value),slotWidth.value,slotWidthValid.value,numericSlotRetry.value],refreshNumericSlot,{flush:'sync'})
watch(()=>[props.open,kernelReady.value,document.value],()=>void refreshSourceDisplay(),{immediate:true,flush:'post'})
watch(()=>[props.open,kernelReady.value,displayCurves.value],()=>void refreshCurveDisplay(),{immediate:true,flush:'post'})
watch(()=>[props.open,kernelReady.value,displayProfiles.value],()=>void refreshProfileDisplay(),{immediate:true,flush:'post'})
watch(()=>[props.open,kernelReady.value,document.value.surfaces,advancedPreview.value.document?.surfaces],()=>void refreshSurfaceDisplay(),{immediate:true,flush:'post'})

watchEffect(onCleanup=>{
 void surfaceDistanceRevision.value
 const enabled=props.open&&surfaceDistanceOpen.value&&!commandActive.value,pair=selectedSurfacePair.value
 const surfaces=document.value.surfaces,maxCells=surfaceDistanceBudget.value
 let current=true
 onCleanup(()=>{current=false;surfaceDistanceWorker.cancel()})
 surfaceDistance.value=null;surfaceDistancePending.value=false;surfaceDistanceRetryVisible.value=false
 if(!enabled||!pair)return
 const [a,b]=pair.map(id=>surfaces!.find(s=>s.id===id)!.surface)
 surfaceDistancePending.value=true
 void surfaceDistanceWorker.run({kind:'surfaceDistance',a,b,toleranceMm:.001,maxCells}).then(value=>{
  if(current)surfaceDistance.value={value,error:''}
 }).catch(()=>{if(current){surfaceDistanceRetryVisible.value=true;surfaceDistance.value={value:null,error:label('Не удалось измерить поверхности. Проверьте геометрию или увеличьте объём расчёта.','Could not measure the surfaces. Check their geometry or increase the calculation budget.')}}})
 .finally(()=>{if(current)surfaceDistancePending.value=false})
})
watchEffect(onCleanup=>{
 void boundaryInspectionRevision.value
 const enabled=props.open&&boundaryInspection.value&&!commandActive.value,pair=selectedSurfacePair.value
 const surfaces=document.value.surfaces,options={...boundaryOptions.value}
 let current=true
 onCleanup(()=>{current=false;boundaryWorker.cancel()})
 boundaryReport.value=null;boundaryPending.value=false;boundaryRetryVisible.value=false
 if(!enabled||!pair)return
 if(!Object.values(boundaryInputValid.value).every(Boolean)){boundaryReport.value={value:null,error:label('Исправьте выделенные поля: зазор не меньше 0,000001 мм, угол от 0 до 90°, целое число точек от 2 до 257.','Correct the highlighted fields: gap at least 0.000001 mm, angle from 0 to 90°, integer sample count from 2 to 257.')};return}
 const [a,b]=pair.map(id=>surfaces!.find(s=>s.id===id)!.surface)
 boundaryPending.value=true
 void boundaryWorker.run({kind:'surfaceBoundary',a,b,options}).then(value=>{
  if(current)boundaryReport.value={value,error:''}
 }).catch(()=>{if(current){boundaryRetryVisible.value=true;boundaryReport.value={value:null,error:label('Не удалось проверить стык поверхностей. Повторите проверку; при повторном отказе проверьте выбранные границы.','Could not inspect the surface boundary. Retry; if it fails again, inspect the selected boundaries.')}}})
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
    <div v-if="kernelLoadError" class="restore-loading" role="alert" aria-label="kernel-load-error">{{ label('Не удалось загрузить геометрическое ядро: ', 'Could not load geometry kernel: ') + kernelLoadError }} <button :disabled="kernelLoading" @click="retryKernelLoad">{{ label('Повторить загрузку ядра', 'Retry kernel loading') }}</button></div>
    <div v-if="restoringDraft" class="restore-loading" role="status" aria-label="draft-recovery">{{ label('Восстанавливаю геометрию…', 'Restoring geometry…') }} <button @click="cancelDraftRecovery">{{ label('Отмена · Esc','Cancel · Esc') }}</button></div>
    <header class="workspace-bar">
      <button v-if="embedded" class="back" @click="emit('close')">← {{ label('Code', 'Code') }}</button>
      <strong>{{ label('Solid', 'Solid') }}</strong>
      <span class="subtle">{{ label('Plasticity-like CAD', 'Plasticity-like CAD') }}</span>
      <div class="history-tools"><button :disabled="!undoable || historyPending" @click="undo()" :title="label('Отменить · Ctrl/⌘ Z', 'Undo · Ctrl/⌘ Z')">↶</button><button :disabled="!redoable || historyPending" @click="undo(true)" :title="label('Повторить · Ctrl/⌘ Shift Z', 'Redo · Ctrl/⌘ Shift Z')">↷</button></div>
      <button class="input-mode-toggle" :aria-pressed="inputMode === 'touch'" :title="label('Переключить управление: тач / мышь', 'Switch controls: touch / mouse')" @click="toggleInputMode">{{ inputMode === 'touch' ? label('Тач', 'Touch') : label('Мышь', 'Mouse') }}</button>
      <button v-if="inputMode === 'touch'" :aria-pressed="touchNavigate" @click="toggleTouchNavigation">{{ label('Навигация', 'Navigate') }}</button>
      <button :aria-pressed="compactWorkspace" @click="compactWorkspace = !compactWorkspace">{{ compactWorkspace ? label('Инструменты', 'Tools') : label('Больше места', 'More space') }}</button>
      <button class="create-toggle" :aria-pressed="createOpen" :aria-expanded="createOpen" :title="label('Фигуры и примитивы', 'Shapes and primitives')" @click="createOpen = !createOpen">{{ label('Создать', 'Create') }}</button>
      <span class="ws-count" aria-live="polite">{{ label('Выбрано', 'Selected') }}: {{ selectedIds.length }}</span>
      <button @click="showHelp = !showHelp" title="Keyboard shortcuts">?</button>
      <button class="command-search" :title="label('Поиск команд · Ctrl/⌘ K', 'Search commands · Ctrl/⌘ K')" @click="paletteOpen = true"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg><span>{{ label('Команда…', 'Command…') }}</span><kbd>Ctrl K</kbd></button>
      <span v-if="sketchSnapPending" role="status" aria-label="sketch-snap-preparation">{{ label('Готовлю привязки эскизов…','Preparing sketch snaps…') }} <button @click="cancelSketchSnaps">Esc</button></span>
      <button v-if="sketchSnapsNeedRetry" @click="refreshSketchSnaps">{{ label('Обновить привязки эскизов','Refresh sketch snaps') }}</button>
      <span v-if="sketchSnapErrors.length" role="alert">{{ label('Не готовы привязки эскизов: ','Sketch snaps unavailable: ')+sketchSnapErrors.map(e=>e.id).join(', ') }} {{ sketchSnapErrors[0].message }}</span>
      <span v-if="snapPreparationPending" role="status" aria-label="snap-preparation">{{ label('Готовлю привязки к телам…','Preparing body snaps…') }} <button @click="cancelSnapPreparation">Esc</button></span>
      <button v-if="snapPreparationNeedsRetry" @click="refreshSnapPreparation">{{ label('Обновить привязки','Refresh snaps') }}</button>
      <span v-if="snapPreparationErrors.length" role="alert">{{ label('Не готовы привязки: ','Snaps unavailable: ')+snapPreparationErrors.map(e=>e.id).join(', ') }} {{ snapPreparationErrors[0].message }}</span>
      <span v-if="faceSketchPending" role="status" aria-label="face-sketch-preparation">{{ label('Готовлю плоскость эскиза…','Preparing sketch plane…') }} <button @click="cancelFaceSketch">Esc</button></span>
      <span v-if="sourceDisplayPending" role="status">{{ label('Готовлю исходные рёбра…','Preparing source edges…') }} <button @click="cancelSourceDisplay">{{ label('Отмена','Cancel') }}</button></span>
      <button v-if="document.sourceBodies?.length && !sourceDisplayPending" @click="refreshSourceDisplay">{{ label('Обновить исходные рёбра','Retry source edges') }}</button>
      <SourceBodyFailureNotice v-for="(message,id) in sourceDisplayErrors" :key="id" :id="String(id)" :name="(document.sourceBodies??[]).find(body=>body.id===id)?.name??String(id)" :error="message" :locale="locale" :pending="sourceDisplayPending" @retry="refreshSourceDisplay"/>
      <span v-if="sourceUnresolvedCount" role="status">{{ label('Не проверены участки граней: ','Unresolved face cells: ')+sourceUnresolvedCount }} · {{ label('Выделены оранжевым.','Shown in orange.') }}</span>
      <span v-if="sourceEdgeSelection" role="status">{{ label('Исходное ребро','Source edge') }}: {{ sourceEdgeSelection }} · {{ label('Предпросмотр исходных поверхностей. Концы заданы интервалами.','Source surface preview. Endpoints are enclosed by intervals.') }}</span>
      <span v-if="bodyEdgesPending" role="status" aria-label="body-edges">{{ label('Готовлю рёбра…','Preparing edges…') }} <button @click="cancelBodyEdges">Esc</button></span>
      <button v-if="pickMode==='edge' && selectedBody && !bodyEdgesPending && !authoredEdges.length" @click="refreshBodyEdges">{{ label('Обновить рёбра','Refresh edges') }}</button>
      <button v-if="topologyRetryVisible && selectedBody" :disabled="topologyPending" @click="error='';prepareTopology(selectedBody)">{{ label('Повторить выбор граней','Retry face selection') }}</button>
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
      <span v-if="historyPending" role="status" aria-label="history-restore">
        <progress v-if="(historyMode==='import' || historyMode==='export') && meshImportProgress" :value="meshImportProgress.completed" :max="meshImportProgress.total" :aria-label="historyMode==='export'?label('Прогресс экспорта меша','Mesh export progress'):label('Прогресс импорта меша','Mesh import progress')"></progress>
        {{ (historyMode==='import' || historyMode==='export') && meshImportStatus ? meshImportStatus : historyMode==='import'?label('Проверяю импорт…','Checking import…'):historyMode==='export'?label('Подготовка экспорта…','Preparing export…'):historyMode==='shared'?label('Загружаю общую версию…','Loading shared version…'):label('Восстанавливаю историю…','Restoring history…') }}
        <button @click="cancelHistoryRestore">Esc</button>
      </span>
      <span class="save-status" :class="{ error: saveError }" role="status" :title="savePending ? label('Сохраняю…','Saving…') : saveError ? label('Не сохранено', 'Unsaved') : label('Сохранено в браузере', 'Saved in browser')" :aria-label="savePending ? label('Сохраняю…','Saving…') : saveError ? label('Не сохранено', 'Unsaved') : label('Сохранено в браузере', 'Saved in browser')">
        <span v-if="savePending">…</span>
        <svg v-else-if="saveError" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 8v5M12 17h.01"/><path d="M10.3 3.9 2.5 18a2 2 0 0 0 1.7 3h15.6a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z"/></svg>
        <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M7 18a4.5 4.5 0 0 1-.6-9A6 6 0 0 1 18 8.5 3.8 3.8 0 0 1 17.5 18z"/><path d="m9 13 2 2 4-4"/></svg>
      </span>
      <details class="file-menu" @keydown.esc="closeFileMenu" @toggle="sourceWallFileMenuToggle"><summary :title="label('Файл', 'File')"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" aria-hidden="true"><path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><path d="M14 3v6h6"/></svg><span>{{ label('Файл', 'File') }}</span><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg></summary><div>
        <button type="button" @click="openNewGroupFromMenu">{{ label('Новая группа из кода', 'New group from source') }}</button>
        <button :disabled="restoringDraft" @click="downloadProject">{{ label('Скачать проект JSON', 'Download JSON project') }}</button>
        <label class="file-open">{{ label('Открыть Solid / RushGraph NURBS', 'Open Solid / RushGraph NURBS') }}<input type="file" accept=".json,application/json" @change="importFile"></label>
        <button type="button" @click="stlInput?.click()">{{ label('Импорт STL / OBJ / PLY / OFF / AMF / 3MF как тело', 'Import STL / OBJ / PLY / OFF / AMF / 3MF as body') }}</button>
        <ManifoldSettings :locale="locale" />
        <input ref="stlInput" type="file" :accept="MESH_IMPORT_ACCEPT" hidden @change="importStl" />
        <button type="button" :disabled="svgBusy" @click="svgInput?.click()">{{ label('Импорт SVG как профили', 'Import SVG as profiles') }}</button>
        <button v-if="draftConflict" type="button" :disabled="loadingLatestDraft" @click="loadLatestDraft">{{ label('Загрузить сохранённую версию · локальная останется в Undo','Load saved version · keep local in Undo') }}</button>
        <button v-if="saveError && !draftConflict" type="button" :disabled="savePending" @click="persist">{{ label('Повторить сохранение черновика','Retry saving draft') }}</button>
        <input ref="svgInput" type="file" accept=".svg,image/svg+xml" hidden @change="importSvg" />
        <button type="button" :disabled="stepBusy" @click="stepInput?.click()">{{ label('Импорт STEP', 'Import STEP') }}</button>
        <input ref="stepInput" type="file" accept=".step,.stp" hidden @change="importStep" />
        <button type="button" :disabled="restoringDraft || !selectedBody" @click="exportBodySvg">{{ label('SVG выбранного тела · проекция XY', 'Selected body SVG · XY projection') }}</button>
        <button type="button" :disabled="restoringDraft || !document.bodies.length" @click="exportBlender">{{ label('Экспорт для Blender', 'Export for Blender') }}</button>
        <template v-if="document.sourceBodies?.length">
          <ExpressionShellCheck v-if="selectedExpressionOptions" :key="selectedSourceBody?.id" :options="selectedExpressionOptions" :active="props.open" :ru="ru" @radius-faces="sourceExpressionRadiusFaces=$event" @faces="sourceExpressionFaces=$event" @display="sourceExpressionDisplay=$event" @boundary-mesh="sourceExpressionBoundaryMesh=$event" @mesh="sourceExpressionMesh=$event" @mesh-region="sourceExpressionMeshRegion=$event" />
          <button type="button" :disabled="restoringDraft || !selectedSourceBody || sourceSeamPending" @click="qualifySourceSeam">{{ label('Проверить касательность ребра','Check source edge tangency') }}</button>
          <span v-if="sourceSeamPending" role="status">{{ label('Проверяю касательность…','Checking tangency…') }} <button type="button" @click="cancelSourceSeam">{{ label('Отмена','Cancel tangency') }}</button></span>
          <span v-if="sourceSeamResult" :role="sourceSeamResult.qualified?'status':'alert'">{{ sourceSeamMessage }} <span v-for="side in sourceSeamResult.endpointLimits?.sides.filter(s=>s.distinctTangentLimits) ?? []" :key="`${side.side}:${side.boundaryEnd}`" data-source-seam-endpoint>{{ label('Концевая точка','Endpoint') }}: {{ side.collapsedPoint?.join(', ') }} {{ label('мм','mm') }}</span> <span v-if="sourceSeamResult.uncertainCanonical">{{ label('Непроверенный интервал ребра','Unverified edge interval') }}: {{ sourceSeamResult.uncertainCanonical }}</span><details><summary>{{ label('Подробности','Details') }}</summary>{{ sourceSeamResult.reason }} · sin² ≤ {{ sourceSeamResult.sineSquaredBounds?.[1] ?? '—' }} · {{ sourceSeamResult.cells }}</details><button v-if="!sourceSeamResult.qualified" type="button" :disabled="sourceSeamPending || !selectedSourceBody" @click="qualifySourceSeam">{{ label('Повторить','Retry tangency') }}</button></span>
          <span v-if="sourceSeamError" role="alert">{{ label('Проверка ребра не завершена. Повторите расчёт.','Edge qualification failed. Retry the calculation.') }} <button type="button" :disabled="sourceSeamPending || !selectedSourceBody" @click="qualifySourceSeam">{{ label('Повторить','Retry tangency') }}</button><details><summary>{{ label('Подробности','Details') }}</summary>{{ sourceSeamError }}</details></span>
          <details class="source-wall-panel" :open="sourceWallOpen" @toggle="sourceWallOpen=($event.target as HTMLDetailsElement).open">
            <summary>{{ label('Толщина исходного тела','Source body wall thickness') }}</summary>
            <p>{{ label('Для проверки выбранного участка укажите две противоположные стороны. Для всей стенки выбирать грани не требуется. Расчёт оценивает отрезки материала, почти перпендикулярные граням.','Choose opposing sides to check a selected region. Whole-wall qualification needs no selected faces. The calculation evaluates material chords nearly perpendicular to the faces.') }}</p>
            <button type="button" :aria-pressed="sourceWallSide===0" @click="sourceWallSide=0">{{ label('Первая сторона','First side') }}</button>
            <button type="button" :aria-pressed="sourceWallSide===1" @click="sourceWallSide=1">{{ label('Вторая сторона','Second side') }}</button>
            <div class="source-wall-face-grid" role="group" :aria-label="label('Грани стенки','Wall faces')">
              <button v-for="face in sourceWallFaces" :key="face" type="button" :aria-pressed="sourceWallGroups[sourceWallSide].includes(face)"
                :style="{borderColor:sourceWallFaceFill(selectedSourceBody?.id??'',face)}" @click="toggleSourceWallFace(face)">{{ label('Грань ','Face ') }}{{ face+1 }}</button>
            </div>
            <p>{{ label('Первая сторона','First side') }}: {{ sourceWallGroups[0].map(f=>f+1).join(', ') || '—' }} · {{ label('Вторая сторона','Second side') }}: {{ sourceWallGroups[1].map(f=>f+1).join(', ') || '—' }}</p>
            <label>{{ label('Минимальная толщина, мм','Minimum thickness, mm') }} <input v-model.number="sourceWallMinimum" type="number" min="1e-9" step="any" :aria-label="label('Минимальная толщина, мм','Minimum source wall thickness, mm')" :aria-invalid="sourceWallError && (!Number.isFinite(sourceWallMinimum) || sourceWallMinimum<=0)?'true':undefined" :aria-describedby="sourceWallError && sourceWallInputError?'source-wall-input-error':undefined" /></label>
            <label>{{ label('Допуск толщины, мм','Thickness tolerance, mm') }} <input v-model.number="sourceWallTolerance" type="number" min="1e-9" step="any" :aria-label="label('Допуск толщины, мм','Source wall thickness tolerance, mm')" :aria-invalid="sourceWallError && !sourceWallScanMode && !sourceWallCoverageMode && (!Number.isFinite(sourceWallTolerance) || sourceWallTolerance<=0)?'true':undefined" :aria-describedby="sourceWallError && sourceWallInputError?'source-wall-input-error':undefined" /></label>
            <button type="button" :disabled="sourceWallPending || !selectedSourceBody || sourceWallGroups.some(g=>!g.length)" @click="qualifySourceWall(false)">{{ label('Проверить толщину','Check source wall thickness') }}</button>
            <button type="button" :disabled="sourceWallPending || !selectedSourceBody" @click="qualifySourceWall(true)">{{ label('Найти тонкие участки','Find thin regions') }}</button>
            <button type="button" :disabled="sourceWallPending || !selectedSourceBody" @click="qualifySourceWall(false,true)">{{ label('Проверить всю стенку','Check whole wall') }}</button>
            <span v-if="sourceWallCoverageResult" :role="sourceWallCoverageResult.wholeWallQualified?'status':'alert'" data-source-wall-coverage-result="true">
              {{ sourceWallCoverageResult.wholeWallQualified?label('По всем граням подтверждён нижний порог толщины для отрезков, почти перпендикулярных граням.','The lower thickness threshold for near-normal chords is qualified across all faces.'):label('Толщина всей стенки не подтверждена. Проверьте выделенные грани; часть пар требует дополнительной диагностики.','Whole-wall thickness is unproven. Inspect highlighted faces; some pairs need further diagnostics.') }}
              {{ sourceWallCoverageResult.lowerMm }} {{ sourceWallCoverageResult.lowerMm!==null?label('мм','mm'):'' }} · {{ sourceWallCoverageResult.pairs.length }}/{{ sourceWallCoverageResult.totalPairs }}
              <button type="button" :disabled="sourceWallPending" @click="qualifySourceWall(false,true)">{{ label('Повторить проверку всей стенки','Retry whole wall') }}</button>
              <details><summary>{{ label('Подробности','Details') }}</summary>
                {{ !sourceWallCoverageResult.enumerationComplete?label('Проверены не все пары граней: достигнут лимит расчёта. Выберите оставшиеся грани для отдельной проверки.','Not all face pairs were checked: the calculation limit was reached. Select the remaining faces for a separate check.'):
                  sourceWallCoverageResult.wholeWallQualified?label('Проверены все пары граней в заданной области проверки.','All face pairs are qualified within the stated checking scope.'):
                  label('Все пары рассмотрены, но часть участков остаётся неопределённой. Проверьте выделенные грани.','All pairs were considered, but some regions remain unresolved. Inspect the highlighted faces.') }}
                · {{ sourceWallCoverageResult.pairs.filter(p=>!p.proven).length }} {{ label('неподтверждённых пар','unproven pairs') }}
                <span v-if="sourceWallCoverageResult.retainedGap" data-source-wall-retained-result="true">
                  · {{ label('Расстояние между обрезанными гранями подтверждено:','Retained face separation qualified:') }}
                  {{ sourceWallCoverageResult.retainedGap.pairs.filter(p=>p.qualified).length }}/{{ sourceWallCoverageResult.retainedGap.pairs.length }}
                  · {{ sourceWallCoverageResult.retainedGap.pairs.reduce((n,p)=>n+p.unresolvedProducts.length,0) }} {{ label('пар участков требуют проверки','region pairs require checking') }}
                </span>
                <span v-if="sourceWallCoverageResult.mixedProducts" data-source-wall-mixed-result="true">
                  · {{ label('Дополнительно подтверждено пар граней:','Additional face pairs qualified:') }}
                  {{ sourceWallCoverageResult.mixedProducts.pairs.filter(p=>p.qualified).length }}/{{ sourceWallCoverageResult.mixedProducts.pairs.length }}
                  · {{ sourceWallCoverageResult.mixedProducts.pairs.reduce((n,p)=>n+p.unresolvedProducts.length,0) }} {{ label('пар участков требуют проверки','region pairs require checking') }}
                  <span v-if="sourceWallCoverageResult.mixedProducts.pairs.some(p=>p.reason==='source-wall-pair-numeric-limit')">
                    · {{ label('Достигнут численный предел. Проверьте выделенные грани отдельно.','A numerical limit was reached. Check the highlighted faces separately.') }}
                  </span>
                </span>
                <span v-if="sourceWallCoverageResult.adaptiveSelf" data-source-wall-self-result="true">
                  · {{ label('Проверка внутри граней','Within-face coverage') }}:
                  {{ sourceWallCoverageResult.adaptiveSelf.faces.filter(f=>f.qualified).length }}/{{ sourceWallCoverageResult.adaptiveSelf.faces.length }}
                  · {{ sourceWallCoverageResult.adaptiveSelf.faces.reduce((n,f)=>n+f.pending,0) }} {{ label('пар участков требуют проверки','region pairs require checking') }}
                  <span v-for="face in sourceWallCoverageResult.adaptiveSelf.faces.filter(f=>!f.qualified)" :key="face.face" class="source-wall-face-pending" :data-source-wall-pending-face="face.face">
                    · {{ label('Грань','Face') }} {{ face.face+1 }}: {{ face.pending }} {{ label('пар участков требуют проверки','region pairs require checking') }}
                  </span>
                </span>
              </details>
            </span>
            <span v-if="sourceWallScanResult" role="status" data-source-wall-scan-result="true">
              {{ sourceWallScanResult.thinFound?label('Найден тонкий участок.','A thin region was found.'):label('Тонкий участок в проверенных пробах не найден.','No thin region was found in tested proposals.') }}
              {{ sourceWallScanResult.proposalsExhausted?label('Заданные пробы выполнены.','The configured proposals are complete.'):label('Поиск остановлен по лимиту попыток.','Search stopped at the attempt limit.') }}
              {{ label('Безопасность всей стенки не подтверждена.','Whole-wall safety is unproven.') }}
              {{ sourceWallScanResult.witness?.lengthMm }} mm · {{ sourceWallScanResult.facesVisited }}/{{ sourceWallScanResult.facesTotal }} · {{ sourceWallScanResult.attempts }}
              <button type="button" :disabled="sourceWallPending" @click="qualifySourceWall(true)">{{ label('Повторить поиск','Retry search') }}</button>
            </span>
            <span v-if="sourceWallPending" role="status">{{ label('Проверяю толщину…','Checking wall thickness…') }} <button type="button" @click="cancelSourceWall">{{ label('Отмена','Cancel wall thickness') }}</button></span>
            <span v-if="sourceWallResult" :role="sourceWallResult.qualified?'status':'alert'">{{ sourceWallMessage }}
              <span v-if="sourceWallResult.intervalMm">{{ sourceWallResult.intervalMm[0] }}–{{ sourceWallResult.intervalMm[1] }} {{ label('мм','mm') }}</span>
              <button v-if="!sourceWallResult.converged" type="button" :disabled="sourceWallPending" @click="qualifySourceWall(sourceWallScanMode,sourceWallCoverageMode)">{{ label('Повторить','Retry wall thickness') }}</button>
              <details><summary>{{ label('Подробности','Details') }}</summary>{{ sourceWallResult.reason }} · {{ sourceWallResult.search.attempts }} · {{ sourceWallResult.clearance.uncertainUv }}</details>
            </span>
            <span v-if="sourceWallError" id="source-wall-input-error" role="alert">{{ sourceWallInputError || label('Расчёт не завершён. Проверьте стороны и числовые параметры, затем повторите.','Calculation failed. Check side selection and numeric values, then retry.') }}
              <button type="button" :disabled="sourceWallPending" @click="qualifySourceWall(sourceWallScanMode,sourceWallCoverageMode)">{{ label('Повторить','Retry wall thickness') }}</button><details><summary>{{ label('Подробности','Details') }}</summary>{{ sourceWallError }}</details></span>
          </details>
          <label>{{ label('Допуск STEP, мм','STEP tolerance, mm') }} <input aria-label="STEP tolerance, mm" v-model.number="sourceStepTolerance" type="number" min="1e-12" step="any" /></label>
          <button type="button" :disabled="restoringDraft || !selectedSourceBody || sourceStepPending" @click="exportSourceStep">{{ label('STEP выбранного исходного тела','Export selected source body STEP') }}</button>
          <span v-if="sourceStepPending" role="status">{{ label('Готовлю STEP…','Preparing STEP…') }} <button type="button" @click="cancelSourceStep">{{ label('Отмена','Cancel STEP') }}</button></span>
          <span v-if="sourceStepError" role="alert">{{ label('STEP не подготовлен. Выберите исходное тело и повторите расчёт; при отказе по допуску увеличьте его.','STEP preparation failed. Select the source body and retry; increase tolerance if its bound cannot be proven.') }} <button type="button" :disabled="!selectedSourceBody || sourceStepPending" @click="exportSourceStep">{{ label('Повторить STEP','Retry STEP') }}</button><details><summary>{{ label('Подробности','Details') }}</summary>{{ sourceStepError }}</details></span>
        </template>
        <button type="button" :disabled="restoringDraft || !selectedBody?.brep" @click="exportStepCurrent">{{ label('STEP выбранного тела · текущая геометрия', 'Export current body STEP') }}</button>
        <button type="button" :disabled="restoringDraft || !document.bodies.length" @click="exportStepAssembly">{{ label('STEP сборки · тела и группы', 'Export bodies and groups STEP') }}</button>
        <button type="button" :disabled="restoringDraft || stepBusy" @click="exportStepOriginal">{{ label('Экспорт AP242-оригинала', 'Export AP242 original') }}</button>
        <button :disabled="!document.bodies.length" @click="download(directBodiesScad(document), 'solid-bodies.scad')">{{ label('Экспорт SCAD (bake)', 'Export SCAD (bake)') }}</button>
        <button :disabled="!document.bodies.length || !canAppend" @click="appendBodies">{{ embedded ? label('Bake в код', 'Bake into code') : label('Bake в Code (append)', 'Bake into Code (append)') }}</button>
        <button :disabled="!document.bodies.length" @click="sendToMesh">{{ label('Открыть в Mesh', 'Open in Mesh') }}</button>
      </div></details>
    </header>
    <div v-show="createOpen || canExtrudeSketch || choosingSketchFace" class="sketch-start-bar" :aria-label="label('От плоской фигуры к объёму', 'From a flat shape to a solid')">
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
    <div v-show="createOpen" class="primitive-bar">
      <strong>{{ label('Примитивы','Primitives') }}</strong>
      <button v-for="kind in primitiveKinds" :key="kind" class="primitive-icon" :disabled="!kernelReady" :title="primitiveLabel(kind)" :aria-label="primitiveLabel(kind)" @click="addPrimitive(kind)"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="PRIMITIVE_ICONS[kind]" /></svg></button>
      <span class="primitive-divider" aria-hidden="true"></span>
      <button class="primitive-icon" :title="label('NURBS-кривая','NURBS curve')" :aria-label="label('NURBS-кривая','NURBS curve')" @click="addNurbs('curve')"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="PRIMITIVE_ICONS.curve" /></svg></button>
      <button class="primitive-icon" :title="label('NURBS-поверхность','NURBS surface')" :aria-label="label('NURBS-поверхность','NURBS surface')" @click="addNurbs('surface')"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="PRIMITIVE_ICONS.surface" /></svg></button>
      <RoundSolidOptions :label="label" :primitive-g2-invalid="primitiveG2Invalid" v-model:top-radius="primitiveTopRadius" v-model:inner-radius="primitiveInnerRadius" v-model:g2-enabled="primitiveG2Enabled" v-model:g2-size="primitiveG2Size" v-model:individual="primitiveG2Individual" v-model:transition-sizes="primitiveG2Sizes" />
      <button v-if="failedPrimitive" :disabled="primitivePending" @click="failedPrimitive && addPrimitive(failedPrimitive)">{{ label('Повторить создание','Retry creation') }}</button>
      <label>{{ label('Сфера / цилиндр / конус','Sphere / cylinder / cone') }} <select v-model="roundGeometry"><option value="exact">{{ label('Точные поверхности','Exact surfaces') }}</option><option value="faceted">{{ label('Гранёный · planar Boolean','Faceted · planar Boolean') }}</option></select></label>
      <label>{{ label('Размер, мм','Size, mm') }} <input v-model.number="primitiveSize" type="number" min="0.1" max="10000" /></label>
      <button type="button" :title="label('Импорт сетки как тело', 'Import mesh as body')" @click="stlInput?.click()">{{ label('Импорт сетки', 'Import mesh') }}</button>
      <button v-if="embedded" :disabled="!canAppend" @click="appendBodies">{{ label('Применить в код (bake)','Apply to code') }}</button>
      <span class="subtle">{{ label('Solid хранит свой документ; bake в .scad — только по кнопке.','Solid keeps its own document; bake to .scad is explicit.') }}</span>
    </div>
    <div class="workspace-state" aria-live="polite">
      <label>{{ label('Создавать в','Create in') }} <select v-model="activeGroup" :aria-label="label('Активная группа','Active group')"><option value="">{{ label('Без группы','Ungrouped') }}</option><option v-for="section in bodySections.filter(s=>s.name)" :key="section.key" :value="section.name!">{{ section.name }}</option></select></label>
      <button :disabled="!selectedIds.length||directTransformPending" @click="moveSelectionToGroup">{{ label('Перенести выбор', 'Move selection to group') }}</button>
      <button :disabled="directTransformPending" @click="addEmptyGroup">{{ label('Новая группа','New empty group') }}</button>
      <button v-if="hiddenIds.length" @click="hiddenIds=[]">{{ label('Показать всё','Show all objects') }} ({{ hiddenIds.length }})</button>
      <span v-if="lockedIds.length">{{ label('Заблокировано','Locked') }}: {{ lockedIds.length }}</span>
      <span v-if="subtract">{{ label('A — основа · B — вырез', 'A — target · B — cutter') }}</span>
      <label v-if="pickMode==='edge' && selectedBody">{{ label('Ребро','Edge') }} <select :value="edgeIndex" :aria-label="label('Выбрать ребро','Select edge')" @change="setPickedEdge(Number(($event.target as HTMLSelectElement).value))" @keydown="edgeSelectionKey"><option :value="-1">—</option><option v-for="edge in featureEdges" :key="edge.i" :value="edge.i">{{ edge.i+1 }}</option></select></label>
      <span v-if="edgeIndexes.length">{{ label('Рёбра','Edges') }}: {{ edgeIndexes.length }}</span>
      <template v-if="pickMode==='edge' && selectedBody">
        <select v-model.number="additionalEdgeIndex" :aria-label="label('Дополнительное ребро','Additional edge')"><option :value="-1">—</option><option v-for="edge in featureEdges" :key="edge.i" :value="edge.i">{{ edge.i+1 }}</option></select>
        <button :disabled="additionalEdgeIndex<0 || !featureEdges.some(edge=>edge.i===additionalEdgeIndex)" @click="addPickedEdge">{{ label('Добавить ребро','Add edge') }}</button>
      </template>
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
    <div v-if="commandActive" class="command-guidance" :class="{ failed: commandFailure, 'runout-guidance': advancedOp==='nurbs-runout' }" :style="advancedOp==='nurbs-runout'?{'--runout-dock-width':(dockOpen?344:46)+'px'}:undefined" role="status">
      <strong>{{ commandFailure ? label('! Ошибка построения', '! Build error') : booleanPending ? (booleanKind==='minkowski3d'?label('Сумма Минковского','Minkowski sum'):'Boolean') : label('Предпросмотр операции', 'Operation preview') }}</strong>
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
                <button class="tool-icon" :title="label('Сумма Минковского: два тела, с проверкой исходных поверхностей','Minkowski sum: two bodies, with source surface verification')" :aria-label="label('Сумма Минковского','Minkowski sum')" :disabled="!twoSelectedBodies || commandActive" @click="applyMinkowski"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true"><path d="M3 8l5-3 5 3-5 3zM3 8v6l5 3 5-3V8M15 12h6M18 9v6M9 18l5 3 7-4v-1"/></svg></button>
                <label v-if="twoSelectedBodies" class="minkowski-tolerance">{{ label('Допуск поверхностей, мм','Surface tolerance, mm') }} <input v-model.number="minkowskiDeviation" type="number" min="0.000000001" max="10000" step="0.1" :aria-label="label('Допуск исходных поверхностей Минковского, мм','Minkowski source surface tolerance, mm')" :disabled="commandActive" style="width:5rem" /></label>
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
              <select v-model="bodyExportFormat" :disabled="historyPending" :aria-label="label('Формат экспорта тела', 'Body export format')"><option v-for="format in MESH_EXPORT_FORMATS" :key="format" :value="format">{{ MESH_FORMAT_LABELS[format] }}</option></select>
              <button class="tool-icon" :disabled="!selectedBody || historyPending" :title="label('Скачать выбранное тело в выбранном формате','Download the selected body in the chosen format')" :aria-label="label('Скачать', 'Download')" @click="downloadBody"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round" aria-hidden="true"><path :d="TOOL_ICONS.download" /></svg></button></template>
          </div>
          <WorkspaceActionBar v-if="pane === '3d' && selectedBody && !commandActive" :label="label('Действия над телом', 'Body actions')">
            <span class="ws-name">{{ selectedBody.name || label('Тело', 'Body') }}</span>
            <span class="ws-sep" aria-hidden="true"></span>
            <button type="button" :aria-pressed="movingBody" :title="label('Двигать тело · G', 'Move body · G')" @click="movingBody = !movingBody">{{ label('Двигать', 'Move') }}</button>
            <button type="button" :title="label('Вычесть: A − B', 'Subtract: A − B')" @click="beginSubtract()">{{ label('Вычесть', 'Subtract') }}</button>
            <button type="button" :aria-pressed="!!isolatedBodyIds.length" @click="toggleBodyIsolation">{{ label('Изолировать', 'Isolate') }}</button>
            <span class="ws-sep" aria-hidden="true"></span>
            <button type="button" class="danger" :title="label('Удалить · Del', 'Delete · Del')" @click="remove">{{ label('Удалить', 'Delete') }}</button>
          </WorkspaceActionBar>
          <div class="canvas-wrap" :class="{'profile-preparation-pane':pane==='2d' && (isProfileCommand(advancedOp)||advancedOp==='offset')}">
            <div class="canvas-viewport">
            <ModelingGridControls :locale="locale" snapping />
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
                <polyline :pointer-events="objectSelectable(curve.id)?undefined:'none'" v-for="curve in nurbsCurvePaths" v-show="!(advancedOp==='nurbs-curve-match' && advancedPreview.document && curve.id===surfaceInputs[1])" :key="'curve-'+curve.id" :data-diagnostic="curve.id===runoutProblemCurve?'runout-target':curve.id===patchBudgetCurve?'patch-budget':undefined" :points="curve.points" fill="none" :stroke="curve.id===patchBudgetCurve||curve.id===runoutProblemCurve?'#ff647c':selectedIds.includes(curve.id)?'#ffc977':'#77eac5'" stroke-width="3" vector-effect="non-scaling-stroke" @pointerdown.stop="pickNurbsCurve($event,curve.id)" />
                <g v-if="selectedNurbs" class="nurbs-cage">
                  <polyline v-if="selectedNurbsCurve" :points="selectedNurbsCurve.curve.controlPoints.map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#ffc977" stroke-dasharray="3 3" vector-effect="non-scaling-stroke" pointer-events="none" />
                  <template v-if="selectedNurbsSurface">
                    <polyline v-for="(_,u) in selectedNurbsSurface.surface.controlPoints" :key="'u-'+u" :points="selectedNurbsSurface.surface.controlPoints[u].map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#ffc977" stroke-dasharray="3 3" vector-effect="non-scaling-stroke" pointer-events="none" />
                    <polyline v-for="(_,v) in selectedNurbsSurface.surface.controlPoints[0]" :key="'v-'+v" :points="selectedNurbsSurface.surface.controlPoints.map(row=>project(row[v],'3d').join(',')).join(' ')" fill="none" stroke="#ffc977" stroke-dasharray="3 3" vector-effect="non-scaling-stroke" pointer-events="none" />
                  </template>
                  <circle v-for="cv in nativeCage" :key="cv.u+'-'+cv.v" :cx="project(cv.point,'3d')[0]" :cy="project(cv.point,'3d')[1]" :r="views['3d']/110" :fill="cvU===cv.u&&cvV===cv.v?'#ff8b77':'#ffc977'" stroke="#2a2114" vector-effect="non-scaling-stroke" @pointerdown.stop="startCv($event,cv.u,cv.v)" />
                </g>
              </g>
              <polyline v-if="pane===mode && (tool==='circle'||tool==='arc') && !draft.length && exactRoundPoints.length" :data-preview="tool==='circle'?'numeric-circle':'numeric-arc'" :points="exactRoundPoints.map(p=>project(pane==='3d'?worldPoint(p,activePlane):p,pane).join(',')).join(' ')" fill="none" stroke="#77eac5" stroke-width="2" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" pointer-events="none" />
              <polyline v-if="pane===mode && !draft.length && (tool==='rectangle'&&exactRectangle || tool==='slot'&&numericSlotResult)" :data-preview="tool==='rectangle'?'numeric-rectangle':'numeric-slot'" :points="[...(tool==='rectangle'?exactRectangle!:numericSlotResult!.points),(tool==='rectangle'?exactRectangle!:numericSlotResult!.points)[0]].map(p=>project(pane==='3d'?worldPoint(p,activePlane):p,pane).join(',')).join(' ')" fill="none" stroke="#77eac5" stroke-width="2" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" pointer-events="none" />
              <g v-if="pane==='3d'" data-source-face-preview="true" :pointer-events="sourceWallOpen && sourceWallMenuOpen?'auto':'none'">
                <rect v-if="sourceExpressionMeshRegionBox" :x="sourceExpressionMeshRegionBox.x" :y="sourceExpressionMeshRegionBox.y" :width="sourceExpressionMeshRegionBox.width" :height="sourceExpressionMeshRegionBox.height" data-expression-mesh-pending="true" fill="none" stroke="#e88733" stroke-width="3" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" pointer-events="none"><title>{{ sourceExpressionMeshRegionBox.face }} · {{ sourceExpressionMeshRegionBox.reason }}</title></rect>
                <polygon v-for="triangle in sourceBoundaryTriangles" :key="'boundary-mesh:'+triangle.key" :points="triangle.points" :data-boundary-mesh-face="triangle.face" :data-boundary-mesh-state="triangle.state" :fill="triangle.state==='qualified'?'#4eaa87':triangle.state==='error'?'#d96565':triangle.state==='pending'?'#d99842':'#657d99'" fill-opacity="0.18" :stroke="triangle.state==='qualified'?'#4eaa87':triangle.state==='error'?'#d96565':triangle.state==='pending'?'#d99842':'#657d99'" stroke-width="0.5" vector-effect="non-scaling-stroke" pointer-events="none"><title>{{ triangle.state==='qualified'?label('Допуск поверхности подтверждён · грань','Surface tolerance qualified · face'):label('Допуск не подтверждён · грань','Tolerance unproven · face') }} {{ triangle.face }}</title></polygon>
                <polygon v-for="triangle in sourceExpressionTriangles" :key="'expression-mesh:'+triangle.key" :points="triangle.points" :data-expression-mesh-face="triangle.face" fill="#079b99" fill-opacity="0.3" stroke="#079b99" stroke-width="0.5" vector-effect="non-scaling-stroke" pointer-events="none"><title>{{ label('Preview · грань','Preview · face') }} {{ triangle.face }} · ≤ {{ triangle.error }} mm</title></polygon>
                <rect v-for="box in sourceExpressionBoxes" :key="'expression:'+box.key" :x="box.x" :y="box.y" :width="box.width" :height="box.height" :data-expression-shell-face="box.face" :data-expression-trim="box.location" :data-radius-violation="sourceExpressionRadiusFaces.includes(box.face)?true:undefined" fill="none" :stroke="sourceExpressionRadiusFaces.includes(box.face)?'#d96565':box.location==='inside'?'#079b99':'#e88733'" stroke-width="1" :stroke-dasharray="box.location==='unresolved'?'3 3':undefined" vector-effect="non-scaling-stroke" pointer-events="none"><title>{{ label('Предложенная оболочка · грань','Proposed shell · face') }} {{ box.face }} · {{ box.location }} · UV {{ box.domain }}</title></rect>
                <polygon v-for="face in sourceDisplayFaces" :key="face.key" :points="face.points" :data-source-body="face.body" :data-source-face="face.face" :fill="sourceWallFaceFill(face.body,face.face)" fill-opacity=".25" stroke="none" @pointerdown.stop @click.stop="face.body===selectedSourceBody?.id && toggleSourceWallFace(face.face)" />
                <rect v-for="box in sourceUnresolvedBoxes" :key="box.key" :x="box.x" :y="box.y" :width="box.width" :height="box.height" :data-source-body="box.body" :data-source-face="box.face" :data-source-wall-error="sourceWallFaceUncertain(box.body,box.face)?'true':undefined" fill="none" :data-source-expression-face="sourceExpressionFaceHighlighted(box.body,box.face)?box.face:undefined" :stroke="sourceExpressionFaceHighlighted(box.body,box.face)?'#ff825e':sourceWallFaceUncertain(box.body,box.face)?'#ff4c4c':'#ff9977'" :stroke-width="sourceExpressionFaceHighlighted(box.body,box.face)||sourceWallFaceUncertain(box.body,box.face)?3:1" stroke-dasharray="3 3" vector-effect="non-scaling-stroke"><title>{{ box.body }} · {{ label('грань','face') }} {{ box.face }} · UV {{ box.uv }}</title></rect>
              </g>
              <g v-if="pane==='3d' && sourceWallMeasurement" data-source-wall-measurement="true" pointer-events="none">
                <polyline :points="sourceWallMeasurement.points" fill="none" stroke="#ffe875" stroke-width="3" stroke-dasharray="5 3" vector-effect="non-scaling-stroke"><title>{{ label('Измерение толщины: линия между центрами координатных интервалов. ','Thickness measurement: line between enclosure centers. ')+sourceWallMeasurement.interval+' mm' }}</title></polyline>
                <g v-for="(end,i) in sourceWallMeasurement.endpoints" :key="i" :data-source-wall-endpoint="i" :data-source-face="end.face">
                  <rect :x="end.x" :y="end.y" :width="end.width" :height="end.height" fill="none" stroke="#ffe875" stroke-width="2" vector-effect="non-scaling-stroke" />
                  <circle :cx="end.center[0]" :cy="end.center[1]" r="3" fill="#ffe875"><title>{{ label('Координатный интервал конца: ','Endpoint coordinate enclosure: ')+end.bounds }}</title></circle>
                </g>
              </g>
              <g v-if="pane==='3d'" data-source-endpoint-enclosures="true" pointer-events="none">
                <rect v-for="box in sourceDisplayEndpointBoxes" :key="box.key" :x="box.x" :y="box.y" :width="box.width" :height="box.height" fill="none" stroke="#ffc977" stroke-width="2" stroke-dasharray="2 2" vector-effect="non-scaling-stroke"><title>{{ label('Интервал конца: ','Endpoint enclosure: ')+box.bounds }}</title></rect>
              </g>
              <g v-if="pane==='3d'" data-source-body-preview="edges">
                <polyline v-for="edge in sourceDisplayEdges" :key="edge.key" :points="edge.points" fill="none" :stroke="sourceEdgeSelection===edge.key?(sourceSeamResult&&!sourceSeamResult.qualified?'#ff9977':'#ffc977'):sourceEdgeHover===edge.key?'#ffffff':'#77eac5'" :stroke-width="sourceEdgeSelection===edge.key?4:2" vector-effect="non-scaling-stroke" :tabindex="edge.selectable?0:-1" :aria-disabled="!edge.selectable" role="button" :aria-label="edge.name+' '+label('ребро','edge')+' '+edge.index" :aria-pressed="sourceEdgeSelection===edge.key" @pointerenter="sourceEdgeHover=edge.key" @pointerleave="sourceEdgeHover=''" @pointerdown.stop.prevent="pickSourceEdge(edge.body,edge.index)" @keydown.enter.stop.prevent="pickSourceEdge(edge.body,edge.index)" @keydown.space.stop.prevent="pickSourceEdge(edge.body,edge.index)" @keydown.esc.stop.prevent="sourceEdgeSelection=''" />
              </g>
              <g v-if="pane==='3d'" data-source-seam-errors="endpoints" pointer-events="none">
                <circle v-for="marker in sourceSeamEndpointMarkers" :key="marker.key" :data-source-body="marker.body" :data-source-seam-error-point="marker.point.join(',')" :cx="marker.screen[0]" :cy="marker.screen[1]" :r="views['3d']/100" fill="#ff647c" stroke="white" stroke-width="2" vector-effect="non-scaling-stroke"><title>{{ label('Разные касательные плоскости в конце: ','Distinct endpoint tangent planes: ')+marker.point.join(', ')+' mm' }}</title></circle>
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
                <polyline v-for="edge in featureEdges" :key="edge.id" :data-topology-edge="edge.id" role="button" tabindex="0" :aria-label="label('Ребро ','Edge ')+(edge.i+1)" :aria-pressed="edgeIndexes.includes(edge.i)" @keydown.enter.stop.prevent="pickEdge($event,edge.i)" @keydown.space.stop.prevent="pickEdge($event,edge.i)" :points="edge.points" fill="none" :stroke="edgeIndexes.includes(edge.i)?(advancedPreview.error && ['edge-fillet','chamfer'].includes(advancedOp??'')?'#f87171':'#ffc977'):'#89baff'" :stroke-width="edgeIndexes.includes(edge.i)?5:3" vector-effect="non-scaling-stroke" @pointerdown="pickEdge($event,edge.i)" />
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
              <g v-if="pane==='3d' && !shellDistanceOpen && !volumeDistanceOpen && !materialPathOpen && measurement?.value" pointer-events="none" data-measurement="distance">
                <polyline :points="[measurement.value.a,measurement.value.b].map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#ffda75" stroke-width="2" vector-effect="non-scaling-stroke"/>
                <g v-for="(point,i) in [measurement.value.a,measurement.value.b]" :key="i" :transform="`translate(${project(point,'3d').join(' ')})`"><circle :r="views['3d']/120" fill="#ffda75"/><text :font-size="views['3d']/40" fill="#ffda75" :x="views['3d']/90">{{ i===0?'A':'B' }}</text></g>
              </g>
              <SolidVolumeWitness v-if="pane==='3d' && volumeContact" :points="volumeContact" :project="project" :size="views['3d']" :ru="ru"/>
              <g v-if="pane==='3d' && materialPathOverlay" pointer-events="none" data-diagnostic="material-path">
                <line :x1="project(materialPathOverlay.line[0],'3d')[0]" :y1="project(materialPathOverlay.line[0],'3d')[1]" :x2="project(materialPathOverlay.line[1],'3d')[0]" :y2="project(materialPathOverlay.line[1],'3d')[1]" :stroke="materialPathOverlay.proven?'#77eac5':'#ff6978'" stroke-dasharray="5 3" stroke-width="2" vector-effect="non-scaling-stroke"/>
                <circle v-for="(m,i) in materialPathOverlay.marks" :key="i" :data-material-face="m.face+1" :cx="project(m.point,'3d')[0]" :cy="project(m.point,'3d')[1]" :r="views['3d']/85" fill="none" :stroke="m.unresolved?'#ffc977':materialPathOverlay.proven?'#77eac5':'#ff6978'" stroke-width="3" vector-effect="non-scaling-stroke"><title>{{label('Грань','Face')}} {{m.face+1}} · {{m.unresolved?label('Непроверенный участок','Unchecked region'):label('Пересечение линии','Line crossing')}}</title></circle>
              </g>
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
              <ProfileIntersectionPresentation v-if="advancedOp==='profile-prepare' && profilePreparation?.value?.report.intersections" mode="preview" :report="profilePreparation.value.report" :project="project" :pane="pane" :plane="profilePreparation.value.plane" :view="views[pane]"/>
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

            </svg>
            <CadDimensionOverlay v-if="pane==='3d' && inlineDimensionVisible && commandAnchor" :anchor="commandAnchor" :center="centers['3d']" :size="views['3d']" v-model="inlineDimension" :locale="locale" :failed="!!commandFailure" :label="label('Размер у манипулятора, мм','Dimension at manipulator, mm')" @validity="quantityValidity('inline',$event)" />
            <div class="zoom-tools"><button :aria-label="label('Приблизить ', 'Zoom in ') + pane" @click="zoom(pane, .8)">+</button><button :aria-label="label('Отдалить ', 'Zoom out ') + pane" @click="zoom(pane, 1.25)">−</button></div>
            <div v-if="pane === '3d'" class="fps-badge" :class="{ low: fps > 0 && fps < 30 }" role="status" :aria-label="label('Кадров в секунду', 'Frames per second')">{{ fps }} FPS · {{ frameMs }} ms<template v-if="gpuActive"> · draw {{ drawMs }} ms</template></div>
            </div>
            <section v-if="(tool==='rectangle'||tool==='slot')&&pane===mode" class="operation-card" :aria-label="tool==='rectangle'?label('Точный прямоугольник','Exact rectangle'):label('Точный паз','Exact slot')">
              <header class="numeric-authoring-header"><strong>{{ tool==='rectangle'?label('Прямоугольник — точка и размеры','Rectangle — origin and size'):label('Паз — центры и ширина','Slot — centers and width') }}</strong>
                <template v-if="tool==='rectangle'"><small v-if="!exactRectangle" role="alert">{{ label('Исправьте точку и размеры. Размеры от 0,01 мм; все углы должны быть в пределах ±1 000 000 мм.','Correct origin and size. Dimensions start at 0.01 mm; all corners must stay within ±1,000,000 mm.') }}</small></template>
                <template v-else><small v-if="!numericSlotInput" role="alert">{{ label('Исправьте центры и ширину. Центры должны различаться минимум на 0,000001 мм, ширина — от 0,02 мм; контур — в пределах ±1 000 000 мм.','Correct centers and width. Centers must be at least 0.000001 mm apart, width starts at 0.02 mm; contour must stay within ±1,000,000 mm.') }}</small><small v-else-if="numericSlotError" role="alert">{{ label('Не удалось подготовить паз. Повторите расчёт или измените центры и ширину.','Could not prepare the slot. Retry or change centers and width.') }}</small><button v-if="numericSlotError" @click="numericSlotRetry++">{{ label('Повторить расчёт паза','Retry slot calculation') }}</button><small v-if="numericSlotPending" role="status" aria-label="numeric-slot-preparation">{{ label('Готовлю точный паз…','Preparing exact slot…') }}</small></template>
              </header>
              <small>{{ tool==='rectangle'?label('Начальная точка и положительные размеры в активной плоскости, мм. Создать — применить, Esc — отменить.','Origin and positive dimensions in the active plane, mm. Create applies; Esc cancels.'):label('Два центра торцов и полная ширина, мм. Создать — применить, Esc — отменить.','Two end-cap centers and full width, mm. Create applies; Esc cancels.') }}</small>
              <label v-for="(axis,i) in ['X','Y']" :key="axis">{{ tool==='slot'?'A · ':'' }}{{ axis }}<CadQuantityInput v-model="draftPoint[i]" :locale="locale" :min="-1000000" :max="1000000" :aria-label="label('Начальная координата ','Origin coordinate ')+axis" @validity="draftPointValid[i]=$event" /></label>
              <template v-if="tool==='rectangle'">
                <label v-for="(axis,i) in [label('Ширина','Width'),label('Высота','Height')]" :key="axis">{{ axis }}<CadQuantityInput v-model="draftSize[i]" :locale="locale" :min=".01" :max="1000000" :aria-label="label('Размер прямоугольника ','Rectangle size ')+axis" @validity="draftSizeValid[i]=$event" /></label>

                <button :disabled="!exactRectangle" @click="createExactRectangle">{{ label('Создать прямоугольник','Create rectangle') }}</button>
              </template>
              <template v-else>
                <label v-for="(axis,i) in ['X','Y']" :key="axis">B · {{ axis }}<CadQuantityInput v-model="draftEnd[i]" :locale="locale" :min="-1000000" :max="1000000" :aria-label="label('Конечная координата ','End coordinate ')+axis" @validity="draftEndValid[i]=$event" /></label>
                <label>{{ label('Ширина паза, мм','Slot width, mm') }}<CadQuantityInput v-model="slotWidth" :locale="locale" :min=".02" :max="1000000" :aria-label="label('Ширина паза, мм','Slot width, mm')" @validity="slotWidthValid=$event" /></label>




                <button :disabled="!numericSlotInput||numericSlotPending||!numericSlotResult" @click="createExactSlot">{{ label('Создать паз','Create slot') }}</button>
              </template>
            </section>
            <section v-if="(tool==='polyline'||tool==='circle'||tool==='arc') && pane===mode" class="operation-card" :aria-label="tool==='arc'?label('Точная дуга','Exact arc'):tool==='circle'?label('Точный круг','Exact circle'):label('Точки ломаной','Polyline points')">
              <strong>{{ tool==='arc'?label('Дуга — центр, радиус и углы','Arc — center, radius and angles'):tool==='circle'?label('Круг — центр и радиус','Circle — center and radius'):label('Ломаная — точные координаты','Polyline — exact coordinates') }}</strong>
              <small>{{ tool==='arc'?label('Углы в градусах. Положительный разворот — против часовой стрелки. Создать дугу — применить, Esc — отменить.','Angles in degrees. Positive sweep runs counterclockwise. Create arc applies; Esc cancels.'):tool==='circle'?label('Центр и радиус в активной плоскости, мм. Создать круг — применить, Esc — отменить.','Center and radius in the active plane, mm. Create circle applies; Esc cancels.'):label('Координаты в активной плоскости, мм. Добавьте точки, затем замкните контур или завершите линию. Esc — отмена.','Coordinates in the active plane, mm. Add points, then close the contour or finish the line. Esc cancels.') }}</small>
              <label v-for="(axis,i) in ['X','Y']" :key="axis">{{ axis }}<CadQuantityInput v-model="draftPoint[i]" :locale="locale" :min="-1000000" :max="1000000" :aria-label="label('Координата точки ','Point coordinate ')+axis" @validity="draftPointValid[i]=$event" /></label>
              <template v-if="tool==='circle'||tool==='arc'">
                <label>{{ label('Радиус, мм','Radius, mm') }}<CadQuantityInput v-model="draftRadius" :locale="locale" :min=".01" :max="1000000" :aria-label="tool==='arc'?label('Радиус дуги','Arc radius'):label('Радиус круга','Circle radius')" @validity="draftRadiusValid=$event" /></label>
                <template v-if="tool==='arc'">
                  <label>{{ label('Начальный угол','Start angle') }}<CadQuantityInput v-model="draftArcStart" kind="angle" :locale="locale" :min="-360000" :max="360000" :aria-label="label('Начальный угол дуги','Arc start angle')" @validity="draftArcValid[0]=$event" /></label>
                  <label>{{ label('Разворот','Sweep') }}<CadQuantityInput v-model="draftArcSweep" kind="angle" :locale="locale" :min="-359.999999" :max="359.999999" :aria-label="label('Разворот дуги','Arc sweep')" @validity="draftArcValid[1]=$event" /></label>
                </template>
                <button :disabled="!exactRoundCurve" @click="createExactRoundCurve">{{ tool==='arc'?label('Создать дугу','Create arc'):label('Создать круг','Create circle') }}</button>
                <small v-if="!exactRoundCurve" role="alert">{{ tool==='arc'?label('Исправьте центр, радиус и углы. Радиус от 0,01 мм; модуль разворота от 0,1° до 360° без полного оборота.','Correct center, radius and angles. Radius starts at 0.01 mm; absolute sweep starts at 0.1° and must be below 360°.'):label('Исправьте центр и радиус (0,01–1 000 000 мм).','Correct the center and radius (0.01–1,000,000 mm).') }}</small>
              </template>
              <template v-else>
              <button :disabled="!canAddDraftPoint" @click="addExactDraftPoint">{{ label('Добавить точку','Add point') }}</button>
              <small v-if="!draftPointValid.every(Boolean)" role="alert">{{ label('Исправьте координаты точки.','Correct the point coordinates.') }}</small>
              <small v-else-if="!canAddDraftPoint" role="status">{{ label('Такая точка уже есть в контуре. Задайте другую или замкните контур.','This point is already in the contour. Choose another point or close the contour.') }}</small>
              <small>{{ draft.length }} {{ label('точек','points') }}</small>
              </template>
            </section>
            <div v-if="advancedOp && pane === (['offset','extend','curve','profile-prepare','profile-union','profile-difference','profile-intersection'].includes(advancedOp) ? '2d' : '3d')" class="operation-card">
              <strong>{{ ({'nurbs-runout':label('G2 переход','G2 runout'),'nurbs-offset':label('Смещение NURBS-кривой','Offset NURBS curve'),'nurbs-point-trim':label('Обрезать NURBS по точке','Trim NURBS at point'),'profile-difference':label('Вычесть области профилей','Subtract profile regions'),'profile-intersection':label('Пересечь точные профили','Intersect exact profiles'),'profile-union':label('Объединить точные профили','Union exact profiles'),'profile-prepare':label('Собрать профиль','Prepare profile'),'nurbs-curve-match':label('Согласовать кривые G1','Match curves G1'),'nurbs-prepare':label('Подготовить границы','Prepare boundaries'),'nurbs-match':label('Согласовать поверхности G1/G2','Match surfaces G1/G2'),'instance-transform':label('Преобразовать экземпляр','Transform instance'),'instance-create':label('Создать связанный экземпляр','Create linked instance'),'instance-place':label('Разместить экземпляр','Place instance'),'nurbs-surface-rebuild':label('Перестроить поверхность','Rebuild surface'),'nurbs-rebuild':label('Перестроить кривую','Rebuild curve'),'nurbs-patch':label('Coons patch','Coons patch'),'nurbs-surface-reduce':label('Снизить степень поверхности','Reduce surface degree'),'nurbs-reduce':label('Снизить степень кривой','Reduce curve degree'),'nurbs-loft':label('Поверхность по сечениям','NURBS loft'),'nurbs-sweep':label('Перенос профиля по пути','Sweep'),loft:label('Линейчатый B-rep loft','Ruled B-rep loft'),push:label('Сдвиг грани','Push / Pull'),chamfer:label('Фаска ребра','Edge chamfer'),'edge-fillet':label('Скругление ребра','Edge fillet'),shell:label('Полое тело','Shell'),split:label('Разрез плоскостью','Plane split'),offset:label('Отступ контура','Offset'),extend:label('Продлить линию','Extend'),curve:label('Окружность / дуга','Circle / arc'),transform:label('Преобразовать выбор','Transform selection')})[advancedOp] }}</strong>
              <small v-if="advancedPreview.error" role="alert">{{ advancedPreview.error }}</small>
              <small v-if="selectedBody?.brep && (advancedOp==='chamfer' || advancedOp==='edge-fillet' && advanced.filletMode==='constant')">{{ label('Точный B-rep: скругление — полные выпуклые продольные цепочки призмы/корпуса или круговые рёбра фланца; фаска — связанные прямые выпуклые рёбра. Shift добавляет рёбра. При отказе уменьшите радиус или измените набор рёбер.', 'Exact B-rep: fillet complete convex longitudinal chains of a prism/enclosure or circular annular rims; chamfer connected straight convex edges. Shift adds edges. If refused, reduce the radius or change the edge selection.') }}</small>
              <small v-else-if="advancedOp === 'push'">{{ label('Сдвиг плоской грани выпуклого тела или торца цилиндра.', 'Move a planar face of a convex solid or a cylinder end cap.') }}</small>
              <small v-else-if="!selectedBody?.brep && ['chamfer','edge-fillet','shell'].includes(advancedOp)">{{ label('Mesh-операция для выпуклых тел с плоскими гранями.', 'Mesh operation for convex solids with planar faces.') }}</small>
              <small v-if="advancedOp==='loft'">{{ label('Выберите с Shift параллельные сечения по порядку. Для сохранённых кривых нужны два соответствующих профиля: перенос и положительный равномерный масштаб.','Shift-select parallel sections in order. Retained curves require two corresponding profiles related by translation and positive uniform scale.') }}</small>
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
                <small>{{ label('Первый вход сохраняет ID и свойства. Кривые сохраняются, размеры удаляются. Undo восстанавливает входы.','The first input keeps its identity and properties. Curves are retained; dimensions are removed. Undo restores the inputs.') }}</small>
                <label class="preparation-tolerance">{{ label('Допуск разрыва, мм','Gap tolerance, mm') }}<CadQuantityInput v-model="advanced.profileGap" :locale="locale" :min="0" :max="1000000" style="width:110px" :aria-label="label('Допуск разрыва, мм','Gap tolerance, mm')" @validity="quantityValidity('profileGap',$event)" /></label>
                <output v-if="profilePreparation?.value?.report.accepted" data-testid="profile-preparation-report">{{ profilePreparation.value.report.accepted?label('Контур замкнут. Добавлено отрезков: ','Closed contour. Added connectors: ')+profilePreparation.value.report.connectors.length:profilePreparationError(profilePreparation.value.report.reason,profilePreparation.value.report.segmentDefect?.kind) }}</output>
                <ProfileIntersectionPresentation v-if="profilePreparation?.value" mode="status" :report="profilePreparation.value.report" :locale="locale"/>
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
              <section v-if="advancedOp==='nurbs-runout'" class="runout-controls">
                <small>{{ label('Создаётся отдельная переходная поверхность. Исходные поверхности сохраняются.','Creates an independent transition surface and preserves the source surfaces.') }}</small>
                <label v-for="(role,i) in [label('Исходная поверхность','Source surface'),label('Плоский торец','Planar end'),label('Граница на торце','End boundary')]" :key="role">{{ role }}<select v-model="surfaceInputs[i]" :aria-label="role" :aria-invalid="(i===2 && !!runoutProblemCurve)||(i===1 && advancedPreview.rawError.includes('affine plane'))"><template v-if="i<2"><option v-for="s in document.surfaces??[]" :key="s.id" :value="s.id">{{ s.name }}</option></template><template v-else><option v-for="c in (document.curves??[]).filter(c=>c.curve.controlPoints[0]?.length===3)" :key="c.id" :value="c.id">{{ c.name }}</option></template></select></label>
                <label>{{ label('Край исходной поверхности','Source edge') }}<select v-model="advanced.matchBoundaryA" :aria-label="label('Край исходной поверхности','Source edge')"><option v-for="b in ['uMin','uMax','vMin','vMax']" :key="b">{{ b }}</option></select></label>
                <label>{{ label('Положение шва, %','Seam position, %') }}<input v-model.number="advanced.runoutPosition" type="number" min="0" max="100" step="1" :aria-label="label('Положение шва, %','Seam position, %')"></label>
                <label>{{ label('Длина касательных, мм','Tangent length, mm') }}<CadQuantityInput v-model="advanced.runoutTangentLength" kind="length" :locale="locale" :min="0.000001" :aria-label="label('Длина касательных, мм','Tangent length, mm')" @validity="quantityValidity('runoutTangentLength',$event)" /></label>
                <label><input v-model="advanced.runoutFlip" type="checkbox">{{ label('Обратить сторону перехода','Reverse transition side') }}</label>
                <label><input v-model="advanced.matchReverse" type="checkbox">{{ label('Обратить направление границы','Reverse boundary direction') }}</label>
                <output v-if="surfaceBuildResult?.runoutReport" data-testid="runout-report">{{ surfaceBuildResult.runoutReport.withinTolerance?label('G2 подтверждена по всей длине обоих швов','G2 confirmed over both complete seams'):label('Гладкость не подтверждена; измените параметры','Smoothness unproven; adjust parameters') }}</output>
              </section>
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
              <label v-if="advancedOp==='edge-fillet' && selectedBody?.brep">{{ label('Тип скругления','Fillet type') }}<select v-model="advanced.filletMode" :aria-label="label('Тип скругления','Fillet type')" @change="quantityValidity('endRadius',true)"><option value="constant">{{ label('Постоянный радиус','Constant radius') }}</option><option value="variable">{{ label('Радиус A → B','Radius A → B') }}</option><option value="corner">{{ label('Угол трёх рёбер','Three-edge corner') }}</option><option value="partial-preview">{{ label("Частичная дуга · просмотр", "Partial arc preview") }}</option></select></label>
              <small v-if="advancedOp==='edge-fillet' && selectedBody?.brep && advanced.filletMode==='partial-preview'" role="status">{{ label('Только предпросмотр: проверка геометрии не завершена. Выберите одну внешнюю дугу кольца. Esc — закрыть.', 'Preview only: geometry checks remain incomplete. Select one outer quarter-circle of an annular body. Esc to close.') }}</small>
              <small v-if="advancedOp==='edge-fillet' && selectedBody?.brep && advanced.filletMode==='variable'">{{ label('Одно прямое ребро параллелепипеда, в том числе повёрнутого. Радиус сечения меняется линейно от A к B; оба радиуса положительны и различны.','One straight edge of a cuboid, including rotated parts. Cross-section radius varies linearly from A to B; both radii must be positive and different.') }}</small>
              <small v-if="advancedOp==='edge-fillet' && selectedBody?.brep && advanced.filletMode==='corner'">{{ label('Три ребра у вершины с максимальными X, Y, Z осевого параллелепипеда. Общий радиус; сферическое сопряжение.','Three edges at the maximum X, Y, Z corner of an axis-aligned cuboid. Equal radius with a spherical corner blend.') }}</small>
              <label v-if="advancedOp==='edge-fillet' && selectedBody?.brep && advanced.filletMode==='variable'">{{ label('Радиус B, мм','Radius B, mm') }}<CadQuantityInput :aria-label="label('Радиус B, мм','Radius B, mm')" v-model="advanced.endRadius" kind="length" :locale="locale" :min=".01" @validity="quantityValidity('endRadius',$event)" /></label>
              <label v-if="['edge-fillet','chamfer','curve'].includes(advancedOp)">{{ advancedOp==='edge-fillet' && selectedBody?.brep && advanced.filletMode==='variable'?label('Радиус A, мм','Radius A, mm'):label('Радиус / размер, мм','Radius / size, mm') }}<CadQuantityInput :aria-label="advancedOp==='edge-fillet' && selectedBody?.brep && advanced.filletMode==='variable'?label('Радиус A, мм','Radius A, mm'):label('Радиус / размер, мм','Radius / size, mm')" v-model="advanced.radius" kind="length" :locale="locale" @validity="quantityValidity('advanced.radius', $event)" :min=".01" :max="advancedOp==='curve'?1000000:undefined" step=".5" /></label>
              <label v-if="advancedOp==='edge-fillet' && !selectedBody?.brep">{{ label('Грани скругления','Fillet segments') }}<input v-model.number="filletSegments" type="number" min="2" max="32" step="1"></label>
              <label v-if="advancedOp==='split'||advancedOp==='transform'||advancedOp==='instance-transform'">{{ label('Ось','Axis') }}<select v-model="advanced.axis"><option>x</option><option>y</option><option>z</option></select></label>
              <small v-if="advancedOp==='split'">{{ label('Обе части сохраняются отдельными телами. Пунктир — плоскость разреза.', 'Both halves remain separate bodies. The dashed outline is the cutting plane.') }}</small>
              <small v-if="advancedOp==='shell'">{{ label('Открытые грани:','Open faces:') }} {{ openingFaces.map(i=>i+1).join(', ') }}</small>
              <template v-if="advancedOp==='curve'">
                <label>{{ label('Центр X','Center X') }}<CadQuantityInput :aria-label="label('Центр X','Center X')" v-model="advanced.cx" kind="length" :locale="locale" @validity="quantityValidity('advanced.cx', $event)" /></label><label>{{ label('Центр Y','Center Y') }}<CadQuantityInput :aria-label="label('Центр Y','Center Y')" v-model="advanced.cy" kind="length" :locale="locale" @validity="quantityValidity('advanced.cy', $event)" /></label>
                <label>{{ label('Начальный угол','Start angle') }}<CadQuantityInput :aria-label="label('Начальный угол','Start angle')" v-model="advanced.start" kind="angle" :locale="locale" @validity="quantityValidity('advanced.start', $event)" /></label>
                <label v-if="selectedSketch?.analytic?.kind==='arc'">{{ label('Угол дуги','Arc sweep') }}<CadQuantityInput :aria-label="label('Угол дуги','Arc sweep')" v-model="advanced.sweep" kind="angle" :locale="locale" @validity="quantityValidity('advanced.sweep', $event)" :min="-360" :max="360" :aria-describedby="invalidQuantities['curve.sweep-range']?'curve-sweep-error':undefined" /></label>
              </template>
              <small v-if="advancedOp==='curve' && invalidQuantities['curve.sweep-range']" id="curve-sweep-error" role="alert">{{ label('Модуль угла дуги должен быть от 0,1° до 360°. Измените угол; знак задаёт направление.','Arc sweep magnitude must be between 0.1° and 360°. Change the sweep; its sign sets the direction.') }}</small>
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
              <SurfacePairChecks :ref="el => profileBridgePanel = el as InstanceType<typeof SurfacePairChecks> | null" v-if="profileBridgeSurfaces" :surfaces="profileBridgeSurfaces" :active="props.open&&!commandActive" :ru="ru" />
              <SurfaceDistanceCheck v-if="selectedSurfacePair" v-model:opened="surfaceDistanceOpen" v-model:budget="surfaceDistanceBudget" :value="surfaceDistance?.value??null" :error="surfaceDistance?.error??''" :pending="surfaceDistancePending" :retry-visible="surfaceDistanceRetryVisible" :ru="ru" @retry="surfaceDistanceRevision++" />
              <section v-if="selectedSurfacePair" aria-label="Surface boundary inspection" class="body-diagnostics">
                <button @click="boundaryInspection=!boundaryInspection" :aria-pressed="boundaryInspection">{{ label('Проверить стык поверхностей','Inspect surface boundary') }}</button>
                <template v-if="boundaryInspection">
                  <small v-if="boundaryPending" role="status" aria-label="surface-boundary-inspection">{{ label('Проверяю стык…','Inspecting boundary…') }} <button @click="boundaryInspection=false">Esc</button></small>
                  <small>{{ selectedSurfacePair.map((id,i)=>(i?'B: ':'A: ')+document.surfaces?.find(s=>s.id===id)?.name).join(' · ') }}</small>
                  <label>{{ label('Граница A','Boundary A') }}<select v-model="boundaryOptions.boundaryA" :aria-label="label('Граница A','Boundary A')"><option v-for="b in ['uMin','uMax','vMin','vMax']" :key="b">{{ b }}</option></select></label>
                  <label>{{ label('Граница B','Boundary B') }}<select v-model="boundaryOptions.boundaryB" :aria-label="label('Граница B','Boundary B')"><option v-for="b in ['uMin','uMax','vMin','vMax']" :key="b">{{ b }}</option></select></label>
                  <label><input type="checkbox" v-model="boundaryOptions.reverse">{{ label('Обратное направление B','Reverse B direction') }}</label>
                  <label>{{ label('Допуск зазора, мм','Gap tolerance, mm') }}<input v-model.number="boundaryOptions.toleranceMm" :aria-label="label('Допуск зазора, мм','Gap tolerance, mm')" :aria-invalid="!boundaryInputValid.gap" :aria-describedby="!boundaryInputValid.gap?'surface-boundary-error':undefined" type="number" min="0.000001" step=".01"></label>
                  <label>{{ label('Допуск угла, °','Angle tolerance, °') }}<input v-model.number="boundaryOptions.angleToleranceDeg" :aria-label="label('Допуск угла, °','Angle tolerance, °')" :aria-invalid="!boundaryInputValid.angle" :aria-describedby="!boundaryInputValid.angle?'surface-boundary-error':undefined" type="number" min="0" max="90" step=".1"></label>
                  <label>{{ label('Точек проверки','Sample count') }}<input v-model.number="boundaryOptions.samples" :aria-label="label('Точек проверки','Sample count')" :aria-invalid="!boundaryInputValid.samples" :aria-describedby="!boundaryInputValid.samples?'surface-boundary-error':undefined" type="number" min="2" max="257" step="1"></label>
                  <template v-if="boundaryReport?.value">
                    <output>{{ label('Максимальный зазор: ','Maximum gap: ')+boundaryReport.value.maxGapMm.toFixed(6) }} mm</output>
                    <output>{{ label('Угол касательных плоскостей: ','Tangent-plane angle: ')+(boundaryReport.value.maxTangentPlaneAngleDeg?.toFixed(6)??label('не определён','undefined')) }}°</output>
                    <strong>{{ boundaryReport.value.sampledWithinTolerance?label('Проверенные точки в допуске','Sampled points within tolerance'):label('Допуск не подтверждён','Tolerance not confirmed') }}</strong>
                  </template>
                  <p v-if="boundaryReport?.error" id="surface-boundary-error" role="alert">{{ boundaryReport.error }}</p><button v-if="boundaryRetryVisible" @click="boundaryInspectionRevision++">{{ label('Повторить проверку стыка','Retry surface boundary inspection') }}</button>
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
              <label>{{ label('Радиус, мм', 'Radius, mm') }}<CadQuantityInput :aria-label="label('Радиус, мм', 'Radius, mm')" v-model="cornerRadius" kind="length" :locale="locale" @validity="quantityValidity('cornerRadius', $event)" :min=".01" :max="advancedOp==='curve'?1000000:undefined" step=".5" /></label>
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
            <label>{{ label('Поверхности вращения','Revolve surfaces') }}<select v-model="revolveGeometry" @keydown="revolveGeometryKey"><option value="faceted" :disabled="!!selectedSketch?.retainedProfile">{{ label('Гранёные','Faceted') }}</option><option value="exact">{{ label('Точные NURBS','Exact NURBS') }}</option></select></label>
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
            <div v-if="pane === '3d' && !advancedOp && !document.bodies.length && !(document.sourceBodies?.length) && !document.curves?.length && !document.surfaces?.length && !solidActive" class="empty-hint"><strong>{{ label('Из плоской фигуры — в объём', 'Turn a flat shape into a solid') }}</strong><span>{{ label('Выберите эскиз слева и нажмите «Выдавить»', 'Select a sketch on the left and press Extrude') }}</span></div>
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
            <li v-else-if="row.kind==='source' && row.item" :data-scene-key="row.key" :style="style" :aria-posinset="position" :aria-setsize="total" class="object-row" :class="{muted:!objectVisible(row.item.id),locked:lockedIds.includes(row.item.id)}">
              <button type="button" :disabled="!objectSelectable(row.item.id)||!sourceDisplayResults[row.item.id]?.edges.length" :aria-label="row.item.name" :aria-pressed="sourceEdgeSelection.startsWith(row.item.id+':')" @click="pickSourceBody(row.item.id)"><span class="dot nurbs"></span><span class="object-name" :title="row.item.name">{{ row.item.name }}</span><small>preview</small></button>
              <button :aria-label="label('Изолировать: ','Isolate: ')+row.item.name" :aria-pressed="isolatedBodyIds.length===1&&isolatedBodyIds[0]===row.item.id" @click="isolatedBodyIds=isolatedBodyIds.length===1&&isolatedBodyIds[0]===row.item.id?[]:[row.item.id]">◎</button>
              <SceneObjectControls :name="row.item.name" :locale="locale" :hidden="!objectVisible(row.item.id)" :locked="lockedIds.includes(row.item.id)" @visibility="toggleObjectState(row.item.id,'hidden')" @lock="toggleObjectState(row.item.id,'locked')" />
            </li>
            <li v-else-if="row.kind==='object' && row.item" :data-scene-key="row.key" :aria-posinset="position" :aria-setsize="total" :style="style" class="object-row" :class="{muted:!objectVisible(row.item.id),locked:lockedIds.includes(row.item.id)}"><button type="button" :disabled="!objectSelectable(row.item.id)" :aria-label="row.item.name" :aria-pressed="selectedIds.includes(row.item.id)" @click="pickObject(row.item.id,row.pane!,$event.shiftKey)"><span class="dot" :class="row.dot" :style="row.item.material?{background:row.item.material.color}:undefined"></span><span class="object-name" :title="row.item.name">{{row.item.name}}</span><small v-if="row.item.instance">{{label('Экземпляр','Instance')}}</small><small v-if="row.item.brep">B-rep</small><small v-if="row.dot!=='body' && row.item.group">{{row.item.group}}</small></button><SceneObjectControls :name="row.item.name" :locale="locale" :hidden="!objectVisible(row.item.id)" :locked="lockedIds.includes(row.item.id)" @visibility="toggleObjectState(row.item.id,'hidden')" @lock="toggleObjectState(row.item.id,'locked')" /></li>
            <li v-else-if="row.kind==='label'" :data-scene-key="row.key" :aria-posinset="position" :aria-setsize="total" :style="style" class="scene-group">{{row.title}}</li>
            <li v-else :data-scene-key="row.key" :aria-posinset="position" :aria-setsize="total" :style="style" class="scene-empty">{{label('Сцена пуста. Добавьте примитив или нарисуйте эскиз.','The scene is empty. Add a primitive or draw a sketch.')}}</li>
          </SceneVirtualList>
          <button type="button" :aria-expanded="manufacturingOpen" @click="manufacturingOpen=!manufacturingOpen">{{ label('Производство: G-code / Laser','Manufacturing: G-code / Laser') }}</button>
          <section v-if="manufacturingOpen" :aria-label="label('Производство','Manufacturing')">
            <p v-if="document.sourceBodies?.length" role="status">{{ label('Для исходных тел ещё не подготовлена замкнутая сетка. Сохраните нативный документ; подготовка G-code и Laser для этой сцены пока недоступна.','Native source bodies need a closed mesh before manufacturing. Save the native document; G-code and Laser preparation for this scene are not yet available.') }}</p>
            <template v-else>
            <SolidGcodePanel :meshes="manufacturingScene" :selection="manufacturingScene.length?[0]:[]" :source="'solid:'+manufacturingRevision" :ready="!restoringDraft" :locale="props.locale"/>
            <SolidLaserPanel :meshes="manufacturingScene" :selection="manufacturingScene.length?[0]:[]" :source="'solid:'+manufacturingRevision" :ready="!restoringDraft" :locale="props.locale"/>
            </template>
          </section>
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
          <fieldset class="manifold-metrics" data-testid="manifold-metrics" :aria-label="label('Метрики manifold-сетки','Manifold mesh metrics')">
            <legend>{{ label('Текущая сетка · Manifold','Current mesh · Manifold') }}</legend>
            <small v-if="manifoldMetricsPending" role="status">{{ label('Вычисляю метрики сетки…','Computing mesh metrics…') }}</small>
            <template v-if="manifoldMetrics?.value">
              <output>V = {{ manifoldMetrics.value.signedVolume.toFixed(6) }} mm³ · A = {{ manifoldMetrics.value.surfaceArea.toFixed(6) }} mm²</output>
              <small>χ = {{ manifoldMetrics.value.eulerCharacteristic }} · {{ label('компонент','components') }}: {{ manifoldMetrics.value.componentCount }} · {{ label('замкнутая','watertight') }}: {{ manifoldMetrics.value.watertight ? label('да','yes') : label('нет','no') }}</small>
              <small>{{ label('Род компонент','Component genus') }}: {{ manifoldMetrics.value.components.map(component => component.genus ?? '—').join(', ') }}</small>
            </template>
            <p v-if="manifoldMetrics?.error" role="alert">{{ manifoldMetrics.error }}</p>
            <button v-if="manifoldMetricsRetryVisible" @click="manifoldMetricsRevision++">{{ label('Повторить метрики сетки','Retry mesh metrics') }}</button>
            <div class="manifold-metrics-export">
              <button type="button" :disabled="manifoldMetricsExportPending" @click="exportManifoldMetrics('json')">{{ label('Экспорт JSON','Export JSON') }}</button>
              <button type="button" :disabled="manifoldMetricsExportPending" @click="exportManifoldMetrics('csv')">{{ label('Экспорт CSV','Export CSV') }}</button>
              <small>{{ label(`Выделено тел: ${selectedBodies.length}`,`Selected bodies: ${selectedBodies.length}`) }}</small>
            </div>
            <small v-if="manifoldMetricsExportPending" role="status">{{ label('Экспортирую метрики…','Exporting metrics…') }}</small>
            <p v-if="manifoldMetricsExportError" role="alert">{{ manifoldMetricsExportError }}</p>
          </fieldset>
          <label v-if="!clearanceOpen && !shellDistanceOpen && !volumeDistanceOpen && !materialPathOpen">{{ label('Вершина A','Vertex A') }}<input v-model.number="measureA" type="number" min="1" :max="solidVertexCount(selectedBody)" :aria-invalid="!vertexAValid" :aria-describedby="!vertexAValid ? 'vertex-measurement-error' : undefined" :aria-label="label('Вершина A','Vertex A')"></label>
          <label v-if="!materialPathOpen">{{ label('Тело B','Body B') }}<select v-model="measureTarget" :aria-describedby="shellDistanceOpen?'shell-distance-error':undefined" :aria-invalid="shellDistanceOpen&&(!measurementTarget?.brep||measurementTarget.id===selectedBody.id)" :aria-label="label('Тело B','Body B')"><option value="">{{ label('Выбранное тело','Selected body') }}</option><option v-for="body in document.bodies" :key="body.id" :value="body.id">{{ body.name }}</option></select></label>
          <label v-if="!clearanceOpen && !shellDistanceOpen && !volumeDistanceOpen && !materialPathOpen">{{ label('Вершина B','Vertex B') }}<input v-model.number="measureB" type="number" min="1" :max="measurementTarget?solidVertexCount(measurementTarget):1" :aria-invalid="!vertexBValid" :aria-describedby="!vertexBValid ? 'vertex-measurement-error' : undefined" :aria-label="label('Вершина B','Vertex B')"></label>
          <small v-if="measurementPending && !shellDistanceOpen && !volumeDistanceOpen && !materialPathOpen" role="status" aria-label="vertex-measurement">{{ label('Измеряю вершины…','Measuring vertices…') }} <button @click="measurementOpen=false">Esc</button></small>
          <output v-if="measurement?.value && !shellDistanceOpen && !volumeDistanceOpen && !materialPathOpen">{{ measurement.value.distanceMm.toFixed(6) }} mm · ΔXYZ [{{ measurement.value.deltaMm.map(v=>v.toFixed(6)).join(', ') }}]</output>
          <small v-if="measurement?.value && !shellDistanceOpen && !volumeDistanceOpen && !materialPathOpen">A [{{ measurement.value.a.map(v=>v.toFixed(6)).join(', ') }}] · B [{{ measurement.value.b.map(v=>v.toFixed(6)).join(', ') }}] mm</small>
          <p v-if="measurement?.error && !shellDistanceOpen && !volumeDistanceOpen && !materialPathOpen" id="vertex-measurement-error" role="alert">{{ measurement.error }}</p><button v-if="measurementRetryVisible" @click="measurementRevision++">{{ label('Повторить измерение вершин','Retry vertex measurement') }}</button>
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
            <p v-if="edgeDistance?.error" id="edge-distance-error" role="alert">{{ edgeDistance.error }}</p><button v-if="edgeDistanceRetryVisible" @click="edgeDistanceRevision++">{{ label('Повторить измерение рёбер','Retry edge distance') }}</button>
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
            <p v-if="faceDistance?.error" id="face-distance-error" role="alert">{{ faceDistance.error }}</p><button v-if="faceDistanceRetryVisible" @click="faceDistanceRevision++">{{ label('Повторить измерение граней','Retry face distance') }}</button>
          </fieldset>
          <SolidVolumeDistance @state="(active,point)=>{volumeDistanceOpen=active;volumeContact=point}" :active="props.open && measurementOpen" :ru="ru" :a="selectedBody.brep" :b="measurementTarget?.brep" :same="selectedBody.id===measurementTarget?.id" :names="[selectedBody.name,measurementTarget?.name??'B']"/>
          <MaterialPathCheck :key="selectedBody.id" :active="props.open && measurementOpen" :ru="ru" :model="selectedBody.brep" @state="(active,overlay)=>{materialPathOpen=active;materialPathOverlay=overlay}"/>
          <button v-if="selectedBody.brep" @click="shellDistanceOpen=!shellDistanceOpen" :aria-pressed="shellDistanceOpen">{{ label('Расстояние между оболочками','Distance between shells') }}</button>
          <fieldset v-if="shellDistanceOpen" class="shell-distance-panel" aria-label="shell-distance">
            <legend>{{ label('Расстояние между всеми гранями оболочек','Distance between all shell faces') }}</legend>
            <label>{{ label('Объём расчёта','Calculation budget') }}<select v-model.number="shellDistanceBudget" :aria-label="label('Объём расчёта оболочек','Shell calculation budget')"><option :value="100000">{{ label('Обычный','Standard') }}</option><option :value="1000000">{{ label('Расширенный','Extended') }}</option></select></label>
            <small v-if="shellDistancePending" role="status">{{ label('Измеряю расстояние…','Measuring distance…') }} <button @click="shellDistanceOpen=false">Esc</button></small>
            <ShellDistanceSummary v-if="shellDistance?.value" :value="shellDistance.value" :ru="ru"/>
            <p v-if="shellDistance?.error" id="shell-distance-error" role="alert">{{ shellDistance.error }}</p><button v-if="shellDistanceRetryVisible" @click="shellDistanceRevision++">{{ label('Повторить измерение оболочек','Retry shell distance') }}</button>
          </fieldset>
          <button @click="clearanceOpen=!clearanceOpen" :aria-pressed="clearanceOpen">{{ label('Зазор тел по сетке','Body mesh clearance') }}</button>
          <template v-if="clearanceOpen">
            <small>{{ label('Расчёт по сеткам тел. Для B-rep точность ограничена детализацией.','Measured on body meshes. B-rep accuracy is limited by tessellation.') }}</small>
            <p v-if="clearancePending" role="status">{{ label('Вычисляем зазор…','Computing clearance…') }}</p>
            <output v-if="clearanceMeasurement?.value">{{ label('Зазор','Clearance') }}: {{ formatMeasurement(clearanceMeasurement.value.gapMm) }} mm · {{ label('Перекрытие','Overlap') }}: {{ formatMeasurement(clearanceMeasurement.value.overlapMm3) }} mm³</output>
            <p v-if="clearanceMeasurement?.error" role="alert">{{ clearanceMeasurement.error }}</p><button v-if="clearanceRetryVisible" @click="clearanceRevision++">{{ label('Повторить расчёт зазора','Retry mesh clearance') }}</button>
          </template>
          <template v-if="selectedBody.brep && edgeIndex>=0">
            <small v-if="curvaturePending" role="status" aria-label="edge-measurement">{{ label('Измеряю кривизну…','Measuring curvature…') }} <button @click="measurementOpen=false">Esc</button></small>
            <label>{{ label('Параметр ребра (0…1)','Edge parameter (0…1)') }}<input v-model.number="curveParameter" type="number" min="0" max="1" step=".05" :aria-invalid="!curveParameterValid" :aria-describedby="!curveParameterValid ? 'curvature-measurement-error' : undefined" :aria-label="label('Параметр ребра','Edge parameter')"></label>
            <output v-if="curvatureMeasurement?.value">{{ label('Локальный радиус кривизны','Local curvature radius') }}: {{ curvatureMeasurement.value.radiusMm===null?'∞':curvatureMeasurement.value.radiusMm.toFixed(6)+' mm' }}</output>
            <p v-if="curvatureMeasurement?.error" id="curvature-measurement-error" role="alert">{{ curvatureMeasurement.error }}</p><button v-if="curvatureRetryVisible" @click="curvatureRevision++">{{ label('Повторить измерение кривизны','Retry curvature measurement') }}</button>
          </template>
          <small v-if="!clearanceOpen && !shellDistanceOpen && !volumeDistanceOpen && !materialPathOpen">{{ label('Расстояние между указанными вершинами, не минимальное расстояние между телами. Для радиуса выберите ребро B-rep.','Distance between the specified vertices, not the minimum distance between bodies. Select a B-rep edge to measure curvature.') }}</small>
        </template>
      </section>
      <section v-if="selectedBody" ref="diagnosticPanel" tabindex="-1" aria-label="Body diagnostics" class="body-diagnostics">
        <button @click="diagnosticsOpen=!diagnosticsOpen" :aria-pressed="diagnosticsOpen">{{ label('Диагностика тела','Body diagnostics') }}</button>
        <template v-if="diagnosticsOpen">
          <strong data-testid="diagnostic-body" :data-body-id="selectedBody.id">{{ label('Тело: ','Body: ')+selectedBody.name }}</strong>
          <button v-if="selectedBody.brep" @click="faceContactsOpen=!faceContactsOpen" :aria-pressed="faceContactsOpen">{{ label('Проверить контакты граней','Inspect face contacts') }}</button>
          <fieldset v-if="faceContactsOpen" aria-label="face-contacts" class="boundary-agreement-panel">
            <legend>{{ label('Контакты B-rep-граней','B-rep face contacts') }}</legend>
            <label class="diagnostic-checkbox"><input v-model="inspectWithinFaces" type="checkbox">{{ label('Проверять внутри граней','Inspect within faces') }}</label>
            <label v-if="inspectWithinFaces" class="diagnostic-checkbox"><input v-model="inspectExactBoundary" type="checkbox">{{ label('Проверять точное совпадение и замыкание границ','Inspect exact boundary agreement and closure') }}</label>
            <label>{{ label('Лимит проверки контактов','Contact inspection limit') }}<input v-model.number="faceContactsBudget" type="number" min="1" max="1000000" step="1" :aria-invalid="faceContactsBudgetInvalid" aria-describedby="face-contacts-error" :aria-label="label('Лимит проверки контактов','Contact inspection limit')"></label>
            <p v-if="faceContactsPending" role="status" aria-label="face-contacts-pending">{{ label('Проверяю контакты…','Inspecting contacts…') }} <button @click="faceContactsOpen=false">Esc</button></p>
            <template v-if="faceContactsResult">
              <template v-if="'absenceProven' in faceContactsResult">
                <p role="status">{{ faceContactsResult.absenceProven ? label('Отсутствие самопересечений подтверждено.','Absence of self-intersections is proven.') : label('Отсутствие самопересечений не доказано.','Absence of self-intersections is unproven.') }}</p>
                <p v-if="faceContactsResult.boundaryEmbedding" data-testid="boundary-embedding-summary">{{ faceContactsResult.boundaryEmbedding.proven ? label('Границы согласованы, замкнуты и не пересекаются.','Boundary agreement, closure and absence of intersections are proven.') : label('Полная проверка границ не завершена.','Complete boundary proof is unresolved.') }}</p>
                <p v-if="faceContactsResult.allFacesInjective">{{ label('Все грани проверены на самоналожение.','All faces are proven free of self-overlap.') }}</p>
                <p v-else>{{ label('Не проверены или не доказаны грани: ','Unvisited or unproven faces: ')+faceContactsResult.faces.filter(f=>!faceAbsenceProven(f)).map(f=>f.face+1).join(', ') }}</p>
                <p v-if="faceContactsResult.faces.some(f=>f.quotientProof?.proven)">{{ label('Самоналожение исключено на гранях со схлопнутой границей: ','Self-overlap is excluded on faces with a collapsed boundary: ')+faceContactsResult.faces.filter(f=>f.quotientProof?.proven).map(f=>f.face+1).join(', ')+label('. Гладкость в конечной точке не подтверждена.','. Endpoint smoothness is unqualified.') }}</p>
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
          <p v-if="sectionInputInvalid" id="solid-section-error" role="alert">{{ label('Сечение не проверено. Исправьте выделенные поля плоскости.','Section not checked. Correct the highlighted plane fields.') }}</p>
          <template v-else-if="diagnostics?.value?.sectionError"><p id="solid-section-error" role="alert">{{ sectionErrorMessage(diagnostics.value.sectionError) }}</p><button v-if="diagnostics.value.sectionError.includes('Section normal')" @click="sectionNormal=[0,0,1]">{{ label('Плоскость XY','XY plane') }}</button><details><summary>{{ label('Технические сведения','Technical details') }}</summary>{{ diagnostics.value.sectionError }}</details></template>
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
      <span v-if="tool==='slot'">{{ label('Потяните от центра одного торца к другому. Esc — отмена.','Drag between the two end-cap centers. Esc cancels.') }}</span><template v-if="tool === 'polyline' && draft.length"><button @click="undoDraftPoint">{{ label('Убрать точку · Backspace','Undo point · Backspace') }}</button><span>{{ draft.length }} {{ label('точек', 'points') }}</span><button :disabled="draft.length < 3" @click="finish(true)">{{ label('Замкнуть контур', 'Close contour') }}</button><button :disabled="draft.length < 2" @click="finish(false)">{{ label('Завершить линию', 'Finish line') }}</button><button @click="cancelGesture">Esc</button></template>
    </footer>
    <CommandPalette v-if="paletteOpen" :open="paletteOpen" :commands="solidCommands" @close="paletteOpen = false" @execute="executeSolidCommand" />
    <div class="input-hint" role="status">{{ inputMode === 'touch' ? label('Два пальца: масштаб и перенос · Навигация: одним пальцем вращать 3D / двигать 2D', 'Two fingers: zoom and pan · Navigate: one finger orbits 3D / pans 2D') : label('ЛКМ: выбор / вращение 3D · СКМ или Shift: перенос · Колесо: масштаб', 'Left drag: select / orbit 3D · Middle drag or Shift: pan · Wheel: zoom') }}</div>
    <div v-if="showHelp" class="help-card"><p>{{ label('Грани: выберите поверхность, затем тяните её или жёлтую ручку. Ctrl/⌘ + клик выбирает несколько открытых граней для Shell. Для фаски и скругления включите «Рёбра».','Faces: select a surface, then drag it or its yellow handle. Ctrl/⌘ click selects multiple Shell openings. Switch to Edges for chamfers and fillets.') }}</p><p>{{ label('Shift + клик и «Рамка» выделяют несколько объектов. Манипулятор двигает, вращает и масштабирует весь выбор. У окружностей и дуг есть ручки центра, радиуса и концов дуги.','Shift click and Box select select multiple objects. The gizmo moves, rotates and scales the whole selection. Circles and arcs have center, radius and arc endpoint handles.') }}</p><strong>{{ label('Управление', 'Controls') }}</strong><p>{{ label('2D: тяните фигуру или вершину. Alt временно отключает привязку. Ломаная замыкается кликом по первой точке.', '2D: drag shapes or vertices. Alt bypasses snapping. Close a polyline by clicking its first point.') }}</p><p>{{ label('3D: тяните для вращения; G включает перемещение тела. ПКМ всегда вращает. Shift или средняя кнопка — панорама. Колесо — масштаб.', '3D: drag to orbit; G enables body movement. Right drag always orbits. Shift or middle drag pans. Wheel zooms.') }}</p><p>{{ label('E — предпросмотр выдавливания; зелёная ручка меняет высоту. Enter подтверждает, Escape отменяет. Ctrl/⌘ Z — отмена, Ctrl/⌘ Shift Z — повтор.', 'E previews extrusion; the green handle changes height. Enter applies, Escape cancels. Ctrl/⌘ Z undoes; Ctrl/⌘ Shift Z redoes.') }}</p><button @click="showHelp = false">{{ label('Понятно', 'Got it') }}</button></div>
    <div v-if="polygonView.limited" class="notice-bar" role="status" aria-label="transparency-limit">{{ label('Прозрачность приблизительная. Скройте часть сцены.','Transparency is approximate. Hide some objects.') }}</div>
    <div v-if="gpuRetryVisible || gpuInitializing" class="notice-bar" role="status" aria-label="gpu-recovery"><span>{{ gpuInitializing ? label('Восстанавливаю WebGPU…','Restoring WebGPU…') : gpuFailureNotice }}</span><button v-if="gpuRetryVisible" type="button" :disabled="gpuInitializing" @click="retryGpuLayer">{{ label('Повторить WebGPU','Retry WebGPU') }}</button></div>
    <div v-if="error" class="error-bar" role="alert">{{ error }} <button @click="error = ''">×</button></div>
    <div v-else-if="notice && !((gpuRetryVisible || gpuInitializing) && notice===gpuFailureNotice)" class="notice-bar" role="status">{{ notice }} <button @click="notice = ''">×</button></div>
  </section>
    <div v-if="minkowskiProof" class="notice-bar" role="status" :aria-label="label('Сертификат суммы Минковского','Minkowski sum certificate')">
      {{ minkowskiProof.resultId===null?label('Сумма подтверждена: результат пуст.','Sum verified: the result is empty.'):minkowskiProof.curvedSourceErrorCertified?label('Проверены границы суммы исходных поверхностей.','Source surface sum enclosures verified.'):label('Сумма полигональных оболочек подтверждена.','Polyhedral sum verified.') }}
      {{ label('Топология при округлении сохранена.','Topology preserved through rounding.') }}
      <span v-if="minkowskiProof.curvedSourceErrorCertified">{{ minkowskiProof.approximationTopology?label('Топология аппроксимации исходных поверхностей и суммы доказана.','Source surface and sum approximation topology proved.'):minkowskiProof.sourceApproximationTopology?.every(Boolean)?label('Топология исходных тел сохранена; для суммы не доказана.','Source body topology is preserved; sum topology is unproved.'):label('Топология аппроксимации исходных поверхностей не доказана.','Source surface approximation topology is unproved.') }}</span>
      {{ minkowskiProof.maximumSetDeviationMm===null?label('Единая граница расстояния не доказана; сохранены внутреннее и внешнее приближения.','A global distance bound is unproved; inner and outer enclosures are retained.'):label('Отклонение результата ≤ ','Result deviation ≤ ')+formatUpperMm(minkowskiProof.maximumSetDeviationMm)+' mm' }}
      {{ minkowskiProof.maximumCoordinateRoundingMm?label('Погрешность координат сетки ≤ ','Mesh coordinate error ≤ ')+formatUpperMm(minkowskiProof.maximumCoordinateRoundingMm)+' mm':label('Координаты сетки точные.','Mesh coordinates are exact.') }}
      <button @click="downloadMinkowskiProof">{{ label('Скачать сертификат','Download certificate') }}</button>
    </div>
</template>
<style scoped src="./styles/direct-modeler.css"></style>
