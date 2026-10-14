/** Detached BSP buffers. Copies survive WASM memory growth; the native lease is always released. */
import {decodeNurbsResult,kernelRuntime} from './kernel'
import type {TransparentTriangle} from '../transparentTriangleSplit'
export interface PackedTransparentBsp {
  planes:Float64Array;links:Uint32Array;owners:Uint32Array;vertices:Float64Array
  root:number;width:number;count:number;operations:number
}
export function buildTransparentBsp(triangles:readonly TransparentTriangle[],limit:number,tolerance:number,operationLimit:number):PackedTransparentBsp {
  const width=triangles[0]?.[0].length??0
  if(triangles.some(t=>t.some(v=>v.length!==width)))throw Error('Inconsistent transparency vertex width')
  if(triangles.length>limit||triangles.length>operationLimit)throw Error('Transparency input limit exceeded')
  if(triangles.length&&width<3)throw Error('Invalid transparency BSP vertex')
  const length=triangles.length*3*width
  if(length>32*1024*1024/8)throw Error('Transparency input exceeds transport limit')
  const input=new Float64Array(length)
  let offset=0
  for(const t of triangles)for(const v of t){input.set(v,offset);offset+=width}
  const {exports:wasm,memory,takeResponse}=kernelRuntime()
  let pointer=0,handle=0
  try {
    pointer=wasm.abi_alloc(input.byteLength)
    if(!pointer&&input.byteLength)throw Error('Transparency upload allocation failed')
    new Uint8Array(memory.buffer,pointer,input.byteLength).set(new Uint8Array(input.buffer))
    handle=decodeNurbsResult<number>(takeResponse(wasm.abi_transparent_bsp(width,pointer,input.length,limit,operationLimit,tolerance)))
    const field=(slot:number)=>wasm.abi_array_field(handle,slot)
    const planes=new Float64Array(memory.buffer,field(0),field(1)).slice()
    const links=new Uint32Array(memory.buffer,field(2),field(3)).slice()
    const owners=new Uint32Array(memory.buffer,field(4),field(5)).slice()
    const vertices=new Float64Array(memory.buffer,field(6),field(7)).slice()
    return {planes,links,owners,vertices,root:field(8),width:field(9),count:field(10),operations:field(11)}
  } finally {
    if(handle)wasm.abi_array_free(handle)
    if(pointer)wasm.abi_free(pointer,input.byteLength)
  }
}
