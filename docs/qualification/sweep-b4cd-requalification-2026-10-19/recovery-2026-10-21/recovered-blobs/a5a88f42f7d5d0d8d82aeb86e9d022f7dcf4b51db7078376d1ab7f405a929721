import {parentPort} from 'node:worker_threads'
import {diagnosticWorkerRuntime} from '../services/brepDiagnosticWorkerRuntime'
if (!parentPort) throw new Error('B-rep diagnostic worker requires a parent port')
let terminalPosted = false
const receive = diagnosticWorkerRuntime((message, transfer) => {
  parentPort!.postMessage(message, transfer)
  if (message.status !== 'started') terminalPosted = true
}, () => {
  // After a terminal the host owns terminate-and-join; avoid racing natural exit.
  if (!terminalPosted) parentPort!.close()
})
parentPort.on('message', receive)
