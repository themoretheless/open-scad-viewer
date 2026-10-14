import { primitiveKinds, type Pane } from './helpers'
import { createSolidPreviewWorker } from '../../services/solidPreviewWorker'
import { type DirectSketch, type DirectDocument, type DirectHistory } from '../../services/directModeling'
import { validateSimpleSketch } from '../../services/directSketchGeometry'
import { sketchDimensions } from '../../services/directDimensions'
import { createSolidNurbsCurve, createSolidNurbsSurface } from '../../services/solidNurbs'
import { type SolidNurbsEditOptions } from '../../services/solidNurbsEdit'
import { computed, ref, watch, onUnmounted, type Ref, type ShallowRef, type ComputedRef } from 'vue'

export interface PrimitivesOptions {
  activeGroup: Ref<string>
  advancedOp: Ref<"split" | "push" | "shell" | "curve" | "transform" | "offset" | "loft" | "chamfer" | "instance-create" | "instance-transform" | "instance-place" | "nurbs-loft" | "nurbs-sweep" | "nurbs-patch" | "nurbs-rebuild" | "nurbs-reduce" | "nurbs-surface-rebuild" | "nurbs-surface-reduce" | "profile-union" | "profile-difference" | "profile-intersection" | "extend" | "edge-fillet" | "profile-prepare" | "nurbs-offset" | "nurbs-point-trim" | "nurbs-match" | "nurbs-prepare" | "nurbs-curve-match" | null>
  brepSegments: Ref<number>
  cancelCommand: () => void
  commit: (next: DirectDocument) => void
  document: ShallowRef<DirectDocument>
  error: Ref<string>
  finishCommit: (added: string[], next?: DirectDocument) => void
  fit: (pane: Pane, selectedOnly?: boolean) => void
  history: DirectHistory
  label: (a: string, b: string) => string
  lockedIds: Ref<string[]>
  nativeNurbsPending: Ref<boolean>
  operation: Ref<"array" | "extrude" | "revolve" | "fillet" | "dogear" | null>
  pickObject: (id: string, pane: Pane, add?: boolean) => void
  prepareCommit: (next: DirectDocument, origin?: "edit" | "file") => { validateLocked: ((resolved: DirectDocument) => void) | undefined; added: string[]; }
  props: Readonly<{ open: boolean }>
  isRestoreDisposed: () => boolean
  run: (action: () => void) => void
  runNativeNurbs: (options: SolidNurbsEditOptions) => Promise<void>
  selectedCurvePair: ComputedRef<string[] | null>
  tool: Ref<"trim" | "select" | "slot" | "circle" | "polyline" | "rectangle" | "arc">
  visibleSketches: ComputedRef<DirectSketch[]>
  workspace: Ref<HTMLElement | undefined>
}

export function usePrimitives(options: PrimitivesOptions) {
  const { activeGroup, advancedOp, brepSegments, cancelCommand, commit, document, error, finishCommit, fit, history, label, lockedIds, nativeNurbsPending, operation, pickObject, prepareCommit, props, isRestoreDisposed, run, runNativeNurbs, selectedCurvePair, tool, visibleSketches, workspace } = options
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
function primitiveLabel(kind: typeof primitiveKinds[number]) { return ({box:label('Куб','Box'),wedge:label('Клин','Wedge'),cylinder:label('Цилиндр','Cylinder'),cone:label('Конус','Cone'),sphere:label('Сфера','Sphere'),frustum:label('Усечённый конус','Frustum'),tube:label('Труба','Tube'),torus:label('Тор','Torus')})[kind] }
const primitiveWorker=createSolidPreviewWorker(),primitivePending=ref(false)
let primitiveGeneration=0
function cancelPrimitive(){primitiveGeneration++;primitiveWorker.cancel();primitivePending.value=false}
onUnmounted(()=>{cancelPrimitive();primitiveWorker.dispose()})
watch(()=>[props.open,document.value,operation.value,advancedOp.value,tool.value,primitiveSize.value,primitiveTopRadius.value,primitiveInnerRadius.value,roundGeometry.value,brepSegments.value,activeGroup.value,lockedIds.value.join(',')],()=>{if(primitivePending.value)cancelPrimitive()},{flush:'sync'})
async function addPrimitive(kind: typeof primitiveKinds[number]) {
 cancelCommand();error.value=''
 const generation=++primitiveGeneration,before=document.value,target=history,id=crypto.randomUUID()
 const current=()=>generation===primitiveGeneration&&props.open&&!isRestoreDisposed()&&before===document.value&&target===history
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
function updateDimensionSketch(sketch:import('../../services/directModeling').DirectSketch){run(()=>{
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
  return { roundGeometry, primitiveSize, primitiveTopRadius, primitiveInnerRadius, TOOL_ICONS, primitiveLabel, primitivePending, cancelPrimitive, addPrimitive, bridgeEndA, bridgeEndB, bridgeTension, bridgeError, bridgeTensionInvalid, addBridgeCurve, updateDimensionSketch, dimensionLabels, addNurbs }
}
