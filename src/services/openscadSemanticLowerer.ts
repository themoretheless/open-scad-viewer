import type { GeometryQuality } from '../core/build'
import {
  deriveSemanticAmbiguityGroupId,
  deriveSemanticCapabilityClosure,
  deriveSemanticOccurrenceId,
  deriveSemanticOperationId,
  deriveSemanticSceneEntityId,
  semanticNodeInputs,
  semanticNodeProducerOperationNames,
  semanticResultItems,
  SEMANTIC_PROGRAM_CAPABILITY_GRAPH,
  SEMANTIC_PROGRAM_EXECUTION_FEATURE,
  SEMANTIC_PROGRAM_EXECUTION_VERSION,
  SEMANTIC_PROGRAM_IDENTITY,
  type SemanticColor,
  type SemanticDiagnosticArgument,
  type SemanticDiscardedEffect,
  type SemanticExecutionPlanV1,
  type SemanticDimension,
  type SemanticDynamicSlot,
  type SemanticIdentityValue,
  type SemanticMatrix4,
  type SemanticNode,
  type SemanticOccurrence,
  type SemanticOperationCategory,
  type SemanticOutputRef,
  type SemanticProgramCoreV1,
  type SemanticProgramV1,
  type SemanticResult,
  type SemanticStaticOperation,
  type SemanticStructuralPathSegment,
  type SemanticTessellationIntent,
  type SemanticValueType,
} from '../core/semanticProgram'
import { clamp01 } from './math3d'
import { parseGeometrySourceRoutingHeader } from '../core/geometryRouting'
import { sha256Hex } from '../core/sha256'
import {
  TT,
  compileOpenSCAD,
  hasOpenScadViewportModifier,
  type CallNode,
  type Expr,
  type ExpressionArgument,
  type FunctionNode,
  type FunctionValue,
  type ModuleNode,
  type OpenScadLanguageProfile,
  type Statement,
  type Value,
} from './openscadCompiler'
import { AbortedError, OpenSCADParseError } from './openscadErrors'
import {
  createDeferredOpenScadBuiltinArguments,
  evaluateOpenScadBuiltinFunction,
  type OpenScadBuiltinValue,
} from './openScadBuiltinFunctions'
import {
  formatOpenScadValue,
  isOpenScadRange,
  openScadIndex,
  openScadMember,
  openScadTruthy,
  openScadUnary,
} from './openScadValueSemantics'
import { OpenScadStableScope } from './openScadStableScope'
import {
  MAX_EVAL_DEPTH,
  MAX_EVAL_OPS,
  MAX_EVALUATED_VALUE_UNITS,
  MAX_EXPRESSION_DEPTH,
  MAX_RANGE_ITEMS,
  MAX_VALUE_ELEMENTS,
  appendComprehensionValues,
  compactDiagnosticText,
  createStableExpressionEvaluators,
  evaluationError,
  isFunctionValue,
  isListComprehensionExpression,
  isStableProfile,
  overlayDynamicVariables,
  registerArrayValue,
  stableOverlayContext,
  type StableFunctionValue,
} from './openScadStableExpressionEval'
import { createOpenScadStableRuntimeVariables } from './openScadStableRuntime'
import {
  attestSemanticProgram,
  encodeSemanticProgram,
} from './semanticProgramCodec'
import { normalizeSemanticProgram, semanticSourceDescriptor } from './semanticProgramValidator'
import {
  type SemanticLoweringReport,
  type SemanticLoweringSuccess,
} from './semanticProgramTrust'
import { canonicalNumber, finiteNumber, resolveStableVariable, stableWarning } from './openscadSemanticLowerer/semanticValueType'
import { evalBuiltin, evalExpression } from './openscadSemanticLowerer/evalExpression'
import { requestedSegments } from './openscadSemanticLowerer/evalPreparedNodes'
export { lowerOpenSCADToSemanticProgramUnchecked } from './openscadSemanticLowerer/lowerOpenSCADToSemanticProgramUnchecked'

export {
  type SemanticLoweringReport,
  type SemanticLoweringSuccess,
} from './semanticProgramTrust'

export const MAX_SHAPES = 1_000
export const MAX_FN = 256
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

export interface RegisteredOperations {
  readonly operations: SemanticStaticOperation[]
  readonly provenance: Array<{ operation: number; span: { start: number; end: number }; label: string }>
  readonly byStatement: Map<Statement, number>
  readonly bodyByStatement: Map<CallNode | ModuleNode, number>
  readonly alternativeByCall: Map<CallNode, number>
  readonly expansionByCall: Map<CallNode, number>
}

interface EvaluationBudget { ops: number }

interface SemanticRuntimeCheckpoint {
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

interface SemanticDiagnosticCheckpoint {
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

const PALETTE: readonly RGBA[] = Object.freeze([
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

export class SemanticProgramBuilder {
  readonly nodes: SemanticNode[] = []
  readonly evaluationOrder: number[] = []
  readonly discardedEffects: SemanticDiscardedEffect[] = []
  readonly occurrences: SemanticOccurrence[] = []
  readonly tessellationIntents: SemanticTessellationIntent[] = []
  readonly diagnosticTemplates: SemanticProgramCoreV1['diagnosticTemplates'][number][] = []
  readonly diagnostics: Array<{ template: number; message: string; span: { start: number; end: number } | null }> = []
  readonly warnings: string[] = []
  terminalOccurrence: number | null = null
  private paletteIndex = 0
  private readonly invocationDuplicates = new Map<string, number>()
  private readonly outputCounts = new Map<number, number>()

  constructor(
    readonly source: string,
    readonly languageContract: 'legacy/current' | 'openscad-viewer/brep-1',
    readonly registered: RegisteredOperations,
  ) {}

  nextColor(): RGBA {
    const color = PALETTE[this.paletteIndex++ % PALETTE.length]
    return [...color] as RGBA
  }

  runtimeCheckpoint(): SemanticRuntimeCheckpoint {
    return {
      nodes: this.nodes.length,
      evaluationOrder: this.evaluationOrder.length,
      discardedEffects: this.discardedEffects.length,
      occurrences: this.occurrences.length,
      tessellationIntents: this.tessellationIntents.length,
      paletteIndex: this.paletteIndex,
      terminalOccurrence: this.terminalOccurrence,
      invocationDuplicates: new Map(this.invocationDuplicates),
      outputCounts: new Map(this.outputCounts),
    }
  }

  /**
   * Full rendering evaluates `%` for language effects, then removes its
   * kernel-visible subtree. Diagnostics intentionally survive this rollback.
   */
  rollbackRuntime(checkpoint: SemanticRuntimeCheckpoint): void {
    this.nodes.length = checkpoint.nodes
    this.evaluationOrder.length = checkpoint.evaluationOrder
    this.discardedEffects.length = checkpoint.discardedEffects
    this.occurrences.length = checkpoint.occurrences
    this.tessellationIntents.length = checkpoint.tessellationIntents
    this.paletteIndex = checkpoint.paletteIndex
    this.terminalOccurrence = checkpoint.terminalOccurrence
    this.invocationDuplicates.clear()
    for (const [key, value] of checkpoint.invocationDuplicates) this.invocationDuplicates.set(key, value)
    this.outputCounts.clear()
    for (const [key, value] of checkpoint.outputCounts) this.outputCounts.set(key, value)
  }

  diagnosticCheckpoint(): SemanticDiagnosticCheckpoint {
    return {
      templates: this.diagnosticTemplates.length,
      diagnostics: this.diagnostics.length,
      warnings: this.warnings.length,
    }
  }

  inheritDiagnostics(
    source: SemanticProgramBuilder,
    checkpoint: SemanticDiagnosticCheckpoint = source.diagnosticCheckpoint(),
  ): void {
    const operationById = new Map(
      this.registered.operations.map(operation => [operation.operationId, operation.id] as const),
    )
    for (const template of source.diagnosticTemplates.slice(0, checkpoint.templates)) {
      const operation = template.operation === null
        ? null
        : operationById.get(source.registered.operations[template.operation].operationId) ?? null
      this.diagnosticTemplates.push({
        ...template,
        id: this.diagnosticTemplates.length,
        operation,
      })
    }
    for (const diagnostic of source.diagnostics.slice(0, checkpoint.diagnostics)) {
      this.diagnostics.push({ ...diagnostic })
    }
    this.warnings.push(...source.warnings.slice(0, checkpoint.warnings))
  }

  addInternalNode(node: SemanticNodeWithoutId): number {
    const id = this.nodes.length
    this.nodes.push({ ...node, id } as SemanticNode)
    this.evaluationOrder.push(id)
    return id
  }

  invocation(node: Statement, parent: number | null, slots: readonly SemanticDynamicSlot[] = []): number {
    const operation = this.registered.byStatement.get(node)
    if (operation === undefined) throw new Error('Semantic operation registry is incomplete')
    return this.invocationOperation(operation, parent, slots)
  }

  captureTerminalOccurrence(occurrence: number): void {
    if (this.terminalOccurrence !== null) return
    const occurrenceId = this.occurrences[occurrence]?.occurrenceId
    if (occurrenceId === undefined) throw new Error('Semantic terminal occurrence is absent')
    const canonical = this.occurrences.findIndex(row => row.occurrenceId === occurrenceId)
    if (canonical < 0) throw new Error('Semantic terminal occurrence has no canonical row')
    this.terminalOccurrence = canonical
  }

  invocationOperation(operation: number, parent: number | null, slots: readonly SemanticDynamicSlot[] = []): number {
    const authoredSlots = slots.length === 0
      ? [{ name: '$evaluation', value: { tag: 'undefined' } as const, duplicateOrdinal: 0 }]
      : slots
    const normalizedSlots = authoredSlots.map((slot, slotIndex) => {
      const valueKey = JSON.stringify(slot.value)
      const key = `${String(parent)}|${operation}|${slotIndex}|${slot.name.length}:${slot.name}${valueKey.length}:${valueKey}`
      const duplicateOrdinal = this.invocationDuplicates.get(key) ?? 0
      this.invocationDuplicates.set(key, duplicateOrdinal + 1)
      return { name: slot.name, value: slot.value, duplicateOrdinal }
    })
    return this.addOccurrence(operation, parent, normalizedSlots, null, null)
  }

  private addOccurrence(
    operation: number,
    parent: number | null,
    slots: readonly SemanticDynamicSlot[],
    node: number | null,
    outputOrdinal: number | null,
  ): number {
    const parentId = parent === null ? null : this.occurrences[parent].occurrenceId
    const staticOperationParent = this.registered.operations[operation].parent
    let staticParent: number | null = null
    if (staticOperationParent !== null) {
      let cursor = parent
      let crossedNonExpansionFrame = false
      while (cursor !== null && this.occurrences[cursor].operation !== staticOperationParent) {
        const category = this.registered.operations[this.occurrences[cursor].operation].category
        if (category !== 'control' && category !== 'module') crossedNonExpansionFrame = true
        cursor = this.occurrences[cursor].parent
      }
      if (cursor === null) throw new Error('Semantic static parent occurrence is missing')
      staticParent = cursor
      const isExpansion = this.registered.operations[operation].name === '$expansion'
      if (isExpansion) {
        if (!this.isChildrenExpansionContinuation(operation, parent, staticParent, slots)) {
          throw new Error('Semantic children expansion is not bound to its caller continuation')
        }
      } else if (crossedNonExpansionFrame) {
        throw new Error('Semantic runtime frame cannot be bound to its static parent')
      }
    }
    const staticParentId = staticParent === null ? null : this.occurrences[staticParent].occurrenceId
    const occurrenceId = deriveSemanticOccurrenceId(
      parentId,
      staticParentId,
      this.registered.operations[operation].operationId,
      slots,
    )
    const id = this.occurrences.length
    const sceneEntityId = outputOrdinal === null ? null : deriveSemanticSceneEntityId(occurrenceId, outputOrdinal)
    this.occurrences.push({
      id,
      occurrenceId,
      operation,
      parent,
      staticParent,
      dynamicSlots: slots,
      node,
      outputOrdinal,
      sceneEntityId,
    })
    return id
  }

  /**
   * `children()` is the sole continuation edge that crosses an activated
   * module definition. Its runtime parent remains the callee-side children()
   * frame, while its static parent is the caller-side `$body` frame. Keep the
   * exception structural and closed so transforms in every other ancestry
   * path remain forbidden.
   */
  private isChildrenExpansionContinuation(
    operation: number,
    parent: number | null,
    staticParent: number,
    slots: readonly SemanticDynamicSlot[],
  ): boolean {
    if (parent === null) return false
    const operations = this.registered.operations
    const expansionOperation = operations[operation]
    const childrenOccurrence = this.occurrences[parent]
    const childrenOperation = operations[childrenOccurrence.operation]
    const callerBodyOccurrence = this.occurrences[staticParent]
    const callerBodyOperation = operations[callerBodyOccurrence.operation]
    const childrenSegment = childrenOperation.structuralPath.at(-1)
    const expansionSegment = expansionOperation.structuralPath.at(-1)
    const bodySegment = callerBodyOperation.structuralPath.at(-1)
    const detachedContinuationExpansion = expansionOperation.name === '$expansion'
      && expansionOperation.category === 'control'
      && expansionSegment?.kind === 'control'
      && expansionSegment.name === '$expansion'
      && expansionOperation.parent === childrenOccurrence.operation
      && childrenOperation.name === 'children'
      && staticParent === parent
    if (detachedContinuationExpansion) return true
    if (expansionOperation.name !== '$expansion'
      || expansionOperation.category !== 'control'
      || expansionSegment?.kind !== 'control'
      || expansionSegment.name !== '$expansion'
      || expansionOperation.parent !== callerBodyOccurrence.operation
      || callerBodyOperation.name !== '$body'
      || callerBodyOperation.category !== 'control'
      || bodySegment?.kind !== 'body'
      || bodySegment.name !== '$body'
      || childrenOperation.name !== 'children'
      || childrenOperation.category !== 'control'
      || childrenSegment?.kind !== 'control'
      || childrenSegment.name !== 'children'
    ) return false

    let staticChildCount = 0
    let soleStaticChild = -1
    for (const candidate of operations) {
      if (candidate.parent !== callerBodyOccurrence.operation) continue
      staticChildCount++
      soleStaticChild = candidate.id
    }
    if (staticChildCount !== 1 || soleStaticChild !== operation) return false

    const sameSlots = (left: readonly SemanticDynamicSlot[], right: readonly SemanticDynamicSlot[]) => (
      left.length === right.length
      && left.every((slot, index) => slot.name === right[index].name
        && JSON.stringify(slot.value) === JSON.stringify(right[index].value))
    )
    if (childrenOccurrence.dynamicSlots.length !== 1
      || childrenOccurrence.dynamicSlots[0].name !== '$index'
      || !sameSlots(childrenOccurrence.dynamicSlots, slots)) return false

    let cursor = childrenOccurrence.parent
    let definitionOccurrence: SemanticOccurrence | null = null
    while (cursor !== null && cursor !== staticParent) {
      const candidate = this.occurrences[cursor]
      const candidateOperation = operations[candidate.operation]
      const candidatePath = candidateOperation.structuralPath
      const childrenPath = childrenOperation.structuralPath
      const lexicallyContainsChildren = candidatePath.length < childrenPath.length
        && candidatePath.every((segment, index) => {
          const childSegment = childrenPath[index]
          return segment.kind === childSegment.kind
            && segment.name === childSegment.name
            && segment.ordinal === childSegment.ordinal
        })
      if (candidateOperation.structuralPath.at(-1)?.kind === 'module'
        && lexicallyContainsChildren) {
        definitionOccurrence = candidate
        break
      }
      cursor = candidate.parent
    }
    if (!definitionOccurrence || definitionOccurrence.parent !== staticParent) return false
    const callerOccurrenceIndex = callerBodyOccurrence.parent
    if (callerOccurrenceIndex === null) return false
    const callerOccurrence = this.occurrences[callerOccurrenceIndex]
    const callerOperation = operations[callerOccurrence.operation]
    const definitionOperation = operations[definitionOccurrence.operation]
    const definitionPath = definitionOperation.structuralPath
    const childrenPath = childrenOperation.structuralPath
    const lexicallyContainsChildren = definitionPath.length < childrenPath.length
      && definitionPath.every((segment, index) => {
        const candidate = childrenPath[index]
        return segment.kind === candidate.kind
          && segment.name === candidate.name
          && segment.ordinal === candidate.ordinal
      })
    let definitionRuntimeChildren = 0
    let soleDefinitionRuntimeChild = -1
    let expansionRuntimeChildren = 0
    for (const candidate of this.occurrences) {
      if (candidate.parent === staticParent) {
        definitionRuntimeChildren++
        soleDefinitionRuntimeChild = candidate.id
      }
      if (candidate.parent === parent) expansionRuntimeChildren++
    }
    return callerBodyOperation.parent === callerOccurrence.operation
      && callerOperation.category === 'module'
      && callerOperation.structuralPath.at(-1)?.kind === 'call'
      && definitionOperation.category === 'module'
      && definitionOperation.structuralPath.at(-1)?.kind === 'module'
      && callerOperation.name === definitionOperation.name
      && sameSlots(callerOccurrence.dynamicSlots, definitionOccurrence.dynamicSlots)
      && lexicallyContainsChildren
      && definitionRuntimeChildren === 1
      && soleDefinitionRuntimeChild === definitionOccurrence.id
      && expansionRuntimeChildren === 0
  }

  produce(
    baseOccurrence: number,
    node: SemanticNodeWithoutId,
    color: RGBA,
    identityOccurrence?: number,
  ): SemanticShape {
    const id = this.nodes.length
    this.nodes.push({ ...node, id } as SemanticNode)
    this.evaluationOrder.push(id)
    const base = this.occurrences[baseOccurrence]
    const expectedOrdinal = this.outputCounts.get(baseOccurrence) ?? 0
    const outputOrdinal = expectedOrdinal
    this.outputCounts.set(baseOccurrence, expectedOrdinal + 1)
    const producer = expectedOrdinal === 0
      ? baseOccurrence
      : this.occurrences.length
    const outputRow: SemanticOccurrence = {
      ...base,
      id: producer,
      node: id,
      outputOrdinal,
      sceneEntityId: deriveSemanticSceneEntityId(base.occurrenceId, outputOrdinal),
    }
    if (producer === baseOccurrence) this.occurrences[baseOccurrence] = outputRow
    else this.occurrences.push(outputRow)
    return {
      node: id,
      dimension: node.valueType.space === 'd2' ? 'region2' : 'solid3',
      producerOccurrence: producer,
      identityOccurrence: identityOccurrence ?? producer,
      color,
    }
  }

  alias(baseOccurrence: number, shapes: readonly SemanticShape[], reownIdentity: boolean): SemanticShape[] {
    return shapes.map((shape, outputOrdinal) => {
      const base = this.occurrences[baseOccurrence]
      const producer = outputOrdinal === 0 ? baseOccurrence : this.occurrences.length
      const row: SemanticOccurrence = {
        ...base,
        id: producer,
        node: shape.node,
        outputOrdinal,
        sceneEntityId: deriveSemanticSceneEntityId(base.occurrenceId, outputOrdinal),
      }
      if (producer === baseOccurrence) this.occurrences[baseOccurrence] = row
      else this.occurrences.push(row)
      return {
        ...shape,
        producerOccurrence: producer,
        identityOccurrence: reownIdentity ? producer : shape.identityOccurrence,
      }
    })
  }

  addTessellationIntent(occurrence: number, requestedSegments: number | null, env: Map<string, Value>): void {
    const chord = env.get('$fs')
    const angle = env.get('$fa')
    this.tessellationIntents.push({
      occurrence,
      chordTolerance: typeof chord === 'number' && chord > 0 ? canonicalNumber(chord) : null,
      angularToleranceDegrees: typeof angle === 'number' && angle > 0 ? canonicalNumber(angle) : null,
      minSegments: requestedSegments === null ? null : Math.max(3, requestedSegments),
      maxSegments: requestedSegments === null ? null : Math.max(3, requestedSegments),
    })
  }

  warn(
    operation: number | null,
    code: string,
    args: readonly SemanticDiagnosticArgument[],
    message: string,
    span: { start: number; end: number } | null,
    dedupe = true,
  ): void {
    const id = this.diagnosticTemplates.length
    this.diagnosticTemplates.push({ id, code, severity: 'warning', operation, arguments: args })
    this.diagnostics.push({ template: id, message, span })
    if (!dedupe || !this.warnings.includes(message)) this.warnings.push(message)
  }

  terminal(
    error: OpenSCADParseError | LegacyBindingCompatibilityError,
    occurrence: number,
  ): number {
    const id = this.diagnosticTemplates.length
    const detail = error instanceof OpenSCADParseError ? error.detail : error.message
    this.diagnosticTemplates.push({
      id,
      code: 'LEGACY_LANGUAGE_ERROR',
      severity: 'error',
      operation: this.occurrences[occurrence].operation,
      arguments: [
        {
          name: 'errorName',
          value: { tag: 'string', value: error.name },
        },
        {
          name: 'detailSha256',
          value: { tag: 'string', value: sha256Hex(new TextEncoder().encode(detail)) },
        },
      ],
    })
    this.diagnostics.push({
      template: id,
      message: error.message,
      span: error instanceof OpenSCADParseError
        ? { start: error.start, end: error.end }
        : null,
    })
    return id
  }
}

export const {
  stableValueContext,
  enterStableStatementScope,
  evaluateSequentialBindings,
  stableIterable,
  evalListComprehension,
  resolveStableExpressionArguments,
  evalAssertExpression,
  evalEchoExpression,
  evalBinaryExpression,
  evalFunctionCall,
  invokeUserFunction,
} = createStableExpressionEvaluators<EvalContext>({
  evalExpression: (expr, ctx, depth) => evalExpression(expr, ctx, depth),
  evalBuiltin: (expr, ctx, depth) => evalBuiltin(expr, ctx, depth),
  resolveStableVariable: (name, ctx, position) => resolveStableVariable(name, ctx, position),
  warn: (ctx, position, message) => stableWarning(ctx, position, message),
  scopeWarn: (ctx, position, message) => stableWarning(ctx, position, message, 'W_OPENSCAD_SCOPE'),
  echoWarn: (ctx, position, message) => stableWarning(ctx, position, message, 'W_OPENSCAD_ECHO', false),
  finiteNumber: (value, ctx, position, label) => finiteNumber(value, ctx, position, label),
  canonicalNumber,
})

export interface EvaluatedStableCompatibilityArguments {
  readonly authored: readonly ExpressionArgument[]
  readonly values: ReadonlyMap<string, Value>
  readonly evaluated: ReadonlyMap<ExpressionArgument, Value>
}

export const IDENTITY_MATRIX: SemanticMatrix4 = [
  1, 0, 0, 0,
  0, 1, 0, 0,
  0, 0, 1, 0,
  0, 0, 0, 1,
]

export interface SemanticReducedShape {
  readonly node: number
  readonly dimension: SemanticDimension
  readonly color: RGBA
  readonly identityOccurrence: number | null
}
