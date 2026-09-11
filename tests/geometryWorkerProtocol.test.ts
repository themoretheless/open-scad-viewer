import { describe, expect, it } from 'vitest'
import {
  GEOMETRY_MANIFEST_ARCHIVE,
  LEGACY_MANIFOLD_EXECUTION,
} from '../src/core/geometryExecution'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  GEOMETRY_WORKER_PAYLOAD_LIMITS,
  isGeometryWorkerEvent,
  isGeometryWorkerRequest,
  type GeometryWorkerEvent,
} from '../src/services/geometryWorkerProtocol'
import type { MeshData } from '../src/core/mesh'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { sha256Hex } from '../src/core/sha256'

const envelope = {
  protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
  documentRevision: 3,
  jobId: 7,
  quality: 'preview' as const,
  sourceSha256: '0'.repeat(64),
}

function validMesh(): MeshData {
  return {
    entityId: 'entity:test',
    vertices: new Float32Array(0),
    indices: new Uint32Array(0),
    edgeIndices: new Uint32Array(0),
    faceIds: new Uint32Array(0),
    color: [1, 0.5, 0.25, 1],
    transform: new Float32Array(16),
    bvh: {
      version: 1,
      vertexStride: 6,
      leafSize: 8,
      nodeCount: 0,
      bounds: new Float32Array(0),
      nodes: new Uint32Array(0),
      triangles: new Uint32Array(0),
    },
    provenance: [],
    topology: { boundary: 0, crease: 0, nonManifold: 0, degenerate: 0 },
  }
}

function triangleMesh(): MeshData {
  return {
    entityId: 'entity:triangle',
    vertices: new Float32Array([
      0, 0, 0, 0, 0, 1,
      1, 0, 0, 0, 0, 1,
      0, 1, 0, 0, 0, 1,
    ]),
    indices: new Uint32Array([0, 1, 2]),
    edgeIndices: new Uint32Array([0, 1, 1, 2, 2, 0]),
    faceIds: new Uint32Array([0]),
    color: [1, 0.5, 0.25, 1],
    transform: new Float32Array([
      1, 0, 0, 0,
      0, 1, 0, 0,
      0, 0, 1, 0,
      0, 0, 0, 1,
    ]),
    bvh: {
      version: 1,
      vertexStride: 6,
      leafSize: 8,
      nodeCount: 1,
      bounds: new Float32Array([0, 0, 0, 1, 1, 0]),
      nodes: new Uint32Array([0, 0x80000001]),
      triangles: new Uint32Array([0]),
    },
    provenance: [],
    topology: { boundary: 3, crease: 0, nonManifold: 0, degenerate: 0 },
  }
}

function twoTriangleMesh(): MeshData {
  const mesh = triangleMesh()
  return {
    ...mesh,
    vertices: new Float32Array([
      0, 0, 0, 0, 0, 1,
      1, 0, 0, 0, 0, 1,
      0, 1, 0, 0, 0, 1,
      1, 1, 0, 0, 0, 1,
    ]),
    indices: new Uint32Array([0, 1, 2, 1, 3, 2]),
    edgeIndices: new Uint32Array([0, 1, 1, 3, 3, 2, 2, 0]),
    faceIds: new Uint32Array([0, 1]),
    bvh: {
      ...mesh.bvh,
      nodeCount: 3,
      bounds: new Float32Array([
        0, 0, 0, 1, 1, 0,
        0, 0, 0, 1, 1, 0,
        0, 0, 0, 1, 1, 0,
      ]),
      nodes: new Uint32Array([
        1, 2,
        0, 0x80000001,
        1, 0x80000001,
      ]),
      triangles: new Uint32Array([0, 1]),
    },
  }
}

function succeeded(): GeometryWorkerEvent {
  return {
    ...envelope,
    status: 'succeeded',
    phase: 'complete',
    execution: {
      ...LEGACY_MANIFOLD_EXECUTION,
      purpose: 'preview',
      quality: 'preview',
      evidence: 'runtime',
      effectiveLimits: { sourceCharacters: 250_000, triangles: 750_000 },
    },
    meshes: [validMesh()],
    warnings: [],
    volume: 0,
    surfaceArea: 0,
    reduced: false,
    timings: { parseMs: 0.1, bindMs: 0, initializeMs: 0.2, evaluateMs: 0.3, analyzeMs: 0.4 },
    durationMs: 1,
  }
}

describe('geometry worker protocol validation', () => {
  it('validates the status-specific phase contract', () => {
    expect(isGeometryWorkerEvent({ ...envelope, status: 'accepted', phase: 'queued' })).toBe(true)
    expect(isGeometryWorkerEvent({ ...envelope, status: 'accepted', phase: 'compiling' })).toBe(false)
    expect(isGeometryWorkerEvent({ ...envelope, status: 'started', phase: 'initializing' })).toBe(true)
    expect(isGeometryWorkerEvent({ ...envelope, status: 'started', phase: 'compiling' })).toBe(true)
    expect(isGeometryWorkerEvent({ ...envelope, status: 'started', phase: 'queued' })).toBe(false)
    expect(isGeometryWorkerEvent(succeeded())).toBe(true)
    expect(isGeometryWorkerEvent({ ...succeeded(), phase: 'serializing' })).toBe(false)
  })

  it('validates every status payload instead of accepting its envelope alone', () => {
    expect(isGeometryWorkerEvent({ ...envelope, status: 'progress', phase: 'compiling', progress: null })).toBe(true)
    expect(isGeometryWorkerEvent({ ...envelope, status: 'progress', phase: 'serializing', progress: 0.75 })).toBe(true)
    expect(isGeometryWorkerEvent({ ...envelope, status: 'progress', phase: 'compiling', progress: 1.01 })).toBe(false)
    expect(isGeometryWorkerEvent({ ...envelope, status: 'progress', phase: 'compiling' })).toBe(false)

    expect(isGeometryWorkerEvent({
      ...envelope,
      status: 'failed',
      phase: 'compiling',
      error: { name: 'Error', message: 'bad source', line: 1, column: 2 },
      durationMs: 2,
    })).toBe(true)
    expect(isGeometryWorkerEvent({ ...envelope, status: 'failed', phase: 'compiling', durationMs: 2 })).toBe(false)

    expect(isGeometryWorkerEvent({
      ...envelope,
      status: 'cancelled',
      phase: 'compiling',
      reason: 'superseded',
      durationMs: 2,
    })).toBe(true)
    expect(isGeometryWorkerEvent({
      ...envelope,
      status: 'cancelled',
      phase: 'compiling',
      reason: 'maybe',
      durationMs: 2,
    })).toBe(false)

    expect(isGeometryWorkerEvent({
      ...envelope,
      status: 'stale',
      phase: 'complete',
      supersededBy: { documentRevision: 4, jobId: 8 },
      durationMs: 2,
    })).toBe(true)
    expect(isGeometryWorkerEvent({
      ...envelope,
      status: 'stale',
      phase: 'complete',
      supersededBy: { documentRevision: -1, jobId: 8 },
      durationMs: 2,
    })).toBe(false)

    expect(isGeometryWorkerEvent({ ...succeeded(), warnings: [42] })).toBe(false)
    expect(isGeometryWorkerEvent({ ...succeeded(), meshes: [{}] })).toBe(false)
    expect(isGeometryWorkerEvent({ ...succeeded(), durationMs: -1 })).toBe(false)
    expect(isGeometryWorkerEvent({ ...succeeded(), reduced: undefined })).toBe(false)
    expect(isGeometryWorkerEvent({ ...succeeded(), reduced: 1 })).toBe(false)
    expect(isGeometryWorkerEvent({ ...succeeded(), reduced: true })).toBe(true)
    expect(isGeometryWorkerEvent({ ...succeeded(), timings: undefined })).toBe(false)
    expect(isGeometryWorkerEvent({ ...succeeded(), timings: { parseMs: -1, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 } })).toBe(false)
    expect(isGeometryWorkerEvent({ ...succeeded(), timings: { parseMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 } })).toBe(false)
  })

  it('requires coherent runtime engine provenance on successful publications', () => {
    expect(isGeometryWorkerEvent({ ...succeeded(), execution: undefined })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      execution: {
        ...LEGACY_MANIFOLD_EXECUTION,
        languageContract: 'openscad-viewer/brep-1',
        engineClass: 'manifold',
        quality: 'preview',
        evidence: 'runtime',
        effectiveLimits: { sourceCharacters: 250_000, triangles: 750_000 },
      },
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      execution: {
        languageContract: 'openscad-viewer/brep-1',
        requiredCapabilities: [],
        engineClass: 'brep',
        engineKey: 'rust-brep-reserved-v1',
        kernelFingerprint: 'not-deployed',
        semanticProgramVersion: 'semantic-program-contract-v1',
        capabilityManifestVersion: 'brep-contract-v1',
        manifestDigest: GEOMETRY_MANIFEST_ARCHIVE['brep-contract-v1'].manifestDigest,
        purpose: 'preview',
        quality: 'preview',
        representation: 'brep',
        evidence: 'runtime',
        effectiveLimits: { sourceCharacters: 250_000 },
        automaticFallback: false,
      },
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      execution: { ...succeeded().execution!, engineKey: 'forged-manifold' },
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      execution: { ...succeeded().execution!, manifestDigest: '0'.repeat(64) },
    })).toBe(false)
  })

  it('accepts planned B-rep provenance on an unavailable-engine failure', () => {
    expect(isGeometryWorkerEvent({
      ...envelope,
      status: 'failed',
      phase: 'compiling',
      execution: {
        languageContract: 'openscad-viewer/brep-1',
        requiredCapabilities: ['geometry.brep'],
        engineClass: 'brep',
        engineKey: 'rust-brep-reserved-v1',
        kernelFingerprint: 'not-deployed',
        semanticProgramVersion: 'semantic-program-contract-v1',
        capabilityManifestVersion: 'brep-contract-v1',
        manifestDigest: GEOMETRY_MANIFEST_ARCHIVE['brep-contract-v1'].manifestDigest,
        purpose: 'preview',
        quality: 'preview',
        representation: 'brep',
        evidence: 'planned',
        effectiveLimits: { sourceCharacters: 250_000 },
        automaticFallback: false,
      },
      error: { name: 'GeometryEngineUnavailableError', message: 'not deployed' },
      durationMs: 1,
    })).toBe(true)
  })

  it('rejects negative identifiers in requests and events', () => {
    expect(isGeometryWorkerRequest({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: -1,
      jobId: 1,
      source: 'cube(1);',
      sourceSha256: '5748fc5442d26f3407c2be861219e61233204258340374390ddc01c0ada7b67d',
      quality: 'preview',
    })).toBe(false)
    expect(isGeometryWorkerRequest({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'cancel',
      documentRevision: 1,
      jobId: -1,
      reason: 'user',
    })).toBe(false)
    expect(isGeometryWorkerEvent({ ...succeeded(), jobId: -1 })).toBe(false)
    expect(isGeometryWorkerEvent({ ...succeeded(), documentRevision: -1 })).toBe(false)
  })

  it('attests exact bounded request source and bounds terminal diagnostics', () => {
    const source = 'cube(1);'
    const request = {
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build' as const,
      documentRevision: 1,
      jobId: 1,
      source,
      sourceSha256: sha256Hex(source),
      quality: 'preview' as const,
    }
    expect(isGeometryWorkerRequest(request)).toBe(true)
    for (const forbidden of ['engine', 'engine_class', 'backend', 'kernel', 'provider', 'automatic_fallback', 'programHash', 'semanticProgram', 'workerEpoch']) {
      expect(isGeometryWorkerRequest({ ...request, [forbidden]: forbidden === 'automatic_fallback' ? false : 'smuggled' })).toBe(false)
    }
    expect(isGeometryWorkerRequest({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'cancel',
      documentRevision: 1,
      jobId: 1,
      reason: 'user',
      engine: 'brep',
    })).toBe(false)
    expect(isGeometryWorkerRequest({ ...request, source: 'sphere(1);' })).toBe(false)
    const oversized = 'x'.repeat(250_001)
    expect(isGeometryWorkerRequest({
      ...request,
      source: oversized,
      sourceSha256: sha256Hex(oversized),
    })).toBe(false)

    const failure = {
      ...envelope,
      status: 'failed' as const,
      phase: 'compiling' as const,
      durationMs: 1,
      error: { name: 'Error', message: 'bounded' },
    }
    expect(isGeometryWorkerEvent(failure)).toBe(true)
    expect(isGeometryWorkerEvent({ ...failure, programHash: '0'.repeat(64) })).toBe(false)
    expect(isGeometryWorkerEvent({ ...succeeded(), engine: 'brep' })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...failure,
      error: { name: 'Error', message: 'bounded', provider: 'brep' },
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...failure,
      error: { name: 'x'.repeat(GEOMETRY_WORKER_PAYLOAD_LIMITS.errorNameCharacters + 1), message: '' },
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...failure,
      error: { name: 'Error', message: 'x'.repeat(GEOMETRY_WORKER_PAYLOAD_LIMITS.errorMessageCharacters + 1) },
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...failure,
      error: { name: 'Error', message: 'partial span', start: 0 },
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...failure,
      error: { name: 'Error', message: 'partial span', end: 1 },
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...failure,
      error: { name: 'Error', message: 'complete span', start: 0, end: 1 },
    })).toBe(true)
  })

  it('rejects duplicate mesh entity identities and oversized provenance labels', () => {
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [validMesh(), validMesh()],
    })).toBe(false)
    const mesh = triangleMesh()
    mesh.provenance = [{
      triangleStart: 0,
      triangleEnd: 1,
      backside: false,
      source: {
        id: 1,
        originalId: 1,
        start: 0,
        end: 1,
        label: 'x'.repeat(GEOMETRY_WORKER_PAYLOAD_LIMITS.sourceLabelCharacters + 1),
      },
    }]
    expect(isGeometryWorkerEvent({ ...succeeded(), meshes: [mesh] })).toBe(false)
  })

  it('requires non-empty sorted non-overlapping provenance runs', () => {
    const source = (id: number) => ({ id, originalId: id, start: id, end: id + 1, label: `run-${id}` })
    const mesh = twoTriangleMesh()
    mesh.provenance = [
      { triangleStart: 0, triangleEnd: 1, backside: false, source: source(0) },
      { triangleStart: 1, triangleEnd: 2, backside: false, source: source(1) },
    ]
    expect(isGeometryWorkerEvent({ ...succeeded(), meshes: [mesh] })).toBe(true)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...mesh, provenance: [mesh.provenance[1]] }],
    })).toBe(true)

    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...mesh, provenance: [...mesh.provenance].reverse() }],
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...mesh, provenance: [
        { ...mesh.provenance[0], triangleEnd: 2 },
        mesh.provenance[1],
      ] }],
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...mesh, provenance: [{
        ...mesh.provenance[0], triangleStart: 1, triangleEnd: 1,
      }] }],
    })).toBe(false)
  })

  it('rejects color components outside the normalized RGBA contract', () => {
    expect(isGeometryWorkerEvent({ ...succeeded(), meshes: [{ ...validMesh(), color: [1, 1, 1, -0.01] }] })).toBe(false)
    expect(isGeometryWorkerEvent({ ...succeeded(), meshes: [{ ...validMesh(), color: [1, 1, 1, 1.01] }] })).toBe(false)
  })

  it('rejects non-finite geometry and transforms at the Worker boundary', () => {
    const badVertex = triangleMesh()
    badVertex.vertices[0] = Number.NaN
    expect(isGeometryWorkerEvent({ ...succeeded(), meshes: [badVertex] })).toBe(false)
    const badTransform = triangleMesh()
    badTransform.transform[3] = Number.POSITIVE_INFINITY
    expect(isGeometryWorkerEvent({ ...succeeded(), meshes: [badTransform] })).toBe(false)
  })

  it('accepts the complete payload produced by the geometry compiler', async () => {
    const result = await parseOpenSCAD('cube(1);', { quality: 'preview' })
    expect(isGeometryWorkerEvent({
      ...envelope,
      status: 'succeeded',
      phase: 'complete',
      ...result,
      execution: {
        ...LEGACY_MANIFOLD_EXECUTION,
        purpose: 'preview',
        quality: 'preview',
        evidence: 'runtime',
        effectiveLimits: { sourceCharacters: 250_000, triangles: 750_000 },
      },
      durationMs: 1,
    })).toBe(true)
  })

  it('rejects mesh indices and BVH triangle references outside their source buffers', () => {
    const mesh = triangleMesh()
    expect(isGeometryWorkerEvent({ ...succeeded(), meshes: [mesh] })).toBe(true)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...mesh, indices: new Uint32Array([0, 1, 3]) }],
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...mesh, edgeIndices: new Uint32Array([0, 3]) }],
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...mesh, bvh: { ...mesh.bvh, triangles: new Uint32Array([1]) } }],
    })).toBe(false)
  })

  it('rejects incompatible strides, invalid leaf ranges and non-finite bounds', () => {
    const mesh = triangleMesh()
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...mesh, bvh: { ...mesh.bvh, vertexStride: 3 } }],
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...mesh, bvh: { ...mesh.bvh, nodes: new Uint32Array([1, 0x80000001]) } }],
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...mesh, bvh: { ...mesh.bvh, bounds: new Float32Array([0, 0, 0, Infinity, 1, 0]) } }],
    })).toBe(false)
  })

  it('rejects cyclic, shared-child and unreachable BVH graphs', () => {
    const single = triangleMesh()
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...single, bvh: { ...single.bvh, nodes: new Uint32Array([0, 0]) } }],
    })).toBe(false)

    const pair = twoTriangleMesh()
    expect(isGeometryWorkerEvent({ ...succeeded(), meshes: [pair] })).toBe(true)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...pair, bvh: { ...pair.bvh, nodes: new Uint32Array([
        1, 1,
        0, 0x80000001,
        1, 0x80000001,
      ]) } }],
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...pair, bvh: { ...pair.bvh, nodes: new Uint32Array([
        0, 0x80000002,
        0, 0x80000001,
        1, 0x80000001,
      ]) } }],
    })).toBe(false)
  })

  it('rejects oversized success envelopes before deep payload validation', () => {
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: Array.from({ length: GEOMETRY_WORKER_PAYLOAD_LIMITS.meshes + 1 }, validMesh),
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      warnings: Array.from({ length: GEOMETRY_WORKER_PAYLOAD_LIMITS.warnings + 1 }, () => 'warning'),
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      warnings: ['x'.repeat(GEOMETRY_WORKER_PAYLOAD_LIMITS.warningCharacters + 1)],
    })).toBe(false)

    const oversized = validMesh()
    oversized.indices = new Uint32Array((GEOMETRY_WORKER_PAYLOAD_LIMITS.triangles + 1) * 3)
    expect(isGeometryWorkerEvent({ ...succeeded(), meshes: [oversized] })).toBe(false)
  })

  it('requires exact enumerable data properties and plain records at every wire boundary', () => {
    const hidden = succeeded() as Record<PropertyKey, unknown>
    Object.defineProperty(hidden, 'hiddenEngine', { value: 'brep', enumerable: false })
    expect(isGeometryWorkerEvent(hidden)).toBe(false)

    const symbolField = succeeded() as Record<PropertyKey, unknown>
    symbolField[Symbol('hidden')] = true
    expect(isGeometryWorkerEvent(symbolField)).toBe(false)

    let getterReads = 0
    const accessor = succeeded() as Record<PropertyKey, unknown>
    Object.defineProperty(accessor, 'durationMs', {
      enumerable: true,
      get() {
        getterReads++
        return getterReads === 1 ? 1 : -1
      },
    })
    expect(isGeometryWorkerEvent(accessor)).toBe(false)
    expect(getterReads).toBe(0)

    const surprisingPrototype = succeeded()
    Object.setPrototypeOf(surprisingPrototype, { engine: 'brep' })
    expect(isGeometryWorkerEvent(surprisingPrototype)).toBe(false)
  })

  it('checks array cardinality before density scans and rejects non-owned buffers', () => {
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      warnings: new Array(GEOMETRY_WORKER_PAYLOAD_LIMITS.warnings + 1),
    })).toBe(false)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      execution: {
        ...succeeded().execution!,
        requiredCapabilities: new Array(1_000_000),
      },
    })).toBe(false)

    const oversizedBacking = new ArrayBuffer(8)
    expect(isGeometryWorkerEvent({
      ...succeeded(),
      meshes: [{ ...validMesh(), vertices: new Float32Array(oversizedBacking, 0, 1) }],
    })).toBe(false)

    if (typeof SharedArrayBuffer !== 'undefined') {
      const shared = new SharedArrayBuffer(0)
      expect(isGeometryWorkerEvent({
        ...succeeded(),
        meshes: [{ ...validMesh(), vertices: new Float32Array(shared) }],
      })).toBe(false)
    }

    const aliased = triangleMesh()
    const sharedIndices = new Uint32Array([0])
    aliased.faceIds = sharedIndices
    aliased.bvh = { ...aliased.bvh, triangles: sharedIndices }
    expect(isGeometryWorkerEvent({ ...succeeded(), meshes: [aliased] })).toBe(false)
  })
})
