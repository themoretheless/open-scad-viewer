import { BrepDiagnosticExecutor, type BrepDiagnosticExecutorOptions, type BrepDiagnosticWorkerPort } from './brepDiagnosticExecutor'

function browserPort(): BrepDiagnosticWorkerPort {
  const worker = new Worker(new URL('../workers/brepDiagnostic.worker.ts', import.meta.url), { type: 'module' })
  return {
    postMessage: value => worker.postMessage(value),
    listen(callbacks) {
      const message = (event: MessageEvent<unknown>) => callbacks.message(event.data)
      const failure = (event: Event) => callbacks.error(event)
      worker.addEventListener('message', message)
      worker.addEventListener('error', failure)
      worker.addEventListener('messageerror', failure)
      return () => {
        worker.removeEventListener('message', message)
        worker.removeEventListener('error', failure)
        worker.removeEventListener('messageerror', failure)
      }
    },
    terminate: () => worker.terminate(),
  }
}
/** Owned disposable browser realm; diagnostic receipts are not production providers. */
export function createBrowserBrepDiagnosticExecutor(options: BrepDiagnosticExecutorOptions = {}): BrepDiagnosticExecutor {
  return new BrepDiagnosticExecutor(browserPort, options)
}
