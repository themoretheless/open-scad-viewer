import { describe, expect, it } from 'vitest'
import {
  GEOMETRY_ENGINE_ROUTES,
  GEOMETRY_MANIFEST_ARCHIVE,
  planGeometrySourceExecution,
  type GeometryEngineRegistrySnapshot,
  type GeometryExecutionDescriptor,
} from '../src/core/geometryExecution'
import { sha256Hex } from '../src/core/sha256'
import { geometryExecutionForError } from '../src/services/geometryBuildEngine'
import { GEOMETRY_WORKER_PAYLOAD_LIMITS } from '../src/services/geometryWorkerProtocol'
import { OpenSCADParseError } from '../src/services/openscadErrors'
import {
  DIRECT_GEOMETRY_IDENTITY,
  DIRECT_GEOMETRY_PROTOCOL_VERSION,
  isDirectGeometryBuildRequest,
  isDirectGeometryCancel,
  isDirectGeometryCapabilities,
  isDirectGeometryCapabilitiesRequest,
  isDirectGeometryNodeTerminal,
  isDirectGeometryStarted,
  reconstructDirectGeometryRemoteError,
  serializeDirectGeometryError,
  type DirectGeometryBuildRequest,
  type DirectGeometryCapabilitiesRequest,
  type DirectGeometryTerminal,
} from '../src/mcp/directGeometryProtocol'

const source = 'cube([1, 2, 3]);'

function buildRequest(): DirectGeometryBuildRequest {
  return {
    protocolVersion: DIRECT_GEOMETRY_PROTOCOL_VERSION,
    type: 'build',
    workerEpoch: 7,
    jobId: 11,
    source,
    sourceSha256: sha256Hex(source),
    quality: 'preview',
    purpose: 'analysis',
    identity: DIRECT_GEOMETRY_IDENTITY,
  }
}

function capabilitiesRequest(): DirectGeometryCapabilitiesRequest {
  return {
    protocolVersion: DIRECT_GEOMETRY_PROTOCOL_VERSION,
    type: 'capabilities',
    workerEpoch: 8,
    jobId: 12,
    sourceSha256: null,
    quality: null,
    purpose: null,
    identity: DIRECT_GEOMETRY_IDENTITY,
  }
}

function runtimeExecution(request: DirectGeometryBuildRequest): GeometryExecutionDescriptor {
  return {
    ...planGeometrySourceExecution(request.source, {
      quality: request.quality,
      purpose: request.purpose,
    }),
    evidence: 'runtime',
  }
}

function evaluationResult(request: DirectGeometryBuildRequest) {
  return {
    meshes: [],
    warnings: [],
    volume: 0,
    surfaceArea: 0,
    quality: request.quality,
    reduced: false,
    timings: { parseMs: 1, initializeMs: 2, evaluateMs: 3, analyzeMs: 4 },
  }
}

function buildSuccess(request: DirectGeometryBuildRequest): DirectGeometryTerminal {
  return {
    protocolVersion: DIRECT_GEOMETRY_PROTOCOL_VERSION,
    workerEpoch: request.workerEpoch,
    jobId: request.jobId,
    sourceSha256: request.sourceSha256,
    quality: request.quality,
    purpose: request.purpose,
    identity: DIRECT_GEOMETRY_IDENTITY,
    status: 'succeeded',
    kind: 'build',
    built: { result: evaluationResult(request), execution: runtimeExecution(request) },
  }
}

function capabilities(): GeometryEngineRegistrySnapshot {
  return {
    contractVersion: 1,
    sourceDirectedRouting: true,
    automaticFallback: false,
    routes: GEOMETRY_ENGINE_ROUTES.map(route => ({ ...route })),
    engines: [
      {
        ...GEOMETRY_MANIFEST_ARCHIVE['manifold-node-v1'],
        availability: 'available',
        unavailableReason: null,
      },
      {
        ...GEOMETRY_MANIFEST_ARCHIVE['brep-contract-v1'],
        availability: 'unavailable',
        unavailableReason: 'The B-rep provider is not deployed.',
      },
    ],
  }
}

describe('direct production geometry protocol', () => {
  it('accepts only exact requests and correlated cancellation', () => {
    const build = buildRequest()
    const discovery = capabilitiesRequest()
    expect(isDirectGeometryBuildRequest(build)).toBe(true)
    expect(isDirectGeometryCapabilitiesRequest(discovery)).toBe(true)
    expect(isDirectGeometryBuildRequest({ ...build, sourceSha256: '0'.repeat(64) })).toBe(false)
    expect(isDirectGeometryBuildRequest({ ...build, engine: 'brep' })).toBe(false)
    expect(isDirectGeometryCapabilitiesRequest({ ...discovery, quality: 'full' })).toBe(false)

    const cancel = {
      protocolVersion: DIRECT_GEOMETRY_PROTOCOL_VERSION,
      type: 'cancel',
      workerEpoch: build.workerEpoch,
      jobId: build.jobId,
      sourceSha256: build.sourceSha256,
      quality: build.quality,
      purpose: build.purpose,
      reason: 'deadline',
      identity: DIRECT_GEOMETRY_IDENTITY,
    }
    expect(isDirectGeometryCancel(cancel, build)).toBe(true)
    expect(isDirectGeometryCancel({ ...cancel, purpose: 'export' }, build)).toBe(false)
    expect(isDirectGeometryCancel({ ...cancel, workerEpoch: 9 }, build)).toBe(false)
  })

  it('binds started and success to every correlation field', () => {
    const request = buildRequest()
    const started = {
      protocolVersion: DIRECT_GEOMETRY_PROTOCOL_VERSION,
      workerEpoch: request.workerEpoch,
      jobId: request.jobId,
      sourceSha256: request.sourceSha256,
      quality: request.quality,
      purpose: request.purpose,
      identity: DIRECT_GEOMETRY_IDENTITY,
      status: 'started',
      kind: 'build',
    }
    expect(isDirectGeometryStarted(started, request)).toBe(true)
    expect(isDirectGeometryStarted({ ...started, purpose: 'export' }, request)).toBe(false)
    expect(isDirectGeometryStarted({ ...started, extra: true }, request)).toBe(false)

    const terminal = buildSuccess(request)
    expect(isDirectGeometryNodeTerminal(terminal, request)).toBe(true)
    expect(isDirectGeometryNodeTerminal({ ...terminal, jobId: request.jobId + 1 }, request)).toBe(false)
    const success = terminal as Extract<
      DirectGeometryTerminal,
      { kind: 'build'; status: 'succeeded' }
    >
    expect(isDirectGeometryNodeTerminal({
      ...success,
      built: { ...success.built, execution: { ...success.built.execution, purpose: 'export' } },
    }, request)).toBe(false)
    expect(isDirectGeometryNodeTerminal({
      ...success,
      built: {
        ...success.built,
        execution: {
          ...success.built.execution,
          requiredCapabilities: Array.from({ length: 257 }, (_, index) => `cap.${index}`),
        },
      },
    }, request)).toBe(false)
    expect(isDirectGeometryNodeTerminal({
      ...success,
      built: {
        ...success.built,
        execution: {
          ...success.built.execution,
          effectiveLimits: { ...success.built.execution.effectiveLimits, injected: 1 },
        },
      },
    }, request)).toBe(false)
  })

  it('identifies the frozen production legacy-direct path without qualification claims', () => {
    expect(DIRECT_GEOMETRY_IDENTITY).toMatchObject({
      executionPath: 'legacy-direct-production',
      engineKey: GEOMETRY_MANIFEST_ARCHIVE['manifold-node-v1'].engineKey,
      manifestDigest: GEOMETRY_MANIFEST_ARCHIVE['manifold-node-v1'].manifestDigest,
      manifestIsolation: GEOMETRY_MANIFEST_ARCHIVE['manifold-node-v1'].isolation,
      hostIsolation: 'disposable-node-worker-per-job',
      automaticFallback: false,
    })
    expect(Object.hasOwn(DIRECT_GEOMETRY_IDENTITY, 'qualificationOnly')).toBe(false)
    expect(Object.isFrozen(DIRECT_GEOMETRY_IDENTITY)).toBe(true)
  })

  it('validates bounded capabilities against both immutable manifests', () => {
    const request = capabilitiesRequest()
    const snapshot = capabilities()
    expect(isDirectGeometryCapabilities(snapshot)).toBe(true)
    expect(isDirectGeometryNodeTerminal({
      protocolVersion: DIRECT_GEOMETRY_PROTOCOL_VERSION,
      workerEpoch: request.workerEpoch,
      jobId: request.jobId,
      sourceSha256: null,
      quality: null,
      purpose: null,
      identity: DIRECT_GEOMETRY_IDENTITY,
      status: 'succeeded',
      kind: 'capabilities',
      capabilities: snapshot,
    }, request)).toBe(true)
    expect(isDirectGeometryCapabilities({
      ...snapshot,
      engines: snapshot.engines.map((engine, index) => index === 0
        ? { ...engine, manifestDigest: '0'.repeat(64) }
        : engine),
    })).toBe(false)
  })

  it('bounds build payloads and sends parse details without source excerpts', () => {
    const request = buildRequest()
    const success = buildSuccess(request) as Extract<
      DirectGeometryTerminal,
      { kind: 'build'; status: 'succeeded' }
    >
    expect(isDirectGeometryNodeTerminal({
      ...success,
      built: {
        ...success.built,
        result: {
          ...success.built.result,
          warnings: ['x'.repeat(GEOMETRY_WORKER_PAYLOAD_LIMITS.warningCharacters + 1)],
        },
      },
    }, request)).toBe(false)

    const parseError = new OpenSCADParseError(
      'secret_model_name();',
      0,
      'Unsupported operation',
    )
    const serialized = serializeDirectGeometryError(parseError)
    expect(serialized).toMatchObject({
      category: 'openscad-parse',
      name: 'OpenSCADParseError',
      message: 'Unsupported operation',
      line: 1,
      column: 1,
    })
    expect(serialized.message).not.toContain('secret_model_name')
    expect(serialized.message).not.toContain('\n')

    const execution = runtimeExecution(request)
    const failure = {
      protocolVersion: DIRECT_GEOMETRY_PROTOCOL_VERSION,
      workerEpoch: request.workerEpoch,
      jobId: request.jobId,
      sourceSha256: request.sourceSha256,
      quality: request.quality,
      purpose: request.purpose,
      identity: DIRECT_GEOMETRY_IDENTITY,
      status: 'failed',
      kind: 'build',
      execution,
      error: serialized,
    }
    expect(isDirectGeometryNodeTerminal(failure, request)).toBe(true)
    expect(isDirectGeometryNodeTerminal({
      ...failure,
      error: {
        ...serialized,
        start: request.source.length + 1,
        end: request.source.length + 2,
      },
    }, request)).toBe(false)
    const reconstructed = reconstructDirectGeometryRemoteError(serialized, request.source, execution)
    expect(reconstructed).toBeInstanceOf(OpenSCADParseError)
    expect(geometryExecutionForError(reconstructed)).toEqual(execution)
  })
})
