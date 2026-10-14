import {
  deriveSemanticAmbiguityGroupId,
  deriveSemanticCapabilityClosure,
  deriveSemanticOccurrenceId,
  deriveSemanticOperationId,
  deriveSemanticSceneEntityId,
  SEMANTIC_PROGRAM_CAPABILITY_GRAPH,
  SEMANTIC_PROGRAM_DIAGNOSTIC_CODES,
  SEMANTIC_PROGRAM_EXECUTION_FEATURE,
  SEMANTIC_PROGRAM_EXECUTION_VERSION,
  SEMANTIC_PROGRAM_IDENTITY,
  SEMANTIC_PROGRAM_LIMITS,
  semanticNodeInputs,
  semanticNodeProducerOperationNames,
  semanticResultItems,
  type SemanticDiagnosticTemplate,
  type SemanticDimension,
  type SemanticDynamicSlot,
  type SemanticIdentityValue,
  type SemanticNode,
  type SemanticOccurrence,
  type SemanticOperationCategory,
  type SemanticProgramCoreV1,
  type SemanticProgramDiagnosticCode,
  type SemanticProgramEnvelopeV1,
  type SemanticProgramV1,
  type SemanticSourceDescriptor,
  type SemanticSourceSpan,
  type SemanticStaticOperation,
  type SemanticStructuralPathSegment,
  type SemanticValueType,
} from '../core/semanticProgram'
import { sha256Hex } from '../core/sha256'
import { parseGeometrySourceRoutingHeader } from '../core/geometryRouting'
import { isSemanticProgramCodecOwnedGraph } from './semanticProgramCodec'
import { deepFreeze } from '../core/objects'
import { utf8Compare } from '../core/utf8Order'
export { validateSemanticProgramV1, normalizeSemanticProgram, normalizeDecodedSemanticProgram, normalizeSemanticProgramCore, normalizeDecodedSemanticProgramCore, normalizeSemanticProgramCoreInput, semanticSourceDescriptor, attestSemanticProgramSource, semanticDiagnosticCodeIsPublic } from './semanticProgramValidator/validateSemanticProgramV1'

export class SemanticProgramValidationError extends TypeError {
  constructor(
    readonly code: SemanticProgramDiagnosticCode,
    readonly path: string,
    detail: string,
  ) {
    super(`${code} at ${path}: ${detail}`)
    this.name = 'SemanticProgramValidationError'
  }
}

export interface SemanticModuleActivationAnchor {
  readonly callOccurrence: number
  readonly bodyOccurrence: number
  readonly definitionOccurrence: number
  readonly expansionOperation: number
}

export interface SemanticOccurrenceValidationIndex {
  readonly operationChildren: readonly (readonly number[])[]
  readonly runtimeChildren: readonly (readonly number[])[]
  readonly activationByDefinition: Map<number, SemanticModuleActivationAnchor | null>
  readonly proofBudget: { steps: number }
}

export interface ValidatedSemanticExecution {
  readonly evaluationOrder: readonly number[]
  readonly discardedEffects: readonly Readonly<{
    tag: 'legacy-difference-cutters'
    root: number
    ownerOccurrence: number
  }>[]
  readonly terminal: null | Readonly<{
    tag: 'legacy-language-error'
    occurrence: number
    diagnosticTemplate: number
    prefixFrontier: readonly Readonly<{ root: number; ownerOccurrence: number | null }>[]
  }>
}

export type SemanticProductionRule =
  | 'primitive' | 'transform-map' | 'projection-map' | 'offset-map'
  | 'preserving-alias' | 'transparent' | 'boolean' | 'hull' | 'difference'
  | 'linear-extrude' | 'rotate-extrude' | 'terminal-unsupported'

export interface SemanticProductionItem {
  readonly node: number
  readonly row: number
  readonly identityOwner: number
  readonly group: number
  readonly directChild: number
}

export type SemanticScheduleEvent =
  | { readonly tag: 'group'; readonly group: number }
  | { readonly tag: 'node'; readonly node: number }

export interface SemanticProductionGroup {
  readonly canonical: number
  readonly outputRows: readonly number[]
  readonly outputItems: readonly SemanticProductionItem[]
  readonly frontier: readonly SemanticProductionItem[]
  readonly terminalItems: readonly SemanticProductionItem[]
  readonly schedule: readonly SemanticScheduleEvent[]
  readonly transparent: boolean
}

export interface SnapshotBudget { values: number }
