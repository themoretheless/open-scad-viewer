<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, shallowRef, watch } from 'vue'
import { bodyPoints, DirectHistory, directBodiesScad, emptyDirectDocument, extrudeDirectSketch, parseDirectDocument, transformDirectPoints, type DirectDocument, type Point2 } from '../services/directModeling'
import { directCornerTool, directRevolveTool, applyDirectRevolve } from '../services/directProfileTools'
import { sampleCurve, transformSketch, bakeSketch, offsetSketch, trimSketch, extendSketch, worldPoint, xyPlane, unit3, cross3, type SketchPlane, type Vec3 } from '../services/directSketchGeometry'
import { solidTopology, facePlane, pushPullFace, bevelSolidEdge, shellSolid, splitSolid, transformSelection } from '../services/directSolidTools'
import { storageGet, storageSet } from '../services/safeStorage'
import { exportPolygonStl, polygonBoundaryLoops, revolvePolygonProfile } from '../services/geometry/polygon'
import { applyDirectExtrusion, circularDirectCopies, defaultDirectCamera, directExtrusionTool, directFaceShade, projectDirectPoint, snapDirectPoint, unprojectDirectXY } from '../services/directModelingTools'
import { stlBufferToPolygonMesh } from '../services/meshEditing'
import { solidDocumentToMeshDocument } from '../services/solidBridge'
import { createSolidNurbsCurve, createSolidNurbsSurface, importModelGraphNurbs, nurbsCurveToSketch, sampleSolidNurbsCurve, tessellateSolidNurbsSurface, updateSolidNurbsControlPoint } from '../services/solidNurbs'
import { elevateNurbsCurve, insertNurbsKnot } from '../services/nurbsCurve'
import { elevateNurbsSurface, insertNurbsSurfaceKnot, isoNurbsCurve, trimNurbsSurface } from '../services/nurbsSurface'
import { extrudeNurbsCurve } from '../services/nurbsConstructors'
const props = defineProps<{ open: boolean; locale: string; canAppend: boolean; remainingSource: number; embedded?: boolean; initialDocument?: DirectDocument; initialSelection?: string; seedDocument?: DirectDocument | null }>()
const emit = defineEmits<{ close: []; append: [source: string]; toMesh: [] }>()
const ru = computed(() => props.locale === 'ru')
const label = (a: string, b: string) => ru.value ? a : b
const key = props.embedded ? 'scad-main-modeler-v1' : 'scad-solid-modeler-v1'
const error = ref(''), saveError = ref(false)
let stored = storageGet(key) ?? storageGet(props.embedded ? 'scad-main-modeler-v1' : 'scad-direct-modeler-v1')
let initial = emptyDirectDocument()
try { if (stored) initial = parseDirectDocument(stored) } catch { error.value = 'Saved solid document is invalid. Import a backup to recover.' }
if (props.initialDocument) initial = props.initialDocument
const history = new DirectHistory(initial)
const document = shallowRef(history.document)
const stlInput = ref<HTMLInputElement>()
const undoable = ref(false), redoable = ref(false)
const selection = ref(props.initialSelection ?? ''), mode = ref<'2d' | '3d'>('2d'), tool = ref<'select' | 'rectangle' | 'circle' | 'arc' | 'polyline' | 'trim'>('select')
const draft = ref<Point2[]>([]), height = ref(10), dx = ref(0), dy = ref(0), dz = ref(0), angle = ref(0), scale = ref(1)
type Pane = '2d' | '3d'
const workspace = ref<HTMLElement>(), splitArea = ref<HTMLElement>()
const split = ref(50)
const views = ref<Record<Pane, number>>({ '2d': 160, '3d': 160 })
const centers = ref<Record<Pane, Point2>>({ '2d': [0, 0], '3d': [0, 0] })
const panes: Pane[] = ['2d', '3d']
const camera = ref(defaultDirectCamera()), hovered = ref(''), snap = ref(true), grid = ref(1)
const snapMarker = ref<Point2 | null>(null)
const drawMeasure = ref(''), operation = ref<'extrude' | 'revolve' | 'fillet' | 'dogear' | 'array' | null>(null)
const cornerVertex = ref(0), cornerRadius = ref(2)
const revolveAxis = ref<'x'|'y'>('y'), revolveOffset = ref(0), revolveAngle = ref(360), revolveSegments = ref(48)
const revolveOptions = () => ({ axis: revolveAxis.value, offset: revolveOffset.value, angle: revolveAngle.value, segments: revolveSegments.value })
const cornerActive = computed(() => operation.value === 'fillet' || operation.value === 'dogear')
const solidActive = computed(() => operation.value === 'extrude' || operation.value === 'revolve')
const cornerPreview = computed(() => {
  if (!cornerActive.value || !selectedSketch.value) return { sketch: null, error: '' }
  try { return { sketch: directCornerTool(selectedSketch.value, cornerVertex.value, cornerRadius.value, operation.value as 'fillet'|'dogear'), error: '' } }
  catch(e) { return { sketch: null, error: e instanceof Error ? e.message : String(e) } }
})
function beginCorner(kind: 'fillet'|'dogear') { boxSelect.value=false; advancedOp.value=null; if (!selectedSketch.value?.closed) return; cancelGesture(); tool.value = 'select'; operation.value = kind; mode.value = '2d' }
function applyCorner() { run(() => {
  if (!cornerPreview.value.sketch) return
  const d = history.document, index = d.sketches.findIndex(s=>s.id===selection.value)
  if (index < 0) return
  d.sketches[index] = cornerPreview.value.sketch; commit(d); operation.value = null
}) }
const baseZ = ref(0), extrusionMode = ref<'new' | 'union' | 'difference'>('new'), targetBody = ref('')
const copyCount = ref(8), copySweep = ref(360), copyX = ref(0), copyY = ref(0)
const previewBody = shallowRef<ReturnType<typeof directExtrusionTool> | null>(null), previewError = ref('')
const movingBody = ref(false), showHelp = ref(false), floorVisible = ref(true)
let fitNextPreview = false
let previewTimer: ReturnType<typeof setTimeout> | undefined
onUnmounted(() => { clearTimeout(previewTimer) })
let orbitDrag: { x: number; y: number; yaw: number; pitch: number; pointer: number; svg: SVGSVGElement } | null = null
let heightDrag: { y: number; height: number; pointer: number; svg: SVGSVGElement } | null = null
let gesture: { start: Point2; document: DirectDocument; vertex: number | null; id: string; pointer: number; pane: Pane; svg: SVGSVGElement; pan: boolean; center: Point2 } | null = null
const extraSelection=ref<string[]>([]),pickMode=ref<'body'|'face'|'edge'>('body'),faceIndex=ref(-1),edgeIndex=ref(-1),openingFaces=ref<number[]>([])
const workplaneOutline=ref<Point2[][]>([])
const activePlane=ref<SketchPlane>(xyPlane()),advancedOp=ref<'push'|'chamfer'|'edge-fillet'|'shell'|'split'|'offset'|'extend'|'curve'|'transform'|null>(null)
const advanced=ref({distance:2,radius:2,axis:'z' as 'x'|'y'|'z',x:0,y:0,z:0,angle:0,scale:1,cx:0,cy:0,start:0,sweep:180,end:'end' as 'start'|'end'})
const boxSelect=ref(false),selectionBox=ref<{start:Point2;end:Point2;pane:Pane}|null>(null),gizmoMode=ref<'move'|'rotate'|'scale'>('move')
let manipulatorDrag:{svg:SVGSVGElement;pointer:number;x:number;y:number;kind:'move'|'rotate'|'scale'|'push'|'split';axis:'x'|'y'|'z';direction:Point2;before:DirectDocument;initial:number;startPoint:Point2;center:Point2}|null=null

let curveDrag:{id:string;kind:'center'|'radius'|'start'|'end';before:DirectDocument;pointer:number}|null=null
let cvDrag:{id:string;u:number;v:number;before:DirectDocument;point:number[];start:Point2;pointer:number;svg:SVGSVGElement}|null=null
let previousFocus: HTMLElement | null = null
watch(() => props.open, async open => {
  if (open) {
    previousFocus = window.document.activeElement as HTMLElement
    await nextTick(); workspace.value?.focus()
  }
  else { cancelGesture(); operation.value = null; previousFocus?.focus() }
}, { immediate: true })

watch(() => props.seedDocument, seed => {
  if (!props.open || !seed) return
  cancelGesture()
  history.commit(parseDirectDocument(JSON.stringify(seed)))
  selection.value = seed.bodies[0]?.id ?? ''
  sync()
})
const selectedSketch = computed(() => document.value.sketches.find(s => s.id === selection.value))
const selectedBody = computed(() => document.value.bodies.find(s => s.id === selection.value))
const selectedNurbsCurve = computed(() => document.value.curves?.find(s => s.id === selection.value))
const selectedNurbsSurface = computed(() => document.value.surfaces?.find(s => s.id === selection.value))
const selectedNurbs = computed(() => selectedNurbsCurve.value ?? selectedNurbsSurface.value)
const cvU = ref(0), cvV = ref(0), cvX = ref(0), cvY = ref(0), cvZ = ref(0), cvWeight = ref(1), knotValue = ref(.5)
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
const selectedIds = computed(() => [...new Set([selection.value,...extraSelection.value].filter(Boolean))])
const topology = computed(() => selectedBody.value ? solidTopology(selectedBody.value.mesh) : {faces:[],edges:[]})
const selectedFace = computed(() => topology.value.faces[faceIndex.value])
const selectedFaceTriangles=computed(()=>new Set((openingFaces.value.length?openingFaces.value:[faceIndex.value]).flatMap(i=>topology.value.faces[i]?.triangles??[])))
const samePlane = (a?:SketchPlane,b?:SketchPlane) => JSON.stringify(a??xyPlane())===JSON.stringify(b??xyPlane())
const visibleSketches = computed(() => document.value.sketches.filter(s=>samePlane(s.plane,activePlane.value)))
function pickObject(id:string,pane:Pane,add=false) {
 if(id!==selection.value){faceIndex.value=edgeIndex.value=-1;openingFaces.value=[]}
 if(add){const ids=new Set(selectedIds.value);ids.has(id)?ids.delete(id):ids.add(id);const all=[...ids];selection.value=all[0]??'';extraSelection.value=all.slice(1)}
 else { selection.value=id;extraSelection.value=[] }
 mode.value=pane
 const sketch=document.value.sketches.find(s=>s.id===id);if(sketch){if(!samePlane(sketch.plane,activePlane.value))workplaneOutline.value=[];activePlane.value=sketch.plane??xyPlane()}
}
function beginAdvanced(kind:typeof advancedOp.value) {
 boxSelect.value=false
 cancelGesture(); operation.value=null;advancedOp.value=kind
 if(kind==='curve'&&selectedSketch.value?.analytic){const a=selectedSketch.value.analytic;advanced.value={...advanced.value,cx:a.center[0],cy:a.center[1],radius:a.radius,start:a.start,sweep:a.sweep}}
}
function faceSketch() {run(()=>{
 if(!selectedBody.value||!selectedFace.value)return
 const b=selectedBody.value,f=selectedFace.value,plane=facePlane(b,f),points=bodyPoints(b)
 workplaneOutline.value=polygonBoundaryLoops({positions:b.mesh.positions,indices:f.triangles.flatMap(t=>b.mesh.indices.slice(t*3,t*3+3))}).map(loop=>loop.map(i=>{const q=points[i].map((v,k)=>v-plane.origin[k]);return [q.reduce((s,v,k)=>s+v*plane.u[k],0),q.reduce((s,v,k)=>s+v*plane.v[k],0)] as Point2}))
 activePlane.value=plane;extraSelection.value=[];selection.value='';tool.value='rectangle';mode.value='2d';centers.value['2d']=[0,0];views.value['2d']=100;advancedOp.value=null
})}
function resultAdvanced():DirectDocument {
 const d=history.document,p=advanced.value,id=selection.value,b=d.bodies.find(b=>b.id===id),s=d.sketches.find(s=>s.id===id)
 const replace=(body:typeof b)=>{if(body)d.bodies[d.bodies.findIndex(b=>b.id===id)]=body}
 switch(advancedOp.value){
  case 'push': if(b)replace(pushPullFace(b,faceIndex.value,p.distance));break
  case 'chamfer': case 'edge-fillet': if(b)replace(bevelSolidEdge(b,edgeIndex.value,p.radius,advancedOp.value==='chamfer'?'chamfer':'fillet'));break
  case 'shell': if(b)replace(shellSolid(b,openingFaces.value.length?openingFaces.value:[faceIndex.value],p.distance));break
  case 'split': if(b){const pair=splitSolid(b,axisVector(p.axis),p.distance);pair[1].id='preview-split';replace(pair[0]);d.bodies.push(pair[1])}break
  case 'offset': if(s)d.sketches[d.sketches.findIndex(s=>s.id===id)]=offsetSketch(s,p.distance);break
  case 'extend': if(s)d.sketches[d.sketches.findIndex(s=>s.id===id)]=extendSketch(s,p.end,d.sketches.filter(o=>samePlane(o.plane,s.plane)));break
  case 'curve': if(s?.analytic){s.analytic={...s.analytic,center:[p.cx,p.cy],radius:p.radius,start:p.start,sweep:p.sweep};s.points=sampleCurve(s.analytic)}break
  case 'transform': return transformSelection(d,selectedIds.value,[p.x,p.y,p.z],axisVector(p.axis),p.angle,p.scale)
 }
 return parseDirectDocument(JSON.stringify(d))
}
const advancedPreview = computed(() => {
 if(!advancedOp.value)return {document:null,error:''}
 // Explicit reactive dependencies: history itself intentionally is not reactive.
 void document.value;void selection.value;void faceIndex.value;void edgeIndex.value;void openingFaces.value;void advanced.value
 try{return {document:resultAdvanced(),error:''}}catch(e){return {document:null,error:e instanceof Error?e.message:String(e)}}
})
function applyAdvanced(){run(()=>{
 if(!advancedPreview.value.document)return
 const d=structuredClone(advancedPreview.value.document),splitBody=d.bodies.find(b=>b.id==='preview-split');if(splitBody)splitBody.id=crypto.randomUUID()
 commit(d);advancedOp.value=null;faceIndex.value=edgeIndex.value=-1;openingFaces.value=[]
})}
const axisVector=(axis:string):Vec3=>axis==='x'?[1,0,0]:axis==='y'?[0,1,0]:[0,0,1]
const advancedPolygons=computed(()=>advancedPreview.value.document?.bodies.filter(b=>selectedIds.value.includes(b.id)||b.id==='preview-split').flatMap(meshPolygons).sort((a,b)=>a.depth-b.depth)??[])
const advancedSketches=computed(()=>advancedPreview.value.document?.sketches.filter(s=>selectedIds.value.includes(s.id)&&samePlane(s.plane,activePlane.value))??[])
const featureEdges=computed(()=>selectedBody.value?topology.value.edges.map((e,i)=>({i,points:[e.a,e.b].map(j=>project(selectedBody.value!.mesh.positions.slice(j*3,j*3+3),'3d').join(',')).join(' ')})):[])
const splitPlanePoints=computed(()=>{
 if(advancedOp.value!=='split'||!selectedBody.value)return ''
 const n=axisVector(advanced.value.axis),u=unit3(cross3(n,n[0]?[0,1,0]:[1,0,0])),v=cross3(n,u),p=bodyPoints(selectedBody.value),size=Math.max(...p.map(p=>Math.hypot(...p)))+10
 return [[-size,-size],[size,-size],[size,size],[-size,size]].map(q=>project(worldPoint(q,{origin:n.map(x=>x*advanced.value.distance) as Vec3,u,v}),'3d').join(',')).join(' ')
})
const gizmoCenter=computed(()=>{
 const p=[...document.value.bodies.filter(b=>selectedIds.value.includes(b.id)).flatMap(bodyPoints),...document.value.sketches.filter(s=>selectedIds.value.includes(s.id)).flatMap(s=>s.points.map(p=>worldPoint(p,s.plane)))]
 if(!p.length)return null
 return [0,1,2].map(k=>(Math.min(...p.map(p=>p[k]))+Math.max(...p.map(p=>p[k])))/2) as Vec3
})
const gizmoAxes=computed(()=>gizmoCenter.value?(['x','y','z'] as const).map((axis,i)=>{
 const n=axisVector(axis),c=gizmoCenter.value!,length=views.value['3d']/7
 const u=unit3(cross3(n,n[0]?[0,1,0]:[1,0,0])),v=cross3(n,u)
 return {axis,color:['#ff7777','#77df9d','#77baff'][i],base:project(c,'3d'),tip:project(c.map((x,k)=>x+n[k]*length),'3d'),ring:Array.from({length:65},(_,i)=>{const a=i*Math.PI/32;return project(c.map((x,k)=>x+length*.7*(u[k]*Math.cos(a)+v[k]*Math.sin(a))),'3d').join(',')}).join(' ')}
}):[])
function startGizmo(e:PointerEvent,kind:'move'|'rotate'|'scale'|'push'|'split',axis:'x'|'y'|'z') {
 e.preventDefault();e.stopPropagation();const svg=canvasOf(e),center=kind==='push'?selectedFace.value?.center:gizmoCenter.value;if(!center)return
 const n=kind==='push'?selectedFace.value!.normal:axisVector(axis),a=project(center,'3d'),b=project(center.map((x,i)=>x+n[i]),'3d')
 if(kind==='push'){advanced.value.distance=0;beginAdvanced('push')}
 manipulatorDrag={svg,pointer:e.pointerId,x:e.clientX,y:e.clientY,kind,axis,direction:[b[0]-a[0],b[1]-a[1]],before:history.document,initial:advanced.value.distance,startPoint:position(e),center:project(center,'3d')};svg.setPointerCapture(e.pointerId)
}
function startCurve(e:PointerEvent,kind:'center'|'radius'|'start'|'end'){
 if(!selectedSketch.value?.analytic)return
 e.preventDefault();e.stopPropagation();advancedOp.value=null;curveDrag={id:selection.value,kind,before:history.document,pointer:e.pointerId};canvasOf(e).setPointerCapture(e.pointerId)
}
function startCv(e:PointerEvent,u:number,v:number) {
 const point=selectedNurbsCurve.value?.curve.controlPoints[u]??selectedNurbsSurface.value?.surface.controlPoints[u]?.[v]
 if(!selectedNurbs.value||!point)return
 e.preventDefault();e.stopPropagation();cvU.value=u;cvV.value=v
 const svg=canvasOf(e);cvDrag={id:selectedNurbs.value.id,u,v,before:history.document,point:[...point],start:position(e),pointer:e.pointerId,svg};svg.setPointerCapture(e.pointerId)
}
function pickEdge(e:PointerEvent,index:number){extraSelection.value=[];e.stopPropagation();edgeIndex.value=index;faceIndex.value=-1;advancedOp.value=null}
function trimAt(id:string,p:Point2){run(()=>{
 const d=history.document,s=d.sketches.find(s=>s.id===id);if(!s)return
 let best=Infinity,index=0,t=0
 for(let i=0;i<(s.closed?s.points.length:s.points.length-1);i++){const a=s.points[i],b=s.points[(i+1)%s.points.length],vx=b[0]-a[0],vy=b[1]-a[1],f=Math.max(0,Math.min(1,((p[0]-a[0])*vx+(p[1]-a[1])*vy)/(vx*vx+vy*vy))),dist=Math.hypot(p[0]-a[0]-f*vx,p[1]-a[1]-f*vy);if(dist<best){best=dist;index=i;t=f}}
 const pieces=trimSketch(s,index,t,d.sketches.filter(o=>samePlane(o.plane,s.plane)));pieces.forEach((p,i)=>{if(i)p.id=crypto.randomUUID()});d.sketches=d.sketches.filter(o=>o.id!==id);d.sketches.push(...pieces);commit(d)
})}

function run(action: () => void) { error.value = ''; try { action() } catch (e) { error.value = e instanceof Error ? e.message : String(e) } }
function persist() {
  const current = storageGet(key)
  if (current !== stored) { saveError.value = true; error.value = label('Документ изменён в другой вкладке. Скачайте JSON, чтобы сохранить свои правки.', 'Document changed in another tab. Download JSON to keep your edits.'); return }
  const persisted = structuredClone(document.value)
  if (!persisted.curves?.length) delete persisted.curves
  if (!persisted.surfaces?.length) delete persisted.surfaces
  const text = JSON.stringify(persisted)
  saveError.value = !storageSet(key, text)
  if (!saveError.value) stored = text
}
function sync() { document.value = history.document; const s=document.value.sketches.find(s=>s.id===selection.value);if(s&&!samePlane(s.plane,activePlane.value)){activePlane.value=s.plane??xyPlane();workplaneOutline.value=[]} undoable.value = history.canUndo; redoable.value = history.canRedo; persist() }
function commit(next: DirectDocument) { history.commit(next); sync() }
function undo(redo = false) { operation.value = null; advancedOp.value=null; cancelGesture(); redo ? history.redo() : history.undo(); sync() }
function addSketch(points: Point2[], closed: boolean, analytic?: import('../services/directSketchGeometry').AnalyticCurve) {
  const d = history.document, id = crypto.randomUUID()
  d.sketches.push({ id, name: label('Эскиз ', 'Sketch ') + (d.sketches.length + 1), points, closed, analytic, plane: JSON.parse(JSON.stringify(activePlane.value)) })
  commit(d); pickObject(id,'2d')
}
function finish(closed: boolean) { run(() => { if (draft.value.length < (closed ? 3 : 2)) return; addSketch(draft.value, closed); draft.value = []; tool.value = 'select' }) }
function beginExtrude(kind: 'extrude'|'revolve' = 'extrude') {
  boxSelect.value=false;advancedOp.value=null
  if (!selectedSketch.value?.closed) return
  fitNextPreview = true; operation.value = kind; mode.value = '3d'; previewError.value = ''
  if (!document.value.bodies.some(b => b.id === targetBody.value)) targetBody.value = document.value.bodies[0]?.id ?? ''
}
function extrude() { run(() => {
  if (!selectedSketch.value || previewError.value || !previewBody.value) return
  const id = crypto.randomUUID()
  commit(operation.value === 'revolve'
    ? applyDirectRevolve(history.document, selectedSketch.value.id, revolveOptions(), extrusionMode.value, targetBody.value, id)
    : applyDirectExtrusion(history.document, selectedSketch.value.id, height.value, baseZ.value, extrusionMode.value, targetBody.value, id))
  operation.value = null; mode.value = '3d'; selection.value = extrusionMode.value === 'new' ? id : targetBody.value; fit('3d')
}) }
const copyPreview = computed(() => {
  if (operation.value !== 'array' || !selectedSketch.value) return []
  try { let i = 0; return circularDirectCopies(selectedSketch.value, copyCount.value, [copyX.value, copyY.value], copySweep.value, () => 'preview-' + i++) } catch { return [] }
})
function applyCopies() { run(() => { if (!selectedSketch.value) return; const d = history.document; d.sketches.push(...circularDirectCopies(selectedSketch.value, copyCount.value, [copyX.value, copyY.value], copySweep.value, () => crypto.randomUUID())); commit(d); operation.value = null }) }
function duplicate() { run(() => {
 const d=history.document,moved=transformSelection(d,selectedIds.value,[10,10,0],[0,0,1],0,1),ids:string[]=[]
 for(const s of moved.sketches)if(selectedIds.value.includes(s.id)){s.id=crypto.randomUUID();s.name=(s.name+' · copy').slice(0,100);ids.push(s.id);d.sketches.push(s)}
 for(const b of moved.bodies)if(selectedIds.value.includes(b.id)){b.id=crypto.randomUUID();b.name=(b.name+' · copy').slice(0,100);ids.push(b.id);d.bodies.push(b)}
 if(!ids.length)return
 commit(d);pickObject(ids[0],mode.value);extraSelection.value=ids.slice(1)
}) }
watch([operation, selectedSketch, height, baseZ, revolveAxis, revolveOffset, revolveAngle, revolveSegments], () => {
  clearTimeout(previewTimer); previewBody.value = null; previewError.value = ''
  if (!solidActive.value || !selectedSketch.value) return
  previewTimer = setTimeout(() => {
    try { previewBody.value = operation.value === 'revolve' ? directRevolveTool(selectedSketch.value!,revolveOptions()) : directExtrusionTool(selectedSketch.value!,height.value,baseZ.value); if (fitNextPreview) { fit('3d'); fitNextPreview = false } }
    catch (e) { previewError.value = e instanceof Error ? e.message : String(e) }
  }, 60)
})
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
function transform() { run(() => { advancedOp.value=null;operation.value=null;
 if(selectedSketch.value&&selectedIds.value.length===1){const d=history.document,i=d.sketches.findIndex(s=>s.id===selection.value);d.sketches[i]=transformSketch(d.sketches[i],[dx.value,dy.value],angle.value,scale.value);commit(d)}
 else commit(transformSelection(history.document,selectedIds.value,[dx.value,dy.value,dz.value],[0,0,1],angle.value,scale.value))
 dx.value=dy.value=dz.value=angle.value=0;scale.value=1
}) }
function appendBodies() {
  run(() => {
    const source = directBodiesScad(document.value)
    if (!props.canAppend || source.length > props.remainingSource) throw new Error(label('Сохраните тела в SCAD: текущий документ не подходит для добавления.', 'Download SCAD: the current document cannot accept these bodies.'))
    emit('append', source); emit('close')
  })
}
function sendToMesh() {
  run(() => {
    storageSet('scad-mesh-modeler-v1', JSON.stringify(solidDocumentToMeshDocument(document.value)))
    emit('toMesh')
  })
}
function download(text: string, name: string) { const url = URL.createObjectURL(new Blob([text], { type: 'text/plain' })); const a = window.document.createElement('a'); a.href = url; a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(url), 1000) }
async function importFile(event: Event) {
  const input = event.target as HTMLInputElement, file = input.files?.[0]
  try { if (file) { if (file.size > 4_000_000) throw new Error('Document exceeds 4 MB.'); const text = await file.text(); run(() => {
    cancelGesture()
    const parsed = JSON.parse(text)
    if (parsed?.language === 'modelgraph/nurbs-1') {
      const imported = importModelGraphNurbs(parsed), d = history.document
      d.curves!.push(...imported.curves); d.surfaces!.push(...imported.surfaces)
      commit(d); selection.value = imported.surfaces[0]?.id ?? imported.curves[0]?.id ?? ''
    } else {
      commit(parseDirectDocument(text)); selection.value = ''
    }
  }) } }
  catch (e) { error.value = String(e) } finally { input.value = '' }
}
async function importStl(event: Event) {
  const input = event.target as HTMLInputElement, file = input.files?.[0]
  try {
    if (!file) return
    if (file.size > 20_000_000) throw new Error('STL exceeds 20 MB.')
    const mesh = stlBufferToPolygonMesh(await file.arrayBuffer())
    run(() => {
      const d = history.document
      const id = crypto.randomUUID()
      d.bodies.push({ id, name: file.name.replace(/\.stl$/i, '') || 'STL', mesh })
      commit(d)
      selection.value = id
      mode.value = '3d'
    })
  } catch (e) { error.value = e instanceof Error ? e.message : String(e) }
  finally { input.value = '' }
}
function project(p: number[], pane: Pane): Point2 { return pane === '2d' ? [p[0], -p[1]] : projectDirectPoint([p[0],p[1],p[2]??0], camera.value).slice(0,2) as Point2 }
function viewBox(pane: Pane) { const size = views.value[pane], center = centers.value[pane]; return `${center[0] - size / 2} ${center[1] - size / 2} ${size} ${size}` }
function zoom(pane: Pane, factor: number) { views.value[pane] = Math.max(.1, Math.min(2e6, views.value[pane] * factor)) }
function fit(pane: Pane) {
  const points = pane === '2d' ? visibleSketches.value.flatMap(s => s.points) : [
    ...[...document.value.bodies, ...(previewBody.value ? [previewBody.value] : [])].flatMap(bodyPoints),
    ...(document.value.curves ?? []).flatMap(item => item.curve.controlPoints),
    ...(document.value.surfaces ?? []).flatMap(item => item.surface.controlPoints.flat()),
  ]
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
    return { id: b.id, triangle: i, key: b.id + ':' + i, points: face.map(p => project(p, '3d').join(',')).join(' '), shade: directFaceShade(b.mesh,i,camera.value), depth: face.reduce((n,p) => n + projectDirectPoint(p,camera.value)[2], 0) }
  })
}
const polygons = computed(() => document.value.bodies.flatMap(meshPolygons).sort((a,b)=>a.depth-b.depth))
const nurbsSurfacePolygons = computed(() => (document.value.surfaces ?? []).flatMap(item => {
  try { return meshPolygons({ id: item.id, name: item.name, mesh: tessellateSolidNurbsSurface(item) }).sort((a,b)=>a.depth-b.depth) }
  catch { return [] }
}))
const nurbsCurvePaths = computed(() => (document.value.curves ?? []).map(item => ({
  id: item.id,
  points: sampleSolidNurbsCurve(item.curve).map(point => project([point[0], point[1], point[2] ?? 0], '3d').join(',')).join(' '),
})))
const nativeCage = computed(() => {
  if (selectedNurbsCurve.value) return selectedNurbsCurve.value.curve.controlPoints.map((point, u) => ({ point, u, v: 0 }))
  if (selectedNurbsSurface.value) return selectedNurbsSurface.value.surface.controlPoints.flatMap((row, u) => row.map((point, v) => ({ point, u, v })))
  return []
})
const floorLines = computed(() => {
  const step = Math.pow(10, Math.floor(Math.log10(views.value['3d'] / 8))), extent = step * 20
  return Array.from({length:41},(_,i) => (i-20)*step).flatMap(n => [[[-extent,n,0],[extent,n,0]],[[n,-extent,0],[n,extent,0]]]).map(line => line.map(p=>project(p,'3d').join(',')).join(' '))
})
const ghostPolygons = computed(() => previewBody.value ? meshPolygons(previewBody.value).sort((a,b)=>a.depth-b.depth) : [])
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
function snapped(p: Point2, e: PointerEvent): Point2 {
  if (!snap.value || e.altKey) { snapMarker.value = null; return p }
  const candidates = visibleSketches.value.filter(s=>s.id!==gesture?.id).flatMap(s=>s.points)
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
function cancelGesture() { if(cvDrag){document.value=cvDrag.before;cvDrag=null} if(curveDrag){document.value=curveDrag.before;curveDrag=null} if(manipulatorDrag){document.value=manipulatorDrag.before;if(manipulatorDrag.kind==='push')advancedOp.value=null;if(manipulatorDrag.kind==='split')advanced.value.distance=manipulatorDrag.initial;manipulatorDrag=null}selectionBox.value=null; if (gesture) { document.value = gesture.document; gesture = null } if (heightDrag) height.value = heightDrag.height; heightDrag = null; orbitDrag = null; draft.value = []; drawMeasure.value = ''; snapMarker.value = null }
function down(e: PointerEvent, pane: Pane, id = '', vertex: number | null = null, triangle = -1) {
  if (![0, 1, 2].includes(e.button) || gesture) return
  if(e.button===0&&id&&e.shiftKey&&pickMode.value==='body'){pickObject(id,pane,true);return}
  const pan = e.button === 1 || e.shiftKey || (pane === '2d' && e.button === 2)
  const target = e.target as SVGElement
  const svg = (target instanceof SVGSVGElement ? target : target.ownerSVGElement)!
  svg.focus(); mode.value = pane
  if(pane==='3d'&&e.button===0&&!pan&&pickMode.value==='face'&&id&&triangle>=0){
    extraSelection.value=[]
    const candidate=solidTopology(document.value.bodies.find(b=>b.id===id)!.mesh).faces.findIndex(f=>f.triangles.includes(triangle))
    if(selection.value===id&&faceIndex.value===candidate&&!e.ctrlKey&&!e.metaKey){startGizmo(e,'push','z');return}
    if(selection.value!==id)pickObject(id,pane)
    faceIndex.value=solidTopology(document.value.bodies.find(b=>b.id===id)!.mesh).faces.findIndex(f=>f.triangles.includes(triangle));edgeIndex.value=-1
    if(e.ctrlKey||e.metaKey){const set=new Set(openingFaces.value);set.has(faceIndex.value)?set.delete(faceIndex.value):set.add(faceIndex.value);openingFaces.value=[...set]}else openingFaces.value=[faceIndex.value]
    advancedOp.value=null;return
  }
  if(boxSelect.value&&e.button===0&&!pan){const p=position(e);selectionBox.value={start:p,end:p,pane};svg.setPointerCapture(e.pointerId);return}
  if (pane === '3d' && !pan && (e.button === 2 || (!movingBody.value && e.button === 0))) {
    if (id && !operation.value && !advancedOp.value) {if(!selectedIds.value.includes(id))pickObject(id,pane)}
    orbitDrag = { x:e.clientX, y:e.clientY, yaw:camera.value.yaw, pitch:camera.value.pitch, pointer:e.pointerId, svg }; svg.setPointerCapture(e.pointerId); return
  }
  let p: Point2
  try { p = plane(position(e), pane) } catch (e) { error.value = String(e); return }
  if (pane === '2d' && !pan) p = snapped(p,e)
  if (pan) { gesture = { start: position(e), document: history.document, vertex: null, id: '', pointer: e.pointerId, pane, svg, pan: true, center: [...centers.value[pane]] }; svg.setPointerCapture(e.pointerId); return }
  if(pane==='2d'&&tool.value==='trim'){if(id)trimAt(id,p);return}
  if (pane === '2d' && cornerActive.value) { if (id === selection.value && vertex !== null) cornerVertex.value = vertex; return }
  if (pane === '2d' && operation.value) operation.value = null
  if (pane === '2d' && vertex !== null) cornerVertex.value = vertex
  if (pane === '2d' && tool.value === 'polyline') { if (draft.value.length >= 3 && Math.hypot(p[0]-draft.value[0][0],p[1]-draft.value[0][1]) < views.value['2d']/100) { finish(true); return } draft.value = [...draft.value, p]; return }
  if (tool.value === 'select' || pane === '3d') { if(!selectedIds.value.includes(id))pickObject(id,pane); if (!id) return }
  gesture = { start: p, document: history.document, vertex, id, pointer: e.pointerId, pane, svg, pan: false, center: [...centers.value[pane]] }
  svg.setPointerCapture(e.pointerId)
}
function move(e: PointerEvent) {
  if(cvDrag&&cvDrag.pointer===e.pointerId){const g=cvDrag,p=position(e),sx=p[0]-g.start[0],sy=p[1]-g.start[1],cy=Math.cos(camera.value.yaw),sn=Math.sin(camera.value.yaw),sp=Math.sin(camera.value.pitch),cp=Math.cos(camera.value.pitch)
    const horizontal=sy*sp,delta=[sx*cy+horizontal*sn,-sx*sn+horizontal*cy,-sy*cp]
    run(()=>{document.value=updateSolidNurbsControlPoint(g.before,g.id,g.u,g.v,g.point.map((value,i)=>value+delta[i]));const edited=document.value.curves?.find(c=>c.id===g.id)?.curve.controlPoints[g.u]??document.value.surfaces?.find(s=>s.id===g.id)?.surface.controlPoints[g.u]?.[g.v];if(edited)[cvX.value,cvY.value,cvZ.value]=edited as [number,number,number]});return
  }
  if(curveDrag){const g=curveDrag,p=plane(position(e),'2d'),d=JSON.parse(JSON.stringify(g.before)) as DirectDocument,s=d.sketches.find(s=>s.id===g.id)!,a=s.analytic!
    if(g.kind==='center')a.center=p
    else if(g.kind==='radius')a.radius=Math.max(.01,Math.hypot(p[0]-a.center[0],p[1]-a.center[1]))
    else {const angle=Math.atan2(p[1]-a.center[1],p[0]-a.center[0])*180/Math.PI;if(g.kind==='start'){const end=a.start+a.sweep;a.start=angle;a.sweep=((end-angle)%360+360)%360||360}else a.sweep=((angle-a.start)%360+360)%360||360}
    run(()=>{s.points=sampleCurve(a);document.value=d});return
  }
  if(selectionBox.value){selectionBox.value.end=position(e);return}
  if(manipulatorDrag){const g=manipulatorDrag,f=views.value['3d']/Math.min(g.svg.clientWidth,g.svg.clientHeight),x=(e.clientX-g.x)*f,y=(e.clientY-g.y)*f,l=g.direction[0]**2+g.direction[1]**2
    const distance=l>.001?(x*g.direction[0]+y*g.direction[1])/l:-y
    if(g.kind==='push'||g.kind==='split'){advanced.value.distance=Math.round((distance+(g.kind==='split'?g.initial:0))*100)/100;return}
    const p=position(e),rotation=(Math.atan2(p[1]-g.center[1],p[0]-g.center[0])-Math.atan2(g.startPoint[1]-g.center[1],g.startPoint[0]-g.center[0]))*180/Math.PI*(projectDirectPoint(axisVector(g.axis),camera.value)[2]>=0?-1:1)
    run(()=>{document.value=transformSelection(g.before,selectedIds.value,g.kind==='move'?axisVector(g.axis).map(v=>v*distance) as Vec3:[0,0,0],axisVector(g.axis),g.kind==='rotate'?rotation:0,g.kind==='scale'?Math.max(.01,1+distance/(views.value['3d']/7)):1)});return
  }
  if (heightDrag && heightDrag.pointer === e.pointerId) {
    const n=cross3((selectedSketch.value?.plane??xyPlane()).u,(selectedSketch.value?.plane??xyPlane()).v), projected=projectDirectPoint(n,camera.value)
    const factor = -views.value['3d'] / Math.min(heightDrag.svg.clientWidth,heightDrag.svg.clientHeight) / (Math.abs(projected[1])<.05 ? -.05 : projected[1])
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
    draft.value = tool.value === 'rectangle' ? [start, [p[0], start[1]], p, [start[0], p[1]]] : Array.from({ length: tool.value==='arc'?33:64 }, (_, i) => { const r = Math.hypot(p[0] - start[0], p[1] - start[1]), a = i * Math.PI / 32; return [start[0] + r * Math.cos(a), start[1] + r * Math.sin(a)] as Point2 })
    drawMeasure.value = tool.value === 'circle' ? `R ${Math.hypot(p[0]-start[0],p[1]-start[1]).toFixed(2)} mm` : `${Math.abs(p[0]-start[0]).toFixed(2)} × ${Math.abs(p[1]-start[1]).toFixed(2)} mm`
    return
  }
  const d: DirectDocument = JSON.parse(JSON.stringify(gesture.document)), delta = [p[0] - start[0], p[1] - start[1], 0]
  const sketch = d.sketches.find(s => s.id === gesture!.id), body = d.bodies.find(b => b.id === gesture!.id)
  if (sketch) { if (gesture.vertex !== null) {delete sketch.analytic; sketch.points[gesture.vertex] = p} else { const moved=transformSketch(sketch,[delta[0],delta[1]],0,1);Object.assign(sketch,moved) } }
  if(selectedIds.value.length>1){const plane=gesture.pane==='2d'?activePlane.value:xyPlane(),worldDelta=worldPoint(delta,{...plane,origin:[0,0,0]});document.value=transformSelection(gesture.document,selectedIds.value,worldDelta,[0,0,1],0,1);return}
  if (body) body.mesh.positions = transformDirectPoints(bodyPoints(body), delta, 0, 1).flat()
  document.value = d
}
function up(e: PointerEvent) {
  if(cvDrag&&cvDrag.pointer===e.pointerId){move(e);cvDrag=null;run(()=>commit(document.value));return}
  if(curveDrag){move(e);curveDrag=null;run(()=>commit(document.value));return}
  if(selectionBox.value){const box=selectionBox.value,min=[Math.min(box.start[0],box.end[0]),Math.min(box.start[1],box.end[1])],max=[Math.max(box.start[0],box.end[0]),Math.max(box.start[1],box.end[1])]
    const items=box.pane==='2d'?visibleSketches.value.map(s=>({id:s.id,points:s.points.map(p=>project(p,'2d'))})):document.value.bodies.map(b=>({id:b.id,points:bodyPoints(b).map(p=>project(p,'3d'))}))
    const ids=items.filter(o=>o.points.every(p=>p[0]>=min[0]&&p[0]<=max[0]&&p[1]>=min[1]&&p[1]<=max[1])).map(o=>o.id);selection.value=ids[0]??'';extraSelection.value=ids.slice(1);selectionBox.value=null;return
  }
  if(manipulatorDrag){const g=manipulatorDrag;move(e);manipulatorDrag=null;if(g.kind==='push')applyAdvanced();else if(g.kind!=='split')run(()=>commit(document.value));return}

  if (heightDrag) { heightDrag = null; return }
  if (orbitDrag) { orbitDrag = null; return }
  drawMeasure.value = ''; snapMarker.value = null
  if (!gesture || gesture.pointer !== e.pointerId) return
  move(e)
  const start=gesture.start, before = gesture.document, pane = gesture.pane, pan = gesture.pan; gesture = null
  if (pan) return
  run(() => {
    if (pane === '2d' && tool.value !== 'select') {
      const points = draft.value
      if(tool.value==='circle'||tool.value==='arc'){const radius=Math.hypot(points[0][0]-start[0],points[0][1]-start[1]);if(radius>=.01){const analytic={kind:tool.value,center:start,radius,start:0,sweep:tool.value==='circle'?360:180} as const;addSketch(sampleCurve(analytic),tool.value==='circle',analytic)}}
      else if (points.length >= 3 && Math.abs(points.reduce((sum, p, i) => { const q = points[(i + 1) % points.length]; return sum + p[0] * q[1] - q[0] * p[1] }, 0)) > 1e-6) addSketch(points, true)
      draft.value = []; tool.value = 'select'
    } else commit(document.value)
  })
  if (error.value && JSON.stringify(history.document) === JSON.stringify(before)) document.value = before
}
function keydown(e: KeyboardEvent) {
  if (e.key === 'Escape') { e.preventDefault(); cancelGesture(); operation.value = null; advancedOp.value=null; return }
  if(e.key==='Enter'&&advancedOp.value){e.preventDefault();applyAdvanced();return}
  if (e.key === 'Enter' && operation.value) { e.preventDefault(); solidActive.value ? extrude() : cornerActive.value ? applyCorner() : applyCopies(); return }
  if ((e.target as HTMLElement).matches('input,textarea,select')) return
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd') { e.preventDefault(); duplicate(); return }
  if (!e.ctrlKey && !e.metaKey && !e.altKey) {
    const k = e.key.toLowerCase(), tools = { v:'select', r:'rectangle', c:'circle', l:'polyline' } as const
    if (k in tools) { cancelGesture(); operation.value = null; advancedOp.value=null; boxSelect.value=false; tool.value = tools[k as keyof typeof tools]; mode.value = '2d' }
    if (k === 'e') beginExtrude()
    if (k === 'f') fit(mode.value)
    if (k === 'g') movingBody.value = !movingBody.value
    if (k === '?') showHelp.value = !showHelp.value
  }
  if ((e.ctrlKey || e.metaKey) && (e.key.toLowerCase() === 'z' || e.key.toLowerCase() === 'y')) { e.preventDefault(); e.stopPropagation(); undo(e.shiftKey || e.key.toLowerCase() === 'y') }
  if (e.key === 'Delete' || e.key === 'Backspace') { e.preventDefault(); remove() }
  if (e.key === 'Escape' && (gesture || draft.value.length)) { e.preventDefault(); e.stopPropagation(); cancelGesture() }
}

if (props.initialDocument) void nextTick(() => { fit('2d'); fit('3d') })
const primitiveKinds = ['box','cylinder','cone','sphere'] as const
const primitiveSize = ref(20)
function primitiveLabel(kind: typeof primitiveKinds[number]) { return ({box:label('Куб','Box'),cylinder:label('Цилиндр','Cylinder'),cone:label('Конус','Cone'),sphere:label('Сфера','Sphere')})[kind] }
function addPrimitive(kind: typeof primitiveKinds[number]) { run(() => {
 const size=primitiveSize.value
 if(!Number.isFinite(size)||size<0.1||size>10000) throw new Error(label('Размер от 0.1 до 10000 мм','Size must be 0.1–10000 mm'))
 const id=crypto.randomUUID(), name=primitiveLabel(kind), r=size/2
 let body
 if(kind==='box'||kind==='cylinder') {
  const points:Point2[]=kind==='box'?[[-r,-r],[r,-r],[r,r],[-r,r]]:Array.from({length:48},(_,i)=>[r*Math.cos(i*Math.PI/24),r*Math.sin(i*Math.PI/24)] as Point2)
  body=extrudeDirectSketch({id:crypto.randomUUID(),name,points,closed:true},size,id)
 } else {
  const profile=kind==='cone'?[[0,0],[r,0],[0,size]]:Array.from({length:25},(_,i)=>i===0?[0,-r]:i===24?[0,r]:[r*Math.sin(i*Math.PI/24),-r*Math.cos(i*Math.PI/24)])
  const mesh=revolvePolygonProfile(profile,360,48,true)
  body={id,name,mesh:{positions:[...mesh.positions],indices:[...mesh.indices]}}
 }
 const d=history.document;d.bodies.push(body);commit(d);pickObject(id,'3d');fit('3d')
}) }
function addNurbs(kind: 'curve'|'surface') { run(() => {
  const d = history.document, item = kind === 'curve' ? createSolidNurbsCurve() : createSolidNurbsSurface()
  if (kind === 'curve') d.curves!.push(item as ReturnType<typeof createSolidNurbsCurve>)
  else d.surfaces!.push(item as ReturnType<typeof createSolidNurbsSurface>)
  commit(d); pickObject(item.id, '3d'); fit('3d')
}) }
function updateCv() { run(() => {
  const point = [cvX.value, cvY.value, cvZ.value]
  if (!point.every(Number.isFinite)) throw new Error('CV coordinates must be finite.')
  if (!Number.isFinite(cvWeight.value) || cvWeight.value <= 0) throw new Error('CV weight must be positive.')
  commit(updateSolidNurbsControlPoint(history.document,selection.value,cvU.value,cvV.value,point,cvWeight.value))
}) }
function insertNativeKnot(axis: 'curve'|'u'|'v') { run(() => {
  const d = history.document, curve = d.curves!.find(item => item.id === selection.value), surface = d.surfaces!.find(item => item.id === selection.value)
  if (axis === 'curve' && curve) curve.curve = insertNurbsKnot(curve.curve, knotValue.value)
  else if (surface && axis !== 'curve') surface.surface = insertNurbsSurfaceKnot(surface.surface, axis, knotValue.value)
  else return
  commit(d)
}) }
function elevateNative(axis: 'curve'|'u'|'v') { run(() => {
  const d = history.document, curve = d.curves!.find(item => item.id === selection.value), surface = d.surfaces!.find(item => item.id === selection.value)
  if (axis === 'curve' && curve) curve.curve = elevateNurbsCurve(curve.curve, curve.curve.degree + 1)
  else if (surface && axis === 'u') surface.surface = elevateNurbsSurface(surface.surface, 'u', surface.surface.degreeU + 1)
  else if (surface && axis === 'v') surface.surface = elevateNurbsSurface(surface.surface, 'v', surface.surface.degreeV + 1)
  else return
  commit(d)
}) }
function curveToSurface() { run(() => {
  if (!selectedNurbsCurve.value) return
  const d = history.document, source = d.curves!.find(item => item.id === selection.value)!
  const item = { id: crypto.randomUUID(), name: `${source.name} · extrude`, surface: extrudeNurbsCurve(source.curve, [0,0,10]), segmentsU: 16, segmentsV: 8 }
  d.surfaces!.push(item); commit(d); pickObject(item.id, '3d'); fit('3d')
}) }
function extractIso(axis: 'u'|'v') { run(() => {
  if (!selectedNurbsSurface.value) return
  const d = history.document, source = d.surfaces!.find(item => item.id === selection.value)!
  const item = { id: crypto.randomUUID(), name: `${source.name} · iso ${axis.toUpperCase()}`, curve: isoNurbsCurve(source.surface, axis, knotValue.value) }
  d.curves!.push(item); commit(d); pickObject(item.id, '3d')
}) }
function trimNativeSurface() { run(() => {
  if(!selectedNurbsSurface.value)return
  const d=history.document,source=d.surfaces!.find(item=>item.id===selection.value)!
  source.surface=trimNurbsSurface(source.surface,trimBounds.value)
  cvU.value=cvV.value=0;commit(d)
}) }
function bakeNurbs() { run(() => {
  const d = history.document
  if (selectedNurbsSurface.value) {
    const source = d.surfaces!.find(item => item.id === selection.value)!
    const id = crypto.randomUUID()
    d.bodies.push({ id, name: `${source.name} · baked mesh`, mesh: tessellateSolidNurbsSurface(source) })
    commit(d); pickObject(id, '3d')
  } else if (selectedNurbsCurve.value) {
    const source = d.curves!.find(item => item.id === selection.value)!
    const sketch = nurbsCurveToSketch(source)
    d.sketches.push(sketch); commit(d); pickObject(sketch.id, '2d'); fit('2d')
  }
}) }
</script>
<template>
  <section v-show="open" ref="workspace" class="direct-workspace" :class="{ embedded }" tabindex="-1" :aria-label="label('Solid — CAD-лепка', 'Solid — CAD sculpt')" @keydown.stop="keydown">
    <header class="workspace-bar">
      <button class="back" @click="emit('close')">← {{ label('Code', 'Code') }}</button>
      <strong>{{ label('Solid', 'Solid') }}</strong>
      <span class="subtle">{{ label('Plasticity-like CAD', 'Plasticity-like CAD') }}</span>
      <div class="history-tools"><button :disabled="!undoable" @click="undo()" :title="label('Отменить · Ctrl/⌘ Z', 'Undo · Ctrl/⌘ Z')">↶</button><button :disabled="!redoable" @click="undo(true)" :title="label('Повторить · Ctrl/⌘ Shift Z', 'Redo · Ctrl/⌘ Shift Z')">↷</button></div>
      <button @click="showHelp = !showHelp" title="Keyboard shortcuts">?</button>
      <span class="save-status" role="status">{{ saveError ? label('Не сохранено', 'Unsaved') : label('Сохранено в браузере', 'Saved in browser') }}</span>
      <details class="file-menu"><summary>{{ label('Файл', 'File') }} ▾</summary><div>
        <button @click="download(JSON.stringify(document), 'solid-model.json')">{{ label('Скачать проект JSON', 'Download JSON project') }}</button>
        <label class="file-open">{{ label('Открыть Solid / ModelGraph NURBS', 'Open Solid / ModelGraph NURBS') }}<input type="file" accept=".json,application/json" @change="importFile"></label>
        <button type="button" @click="stlInput?.click()">{{ label('Импорт STL как тело', 'Import STL as body') }}</button>
        <input ref="stlInput" type="file" accept=".stl,model/stl" hidden @change="importStl" />
        <button :disabled="!document.bodies.length" @click="download(directBodiesScad(document), 'solid-bodies.scad')">{{ label('Экспорт SCAD (bake)', 'Export SCAD (bake)') }}</button>
        <button :disabled="!document.bodies.length || !canAppend" @click="appendBodies">{{ embedded ? label('Bake в код', 'Bake into code') : label('Bake в Code (append)', 'Bake into Code (append)') }}</button>
        <button :disabled="!document.bodies.length" @click="sendToMesh">{{ label('Открыть в Mesh', 'Open in Mesh') }}</button>
      </div></details>
    </header>
    <div class="primitive-bar">
      <strong>{{ label('Примитивы','Primitives') }}</strong>
      <button v-for="kind in primitiveKinds" :key="kind" @click="addPrimitive(kind)">{{ primitiveLabel(kind) }}</button>
      <button @click="addNurbs('curve')">+ {{ label('NURBS-кривая','NURBS curve') }}</button>
      <button @click="addNurbs('surface')">+ {{ label('NURBS-поверхность','NURBS surface') }}</button>
      <label>{{ label('Размер, мм','Size, mm') }} <input v-model.number="primitiveSize" type="number" min="0.1" max="10000" /></label>
      <button type="button" @click="stlInput?.click()">STL</button>
      <button v-if="embedded" :disabled="!canAppend" @click="appendBodies">{{ label('Применить в код (bake)','Apply to code') }}</button>
      <span class="subtle">{{ label('Solid хранит свой документ; bake в .scad — только по кнопке.','Solid keeps its own document; bake to .scad is explicit.') }}</span>
    </div>
    <div ref="splitArea" class="split-workspace" :style="{ '--split': split + '%' }">
      <template v-for="pane in panes" :key="pane">
        <div v-if="pane === '3d'" class="splitter" role="separator" tabindex="0" aria-orientation="vertical" :aria-label="label('Ширина 2D и 3D', '2D and 3D width')" :aria-valuenow="Math.round(split)" :aria-valuemin="25" :aria-valuemax="75" @pointerdown="resizeSplit" @pointermove="moveSplit" @pointerup="($event.currentTarget as HTMLElement).releasePointerCapture($event.pointerId)" @keydown="splitKey" @dblclick="split = 50"><span /></div>
        <section class="pane" :class="{ active: mode === pane }" :aria-label="pane === '2d' ? label('2D — эскизы', '2D — sketches') : label('3D — тела', '3D — bodies')">
          <header class="pane-heading"><strong>{{ pane === '2d' ? label('2D · Эскизы', '2D · Sketches') : label('3D · Тела', '3D · Bodies') }}</strong><span>{{ pane === '2d' ? (samePlane(activePlane,xyPlane())?label('Плоскость XY · мм', 'XY plane · mm'):label('Плоскость грани · мм','Face plane · mm')) : label('Орбита · мм', 'Orbit · mm') }}</span><button @click="fit(pane)">{{ label('Вписать', 'Fit') }}</button></header>
          <div class="pane-tools">
            <template v-if="pane === '2d'">
              <button v-for="(name, value) in { select: label('↖ Выбор · V', '↖ Select · V'), rectangle: label('□ Прямоугольник · R', '□ Rectangle · R'), circle: label('○ Круг · C', '○ Circle · C'), arc: label('◜ Дуга', '◜ Arc'), trim: label('✂ Обрезать','✂ Trim'), polyline: label('⌁ Ломаная · L', '⌁ Polyline · L') }" :key="value" :aria-pressed="tool === value" @click="cancelGesture(); operation = null; advancedOp=null; boxSelect=false; tool = value; mode = '2d'">{{ name }}</button>
              <button :aria-pressed="boxSelect" @click="boxSelect=!boxSelect">{{ label('Рамка','Box select') }}</button>
              <button @click="activePlane=xyPlane();workplaneOutline=[];selection='';extraSelection=[]">XY</button>
              <label class="snap-toggle"><input v-model="snap" type="checkbox">{{ label('Привязка', 'Snap') }}</label><input v-if="snap" class="grid-input" v-model.number="grid" type="number" min=".01" step=".5" :aria-label="label('Шаг сетки', 'Grid step')">
            </template>
            <template v-else>
              <button v-for="(name,value) in {body:label('Тела','Bodies'),face:label('Грани','Faces'),edge:label('Рёбра','Edges')}" :key="value" :aria-pressed="pickMode===value" @click="pickMode=value;advancedOp=null;boxSelect=false">{{ name }}</button>
              <button :aria-pressed="boxSelect" @click="boxSelect=!boxSelect">{{ label('Рамка','Box select') }}</button>
              <select v-model="gizmoMode" :aria-label="label('Манипулятор','Manipulator')"><option value="move">{{ label('Двигать','Move') }}</option><option value="rotate">{{ label('Вращать','Rotate') }}</option><option value="scale">{{ label('Масштаб','Scale') }}</option></select>
              <button :aria-pressed="!movingBody" @click="movingBody = false">{{ label('↻ Обзор', '↻ Orbit') }}</button><button :aria-pressed="movingBody" @click="movingBody = true">{{ label('↔ Двигать · G', '↔ Move · G') }}</button><button @click="camera = defaultDirectCamera(); fit('3d')">ISO</button><button :aria-pressed="floorVisible" @click="floorVisible = !floorVisible" :aria-label="label('Сетка 3D', '3D grid')">#</button><span class="subtle">{{ pickMode==='face'?label('Ctrl/⌘ + клик — несколько открытых граней','Ctrl/⌘ click — multiple openings'):label('ПКМ — вращать · Shift — панорама', 'Right drag to orbit · Shift to pan') }}</span><button :disabled="!selectedBody" @click="run(() => download(exportPolygonStl(selectedBody!.mesh), 'body.stl'))">↓ STL</button></template>
          </div>
          <div class="canvas-wrap">
            <svg :viewBox="viewBox(pane)" tabindex="0" :aria-label="pane === '2d' ? label('Холст эскизов 2D', '2D sketch canvas') : label('Холст тел 3D', '3D body canvas')" @contextmenu.prevent @wheel.prevent="zoom(pane, $event.deltaY > 0 ? 1.1 : 1/1.1)" @pointerdown="down($event, pane)" @pointermove="move" @pointerup="up" @pointercancel="cancelGesture" @lostpointercapture="cancelGesture">
              <defs><pattern :id="'direct-grid-' + pane" width="10" height="10" patternUnits="userSpaceOnUse"><path d="M 10 0 L 0 0 0 10" fill="none" stroke="var(--border)" stroke-opacity=".45" stroke-width=".5" vector-effect="non-scaling-stroke" /></pattern></defs>
              <rect v-if="pane === '2d'" x="-2000000" y="-2000000" width="4000000" height="4000000" :fill="'url(#direct-grid-' + pane + ')'" />
              <g v-if="pane === '3d' && floorVisible" pointer-events="none"><polyline v-for="(line,i) in floorLines" :key="i" :points="line" fill="none" stroke="var(--border)" stroke-opacity=".45" stroke-width=".5" vector-effect="non-scaling-stroke" /></g>
              <path v-if="pane === '2d'" d="M -2000000 0 H 2000000 M 0 -2000000 V 2000000" stroke="var(--border)" vector-effect="non-scaling-stroke" />
              <g v-if="pane === '2d'">
                <path v-for="(loop,i) in workplaneOutline" :key="'workplane-'+i" :d="'M '+loop.map(p=>project(p,'2d').join(',')).join(' L ')+' Z'" fill="var(--border)" fill-opacity=".2" stroke="var(--text-dim)" stroke-dasharray="3 3" vector-effect="non-scaling-stroke" pointer-events="none" />
                <g v-for="s in visibleSketches" :key="s.id">
                  <path :d="'M ' + s.points.map(p => project(p, '2d').join(',')).join(' L ') + (s.closed ? ' Z' : '')" :class="{ selected: selectedIds.includes(s.id), hovered: hovered === s.id }" @pointerenter="hovered = s.id" @pointerleave="hovered = ''" :fill="selectedIds.includes(s.id) && s.closed ? 'var(--accent)' : 'none'" fill-opacity=".08" pointer-events="all" stroke="var(--accent)" stroke-width="2" vector-effect="non-scaling-stroke" @pointerdown.stop="down($event, pane, s.id)" />
                  <template v-if="selection === s.id && tool === 'select' && !s.analytic"><circle v-for="(p, i) in s.points" :key="i" :cx="p[0]" :cy="-p[1]" :r="views['2d'] / 150" fill="var(--accent)" @pointerdown.stop="down($event, pane, s.id, i)" /></template>
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
                <path v-for="s in copyPreview" :key="s.id" :d="'M '+s.points.map(p=>project(p,'2d').join(',')).join(' L ')+(s.closed?' Z':'')" fill="none" stroke="#b894ff" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" pointer-events="none" />
                <circle v-if="snapMarker" :cx="snapMarker[0]" :cy="-snapMarker[1]" :r="views['2d']/100" fill="none" stroke="#77eac5" vector-effect="non-scaling-stroke" pointer-events="none" />
                <polyline v-if="draft.length"
 :points="draft.map(p => project(p, '2d').join(',')).join(' ')" fill="none" stroke="var(--accent)" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" pointer-events="none" />
              </g>
              <g v-else>
                <polygon v-for="p in polygons" v-show="!advancedPreview.document || !selectedIds.includes(p.id)" :key="p.key" :points="p.points" :fill="`hsl(${selectedBody?.id===p.id && selectedFaceTriangles.has(p.triangle) && pickMode==='face' ? 40 : selectedIds.includes(p.id) ? 266 : hovered === p.id ? 190 : 220} 45% ${p.shade}%)`" :stroke="`hsl(${selectedIds.includes(p.id) ? 266 : 220} 45% ${p.shade}%)`" stroke-width=".6" @pointerenter="hovered = p.id" @pointerleave="hovered = ''" vector-effect="non-scaling-stroke" @pointerdown.stop="down($event, pane, p.id, null, p.triangle)" />
                <polygon v-for="p in nurbsSurfacePolygons" :key="'surface-'+p.key" :points="p.points" :fill="selectedIds.includes(p.id)?'#8061bd':'#315f72'" fill-opacity=".72" stroke="#77eac5" stroke-opacity=".35" stroke-width=".5" vector-effect="non-scaling-stroke" @pointerdown.stop="pickObject(p.id,'3d')" />
                <polyline v-for="curve in nurbsCurvePaths" :key="'curve-'+curve.id" :points="curve.points" fill="none" :stroke="selectedIds.includes(curve.id)?'#ffc977':'#77eac5'" stroke-width="3" vector-effect="non-scaling-stroke" @pointerdown.stop="pickObject(curve.id,'3d')" />
                <g v-if="selectedNurbs" class="nurbs-cage">
                  <polyline v-if="selectedNurbsCurve" :points="selectedNurbsCurve.curve.controlPoints.map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#ffc977" stroke-dasharray="3 3" vector-effect="non-scaling-stroke" pointer-events="none" />
                  <template v-if="selectedNurbsSurface">
                    <polyline v-for="(_,u) in selectedNurbsSurface.surface.controlPoints" :key="'u-'+u" :points="selectedNurbsSurface.surface.controlPoints[u].map(p=>project(p,'3d').join(',')).join(' ')" fill="none" stroke="#ffc977" stroke-dasharray="3 3" vector-effect="non-scaling-stroke" pointer-events="none" />
                    <polyline v-for="(_,v) in selectedNurbsSurface.surface.controlPoints[0]" :key="'v-'+v" :points="selectedNurbsSurface.surface.controlPoints.map(row=>project(row[v],'3d').join(',')).join(' ')" fill="none" stroke="#ffc977" stroke-dasharray="3 3" vector-effect="non-scaling-stroke" pointer-events="none" />
                  </template>
                  <circle v-for="cv in nativeCage" :key="cv.u+'-'+cv.v" :cx="project(cv.point,'3d')[0]" :cy="project(cv.point,'3d')[1]" :r="views['3d']/110" :fill="cvU===cv.u&&cvV===cv.v?'#ff8b77':'#ffc977'" stroke="#2a2114" vector-effect="non-scaling-stroke" @pointerdown.stop="startCv($event,cv.u,cv.v)" />
                </g>
              </g>
              <g v-if="pane === '3d' && solidActive" pointer-events="none"><polygon v-for="p in ghostPolygons" :key="p.key" :points="p.points" :fill="extrusionMode === 'difference' ? '#ff647c' : '#75e4b8'" fill-opacity=".28" :stroke="extrusionMode === 'difference' ? '#ff647c' : '#75e4b8'" stroke-width=".7" vector-effect="non-scaling-stroke" /></g>
              <g v-if="pane === '3d' && extrusionHandle" class="height-handle" @pointerdown.stop="dragHeight">
                <line :x1="extrusionHandle.base[0]" :y1="extrusionHandle.base[1]" :x2="extrusionHandle.top[0]" :y2="extrusionHandle.top[1]" stroke="#77eac5" stroke-width="3" vector-effect="non-scaling-stroke" />
                <circle :cx="extrusionHandle.top[0]" :cy="extrusionHandle.top[1]" :r="views['3d']/55" fill="#77eac5" stroke="#18332d" stroke-width="2" vector-effect="non-scaling-stroke" />
                <text :x="extrusionHandle.top[0]+views['3d']/35" :y="extrusionHandle.top[1]" :font-size="views['3d']/45" fill="#77eac5">{{ height.toFixed(2) }} mm</text>
              </g>
              <g v-if="pane==='3d'" pointer-events="none">
                <path v-for="s in document.sketches" :key="'plane-'+s.id" :d="'M '+s.points.map(p=>project(worldPoint(p,s.plane),'3d').join(',')).join(' L ')+(s.closed?' Z':'')" fill="none" stroke="#77eac5" stroke-opacity=".6" vector-effect="non-scaling-stroke" />
                <polygon v-for="p in advancedPolygons" :key="'advanced-'+p.key" :points="p.points" :fill="p.id==='preview-split'?'#ffc977':'#77eac5'" fill-opacity=".45" stroke="#77eac5" stroke-width=".5" vector-effect="non-scaling-stroke" />
                <polygon v-if="splitPlanePoints" :points="splitPlanePoints" fill="#ffc977" fill-opacity=".12" stroke="#ffc977" stroke-dasharray="5 3" vector-effect="non-scaling-stroke" />
              </g>
              <circle v-if="pane==='3d' && advancedOp==='split' && gizmoCenter" :cx="project(gizmoCenter.map((x,i)=>axisVector(advanced.axis)[i]?advanced.distance:x),'3d')[0]" :cy="project(gizmoCenter.map((x,i)=>axisVector(advanced.axis)[i]?advanced.distance:x),'3d')[1]" :r="views['3d']/65" fill="#ffc977" style="cursor:grab" @pointerdown.stop="startGizmo($event,'split',advanced.axis)" />
              <g v-if="pane==='3d' && pickMode==='edge'">
                <polyline v-for="edge in featureEdges" :key="edge.i" :points="edge.points" fill="none" :stroke="edgeIndex===edge.i?'#ffc977':'#89baff'" :stroke-width="edgeIndex===edge.i?5:3" vector-effect="non-scaling-stroke" @pointerdown.stop="pickEdge($event,edge.i)" />
              </g>
              <g v-if="pane==='3d' && !advancedOp && pickMode==='body'">
                <g v-for="axis in gizmoAxes" :key="axis.axis">
                  <polyline v-if="gizmoMode==='rotate'" :points="axis.ring" fill="none" :stroke="axis.color" stroke-width="3" vector-effect="non-scaling-stroke" style="cursor:grab" @pointerdown.stop="startGizmo($event,'rotate',axis.axis)" />
                  <g v-else style="cursor:grab" @pointerdown.stop="startGizmo($event,gizmoMode,axis.axis)">
                    <line :x1="axis.base[0]" :y1="axis.base[1]" :x2="axis.tip[0]" :y2="axis.tip[1]" :stroke="axis.color" stroke-width="3" vector-effect="non-scaling-stroke" />
                    <circle :cx="axis.tip[0]" :cy="axis.tip[1]" :r="views['3d']/95" :fill="axis.color" />
                    <text :x="axis.tip[0]" :y="axis.tip[1]-views['3d']/65" :font-size="views['3d']/55" :fill="axis.color">{{ axis.axis.toUpperCase() }}</text>
                  </g>
                </g>
              </g>
              <g v-if="pane==='3d' && selectedFace && pickMode==='face' && !advancedOp" style="cursor:ns-resize" @pointerdown.stop="startGizmo($event,'push','z')">
                <circle :cx="project(selectedFace.center,'3d')[0]" :cy="project(selectedFace.center,'3d')[1]" :r="views['3d']/70" fill="#ffc977" />
              </g>
              <g v-if="pane==='2d'" pointer-events="none"><path v-for="s in advancedSketches" :key="'adv-'+s.id" :d="'M '+s.points.map(p=>project(p,'2d').join(',')).join(' L ')+(s.closed?' Z':'')" fill="none" stroke="#77eac5" stroke-width="3" vector-effect="non-scaling-stroke" /></g>
              <rect v-if="selectionBox?.pane===pane" :x="Math.min(selectionBox.start[0],selectionBox.end[0])" :y="Math.min(selectionBox.start[1],selectionBox.end[1])" :width="Math.abs(selectionBox.end[0]-selectionBox.start[0])" :height="Math.abs(selectionBox.end[1]-selectionBox.start[1])" fill="#77baff" fill-opacity=".12" stroke="#77baff" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" pointer-events="none" />

            </svg>
            <div v-if="advancedOp && pane === (['offset','extend','curve'].includes(advancedOp) ? '2d' : '3d')" class="operation-card">
              <strong>{{ ({push:label('Сдвиг грани','Push / Pull'),chamfer:label('Фаска ребра','Edge chamfer'),'edge-fillet':label('Скругление ребра','Edge fillet'),shell:label('Полое тело','Shell'),split:label('Разрез плоскостью','Plane split'),offset:label('Отступ контура','Offset'),extend:label('Продлить линию','Extend'),curve:label('Окружность / дуга','Circle / arc'),transform:label('Преобразовать выбор','Transform selection')})[advancedOp] }}</strong>
              <small v-if="['push','chamfer','edge-fillet','shell'].includes(advancedOp)">{{ label('Для выпуклых тел с плоскими гранями.', 'For convex solids with planar faces.') }}</small>
              <label v-if="['push','shell','split','offset'].includes(advancedOp)">{{ advancedOp==='shell'?label('Толщина, мм','Thickness, mm'):label('Расстояние, мм','Distance, mm') }}<input v-model.number="advanced.distance" type="number" step=".5"></label>
              <label v-if="['edge-fillet','chamfer','curve'].includes(advancedOp)">{{ label('Радиус / размер, мм','Radius / size, mm') }}<input v-model.number="advanced.radius" type="number" min=".01" step=".5"></label>
              <label v-if="advancedOp==='split'||advancedOp==='transform'">{{ label('Ось','Axis') }}<select v-model="advanced.axis"><option>x</option><option>y</option><option>z</option></select></label>
              <small v-if="advancedOp==='split'">{{ label('Обе части сохраняются отдельными телами. Пунктир — плоскость разреза.', 'Both halves remain separate bodies. The dashed outline is the cutting plane.') }}</small>
              <small v-if="advancedOp==='shell'">{{ label('Открытые грани:','Open faces:') }} {{ openingFaces.map(i=>i+1).join(', ') }}</small>
              <template v-if="advancedOp==='curve'">
                <label>{{ label('Центр X','Center X') }}<input v-model.number="advanced.cx" type="number"></label><label>{{ label('Центр Y','Center Y') }}<input v-model.number="advanced.cy" type="number"></label>
                <label>{{ label('Начальный угол','Start angle') }}<input v-model.number="advanced.start" type="number"></label>
                <label v-if="selectedSketch?.analytic?.kind==='arc'">{{ label('Угол дуги','Arc sweep') }}<input v-model.number="advanced.sweep" type="number" min="-360" max="360"></label>
              </template>
              <label v-if="advancedOp==='extend'">{{ label('Конец','Endpoint') }}<select v-model="advanced.end"><option value="start">{{ label('Начало','Start') }}</option><option value="end">{{ label('Конец','End') }}</option></select></label>
              <template v-if="advancedOp==='transform'">
                <label>X<input v-model.number="advanced.x" type="number"></label><label>Y<input v-model.number="advanced.y" type="number"></label><label>Z<input v-model.number="advanced.z" type="number"></label>
                <label>{{ label('Поворот, °','Rotation, °') }}<input v-model.number="advanced.angle" type="number"></label><label>{{ label('Масштаб','Scale') }}<input v-model.number="advanced.scale" type="number" min=".01" step=".1"></label>
              </template>
              <small v-if="advancedPreview.error" role="alert">{{ advancedPreview.error }}</small>
              <div><button class="primary" :disabled="!advancedPreview.document" @click="applyAdvanced">{{ label('Готово · Enter','Apply · Enter') }}</button><button @click="advancedOp=null">Esc</button></div>
            </div>

            <div v-if="pane==='3d' && selectedNurbs" class="operation-card nurbs-card">
              <strong>{{ selectedNurbsCurve ? label('NURBS-кривая · CV','NURBS curve · CV') : label('NURBS-поверхность · CV','NURBS surface · CV') }}</strong>
              <small>{{ label('Тяните жёлтые CV прямо в 3D-виде. Перетаскивание идёт в плоскости экрана; точные XYZ и вес — ниже.','Drag yellow CVs directly in the 3D view. Dragging follows the screen plane; exact XYZ and weight are below.') }}</small>
              <label>U / CV <select v-model.number="cvU"><option v-for="(_,i) in (selectedNurbsCurve?.curve.controlPoints ?? selectedNurbsSurface?.surface.controlPoints ?? [])" :key="i" :value="i">{{ i }}</option></select></label>
              <label v-if="selectedNurbsSurface">V / CV <select v-model.number="cvV"><option v-for="(_,i) in selectedNurbsSurface.surface.controlPoints[cvU] ?? []" :key="i" :value="i">{{ i }}</option></select></label>
              <label>X <input v-model.number="cvX" type="number" step=".5"></label>
              <label>Y <input v-model.number="cvY" type="number" step=".5"></label>
              <label>Z <input v-model.number="cvZ" type="number" step=".5"></label>
              <label>{{ label('Вес','Weight') }} <input v-model.number="cvWeight" type="number" min=".000001" step=".1"></label>
              <button class="primary" @click="updateCv">{{ label('Применить CV','Apply CV') }}</button>
              <label>{{ label('Параметр узла / iso','Knot / iso parameter') }} <input v-model.number="knotValue" type="number" step=".1"></label>
              <div v-if="selectedNurbsCurve"><button @click="insertNativeKnot('curve')">Insert knot</button><button @click="elevateNative('curve')">Degree +1</button></div>
              <template v-if="selectedNurbsSurface">
                <div><button @click="insertNativeKnot('u')">Knot U</button><button @click="insertNativeKnot('v')">Knot V</button></div>
                <div><button @click="elevateNative('u')">Degree U +1</button><button @click="elevateNative('v')">Degree V +1</button></div>
                <div><button @click="extractIso('u')">Iso U</button><button @click="extractIso('v')">Iso V</button></div>
                <small>{{ label('Прямоугольная обрезка сохраняет точную NURBS-поверхность в новом диапазоне параметров.','Rectangular parameter trim keeps an exact NURBS surface over the new domain.') }}</small>
                <div class="trim-grid"><label>U min<input v-model.number="trimBounds[0]" type="number" step=".05"></label><label>U max<input v-model.number="trimBounds[1]" type="number" step=".05"></label><label>V min<input v-model.number="trimBounds[2]" type="number" step=".05"></label><label>V max<input v-model.number="trimBounds[3]" type="number" step=".05"></label></div>
                <button @click="trimNativeSurface">{{ label('Обрезать диапазон UV','Trim UV domain') }}</button>
                <label>U segments <input v-model.number="selectedNurbsSurface.segmentsU" type="number" min="2" max="64" @change="commit(document)"></label>
                <label>V segments <input v-model.number="selectedNurbsSurface.segmentsV" type="number" min="2" max="64" @change="commit(document)"></label>
              </template>
              <button v-if="selectedNurbsCurve" @click="curveToSurface">{{ label('Выдавить NURBS 10 мм','Extrude NURBS 10 mm') }}</button>
              <button @click="bakeNurbs">{{ selectedNurbsSurface ? label('Bake поверхности в mesh-тело','Bake surface to mesh body') : label('Копировать sampled-кривую в эскиз','Copy sampled curve to sketch') }}</button>
            </div>

            <div v-if="pane === '2d' && cornerActive" class="operation-card">
              <strong>{{ operation === 'fillet' ? label('Скругление', 'Fillet') : 'DogEar' }}</strong>
              <small>{{ label('Нажмите вершину контура для выбора угла.', 'Click a contour vertex to choose a corner.') }}</small>
              <small v-if="operation === 'dogear'">{{ label('Круглый выход в прямом углу. Радиус соответствует радиусу инструмента.', 'Circular relief at a right-angle corner. Radius is the tool radius.') }}</small>
              <label>{{ label('Вершина', 'Vertex') }}<select v-model.number="cornerVertex" :aria-label="label('Вершина', 'Vertex')"><option v-for="(_, i) in selectedSketch?.points" :key="i" :value="i">{{ i + 1 }}</option></select></label>
              <label>{{ label('Радиус, мм', 'Radius, mm') }}<input v-model.number="cornerRadius" type="number" min=".01" step=".5"></label>
              <small v-if="cornerPreview.error" role="alert">{{ cornerPreview.error }}</small>
              <div><button class="primary" :disabled="!cornerPreview.sketch" @click="applyCorner">{{ label('Готово · Enter', 'Apply · Enter') }}</button><button @click="operation = null">Esc</button></div>
            </div>
            <div v-if="pane === '2d' && drawMeasure" class="live-measure">{{ drawMeasure }}</div>
            <div v-if="pane === '3d' && solidActive" class="operation-card">
              <strong>{{ operation === 'revolve' ? label('Вращение профиля', 'Revolve profile') : label('Выдавливание', 'Extrusion') }}</strong><small v-if="operation === 'extrude'">{{ label('Тяните зелёную ручку или введите размер', 'Drag the green handle or enter a dimension') }}</small>
              <div class="segmented"><button v-for="(name,value) in {new:label('Новое','New'),union:label('Добавить','Add'),difference:label('Вычесть','Cut')}" :key="value" :aria-pressed="extrusionMode === value" @click="extrusionMode = value">{{ name }}</button></div>
              <label v-if="operation === 'extrude'">{{ label('Высота, мм', 'Height, mm') }}<input v-model.number="height" type="number" step="1"></label>
              <label v-if="operation === 'extrude'">{{ label('Начало Z, мм', 'Start Z, mm') }}<input v-model.number="baseZ" type="number" step="1"></label>
              <template v-if="operation === 'revolve'">
                <label>{{ label('Ось в эскизе', 'Sketch axis') }}<select v-model="revolveAxis"><option value="y">Y · {{ label('вертикаль', 'vertical') }}</option><option value="x">X · {{ label('горизонталь', 'horizontal') }}</option></select></label>
                <label>{{ label('Смещение оси, мм', 'Axis offset, mm') }}<input v-model.number="revolveOffset" type="number" step="1"></label>
                <label>{{ label('Угол, °', 'Angle, °') }}<input v-model.number="revolveAngle" type="number" min="-360" max="360" step="15"></label>
                <label>{{ label('Сегменты', 'Segments') }}<input v-model.number="revolveSegments" type="number" min="8" max="128"></label>
                <small>{{ label('Пунктир слева — ось вращения. Контур должен лежать по одну сторону от неё.', 'The dashed line on the left is the rotation axis. Keep the profile on one side of it.') }}</small>
              </template>
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
            <div v-if="pane === '3d' && !document.bodies.length && !solidActive" class="empty-hint"><strong>{{ label('Здесь появится объём', 'Your solid appears here') }}</strong><span>{{ label('Выберите эскиз слева и нажмите «Выдавить»', 'Select a sketch on the left and press Extrude') }}</span></div>
            <div class="zoom-tools"><button :aria-label="label('Приблизить ', 'Zoom in ') + pane" @click="zoom(pane, .8)">+</button><button :aria-label="label('Отдалить ', 'Zoom out ') + pane" @click="zoom(pane, 1.25)">−</button></div>
          </div>
          <div class="object-strip"><span>{{ pane === '2d' ? label('Эскизы', 'Sketches') : label('Тела · NURBS', 'Bodies · NURBS') }}</span><button v-for="item in (pane === '2d' ? document.sketches : [...document.bodies,...(document.curves ?? []),...(document.surfaces ?? [])])" :key="item.id" :aria-pressed="selectedIds.includes(item.id)" @click="pickObject(item.id,pane,$event.shiftKey)">{{ item.name }}</button></div>
        </section>
      </template>
    </div>
    <footer class="context-bar">
      <template v-if="tool === 'polyline' && draft.length"><span>{{ draft.length }} {{ label('точек', 'points') }}</span><button :disabled="draft.length < 3" @click="finish(true)">{{ label('Замкнуть контур', 'Close contour') }}</button><button :disabled="draft.length < 2" @click="finish(false)">{{ label('Завершить линию', 'Finish line') }}</button><button @click="cancelGesture">Esc</button></template>
      <template v-else-if="selectedSketch || selectedBody || selectedNurbs">
        <strong>{{ selectedIds.length>1 ? label('Выбрано: ','Selected: ')+selectedIds.length : selectedSketch?.name || selectedBody?.name || selectedNurbs?.name }}</strong>
        <button v-if="selectedBody && selectedFace" @click="beginAdvanced('push')">Push / Pull</button>
        <button v-if="selectedBody && selectedFace" @click="faceSketch">{{ label('Эскиз на грани','Sketch on face') }}</button>
        <button v-if="selectedBody && selectedFace" @click="beginAdvanced('shell')">Shell</button>
        <button v-if="selectedBody && edgeIndex>=0" @click="beginAdvanced('chamfer')">{{ label('Фаска 3D','Chamfer 3D') }}</button>
        <button v-if="selectedBody && edgeIndex>=0" @click="beginAdvanced('edge-fillet')">{{ label('Скруглить 3D','Fillet 3D') }}</button>
        <button v-if="selectedBody" @click="beginAdvanced('split')">{{ label('Разрезать','Split') }}</button>
        <button v-if="selectedSketch?.analytic" @click="beginAdvanced('curve')">{{ label('Параметры кривой','Curve parameters') }}</button>
        <button v-if="selectedSketch && (selectedSketch.closed||selectedSketch.analytic)" @click="beginAdvanced('offset')">Offset</button>
        <button v-if="selectedSketch && !selectedSketch.closed && !selectedSketch.analytic" @click="beginAdvanced('extend')">{{ label('Продлить','Extend') }}</button>
        <button v-if="!selectedNurbs" @click="beginAdvanced('transform')">{{ label('Преобразовать выбор','Transform selection') }}</button>
        <button v-if="selectedSketch" class="primary" :disabled="!selectedSketch.closed" @click="beginExtrude()">{{ label('Выдавить · E', 'Extrude · E') }}</button>
        <button v-if="selectedSketch" :disabled="!selectedSketch.closed" @click="beginExtrude('revolve')">{{ label('Вращение', 'Revolve') }}</button>
        <button v-if="selectedSketch" :disabled="!selectedSketch.closed" @click="beginCorner('fillet')">{{ label('Скруглить', 'Fillet') }}</button>
        <button v-if="selectedSketch" :disabled="!selectedSketch.closed" @click="beginCorner('dogear')">DogEar</button>
        <button v-if="selectedSketch" @click="advancedOp=null; operation = operation === 'array' ? null : 'array'">{{ label('Круговые копии', 'Circular copies') }}</button><button v-if="!selectedNurbs" @click="duplicate">{{ label('Копия · ⌘/Ctrl D', 'Duplicate · ⌘/Ctrl D') }}</button>
        <details v-if="!selectedNurbs" class="transform-menu"><summary>{{ label('Точные преобразования', 'Exact transforms') }}</summary><div>
          <label>X <input v-model.number="dx" type="number" aria-label="ΔX"></label><label>Y <input v-model.number="dy" type="number" aria-label="ΔY"></label><label v-if="selectedBody">Z <input v-model.number="dz" type="number" aria-label="ΔZ"></label>
          <label>↻ <input v-model.number="angle" type="number" :aria-label="label('Поворот Z', 'Z rotation')">°</label><label>× <input v-model.number="scale" type="number" min=".001" step=".1" :aria-label="label('Масштаб', 'Scale')"></label><button @click="transform">{{ label('Применить', 'Apply') }}</button>
        </div></details><button class="delete" @click="remove">{{ label('Удалить', 'Delete') }}</button>
      </template>
      <span v-else class="subtle">{{ label('R — прямоугольник · C — круг · L — ломаная · V — выбор · E — выдавить · F — вписать · ? — помощь', 'R rectangle · C circle · L polyline · V select · E extrude · F fit · ? help') }}</span>
    </footer>
    <div v-if="showHelp" class="help-card"><p>{{ label('Грани: выберите поверхность, затем тяните её или жёлтую ручку. Ctrl/⌘ + клик выбирает несколько открытых граней для Shell. Для фаски и скругления включите «Рёбра».','Faces: select a surface, then drag it or its yellow handle. Ctrl/⌘ click selects multiple Shell openings. Switch to Edges for chamfers and fillets.') }}</p><p>{{ label('Shift + клик и «Рамка» выделяют несколько объектов. Манипулятор двигает, вращает и масштабирует весь выбор. У окружностей и дуг есть ручки центра, радиуса и концов дуги.','Shift click and Box select select multiple objects. The gizmo moves, rotates and scales the whole selection. Circles and arcs have center, radius and arc endpoint handles.') }}</p><strong>{{ label('Управление', 'Controls') }}</strong><p>{{ label('2D: тяните фигуру или вершину. Alt временно отключает привязку. Ломаная замыкается кликом по первой точке.', '2D: drag shapes or vertices. Alt bypasses snapping. Close a polyline by clicking its first point.') }}</p><p>{{ label('3D: тяните для вращения; G включает перемещение тела. ПКМ всегда вращает. Shift или средняя кнопка — панорама. Колесо — масштаб.', '3D: drag to orbit; G enables body movement. Right drag always orbits. Shift or middle drag pans. Wheel zooms.') }}</p><p>{{ label('E — предпросмотр выдавливания; зелёная ручка меняет высоту. Enter подтверждает, Escape отменяет. Ctrl/⌘ Z — отмена, Ctrl/⌘ Shift Z — повтор.', 'E previews extrusion; the green handle changes height. Enter applies, Escape cancels. Ctrl/⌘ Z undoes; Ctrl/⌘ Shift Z redoes.') }}</p><button @click="showHelp = false">{{ label('Понятно', 'Got it') }}</button></div>
    <div v-if="error" class="error-bar" role="alert">{{ error }} <button @click="error = ''">×</button></div>
  </section>
</template>
<style scoped>
.direct-workspace{position:fixed;inset:44px 0 0;z-index:20;display:flex;flex-direction:column;min-height:0;background:var(--bg);color:var(--text);outline:none;font-size:13px}.workspace-bar{display:flex;align-items:center;gap:16px;padding:10px 16px;border-bottom:1px solid var(--border);background:var(--surface)}button,input,summary,.file-open{color:var(--text);background:var(--surface-raised);border:1px solid var(--border);border-radius:5px;padding:7px 10px;font:inherit}button,summary{cursor:pointer}button:disabled{opacity:.4;cursor:default}button:hover:not(:disabled){background:var(--hover)}button:focus-visible,summary:focus-visible{outline:2px solid var(--accent)}[aria-pressed=true]{border-color:var(--accent);color:var(--accent)}.back{background:transparent}.history-tools{display:flex;gap:4px}.history-tools button{font-size:20px;padding:2px 12px}.save-status{margin-left:auto;color:var(--text-dim);font-size:12px}.file-menu{position:relative}.file-menu>div{position:absolute;right:0;top:40px;z-index:5;width:250px;display:grid;gap:6px;padding:10px;background:var(--surface);border:1px solid var(--border);box-shadow:0 8px 30px #0004}.file-open input{display:block;width:100%;padding:4px;font-size:11px}.split-workspace{flex:1;min-height:0;display:grid;grid-template-columns:minmax(0,var(--split)) 7px minmax(0,1fr)}.pane{display:flex;flex-direction:column;min-width:0;min-height:0}.pane-heading{display:flex;align-items:center;gap:12px;padding:10px 14px;background:var(--surface);border-bottom:1px solid var(--border)}.pane-heading strong{font-size:14px}.pane-heading span{font-size:11px;color:var(--text-dim)}.pane-heading button{margin-left:auto;padding:4px 9px}.pane-tools{min-height:46px;padding:7px 12px;display:flex;gap:5px;align-items:center;flex-wrap:wrap;border-bottom:1px solid var(--border)}.pane-tools .subtle{flex:1}.pane-tools button{font-size:12px}.canvas-wrap{flex:1;min-height:120px;position:relative;overflow:hidden}.canvas-wrap svg{width:100%;height:100%;display:block;touch-action:none;outline:none}.canvas-wrap svg:focus-visible{box-shadow:inset 0 0 0 2px var(--accent)}.selected{stroke-width:3}.splitter{background:var(--surface-raised);cursor:col-resize;touch-action:none;display:flex;align-items:center;justify-content:center;border-inline:1px solid var(--border)}.splitter:hover,.splitter:focus-visible{background:var(--accent)}.splitter span{height:35px;width:2px;background:var(--text-dim);border-radius:2px}.object-strip{min-height:47px;max-height:90px;overflow:auto;display:flex;align-items:center;gap:6px;padding:8px 12px;flex-wrap:wrap;border-top:1px solid var(--border);background:var(--surface)}.object-strip>span{font-size:11px;color:var(--text-dim);margin-right:5px}.object-strip button{font-size:12px;padding:4px 8px}.context-bar{min-height:60px;display:flex;align-items:center;gap:12px;flex-wrap:wrap;padding:10px 16px;border-top:1px solid var(--border);background:var(--surface)}.context-bar label{display:flex;align-items:center;gap:5px;color:var(--text-dim)}.context-bar input{width:65px;padding:6px}.primary{background:var(--accent);color:var(--bg);font-weight:600}.delete{margin-left:auto}.subtle{color:var(--text-dim);font-size:12px}.empty-hint{position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;text-align:center;pointer-events:none;color:var(--text-dim);padding:25px}.empty-hint strong{font-size:18px;font-weight:500}.empty-hint span{font-size:12px;max-width:280px}.zoom-tools{position:absolute;right:14px;bottom:14px;display:flex;gap:4px}.zoom-tools button{font-size:18px}.error-bar{padding:10px 16px;color:var(--danger);background:var(--surface);display:flex;justify-content:space-between}@media(max-width:750px){.direct-workspace{inset:0}.workspace-bar{gap:8px;padding:8px}.workspace-bar>strong{font-size:12px}.save-status{display:none}.pane-heading{padding:8px;gap:5px}.pane-heading span{display:none}.pane-tools{padding:5px}.pane-tools button{padding:5px;font-size:11px}.context-bar{gap:7px;padding:8px}.context-bar input{width:52px}.empty-hint strong{font-size:14px}}
.hovered{stroke:#e1d4ff;stroke-width:3}.operation-card{position:absolute;right:14px;top:14px;width:245px;display:grid;gap:10px;padding:15px;background:var(--surface);border:1px solid var(--border);border-radius:9px;box-shadow:0 8px 24px #0003}.operation-card small{font-size:11px;color:var(--text-dim);line-height:1.5}.operation-card label{display:flex;justify-content:space-between;align-items:center;gap:8px}.operation-card input{width:90px}.operation-card select{max-width:145px;background:var(--surface-raised);color:var(--text);padding:5px;border:1px solid var(--border)}.operation-card>div{display:flex;gap:5px}.segmented button{padding:5px 8px;font-size:12px}.live-measure{position:absolute;left:14px;top:14px;padding:8px 12px;border-radius:5px;background:var(--surface);color:var(--accent);font:14px monospace;pointer-events:none}.height-handle{cursor:ns-resize}.snap-toggle{display:flex;align-items:center;gap:4px;font-size:11px;margin-left:auto}.grid-input{width:50px;padding:4px}.transform-menu{position:relative}.transform-menu>div{position:absolute;bottom:40px;left:0;width:270px;display:flex;flex-wrap:wrap;gap:10px;padding:14px;border:1px solid var(--border);background:var(--surface);border-radius:8px;box-shadow:0 8px 24px #0003}.help-card{max-height:75vh;overflow:auto;position:absolute;right:18px;bottom:76px;width:min(360px,85vw);padding:20px;background:var(--surface);border:1px solid var(--border);border-radius:10px;box-shadow:0 8px 30px #0004;font-size:13px;line-height:1.6;z-index:5}@media(max-width:750px){.operation-card{width:195px;padding:10px;right:8px;top:8px}.pane-tools .subtle{display:none}.snap-toggle{margin-left:0}}
.nurbs-card{max-height:calc(100% - 28px);overflow:auto}.nurbs-cage circle{cursor:move}.trim-grid{display:grid!important;grid-template-columns:1fr 1fr;gap:5px!important}.trim-grid label{display:grid!important;gap:2px!important;font-size:11px}.trim-grid input{width:100%!important;box-sizing:border-box}
</style>

<style scoped>
.direct-workspace.embedded{position:absolute;inset:0;z-index:9}.embedded .workspace-bar{flex-wrap:wrap;gap:8px;padding:6px}.primitive-bar{display:flex;flex-wrap:wrap;gap:6px;align-items:center;padding:8px;border-bottom:1px solid var(--border)}.primitive-bar input{width:75px}.embedded .pane-tools{padding:5px}.embedded .save-status{display:none}
</style>
