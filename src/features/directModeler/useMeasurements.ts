import { type Vec3 } from '../../services/directSketchGeometry'
import { createSolidClearanceWorker } from '../../services/solidClearanceWorker'
import { type CadPairReport } from '../../services/cadInspection'
import { createSolidPreviewWorker } from '../../services/solidPreviewWorker'
import { solidVertexCount, type PointMeasurement, type CurveMeasurement, type FaceDistanceResult, type ShellDistanceResult } from '../../services/solidMeasurements'
import { type NurbsCurveDistance } from '../../services/nurbsCurve'
import { type DirectDocument, type DirectBody } from '../../services/directModeling'
import { computed, ref, shallowRef, watchEffect, onUnmounted, type Ref, type ShallowRef, type ComputedRef } from 'vue'

export interface MeasurementsOptions {
  document: ShallowRef<DirectDocument>
  edgeIndex: Ref<number>
  label: (a: string, b: string) => string
  props: Readonly<{ open: boolean }>
  selectedBody: ComputedRef<DirectBody | undefined>
  selection: Ref<string>
}

export function useMeasurements(options: MeasurementsOptions) {
  const { document, edgeIndex, label, props, selectedBody, selection } = options
const volumeDistanceOpen=ref(false),volumeContact=shallowRef<Vec3[]|null>(null)
const materialPathOpen=ref(false)
const materialPathOverlay=shallowRef<import('../../services/solidMaterialVolume').MaterialOverlay|null>(null)
const measurementOpen=ref(false),measureTarget=ref(''),measureA=ref(1),measureB=ref(2),curveParameter=ref(.5)
const formatMeasurement=(value:number)=>value!==0&&Math.abs(value)<1e-6?value.toExponential(3):value.toFixed(6)
const clearanceOpen=ref(false),clearanceRevision=ref(0),clearanceRetryVisible=ref(false)
const measurementTarget=computed(()=>measureTarget.value?document.value.bodies.find(body=>body.id===measureTarget.value):selectedBody.value)
const clearanceWorker=createSolidClearanceWorker()
const clearanceMeasurement=shallowRef<{value:CadPairReport|null;error:string}|null>(null)
const clearancePending=ref(false)
onUnmounted(()=>clearanceWorker.dispose())
watchEffect(onCleanup=>{
 void clearanceRevision.value
 const enabled=props.open&&measurementOpen.value&&clearanceOpen.value
 const source=selectedBody.value,target=measurementTarget.value,snapshot=document.value
 const sourceId=selection.value,targetId=measureTarget.value
 let current=true
 onCleanup(()=>{current=false;clearanceWorker.cancel()})
 clearanceMeasurement.value=null;clearancePending.value=false;clearanceRetryVisible.value=false
 if(!enabled||!source)return
 if(!target||target.id===source.id){clearanceMeasurement.value={value:null,error:label('Выберите другое тело B.','Choose a different body B.')};return}
 clearancePending.value=true
 const valid=()=>current&&props.open&&document.value===snapshot&&selection.value===sourceId&&measureTarget.value===targetId&&measurementOpen.value&&clearanceOpen.value
 void clearanceWorker.run({kind:'inspect',bodies:[source,target].map(({id,name,mesh})=>({id,name,mesh}))}).then(reports=>{
  if(valid())clearanceMeasurement.value={value:reports[0],error:''}
 }).catch(()=>{if(valid()){clearanceRetryVisible.value=true;clearanceMeasurement.value={value:null,error:label('Не удалось вычислить зазор по сетке. Повторите расчёт; при повторном отказе проверьте сетки выбранных тел.','Could not compute mesh clearance. Retry; if it fails again, inspect the selected body meshes.')}}})
 .finally(()=>{if(valid())clearancePending.value=false})
})
const measurementWorker=createSolidPreviewWorker(),curvatureWorker=createSolidPreviewWorker()
const measurementPending=ref(false),curvaturePending=ref(false)
const vertexAValid=computed(()=>!!selectedBody.value && Number.isInteger(measureA.value) && measureA.value>=1 && measureA.value<=solidVertexCount(selectedBody.value))
const vertexBValid=computed(()=>!!measurementTarget.value && Number.isInteger(measureB.value) && measureB.value>=1 && measureB.value<=solidVertexCount(measurementTarget.value))
const curveParameterValid=computed(()=>Number.isFinite(curveParameter.value) && curveParameter.value>=0 && curveParameter.value<=1)
const measurementRevision=ref(0),curvatureRevision=ref(0),measurementRetryVisible=ref(false),curvatureRetryVisible=ref(false)
const measurement=shallowRef<{value:PointMeasurement|null;error:string}|null>(null)
const curvatureMeasurement=shallowRef<{value:CurveMeasurement|null;error:string}|null>(null)
onUnmounted(()=>{measurementWorker.dispose();curvatureWorker.dispose()})
watchEffect(onCleanup=>{
 const enabled=props.open&&measurementOpen.value&&!clearanceOpen.value&&!volumeDistanceOpen.value&&!materialPathOpen.value
 const source=selectedBody.value,target=measurementTarget.value,a=measureA.value-1,b=measureB.value-1
 let current=true
 onCleanup(()=>{current=false;measurementWorker.cancel()})
 void measurementRevision.value
 measurement.value=null;measurementPending.value=false;measurementRetryVisible.value=false
 if(!enabled||!source)return
 if(!target){measurement.value={value:null,error:label('Выберите существующее тело B.','Choose an existing target body.')};return}
 if(!vertexAValid.value || !vertexBValid.value){measurement.value={value:null,error:!vertexAValid.value?label('Укажите существующую вершину A.','Choose an existing vertex A.'):label('Укажите существующую вершину B.','Choose an existing vertex B.')};return}
 measurementPending.value=true
 void measurementWorker.run({kind:'measureVertices',a:source,indexA:a,b:target,indexB:b}).then(value=>{
  if(current)measurement.value={value,error:''}
 }).catch(()=>{if(current){measurementRetryVisible.value=true;measurement.value={value:null,error:label('Не удалось измерить вершины. Проверьте тела и повторите измерение.','Could not measure these vertices. Check the bodies and retry the measurement.')}}})
 .finally(()=>{if(current)measurementPending.value=false})
})
watchEffect(onCleanup=>{
 const enabled=props.open&&measurementOpen.value,body=selectedBody.value,edge=edgeIndex.value,parameter=curveParameter.value
 let current=true
 onCleanup(()=>{current=false;curvatureWorker.cancel()})
 void curvatureRevision.value
 curvatureMeasurement.value=null;curvaturePending.value=false;curvatureRetryVisible.value=false
 if(!enabled||!body?.brep||edge<0)return
 if(!curveParameterValid.value){curvatureMeasurement.value={value:null,error:label('Задайте параметр ребра от 0 до 1.','Set an edge parameter between 0 and 1.')};return}
 curvaturePending.value=true
 void curvatureWorker.run({kind:'measureEdge',body,edge,parameter}).then(value=>{
  if(current)curvatureMeasurement.value={value,error:''}
 }).catch(()=>{if(current){curvatureRetryVisible.value=true;curvatureMeasurement.value={value:null,error:label('Не удалось измерить кривизну. Проверьте ребро и повторите измерение.','Could not measure curvature. Check the edge and retry the measurement.')}}})
 .finally(()=>{if(current)curvaturePending.value=false})
})
const edgeDistanceOpen=ref(false),edgeDistanceA=ref(1),edgeDistanceB=ref(2),edgeDistanceBudget=ref(10000)
const edgeDistanceRevision=ref(0),edgeDistanceRetryVisible=ref(false)
const edgeDistanceWorker=createSolidPreviewWorker(),edgeDistancePending=ref(false)
const edgeDistance=shallowRef<{value:NurbsCurveDistance|null;error:string}|null>(null)
onUnmounted(()=>edgeDistanceWorker.dispose())
watchEffect(onCleanup=>{
 void edgeDistanceRevision.value
 const enabled=props.open&&measurementOpen.value&&edgeDistanceOpen.value
 const source=selectedBody.value,target=measurementTarget.value,a=edgeDistanceA.value-1,b=edgeDistanceB.value-1,maxCells=edgeDistanceBudget.value
 let current=true
 onCleanup(()=>{current=false;edgeDistanceWorker.cancel()})
 edgeDistance.value=null;edgeDistancePending.value=false;edgeDistanceRetryVisible.value=false
 if(!enabled||!source)return
 if(!source.brep||!target?.brep){edgeDistance.value={value:null,error:label('Выберите два тела с B-rep геометрией.','Choose two bodies with B-rep geometry.')};return}
 if(!Number.isInteger(a)||!source.brep.edges[a]){edgeDistance.value={value:null,error:label('Укажите существующее ребро A.','Choose an existing edge A.')};return}
 if(!Number.isInteger(b)||!target.brep.edges[b]){edgeDistance.value={value:null,error:label('Укажите существующее ребро B.','Choose an existing edge B.')};return}
 edgeDistancePending.value=true
 void edgeDistanceWorker.run({kind:'curveDistance',a:source.brep.edges[a].curve,b:target.brep.edges[b].curve,toleranceMm:.001,maxCells}).then(value=>{
  if(current)edgeDistance.value={value,error:''}
 }).catch(()=>{if(current){edgeDistanceRetryVisible.value=true;edgeDistance.value={value:null,error:label('Не удалось измерить выбранные рёбра. Проверьте геометрию или увеличьте объём расчёта.','Could not measure these edges. Check their geometry or increase the calculation budget.')}}})
 .finally(()=>{if(current)edgeDistancePending.value=false})
})
const faceDistanceOpen=ref(false),faceDistanceA=ref(1),faceDistanceB=ref(2),faceDistanceBudget=ref(10000)
const faceDistanceRevision=ref(0),faceDistanceRetryVisible=ref(false)
const faceDistanceWorker=createSolidPreviewWorker(),faceDistancePending=ref(false)
const faceDistance=shallowRef<{value:FaceDistanceResult|null;error:string}|null>(null)
onUnmounted(()=>faceDistanceWorker.dispose())
watchEffect(onCleanup=>{
 void faceDistanceRevision.value
 const enabled=props.open&&measurementOpen.value&&faceDistanceOpen.value
 const source=selectedBody.value,target=measurementTarget.value,a=faceDistanceA.value-1,b=faceDistanceB.value-1,maxCells=faceDistanceBudget.value
 let current=true
 onCleanup(()=>{current=false;faceDistanceWorker.cancel()})
 faceDistance.value=null;faceDistancePending.value=false;faceDistanceRetryVisible.value=false
 if(!enabled||!source)return
 if(!source.brep||!target?.brep){faceDistance.value={value:null,error:label('Выберите два тела с B-rep геометрией.','Choose two bodies with B-rep geometry.')};return}
 if(!Number.isInteger(a)||!source.brep.faces[a]){faceDistance.value={value:null,error:label('Укажите существующую грань A.','Choose an existing face A.')};return}
 if(!Number.isInteger(b)||!target.brep.faces[b]){faceDistance.value={value:null,error:label('Укажите существующую грань B.','Choose an existing face B.')};return}
 faceDistancePending.value=true
 void faceDistanceWorker.run({kind:'faceDistance',options:{a:source.brep,b:target.brep,faceA:a,faceB:b,toleranceMm:.001,toleranceUv:1e-7,maxCells,maxDomainCells:maxCells===10000?1000000:8000000}}).then(value=>{
  if(current)faceDistance.value={value,error:''}
 }).catch(()=>{if(current){faceDistanceRetryVisible.value=true;faceDistance.value={value:null,error:label('Не удалось измерить выбранные грани. Проверьте геометрию или увеличьте объём расчёта.','Could not measure these faces. Check their geometry or increase the calculation budget.')}}})
 .finally(()=>{if(current)faceDistancePending.value=false})
})
const shellDistanceOpen=ref(false),shellDistanceBudget=ref(100000)
const shellDistanceRevision=ref(0),shellDistanceRetryVisible=ref(false)
const shellDistanceWorker=createSolidPreviewWorker(),shellDistancePending=ref(false)
const shellDistance=shallowRef<{value:ShellDistanceResult|null;error:string}|null>(null)
onUnmounted(()=>shellDistanceWorker.dispose())
watchEffect(onCleanup=>{
 void shellDistanceRevision.value
 const enabled=props.open&&measurementOpen.value&&shellDistanceOpen.value
 const source=selectedBody.value,target=measurementTarget.value,maxCells=shellDistanceBudget.value
 let current=true
 onCleanup(()=>{current=false;shellDistanceWorker.cancel()})
 shellDistance.value=null;shellDistancePending.value=false;shellDistanceRetryVisible.value=false
 if(!enabled||!source)return
 if(!source.brep||!target?.brep){shellDistance.value={value:null,error:label('Выберите два тела с B-rep геометрией.','Choose two bodies with B-rep geometry.')};return}
 if(source.id===target.id){shellDistance.value={value:null,error:label('Выберите другое тело B.','Choose a different body B.')};return}
 shellDistancePending.value=true
 void shellDistanceWorker.run({kind:'shellDistance',options:{a:source.brep,b:target.brep,toleranceMm:.001,toleranceUv:1e-7,maxCells,maxDomainCells:maxCells===100000?1000000:8000000}}).then(value=>{
  if(current)shellDistance.value={value,error:''}
 }).catch(()=>{if(current){shellDistanceRetryVisible.value=true;shellDistance.value={value:null,error:label('Не удалось измерить оболочки тел. Проверьте геометрию или увеличьте объём расчёта.','Could not measure these boundary shells. Check their geometry or increase the calculation budget.')}}})
 .finally(()=>{if(current)shellDistancePending.value=false})
})
  return { volumeDistanceOpen, volumeContact, materialPathOpen, materialPathOverlay, measurementOpen, measureTarget, measureA, measureB, curveParameter, formatMeasurement, clearanceOpen, clearanceRevision, clearanceRetryVisible, measurementTarget, clearanceMeasurement, clearancePending, measurementPending, curvaturePending, vertexAValid, vertexBValid, curveParameterValid, measurementRevision, curvatureRevision, measurementRetryVisible, curvatureRetryVisible, measurement, curvatureMeasurement, edgeDistanceOpen, edgeDistanceA, edgeDistanceB, edgeDistanceBudget, edgeDistanceRevision, edgeDistanceRetryVisible, edgeDistancePending, edgeDistance, faceDistanceOpen, faceDistanceA, faceDistanceB, faceDistanceBudget, faceDistanceRevision, faceDistanceRetryVisible, faceDistancePending, faceDistance, shellDistanceOpen, shellDistanceBudget, shellDistanceRevision, shellDistanceRetryVisible, shellDistancePending, shellDistance }
}
