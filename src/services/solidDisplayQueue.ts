import type {MainSolidJob} from './mainSolidProtocol'
import type {DirectBody} from './directModeling'
import {SolidDisplayCache,solidDisplayKey,type DisplayMesh} from './solidDisplayCache'
type DisplayJob=Extract<MainSolidJob,{kind:'displayMesh'}>
interface DisplayPort {run(job:DisplayJob):Promise<DisplayMesh>;cancel():void}
function yieldDisplayScan():Promise<void>{
 return new Promise(resolve=>{
  const channel=new MessageChannel()
  channel.port1.onmessage=()=>{channel.port1.close();channel.port2.close();resolve()}
  channel.port2.postMessage(null)
 })
}
/** One request in flight bounds temporary response allocations. Cache publication
 * belongs to a generation; a cancelled scene never receives its old display data. */
export class SolidDisplayQueue {
 private generation=0
 private keys=new WeakMap<DirectBody,string>()
 constructor(private port:DisplayPort,private cache:SolidDisplayCache,private yieldScan:()=>Promise<void>=yieldDisplayScan){}
 cancel(){this.generation++;this.keys=new WeakMap();this.port.cancel()}
 /** Lookup only within the prepared scene revision; never serialize during rendering. */
 get(body:DirectBody){const key=this.keys.get(body);return key===undefined?undefined:this.cache.get(key)}
 async prepare(bodies:readonly DirectBody[]):Promise<boolean>{
  this.cancel();const generation=this.generation
  const requested=new Map<string,DirectBody>(),preparedKeys=new WeakMap<DirectBody,string>()
  let scanned=0,yielded=false,hasCached=false,sliceStart=performance.now()
  for(const body of bodies)if(body.brep){
   const key=solidDisplayKey(body);preparedKeys.set(body,this.cache.canonicalKey(key)??key)
   if(this.cache.get(key))hasCached=true;else requested.set(key,body)
   // Key construction traverses full B-rep geometry. Let input/cancellation run
   // between bounded batches rather than blocking a thousand-body scene scan.
   if((++scanned%32===0||performance.now()-sliceStart>=8)&&scanned<bodies.length){
    yielded=true;await this.yieldScan()
    if(generation!==this.generation)return false
    sliceStart=performance.now()
   }
  }
  this.keys=preparedKeys
  // A render can happen while a scan yields; publish newly available cache keys
  // even when every refinement was already cached.
  let changed=yielded&&hasCached
  for(const [key,body] of requested){
   try{
    const result=await this.port.run({kind:'displayMesh',mesh:body.mesh,brep:body.brep,segments:12})
    if(generation!==this.generation)return false
    // Defend against in-place edits in addition to the caller's revision watcher.
    if(solidDisplayKey(body)!==key){this.keys.delete(body);continue}
    this.cache.set(key,result);changed=true
   }catch{
    if(generation!==this.generation)return false
    // A failed refinement keeps the working mesh. Continue the other bodies.
   }
  }
  return generation===this.generation&&changed
 }
}
