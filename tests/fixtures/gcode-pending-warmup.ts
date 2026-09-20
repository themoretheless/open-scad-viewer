import {workerData} from 'node:worker_threads'
import {setOptionalWasmCompiler} from '../../src/services/wasmCompilation'

const state = new Int32Array(workerData.warmState)
setOptionalWasmCompiler(async () => {
  Atomics.store(state, 0, 1)
  return new Promise<never>(() => {})
})
await import('../../src/workers/gcodePreview.worker')
