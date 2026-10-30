// lowererBuilder: MAX_SHAPES.
import type { GeometryQuality } from '../../core/build'

import {
  deriveSemanticAmbiguityGroupId,
  deriveSemanticOccurrenceId,
  deriveSemanticOperationId,
  deriveSemanticSceneEntityId,
  semanticNodeInputs,
  semanticNodeProducerOperationNames,
  semanticResultItems,
  SEMANTIC_PROGRAM_EXECUTION_VERSION,
  type SemanticColor,
  type SemanticDiagnosticArgument,
  type SemanticDimension,
  type SemanticDiscardedEffect,
  type SemanticDynamicSlot,
  type SemanticExecutionPlanV1,
  type SemanticIdentityValue,
  type SemanticNode,
  type SemanticOccurrence,
  type SemanticOperationCategory,
  type SemanticOutputRef,
  type SemanticProgramCoreV1,
  type SemanticResult,
  type SemanticStaticOperation,
  type SemanticStructuralPathSegment,
  type SemanticTessellationIntent,
  type SemanticValueType,
} from '../../core/semanticProgram'

import { sha256Hex } from '../../core/sha256'

import type {
  CallNode,
  FunctionNode,
  ModuleNode,
  OpenScadLanguageProfile,
  Statement,
  Value,
} from '../../services/openscadCompiler'

import { OpenSCADParseError } from '../../services/openscadErrors'

import {
  formatOpenScadValue,
  isOpenScadRange,
} from '../../services/openScadValueSemantics'

import type { OpenScadStableScope } from '../../services/openScadStableScope'

import { isFunctionValue } from '../../services/openScadStableExpressionEval'
import { SemanticProgramBuilder } from './SemanticProgramBuilder'


export const MAX_SHAPES = 1_000

export const MAX_EXTRUDE_SLICES = 512


export type RGBA = [number, number, number, number]


export interface SemanticShape {
  readonly node: number
  readonly dimension: SemanticDimension
  readonly producerOccurrence: number
  readonly identityOccurrence: number
  readonly color: RGBA
}


export type SemanticNodeWithoutId<T = SemanticNode> = T extends SemanticNode ? Omit<T, 'id'> : never


export function semanticValueType(
  contract: 'legacy/current' | 'openscad-viewer/brep-1',
  geometryKind: SemanticValueType['geometryKind'],
  space: SemanticValueType['space'],
): SemanticValueType {
  return {
    geometryKind,
    space,
    representation: contract === 'legacy/current' ? 'mesh' : 'analytic-brep',
    evidence: { tag: 'representation-preserving' },
  }
}


export interface RegisteredOperations {
  readonly operations: SemanticStaticOperation[]
  readonly provenance: Array<{ operation: number; span: { start: number; end: number }; label: string }>
  readonly byStatement: Map<Statement, number>
  readonly bodyByStatement: Map<CallNode | ModuleNode, number>
  readonly alternativeByCall: Map<CallNode, number>
  readonly expansionByCall: Map<CallNode, number>
}


export interface EvaluationBudget { ops: number }


export interface SemanticRuntimeCheckpoint {
  readonly nodes: number
  readonly evaluationOrder: number
  readonly discardedEffects: number
  readonly occurrences: number
  readonly tessellationIntents: number
  readonly paletteIndex: number
  readonly terminalOccurrence: number | null
  readonly invocationDuplicates: ReadonlyMap<string, number>
  readonly outputCounts: ReadonlyMap<number, number>
}


export interface SemanticDiagnosticCheckpoint {
  readonly templates: number
  readonly diagnostics: number
  readonly warnings: number
}


export interface PassedCallChildren {
  readonly owner: CallNode
  readonly bodyOccurrence: number
  readonly expansionOperation: number
  readonly statements: readonly Statement[]
  readonly env: Map<string, Value>
  readonly scope?: OpenScadStableScope
  readonly continuation?: PassedCallChildren
}


export interface EvalContext {
  readonly source: string
  readonly languageProfile: OpenScadLanguageProfile
  readonly env: Map<string, Value>
  readonly functions: Map<string, FunctionNode>
  readonly modules: Map<string, ModuleNode>
  readonly builder: SemanticProgramBuilder
  readonly quality: GeometryQuality
  readonly depth: number
  readonly budget: EvaluationBudget
  readonly reduced: { value: boolean }
  readonly valueBudget: { used: number }
  readonly valueWeights: WeakMap<Value[], number>
  readonly valueDepths: WeakMap<Value[], number>
  readonly parentOccurrence: number | null
  readonly pendingSlots: readonly SemanticDynamicSlot[]
  readonly callChildren?: Readonly<PassedCallChildren>
  readonly functionStack: readonly string[]
  readonly moduleStack: readonly string[]
  readonly stableScope?: OpenScadStableScope
  readonly scopeVisibleBefore?: number
  /** Once `!` wins at runtime, nested root markers are ordinary calls. */
  readonly viewportRootLocked?: boolean
  /** The winning call ignores its own `#`/`%` chain, as OpenSCAD does. */
  readonly viewportRootOwner?: CallNode
  readonly shouldAbort?: () => boolean
}


export class StableViewportRootSelection {
  constructor(
    readonly node: CallNode,
    readonly context: EvalContext,
    readonly diagnosticsBeforeCandidate: SemanticDiagnosticCheckpoint,
  ) {}
}


export interface SemanticLoweringOptions {
  readonly languageProfile?: OpenScadLanguageProfile
  readonly quality?: GeometryQuality
  /** Host animation position exposed as OpenSCAD's dynamic `$t` (0..1). */
  readonly animationTime?: number
  readonly shouldAbort?: () => boolean
  /** Qualification-only: preserve a deterministic evaluation failure as a kernel-prefix plan. */
  readonly captureTerminalFailure?: boolean
}


export class LegacyBindingCompatibilityError extends TypeError {
  constructor(message: string) {
    super(message)
    this.name = 'TypeError'
  }
}


export class StableBuiltinValueError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'StableBuiltinValueError'
  }
}


export const PALETTE: readonly RGBA[] = Object.freeze([
  [0.26, 0.52, 0.96, 1], [0.96, 0.52, 0.26, 1],
  [0.26, 0.86, 0.56, 1], [0.86, 0.26, 0.66, 1],
  [0.96, 0.86, 0.26, 1], [0.46, 0.76, 0.86, 1],
  [0.76, 0.56, 0.96, 1], [0.56, 0.86, 0.36, 1],
])


export const CSS_COLORS: Readonly<Record<string, RGBA>> = Object.freeze({
  red: [1, 0, 0, 1], green: [0, 0.5, 0, 1], blue: [0, 0, 1, 1],
  yellow: [1, 1, 0, 1], cyan: [0, 1, 1, 1], magenta: [1, 0, 1, 1],
  white: [1, 1, 1, 1], black: [0, 0, 0, 1], orange: [1, 0.65, 0, 1],
  gray: [0.5, 0.5, 0.5, 1], grey: [0.5, 0.5, 0.5, 1],
  pink: [1, 0.75, 0.8, 1], purple: [0.5, 0, 0.5, 1], brown: [0.65, 0.16, 0.16, 1],
  lime: [0, 1, 0, 1], navy: [0, 0, 0.5, 1], teal: [0, 0.5, 0.5, 1],
})


export function canonicalNumber(value: number): number {
  return Object.is(value, -0) ? 0 : value
}


export function identityValue(value: Value, depth = 0): SemanticIdentityValue {
  if (depth > 32) throw new TypeError('Identity value exceeds 32 nested levels')
  if (value === undefined) return { tag: 'undefined' }
  if (value === null) return { tag: 'null' }
  if (typeof value === 'boolean') return { tag: 'boolean', value }
  if (typeof value === 'number') return { tag: 'number', value: canonicalNumber(value) }
  if (typeof value === 'string') return { tag: 'string', value }
  if (isFunctionValue(value)) return { tag: 'string', value: `function:${value.name ?? '<anonymous>'}` }
  if (isOpenScadRange(value)) return { tag: 'string', value: `range:${formatOpenScadValue(value)}` }
  return { tag: 'vector', items: value.map(item => identityValue(item, depth + 1)) }
}


export function operationCategory(
  name: string,
  moduleNames: ReadonlySet<string>,
  languageProfile: OpenScadLanguageProfile,
): SemanticOperationCategory {
  if (moduleNames.has(name)) return 'module'
  if (['translate', 'rotate', 'scale', 'mirror', 'multmatrix'].includes(name)) return 'transform'
  if (['union', 'difference', 'intersection', 'hull'].includes(name)) return 'boolean'
  if (['if', 'let', 'for', 'children', 'group', 'render'].includes(name)
    || (languageProfile === 'openscad/stable-2021.01' && ['assign', 'child'].includes(name))
    || (languageProfile === 'openscad/stable-2021.01' && name === 'echo')) return 'control'
  if (name === 'assert') return 'assertion'
  if (name === 'color') return 'presentation'
  return 'geometry'
}


export function semanticOperationName(
  statement: Exclude<Statement, FunctionNode>,
  languageProfile: OpenScadLanguageProfile,
): string {
  if (statement.type === 'assign') return '$assign'
  if (languageProfile !== 'openscad/stable-2021.01' || statement.type !== 'call') {
    return statement.name
  }
  switch (statement.name) {
    // Semantic-program v1 intentionally has a closed production vocabulary.
    // Historical spellings are lowered onto the equivalent frozen operation;
    // provenance still retains the authored spelling and source span.
    case 'assign': return '$assign'
    case 'child': return 'children'
    case 'dxf_linear_extrude': return 'linear_extrude'
    case 'dxf_rotate_extrude': return 'rotate_extrude'
    default: return statement.name
  }
}
