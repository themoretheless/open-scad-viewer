import type { GeometryLanguageContract } from './geometryExecution'
import { sha256Hex } from './sha256'
import { utf8Compare } from './utf8Order'
export { semanticResultItems, semanticNodeInputs, semanticNodeProducerOperationNames, deriveSemanticCapabilityClosure, deriveSemanticOperationId, deriveSemanticAmbiguityGroupId, deriveSemanticOccurrenceId, deriveSemanticSceneEntityId } from './semanticProgram/semanticResultItems'

export const SEMANTIC_PROGRAM_CONTRACT = 'semantic-program-contract-v1' as const
export const SEMANTIC_PROGRAM_IDENTITY = 'semantic-program-core-v1' as const
export const SEMANTIC_PROGRAM_CAPABILITY_GRAPH = 'semantic-capabilities-v1' as const
export const SEMANTIC_PROGRAM_SCHEMA_VERSION = Object.freeze({ major: 1 as const, minor: 2 as const })
export const SEMANTIC_PROGRAM_EXECUTION_VERSION = 'semantic-execution-v2' as const
export const SEMANTIC_PROGRAM_EXECUTION_FEATURE = 'semantic.execution-v2' as const

export const SEMANTIC_PROGRAM_LIMITS = Object.freeze({
  nodes: 25_000,
  operations: 50_000,
  occurrences: 100_000,
  outputs: 1_000,
  diagnostics: 10_000,
  // One discarded effect must own at least one retained DAG node, so the node
  // budget is the natural closed upper bound. A smaller policy cap changes
  // otherwise valid legacy evaluation semantics (for example 1,001 eager
  // empty-base differences).
  discardedEffects: 25_000,
  tessellationIntents: 50_000,
  declaredCapabilities: 32,
  capabilityClosure: 128,
  sourceCodeUnits: 250_000,
  // A rendered source-bound diagnostic can contain both a semantic detail and
  // the complete offending source line. It is still bounded by canonical
  // aggregate bytes and is not part of SPC identity.
  stringCodeUnits: 1_000_000,
  diagnosticMessageCodeUnits: 1_000_000,
  canonicalStringBytes: 16 * 1024 * 1024,
  snapshotValues: 1_000_000,
  snapshotDepth: 256,
  identityValueDepth: 32,
} as const)

export const SEMANTIC_PROGRAM_DIAGNOSTIC_CODES = Object.freeze([
  'E_SEMANTIC_BYTES',
  'E_SEMANTIC_SCHEMA',
  'E_SEMANTIC_VERSION',
  'E_SEMANTIC_FIELD',
  'E_SEMANTIC_LIMIT',
  'E_SEMANTIC_NUMBER',
  'E_SEMANTIC_REFERENCE',
  'E_SEMANTIC_TYPE',
  'E_SEMANTIC_DAG',
  'E_SEMANTIC_IDENTITY',
  'E_SEMANTIC_PROVENANCE',
  'E_SEMANTIC_CAPABILITY_CLOSURE',
  'E_SEMANTIC_ORDER',
  'E_SEMANTIC_MIGRATION',
  'E_SEMANTIC_RELOWER_REQUIRED',
] as const)

export type SemanticProgramDiagnosticCode = typeof SEMANTIC_PROGRAM_DIAGNOSTIC_CODES[number]
/** Legacy evaluator convenience only; it is not serialized in SPC1. */
export type SemanticDimension = 'region2' | 'solid3'
export type SemanticGeometryKind = 'curve' | 'wire' | 'region' | 'sheet' | 'solid' | 'solid-set'
export type SemanticSpace = 'd2' | 'd3'
export type SemanticRepresentation = 'analytic-brep' | 'rational-brep' | 'certified-approx-brep' | 'mesh'
export type SemanticRepresentationEvidence =
  | Readonly<{ tag: 'representation-preserving' }>
  | Readonly<{
    tag: 'certified-approximation'
    certificateProfile: string
    certificatePolicyHash: string
  }>
export interface SemanticValueType {
  readonly geometryKind: SemanticGeometryKind
  readonly space: SemanticSpace
  readonly representation: SemanticRepresentation
  readonly evidence: SemanticRepresentationEvidence
}
export type SemanticNodeId = number
export type SemanticOperationIndex = number
export type SemanticOccurrenceIndex = number
export type SemanticVec2 = readonly [number, number]
export type SemanticVec3 = readonly [number, number, number]
export type SemanticColor = readonly [number, number, number, number]
/** Column-major affine 4x4; composition is parent × local. */
export type SemanticMatrix4 = readonly [
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
]

export type SemanticOperationId = `opv1:${string}`
export type SemanticOccurrenceId = `occv1:${string}`
export type SemanticSceneEntityId = `entity:v2:${string}`
export type SemanticAmbiguityGroupId = `ambv1:${string}`

export interface SemanticSchemaVersion {
  readonly major: 1
  readonly minor: 2
}

export interface SemanticLanguageIdentity {
  readonly contract: GeometryLanguageContract
  readonly semanticsRevision: string
  readonly capabilityGraphVersion: typeof SEMANTIC_PROGRAM_CAPABILITY_GRAPH
}

export interface SemanticUnits {
  readonly length: 'millimeter'
  readonly angle: 'degree'
  readonly handedness: 'right'
  readonly upAxis: 'z'
  readonly matrixLayout: 'column-major'
  readonly composition: 'parent-times-local'
}

export type SemanticOperationCategory =
  | 'geometry'
  | 'transform'
  | 'boolean'
  | 'control'
  | 'module'
  | 'assertion'
  | 'presentation'

export type SemanticPathSegmentKind = 'call' | 'module' | 'control' | 'branch' | 'body'

export interface SemanticStructuralPathSegment {
  readonly kind: SemanticPathSegmentKind
  readonly name: string
  readonly ordinal: number
}

export interface SemanticStaticOperation {
  readonly id: SemanticOperationIndex
  readonly operationId: SemanticOperationId
  readonly parent: SemanticOperationIndex | null
  readonly childOrdinal: number
  readonly name: string
  readonly category: SemanticOperationCategory
  readonly structuralPath: readonly SemanticStructuralPathSegment[]
  readonly identityEvidence: 'structural-unique' | 'same-name-positional'
  readonly ambiguityGroup: SemanticAmbiguityGroupId | null
}

export type SemanticIdentityValue =
  | Readonly<{ tag: 'undefined' }>
  | Readonly<{ tag: 'null' }>
  | Readonly<{ tag: 'boolean'; value: boolean }>
  | Readonly<{ tag: 'number'; value: number }>
  | Readonly<{ tag: 'string'; value: string }>
  | Readonly<{ tag: 'vector'; items: readonly SemanticIdentityValue[] }>

export interface SemanticDynamicSlot {
  readonly name: string
  readonly value: SemanticIdentityValue
  /** Distinguishes repeated equal loop values without string coercion. */
  readonly duplicateOrdinal: number
}

export interface SemanticOccurrence {
  readonly id: SemanticOccurrenceIndex
  readonly occurrenceId: SemanticOccurrenceId
  readonly operation: SemanticOperationIndex
  readonly parent: SemanticOccurrenceIndex | null
  readonly staticParent: SemanticOccurrenceIndex | null
  readonly dynamicSlots: readonly SemanticDynamicSlot[]
  readonly node: SemanticNodeId | null
  readonly outputOrdinal: number | null
  readonly sceneEntityId: SemanticSceneEntityId | null
}

interface SemanticNodeBase {
  readonly id: SemanticNodeId
  readonly kind: string
  readonly valueType: SemanticValueType
}

export interface SemanticBoxNode extends SemanticNodeBase {
  readonly kind: 'box'
  readonly size: SemanticVec3
  readonly center: boolean
}

export interface SemanticSphereAnalyticNode extends SemanticNodeBase {
  readonly kind: 'sphere-analytic'
  readonly radius: number
}

export interface SemanticSpherePolygonalNode extends SemanticNodeBase {
  readonly kind: 'sphere-polygonal'
  readonly radius: number
  readonly radialSegments: number
}

export interface SemanticCylinderAnalyticNode extends SemanticNodeBase {
  readonly kind: 'cylinder-analytic'
  readonly height: number
  readonly radiusBottom: number
  readonly radiusTop: number
  readonly center: boolean
}

export interface SemanticCylinderPolygonalNode extends SemanticNodeBase {
  readonly kind: 'cylinder-polygonal'
  readonly height: number
  readonly radiusBottom: number
  readonly radiusTop: number
  readonly center: boolean
  readonly radialSegments: number
}

export interface SemanticPolyhedronNode extends SemanticNodeBase {
  readonly kind: 'polyhedron'
  readonly vertices: readonly SemanticVec3[]
  readonly triangles: readonly (readonly [number, number, number])[]
}

export interface SemanticRectangleNode extends SemanticNodeBase {
  readonly kind: 'rectangle'
  readonly size: SemanticVec2
  readonly center: boolean
}

export interface SemanticCircleAnalyticNode extends SemanticNodeBase {
  readonly kind: 'circle-analytic'
  readonly radius: number
}

export interface SemanticCirclePolygonalNode extends SemanticNodeBase {
  readonly kind: 'circle-polygonal'
  readonly radius: number
  readonly radialSegments: number
}

export interface SemanticPolygonNode extends SemanticNodeBase {
  readonly kind: 'polygon'
  readonly rings: readonly (readonly SemanticVec2[])[]
  readonly fillRule: 'even-odd'
}

export interface SemanticTransformNode extends SemanticNodeBase {
  readonly kind: 'transform'
  readonly input: SemanticNodeId
  readonly matrix: SemanticMatrix4
}

export interface SemanticBooleanNode extends SemanticNodeBase {
  readonly kind: 'boolean'
  readonly operation: 'union' | 'intersection' | 'difference'
  /** Authored order is semantic and is never sorted. */
  readonly inputs: readonly SemanticNodeId[]
}

export interface SemanticHullNode extends SemanticNodeBase {
  readonly kind: 'hull'
  readonly inputs: readonly SemanticNodeId[]
}

export interface SemanticLinearExtrudeNode extends SemanticNodeBase {
  readonly kind: 'linear-extrude'
  readonly input: SemanticNodeId
  readonly height: number
  readonly twistDegrees: number
  readonly slices: number
  readonly scale: SemanticVec2
  readonly center: boolean
}

export interface SemanticRotateExtrudeAnalyticNode extends SemanticNodeBase {
  readonly kind: 'rotate-extrude-analytic'
  readonly input: SemanticNodeId
  readonly angleDegrees: number
}

export interface SemanticRotateExtrudePolygonalNode extends SemanticNodeBase {
  readonly kind: 'rotate-extrude-polygonal'
  readonly input: SemanticNodeId
  readonly angleDegrees: number
  readonly radialSegments: number
}

export interface SemanticProjectionNode extends SemanticNodeBase {
  readonly kind: 'projection'
  readonly input: SemanticNodeId
  readonly cut: boolean
}

export interface SemanticOffsetNode extends SemanticNodeBase {
  readonly kind: 'offset'
  readonly input: SemanticNodeId
  readonly distance: number
}

export type SemanticNode =
  | SemanticBoxNode
  | SemanticSphereAnalyticNode
  | SemanticSpherePolygonalNode
  | SemanticCylinderAnalyticNode
  | SemanticCylinderPolygonalNode
  | SemanticPolyhedronNode
  | SemanticRectangleNode
  | SemanticCircleAnalyticNode
  | SemanticCirclePolygonalNode
  | SemanticPolygonNode
  | SemanticTransformNode
  | SemanticBooleanNode
  | SemanticHullNode
  | SemanticLinearExtrudeNode
  | SemanticRotateExtrudeAnalyticNode
  | SemanticRotateExtrudePolygonalNode
  | SemanticProjectionNode
  | SemanticOffsetNode

export interface SemanticOutputRef {
  readonly node: SemanticNodeId
  /** Occurrence that materialized this DAG node. */
  readonly producerOccurrence: SemanticOccurrenceIndex
  /** Occurrence that owns stable scene identity; transforms may preserve it. */
  readonly identityOccurrence: SemanticOccurrenceIndex
  readonly color: SemanticColor
}

export type SemanticResult =
  | Readonly<{ tag: 'empty'; type: 'never' }>
  | Readonly<{ tag: 'single'; item: SemanticOutputRef }>
  | Readonly<{ tag: 'multi'; items: readonly SemanticOutputRef[] }>

/**
 * A kernel-visible value which the language evaluates eagerly but never
 * publishes. V1 admits only the cutter reduction of a legacy difference with
 * a mathematically empty first child; future effect kinds require a schema
 * revision or recognized feature.
 */
export interface SemanticDiscardedEffect {
  readonly tag: 'legacy-difference-cutters'
  readonly root: SemanticNodeId
  readonly ownerOccurrence: SemanticOccurrenceIndex
}

export interface SemanticTerminalPrefixRoot {
  readonly root: SemanticNodeId
  readonly ownerOccurrence: SemanticOccurrenceIndex | null
}

export interface SemanticLanguageTerminal {
  readonly tag: 'legacy-language-error'
  /** Canonical row of the interrupted dynamic activation. */
  readonly occurrence: SemanticOccurrenceIndex
  readonly diagnosticTemplate: number
  readonly prefixFrontier: readonly SemanticTerminalPrefixRoot[]
}

export interface SemanticExecutionPlanV1 {
  readonly version: typeof SEMANTIC_PROGRAM_EXECUTION_VERSION
  /** Exact, complete topological kernel-call order; every node appears once. */
  readonly evaluationOrder: readonly SemanticNodeId[]
  readonly discardedEffects: readonly SemanticDiscardedEffect[]
  readonly terminal: SemanticLanguageTerminal | null
}

export interface SemanticDiagnosticArgument {
  readonly name: string
  readonly value: SemanticIdentityValue
}

export interface SemanticDiagnosticTemplate {
  readonly id: number
  readonly code: string
  readonly severity: 'info' | 'warning' | 'error'
  readonly operation: SemanticOperationIndex | null
  readonly arguments: readonly SemanticDiagnosticArgument[]
}

/** SPC1 payload. Every field is part of programHash. */
export interface SemanticProgramCoreV1 {
  readonly schema: 'semantic-program-core'
  readonly schemaVersion: SemanticSchemaVersion
  readonly requiredFeatures: readonly string[]
  readonly identityVersion: typeof SEMANTIC_PROGRAM_IDENTITY
  readonly language: SemanticLanguageIdentity
  readonly units: SemanticUnits
  readonly operations: readonly SemanticStaticOperation[]
  readonly occurrences: readonly SemanticOccurrence[]
  readonly nodes: readonly SemanticNode[]
  readonly execution: SemanticExecutionPlanV1
  readonly result: SemanticResult
  readonly declaredCapabilities: readonly string[]
  readonly capabilityClosure: readonly string[]
  readonly diagnosticTemplates: readonly SemanticDiagnosticTemplate[]
}

export interface SemanticSourceDescriptor {
  readonly sha256: string
  readonly utf8ByteLength: number
  readonly utf16CodeUnitLength: number
}

export interface SemanticSourceSpan {
  readonly start: number
  readonly end: number
}

export interface SemanticOperationProvenance {
  readonly operation: SemanticOperationIndex
  /** Half-open JavaScript/DOM UTF-16 offsets, bound to envelope.source. */
  readonly span: SemanticSourceSpan
  readonly label: string
}

export interface SemanticTessellationIntent {
  readonly occurrence: SemanticOccurrenceIndex
  readonly chordTolerance: number | null
  readonly angularToleranceDegrees: number | null
  readonly minSegments: number | null
  readonly maxSegments: number | null
}

export interface SemanticProgramDiagnostic {
  readonly template: number
  readonly message: string
  readonly span: SemanticSourceSpan | null
}

/** SPE1 payload. Source/provenance/policy/display fields never enter programHash. */
export interface SemanticProgramEnvelopeV1 {
  readonly schema: 'semantic-program-envelope'
  readonly schemaVersion: SemanticSchemaVersion
  readonly source: SemanticSourceDescriptor
  readonly core: SemanticProgramCoreV1
  readonly provenance: readonly SemanticOperationProvenance[]
  readonly tessellationIntents: readonly SemanticTessellationIntent[]
  readonly diagnostics: readonly SemanticProgramDiagnostic[]
}

/** Public short name; deliberately aliases the source-bound envelope, not core. */
export type SemanticProgramV1 = SemanticProgramEnvelopeV1

export const CAPABILITY_BY_KIND: Readonly<Record<SemanticNode['kind'], string>> = Object.freeze({
  box: 'construct.box',
  'sphere-analytic': 'construct.sphere.analytic',
  'sphere-polygonal': 'construct.sphere.polygonal',
  'cylinder-analytic': 'construct.cylinder.analytic',
  'cylinder-polygonal': 'construct.cylinder.polygonal',
  polyhedron: 'construct.polyhedron',
  rectangle: 'construct.rectangle',
  'circle-analytic': 'construct.circle.analytic',
  'circle-polygonal': 'construct.circle.polygonal',
  polygon: 'construct.polygon',
  transform: 'operation.transform',
  boolean: 'operation.boolean',
  hull: 'operation.hull',
  'linear-extrude': 'operation.linear-extrude',
  'rotate-extrude-analytic': 'operation.rotate-extrude.analytic',
  'rotate-extrude-polygonal': 'operation.rotate-extrude.polygonal',
  projection: 'operation.projection',
  offset: 'operation.offset',
})
