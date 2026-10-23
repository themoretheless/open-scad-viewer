import {setOptionalWasmCompiler} from '../services/wasmCompilation'
import {compileStreamingWasm} from '../services/wasmStreaming'
import {createMainSolidWorkerHandler} from '../services/mainSolidWorkerRuntime'
// This browser worker composition root opts into bounded, digest-verified fetch.
// Missing or mismatched assets retain the embedded kernel fallback.
setOptionalWasmCompiler((url,identity)=>compileStreamingWasm(url,identity,true))
const handle=createMainSolidWorkerHandler((message,transfer)=>transfer?.length?self.postMessage(message,{transfer}):self.postMessage(message))
self.addEventListener('message',event=>{void handle(event.data)})
