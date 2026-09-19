/** Synchronous host boundary for the own Rust geometry libraries; no geometry fallback. */
import {encodeBinary} from '../valueBinaryCodec'
import {decodePacked, writeLinear} from '../wasmHost'
import {unpackBrotliWasmBase64} from '../wasmBrotliPacking'
import wasmBase64 from '../../generated/geometry-kernels/bytes'
import {
  assertGeometryLeaseCurrent,
  bumpGeometryKernelEpoch,
  cancelGeometryKernel,
  geometryKernelEpoch,
  isGeometryKernelCancelled,
  issueGeometryLease,
  publishLastKnownGood,
  readLastKnownGood,
  releaseGeometryLease,
  resetGeometryKernelCancel,
  GeometryLeaseError,
  type GeometryLease,
  type LastKnownGoodSnapshot,
} from './kernelLeases'

export {
  assertGeometryLeaseCurrent,
  bumpGeometryKernelEpoch,
  cancelGeometryKernel,
  geometryKernelEpoch,
  isGeometryKernelCancelled,
  issueGeometryLease,
  publishLastKnownGood,
  readLastKnownGood,
  releaseGeometryLease,
  resetGeometryKernelCancel,
  GeometryLeaseError,
  type GeometryLease,
  type LastKnownGoodSnapshot,
}

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
 abi_picking_create(stride:number,leaf:number,vp:number,vl:number,ip:number,il:number):bigint
 abi_solid_placement(vp:number,vl:number,ip:number,il:number,mp:number,ml:number):bigint
 abi_export_alloc(len:number):number
 abi_export_prepare(vp:number,vl:number,ip:number,il:number,mp:number,ml:number,float32:number):bigint
 abi_export_append(handle:number,vp:number,vl:number,ip:number,il:number,mp:number,ml:number):bigint
 abi_semantic_edges(vp:number,vl:number,ip:number,il:number,mfp:number,mfl:number,mtp:number,mtl:number,weld:number,creaseDotThreshold:number):bigint
 abi_array_field(handle:number,slot:number):number
 abi_array_free(handle:number):void
}
let wasm:KernelExports
let initialized = false
let wasmMemory: WebAssembly.Memory
let readingCadMesh = false
let warming: Promise<void> | undefined
/** Compile before interactive use. Never replace an instance owning live handles. */
export function warmGeometryKernel(): Promise<void> {
  if (initialized) return Promise.resolve()
  warming ??= (async () => {
    const module = await WebAssembly.compile(unpackBrotliWasmBase64(wasmBase64))
    // Instantiate asynchronously as well: browsers refuse a synchronous instantiation of a module over
    // 8 MB on the main thread, which is where the viewport and the Solid workspace warm the kernel.
    const instance = await WebAssembly.instantiate(module)
    // A synchronous caller may have initialized the runtime while compilation
    // was pending. Its native snapshots must remain attached to that instance.
    if (!initialized) {
      wasm = instance.exports as KernelExports
      wasmMemory = wasm.memory
      initialized = true
    }
  })().finally(() => { warming = undefined })
  return warming
}
/** True once an instance exists, so main-thread callers can avoid the synchronous compile path. */
export function isGeometryKernelReady(): boolean { return initialized }
function initialize(): void {
  if (readingCadMesh) throw new Error('WASM calls are not allowed while reading a borrowed CAD mesh')
  if (initialized) return
  if (typeof window !== 'undefined') {
    throw new Error('Geometry kernel must be warmed with warmGeometryKernel() before use on the main thread')
  }
  wasm = new WebAssembly.Instance(new WebAssembly.Module(unpackBrotliWasmBase64(wasmBase64))).exports as KernelExports
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
export function callGeometryRust<T>(op:string,args:object):T{
  if (isGeometryKernelCancelled()) {
    throw new GeometryLeaseError('Geometry kernel cancelled; refuse WASM request')
  }
  return decodeNurbsResult<T>(request(0,{op,...args}))
}

export function createRustSurfaceEvaluator(surface:object){
 const id=decodeNurbsResult<number>(request(6,surface));let disposed=false
 return {evaluate(u:number,v:number){if(disposed)throw new Error('NURBS surface evaluator is disposed.');return request(7,{id,u,v})},free(){if(!disposed){request(8,{id});disposed=true}}}
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
