import { boundedSceneEntityId } from '../core/boundedSceneEntityId'
import type { GeometryQuality } from '../core/build'
import type {
  MeshData,
  MeshProvenanceRun,
  MeshSourceReference,
  SceneEntityId,
  SourceOperationId,
} from '../core/mesh'
import {
  semanticNodeProducerOperationNames,
  type SemanticIdentityValue,
  type SemanticOccurrence,
  type SemanticProgramV1,
  type SemanticStaticOperation,
} from '../core/semanticProgram'
import { geometryAssetId } from '../core/scene'
import { identity } from './math3d'
import { buildMeshBvh } from './meshBvh'
import { extractSemanticEdges } from './meshTopology'
import {
  analyzeManifoldPlanSolid,
  inspectManifoldPlanPayload,
  type ManifoldPlanPayload,
  type ManifoldPlanPayloadFamily,
} from './manifoldPlanBackend'
import type { SemanticExecutionResult } from './semanticProgramExecutor'

const MAX_TRIANGLES = 750_000

export interface LegacyV5AssemblyArtifact {
  readonly program: SemanticProgramV1
  readonly warnings: readonly string[]
  readonly fullEquivalent: boolean
}

export interface LegacyV5AssemblyOptions {
  readonly quality?: GeometryQuality
  readonly shouldAbort?: () => boolean
  readonly onYield?: () => void
  readonly yieldControl?: () => Promise<void>
}

export interface LegacyV5AssemblyResult {
  readonly meshes: MeshData[]
  readonly warnings: string[]
  readonly volume: number
  readonly surfaceArea: number
  readonly quality: GeometryQuality
  readonly reduced: boolean
}

export type LegacyV5AssemblyErrorCode =
  | 'E_LEGACY_V5_ABORTED'
  | 'E_LEGACY_V5_NORMALS_MISSING'
  | 'E_LEGACY_V5_TRIANGLE_LIMIT'

export class LegacyV5AssemblyError extends Error {
  constructor(
    readonly code: LegacyV5AssemblyErrorCode,
    message: string,
  ) {
    super(message)
    this.name = 'LegacyV5AssemblyError'
  }
}

function directStatementType(operation: SemanticStaticOperation): 'call' | 'module' | null {
  const segment = operation.structuralPath.at(-1)
  if (segment === undefined || segment.name.startsWith('$')) return null
  if (segment.kind === 'module') return 'module'
  return segment.kind === 'call' || segment.kind === 'control' ? 'call' : null
}

function sameStructuralPath(
  left: SemanticStaticOperation['structuralPath'],
  right: SemanticStaticOperation['structuralPath'],
): boolean {
  return left.length === right.length && left.every((segment, index) => {
    const candidate = right[index]
    return segment.kind === candidate.kind
      && segment.name === candidate.name
      && segment.ordinal === candidate.ordinal
  })
}

function directSiblingOrdinal(
  operation: SemanticStaticOperation,
  operations: readonly SemanticStaticOperation[],
): number {
  const statementType = directStatementType(operation)
  if (statementType === null) throw new TypeError('Legacy operation identity requires a source statement')
  let ordinal = 0
  for (const candidate of operations) {
    if (candidate.id >= operation.id) break
    if (candidate.parent === operation.parent
      && directStatementType(candidate) === statementType
      && candidate.name === operation.name) ordinal++
  }
  return ordinal
}

/** Exact compatibility projection for the pinned compiler's `op:...` IDs. */
export function legacyOperationId(
  program: SemanticProgramV1,
  operationIndex: number,
): SourceOperationId {
  const operation = program.core.operations[operationIndex]
  if (operation === undefined) throw new TypeError('Semantic operation index is out of bounds')
  const tokens = ['root']
  let lastStatementType: 'call' | 'module' | null = null
  for (let length = 1; length <= operation.structuralPath.length; length++) {
    const path = operation.structuralPath.slice(0, length)
    const segment = path[length - 1]
    const pathOperation = program.core.operations.find(candidate => (
      sameStructuralPath(candidate.structuralPath, path)
    ))
    if (segment.kind === 'call' || (segment.kind === 'control' && !segment.name.startsWith('$'))) {
      if (pathOperation === undefined) throw new TypeError('Semantic operation path is incomplete')
      tokens.push(`call:${segment.name}#${directSiblingOrdinal(pathOperation, program.core.operations)}`)
      lastStatementType = 'call'
    } else if (segment.kind === 'module') {
      if (pathOperation === undefined) throw new TypeError('Semantic module path is incomplete')
      tokens.push(`module:${segment.name}#${directSiblingOrdinal(pathOperation, program.core.operations)}`)
      lastStatementType = 'module'
    } else if (segment.kind === 'body') {
      tokens.push(lastStatementType === 'module' ? 'body' : 'children')
    } else if (segment.kind === 'branch') {
      tokens.push(segment.name === '$else' ? 'alternative' : 'children')
    }
    // `$expansion` is a SemanticProgram continuation proof frame. The pinned
    // source compiler has no corresponding path token.
  }
  return `op:${tokens.map(encodeURIComponent).join('/')}`
}

function legacyIdentityValue(value: SemanticIdentityValue): string {
  switch (value.tag) {
    case 'undefined': return 'undef'
    case 'null': return 'null'
    case 'boolean': return String(value.value)
    case 'number': return String(value.value)
    case 'string': return JSON.stringify(value.value)
    case 'vector': return `[${value.items.map(legacyIdentityValue).join(',')}]`
  }
}

function runtimeOccurrenceChain(
  program: SemanticProgramV1,
  occurrence: SemanticOccurrence,
): readonly SemanticOccurrence[] {
  const chain: SemanticOccurrence[] = []
  let cursor: SemanticOccurrence | undefined = occurrence
  const seen = new Set<number>()
  while (cursor !== undefined) {
    if (seen.has(cursor.id)) throw new TypeError('Semantic occurrence ancestry contains a cycle')
    seen.add(cursor.id)
    chain.push(cursor)
    cursor = cursor.parent === null ? undefined : program.core.occurrences[cursor.parent]
  }
  return chain.reverse()
}

/** Exact compatibility projection for the pinned evaluator's entity path. */
export function legacySceneEntityId(
  program: SemanticProgramV1,
  occurrence: SemanticOccurrence,
): SceneEntityId {
  let path = 'root'
  for (const runtime of runtimeOccurrenceChain(program, occurrence)) {
    const operation = program.core.operations[runtime.operation]
    if (directStatementType(operation) !== 'call') continue
    path += `>${legacyOperationId(program, operation.id)}`
    if (operation.name === 'for') {
      const iterator = [...runtime.dynamicSlots].reverse().find(slot => !slot.name.startsWith('$'))
      if (iterator !== undefined) {
        path += `>loop:${encodeURIComponent(iterator.name)}=${encodeURIComponent(legacyIdentityValue(iterator.value))}#${iterator.duplicateOrdinal}`
      }
    }
  }
  return boundedSceneEntityId(path)
}

function producerOccurrenceByNode(program: SemanticProgramV1): ReadonlyMap<number, SemanticOccurrence> {
  const result = new Map<number, SemanticOccurrence>()
  for (const occurrence of program.core.occurrences) {
    if (occurrence.node !== null && occurrence.outputOrdinal !== null && !result.has(occurrence.node)) {
      const operation = program.core.operations[occurrence.operation]
      if (semanticNodeProducerOperationNames(program.core.nodes[occurrence.node]).includes(operation.name)) {
        result.set(occurrence.node, occurrence)
      }
    }
  }
  return result
}

function sourceReference(
  program: SemanticProgramV1,
  producers: ReadonlyMap<number, SemanticOccurrence>,
  nodeIndex: number,
  originalId: number,
): MeshSourceReference | null {
  const occurrence = producers.get(nodeIndex)
  if (occurrence === undefined) return null
  const operation = program.core.operations[occurrence.operation]
  const provenance = program.provenance[occurrence.operation]
  if (operation === undefined || provenance === undefined) return null
  return {
    id: provenance.span.start,
    operationId: legacyOperationId(program, operation.id),
    instanceId: legacySceneEntityId(program, occurrence),
    originalId,
    start: provenance.span.start,
    end: provenance.span.end,
    label: `${operation.name}()`,
  }
}

function abortIfRequested(options: LegacyV5AssemblyOptions): void {
  if (options.shouldAbort?.()) {
    throw new LegacyV5AssemblyError('E_LEGACY_V5_ABORTED', 'Legacy v5 assembly was cancelled')
  }
}

async function yieldForCancellation(options: LegacyV5AssemblyOptions): Promise<void> {
  await (options.yieldControl?.() ?? new Promise<void>(resolve => setTimeout(resolve, 0)))
  options.onYield?.()
  abortIfRequested(options)
}

/**
 * Source-free compatibility assembler. It consumes only a trusted semantic
 * artifact and an owning execution result; protocol envelopes and source-bound
 * error rendering belong to the outer facade.
 */
export async function assembleLegacyV5Result(
  artifact: LegacyV5AssemblyArtifact,
  execution: SemanticExecutionResult<ManifoldPlanPayloadFamily>,
  options: LegacyV5AssemblyOptions = {},
): Promise<LegacyV5AssemblyResult> {
  const warnings = [...artifact.warnings]
  // The pinned evaluator warns for authored top-level 2D shape records even
  // when the kernel geometry is empty; typed emptiness must not erase that
  // observable shape count.
  const sections = execution.outputs.filter(output => output.value.valueType.space === 'd2')
  if (sections.length > 0) {
    warnings.push(`${sections.length} top-level 2D object(s) are not displayed; wrap them in linear_extrude() or rotate_extrude()`)
  }

  const meshes: MeshData[] = []
  const producers = producerOccurrenceByNode(artifact.program)
  let volume = 0
  let surfaceArea = 0
  let triangleCount = 0
  for (const output of execution.outputs) {
    abortIfRequested(options)
    if (output.value.tag !== 'value') continue
    const payload = output.value.payload as ManifoldPlanPayload
    if (inspectManifoldPlanPayload(payload).dimension !== 3) continue
    const analysis = analyzeManifoldPlanSolid(payload)
    volume += analysis.volume
    surfaceArea += analysis.surfaceArea
    const kernelMesh = analysis.mesh
    triangleCount += kernelMesh.numTri
    if (triangleCount > MAX_TRIANGLES) {
      throw new LegacyV5AssemblyError(
        'E_LEGACY_V5_TRIANGLE_LIMIT',
        `Rendered model exceeds ${MAX_TRIANGLES.toLocaleString()} triangles`,
      )
    }
    if (kernelMesh.numProp < 6) {
      throw new LegacyV5AssemblyError(
        'E_LEGACY_V5_NORMALS_MISSING',
        'Geometry kernel did not produce normals',
      )
    }
    const vertices = new Float32Array(kernelMesh.numVert * 6)
    for (let vertex = 0; vertex < kernelMesh.numVert; vertex++) {
      const sourceOffset = vertex * kernelMesh.numProp
      const targetOffset = vertex * 6
      for (let channel = 0; channel < 6; channel++) {
        vertices[targetOffset + channel] = kernelMesh.vertProperties[sourceOffset + channel]
      }
      if ((vertex & 0x3fff) === 0x3fff) await yieldForCancellation(options)
    }
    const indices = new Uint32Array(kernelMesh.triVerts)
    const bvh = buildMeshBvh(vertices, indices)
    await yieldForCancellation(options)
    const semanticEdges = extractSemanticEdges(vertices, indices, {
      creaseAngleDegrees: 30,
      mergeFromVert: kernelMesh.mergeFromVert,
      mergeToVert: kernelMesh.mergeToVert,
    })
    await yieldForCancellation(options)
    const view = inspectManifoldPlanPayload(payload)
    const provenance: MeshProvenanceRun[] = []
    for (let run = 0; run < kernelMesh.runOriginalID.length; run++) {
      const triangleStart = (kernelMesh.runIndex[run] ?? 0) / 3
      const triangleEnd = (kernelMesh.runIndex[run + 1] ?? kernelMesh.triVerts.length) / 3
      if (triangleEnd <= triangleStart) continue
      const originalId = kernelMesh.runOriginalID[run]
      const sourceNode = view.nodeByOriginalId.get(originalId)
      provenance.push({
        triangleStart,
        triangleEnd,
        source: sourceNode === undefined
          ? null
          : sourceReference(artifact.program, producers, sourceNode, originalId),
        backside: ((kernelMesh.runFlags[run] ?? 0) & 1) !== 0,
      })
      if ((run & 0x1fff) === 0x1fff) await yieldForCancellation(options)
    }
    if (provenance.length === 0 && kernelMesh.numTri > 0) {
      provenance.push({
        triangleStart: 0,
        triangleEnd: kernelMesh.numTri,
        source: null,
        backside: false,
      })
    }
    meshes.push({
      entityId: legacySceneEntityId(artifact.program, output.occurrence),
      geometryAssetId: geometryAssetId(vertices, indices),
      vertices,
      indices,
      bvh,
      edgeIndices: semanticEdges.indices,
      color: [...output.color],
      transform: identity(),
      faceIds: new Uint32Array(kernelMesh.faceID),
      faceIdsSurfaceGroups: true,
      provenance,
      topology: semanticEdges.diagnostics,
    })
    await yieldForCancellation(options)
  }
  return {
    meshes,
    warnings,
    volume,
    surfaceArea,
    quality: options.quality ?? 'full',
    reduced: !artifact.fullEquivalent,
  }
}
