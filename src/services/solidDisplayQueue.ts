import type {MainSolidJob} from './mainSolidProtocol'
import type {DirectBody} from './directModeling'
import {SolidDisplayCache,solidDisplayKey,type DisplayMesh} from './solidDisplayCache'
type DisplayJob=Extract<MainSolidJob,{kind:'displayMesh'}>
interface DisplayPort {run(job:DisplayJob):Promise<DisplayMesh>;cancel():void}
/** One request in flight bounds temporary response allocations. Cache publication
 * belongs to a generation; a cancelled scene never receives its old display data. */
export class SolidDisplayQueue {
 private generation=0
 constructor(private port:DisplayPort,private cache:SolidDisplayCache){}
 cancel(){this.generation++;this.port.cancel()}
 async prepare(bodies:readonly DirectBody[]):Promise<boolean>{
  this.cancel();const generation=this.generation
  const requested=new Map<string,DirectBody>()
  for(const body of bodies)if(body.brep){const key=solidDisplayKey(body);if(!this.cache.get(key))requested.set(key,body)}
  let changed=false
  for(const [key,body] of requested){
   try{
    const result=await this.port.run({kind:'displayMesh',mesh:body.mesh,brep:body.brep,segments:12})
    if(generation!==this.generation)return false
    // Defend against in-place edits in addition to the caller's revision watcher.
    if(solidDisplayKey(body)!==key)continue
    this.cache.set(key,result);changed=true
   }catch{
    if(generation!==this.generation)return false
    // A failed refinement keeps the working mesh. Continue the other bodies.
   }
  }
  return generation===this.generation&&changed
 }
}
