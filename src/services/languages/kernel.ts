/**
 * Host boundary for the OpenSCAD and ModelGraph frontends.
 *
 * The frontends ship as their own wasm module, so a session that never compiles source never loads
 * them and the geometry kernel stays small enough to instantiate on the main thread. The transport
 * mirrors `services/geometry/kernel`: the host owns request buffers and frees every response.
 */
import {encodeBinary} from '../valueBinaryCodec'
import {decodePacked, writeLinear} from '../wasmHost'
import {unpackBrotliWasmBase64} from '../wasmBrotliPacking'
import wasmBase64 from '../../generated/language-kernel/bytes'

interface LanguageKernelExports {
  memory: WebAssembly.Memory
  abi_alloc(len: number): number
  abi_free(ptr: number, len: number): void
  abi_request(op: number, ptr: number, len: number): bigint
}

let wasm: LanguageKernelExports
let wasmMemory: WebAssembly.Memory
let initialized = false
let warming: Promise<void> | undefined

/** True once an instance exists, so main-thread callers can avoid the synchronous compile path. */
export function isLanguageKernelReady(): boolean { return initialized }

/** Compile and instantiate asynchronously; browsers refuse the synchronous path on the main thread. */
export function warmLanguageKernel(): Promise<void> {
  if (initialized) return Promise.resolve()
  warming ??= (async () => {
    const module = await WebAssembly.compile(unpackBrotliWasmBase64(wasmBase64))
    const instance = await WebAssembly.instantiate(module)
    if (!initialized) {
      wasm = instance.exports as unknown as LanguageKernelExports
      wasmMemory = wasm.memory
      initialized = true
    }
  })().finally(() => { warming = undefined })
  return warming
}

function initialize(): void {
  if (initialized) return
  wasm = new WebAssembly.Instance(new WebAssembly.Module(unpackBrotliWasmBase64(wasmBase64)))
    .exports as unknown as LanguageKernelExports
  wasmMemory = wasm.memory
  initialized = true
}

/** One language ABI call. Operation numbers stay the ones the geometry ABI reserved for frontends. */
export function languageRequest(op: number, value: unknown): unknown {
  initialize()
  const bytes = encodeBinary(value)
  const ptr = writeLinear(wasmMemory, len => wasm.abi_alloc(len), bytes)
  if (!ptr) throw new Error('Language WASM request allocation failed')
  try { return decodePacked(wasmMemory, (pointer, size) => wasm.abi_free(pointer, size), wasm.abi_request(op, ptr, bytes.length)) }
  finally { wasm.abi_free(ptr, bytes.length) }
}

/** No JavaScript compiler fallback: both browser and Node use the same Rust frontend. */
export function compileTextRust<T>(source: string): T {
  const result = languageRequest(1,source) as {ok:boolean;value:T;message:string}
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
  return languageRequest(10,{source,profile}) as ScadRustCompileResult
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
  return languageRequest(11,{source,profile}) as ScadRustEvalResult
}

export type GraphRustResult<T> = {ok:true;value:T} | {ok:false;error:{code:string;path:string;message:string;details?:unknown};customizer?:unknown}
export function prepareGraphRust<T>(kind:'graph'|'nurbs'|'text'|'textNurbs',value:unknown):GraphRustResult<T> {
  if(kind==='text') {
    if(typeof value !== 'string' || value.length > 262144)return {ok:false,error:{code:'text_error',path:'',message:'ModelGraph Text exceeds 256 KiB.'}}
    return languageRequest(4,value) as GraphRustResult<T>
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
  return languageRequest(kind==='graph'?2:kind==='textNurbs'?5:3,value) as GraphRustResult<T>
}
