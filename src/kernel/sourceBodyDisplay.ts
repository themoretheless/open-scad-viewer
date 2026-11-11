import {callGeometryRust} from './geometry/kernel'
import {sourceBodyRecordOptions,type SourceBodyRecord} from './sourceBodyArchive'
import type {SourceBodyResult,SourceBodyDefinition} from './sourceBody'
import type {MainSolidWorkerClient} from './mainSolidWorkerClient'
/** Native incidence addresses locate a refusal; they do not authorize geometry. */
export class SourceBodyDisplayError extends Error {
 readonly code='CAD_SOURCE_BODY_RESTORE'
 readonly pair:number|null
 readonly faces:number[]
 constructor(readonly diagnostics:SourceBodyResult['diagnostics'],definition:SourceBodyDefinition,
  readonly sourceBodyId:string){
  super(diagnostics.reason)
  const candidate=(diagnostics.incidence as {uncertainPair?:unknown}|null)?.uncertainPair
  this.pair=typeof candidate==='number'&&Number.isSafeInteger(candidate)&&candidate>=0
   &&candidate<definition.shell.pairs.length?candidate:null
  this.faces=this.pair===null?[]:[...new Set(definition.shell.pairs[this.pair]!.uses.map(use=>use[0]!))]
 }
}
export function sourceBodyDisplayMessage(error:Error,locale:string):string {
 const label=(ru:string,en:string)=>locale==='ru'?ru:en
 if(error instanceof SourceBodyDisplayError){
  const reason=error.message
  if(reason==='mapped-source-endpoints-unproven')return label('Концы ребра не согласованы с гранями. Проверьте точки обрезки и концевые переходы.','Edge endpoints do not agree with the faces. Check trim points and end transitions.')
  if(reason==='mapped-source-composition-unproven')return label('Кривая ребра не подтверждена на обеих гранях. Проверьте исходные кривые и их диапазоны.','The edge curve is unverified on both faces. Check the original curves and their parameter ranges.')
  if(reason==='mapped-source-uses-not-opposite')return label('Направления соседних границ не противоположны. Проверьте ориентацию граней.','Adjacent boundaries do not have opposite directions. Check face orientation.')
  if(reason.includes('work-limit'))return label('Исчерпан лимит проверки. Увеличьте лимиты или разделите деталь и повторите расчёт.','The verification work limit was reached. Increase limits or split the part, then retry.')
 }
 return label('Подготовка рёбер не завершена. Повторите расчёт; если отказ повторяется, проверьте исходную геометрию и лимиты.','Edge preparation is incomplete. Retry; if it fails again, inspect the source geometry and work limits.')
}
export function sourceBodyDisplayLocation(error:Error,locale:string):string {
 if(!(error instanceof SourceBodyDisplayError)||error.pair===null)return ''
 return locale==='ru'?`Ребро ${error.pair+1} · грани ${error.faces.map(n=>n+1).join(', ')}`
  :`Edge ${error.pair+1} · faces ${error.faces.map(n=>n+1).join(', ')}`
}
/** A separate client keeps source preview cancellation independent of edits. */
export class SourceBodyDisplay {
 private generation=0
 constructor(private worker:Pick<MainSolidWorkerClient,'run'|'cancel'>){}
 cancel(){this.generation++;this.worker.cancel()}
 async prepare(records:SourceBodyRecord[],publish:(id:string,result:SourceBodyResult)=>void,fail:(id:string,error:unknown)=>void){
  this.cancel();const generation=this.generation
  for(const record of records){
   if(generation!==this.generation)return
   try{
    const source=sourceBodyRecordOptions(record)
    const count=source.definition.shell.pairs.length
    const budget=callGeometryRust<{displaySegments:number;divisions:number;domainCellsPerFace:number}>('cad_client_geometry',{operation:'sourceDisplayBudget',edges:count,faces:source.definition.shell.regions.length})
    const options={...source,displaySegments:budget.displaySegments,faceDisplay:{divisions:budget.divisions,toleranceUv:source.limits.embedding.toleranceUv,domainCellsPerFace:budget.domainCellsPerFace}}
    const result=await this.worker.run({kind:'sourceBodyRestore',options})
    if(generation!==this.generation)return
    if(!result.admitted)throw new SourceBodyDisplayError(result.diagnostics,source.definition,record.id)
    publish(record.id,result)
   }catch(error){if(generation!==this.generation)return;fail(record.id,error)}
  }
 }
}
