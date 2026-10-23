import { parentPort } from 'node:worker_threads'
import { isMcpManifoldPlanQualificationNodeTerminal } from '../services/manifoldPlanQualificationProtocol'
import { manifoldPlanQualificationWorkerRuntime } from '../services/manifoldPlanQualificationWorkerRuntime'

if (parentPort === null) throw new Error('Manifold qualification worker requires a parent port')

const receive = manifoldPlanQualificationWorkerRuntime({
  post: (message, transfer) => parentPort!.postMessage(message, transfer),
  close: () => parentPort!.close(),
  isTerminalPublishable: isMcpManifoldPlanQualificationNodeTerminal,
  unpublishableErrorMessage: 'Manifold plan result cannot be published by the qualification child',
  cloneFailureErrorMessage: null,
  invalidStartedHandshakeError: () => new TypeError('Invalid Manifold qualification started handshake'),
})

parentPort.on('message', receive)
