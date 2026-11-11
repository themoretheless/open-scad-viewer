import {setOptionalWasmCompiler} from '../kernel/wasmCompilation'
import {compileStreamingWasm} from '../kernel/wasmStreaming'
import {createMainSolidWorkerHandler} from '../kernel/mainSolidWorkerRuntime'
import type {MainSolidProgressMessage,MainSolidResponse} from '../kernel/mainSolidProtocol'
// This browser worker composition root opts into bounded, digest-verified fetch.
// Missing or mismatched assets retain the embedded kernel fallback.
setOptionalWasmCompiler((url,identity)=>compileStreamingWasm(url,identity,true))
const post=(message:MainSolidResponse|MainSolidProgressMessage,transfer?:ArrayBuffer[])=>transfer?.length?self.postMessage(message,{transfer}):self.postMessage(message)
const handle=createMainSolidWorkerHandler(post,post)
self.addEventListener('message',event=>{void handle(event.data)})
