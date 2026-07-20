import { describe, expect, it } from 'vitest'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  isGeometryWorkerEvent,
  isGeometryWorkerRequest,
  type GeometryWorkerEvent,
} from '../src/services/geometryWorkerProtocol'
import { parseOpenSCAD, type MeshData } from '../src/services/openscadParser'

const envelope = {
  protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
  documentRevision: 3,
  jobId: 7,
  quality: 'preview' as const,
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

function succeeded(): GeometryWorkerEvent {
  return {
    ...envelope,
    status: 'succeeded',
    phase: 'complete',
    meshes: [validMesh()],
    warnings: [],
    volume: 0,
    surfaceArea: 0,
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
  })

  it('rejects negative identifiers in requests and events', () => {
    expect(isGeometryWorkerRequest({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: -1,
      jobId: 1,
      source: 'cube(1);',
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

  it('accepts the complete payload produced by the geometry compiler', async () => {
    const result = await parseOpenSCAD('cube(1);', { quality: 'preview' })
    expect(isGeometryWorkerEvent({
      ...envelope,
      status: 'succeeded',
      phase: 'complete',
      ...result,
      durationMs: 1,
    })).toBe(true)
  })
})
