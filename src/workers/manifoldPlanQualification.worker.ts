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

self.addEventListener('message', (event: MessageEvent<unknown>) => receive(event.data))
