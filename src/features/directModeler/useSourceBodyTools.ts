import { createSolidPreviewWorker } from '../../services/solidPreviewWorker'
import { type SourceSeamResult, type SourceBodyResult } from '../../services/sourceBody'
import { sourceBodyRecordOptions } from '../../services/sourceBodyArchive'
import { type SourceWallResult } from '../../services/sourceWallTransport'
import { type SourceWallCoverageResult } from '../../services/sourceWallCoverageTransport'
import { type SourceWallScanResult } from '../../services/sourceWallScanTransport'
import { download, type Pane } from './helpers'
import { type DirectDocument, type Point2, type DirectBody } from '../../services/directModeling'
import { computed, ref, shallowRef, watch, onUnmounted, type Ref, type ShallowRef, type ComputedRef } from 'vue'

export interface SourceBodyToolsOptions {
  document: ShallowRef<DirectDocument>
  error: Ref<string>
  label: (a: string, b: string) => string
  objectInView: (id: string) => boolean
  objectSelectable: (id: string) => boolean
  project: (p: ArrayLike<number>, pane: Pane) => Point2
  props: Readonly<{ open: boolean }>
  isRestoreDisposed: () => boolean
  restoringDraft: Ref<boolean>
  selectedBody: ComputedRef<DirectBody | undefined>
  sourceDisplayResults: ShallowRef<Record<string, SourceBodyResult>>
  sourceEdgeSelection: Ref<string>
}

export function useSourceBodyTools(options: SourceBodyToolsOptions) {
  const { document, error, label, objectInView, objectSelectable, project, props, isRestoreDisposed, restoringDraft, selectedBody, sourceDisplayResults, sourceEdgeSelection } = options
const sourceStepWorker=createSolidPreviewWorker(),sourceStepPending=ref(false),sourceStepError=ref(''),sourceStepTolerance=ref(1e-7)
const selectedSourceBody=computed(()=>(document.value.sourceBodies??[]).find(record=>sourceEdgeSelection.value.startsWith(record.id+':')))
const sourceSeamWorker=createSolidPreviewWorker(),sourceSeamPending=ref(false),sourceSeamError=ref('')
const sourceSeamResult=shallowRef<SourceSeamResult|null>(null)
let sourceSeamGeneration=0
function cancelSourceSeam(){sourceSeamGeneration++;sourceSeamWorker.cancel();sourceSeamPending.value=false}
watch(()=>[document.value,props.open,sourceEdgeSelection.value],()=>{
 cancelSourceSeam();sourceSeamResult.value=null;sourceSeamError.value=''
},{flush:'sync'})
onUnmounted(()=>{cancelSourceSeam();sourceSeamWorker.dispose()})
const sourceSeamMessage=computed(()=>{
 const result=sourceSeamResult.value
 if(!result)return ''
 if(result.qualified)return label('Касательность подтверждена на всём ребре.','Tangency is qualified over the whole edge.')
 if(result.reason==='source-seam-endpoint-normal-unresolved')return label('Нормаль на конце ребра не подтверждена. Проверьте концевой переход; повтор с тем же бюджетом не устраняет вырождение.','The endpoint normal is unresolved. Inspect the end transition; repeating the same budget does not resolve a singularity.')
 if(result.reason==='source-seam-angular-envelope-oblique')return label('Касательность в выбранном допуске не подтверждена. Проверьте сопряжение соседних граней.','Tangency within tolerance is unproven. Inspect the adjacent face transition.')
 return label('Проверка участка не завершена. Повторите расчёт или уточните геометрию и лимиты.','The interval check is incomplete. Retry or inspect the geometry and work limits.')
})
async function qualifySourceSeam(){
 cancelSourceSeam();sourceSeamResult.value=null;sourceSeamError.value=''
 const record=selectedSourceBody.value,source=document.value,selection=sourceEdgeSelection.value,generation=sourceSeamGeneration
 const selected=sourceDisplayResults.value[record?.id??'']?.edges.find(edge=>record!.id+':'+edge.index===selection)
 if(!record||!selected||restoringDraft.value||!props.open||!objectSelectable(record.id))return
 sourceSeamPending.value=true
 try{
  const result=await sourceSeamWorker.run({kind:'sourceBodyRestore',options:{...sourceBodyRecordOptions(record),
   displaySegments:0,faceDisplay:undefined,stepExchange:undefined,
   seamQualification:{edge:selected.index,limits:{maxSineSquared:1e-6,cells:100000,curveSpans:100000,normalSpans:100000}}}})
  if(generation!==sourceSeamGeneration||document.value!==source||sourceEdgeSelection.value!==selection||selectedSourceBody.value!==record||!props.open||isRestoreDisposed())return
  if(!result.admitted||!result.seamQualification)throw Error(result.diagnostics.reason)
  sourceSeamResult.value=result.seamQualification
 }catch(e){if(generation===sourceSeamGeneration)sourceSeamError.value=e instanceof Error?e.message:String(e)}
 finally{if(generation===sourceSeamGeneration)sourceSeamPending.value=false}
}
const sourceWallWorker=createSolidPreviewWorker(),sourceWallPending=ref(false),sourceWallError=ref('')
const sourceWallResult=shallowRef<SourceWallResult|null>(null),sourceWallOpen=ref(false),sourceWallMenuOpen=ref(false)
const sourceWallCoverageResult=shallowRef<SourceWallCoverageResult|null>(null),sourceWallCoverageMode=ref(false)
const sourceWallScanResult=shallowRef<SourceWallScanResult|null>(null),sourceWallScanMode=ref(false)
const sourceWallGroups=ref<[number[],number[]]>([[],[]]),sourceWallSide=ref<0|1>(0)
const sourceWallMinimum=ref(1),sourceWallTolerance=ref(0.02)
let sourceWallGeneration=0
function cancelSourceWall(){sourceWallGeneration++;sourceWallWorker.cancel();sourceWallPending.value=false}
watch(()=>[document.value,props.open,selectedSourceBody.value],()=>{
 cancelSourceWall();sourceWallResult.value=null;sourceWallScanResult.value=null;sourceWallCoverageResult.value=null;sourceWallError.value='';sourceWallGroups.value=[[],[]]
},{flush:'sync'})
watch(()=>[sourceWallGroups.value,sourceWallMinimum.value,sourceWallTolerance.value,sourceWallOpen.value],()=>{
 cancelSourceWall();sourceWallResult.value=null;sourceWallScanResult.value=null;sourceWallCoverageResult.value=null;sourceWallError.value=''
},{deep:true,flush:'sync'})
onUnmounted(()=>{cancelSourceWall();sourceWallWorker.dispose()})
const sourceWallFaces=computed(()=>Array.from({length:sourceDisplayResults.value[selectedSourceBody.value?.id??'']?.faceCount??0},(_,i)=>i))
function sourceWallFileMenuToggle(e:Event){
 sourceWallMenuOpen.value=(e.currentTarget as HTMLDetailsElement).open
 if(!sourceWallMenuOpen.value && sourceWallPending.value)cancelSourceWall()
}
function toggleSourceWallFace(face:number){
 if(!selectedSourceBody.value||!objectSelectable(selectedSourceBody.value.id))return
 const side=sourceWallSide.value,other=side===0?1:0
 const groups:[number[],number[]]=[[...sourceWallGroups.value[0]],[...sourceWallGroups.value[1]]]
 groups[side]=groups[side].includes(face)?groups[side].filter(f=>f!==face):[...groups[side],face].sort((a,b)=>a-b)
 groups[other]=groups[other].filter(f=>f!==face);sourceWallGroups.value=groups
}
const sourceWallUnprovenFaces=computed(()=>{
 const faces=new Set<number>()
 for(const pair of sourceWallCoverageResult.value?.pairs??[])if(!pair.proven){faces.add(pair.faces[0]);faces.add(pair.faces[1])}
 return faces
})
function sourceWallFaceUncertain(body:string,face:number){
 return sourceWallOpen.value && body===selectedSourceBody.value?.id && (!!sourceWallResult.value?.clearance.uncertainFaces?.includes(face)
  ||!!(sourceWallCoverageResult.value&&(!sourceWallCoverageResult.value.enumerationComplete||sourceWallUnprovenFaces.value.has(face)))
  ||!!(sourceWallScanResult.value?.thinFound&&sourceWallScanResult.value.witness?.endpoints.some(e=>e.face===face)))
}
function sourceWallFaceFill(body:string,face:number){
 if(!sourceWallOpen.value||body!==selectedSourceBody.value?.id)return '#77b8b0'
 if(sourceWallFaceUncertain(body,face))return '#ff9977'
 return sourceWallGroups.value[0].includes(face)?'#58c4f2':sourceWallGroups.value[1].includes(face)?'#dfacff':'#77b8b0'
}
const sourceWallMeasurement=computed(()=>{
 const record=selectedSourceBody.value,r=sourceWallResult.value,scan=sourceWallScanResult.value,w=scan?.witness??r?.witness
 if(!sourceWallOpen.value||!record||!objectInView(record.id)||(!scan&&!r?.qualified)||!w)return null
 const endpoints=w.endpoints.map(e=>{
  const center=project(e.worldMm.map(q=>q[0]/2+q[1]/2),'3d')
  const corners=Array.from({length:8},(_,k)=>project(e.worldMm.map((q,i)=>q[(k>>i)&1]),'3d'))
  const x=Math.min(...corners.map(p=>p[0])),y=Math.min(...corners.map(p=>p[1]))
  return {face:e.face,center,x,y,width:Math.max(...corners.map(p=>p[0]))-x,height:Math.max(...corners.map(p=>p[1]))-y,bounds:e.worldMm}
 })
 if(endpoints.some(e=>![...e.center,e.x,e.y,e.width,e.height].every(Number.isFinite)))return null
 return {endpoints,points:endpoints.map(e=>e.center.join(',')).join(' '),interval:scan?w.lengthMm:r!.intervalMm}
})
const sourceWallMessage=computed(()=>{
 const r=sourceWallResult.value;if(!r)return ''
 if(r.qualified)return r.converged?label('Толщина между выбранными сторонами подтверждена.','Thickness between the selected sides is qualified.'):
  label('Интервал толщины подтверждён, но заданный допуск ещё не достигнут.','Thickness bounds are qualified; the requested tolerance is not reached.')
 return r.reason==='source-wall-clearance-unproven'?label('Нижняя граница не подтверждена. Проверьте выделенные грани и выбор противоположных сторон.','The lower bound is unproven. Inspect highlighted faces and choose opposing sides.'):
  label('Допустимый участок материала не найден. Уточните выбор сторон; отсутствие тонких участков не подтверждено.','No qualified material chord was found. Refine the side selection; absence of thin regions is unproven.')
})
const sourceWallInputError=computed(()=>{
 if(!Number.isFinite(sourceWallMinimum.value)||sourceWallMinimum.value<=0)return label('Укажите минимальную толщину больше нуля.','Enter a minimum thickness greater than zero.')
 if(!sourceWallScanMode.value&&!sourceWallCoverageMode.value&&(!Number.isFinite(sourceWallTolerance.value)||sourceWallTolerance.value<=0))return label('Укажите допуск толщины больше нуля.','Enter a thickness tolerance greater than zero.')
 return ''
})
async function qualifySourceWall(scan=false,coverage=false){
 sourceWallScanMode.value=scan;sourceWallCoverageMode.value=coverage
 cancelSourceWall();sourceWallResult.value=null;sourceWallScanResult.value=null;sourceWallCoverageResult.value=null;sourceWallError.value=''
 const record=selectedSourceBody.value,source=document.value,generation=sourceWallGeneration
 if(!record||!props.open||!sourceWallOpen.value||restoringDraft.value||!objectSelectable(record.id)||(!scan&&!coverage&&sourceWallGroups.value.some(g=>!g.length)))return
 if(sourceWallInputError.value){sourceWallError.value=sourceWallInputError.value;return}
 sourceWallPending.value=true
 try{
  const result=await sourceWallWorker.run({kind:'sourceBodyRestore',options:{...sourceBodyRecordOptions(record),
   displaySegments:0,faceDisplay:undefined,stepExchange:undefined,seamQualification:undefined,
   ...(coverage?{wallCoverage:{minimumMm:sourceWallMinimum.value,adaptiveSelf:{planeControls:1000000,cells:10000,spans:100000,cellsPerFace:512,spansPerFace:4096},limits:{pairs:100000,planeControls:1000000,normalSpans:10000,gapCells:10000,gapSpans:20000,maxSineSquared:1e-6}}}:scan?{wallScan:{minimumMm:sourceWallMinimum.value,toleranceUv:1e-7,grid:3,maxAttempts:256,
    limits:{cells:10000,domainCells:10000,normalSpans:1000,maxSineSquared:1e-6}}}:{wallQualification:{groups:[[...sourceWallGroups.value[0]],[...sourceWallGroups.value[1]]],minimumMm:sourceWallMinimum.value,
    toleranceMm:sourceWallTolerance.value,toleranceUv:1e-7,grid:3,maxAttempts:256,
    limits:{gapCells:50000,gapSpans:100000,cells:10000,domainCells:10000,normalSpans:1000,maxSineSquared:1e-6}}})}})
  if(generation!==sourceWallGeneration||document.value!==source||selectedSourceBody.value!==record||!props.open||!sourceWallOpen.value||isRestoreDisposed())return
  if(!result.admitted||(coverage?!result.wallCoverage:scan?!result.wallScan:!result.wallQualification))throw Error(result.diagnostics.reason)
  if(coverage)sourceWallCoverageResult.value=result.wallCoverage!;else if(scan)sourceWallScanResult.value=result.wallScan!;else sourceWallResult.value=result.wallQualification!
 }catch(e){if(generation===sourceWallGeneration)sourceWallError.value=e instanceof Error?e.message:String(e)}
 finally{if(generation===sourceWallGeneration)sourceWallPending.value=false}
}
let sourceStepGeneration=0
function cancelSourceStep(){sourceStepGeneration++;sourceStepWorker.cancel();sourceStepPending.value=false}
watch(()=>[document.value,props.open,sourceEdgeSelection.value,sourceStepTolerance.value],()=>{cancelSourceStep();sourceStepError.value=''},{flush:'sync'})
onUnmounted(()=>{cancelSourceStep();sourceStepWorker.dispose()})
async function exportSourceStep(){
 cancelSourceStep();sourceStepError.value=''
 const record=selectedSourceBody.value,source=document.value,generation=sourceStepGeneration
 if(!record||restoringDraft.value||!props.open)return
 sourceStepPending.value=true
 try{
  const options={...sourceBodyRecordOptions(record),displaySegments:0,faceDisplay:undefined,
   stepExchange:{toleranceMm:sourceStepTolerance.value,trimWork:10_000_000,
    limits:{rootChecks:100000,mappingCells:100000,replayMappingPerUse:100000,
     exactWork:100_000_000,driverCells:100000,spans:100000,endpoints:100000}}}
  const result=await sourceStepWorker.run({kind:'sourceBodyRestore',options})
  if(generation!==sourceStepGeneration||document.value!==source||selectedSourceBody.value!==record||!props.open||isRestoreDisposed())return
  if(!result.admitted)throw Error(result.diagnostics.reason)
  const step=result.stepExchange
  if(!step?.prepared)throw Error(step?.reason??'source-step-preparation-unproven')
  download(step.text,record.name.replace(/[^a-zA-Z0-9_-]/g,'_')+'.step')
 }catch(e){if(generation===sourceStepGeneration)sourceStepError.value=e instanceof Error?e.message:String(e)}
 finally{if(generation===sourceStepGeneration)sourceStepPending.value=false}
}
async function exportStepCurrent() {
  if(restoringDraft.value)return
  const body=selectedBody.value
  if(!body)return
  const snapshot=structuredClone(body)
  error.value=''
  try {
    const {exportSolidStepCurrent}=await import('../../services/solidStepExchange')
    download(await exportSolidStepCurrent(snapshot),'edited-body.step')
  } catch(e){error.value=e instanceof Error?e.message:String(e)}
}
async function exportStepAssembly() {
  if(restoringDraft.value)return
  const snapshot=structuredClone(document.value)
  error.value=''
  try {
    const {exportSolidStepAssembly}=await import('../../services/solidStepAssembly')
    download(await exportSolidStepAssembly(snapshot),'edited-scene.step')
  } catch(e){error.value=e instanceof Error?e.message:String(e)}
}
async function exportStepOriginal() {
  if(restoringDraft.value)return
  error.value = ''
  try {
    const {exportSolidStepOriginal} = await import('../../services/solidStepExchange')
    download(await exportSolidStepOriginal(), 'retained-model.step')
  } catch (e) { error.value = e instanceof Error ? e.message : String(e) }
}
  return { sourceStepPending, sourceStepError, sourceStepTolerance, selectedSourceBody, sourceSeamPending, sourceSeamError, sourceSeamResult, cancelSourceSeam, sourceSeamMessage, qualifySourceSeam, sourceWallPending, sourceWallError, sourceWallResult, sourceWallOpen, sourceWallMenuOpen, sourceWallCoverageResult, sourceWallCoverageMode, sourceWallScanResult, sourceWallScanMode, sourceWallGroups, sourceWallSide, sourceWallMinimum, sourceWallTolerance, cancelSourceWall, sourceWallFaces, sourceWallFileMenuToggle, toggleSourceWallFace, sourceWallFaceUncertain, sourceWallFaceFill, sourceWallMeasurement, sourceWallMessage, sourceWallInputError, qualifySourceWall, cancelSourceStep, exportSourceStep, exportStepCurrent, exportStepAssembly, exportStepOriginal }
}
