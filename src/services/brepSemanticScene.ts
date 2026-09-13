/** Internal SemanticProgram → native B-rep → display adapter.
 * This executable seam does not register a qualified production engine.
 */
import type {GeometryEvaluationResult, GeometryQuality} from '../core/build'
import type {MeshData} from '../core/mesh'
import {createNativeGeometryArtifact, MAX_NATIVE_GEOMETRY_CHARACTERS} from '../core/nativeGeometry'
import {geometryAssetId} from '../core/scene'
import {sha256Hex} from '../core/sha256'
import {type SemanticOccurrenceId, type SemanticOperationId} from '../core/semanticProgram'
import {executeBrepNativeProgram} from './brepNativeExecutor'
import {prepareBrepDisplay} from './geometry/brep'
import {buildMeshBvh} from './meshBvh'
import {extractSemanticEdges} from './meshTopology'
import {SemanticProgramExecutionError, type SemanticExecutionControl} from './semanticProgramExecutor'
import {requireTrustedSemanticLowering, type SemanticLoweringSuccess} from './semanticProgramLowerer'
import type {SemanticProgramAttestation} from './semanticProgramCodec'

export interface BrepSemanticDisplayPolicy {
  readonly quality: GeometryQuality
  /** Explicit uniform samples per retained patch/edge, within the native limit.
   * This policy does not claim to enforce source chord/angle intents.
   */
  readonly segments: number
}

export interface BrepSemanticOutputRecord {
  readonly occurrenceId: SemanticOccurrenceId
  readonly operationId: SemanticOperationId
  readonly entityId: `entity:v2:${string}`
  readonly nodeIndex: number
  readonly empty: boolean
  readonly snapshotRevision: string | null
  readonly geometryAssetId: MeshData['geometryAssetId'] | null
  readonly topologyFaceIds: readonly string[]
}

export interface BrepSemanticSceneBuild {
  readonly result: GeometryEvaluationResult
  readonly attestation: SemanticProgramAttestation
  /** Actual display settings have an identity separate from authored intent. */
  readonly displayPolicyHash: string
  readonly outputs: readonly BrepSemanticOutputRecord[]
  readonly metrics: 'display_mesh_estimates'
  readonly deviationStatus: 'not_certified'
}

function checkControl(control: SemanticExecutionControl): void {
  if (control.signal?.aborted || control.shouldAbort?.()) {
    throw new SemanticProgramExecutionError('E_SEMANTIC_ABORTED', null, 'B-rep scene assembly cancelled')
  }
  if (control.deadlineAt !== undefined) {
    const now = (control.now ?? (() => performance.now()))()
    if (!Number.isFinite(now) || now >= control.deadlineAt) {
      throw new SemanticProgramExecutionError('E_SEMANTIC_DEADLINE', null, 'B-rep scene assembly deadline exceeded')
    }
  }
}

/** Publish all outputs atomically and release every backend lease on any exit.
 * Synchronous WASM calls require worker isolation for hard cancellation.
 */
export async function buildBrepSemanticScene(
  input: SemanticLoweringSuccess,
  policy: BrepSemanticDisplayPolicy,
  control: SemanticExecutionControl = {},
): Promise<BrepSemanticSceneBuild> {
  const trusted = requireTrustedSemanticLowering(input)
  if (!Number.isSafeInteger(policy.segments) || policy.segments < 1 || policy.segments > 32
    || (policy.quality !== 'preview' && policy.quality !== 'full')) {
    throw new RangeError('B-rep display requires preview/full quality and 1..32 segments')
  }
  const displayPolicy = Object.freeze({version: 1, sampling: 'uniform-patch' as const, quality: policy.quality, segments: policy.segments})
  const start = performance.now()
  const execution = await executeBrepNativeProgram(trusted, control)
  const evaluated = performance.now()
  let assembled: BrepSemanticSceneBuild
  try {
    const meshes: MeshData[] = [], outputs: BrepSemanticOutputRecord[] = []
    let volume = 0, surfaceArea = 0, triangles = 0, nativeCharacters = 0
    const provenanceByOperation = new Map(execution.program.provenance.map(p => [p.operation, p]))
    for (let index = 0; index < execution.outputs.length; index++) {
      checkControl(control)
      const output = execution.outputs[index], item = output.reference
      const entityId = output.occurrence.sceneEntityId
      if (entityId === null) throw new Error('B-rep result occurrence has no scene identity')
      const operationId = execution.program.core.operations[output.occurrence.operation].operationId
      const identity = {occurrenceId: output.occurrence.occurrenceId, operationId, entityId, nodeIndex: item.node}
      if (output.value.tag === 'empty') {
        outputs.push(Object.freeze({...identity, empty: true, snapshotRevision: null, geometryAssetId: null, topologyFaceIds: Object.freeze([])}))
        continue
      }
      if(output.value.geometry.kind!=='solid')throw new Error('B-rep scene output must be a solid')
      const {model} = output.value.geometry
      const mesh = prepareBrepDisplay(model, displayPolicy.segments)
      checkControl(control)
      triangles += mesh.displayIndices.length / 3
      if (triangles > 20_000) throw new RangeError('B-rep scene exceeds 20000 display triangles')
      if (!mesh.displayIndices.length || !mesh.report.closed) throw new Error('Nonempty B-rep solid did not produce a closed display mesh')
      const snapshot = createNativeGeometryArtifact(entityId, 'brep', {geometry: model}, execution.program)
      nativeCharacters += snapshot.geometryJson.length + snapshot.documentJson.length
      if (nativeCharacters > MAX_NATIVE_GEOMETRY_CHARACTERS) throw new RangeError('B-rep scene exceeds its aggregate native snapshot limit')
      const vertices = new Float32Array(mesh.displayVertices), indices = new Uint32Array(mesh.displayIndices)
      surfaceArea += mesh.surfaceArea
      const sourceOccurrence = execution.program.core.occurrences[item.producerOccurrence]
      const source = provenanceByOperation.get(sourceOccurrence.operation)
      const edges = extractSemanticEdges(vertices, indices)
      const assetId = geometryAssetId(vertices, indices)
      meshes.push({
        vertices, indices, geometryAssetId: assetId, entityId, nativeGeometry: snapshot,
        bvh: buildMeshBvh(vertices, indices), edgeIndices: edges.indices,
        topology: {boundary: mesh.report.boundaryEdges, nonManifold: mesh.report.nonManifoldEdges,
          degenerate: mesh.report.degenerateTriangles, crease: edges.diagnostics.crease},
        color: [...output.color], transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
        faceIds: new Uint32Array(mesh.faceIds), faceIdsAuthoritative: true,
        provenance: [{triangleStart: 0, triangleEnd: indices.length / 3, backside: false,
          source: source ? {id: sourceOccurrence.operation, originalId: sourceOccurrence.operation,
            instanceId: entityId, start: source.span.start, end: source.span.end, label: source.label} : null}],
      })
      volume += Math.abs(mesh.report.signedVolumeMm3)
      outputs.push(Object.freeze({...identity, empty: false, snapshotRevision: snapshot.revision, geometryAssetId: assetId,
        topologyFaceIds: Object.freeze([...(model.topologyIds?.faces ?? [])])}))
    }
    checkControl(control)
    if (!Number.isFinite(volume) || !Number.isFinite(surfaceArea)) throw new Error('B-rep display metrics are non-finite')
    assembled = Object.freeze({
      result: {meshes, warnings: [...trusted.warnings], volume, surfaceArea, quality: displayPolicy.quality, reduced: false,
        timings: {parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: evaluated - start, analyzeMs: performance.now() - evaluated}},
      attestation: execution.attestation,
      displayPolicyHash: sha256Hex('brep-display-policy-v1\n' + JSON.stringify(displayPolicy)),
      outputs: Object.freeze(outputs), metrics: 'display_mesh_estimates', deviationStatus: 'not_certified',
    })
  } finally {
    await execution.dispose()
  }
  // Disposal is an asynchronous ownership boundary. Cancellation while it was
  // pending must still prevent publication of the assembled scene.
  checkControl(control)
  return assembled
}
