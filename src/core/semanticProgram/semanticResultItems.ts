import { CAPABILITY_BY_KIND } from '../semanticProgram'
import type { SemanticAmbiguityGroupId, SemanticDynamicSlot, SemanticNode, SemanticOccurrenceId, SemanticOperationCategory, SemanticOperationId, SemanticOutputRef, SemanticProgramCoreV1, SemanticResult, SemanticSceneEntityId, SemanticStructuralPathSegment } from '../semanticProgram'
import { sha256Hex } from '../sha256'
import { utf8Compare } from '../utf8Order'

export function semanticResultItems(result: SemanticResult): readonly SemanticOutputRef[] {
  switch (result.tag) {
    case 'empty': return []
    case 'single': return [result.item]
    case 'multi': return result.items
  }
}

export function semanticNodeInputs(node: SemanticNode): readonly number[] {
  switch (node.kind) {
    case 'transform':
    case 'linear-extrude':
    case 'rotate-extrude-analytic':
    case 'rotate-extrude-polygonal':
    case 'projection':
    case 'offset':
      return [node.input]
    case 'boolean':
    case 'hull':
      return node.inputs
    default:
      return []
  }
}

/** Source operation names that may author a concrete node (aliases excluded). */
export function semanticNodeProducerOperationNames(node: SemanticNode): readonly string[] {
  switch (node.kind) {
    case 'box': return ['cube']
    case 'sphere-analytic':
    case 'sphere-polygonal': return ['sphere']
    case 'cylinder-analytic':
    case 'cylinder-polygonal': return ['cylinder']
    case 'polyhedron': return ['polyhedron']
    case 'rectangle': return ['square']
    case 'circle-analytic':
    case 'circle-polygonal': return ['circle']
    case 'polygon': return ['polygon']
    case 'transform': return ['translate', 'rotate', 'scale', 'mirror', 'multmatrix']
    case 'boolean': return [node.operation]
    case 'hull': return ['hull']
    case 'linear-extrude': return ['linear_extrude']
    case 'rotate-extrude-analytic':
    case 'rotate-extrude-polygonal': return ['rotate_extrude']
    case 'projection': return ['projection']
    case 'offset': return ['offset']
  }
}

export function deriveSemanticCapabilityClosure(
  core: Pick<SemanticProgramCoreV1, 'language' | 'nodes' | 'result' | 'operations' | 'occurrences' | 'declaredCapabilities' | 'diagnosticTemplates'>,
): readonly string[] {
  const capabilities = new Set(core.declaredCapabilities)
  capabilities.add('semantic.program-v1')
  capabilities.add('semantic.operation-graph')
  capabilities.add('semantic.identity-evidence')
  capabilities.add(`semantic.result.${core.result.tag}`)
  if (core.occurrences.length > 0) capabilities.add('semantic.occurrences')
  if (core.diagnosticTemplates.length > 0) capabilities.add('diagnostics.deterministic')
  for (const node of core.nodes) {
    capabilities.add(CAPABILITY_BY_KIND[node.kind])
    capabilities.add(`geometry.kind.${node.valueType.geometryKind}`)
    capabilities.add(`geometry.space.${node.valueType.space}`)
    capabilities.add(`representation.${node.valueType.representation}`)
    capabilities.add(`evidence.${node.valueType.evidence.tag}`)
    if (node.valueType.evidence.tag === 'certified-approximation') {
      capabilities.add(`evidence.profile.${node.valueType.evidence.certificateProfile}`)
      capabilities.add(`evidence.policy.sha256.${node.valueType.evidence.certificatePolicyHash}`)
    }
    capabilities.add('geometry.value')
    if (node.kind === 'boolean') {
      capabilities.add(`operation.boolean.${node.operation}`)
      capabilities.add('operation.boolean')
    }
    for (const inputIndex of semanticNodeInputs(node)) {
      const input = core.nodes[inputIndex]
      if (input.valueType.geometryKind !== node.valueType.geometryKind
        || input.valueType.space !== node.valueType.space) {
        capabilities.add(
          `geometry.transition.${input.valueType.geometryKind}.${input.valueType.space}`
          + `.to.${node.valueType.geometryKind}.${node.valueType.space}`,
        )
      }
      if (input.valueType.representation !== node.valueType.representation) {
        capabilities.add(`representation.transition.${input.valueType.representation}.to.${node.valueType.representation}`)
      }
      if (input.valueType.evidence.tag !== node.valueType.evidence.tag) {
        capabilities.add(`evidence.transition.${input.valueType.evidence.tag}.to.${node.valueType.evidence.tag}`)
      }
    }
  }
  return Object.freeze([...capabilities].sort(utf8Compare))
}

export function stableJson(value: unknown): string {
  if (value === null || typeof value === 'boolean' || typeof value === 'number' || typeof value === 'string') return JSON.stringify(value)
  if (Array.isArray(value)) return `[${value.map(stableJson).join(',')}]`
  return `{${Object.entries(value as Record<string, unknown>)
    .sort(([left], [right]) => utf8Compare(left, right))
    .map(([key, item]) => `${JSON.stringify(key)}:${stableJson(item)}`)
    .join(',')}}`
}

export function identityDigest(domain: string, payload: unknown): string {
  const domainBytes = new TextEncoder().encode(domain)
  const payloadBytes = new TextEncoder().encode(stableJson(payload))
  const preimage = new Uint8Array(4 + domainBytes.length + 4 + payloadBytes.length)
  const view = new DataView(preimage.buffer)
  view.setUint32(0, domainBytes.length, false)
  preimage.set(domainBytes, 4)
  view.setUint32(4 + domainBytes.length, payloadBytes.length, false)
  preimage.set(payloadBytes, 8 + domainBytes.length)
  return sha256Hex(preimage)
}

export function deriveSemanticOperationId(path: readonly SemanticStructuralPathSegment[]): SemanticOperationId {
  return `opv1:${identityDigest('semantic-operation-v1', path)}`
}

export function deriveSemanticAmbiguityGroupId(
  parentPath: readonly SemanticStructuralPathSegment[],
  category: SemanticOperationCategory,
  name: string,
): SemanticAmbiguityGroupId {
  return `ambv1:${identityDigest('semantic-ambiguity-group-v1', { parentPath, category, name })}`
}

export function deriveSemanticOccurrenceId(
  parentOccurrenceId: SemanticOccurrenceId | null,
  staticParentOccurrenceId: SemanticOccurrenceId | null,
  operationId: SemanticOperationId,
  dynamicSlots: readonly SemanticDynamicSlot[],
): SemanticOccurrenceId {
  return `occv1:${identityDigest('semantic-occurrence-v1', {
    parentOccurrenceId,
    staticParentOccurrenceId,
    operationId,
    dynamicSlots,
  })}`
}

export function deriveSemanticSceneEntityId(
  occurrenceId: SemanticOccurrenceId,
  outputOrdinal: number,
): SemanticSceneEntityId {
  return `entity:v2:${identityDigest('semantic-scene-entity-v1', { occurrenceId, outputOrdinal })}`
}
