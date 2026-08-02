import { parentPort, workerData } from 'node:worker_threads'

if (parentPort === null) throw new Error('Qualification shadow fixture requires a parent port')
if (workerData === null || typeof workerData !== 'object'
  || !(workerData.entered instanceof SharedArrayBuffer)) {
  throw new TypeError('Qualification shadow fixture requires a shared entry latch')
}

const entered = new Int32Array(workerData.entered)

parentPort.on('message', request => {
  if (request === null || typeof request !== 'object' || request.type !== 'evaluate') return

  parentPort.postMessage({
    protocolVersion: request.protocolVersion,
    workerEpoch: request.workerEpoch,
    jobId: request.jobId,
    sourceSha256: request.sourceSha256,
    identity: request.identity,
    status: 'started',
  })

  // This output must be captured by the test-only MCP parent. If it reaches
  // process.stdout it corrupts the newline-delimited JSON-RPC transport.
  process.stdout.write('qualification-worker-private-stdout\n')
  Atomics.store(entered, 0, 1)
  Atomics.notify(entered, 0)

  // Deliberately non-cooperative. It cannot process the cancel command and
  // can be reclaimed only through Worker.terminate() + its resolved join.
  while (true) { /* qualification fixture */ }
})
