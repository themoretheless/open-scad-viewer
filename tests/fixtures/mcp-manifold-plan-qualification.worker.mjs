import { parentPort } from 'node:worker_threads'

const emptyResult = quality => ({
  meshes: [],
  warnings: [],
  volume: 0,
  surfaceArea: 0,
  quality,
  reduced: false,
  timings: { parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
})

const envelope = request => ({
  protocolVersion: request.protocolVersion,
  workerEpoch: request.workerEpoch,
  jobId: request.jobId,
  sourceSha256: request.sourceSha256,
  identity: request.identity,
})

const success = request => ({
  ...envelope(request),
  status: 'succeeded',
  result: emptyResult(request.quality),
})

const started = request => ({
  ...envelope(request),
  status: 'started',
})

const emptyMesh = () => ({
  vertices: new Float32Array(0),
  indices: new Uint32Array(0),
  bvh: {
    version: 1,
    vertexStride: 6,
    leafSize: 8,
    nodeCount: 0,
    bounds: new Float32Array(0),
    nodes: new Uint32Array(0),
    triangles: new Uint32Array(0),
  },
  edgeIndices: new Uint32Array(0),
  color: [1, 1, 1, 1],
  transform: new Float32Array(16),
  faceIds: new Uint32Array(0),
  provenance: [],
  topology: { boundary: 0, crease: 0, nonManifold: 0, degenerate: 0 },
})

parentPort.on('message', request => {
  if (request?.type !== 'evaluate') return
  if (request.source === 'startup-hang') {
    // The parent must use its startup timer because no handshake is emitted.
    while (true) { /* qualification fixture */ }
  }
  if (request.source === 'terminal-before-started') {
    parentPort.postMessage(success(request))
    return
  }
  parentPort.postMessage(started(request))
  if (request.source === 'hang') {
    // Deliberately non-cooperative: only Worker.terminate() can stop it.
    while (true) { /* qualification fixture */ }
  }
  if (request.source === 'slow-success') {
    setTimeout(() => parentPort.postMessage(success(request)), 500)
    return
  }
  if (request.source === 'noisy-success') {
    process.stdout.write(`MCP_CHILD_STDOUT_MARKER:${'o'.repeat(128 * 1024)}`)
    process.stderr.write(`MCP_CHILD_STDERR_MARKER:${'e'.repeat(128 * 1024)}`)
    parentPort.postMessage(success(request))
    return
  }
  if (request.source === 'oversized-ipc') {
    parentPort.postMessage({
      ...success(request),
      result: {
        ...emptyResult(request.quality),
        meshes: Array.from({ length: 257 }, emptyMesh),
      },
    })
    return
  }
  if (request.source === 'mutated-identity') {
    parentPort.postMessage({
      ...success(request),
      identity: { ...request.identity, engineKey: 'forged-cross-engine-key' },
    })
    return
  }
  if (request.source === 'failure') {
    parentPort.postMessage({
      ...envelope(request),
      status: 'failed',
      error: {
        name: 'FixtureKernelError',
        message: 'fixture failure',
        code: 'E_FIXTURE_KERNEL',
        line: null,
        column: null,
        start: null,
        end: null,
      },
    })
    return
  }
  if (request.source === 'double-terminal') {
    parentPort.postMessage(success(request))
    parentPort.postMessage({
      ...envelope(request),
      status: 'failed',
      error: {
        name: 'LateFixtureError',
        message: 'must be ignored',
        code: 'E_LATE_FIXTURE',
        line: null,
        column: null,
        start: null,
        end: null,
      },
    })
    return
  }
  parentPort.postMessage(success(request))
})
