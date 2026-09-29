export interface GeometryDisplayResult {changed:boolean;errors:{id:string;message:string}[]}
/** Exact geometry keys; a single response in flight and a bounded retained cache. */
export class SolidGeometryDisplayQueue<Item extends {id:string}, Mesh, Job> {
 private generation=0
 private cache=new Map<string,{mesh:Mesh;weight:number}>()
 private weight=0
 constructor(private port:{run(job:Job):Promise<Mesh>;cancel():void},private key:(item:Item)=>string,private job:(item:Item)=>Job,private meshWeight:(mesh:Mesh)=>number,private maxEntries=256,private maxWeight=64_000_000){
  if(!Number.isSafeInteger(maxEntries)||maxEntries<1||!Number.isSafeInteger(maxWeight)||maxWeight<1)throw Error('Invalid geometry display cache limits')
 }
 get size(){return this.cache.size}
 get retainedWeight(){return this.weight}
 get(item:Item){return this.cache.get(this.key(item))?.mesh}
 cancel(){this.generation++;this.port.cancel()}
 clear(){this.cancel();this.cache.clear();this.weight=0}
 private put(key:string,mesh:Mesh){
  const weight=256+key.length*2+this.meshWeight(mesh)
  if(weight>this.maxWeight)return false
  while(this.cache.size>=this.maxEntries||this.weight+weight>this.maxWeight){const oldest=this.cache.keys().next().value!;this.weight-=this.cache.get(oldest)!.weight;this.cache.delete(oldest)}
  this.cache.set(key,{mesh,weight});this.weight+=weight;return true
 }
 async prepare(items:readonly Item[]):Promise<GeometryDisplayResult>{
  this.cancel();const generation=this.generation,result:GeometryDisplayResult={changed:false,errors:[]}
  const requested=new Map<string,Item>()
  for(const item of items){const key=this.key(item);if(!this.cache.has(key))requested.set(key,item)}
  for(const [key,item] of requested){
   try{
    const mesh=await this.port.run(this.job(item))
    if(generation!==this.generation)return {changed:false,errors:[]}
    if(this.key(item)!==key)continue
    if(this.put(key,mesh))result.changed=true
    else result.errors.push({id:item.id,message:'Geometry display exceeds the cache budget.'})
   }catch(e){if(generation!==this.generation)return {changed:false,errors:[]};result.errors.push({id:item.id,message:e instanceof Error?e.message:String(e)})}
  }
  for(const item of items)if(!this.get(item)&&!result.errors.some(e=>e.id===item.id))result.errors.push({id:item.id,message:'Geometry display is unavailable. Reduce detail or retry.'})
  return result
 }
}
