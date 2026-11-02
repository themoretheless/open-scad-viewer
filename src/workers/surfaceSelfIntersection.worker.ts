import {encodeBinary,decodeBinary} from '../services/valueBinaryCodec'
import {isSurfaceAuditReport,type SurfaceAuditRequest} from '../services/surfaceSelfIntersectionProtocol'
let admitted=false
self.addEventListener('message',async(event:MessageEvent<SurfaceAuditRequest>)=>{
 if(admitted)return
 admitted=true
 const r=event.data
 if(r?.type!=='surface-self-audit/1'||typeof r.id!=='string'||r.id.length>128||!(r.module instanceof WebAssembly.Module)){self.close();return}
 try{
  if(WebAssembly.Module.imports(r.module).length)throw new Error('Surface audit requires an import-free module')
  const instance=await WebAssembly.instantiate(r.module,{})
  const w=instance.exports as unknown as {memory:WebAssembly.Memory;abi_alloc(n:number):number;abi_free(p:number,n:number):void;abi_request(op:number,p:number,n:number):bigint}
  const data=encodeBinary({op:'nurbs_surface_self_intersection_audit',surface:r.surface,maxSpans:r.maxSpans})
  const ptr=w.abi_alloc(data.length)
  if(!ptr)throw new Error('Surface audit allocation refused')
  let reply:any
  try{
   new Uint8Array(w.memory.buffer,ptr,data.length).set(data)
   const packed=w.abi_request(0,ptr,data.length),p=Number(packed&0xffffffffn),n=Number(packed>>32n)
   try{reply=decodeBinary(new Uint8Array(w.memory.buffer,p,n))}finally{w.abi_free(p,n)}
  }finally{w.abi_free(ptr,data.length)}
  if(reply?.ok!==true)throw new Error(`${reply?.error?.code??'NATIVE_AUDIT'}: ${reply?.error?.message??'Native surface audit refused'}`)
  if(!isSurfaceAuditReport(reply.value))throw new Error('Invalid native surface audit report')
  self.postMessage({type:'surface-self-result/1',id:r.id,ok:true,report:reply.value})
 }catch(error){
  self.postMessage({type:'surface-self-result/1',id:r.id,ok:false,error:(error instanceof Error?error.message:'Surface audit failed').slice(0,1024)})
 }finally{self.close()}
})
