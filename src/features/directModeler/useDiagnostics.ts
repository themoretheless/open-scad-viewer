import { type BoundaryAgreement } from '../../services/solidBoundaryAgreement'
import { createSolidPreviewWorker } from '../../services/solidPreviewWorker'
import { type FaceContacts } from '../../services/solidFaceContacts'
import { type SelfIntersection } from '../../services/solidSelfIntersection'
import { type inspectSolidDisplay, inspectSolidIntersections } from '../../services/solidDiagnostics'
import { createSolidClearanceWorker } from '../../services/solidClearanceWorker'
import { type DirectDocument, type DirectBody } from '../../services/directModeling'
import { computed, ref, shallowRef, watchEffect, onUnmounted, type ShallowRef, type ComputedRef } from 'vue'

export interface DiagnosticsOptions {
  document: ShallowRef<DirectDocument>
  label: (a: string, b: string) => string
  props: Readonly<{ open: boolean }>
  selectedBody: ComputedRef<DirectBody | undefined>
}

export function useDiagnostics(options: DiagnosticsOptions) {
  const { document, label, props, selectedBody } = options
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
const inspectWithinFaces=ref(false),inspectExactBoundary=ref(false)
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
 void faceContactsWorker.run({...inspectWithinFaces.value?{kind:'selfIntersection' as const,maxSpans:Math.min(maxCells,100000),...(inspectExactBoundary.value?{boundaryAudit:{exactWork:Math.min(maxCells*100,1000000),trimPairs:10000,trimCells:Math.min(maxCells,100000),trimDomainCells:Math.min(maxCells*100,1000000)}}:{})}:{kind:'faceContacts' as const},model:body.brep,toleranceUv:1e-8,limits:{maxPairs:10000,maxCells,maxDomainCells:Math.min(8000000,maxCells*100),cellsPerPair:Math.min(100000,Math.max(1,Math.floor(maxCells/20))),domainCellsPerPair:Math.min(1000000,Math.max(1,maxCells*5)),maxBoxes:64}})
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
  return { diagnosticPanel, diagnosticsOpen, sectionZ, sectionNormal, diagnosticsRevision, boundaryOpen, boundaryBudget, boundaryRevision, boundaryAgreementPending, boundaryIndex, boundaryResult, boundaryError, boundaryBudgetInvalid, boundaryDefects, boundarySelected, boundaryLine, faceContactsOpen, faceContactsBudget, faceContactsRevision, faceContactsPending, faceContactIndex, inspectWithinFaces, inspectExactBoundary, faceContactsResult, faceContactsError, faceContactsBudgetInvalid, faceContactDefects, faceContactSelected, faceContactPoint, sectionValidity, sectionInputInvalid, sectionErrorMessage, diagnosticsPending, diagnostics, intersectionInspection, intersectionPending, intersectionWork, intersectionIndex, selectedIntersection, selectedIntersectionLines, collapsedSectionPoints }
}
