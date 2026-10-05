import {sourceBodyRecordOptions,type SourceBodyRecord} from './sourceBodyArchive'
import type {SourceBodyResult} from './sourceBody'
import type {MainSolidWorkerClient} from './mainSolidWorkerClient'
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
    if(count>65536)throw new Error('Source edge display exceeds 65536 edges per body')
    const options={...source,displaySegments:Math.min(32,Math.floor(65536/Math.max(1,count)))}
    const result=await this.worker.run({kind:'sourceBodyRestore',options})
    if(generation!==this.generation)return
    if(!result.admitted)throw Object.assign(new Error(result.diagnostics.reason),{code:'CAD_SOURCE_BODY_RESTORE',sourceBodyId:record.id,diagnostics:result.diagnostics})
    publish(record.id,result)
   }catch(error){if(generation!==this.generation)return;fail(record.id,error)}
  }
 }
}
