/** Synchronous host boundary for the own Rust geometry libraries; no geometry fallback. */
import { initSync, execute, SurfaceEvaluator, compile_modelgraph_text, compile_modelgraph, compile_modelgraph_nurbs, compile_modelgraph_text_nurbs, execute_modelgraph_text, cad_mesh_buffer, cad_import_mesh } from '../generated/geometry-kernels/kernel.js'
import { gunzipSync } from 'fflate/browser'
import wasmBase64 from '../generated/geometry-kernels/bytes'

export class GeometryKernelError extends Error {
  constructor(public readonly code: string, message: string) { super(message); this.name = 'GeometryKernelError' }
}
export class NurbsCurveError extends GeometryKernelError {
  constructor(public readonly code: 'NURBS_INVALID_INPUT' | 'NURBS_RESOURCE_LIMIT' | 'NURBS_NUMERIC_ERROR', message: string) {
    super(code, message)
    this.name = 'NurbsCurveError'
  }
}
let initialized = false
let wasmMemory: WebAssembly.Memory
let readingCadMesh = false
function initialize(): void {
  if (readingCadMesh) throw new Error('WASM calls are not allowed while reading a borrowed CAD mesh')
  if (initialized) return
  const binary = atob(wasmBase64)
  wasmMemory = initSync({ module: gunzipSync(Uint8Array.from(binary, character => character.charCodeAt(0))) }).memory
  initialized = true
}
export function decodeNurbsResult<T>(text: string): T {
  const result = JSON.parse(text)
  if (!result.ok) {
    const ErrorType = result.error.code.startsWith('NURBS_') ? NurbsCurveError : GeometryKernelError
    throw new ErrorType(result.error.code, result.error.message)
  }
  return result.value as T
}
export function callGeometryRust<T>(op: string, args: object): T {
  initialize()
  return decodeNurbsResult<T>(execute(JSON.stringify({ op, ...args })))
}
export function createRustSurfaceEvaluator(surface: object): SurfaceEvaluator {
  initialize()
  try { return new SurfaceEvaluator(JSON.stringify(surface)) }
  catch (error) {
    if (typeof error === 'string') {
      const detail = JSON.parse(error)
      throw new NurbsCurveError(detail.code, detail.message)
    }
    throw error
  }
}

/** No JavaScript compiler fallback: both browser and Node use the same Rust frontend. */
export function compileTextRust<T>(source: string): T {
  initialize()
  const result = JSON.parse(compile_modelgraph_text(source))
  if (!result.ok) throw new Error(result.message)
  return result.value as T
}

export type GraphRustResult<T> = {ok:true;value:T} | {ok:false;error:{code:string;path:string;message:string;details?:unknown};customizer?:unknown}
export function prepareGraphRust<T>(kind:'graph'|'nurbs'|'text'|'textNurbs',value:unknown):GraphRustResult<T> {
  if(kind==='text') {
    if(typeof value !== 'string' || value.length > 262144)return {ok:false,error:{code:'text_error',path:'',message:'ModelGraph Text exceeds 256 KiB.'}}
    initialize()
    return JSON.parse(execute_modelgraph_text(value))
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
  let scheduled=1,input:string|undefined
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
    input=JSON.stringify(value)
  }catch{
    return {ok:false,error:{code:'invalid_document',path:'/',message:'Document could not be serialized as JSON.'}}
  }
  if(input===undefined)return {ok:false,error:{code:'invalid_document',path:'/',message:'Expected a JSON document.'}}
  initialize()
  return JSON.parse(kind==='graph'?compile_modelgraph(input):kind==='textNurbs'?compile_modelgraph_text_nurbs(input):compile_modelgraph_nurbs(input))
}


export interface CadMeshViews {
  readonly positions: Float64Array
  readonly indices: Uint32Array
  readonly faceIds: Uint32Array
}
function cadBinaryError(error: unknown): never {
  if (typeof error === 'string') {
    const detail = JSON.parse(error) as {code:string;message:string}
    throw new GeometryKernelError(detail.code, detail.message)
  }
  throw error
}
/** Internal synchronous borrow: do not retain or mutate these views. The caller
 * must build its owned renderer buffers before returning. No WASM calls or await
 * are permitted while the views exist, because memory.grow invalidates them. */
export function withCadMesh<T>(id:number, read:(mesh:CadMeshViews)=>T):T {
  initialize()
  let snapshot: ReturnType<typeof cad_mesh_buffer>
  try { snapshot = cad_mesh_buffer(id) } catch (error) { return cadBinaryError(error) }
  try {
    // Allocation above may grow memory. Never cache memory.buffer between calls.
    const buffer = wasmMemory.buffer
    const mesh:CadMeshViews = {
      positions:new Float64Array(buffer,snapshot.positions_ptr(),snapshot.positions_len()),
      indices:new Uint32Array(buffer,snapshot.indices_ptr(),snapshot.indices_len()),
      faceIds:new Uint32Array(buffer,snapshot.face_ids_ptr(),snapshot.face_ids_len()),
    }
    readingCadMesh = true
    const result = read(mesh)
    if (result instanceof Promise) throw new TypeError('CAD mesh reader must be synchronous')
    return result
  } finally {
    readingCadMesh = false
    snapshot.free()
  }
}
/** wasm-bindgen copies the packed input once into Rust-owned memory. There is
 * no intermediate number array, JSON text or JSON parsing for imported meshes. */
export function importCadMesh(stride:number,vertices:Float32Array,indices:Uint32Array):number {
  initialize()
  try { return cad_import_mesh(stride,vertices,indices) } catch(error) { return cadBinaryError(error) }
}
