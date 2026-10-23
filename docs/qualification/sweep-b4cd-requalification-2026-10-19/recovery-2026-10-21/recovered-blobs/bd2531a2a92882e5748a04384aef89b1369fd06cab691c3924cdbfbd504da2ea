import {Worker} from 'node:worker_threads'
import {BrepDiagnosticExecutor, type BrepDiagnosticExecutorOptions, type BrepDiagnosticWorkerPort} from '../services/brepDiagnosticExecutor'

function nodePort(): BrepDiagnosticWorkerPort {
  const worker = new Worker(new URL('./brepDiagnostic.worker.mjs', import.meta.url), {
    stdout: true, stderr: true, env: {}, execArgv: [],
    // V8 heap/stack limits; WebAssembly linear memory and process RSS are not covered.
    resourceLimits: {maxOldGenerationSizeMb: 256, maxYoungGenerationSizeMb: 32, stackSizeMb: 8},
  })
  // No child log is retained or forwarded. Worker output cannot fill an undrained pipe.
  const discard = () => {}
  worker.stdout.on('data', discard); worker.stderr.on('data', discard)
  return {
    postMessage: value => worker.postMessage(value),
    terminate: async () => {await worker.terminate(); worker.stdout.off('data', discard); worker.stderr.off('data', discard)},
    unref: () => worker.unref(),
    listen: callbacks => {
      worker.on('message', callbacks.message); worker.on('error', callbacks.error)
      worker.on('messageerror', callbacks.error); worker.on('exit', callbacks.exit)
      return () => {worker.off('message', callbacks.message); worker.off('error', callbacks.error)
        worker.off('messageerror', callbacks.error); worker.off('exit', callbacks.exit)}
    },
  }
}
/** Internal Node diagnostic API; no MCP tool or immutable provider registration. */
export function createBrepDiagnosticSupervisor(options: BrepDiagnosticExecutorOptions = {}): BrepDiagnosticExecutor {
  return new BrepDiagnosticExecutor(nodePort, options)
}
