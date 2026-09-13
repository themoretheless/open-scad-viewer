import {BrepDiagnosticExecutor, type BrepDiagnosticExecutorOptions, type BrepDiagnosticWorkerPort} from './brepDiagnosticExecutor'

function browserPort(): BrepDiagnosticWorkerPort {
  const worker = new Worker(new URL('../workers/brepDiagnostic.worker.ts', import.meta.url), {type: 'module'})
  return {
    postMessage: value => worker.postMessage(value),
    terminate: () => worker.terminate(),
    listen: callbacks => {
      const message = (event: MessageEvent<unknown>) => callbacks.message(event.data)
      const error = (event: ErrorEvent | MessageEvent<unknown>) => callbacks.error(event)
      worker.addEventListener('message', message)
      worker.addEventListener('error', error)
      worker.addEventListener('messageerror', error)
      return () => {worker.removeEventListener('message', message); worker.removeEventListener('error', error); worker.removeEventListener('messageerror', error)}
    },
  }
}
/** Explicit developer/diagnostic API, never an active production source route. */
export function createBrepDiagnosticWorkerLane(options: BrepDiagnosticExecutorOptions = {}): BrepDiagnosticExecutor {
  return new BrepDiagnosticExecutor(browserPort, options)
}
