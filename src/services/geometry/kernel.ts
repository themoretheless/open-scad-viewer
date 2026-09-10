/** Synchronous host boundary for the own Rust geometry libraries; no geometry fallback. */
import {encodeBinary} from '../valueBinaryCodec'
import {decodePacked, writeLinear} from '../wasmHost'
import {unpackWasmBase64} from '../wasmPacking'
import wasmBase64 from '../../generated/geometry-kernels/bytes'

export class GeometryKernelError extends Error {
  constructor(public readonly code: string, message: string) { super(message); this.name = 'GeometryKernelError' }
}
export class NurbsCurveError extends GeometryKernelError {
  constructor(public readonly code: 'NURBS_INVALID_INPUT' | 'NURBS_RESOURCE_LIMIT' | 'NURBS_NUMERIC_ERROR', message: string) {
    super(code, message)
    this.name = 'NurbsCurveError'
  }
}
interface KernelExports extends WebAssembly.Exports {
 memory: WebAssembly.Memory
 abi_alloc(len:number):number
 abi_free(ptr:number,len:number):void
 abi_request(op:number,ptr:number,len:number):bigint
 abi_mesh_field(ptr:number,field:number):number
 abi_mesh_free(ptr:number):void
 abi_import_mesh(stride:number,vp:number,vl:number,ip:number,il:number):bigint
 abi_bvh_build(stride:number,leaf:number,vp:number,vl:number,ip:number,il:number):bigint
 abi_semantic_edges(vp:number,vl:number,ip:number,il:number,mfp:number,mfl:number,mtp:number,mtl:number,weld:number,creaseDotThreshold:number):bigint
 abi_array_field(handle:number,slot:number):number
 abi_array_free(handle:number):void
}
let wasm:KernelExports
let initialized = false
let wasmMemory: WebAssembly.Memory
let readingCadMesh = false
function initialize(): void {
  if (readingCadMesh) throw new Error('WASM calls are not allowed while reading a borrowed CAD mesh')
  if (initialized) return
  wasm = new WebAssembly.Instance(new WebAssembly.Module(unpackWasmBase64(wasmBase64))).exports as KernelExports
  wasmMemory=wasm.memory
  initialized = true
}
function takeResponse(packed:bigint):unknown {
 return decodePacked(wasmMemory,(pointer,size)=>wasm.abi_free(pointer,size),packed)
}
function request(op:number,value:unknown):unknown{
 initialize()
 const bytes=encodeBinary(value),ptr=writeLinear(wasmMemory,len=>wasm.abi_alloc(len),bytes)
 if(!ptr)throw new Error('WASM request allocation failed')
 try{return takeResponse(wasm.abi_request(op,ptr,bytes.length))}
 finally{wasm.abi_free(ptr,bytes.length)}
}
export function decodeNurbsResult<T>(result:unknown):T{
 const envelope=result as {ok:boolean;value:T;error:{code:string;message:string}}
 if(!envelope.ok){const ErrorType=envelope.error.code.startsWith('NURBS_')?NurbsCurveError:GeometryKernelError;throw new ErrorType(envelope.error.code,envelope.error.message)}
 return envelope.value
}
export function callGeometryRust<T>(op:string,args:object):T{return decodeNurbsResult<T>(request(0,{op,...args}))}
export function createRustSurfaceEvaluator(surface:object){
 const id=decodeNurbsResult<number>(request(6,surface));let disposed=false
 return {evaluate(u:number,v:number){if(disposed)throw new Error('NURBS surface evaluator is disposed.');return request(7,{id,u,v})},free(){if(!disposed){request(8,{id});disposed=true}}}
}

/** No JavaScript compiler fallback: both browser and Node use the same Rust frontend. */
export function compileTextRust<T>(source: string): T {
  initialize()
  const result = request(1,source) as {ok:boolean;value:T;message:string}
  if (!result.ok) throw new Error(result.message)
  return result.value as T
}

/** Stage-1 OpenSCAD migration binding: parse via the Rust openscad-core frontend (ABI op 10). */
export interface ScadRustDiagnostic {
  readonly code: string | null
  readonly message: string
  readonly start: number
  readonly end: number
  readonly line: number
  readonly column: number
}
export type ScadRustCompileResult =
  | {readonly ok: true; readonly ast: readonly unknown[]}
  | {readonly ok: false; readonly diagnostics: readonly ScadRustDiagnostic[]}
export function scadCompileRust(source: string, profile: 'openscad-viewer-subset@1' | 'openscad/stable-2021.01' = 'openscad-viewer-subset@1'): ScadRustCompileResult {
  initialize()
  return request(10,{source,profile}) as ScadRustCompileResult
}

/** Stage-2 OpenSCAD migration binding: value evaluation via the Rust openscad-core evaluator (ABI op 11). */
export interface ScadRustShapeDescriptor {
  readonly name: string
  readonly dimension: number
}
export type ScadRustEvalResult =
  | {readonly ok: true; readonly shapes: readonly ScadRustShapeDescriptor[]; readonly warnings: readonly string[]; readonly reduced: boolean}
  | {readonly ok: false; readonly diagnostics?: readonly ScadRustDiagnostic[]; readonly aborted?: boolean}
export function scadEvalRust(source: string, profile: 'openscad-viewer-subset@1' | 'openscad/stable-2021.01' = 'openscad-viewer-subset@1'): ScadRustEvalResult {
  initialize()
  return request(11,{source,profile}) as ScadRustEvalResult
}

export type GraphRustResult<T> = {ok:true;value:T} | {ok:false;error:{code:string;path:string;message:string;details?:unknown};customizer?:unknown}
export function prepareGraphRust<T>(kind:'graph'|'nurbs'|'text'|'textNurbs',value:unknown):GraphRustResult<T> {
  if(kind==='text') {
    if(typeof value !== 'string' || value.length > 262144)return {ok:false,error:{code:'text_error',path:'',message:'ModelGraph Text exceeds 256 KiB.'}}
    initialize()
    return request(4,value) as GraphRustResult<T>
  }
  // NURBS counts resolved values in Rust: each {param:id} becomes one value.
  // Its raw transport may therefore contain up to twice the 30000-value budget.
  // Keep paths linked until an error occurs, avoiding string allocation per value.
  type PendingValue={value:unknown;depth:number;parent?:PendingValue;key?:string|number}
  const pending:PendingValue[]=[{value,depth:0}]
  const limit=kind==='graph'?20000:60000
  const limitError=():GraphRustResult<T>=>({ok:false,error:{code:'input_limit',path:'/',message:'Document nesting or size limit exceeded.'}})
  const pathOf=(item:PendingValue)=>{
    const keys:(string|number)[]=[]
    while(item.parent){keys.push(item.key!);item=item.parent}
    return '/'+keys.reverse().join('/')
  }
  let scheduled=1
  try {
    while(pending.length){
      const item=pending.pop()!
      if(item.depth>64)return limitError()
      const type=typeof item.value
      if(type==='undefined'||type==='bigint'||type==='function'||type==='symbol'||(type==='number'&&!Number.isFinite(item.value)))
        return {ok:false,error:{code:'invalid_document',path:pathOf(item),message:'Expected a finite JSON value.'}}
      if(Array.isArray(item.value)){
        // Check before materializing/enqueuing children, including sparse arrays.
        if(item.value.length>limit-scheduled)return limitError()
        scheduled+=item.value.length
        for(let i=item.value.length-1;i>=0;i--)pending.push({value:item.value[i],depth:item.depth+1,parent:item,key:i})
      }else if(item.value&&type==='object'){
        for(const key in item.value){
          if(!Object.prototype.hasOwnProperty.call(item.value,key))continue
          if(++scheduled>limit)return limitError()
          pending.push({value:(item.value as Record<string,unknown>)[key],depth:item.depth+1,parent:item,key})
        }
      }
    }
  }catch{
    return {ok:false,error:{code:'invalid_document',path:'/',message:'Document could not be inspected.'}}
  }
  initialize()
  return request(kind==='graph'?2:kind==='textNurbs'?5:3,value) as GraphRustResult<T>
}


export interface CadMeshViews {
  readonly positions: Float64Array
  readonly indices: Uint32Array
  readonly faceIds: Uint32Array
}
/** Internal synchronous borrow: do not retain or mutate these views. The caller
 * must build its owned renderer buffers before returning. No WASM calls or await
 * are permitted while the views exist, because memory.grow invalidates them. */
export function withCadMesh<T>(id:number, read:(mesh:CadMeshViews)=>T):T {
  initialize()
  if(!Number.isInteger(id)||id<1||id>0xffffffff)throw new GeometryKernelError('GEOMETRY_INVALID_INPUT','Invalid CAD handle')
  const snapshot=decodeNurbsResult<number>(request(9,{id}))
  try {
    // Allocation above may grow memory. Never cache memory.buffer between calls.
    const buffer = wasmMemory.buffer
    const mesh:CadMeshViews = {
      positions:new Float64Array(buffer,wasm.abi_mesh_field(snapshot,0),wasm.abi_mesh_field(snapshot,1)),
      indices:new Uint32Array(buffer,wasm.abi_mesh_field(snapshot,2),wasm.abi_mesh_field(snapshot,3)),
      faceIds:new Uint32Array(buffer,wasm.abi_mesh_field(snapshot,4),wasm.abi_mesh_field(snapshot,5)),
    }
    readingCadMesh = true
    const result = read(mesh)
    if (result instanceof Promise) throw new TypeError('CAD mesh reader must be synchronous')
    return result
  } finally {
    readingCadMesh = false
    wasm.abi_mesh_free(snapshot)
  }
}
/** Packed input is copied into temporary linear-memory buffers and validated by Rust. */
export function importCadMesh(stride:number,vertices:Float32Array,indices:Uint32Array):number {
 initialize()
 if(!Number.isInteger(stride)||stride<3||stride>64)throw new GeometryKernelError('GEOMETRY_INVALID_INPUT','Invalid mesh vertex stride')
 let vp=0,ip=0
 try{
  vp=wasm.abi_alloc(vertices.byteLength);ip=wasm.abi_alloc(indices.byteLength)
  if(!vp||!ip)throw new GeometryKernelError('GEOMETRY_RESOURCE_LIMIT','Mesh exceeds transport limit')
  new Uint8Array(wasmMemory.buffer,vp,vertices.byteLength).set(new Uint8Array(vertices.buffer,vertices.byteOffset,vertices.byteLength))
  new Uint8Array(wasmMemory.buffer,ip,indices.byteLength).set(new Uint8Array(indices.buffer,indices.byteOffset,indices.byteLength))
  return decodeNurbsResult<number>(takeResponse(wasm.abi_import_mesh(stride,vp,vertices.length,ip,indices.length)))
 }finally{if(ip)wasm.abi_free(ip,indices.byteLength);if(vp)wasm.abi_free(vp,vertices.byteLength)}
}

/** Low-level access for raw-buffer analysis bindings (services/geometry/meshAnalysis). */
export function kernelRuntime():{exports:KernelExports,memory:WebAssembly.Memory,takeResponse:(packed:bigint)=>unknown}{
 initialize()
 return {exports:wasm,memory:wasmMemory,takeResponse}
}
