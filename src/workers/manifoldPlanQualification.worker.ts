import { installGeometryKernelFromTrustedHost } from '../services/geometry/kernel'
import { isMcpManifoldPlanQualificationTerminal } from '../services/manifoldPlanQualificationProtocol'
import { manifoldPlanQualificationWorkerRuntime } from '../services/manifoldPlanQualificationWorkerRuntime'

const receive = manifoldPlanQualificationWorkerRuntime({
  post: (message, transfer) => self.postMessage(message, { transfer }),
  close: () => self.close(),
  isTerminalPublishable: isMcpManifoldPlanQualificationTerminal,
  unpublishableErrorMessage: 'Manifold plan result cannot be published by the qualification Worker',
  cloneFailureErrorMessage: 'Manifold plan result could not cross the qualification Worker boundary',
  invalidStartedHandshakeError: () => new Error('Invalid Manifold plan qualification startup acknowledgement'),
})

let bootstrap: Promise<void> | undefined
self.addEventListener('message', (event: MessageEvent<unknown>) => {
  const value = event.data
  if (value && typeof value === 'object' && 'type' in value && value.type === 'native-geometry-module') {
    if (bootstrap) throw new Error('Duplicate native geometry module bootstrap')
    const payload = value as { module: WebAssembly.Module; identity: {sha256:string;byteLength:number} }
    bootstrap = installGeometryKernelFromTrustedHost(payload.module, payload.identity)
    // Expose bootstrap failure as a Worker crash; it must never fall back to a
    // separately compiled artifact or admit the subsequent model request.
    void bootstrap.catch(error => { setTimeout(() => { throw error }, 0) })
    return
  }
  if (bootstrap) void bootstrap.then(() => receive(value), () => {})
  else receive(value)
})
