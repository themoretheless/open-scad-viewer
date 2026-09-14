/** Host transport and UI snapshots; Rust owns graph execution and result selection. */
import type {SemanticColor, SemanticOccurrence, SemanticOutputRef, SemanticValueType} from '../core/semanticProgram'
import {MAX_NATIVE_GEOMETRY_CHARACTERS} from '../core/nativeGeometry'
import {callGeometryRust,GeometryKernelError} from './geometry/kernel'
import type {NurbsBrep} from './geometry/brep'
import type {BrepProfile} from './geometry/brepProfile'
import {BrepSemanticBackendError} from './brepSemanticErrors'
import {requireTrustedSemanticLowering,type SemanticLoweringSuccess} from './semanticProgramLowerer'
import {checkSemanticExecutionControl,SemanticProgramExecutionError,type SemanticExecutionControl} from './semanticProgramExecutor'

type Geometry={kind:'solid';model:NurbsBrep}|{kind:'profile';profile:BrepProfile}
type NativeValue={tag:'empty';valueType:SemanticValueType}|{tag:'value';valueType:SemanticValueType;geometry:Geometry}
type NativeFailure={tag:'failed';node:number|null;code:string;message:string}
interface NativeCommit {tag:'committed';outcomes:NativeValue[];resultItems:{reference:SemanticOutputRef;outcomeIndex:number}[]}
export interface BrepNativeOutput {readonly reference:SemanticOutputRef;readonly value:NativeValue;readonly occurrence:SemanticOccurrence;readonly color:SemanticColor}
function freeze<T>(value:T):T {
 if(value!==null&&typeof value==='object'&&!Object.isFrozen(value)){
  for(const child of Object.values(value))freeze(child)
  Object.freeze(value)
 }
 return value
}
function failure(report:NativeFailure):never {
 const error=new SemanticProgramExecutionError('E_SEMANTIC_BACKEND_FAILURE',report.node,report.message)
 error.backendCause=new BrepSemanticBackendError(report.code.startsWith('BREP_UNSUPPORTED_')?'E_BREP_SEMANTIC_UNSUPPORTED':report.code==='BREP_SEMANTIC_BUDGET'?'E_BREP_SEMANTIC_BUDGET':report.code==='BREP_SEMANTIC_CONTRACT'?'E_BREP_SEMANTIC_CONTRACT':'E_BREP_SEMANTIC_KERNEL',report.message,new GeometryKernelError(report.code,report.message))
 throw error
}
export async function executeBrepNativeProgram(input:SemanticLoweringSuccess,control:SemanticExecutionControl={}) {
 const trusted=requireTrustedSemanticLowering(input),program=trusted.program,core=program.core
 const maxNodes=control.maxNodes??core.nodes.length
 if(!Number.isSafeInteger(maxNodes)||maxNodes<0||core.nodes.length>maxNodes)throw new SemanticProgramExecutionError('E_SEMANTIC_BUDGET',null,'Semantic node budget exceeded')
 checkSemanticExecutionControl(control,null)
 if(core.language.contract!=='openscad-viewer/brep-1'){
  const error=new SemanticProgramExecutionError('E_SEMANTIC_BACKEND_BEGIN',null,'Native B-rep execution requires brep-1')
  error.backendCause=new BrepSemanticBackendError('E_BREP_SEMANTIC_UNSUPPORTED','B-rep semantic backend requires openscad-viewer/brep-1')
  throw error
 }
 let session:string|undefined,active:number|null=null
 let primary:SemanticProgramExecutionError|undefined
 try{
  const begun=callGeometryRust<{session:string}|NativeFailure>('brep_session',{action:'graph-begin',nodes:core.nodes,execution:core.execution,result:core.result,occurrences:core.occurrences,operations:core.operations,source:program.source,schema:core.schema,schemaVersion:core.schemaVersion,requiredFeatures:core.requiredFeatures,identityVersion:core.identityVersion,language:core.language,units:core.units,declaredCapabilities:core.declaredCapabilities,capabilityClosure:core.capabilityClosure,diagnosticTemplates:core.diagnosticTemplates,provenance:program.provenance,tessellationIntents:program.tessellationIntents,diagnostics:program.diagnostics,envelope:program,sourceText:trusted.sourceText,maxNodes:Math.max(1,maxNodes),maxBytes:MAX_NATIVE_GEOMETRY_CHARACTERS*2,maxCharacters:MAX_NATIVE_GEOMETRY_CHARACTERS})
  if('tag'in begun)failure(begun)
  session=begun.session
  for(let completed=0;completed<core.nodes.length;){
   active=completed
   checkSemanticExecutionControl(control,active)
   const progress=callGeometryRust<{tag:'progress'|'ready';completedNodes:number}|NativeFailure>('brep_session',{action:'graph-advance',session,limit:1})
   if(progress.tag==='failed')failure(progress)
   completed=progress.completedNodes
   checkSemanticExecutionControl(control,active)
   try{control.onNode?.(completed,core.nodes.length)}catch{throw new SemanticProgramExecutionError('E_SEMANTIC_CALLBACK',active,'Semantic execution progress callback failed')}
   // Let host cancellation run; no graph or geometry decisions occur here.
   if(completed%16===0)await new Promise<void>(resolve=>setTimeout(resolve,0))
   else await Promise.resolve()
   checkSemanticExecutionControl(control,active)
  }
  active=null
  checkSemanticExecutionControl(control,null)
  const report=callGeometryRust<NativeCommit|NativeFailure>('brep_session',{action:'graph-commit',session})
  session=undefined // commit consumes native ownership on success and refusal
  if(report.tag==='failed')failure(report)
  checkSemanticExecutionControl(control,null)
  let outputs:readonly BrepNativeOutput[]=freeze(report.resultItems.map(item=>({reference:item.reference,value:report.outcomes[item.outcomeIndex],occurrence:core.occurrences[item.reference.identityOccurrence],color:item.reference.color})))
  let disposed=false
  const dispose=async()=>{outputs=[];disposed=true}
  return Object.freeze({program,attestation:trusted.attestation,get outputs(){return outputs},get disposed(){return disposed},dispose,[Symbol.asyncDispose]:dispose})
 }catch(error){
  if(error instanceof SemanticProgramExecutionError)primary=error
  else if(error instanceof GeometryKernelError){
   try{failure({tag:'failed',node:active,code:error.code,message:error.message})}catch(mapped){primary=mapped as SemanticProgramExecutionError}
  }else{
   primary=new SemanticProgramExecutionError('E_SEMANTIC_BACKEND_FAILURE',active,'Native B-rep execution failed')
   primary.backendCause=error
  }
  throw primary
 }finally{
  if(session!==undefined){
   try{callGeometryRust('brep_session',{action:'dispose',session})}catch(cleanupError){
    if(primary)primary.cleanupError=cleanupError
    else throw cleanupError
   }
  }
 }
}
