import {diagnosticWorkerRuntime} from '../services/brepDiagnosticWorkerRuntime'
const receive = diagnosticWorkerRuntime((message, transfer) => self.postMessage(message, {transfer}), () => self.close())
self.addEventListener('message', (event: MessageEvent<unknown>) => receive(event.data))
