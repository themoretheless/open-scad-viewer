import type { GeometryEvaluationResult } from '../../src/core/build'
import {
  meshTransferables,
  type MeshData,
  type SceneEntityId,
  type SourceOperationId,
} from '../../src/core/mesh'
import {
  MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
  MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
  isMcpManifoldPlanQualificationRequest,
  isMcpManifoldPlanQualificationStarted,
  isMcpManifoldPlanQualificationTerminal,
  type McpManifoldPlanQualificationRequest,
  type McpManifoldPlanQualificationTerminal,
} from '../../src/services/manifoldPlanQualificationProtocol'

function envelope(request: McpManifoldPlanQualificationRequest) {
  return {
    protocolVersion: MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
    workerEpoch: request.workerEpoch,
    jobId: request.jobId,
    sourceSha256: request.sourceSha256,
    identity: MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
  } as const
}

type ControlledIdentityField = 'entityId' | 'instanceId' | 'operationId'

// The source is only a fixture selector. Identity text is constructed below,
// independently of source/lowering, and exactly one field receives the named
// UTF-16 length while both companion identities remain short constants.
const CONTROLLED_IDENTITY_SOURCE = /^identity-(entityId|instanceId|operationId)-(256|257)$/

function operationId(length: number): SourceOperationId {
  return `op:${'o'.repeat(length - 3)}`
}

function sceneEntityId(length: number, fill: string): SceneEntityId {
  return `entity:${fill.repeat(length - 7)}`
}

function controlledMesh(
  request: McpManifoldPlanQualificationRequest,
  field: ControlledIdentityField,
  idLength: number,
): MeshData {
  return {
    entityId: field === 'entityId'
      ? sceneEntityId(idLength, 'e')
      : 'entity:controlled-mesh',
    vertices: new Float32Array([
      0, 0, 0, 0, 0, 1,
      1, 0, 0, 0, 0, 1,
      0, 1, 0, 0, 0, 1,
    ]),
    indices: new Uint32Array([0, 1, 2]),
    edgeIndices: new Uint32Array([0, 1, 1, 2, 2, 0]),
    faceIds: new Uint32Array([0]),
    color: [1, 1, 1, 1],
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
    provenance: [{
      triangleStart: 0,
      triangleEnd: 1,
      source: {
        id: 1,
        operationId: field === 'operationId'
          ? operationId(idLength)
          : 'op:controlled-operation',
        instanceId: field === 'instanceId'
          ? sceneEntityId(idLength, 'i')
          : 'entity:controlled-instance',
        originalId: 1,
        start: 0,
        end: request.source.length,
        label: 'controlled operation boundary',
      },
      backside: false,
    }],
    topology: { boundary: 3, crease: 0, nonManifold: 0, degenerate: 0 },
  }
}

function result(request: McpManifoldPlanQualificationRequest): GeometryEvaluationResult {
  const match = CONTROLLED_IDENTITY_SOURCE.exec(request.source)
  return {
    meshes: match === null
      ? []
      : [controlledMesh(request, match[1] as ControlledIdentityField, Number(match[2]))],
    warnings: [],
    volume: 0,
    surfaceArea: 0,
    quality: request.quality,
    reduced: false,
    timings: { parseMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
  }
}

function postTerminal(
  request: McpManifoldPlanQualificationRequest,
  candidate: McpManifoldPlanQualificationTerminal,
): void {
  const publishable = isMcpManifoldPlanQualificationTerminal(candidate, request)
    ? candidate
    : {
        ...envelope(request),
        status: 'failed' as const,
        error: {
          name: 'ControlledQualificationProtocolError',
          message: 'Controlled result cannot cross the qualification Worker boundary',
          code: 'CONTROLLED_RESULT_UNPUBLISHABLE',
          line: null,
          column: null,
          start: null,
          end: null,
        },
      }
  if (publishable.status === 'succeeded') {
    self.postMessage(publishable, { transfer: meshTransferables(publishable.result.meshes) })
  } else {
    self.postMessage(publishable)
  }
  self.close()
}

self.addEventListener('message', (event: MessageEvent<unknown>) => {
  if (!isMcpManifoldPlanQualificationRequest(event.data)) return
  const request = event.data
  const started = { ...envelope(request), status: 'started' as const }
  if (!isMcpManifoldPlanQualificationStarted(started, request)) return
  self.postMessage(started)
  if (request.source === 'hang') {
    // Deliberately non-cooperative for far longer than the 80 ms deadline. The
    // finite safety horizon prevents a broken test runner from burning a core
    // forever, while the parent must still terminate this realm to pass.
    const safetyHorizon = Date.now() + 5_000
    while (Date.now() < safetyHorizon) { /* qualification fixture */ }
  }
  postTerminal(request, { ...envelope(request), status: 'succeeded', result: result(request) })
})
