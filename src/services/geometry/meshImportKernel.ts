/** Raw-buffer file imports and display preparation; all numerical work is native. */
import {kernelRuntime, decodeNurbsResult, GeometryKernelError} from './kernel'
function upload(bytes:Uint8Array):number {
 const {exports:wasm}=kernelRuntime(),ptr=wasm.abi_alloc(bytes.length)
 if(!ptr&&bytes.length)throw new GeometryKernelError('GEOMETRY_RESOURCE_LIMIT','Mesh exceeds transport limit')
 if(bytes.length)new Uint8Array(kernelRuntime().memory.buffer,ptr,bytes.length).set(bytes)
 return ptr
}
export function decodeMeshBytesInKernel(format:'obj'|'ply'|'stl'|'off'|'binary-stl',bytes:Uint8Array):{positions:Float64Array;indices:Uint32Array}{
 const {exports:wasm,takeResponse}=kernelRuntime();let ptr=0,handle=0
 try {
  ptr=upload(bytes);handle=decodeNurbsResult<number>(takeResponse(wasm.abi_mesh_decode(['obj','ply','stl','off','binary-stl'].indexOf(format),ptr,bytes.length)))
  const vp=wasm.abi_array_field(handle,0),vl=wasm.abi_array_field(handle,1),ip=wasm.abi_array_field(handle,2),il=wasm.abi_array_field(handle,3)
  const memory=kernelRuntime().memory.buffer
  return {positions:new Float64Array(memory,vp,vl).slice(),indices:new Uint32Array(memory,ip,il).slice()}
 }finally{if(handle)wasm.abi_array_free(handle);if(ptr)wasm.abi_free(ptr,bytes.length)}
}
export function renderTriangleSoupInKernel(positions:Float32Array):{vertices:Float32Array;indices:Uint32Array;faceIds:Uint32Array;discarded:number}{
 const {exports:wasm,takeResponse}=kernelRuntime();let ptr=0,handle=0
 try {
  ptr=upload(new Uint8Array(positions.buffer,positions.byteOffset,positions.byteLength))
  const result=decodeNurbsResult<{handle:number;discarded:number}>(takeResponse(wasm.abi_mesh_soup_render(ptr,positions.length)));handle=result.handle
  const vp=wasm.abi_array_field(handle,0),vl=wasm.abi_array_field(handle,1),ip=wasm.abi_array_field(handle,2),il=wasm.abi_array_field(handle,3),fp=wasm.abi_array_field(handle,8),fl=wasm.abi_array_field(handle,9)
  const memory=kernelRuntime().memory.buffer
  return {vertices:new Float32Array(memory,vp,vl).slice(),indices:new Uint32Array(memory,ip,il).slice(),faceIds:new Uint32Array(memory,fp,fl).slice(),discarded:result.discarded}
 }finally{if(handle)wasm.abi_array_free(handle);if(ptr)wasm.abi_free(ptr,positions.byteLength)}
}
