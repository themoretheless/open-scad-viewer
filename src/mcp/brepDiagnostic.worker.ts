import {parentPort} from 'node:worker_threads'
import {diagnosticWorkerRuntime} from '../services/brepDiagnosticWorkerRuntime'
if (!parentPort) throw new Error('B-rep diagnostic worker requires a parent port')
const receive = diagnosticWorkerRuntime((message, transfer) => parentPort!.postMessage(message, transfer), () => parentPort!.close())
parentPort.on('message', receive)
