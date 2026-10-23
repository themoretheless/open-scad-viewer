import type {DirectBody} from './directModeling'
import type {SnapGeometry} from './modelingSnaps'
import type {MainSolidJob} from './mainSolidProtocol'
import {stringifyMeshJson} from './meshJson'
type Job=Extract<MainSolidJob,{kind:'bodySnaps'}>
interface Port {run(job:Job):Promise<SnapGeometry>;cancel():void}
const key=(body:DirectBody)=>stringifyMeshJson({mesh:body.mesh,brep:body.brep})
/** Exact content keys survive history copies; per-object keys assume immutable snapshots. */
export class SolidSnapPreparation {
 private generation=0
 private cache=new Map<string,SnapGeometry>()
 private keys=new WeakMap<DirectBody,string>()
 constructor(private port:Port){}
 get size(){return this.cache.size}
 get(body:DirectBody){let fingerprint=this.keys.get(body);if(fingerprint===undefined){fingerprint=key(body);this.keys.set(body,fingerprint)}return this.cache.get(fingerprint)}
 cancel(){this.generation++;this.port.cancel()}
 clear(){this.cancel();this.cache.clear()}
 async prepare(bodies:readonly DirectBody[]){
  this.cancel();const generation=this.generation,active=new Map(bodies.map(body=>[key(body),body])),errors:{id:string;message:string}[]=[]
  for(const body of this.cache.keys())if(!active.has(body))this.cache.delete(body)
  let changed=false
  for(const [expected,body] of active){
   this.keys.set(body,expected)
   if(this.cache.has(expected))continue
   try{
    const geometry=await this.port.run({kind:'bodySnaps',body})
    if(generation!==this.generation)return {changed:false,errors:[]}
    if(key(body)!==expected){errors.push({id:body.id,message:'Snap input changed during preparation.'});continue}
    this.cache.set(expected,geometry);changed=true
   }catch(e){if(generation!==this.generation)return {changed:false,errors:[]};errors.push({id:body.id,message:e instanceof Error?e.message:String(e)})}
  }
  return {changed,errors}
 }
}
