import {isOriginalQuadricGraphReport,type OriginalQuadricGraphRequest} from '../kernel/originalQuadricGraphProtocol'
import {isOriginalQuadricCarrierReport,type OriginalQuadricCarrierRequest} from '../kernel/originalQuadricCarrierProtocol'
import {isOriginalPatchContactsReport,type OriginalPatchContactsRequest} from '../kernel/originalPatchContactsProtocol'
import {isSurfaceIsoTargetReport,type SurfaceIsoTargetRequest} from '../kernel/surfaceIsoTargetProtocol'
import {encodeBinary,decodeBinary} from '../core/valueBinaryCodec'
import {isSurfaceAuditReport,type SurfaceAuditRequest} from '../kernel/surfaceSelfIntersectionProtocol'
let admitted=false
self.addEventListener('message',async(event:MessageEvent<SurfaceAuditRequest|SurfaceIsoTargetRequest|OriginalPatchContactsRequest|OriginalQuadricCarrierRequest|OriginalQuadricGraphRequest>)=>{
 if(admitted)return
 admitted=true
 const r=event.data
 if(!['surface-self-audit/1','surface-iso-target-window/1','original-patch-contacts/1','original-quadric-carrier-roots/1','original-quadric-graph-contacts/1'].includes(r?.type)||typeof r.id!=='string'||r.id.length>128||!(r.module instanceof WebAssembly.Module)){self.close();return}
 try{
  if(WebAssembly.Module.imports(r.module).length)throw new Error('Surface audit requires an import-free module')
  const instance=await WebAssembly.instantiate(r.module,{})
  const w=instance.exports as unknown as {memory:WebAssembly.Memory;abi_alloc(n:number):number;abi_free(p:number,n:number):void;abi_request(op:number,p:number,n:number):bigint}
  let payload
  switch(r.type){
   case 'original-quadric-graph-contacts/1':payload={op:'nurbs_original_quadric_graph_contacts',curve:r.curve,patch:r.patch,quadric:r.quadric,constantProducts:r.constantProducts,tolerance:r.tolerance,maxSpans:r.maxSpans,maxExactWork:r.maxExactWork};break
   case 'original-quadric-carrier-roots/1':payload={op:'nurbs_original_quadric_carrier_roots',curve:r.curve,quadric:r.quadric,constantProducts:r.constantProducts,tolerance:r.tolerance,maxSpans:r.maxSpans,maxExactWork:r.maxExactWork};break
   case 'original-patch-contacts/1':payload={op:'nurbs_original_affine_patch_contacts',curve:r.curve,patch:r.patch,tolerance:r.tolerance,maxSpans:r.maxSpans,maxExactWork:r.maxExactWork};break
   case 'surface-iso-target-window/1':payload={op:'nurbs_surface_iso_target_window',source:r.source,axis:r.axis,parameter:r.parameter,target:r.target,window:r.window,maxCells:r.maxCells};break
   case 'surface-self-audit/1':payload={op:'nurbs_surface_self_intersection_audit',surface:r.surface,maxSpans:r.maxSpans};break
  }
  const data=encodeBinary(payload)
  const ptr=w.abi_alloc(data.length)
  if(!ptr)throw new Error('Surface audit allocation refused')
  let reply:any
  try{
   new Uint8Array(w.memory.buffer,ptr,data.length).set(data)
   const packed=w.abi_request(0,ptr,data.length),p=Number(packed&0xffffffffn),n=Number(packed>>32n)
   try{reply=decodeBinary(new Uint8Array(w.memory.buffer,p,n))}finally{w.abi_free(p,n)}
  }finally{w.abi_free(ptr,data.length)}
  if(reply?.ok!==true)throw new Error(`${reply?.error?.code??'NATIVE_AUDIT'}: ${reply?.error?.message??'Native surface audit refused'}`)
  if(!(r.type==='original-quadric-graph-contacts/1'?isOriginalQuadricGraphReport(reply.value):r.type==='original-quadric-carrier-roots/1'?isOriginalQuadricCarrierReport(reply.value):r.type==='original-patch-contacts/1'?isOriginalPatchContactsReport(reply.value):r.type==='surface-iso-target-window/1'?isSurfaceIsoTargetReport(reply.value):isSurfaceAuditReport(reply.value)))throw new Error('Invalid native surface audit report')
  self.postMessage({type:'surface-self-result/1',id:r.id,ok:true,report:reply.value})
 }catch(error){
  self.postMessage({type:'surface-self-result/1',id:r.id,ok:false,error:(error instanceof Error?error.message:'Surface audit failed').slice(0,1024)})
 }finally{self.close()}
})
