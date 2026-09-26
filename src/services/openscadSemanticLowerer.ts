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

export {
  type SemanticLoweringReport,
  type SemanticLoweringSuccess,
} from './semanticProgramTrust'

const MAX_SHAPES = 1_000
const MAX_FN = 256
const MAX_EXTRUDE_SLICES = 512

type RGBA = [number, number, number, number]

interface SemanticShape {
  readonly node: number
  readonly dimension: SemanticDimension
  readonly producerOccurrence: number
  readonly identityOccurrence: number
  readonly color: RGBA
}

type SemanticNodeWithoutId<T = SemanticNode> = T extends SemanticNode ? Omit<T, 'id'> : never

function semanticValueType(
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

interface RegisteredOperations {
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

interface PassedCallChildren {
  readonly owner: CallNode
  readonly bodyOccurrence: number
  readonly expansionOperation: number
  readonly statements: readonly Statement[]
  readonly env: Map<string, Value>
  readonly scope?: OpenScadStableScope
  readonly continuation?: PassedCallChildren
}

interface EvalContext {
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

class StableViewportRootSelection {
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

class LegacyBindingCompatibilityError extends TypeError {
  constructor(message: string) {
    super(message)
    this.name = 'TypeError'
  }
}

class StableBuiltinValueError extends Error {
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

const CSS_COLORS: Readonly<Record<string, RGBA>> = Object.freeze({
  red: [1, 0, 0, 1], green: [0, 0.5, 0, 1], blue: [0, 0, 1, 1],
  yellow: [1, 1, 0, 1], cyan: [0, 1, 1, 1], magenta: [1, 0, 1, 1],
  white: [1, 1, 1, 1], black: [0, 0, 0, 1], orange: [1, 0.65, 0, 1],
  gray: [0.5, 0.5, 0.5, 1], grey: [0.5, 0.5, 0.5, 1],
  pink: [1, 0.75, 0.8, 1], purple: [0.5, 0, 0.5, 1], brown: [0.65, 0.16, 0.16, 1],
  lime: [0, 1, 0, 1], navy: [0, 0, 0.5, 1], teal: [0, 0.5, 0.5, 1],
})

function canonicalNumber(value: number): number {
  return Object.is(value, -0) ? 0 : value
}

function identityValue(value: Value, depth = 0): SemanticIdentityValue {
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

function operationCategory(
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

function semanticOperationName(
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

function registerOperations(
  ast: readonly Statement[],
  moduleNames: ReadonlySet<string>,
  languageProfile: OpenScadLanguageProfile,
  detachedRoot?: CallNode,
  detachedContinuations?: ReadonlyMap<CallNode, Readonly<PassedCallChildren>>,
): RegisteredOperations {
  const operations: SemanticStaticOperation[] = []
  const provenance: RegisteredOperations['provenance'] = []
  const byStatement = new Map<Statement, number>()
  const bodyByStatement = new Map<CallNode | ModuleNode, number>()
  const alternativeByCall = new Map<CallNode, number>()
  const expansionByCall = new Map<CallNode, number>()

  const addFrame = (
    owner: CallNode | ModuleNode,
    parent: number,
    parentPath: readonly SemanticStructuralPathSegment[],
    name: '$body' | '$then' | '$else' | '$expansion',
    kind: 'body' | 'branch' | 'control',
  ): number => {
    const structuralPath = [...parentPath, { kind, name, ordinal: 0 } as const]
    const id = operations.length
    operations.push({
      id,
      operationId: deriveSemanticOperationId(structuralPath),
      parent,
      childOrdinal: 0,
      name,
      category: 'control',
      structuralPath,
      identityEvidence: 'structural-unique',
      ambiguityGroup: null,
    })
    provenance.push({
      operation: id,
      span: { start: owner.p, end: owner.end },
      label: `${owner.name}.${name.slice(1)}`,
    })
    return id
  }

  const visitSiblings = (
    statements: readonly Statement[],
    parent: number | null,
    parentPath: readonly SemanticStructuralPathSegment[],
    allowDetachedRoot = false,
  ) => {
    // Function declarations affect the value environment but do not create a
    // geometry operation or runtime occurrence in the semantic DAG.
    const candidates: Array<Exclude<Statement, FunctionNode>> = []
    const appendCandidates = (items: readonly Statement[]) => {
      for (const statement of items) {
        if (statement === detachedRoot && !allowDetachedRoot) continue
        if (statement.type === 'function') continue
        // Frozen semantic-program v1 has no effect-only occurrence production
        // rule. In the full frontend profile echo is therefore represented as
        // a diagnostic effect while its transparent children occupy the same
        // structural scope.
        if (languageProfile === 'openscad/stable-2021.01'
          && statement.type === 'call' && statement.name === 'echo') {
          appendCandidates(statement.children)
        } else candidates.push(statement)
      }
    }
    appendCandidates(statements)
    const totals = new Map<string, number>()
    candidates.forEach(statement => {
      const category = statement.type === 'assign'
        ? 'control'
        : statement.type === 'module' ? 'module' : operationCategory(statement.name, moduleNames, languageProfile)
      const name = semanticOperationName(statement, languageProfile)
      const key = `${category}\0${name}`
      totals.set(key, (totals.get(key) ?? 0) + 1)
    })
    const ordinals = new Map<string, number>()
    for (const statement of candidates) {
      const category = statement.type === 'assign'
        ? 'control'
        : statement.type === 'module' ? 'module' : operationCategory(statement.name, moduleNames, languageProfile)
      const name = semanticOperationName(statement, languageProfile)
      const key = `${category}\0${name}`
      const ordinal = ordinals.get(key) ?? 0
      ordinals.set(key, ordinal + 1)
      const segment: SemanticStructuralPathSegment = {
        kind: statement.type === 'module' ? 'module' : category === 'control' ? 'control' : 'call',
        name,
        ordinal,
      }
      const structuralPath = [...parentPath, segment]
      const ambiguous = (totals.get(key) ?? 0) > 1
      const id = operations.length
      const operation: SemanticStaticOperation = {
        id,
        operationId: deriveSemanticOperationId(structuralPath),
        parent,
        childOrdinal: ordinal,
        name,
        category,
        structuralPath,
        identityEvidence: ambiguous ? 'same-name-positional' : 'structural-unique',
        ambiguityGroup: ambiguous
          ? deriveSemanticAmbiguityGroupId(parentPath, category, name)
          : null,
      }
      operations.push(operation)
      provenance.push({
        operation: id,
        span: { start: statement.p, end: statement.end },
        label: statement.type === 'assign'
          ? `${statement.name}=`
          : `${statement.name}${statement.type === 'call' ? '()' : ''}`,
      })
      byStatement.set(statement, id)
      if (statement.type === 'assign') {
        continue
      } else if (statement.type === 'module') {
        const body = addFrame(statement, id, structuralPath, '$body', 'body')
        bodyByStatement.set(statement, body)
        visitSiblings(statement.children, body, operations[body].structuralPath)
      } else if (statement.name === 'if') {
        const thenFrame = addFrame(statement, id, structuralPath, '$then', 'branch')
        bodyByStatement.set(statement, thenFrame)
        visitSiblings(statement.children, thenFrame, operations[thenFrame].structuralPath)
        const elseFrame = addFrame(statement, id, structuralPath, '$else', 'branch')
        alternativeByCall.set(statement, elseFrame)
        visitSiblings(statement.alternative, elseFrame, operations[elseFrame].structuralPath)
      } else if ((statement.name === 'children' || statement.name === 'child')
        && detachedContinuations?.has(statement)) {
        // children() owns a runtime caller continuation rather than syntactic
        // children. When it becomes a detached root, give that continuation a
        // real static expansion frame in the root registry so replay can keep
        // the selected caller geometry without recreating discarded ancestors.
        const expansion = addFrame(statement, id, structuralPath, '$expansion', 'control')
        expansionByCall.set(statement, expansion)
        visitSiblings(
          detachedContinuations.get(statement)!.statements,
          expansion,
          operations[expansion].structuralPath,
        )
      } else {
        if (statement.children.length > 0 || category === 'module') {
          const body = addFrame(statement, id, structuralPath, '$body', 'body')
          bodyByStatement.set(statement, body)
          let childParent = body
          if (category === 'module') {
            childParent = addFrame(statement, body, operations[body].structuralPath, '$expansion', 'control')
            expansionByCall.set(statement, childParent)
          }
          visitSiblings(statement.children, childParent, operations[childParent].structuralPath)
        }
      }
    }
  }
  const roots = detachedRoot === undefined
    ? ast
    : [...ast.filter(statement => statement !== detachedRoot), detachedRoot]
  visitSiblings(roots, null, [], detachedRoot !== undefined)
  return { operations, provenance, byStatement, bodyByStatement, alternativeByCall, expansionByCall }
}

class SemanticProgramBuilder {
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

function poll(ctx: EvalContext): void {
  if (ctx.shouldAbort?.()) throw new AbortedError()
}

function registerStringValue(value: string, ctx: EvalContext, position: number, label = 'Evaluation'): string {
  ctx.valueBudget.used += Math.max(1, value.length)
  if (ctx.valueBudget.used > MAX_EVALUATED_VALUE_UNITS) evaluationError(ctx, position, `${label} exceeds the ${MAX_EVALUATED_VALUE_UNITS.toLocaleString()} value-allocation budget`)
  return value
}

function finiteNumber(value: Value, ctx: EvalContext, position: number, label: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) evaluationError(ctx, position, `${label} must be a finite number`)
  return canonicalNumber(value)
}

function vectorValue(value: Value, ctx: EvalContext, position: number, label: string): number[] {
  if (!Array.isArray(value)) evaluationError(ctx, position, `${label} must be a vector`)
  return value.map(item => finiteNumber(item, ctx, position, label))
}

function truthy(value: Value): boolean {
  return value !== false && value !== undefined && value !== 0 && value !== ''
    && (!Array.isArray(value) || value.length > 0)
}

function deepEqual(left: Value, right: Value): boolean {
  if (Array.isArray(left) && Array.isArray(right)) {
    return left.length === right.length && left.every((value, index) => deepEqual(value, right[index]))
  }
  return left === right
}

function valueToString(value: Value): string {
  if (Array.isArray(value)) return `[${value.map(valueToString).join(', ')}]`
  if (isFunctionValue(value)) return 'function(...)'
  if (value === undefined) return 'undef'
  return String(value)
}

function resolveStableVariable(
  name: string,
  ctx: EvalContext,
  position: number,
): { found: boolean; value: Value } {
  if (name.startsWith('$') && ctx.env.has(name)) {
    return { found: true, value: ctx.env.get(name) }
  }
  let scope = ctx.stableScope
  let visibleBefore = ctx.scopeVisibleBefore ?? Number.POSITIVE_INFINITY
  while (scope !== undefined && scope !== null) {
    const local = scope.resolveLocalVariable(
      name,
      visibleBefore,
      (expression, site) => evalExpression(expression, {
        ...ctx,
        env: site.env,
        stableScope: site.scope,
        scopeVisibleBefore: site.visibleBefore,
      }),
      message => stableWarning(ctx, position, message, 'W_OPENSCAD_SCOPE'),
    )
    if (local.found) return local
    if (scope === ctx.stableScope && ctx.env.has(name)) {
      return { found: true, value: ctx.env.get(name) }
    }
    scope = scope.parent ?? undefined
    visibleBefore = Number.POSITIVE_INFINITY
  }
  if (ctx.env.has(name)) return { found: true, value: ctx.env.get(name) }
  return { found: false, value: undefined }
}

const {
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

function stableWarning(
  ctx: EvalContext,
  position: number,
  message: string,
  code = 'W_OPENSCAD_VALUE',
  dedupe = true,
): void {
  ctx.builder.warn(
    null,
    code,
    [],
    message,
    { start: position, end: Math.min(ctx.source.length, position + 1) },
    dedupe,
  )
}

function evalExpression(expr: Expr, ctx: EvalContext, depth = 0): Value {
  if (++ctx.budget.ops > MAX_EVAL_OPS) evaluationError(ctx, expr.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
  if (depth >= MAX_EXPRESSION_DEPTH) evaluationError(ctx, expr.p, `Expression exceeds ${MAX_EXPRESSION_DEPTH} evaluated levels`)
  const evaluate = (child: Expr) => evalExpression(child, ctx, depth + 1)
  switch (expr.kind) {
    case 'literal': return expr.value
    case 'identifier': {
      if (expr.name === 'PI') return Math.PI
      const resolved = isStableProfile(ctx)
        ? resolveStableVariable(expr.name, ctx, expr.p)
        : { found: ctx.env.has(expr.name), value: ctx.env.get(expr.name) }
      if (!resolved.found) {
        if (!isStableProfile(ctx)) evaluationError(ctx, expr.p, `Unknown variable ${expr.name}`)
        stableWarning(ctx, expr.p, `Ignoring unknown variable '${expr.name}'`)
        return undefined
      }
      return resolved.value
    }
    case 'vector': {
      if (!isStableProfile(ctx)) return registerArrayValue(expr.items.map(evaluate), ctx, expr.p)
      const values: Value[] = []
      for (const item of expr.items) {
        const value = evaluate(item)
        if (isListComprehensionExpression(item) && Array.isArray(value)) {
          appendComprehensionValues(values, value, item, ctx)
        } else values.push(value)
      }
      return registerArrayValue(values, ctx, expr.p)
    }
    case 'range': {
      if (isStableProfile(ctx)) {
        const start = evaluate(expr.start)
        const end = evaluate(expr.end)
        const step = expr.step === undefined ? 1 : evaluate(expr.step)
        if (typeof start !== 'number' || typeof step !== 'number' || typeof end !== 'number'
          || ![start, step, end].every(Number.isFinite)) {
          stableWarning(ctx, expr.p, 'Invalid range bounds produce undef')
          return undefined
        }
        if (step > 0 && start > end) {
          stableWarning(ctx, expr.p, 'begin is greater than the end, but step is positive')
        } else if (step < 0 && start < end) {
          stableWarning(ctx, expr.p, 'begin is smaller than the end, but step is negative')
        }
        return { kind: 'range-value', start, step, end }
      }
      const start = finiteNumber(evaluate(expr.start), ctx, expr.p, 'range start')
      const end = finiteNumber(evaluate(expr.end), ctx, expr.p, 'range end')
      const step = expr.step ? finiteNumber(evaluate(expr.step), ctx, expr.p, 'range step') : 1
      if (step === 0) evaluationError(ctx, expr.p, 'Range step cannot be zero')
      const values: number[] = []
      const forward = step > 0
      for (let value = start; forward ? value <= end + 1e-10 : value >= end - 1e-10; value += step) {
        values.push(canonicalNumber(value))
        if (values.length > MAX_RANGE_ITEMS) evaluationError(ctx, expr.p, `Range exceeds ${MAX_RANGE_ITEMS.toLocaleString()} items`)
      }
      return registerArrayValue(values, ctx, expr.p)
    }
    case 'unary': {
      const value = evaluate(expr.value)
      if (isStableProfile(ctx)) return openScadUnary(expr.op, value, stableValueContext(ctx, expr.p))
      if (expr.op === TT.Not) return !truthy(value)
      const number = finiteNumber(value, ctx, expr.p, 'unary operand')
      return canonicalNumber(expr.op === TT.Minus ? -number : number)
    }
    case 'binary': return evalBinaryExpression(expr, ctx, depth)
    case 'ternary': return (isStableProfile(ctx) ? openScadTruthy(evaluate(expr.test)) : truthy(evaluate(expr.test)))
      ? evaluate(expr.yes)
      : evaluate(expr.no)
    case 'index': {
      const value = evaluate(expr.value)
      if (isStableProfile(ctx)) {
        return openScadIndex(value, evaluate(expr.index), stableValueContext(ctx, expr.p))
      }
      const index = Math.trunc(finiteNumber(evaluate(expr.index), ctx, expr.p, 'index'))
      if (Array.isArray(value) || typeof value === 'string') return value[index] as Value
      evaluationError(ctx, expr.p, 'Only vectors and strings can be indexed')
    }
    case 'member': {
      const value = evaluate(expr.value)
      if (isStableProfile(ctx)) return openScadMember(value, expr.name, stableValueContext(ctx, expr.p))
      if (Array.isArray(value)) {
        const index = ({ x: 0, y: 1, z: 2 } as const)[expr.name as 'x' | 'y' | 'z']
        if (index !== undefined) return value[index]
      }
      evaluationError(ctx, expr.p, `Value has no member ${expr.name}`)
    }
    case 'function': {
      const value: FunctionValue = {
        kind: 'function-value',
        name: null,
        params: expr.params,
        body: expr.body,
        closure: new Map(ctx.env),
      }
      if (isStableProfile(ctx) && ctx.stableScope !== undefined) {
        return { ...value, lexicalScope: ctx.stableScope } as StableFunctionValue
      }
      return value
    }
    case 'call': return evalFunctionCall(expr, ctx, depth)
    case 'let': {
      if (!isStableProfile(ctx)) evaluationError(ctx, expr.p, 'let expression is not supported')
      const env = evaluateSequentialBindings(expr.args, ctx, depth + 1)
      return evalExpression(expr.body, stableOverlayContext(ctx, env), depth + 1)
    }
    case 'assert':
      if (!isStableProfile(ctx)) evaluationError(ctx, expr.p, 'assert expression is not supported')
      return evalAssertExpression(expr, ctx, depth)
    case 'echo':
      if (!isStableProfile(ctx)) evaluationError(ctx, expr.p, 'echo expression is not supported')
      return evalEchoExpression(expr, ctx, depth)
    case 'lc-for':
    case 'lc-for-c':
    case 'lc-if':
    case 'lc-let':
    case 'lc-each':
      if (!isStableProfile(ctx)) evaluationError(ctx, expr.p, 'list comprehension is not supported')
      return registerArrayValue(evalListComprehension(expr, ctx, depth), ctx, expr.p, 'list comprehension')
  }
}

function evalBuiltin(expr: Extract<Expr, { kind: 'call' }>, ctx: EvalContext, depth: number): Value {
  const name = expr.name
  if (name === null) evaluationError(ctx, expr.p, 'Expression is not callable')
  if (name === 'assert') evaluationError(ctx, expr.p, 'Expression-form assert() is not supported; use statement assert()')
  if (isStableProfile(ctx) && (name === 'dxf_dim' || name === 'dxf_cross')) {
    semanticProjectAssetRequired(ctx, expr.p, undefined, `${name}()`)
  }
  if (!isStableProfile(ctx) && expr.args.some(argument => argument.name !== undefined)) {
    evaluationError(ctx, expr.p, `${name}() does not accept named arguments in this engine revision`)
  }
  const evaluateArgument = (argument: ExpressionArgument): Value => {
    // OpenSCAD deliberately permits probing an undeclared bare name with
    // is_undef() without emitting the ordinary unknown-variable warning.
    if (isStableProfile(ctx) && name === 'is_undef' && expr.args.length === 1
      && argument.value.kind === 'identifier') {
      if (argument.value.name === 'PI') return Math.PI
      const resolved = resolveStableVariable(argument.value.name, ctx, argument.value.p)
      return resolved.found ? resolved.value : undefined
    }
    return evalExpression(argument.value, ctx, depth + 1)
  }
  const values = isStableProfile(ctx)
    ? createDeferredOpenScadBuiltinArguments(
        expr.args.length,
        index => evaluateArgument(expr.args[index]),
      )
    : expr.args.map(evaluateArgument)
  let result: ReturnType<typeof evaluateOpenScadBuiltinFunction>
  try {
    result = evaluateOpenScadBuiltinFunction(
      name,
      values as OpenScadBuiltinValue[],
      {
        error: message => {
          if (isStableProfile(ctx)) throw new StableBuiltinValueError(message)
          return evaluationError(ctx, expr.p, message)
        },
        warning: message => stableWarning(ctx, expr.p, message, 'W_OPENSCAD_BUILTIN'),
        registerArray: (items, label) => {
          if (items.length > MAX_VALUE_ELEMENTS) {
            evaluationError(ctx, expr.p, `${label} exceeds ${MAX_VALUE_ELEMENTS.toLocaleString()} elements`)
          }
          return registerArrayValue([...items] as Value[], ctx, expr.p, label)
        },
        registerString: (value, label) => {
          if (value.length > MAX_VALUE_ELEMENTS) {
            evaluationError(ctx, expr.p, `${label} exceeds ${MAX_VALUE_ELEMENTS.toLocaleString()} characters`)
          }
          return registerStringValue(value, ctx, expr.p, label)
        },
        random: Math.random,
        parentModule: moduleDepth => ctx.moduleStack.at(-1 - moduleDepth),
        isFunction: value => isFunctionValue(value as Value),
      },
    )
  } catch (error) {
    if (!(error instanceof StableBuiltinValueError)) throw error
    stableWarning(ctx, expr.p, error.message, 'W_OPENSCAD_BUILTIN')
    return undefined
  }
  if (!result.recognized) evaluationError(ctx, expr.p, `Unsupported function ${name}()`)
  return result.value as Value
}

function arg(node: CallNode, name: string, position: number, fallback: Value, ctx: EvalContext): Value {
  const expression = node.args[name] ?? node.args[`_${position}`]
  return expression ? evalExpression(expression, ctx) : fallback
}

function callExpressionArguments(node: CallNode): ExpressionArgument[] {
  if (node.callArguments !== undefined) return [...node.callArguments]
  return Object.entries(node.args).map(([key, value]) => {
    const span = node.argSpans[key]
    return {
      name: node.argKinds[key] === 'named' ? key : undefined,
      value,
      p: span?.start ?? value.p,
      end: span?.end ?? value.p,
    }
  })
}

interface EvaluatedStableCompatibilityArguments {
  readonly authored: readonly ExpressionArgument[]
  readonly values: ReadonlyMap<string, Value>
  readonly evaluated: ReadonlyMap<ExpressionArgument, Value>
}

/** Bind legacy built-in modules from the complete authored argument stream. */
function evaluateStableCompatibilityArguments(
  node: CallNode,
  positionalNames: readonly string[],
  namedOnlyNames: readonly string[],
  ctx: EvalContext,
): EvaluatedStableCompatibilityArguments {
  const authored = callExpressionArguments(node)
  const allowed = new Set([...positionalNames, ...namedOnlyNames])
  const resolved = new Map<string, ExpressionArgument>()
  for (const argument of authored) {
    const name = argument.name ?? positionalNames.find(parameter => !resolved.has(parameter))
    if (name === undefined) {
      stableWarning(ctx, argument.p, `Ignoring excess positional argument to ${node.name}()`)
      continue
    }
    if (!allowed.has(name)) {
      stableWarning(ctx, argument.p, `Ignoring unknown argument ${name} to ${node.name}()`)
      continue
    }
    if (resolved.has(name)) {
      stableWarning(ctx, argument.p, `Argument ${name} was specified more than once for ${node.name}()`)
    }
    resolved.set(name, argument)
  }

  // Stable module calls evaluate every authored argument exactly once. The
  // preserved stream matters for duplicate names and ignored expressions.
  const evaluated = new Map<ExpressionArgument, Value>()
  for (const argument of authored) evaluated.set(argument, evalExpression(argument.value, ctx))
  return Object.freeze({
    authored: Object.freeze(authored),
    evaluated,
    values: new Map([...resolved].map(([name, argument]) => [name, evaluated.get(argument)])),
  })
}

function semanticProjectAssetRequired(
  ctx: EvalContext,
  position: number,
  end: number | undefined,
  callable: string,
): never {
  throw new OpenSCADParseError(
    ctx.source,
    position,
    `${callable} requires an OpenSCAD project asset context; single-source semantic lowering cannot resolve project files.`,
    'E_IMPORT_PROJECT_REQUIRED',
    end,
  )
}

function stableCompatibilityDeprecation(
  node: CallNode,
  ctx: EvalContext,
  replacement: string,
): void {
  const message = node.name === 'child'
    ? 'child() will be removed in future releases. Use children() instead.'
    : `The ${node.name}() module will be removed in future releases. Use ${replacement} instead.`
  stableWarning(ctx, node.p, message, 'W_OPENSCAD_DEPRECATED')
}

function bindAssertArguments(node: CallNode, ctx: EvalContext): { condition: Expr; conditionText: string; message?: Expr } {
  const keys = Object.keys(node.args)
  const unknown = keys.find(key => node.argKinds[key] === 'named'
    ? key !== 'condition' && key !== 'message'
    : key !== '_0' && key !== '_1')
  if (unknown) evaluationError(ctx, node.p, `assert() does not accept argument ${unknown}`)
  const has = (key: string) => Object.hasOwn(node.args, key)
  if (has('_0') && has('condition')) evaluationError(ctx, node.p, 'assert() condition was provided more than once')
  if (has('_1') && has('message')) evaluationError(ctx, node.p, 'assert() message was provided more than once')
  const conditionKey = has('condition') ? 'condition' : has('_0') ? '_0' : null
  if (!conditionKey) evaluationError(ctx, node.p, 'assert() requires a condition')
  const messageKey = has('message') ? 'message' : has('_1') ? '_1' : null
  const span = node.argSpans[conditionKey]
  return {
    condition: node.args[conditionKey],
    conditionText: compactDiagnosticText(span ? ctx.source.slice(span.start, span.end) : 'condition'),
    message: messageKey ? node.args[messageKey] : undefined,
  }
}

function collectModules(nodes: readonly Statement[], modules: Map<string, ModuleNode>): void {
  for (const node of nodes) {
    if (node.type === 'module') {
      modules.set(node.name, node)
      collectModules(node.children, modules)
    } else if (node.type === 'call') {
      collectModules(node.children, modules)
      collectModules(node.alternative, modules)
    }
  }
}

function collectFunctions(nodes: readonly Statement[], functions: Map<string, FunctionNode>): void {
  for (const node of nodes) {
    if (node.type === 'function') functions.set(node.name, node)
    if (node.type === 'call') {
      collectFunctions(node.children, functions)
      collectFunctions(node.alternative, functions)
    } else if (node.type === 'module') collectFunctions(node.children, functions)
  }
}

function detachedChildrenContinuationMap(
  root: CallNode,
  passed: Readonly<PassedCallChildren> | undefined,
): ReadonlyMap<CallNode, Readonly<PassedCallChildren>> | undefined {
  if ((root.name !== 'children' && root.name !== 'child') || passed === undefined) return undefined
  const result = new Map<CallNode, Readonly<PassedCallChildren>>([[root, passed]])
  const visited = new Set<Readonly<PassedCallChildren>>()
  let current: Readonly<PassedCallChildren> | undefined = passed
  while (current !== undefined && !visited.has(current)) {
    visited.add(current)
    const continuation: Readonly<PassedCallChildren> | undefined = current.continuation
    if (continuation === undefined) break
    const collectConsumers = (statements: readonly Statement[]): void => {
      for (const statement of statements) {
        if (statement.type !== 'call') continue
        if (statement.name === 'children' || statement.name === 'child') result.set(statement, continuation)
        collectConsumers(statement.children)
        collectConsumers(statement.alternative)
      }
    }
    collectConsumers(current.statements)
    current = continuation
  }
  return result
}

function evalNodes(nodes: readonly Statement[], parent: EvalContext, scoped = true): SemanticShape[] {
  const ctx: EvalContext = isStableProfile(parent)
    ? enterStableStatementScope(nodes, parent)
    : { ...parent, env: scoped ? new Map(parent.env) : parent.env }
  return evalPreparedNodes(nodes, ctx)
}

function viewportSemanticSlots(node: CallNode, ctx: EvalContext): SemanticDynamicSlot[] {
  if (!isStableProfile(ctx) || ctx.viewportRootOwner === node) return []
  const slots: SemanticDynamicSlot[] = []
  if (hasOpenScadViewportModifier(node, 'highlight')) {
    slots.push(semanticSlot('$viewport-highlight', true))
  }
  if (hasOpenScadViewportModifier(node, 'background')) {
    slots.push(semanticSlot('$viewport-background', true))
  }
  return slots
}

function viewportRootActivates(node: CallNode, shapes: readonly SemanticShape[]): boolean {
  if (shapes.length > 0) return true
  // These effect/branch calls do not contribute an empty CSG container in
  // OpenSCAD 2021.01. Other evaluated module instantiations do, so a `!` on
  // union(), transform{}, group{}, for(empty), or an empty user module still
  // wins and deliberately yields an empty top-level result.
  return !['if', 'assert', 'echo', 'children', 'child'].includes(node.name)
}

function evalPreparedNodes(nodes: readonly Statement[], ctx: EvalContext): SemanticShape[] {
  const output: SemanticShape[] = []
  for (const statement of nodes) {
    if (isStableProfile(ctx) && statement.type === 'call'
      && hasOpenScadViewportModifier(statement, 'disable')) {
      // `*` is the sole modifier which suppresses language evaluation itself.
      continue
    }
    const occurrenceStart = ctx.builder.occurrences.length
    const background = isStableProfile(ctx) && statement.type === 'call'
      && ctx.viewportRootOwner !== statement
      && hasOpenScadViewportModifier(statement, 'background')
    const backgroundCheckpoint = background && ctx.quality === 'full'
      ? ctx.builder.runtimeCheckpoint()
      : null
    try {
      poll(ctx)
      if (++ctx.budget.ops > MAX_EVAL_OPS) evaluationError(ctx, statement.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
      if (statement.type === 'assign') {
        ctx.builder.invocation(statement, ctx.parentOccurrence, ctx.pendingSlots)
        if (!isStableProfile(ctx)) ctx.env.set(statement.name, evalExpression(statement.value, ctx))
        continue
      }
      if (statement.type === 'module') {
        ctx.builder.invocation(statement, ctx.parentOccurrence, ctx.pendingSlots)
        continue
      }
      if (statement.type === 'function') continue
      const shapes = evalNode(statement, ctx)
      if (backgroundCheckpoint !== null) {
        ctx.builder.rollbackRuntime(backgroundCheckpoint)
      } else {
        output.push(...shapes)
        if (background && shapes.length > 0) ctx.reduced.value = true
      }
      if (output.length > MAX_SHAPES) evaluationError(ctx, statement.p, `Model exceeds the ${MAX_SHAPES.toLocaleString()} object limit`)
    } catch (error) {
      const capturable = error instanceof OpenSCADParseError
        || error instanceof LegacyBindingCompatibilityError
      if (capturable && ctx.builder.terminalOccurrence === null) {
        const operation = ctx.builder.registered.byStatement.get(statement)
        // Full-profile echo is intentionally transparent in semantic-program
        // v1. An expression error inside that effect has no static occurrence
        // to attach to, so preserve the positioned language error instead of
        // replacing it with an internal registry failure.
        if (operation === undefined) throw error
        let occurrence = -1
        for (let index = ctx.builder.occurrences.length - 1; index >= occurrenceStart; index--) {
          const candidate = ctx.builder.occurrences[index]
          if (candidate.operation === operation && candidate.parent === ctx.parentOccurrence) {
            occurrence = index
            break
          }
        }
        if (occurrence < 0) {
          occurrence = ctx.builder.invocation(statement, ctx.parentOccurrence, ctx.pendingSlots)
        }
        ctx.builder.captureTerminalOccurrence(occurrence)
      }
      throw error
    }
  }
  return output
}

function frameChildren(
  owner: CallNode | ModuleNode,
  statements: readonly Statement[],
  ownerOccurrence: number,
  ctx: EvalContext,
  alternative = false,
): SemanticShape[] {
  const framed = frameContext(owner, ownerOccurrence, ctx, alternative)
  return framed ? evalNodes(statements, framed) : []
}

function frameContext(
  owner: CallNode | ModuleNode,
  ownerOccurrence: number,
  ctx: EvalContext,
  alternative = false,
): EvalContext | null {
  const operation = alternative && owner.type === 'call'
    ? ctx.builder.registered.alternativeByCall.get(owner)
    : ctx.builder.registered.bodyByStatement.get(owner)
  if (operation === undefined) return null
  const frame = ctx.builder.invocationOperation(operation, ownerOccurrence)
  return { ...ctx, parentOccurrence: frame, pendingSlots: [] }
}

const IDENTITY_MATRIX: SemanticMatrix4 = [
  1, 0, 0, 0,
  0, 1, 0, 0,
  0, 0, 1, 0,
  0, 0, 0, 1,
]

function multiplyMatrix(left: SemanticMatrix4, right: SemanticMatrix4): SemanticMatrix4 {
  const output = new Array<number>(16).fill(0)
  for (let column = 0; column < 4; column++) {
    for (let row = 0; row < 4; row++) {
      for (let inner = 0; inner < 4; inner++) output[column * 4 + row] += left[inner * 4 + row] * right[column * 4 + inner]
      output[column * 4 + row] = canonicalNumber(output[column * 4 + row])
    }
  }
  return output as unknown as SemanticMatrix4
}

function translationMatrix(x: number, y: number, z: number): SemanticMatrix4 {
  const matrix = [...IDENTITY_MATRIX] as number[]
  matrix[12] = x; matrix[13] = y; matrix[14] = z
  return matrix as unknown as SemanticMatrix4
}

function scaleMatrix(x: number, y: number, z: number): SemanticMatrix4 {
  const matrix = [...IDENTITY_MATRIX] as number[]
  matrix[0] = x; matrix[5] = y; matrix[10] = z
  return matrix as unknown as SemanticMatrix4
}

function axisAngleMatrix(axis: readonly number[], degrees: number, ctx: EvalContext, position: number): SemanticMatrix4 {
  const length = Math.hypot(axis[0] ?? 0, axis[1] ?? 0, axis[2] ?? 0)
  if (length === 0) evaluationError(ctx, position, 'Rotation axis cannot be zero')
  const x = (axis[0] ?? 0) / length, y = (axis[1] ?? 0) / length, z = (axis[2] ?? 0) / length
  const angle = degrees * Math.PI / 180, c = Math.cos(angle), s = Math.sin(angle), t = 1 - c
  return [
    canonicalNumber(t*x*x+c), canonicalNumber(t*x*y+s*z), canonicalNumber(t*x*z-s*y), 0,
    canonicalNumber(t*x*y-s*z), canonicalNumber(t*y*y+c), canonicalNumber(t*y*z+s*x), 0,
    canonicalNumber(t*x*z+s*y), canonicalNumber(t*y*z-s*x), canonicalNumber(t*z*z+c), 0,
    0, 0, 0, 1,
  ]
}

function eulerMatrix(x: number, y: number, z: number): SemanticMatrix4 {
  const rx = axisAngleMatrixUnchecked([1, 0, 0], x)
  const ry = axisAngleMatrixUnchecked([0, 1, 0], y)
  const rz = axisAngleMatrixUnchecked([0, 0, 1], z)
  return multiplyMatrix(rz, multiplyMatrix(ry, rx))
}

function axisAngleMatrixUnchecked(axis: readonly [number, number, number], degrees: number): SemanticMatrix4 {
  const angle = degrees * Math.PI / 180, c = Math.cos(angle), s = Math.sin(angle), t = 1 - c
  const [x, y, z] = axis
  return [
    canonicalNumber(t*x*x+c), canonicalNumber(t*x*y+s*z), canonicalNumber(t*x*z-s*y), 0,
    canonicalNumber(t*x*y-s*z), canonicalNumber(t*y*y+c), canonicalNumber(t*y*z+s*x), 0,
    canonicalNumber(t*x*z+s*y), canonicalNumber(t*y*z-s*x), canonicalNumber(t*z*z+c), 0,
    0, 0, 0, 1,
  ]
}

function mirrorMatrix(
  vector: readonly number[],
  dimension: SemanticDimension,
  ctx: EvalContext,
  position: number,
): SemanticMatrix4 {
  const zValue = dimension === 'region2' ? 0 : vector[2] ?? 0
  const length = Math.hypot(vector[0] ?? 0, vector[1] ?? 0, zValue)
  if (length === 0) {
    if (ctx.builder.languageContract === 'openscad-viewer/brep-1') {
      evaluationError(ctx, position, 'Mirror normal cannot be zero')
    }
    return [
      0, 0, 0, 0,
      0, 0, 0, 0,
      0, 0, dimension === 'region2' ? 1 : 0, 0,
      0, 0, 0, 1,
    ]
  }
  const x = (vector[0] ?? 0) / length, y = (vector[1] ?? 0) / length, z = zValue / length
  return [
    canonicalNumber(1-2*x*x), canonicalNumber(-2*x*y), canonicalNumber(-2*x*z), 0,
    canonicalNumber(-2*y*x), canonicalNumber(1-2*y*y), canonicalNumber(-2*y*z), 0,
    canonicalNumber(-2*z*x), canonicalNumber(-2*z*y), canonicalNumber(1-2*z*z), 0,
    0, 0, 0, 1,
  ]
}

function matrixValue(value: Value, ctx: EvalContext, position: number): SemanticMatrix4 {
  if (!Array.isArray(value) || value.length < 3) evaluationError(ctx, position, 'multmatrix requires a 4x4 matrix')
  const rows = value.map(row => vectorValue(row, ctx, position, 'matrix row'))
  if (rows.some(row => row.length < 4)) evaluationError(ctx, position, 'multmatrix requires a 4x4 matrix')
  const matrix: number[] = []
  for (let column = 0; column < 4; column++) {
    for (let row = 0; row < 4; row++) matrix.push(canonicalNumber(rows[row]?.[column] ?? (row === column ? 1 : 0)))
  }
  if (matrix[3] !== 0 || matrix[7] !== 0 || matrix[11] !== 0 || matrix[15] !== 1) {
    if (ctx.builder.languageContract === 'legacy/current') {
      // The pinned Manifold transform consumes only the affine 3x4 portion and
      // ignores the authored fourth row. Canonicalize that legacy equivalence
      // instead of weakening the SPE1 affine-matrix invariant.
      matrix[3] = 0
      matrix[7] = 0
      matrix[11] = 0
      matrix[15] = 1
    } else {
      evaluationError(ctx, position, 'multmatrix must be affine')
    }
  }
  return matrix as unknown as SemanticMatrix4
}

function parseColor(value: Value, ctx: EvalContext, position: number): RGBA {
  if (Array.isArray(value)) {
    const channels = vectorValue(value, ctx, position, 'color')
    return [clamp01(channels[0] ?? 0.5), clamp01(channels[1] ?? 0.5), clamp01(channels[2] ?? 0.5), clamp01(channels[3] ?? 1)]
  }
  if (typeof value !== 'string') evaluationError(ctx, position, 'color() expects a name or RGB(A) vector')
  const named = CSS_COLORS[value.toLowerCase()]
  if (named) return [...named]
  const match = value.match(/^#([0-9a-f]{6}|[0-9a-f]{8})$/i)
  if (!match) evaluationError(ctx, position, `Unknown color ${value}`)
  const hex = match[1]
  return [
    parseInt(hex.slice(0, 2), 16) / 255,
    parseInt(hex.slice(2, 4), 16) / 255,
    parseInt(hex.slice(4, 6), 16) / 255,
    hex.length === 8 ? parseInt(hex.slice(6, 8), 16) / 255 : 1,
  ]
}

function semanticSlot(name: string, value: Value): SemanticDynamicSlot {
  return { name, value: identityValue(value), duplicateOrdinal: 0 }
}

function invoke(
  node: CallNode | ModuleNode,
  ctx: EvalContext,
  slots: readonly SemanticDynamicSlot[] = [],
): number {
  const viewportSlots = node.type === 'call' ? viewportSemanticSlots(node, ctx) : []
  return ctx.builder.invocation(
    node,
    ctx.parentOccurrence,
    [...ctx.pendingSlots, ...viewportSlots, ...slots],
  )
}

function nodeValueType(shape: SemanticShape, ctx: EvalContext): SemanticValueType {
  return ctx.builder.nodes[shape.node].valueType
}

function requestedSegments(node: CallNode, ctx: EvalContext): number | null {
  const local = arg(node, '$fn', -1, undefined, ctx)
  const global = ctx.env.get('$fn')
  const raw = local === undefined || local === 0 ? global : local
  return raw === undefined || raw === 0
    ? null
    : Math.round(finiteNumber(raw, ctx, node.p, '$fn'))
}

function semanticSegments(
  node: CallNode,
  ctx: EvalContext,
  occurrence: number,
  fallback: number,
  minimum: number,
): number | null {
  const requested = requestedSegments(node, ctx)
  return semanticSegmentsFromRequested(node, ctx, occurrence, requested, fallback, minimum)
}

function semanticSegmentsFromRequested(
  node: CallNode,
  ctx: EvalContext,
  occurrence: number,
  requested: number | null,
  fallback: number,
  minimum: number,
): number | null {
  if (ctx.builder.languageContract === 'openscad-viewer/brep-1') {
    ctx.builder.addTessellationIntent(occurrence, requested, ctx.env)
    return null
  }
  const maximum = ctx.quality === 'preview' ? 48 : MAX_FN
  const previewFallback = ctx.quality === 'preview' ? Math.min(fallback, 24) : fallback
  let segments = requested ?? previewFallback
  if (segments > maximum) {
    const operation = ctx.builder.registered.byStatement.get(node) ?? null
    ctx.builder.warn(
      operation,
      'SEMANTIC_SEGMENTS_CLAMPED',
      [
        { name: 'requested', value: identityValue(segments) },
        { name: 'maximum', value: identityValue(maximum) },
      ],
      `$fn=${segments} was clamped to ${maximum} for ${ctx.quality} rendering`,
      { start: node.p, end: node.end },
    )
    segments = maximum
  }
  segments = Math.max(minimum, segments)
  if (ctx.quality === 'preview') {
    const fullSegments = Math.max(minimum, Math.min(requested ?? fallback, MAX_FN))
    if (segments !== fullSegments) ctx.reduced.value = true
  }
  return segments
}

function primitiveColor(ctx: EvalContext): RGBA { return ctx.builder.nextColor() }

function transformShapes(
  occurrence: number,
  shapes: readonly SemanticShape[],
  matrixFor: (shape: SemanticShape) => SemanticMatrix4,
  ctx: EvalContext,
): SemanticShape[] {
  return shapes.map(shape => ctx.builder.produce(occurrence, {
    kind: 'transform',
    valueType: nodeValueType(shape, ctx),
    input: shape.node,
    matrix: matrixFor(shape),
  }, shape.color, shape.identityOccurrence))
}

function combineShapes(
  occurrence: number,
  shapes: readonly SemanticShape[],
  operation: 'union' | 'intersection' | 'difference' | 'hull',
  ctx: EvalContext,
  position: number,
): SemanticShape[] {
  if (shapes.length === 0) return []
  const dimension = shapes[0].dimension
  if (shapes.some(shape => shape.dimension !== dimension)) {
    evaluationError(ctx, position, `${operation}() cannot mix 2D and 3D children`)
  }
  if (shapes.length === 1) return ctx.builder.alias(occurrence, shapes, operation === 'hull')
  const firstType = nodeValueType(shapes[0], ctx)
  const valueType: SemanticValueType = {
    ...firstType,
    geometryKind: dimension === 'region2' ? 'region' : 'solid-set',
    space: dimension === 'region2' ? 'd2' : 'd3',
  }
  const common = {
    valueType,
    inputs: shapes.map(shape => shape.node),
  }
  const semanticNode: SemanticNodeWithoutId = operation === 'hull'
    ? { kind: 'hull', ...common }
    : { kind: 'boolean', operation, ...common }
  return [ctx.builder.produce(occurrence, semanticNode, shapes[0].color)]
}

interface SemanticReducedShape {
  readonly node: number
  readonly dimension: SemanticDimension
  readonly color: RGBA
  readonly identityOccurrence: number | null
}

function unionReduceInternal(
  shapes: readonly SemanticShape[],
  ctx: EvalContext,
  position: number,
): SemanticReducedShape | null {
  if (shapes.length === 0) return null
  const dimension = shapes[0].dimension
  if (shapes.some(shape => shape.dimension !== dimension)) evaluationError(ctx, position, 'union() cannot mix 2D and 3D children')
  if (shapes.length === 1) return {
    node: shapes[0].node,
    dimension,
    color: shapes[0].color,
    identityOccurrence: shapes[0].identityOccurrence,
  }
  const firstType = nodeValueType(shapes[0], ctx)
  const node = ctx.builder.addInternalNode({
    kind: 'boolean',
    operation: 'union',
    inputs: shapes.map(shape => shape.node),
    valueType: {
      ...firstType,
      geometryKind: dimension === 'region2' ? 'region' : 'solid-set',
      space: dimension === 'region2' ? 'd2' : 'd3',
    },
  })
  return { node, dimension, color: shapes[0].color, identityOccurrence: null }
}

function makeCylinder(node: CallNode, ctx: EvalContext): SemanticShape[] {
  const height = finiteNumber(arg(node, 'h', 0, 1, ctx), ctx, node.p, 'cylinder height')
  if (height <= 0) evaluationError(ctx, node.p, 'Cylinder height must be positive')
  let low = arg(node, 'r1', 1, undefined, ctx)
  let high = arg(node, 'r2', 2, undefined, ctx)
  const radius = arg(node, 'r', 1, undefined, ctx)
  const diameter = arg(node, 'd', -1, undefined, ctx)
  const diameterLow = arg(node, 'd1', -1, undefined, ctx)
  const diameterHigh = arg(node, 'd2', -1, undefined, ctx)
  if (diameterLow !== undefined) low = finiteNumber(diameterLow, ctx, node.p, 'd1') / 2
  if (diameterHigh !== undefined) high = finiteNumber(diameterHigh, ctx, node.p, 'd2') / 2
  if (low === undefined && high === undefined) {
    const base = diameter !== undefined
      ? finiteNumber(diameter, ctx, node.p, 'diameter') / 2
      : radius !== undefined ? finiteNumber(radius, ctx, node.p, 'radius') : 1
    low = base
    high = base
  }
  if (low === undefined) low = high
  if (high === undefined) high = low
  const radiusBottom = finiteNumber(low, ctx, node.p, 'r1')
  const radiusTop = finiteNumber(high, ctx, node.p, 'r2')
  if (radiusBottom < 0 || radiusTop < 0 || (radiusBottom === 0 && radiusTop === 0)) {
    evaluationError(ctx, node.p, 'Cylinder radii must be non-negative and not both zero')
  }
  const center = arg(node, 'center', 3, false, ctx) === true
  const occurrence = invoke(node, ctx)
  const segments = semanticSegments(node, ctx, occurrence, 32, 3)
  return [ctx.builder.produce(occurrence, segments === null ? {
    kind: 'cylinder-analytic',
    valueType: semanticValueType(ctx.builder.languageContract, 'solid', 'd3'),
    height, radiusBottom, radiusTop, center,
  } : {
    kind: 'cylinder-polygonal',
    valueType: semanticValueType(ctx.builder.languageContract, 'solid-set', 'd3'),
    height, radiusBottom, radiusTop, center, radialSegments: segments,
  }, primitiveColor(ctx))]
}

function makePolyhedron(node: CallNode, ctx: EvalContext): SemanticShape[] {
  const points = arg(node, 'points', 0, [], ctx)
  const faces = arg(node, 'faces', 1, arg(node, 'triangles', 1, [], ctx), ctx)
  if (!Array.isArray(points) || !Array.isArray(faces)) evaluationError(ctx, node.p, 'polyhedron points and faces must be vectors')
  const vertices = points.map((point, index) => {
    const vector = vectorValue(point, ctx, node.p, `polyhedron point ${index}`)
    if (vector.length < 3) evaluationError(ctx, node.p, 'Each polyhedron point needs three coordinates')
    return [vector[0], vector[1], vector[2]] as const
  })
  const triangles: Array<readonly [number, number, number]> = []
  for (const face of faces) {
    const polygon = vectorValue(face, ctx, node.p, 'polyhedron face')
      .map(value => canonicalNumber(Math.trunc(value)))
    if (polygon.length < 3) evaluationError(ctx, node.p, 'Each polyhedron face needs at least three vertices')
    for (const index of polygon) {
      if (index < 0 || index >= vertices.length) evaluationError(ctx, node.p, 'Polyhedron face index is out of bounds')
    }
    for (let index = 1; index < polygon.length - 1; index++) {
      triangles.push([polygon[0], polygon[index + 1], polygon[index]])
    }
  }
  if (vertices.length < 4) {
    if (ctx.builder.languageContract === 'legacy/current') {
      if (vertices.length !== 0 || faces.length !== 0) {
        evaluationError(ctx, node.p, 'Invalid manifold polyhedron: Not manifold')
      }
    } else {
      evaluationError(ctx, node.p, 'A solid polyhedron needs at least four points')
    }
  }
  const occurrence = invoke(node, ctx)
  return [ctx.builder.produce(occurrence, {
    kind: 'polyhedron',
    valueType: semanticValueType(ctx.builder.languageContract, 'solid-set', 'd3'),
    vertices,
    triangles,
  }, primitiveColor(ctx))]
}

function makePolygon(node: CallNode, ctx: EvalContext): SemanticShape[] {
  const pointsValue = arg(node, 'points', 0, [], ctx)
  if (!Array.isArray(pointsValue)) evaluationError(ctx, node.p, 'polygon points must be a vector')
  const points = pointsValue.map(point => {
    const vector = vectorValue(point, ctx, node.p, 'polygon point')
    if (vector.length < 2) evaluationError(ctx, node.p, 'Each polygon point needs two coordinates')
    return [vector[0], vector[1]] as const
  })
  const pathsValue = arg(node, 'paths', 1, undefined, ctx)
  let rings: Array<readonly (readonly [number, number])[]>
  if (pathsValue === undefined) {
    if (ctx.builder.languageContract === 'legacy/current' && points.length === 0) {
      // The pinned JS binding receives the flat empty point vector here and
      // throws before it can reinterpret it as one empty ring.
      throw new LegacyBindingCompatibilityError("Cannot read properties of undefined (reading 'length')")
    }
    rings = [points]
  } else {
    if (!Array.isArray(pathsValue)) evaluationError(ctx, node.p, 'polygon paths must be a vector')
    rings = pathsValue.map(path => vectorValue(path, ctx, node.p, 'polygon path').map(rawIndex => {
      const point = points[Math.trunc(rawIndex)]
      if (!point) evaluationError(ctx, node.p, 'Polygon path index is out of bounds')
      return point
    }))
  }
  if (rings.length === 0) {
    if (ctx.builder.languageContract === 'legacy/current') {
      throw new LegacyBindingCompatibilityError("Cannot read properties of undefined (reading 'length')")
    }
    evaluationError(ctx, node.p, 'Polygon rings need at least three points')
  }
  if (ctx.builder.languageContract !== 'legacy/current' && rings.some(ring => ring.length < 3)) {
    evaluationError(ctx, node.p, 'Polygon rings need at least three points')
  }
  const occurrence = invoke(node, ctx)
  return [ctx.builder.produce(occurrence, {
    kind: 'polygon',
    valueType: semanticValueType(ctx.builder.languageContract, 'region', 'd2'),
    rings,
    fillRule: 'even-odd',
  }, primitiveColor(ctx))]
}

function compatibilityRequestedSegments(
  values: ReadonlyMap<string, Value>,
  node: CallNode,
  ctx: EvalContext,
): number | null {
  const local = values.get('$fn')
  const global = ctx.env.get('$fn')
  const raw = local === undefined || local === 0 ? global : local
  return raw === undefined || raw === 0
    ? null
    : Math.round(finiteNumber(raw, ctx, node.p, '$fn'))
}

function compatibilityPlanarChildren(
  node: CallNode,
  occurrence: number,
  ctx: EvalContext,
): SemanticShape[] {
  const children = frameChildren(node, node.children, occurrence, ctx)
  const planar = children.filter(shape => shape.dimension === 'region2')
  if (planar.length !== children.length) {
    stableWarning(
      ctx,
      node.p,
      `${node.name}() ignored non-2D child geometry`,
      'W_OPENSCAD_COMPATIBILITY_CHILD',
    )
  }
  return planar
}

function evalDxfLinearExtrude(node: CallNode, ctx: EvalContext): SemanticShape[] {
  stableCompatibilityDeprecation(node, ctx, 'linear_extrude()')
  const evaluated = evaluateStableCompatibilityArguments(
    node,
    ['file', 'layer', 'height', 'origin', 'scale', 'center', 'twist', 'slices'],
    ['convexity', '$fn', '$fa', '$fs'],
    ctx,
  )
  const file = evaluated.values.get('file')
  if (typeof file === 'string' && file.length > 0) {
    semanticProjectAssetRequired(ctx, node.p, node.end, 'dxf_linear_extrude()')
  }

  let heightValue = evaluated.values.get('height')
  const first = evaluated.authored[0]
  if (heightValue === undefined && first?.name === undefined) {
    const firstValue = evaluated.evaluated.get(first)
    if (typeof firstValue === 'number') heightValue = firstValue
  }
  let height = typeof heightValue === 'number' && Number.isFinite(heightValue) ? heightValue : 100
  if (height <= 0) height = 0

  const rawScale = evaluated.values.get('scale')
  let scale: readonly [number, number] = [1, 1]
  if (typeof rawScale === 'number' && Number.isFinite(rawScale)) {
    scale = [Math.max(0, rawScale), Math.max(0, rawScale)]
  } else if (Array.isArray(rawScale) && rawScale.length === 2) {
    const scaleX = rawScale[0]
    const scaleY = rawScale[1]
    if (typeof scaleX === 'number' && Number.isFinite(scaleX)
      && typeof scaleY === 'number' && Number.isFinite(scaleY)) {
      scale = [Math.max(0, scaleX), Math.max(0, scaleY)]
    }
  }
  const twistValue = evaluated.values.get('twist')
  const twistDegrees = typeof twistValue === 'number' && Number.isFinite(twistValue) ? twistValue : 0
  const slicesValue = evaluated.values.get('slices')
  const slices = typeof slicesValue === 'number' && Number.isFinite(slicesValue) && slicesValue > 0
    ? Math.min(MAX_EXTRUDE_SLICES, Math.trunc(slicesValue))
    : 0
  const center = evaluated.values.get('center') === true

  const occurrence = invoke(node, ctx)
  const shapes = compatibilityPlanarChildren(node, occurrence, ctx)
  if (height === 0 || shapes.length === 0) return []
  const profile = unionReduceInternal(shapes, ctx, node.p)!
  const inputType = ctx.builder.nodes[profile.node].valueType
  return [ctx.builder.produce(occurrence, {
    kind: 'linear-extrude',
    valueType: { ...inputType, geometryKind: 'solid-set', space: 'd3' },
    input: profile.node,
    height,
    twistDegrees,
    slices,
    scale,
    center,
  }, profile.color)]
}

function evalDxfRotateExtrude(node: CallNode, ctx: EvalContext): SemanticShape[] {
  stableCompatibilityDeprecation(node, ctx, 'rotate_extrude()')
  const evaluated = evaluateStableCompatibilityArguments(
    node,
    ['file', 'layer', 'origin', 'scale'],
    ['convexity', 'angle', '$fn', '$fa', '$fs'],
    ctx,
  )
  const file = evaluated.values.get('file')
  if (file !== undefined && (typeof file !== 'string' || file.length > 0)) {
    semanticProjectAssetRequired(ctx, node.p, node.end, 'dxf_rotate_extrude()')
  }
  const rawAngle = evaluated.values.get('angle')
  let angleDegrees = typeof rawAngle === 'number' && Number.isFinite(rawAngle) ? rawAngle : 360
  if (angleDegrees <= -360 || angleDegrees > 360) angleDegrees = 360

  const occurrence = invoke(node, ctx)
  const shapes = compatibilityPlanarChildren(node, occurrence, ctx)
  if (angleDegrees === 0 || shapes.length === 0) return []
  const profile = unionReduceInternal(shapes, ctx, node.p)!
  const requested = compatibilityRequestedSegments(evaluated.values, node, ctx)
  const segments = semanticSegmentsFromRequested(node, ctx, occurrence, requested, 48, 3)
  const valueType = {
    ...ctx.builder.nodes[profile.node].valueType,
    geometryKind: 'solid-set' as const,
    space: 'd3' as const,
  }
  return [ctx.builder.produce(occurrence, segments === null ? {
    kind: 'rotate-extrude-analytic', valueType, input: profile.node, angleDegrees,
  } : {
    kind: 'rotate-extrude-polygonal', valueType, input: profile.node, angleDegrees, radialSegments: segments,
  }, profile.color)]
}

function evalPassedCallChildren(
  node: CallNode,
  ctx: EvalContext,
  occurrence: number,
  indexValue: Value,
  passed: Readonly<PassedCallChildren>,
  statements: readonly Statement[],
): SemanticShape[] {
  if (statements.length === 0) return []
  const currentOperation = ctx.builder.registered.byStatement.get(node)
  const expansionOperation = ctx.builder.registered.expansionByCall.get(node)
    ?? passed.expansionOperation
  const expansionPath = ctx.builder.registered.operations[expansionOperation].structuralPath
  const currentPath = currentOperation === undefined
    ? []
    : ctx.builder.registered.operations[currentOperation].structuralPath
  const legacyReentry = !isStableProfile(ctx)
    && ctx.builder.languageContract === 'legacy/current'
    && expansionPath.length < currentPath.length
    && expansionPath.every((segment, index) => {
      const candidate = currentPath[index]
      return segment.kind === candidate.kind
        && segment.name === candidate.name
        && segment.ordinal === candidate.ordinal
    })
  const expansion = legacyReentry
    ? occurrence
    : ctx.builder.invocationOperation(
      expansionOperation,
      occurrence,
      [semanticSlot('$index', indexValue)],
    )
  return evalNodes(statements, {
    ...ctx,
    env: new Map(passed.env),
    stableScope: isStableProfile(ctx) ? passed.scope ?? ctx.stableScope : ctx.stableScope,
    scopeVisibleBefore: Number.POSITIVE_INFINITY,
    parentOccurrence: expansion,
    pendingSlots: [],
    callChildren: !isStableProfile(ctx) && ctx.builder.languageContract === 'legacy/current'
      ? passed
      : passed.continuation,
  })
}

function evalNode(node: CallNode, parent: EvalContext): SemanticShape[] {
  if (isStableProfile(parent) && !parent.viewportRootLocked
    && hasOpenScadViewportModifier(node, 'root')) {
    // Selection is runtime-first and value-producing, not syntax-first: an
    // unvisited false branch, unused module, or empty call cannot suppress
    // later geometry. Probe the candidate with nested roots locked, then replay
    // a non-empty winner against the captured environment as a detached root.
    const diagnosticsBeforeCandidate = parent.builder.diagnosticCheckpoint()
    const shapes = evalNode(node, {
      ...parent,
      viewportRootLocked: true,
      viewportRootOwner: node,
    })
    if (viewportRootActivates(node, shapes)) {
      throw new StableViewportRootSelection(node, parent, diagnosticsBeforeCandidate)
    }
    return shapes
  }
  if (parent.depth >= MAX_EVAL_DEPTH) evaluationError(parent, node.p, `Evaluation exceeds ${MAX_EVAL_DEPTH} nested calls`)
  const ctx: EvalContext = { ...parent, depth: parent.depth + 1 }

  switch (node.name) {
    case 'assign': {
      if (!isStableProfile(ctx)) evaluationError(ctx, node.p, 'Unsupported geometry operation assign()')
      const env = new Map(ctx.env)
      const bindings = new Map<string, Value>()
      for (const argument of callExpressionArguments(node)) {
        // Historical assign() ignores positional arguments without evaluating
        // them, and evaluates every named RHS against the caller in parallel.
        if (argument.name === undefined) continue
        const value = evalExpression(argument.value, ctx)
        bindings.set(argument.name, value)
        env.set(argument.name, value)
      }
      const occurrence = invoke(
        node,
        ctx,
        [...bindings].map(([name, value]) => semanticSlot(name, value)),
      )
      return frameChildren(node, node.children, occurrence, stableOverlayContext(ctx, env))
    }
    case 'dxf_linear_extrude':
      if (!isStableProfile(ctx)) evaluationError(ctx, node.p, 'Unsupported geometry operation dxf_linear_extrude()')
      return evalDxfLinearExtrude(node, ctx)
    case 'dxf_rotate_extrude':
      if (!isStableProfile(ctx)) evaluationError(ctx, node.p, 'Unsupported geometry operation dxf_rotate_extrude()')
      return evalDxfRotateExtrude(node, ctx)
    case 'import_stl':
    case 'import_off':
    case 'import_dxf':
      if (!isStableProfile(ctx)) evaluationError(ctx, node.p, `Unsupported geometry operation ${node.name}()`)
      return semanticProjectAssetRequired(ctx, node.p, node.end, `${node.name}()`)
    case 'assert': {
      if (isStableProfile(ctx)) {
        const args = callExpressionArguments(node)
        const resolved = resolveStableExpressionArguments(args, ['condition', 'message'], ctx)
        const conditionArgument = resolved.get('condition')
        const messageArgument = resolved.get('message')
        const condition = conditionArgument === undefined
          ? undefined
          : evalExpression(conditionArgument.value, ctx)
        const message = messageArgument === undefined
          ? undefined
          : evalExpression(messageArgument.value, ctx)
        const conditionText = conditionArgument === undefined
          ? 'undef'
          : compactDiagnosticText(ctx.source.slice(conditionArgument.p, conditionArgument.end))
        const occurrence = invoke(node, ctx, [semanticSlot('condition', condition), semanticSlot('message', message)])
        if (!openScadTruthy(condition)) {
          const detail = messageArgument === undefined
            ? ''
            : `: ${compactDiagnosticText(formatOpenScadValue(message))}`
          evaluationError(ctx, node.p, `Assertion '${conditionText}' failed${detail}`)
        }
        return ctx.builder.alias(occurrence, frameChildren(node, node.children, occurrence, ctx), false)
      }
      const bound = bindAssertArguments(node, ctx)
      const condition = evalExpression(bound.condition, ctx)
      const message = bound.message ? evalExpression(bound.message, ctx) : undefined
      const occurrence = invoke(node, ctx, [semanticSlot('condition', condition), semanticSlot('message', message)])
      if (!truthy(condition)) {
        const detail = bound.message ? `: ${compactDiagnosticText(valueToString(message))}` : ''
        evaluationError(ctx, node.p, `Assertion '${bound.conditionText}' failed${detail}`)
      }
      return ctx.builder.alias(occurrence, frameChildren(node, node.children, occurrence, ctx), false)
    }
    case 'cube': {
      const raw = arg(node, 'size', 0, 1, ctx)
      const size = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'cube size') : [finiteNumber(raw, ctx, node.p, 'cube size')]
      const dimensions = [size[0] ?? 1, size[1] ?? size[0] ?? 1, size[2] ?? size[0] ?? 1] as const
      if (dimensions.some(value => value <= 0)) evaluationError(ctx, node.p, 'Cube dimensions must be positive')
      const occurrence = invoke(node, ctx)
      return [ctx.builder.produce(occurrence, {
        kind: 'box',
        valueType: semanticValueType(ctx.builder.languageContract, ctx.builder.languageContract === 'legacy/current' ? 'solid-set' : 'solid', 'd3'),
        size: dimensions,
        center: arg(node, 'center', 1, false, ctx) === true,
      }, primitiveColor(ctx))]
    }
    case 'sphere': {
      let radius = arg(node, 'r', 0, undefined, ctx)
      const diameter = arg(node, 'd', -1, undefined, ctx)
      if (radius === undefined) radius = diameter === undefined ? 1 : finiteNumber(diameter, ctx, node.p, 'sphere diameter') / 2
      const parsedRadius = finiteNumber(radius, ctx, node.p, 'sphere radius')
      if (parsedRadius <= 0) evaluationError(ctx, node.p, 'Sphere radius must be positive')
      const occurrence = invoke(node, ctx)
      const segments = semanticSegments(node, ctx, occurrence, 32, 4)
      return [ctx.builder.produce(occurrence, segments === null ? {
        kind: 'sphere-analytic',
        valueType: semanticValueType(ctx.builder.languageContract, 'solid', 'd3'),
        radius: parsedRadius,
      } : {
        kind: 'sphere-polygonal',
        valueType: semanticValueType(ctx.builder.languageContract, 'solid-set', 'd3'),
        radius: parsedRadius,
        radialSegments: segments,
      }, primitiveColor(ctx))]
    }
    case 'cylinder': return makeCylinder(node, ctx)
    case 'polyhedron': return makePolyhedron(node, ctx)
    case 'square': {
      const raw = arg(node, 'size', 0, 1, ctx)
      const size = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'square size') : [finiteNumber(raw, ctx, node.p, 'square size')]
      const dimensions = [size[0] ?? 1, size[1] ?? size[0] ?? 1] as const
      if (dimensions.some(value => value <= 0)) evaluationError(ctx, node.p, 'Square dimensions must be positive')
      const occurrence = invoke(node, ctx)
      return [ctx.builder.produce(occurrence, {
        kind: 'rectangle',
        valueType: semanticValueType(ctx.builder.languageContract, 'region', 'd2'),
        size: dimensions,
        center: arg(node, 'center', 1, false, ctx) === true,
      }, primitiveColor(ctx))]
    }
    case 'circle': {
      let radius = arg(node, 'r', 0, undefined, ctx)
      const diameter = arg(node, 'd', -1, undefined, ctx)
      if (radius === undefined) radius = diameter === undefined ? 1 : finiteNumber(diameter, ctx, node.p, 'circle diameter') / 2
      const parsedRadius = finiteNumber(radius, ctx, node.p, 'circle radius')
      if (parsedRadius <= 0) evaluationError(ctx, node.p, 'Circle radius must be positive')
      const occurrence = invoke(node, ctx)
      const segments = semanticSegments(node, ctx, occurrence, 48, 3)
      return [ctx.builder.produce(occurrence, segments === null ? {
        kind: 'circle-analytic',
        valueType: semanticValueType(ctx.builder.languageContract, 'region', 'd2'),
        radius: parsedRadius,
      } : {
        kind: 'circle-polygonal',
        valueType: semanticValueType(ctx.builder.languageContract, 'region', 'd2'),
        radius: parsedRadius,
        radialSegments: segments,
      }, primitiveColor(ctx))]
    }
    case 'polygon': return makePolygon(node, ctx)
    case 'translate': {
      const vector = vectorValue(arg(node, 'v', 0, [0, 0, 0], ctx), ctx, node.p, 'translate vector')
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      const matrix = translationMatrix(vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0)
      return transformShapes(occurrence, shapes, () => matrix, ctx)
    }
    case 'rotate': {
      const angle = arg(node, 'a', 0, 0, ctx)
      const axis = arg(node, 'v', 1, undefined, ctx)
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      return transformShapes(occurrence, shapes, shape => {
        if (shape.dimension === 'region2') {
          const degrees = Array.isArray(angle)
            ? vectorValue(angle, ctx, node.p, 'rotation')[2] ?? 0
            : finiteNumber(angle, ctx, node.p, 'rotation')
          return axisAngleMatrixUnchecked([0, 0, 1], degrees)
        }
        if (Array.isArray(angle)) {
          const vector = vectorValue(angle, ctx, node.p, 'rotation')
          return eulerMatrix(vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0)
        }
        const degrees = finiteNumber(angle, ctx, node.p, 'rotation')
        return axis === undefined
          ? axisAngleMatrixUnchecked([0, 0, 1], degrees)
          : axisAngleMatrix(vectorValue(axis, ctx, node.p, 'rotation axis'), degrees, ctx, node.p)
      }, ctx)
    }
    case 'scale': {
      const raw = arg(node, 'v', 0, [1, 1, 1], ctx)
      const values = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'scale vector') : [finiteNumber(raw, ctx, node.p, 'scale')]
      const x = values[0] ?? 1, y = values[1] ?? x, z = values[2] ?? x
      if ([x, y, z].some(value => value === 0)) evaluationError(ctx, node.p, 'Scale values cannot be zero')
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      const matrix = scaleMatrix(x, y, z)
      return transformShapes(occurrence, shapes, () => matrix, ctx)
    }
    case 'mirror': {
      const vector = vectorValue(arg(node, 'v', 0, [1, 0, 0], ctx), ctx, node.p, 'mirror normal')
      const occurrence = invoke(node, ctx)
      return transformShapes(
        occurrence,
        frameChildren(node, node.children, occurrence, ctx),
        shape => mirrorMatrix(vector, shape.dimension, ctx, node.p),
        ctx,
      )
    }
    case 'multmatrix': {
      const matrix = matrixValue(arg(node, 'm', 0, undefined, ctx), ctx, node.p)
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      if (shapes.some(shape => shape.dimension !== 'solid3')) {
        if (ctx.builder.languageContract !== 'openscad-viewer/brep-1') {
          evaluationError(ctx, node.p, 'multmatrix currently supports 3D children only')
        }
        // A retained 2D profile has no implicit projection back from space.
        // Admit only matrices whose image of the entire XY plane is still XY;
        // the backend separately checks singularity and retained curve kinds.
        if (matrix[2] !== 0 || matrix[6] !== 0 || matrix[14] !== 0) {
          evaluationError(ctx, node.p, '2D B-rep multmatrix must preserve the XY plane')
        }
      }
      return transformShapes(occurrence, shapes, () => matrix, ctx)
    }
    case 'color': {
      const color = parseColor(arg(node, 'c', 0, [0.5, 0.5, 0.5], ctx), ctx, node.p)
      const alpha = arg(node, 'alpha', 1, undefined, ctx)
      if (alpha !== undefined) color[3] = clamp01(finiteNumber(alpha, ctx, node.p, 'color alpha'))
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx).map(shape => ({ ...shape, color: [...color] as RGBA }))
      return ctx.builder.alias(occurrence, shapes, false)
    }
    case 'union':
    case 'intersection':
    case 'hull': {
      const occurrence = invoke(node, ctx)
      return combineShapes(occurrence, frameChildren(node, node.children, occurrence, ctx), node.name, ctx, node.p)
    }
    case 'difference': {
      const occurrence = invoke(node, ctx)
      const framed = frameContext(node, occurrence, ctx)
      // The pinned evaluator completes the first-child value (including its
      // implicit union) before it starts evaluating any cutter statement.
      // Keep those two scoped evalNodes calls separate: kernel failure in the
      // base must win over a later cutter-language diagnostic.
      const childContext = framed !== null && isStableProfile(ctx)
        ? enterStableStatementScope(node.children, framed)
        : null
      const baseShapes = framed === null || node.children.length === 0
        ? []
        : childContext === null
          ? evalNodes(node.children.slice(0, 1), framed)
          : evalPreparedNodes(node.children.slice(0, 1), childContext)
      const base = unionReduceInternal(baseShapes, ctx, node.p)
      const cutterShapes = framed === null
        ? []
        : childContext === null
          ? evalNodes(node.children.slice(1), framed)
          : evalPreparedNodes(node.children.slice(1), childContext)
      const cutters = unionReduceInternal(cutterShapes, ctx, node.p)
      if (base === null) {
        if (cutters !== null) {
          ctx.builder.discardedEffects.push({
            tag: 'legacy-difference-cutters',
            root: cutters.node,
            ownerOccurrence: occurrence,
          })
        }
        return []
      }
      if (cutters === null) {
        if (base.identityOccurrence !== null && baseShapes.length === 1) {
          return ctx.builder.alias(occurrence, baseShapes, false)
        }
        return ctx.builder.alias(occurrence, [{
          node: base.node,
          dimension: base.dimension,
          producerOccurrence: occurrence,
          identityOccurrence: occurrence,
          color: base.color,
        }], true)
      }
      if (base.dimension !== cutters.dimension) evaluationError(ctx, node.p, 'difference() cannot mix 2D and 3D children')
      const inputType = ctx.builder.nodes[base.node].valueType
      // Cutting a single solid can split it into multiple disconnected solids.
      // The Boolean carrier is therefore SolidSet even when its base is Solid.
      const valueType: SemanticValueType = {
        ...inputType,
        geometryKind: inputType.space === 'd2' ? 'region' : 'solid-set',
      }
      return [ctx.builder.produce(occurrence, {
        kind: 'boolean', operation: 'difference', valueType,
        inputs: [base.node, cutters.node],
      }, base.color)]
    }
    case 'linear_extrude': {
      const height = finiteNumber(arg(node, 'height', 0, 1, ctx), ctx, node.p, 'extrusion height')
      if (height <= 0) evaluationError(ctx, node.p, 'linear_extrude() height must be positive')
      const twistDegrees = finiteNumber(arg(node, 'twist', -1, 0, ctx), ctx, node.p, 'extrusion twist')
      const rawSlices = Math.max(0, Math.trunc(finiteNumber(arg(node, 'slices', -1, 0, ctx), ctx, node.p, 'extrusion slices')))
      const slices = Math.min(MAX_EXTRUDE_SLICES, rawSlices)
      const rawScale = arg(node, 'scale', -1, [1, 1], ctx)
      const scaleValues = Array.isArray(rawScale) ? vectorValue(rawScale, ctx, node.p, 'extrusion scale') : [finiteNumber(rawScale, ctx, node.p, 'extrusion scale')]
      const scale = [scaleValues[0] ?? 1, scaleValues[1] ?? scaleValues[0] ?? 1] as const
      if (ctx.builder.languageContract === 'openscad-viewer/brep-1' && scale.some(value => value === 0)) {
        evaluationError(ctx, node.p, 'brep-1 extrusion scale cannot contain zero')
      }
      const center = arg(node, 'center', -1, false, ctx) === true
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      if (shapes.length === 0) return []
      if (shapes.some(shape => shape.dimension !== 'region2')) evaluationError(ctx, node.p, 'linear_extrude() requires 2D children')
      const profile = unionReduceInternal(shapes, ctx, node.p)!
      const inputType = ctx.builder.nodes[profile.node].valueType
      return [ctx.builder.produce(occurrence, {
        kind: 'linear-extrude',
        valueType: { ...inputType, geometryKind: 'solid-set', space: 'd3' },
        input: profile.node,
        height, twistDegrees, slices, scale, center,
      }, profile.color)]
    }
    case 'rotate_extrude': {
      const angleDegrees = finiteNumber(arg(node, 'angle', -1, 360, ctx), ctx, node.p, 'revolve angle')
      if (ctx.builder.languageContract === 'openscad-viewer/brep-1' && (angleDegrees <= 0 || angleDegrees > 360)) {
        evaluationError(ctx, node.p, 'brep-1 revolve angle must be in (0, 360] degrees')
      }
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      if (shapes.length === 0) return []
      if (shapes.some(shape => shape.dimension !== 'region2')) evaluationError(ctx, node.p, 'rotate_extrude() requires 2D children')
      const profile = unionReduceInternal(shapes, ctx, node.p)!
      const segments = semanticSegments(node, ctx, occurrence, 48, 3)
      const valueType = { ...ctx.builder.nodes[profile.node].valueType, geometryKind: 'solid-set' as const, space: 'd3' as const }
      return [ctx.builder.produce(occurrence, segments === null ? {
        kind: 'rotate-extrude-analytic', valueType, input: profile.node, angleDegrees,
      } : {
        kind: 'rotate-extrude-polygonal', valueType, input: profile.node, angleDegrees, radialSegments: segments,
      }, profile.color)]
    }
    case 'projection': {
      const cut = arg(node, 'cut', 0, false, ctx) === true
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      return shapes.map(shape => {
        if (shape.dimension !== 'solid3') evaluationError(ctx, node.p, 'projection() requires 3D children')
        return ctx.builder.produce(occurrence, {
          kind: 'projection',
          valueType: { ...nodeValueType(shape, ctx), geometryKind: 'region', space: 'd2' },
          input: shape.node,
          cut,
        }, shape.color)
      })
    }
    case 'offset': {
      const distance = finiteNumber(arg(node, 'r', 0, arg(node, 'delta', 0, 1, ctx), ctx), ctx, node.p, 'offset distance')
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      return shapes.map(shape => {
        if (shape.dimension !== 'region2') evaluationError(ctx, node.p, 'offset() requires 2D children')
        return ctx.builder.produce(occurrence, {
          kind: 'offset', valueType: nodeValueType(shape, ctx), input: shape.node, distance,
        }, shape.color)
      })
    }
    case 'group':
    case 'render': {
      const occurrence = invoke(node, ctx)
      return frameChildren(node, node.children, occurrence, ctx)
    }
    case 'echo': {
      if (!isStableProfile(ctx)) {
        evaluationError(ctx, node.p, 'Unsupported geometry operation echo()')
      }
      const values = Object.entries(node.args).map(([name, expression]) => {
        const value = formatOpenScadValue(evalExpression(expression, ctx))
        return node.argKinds[name] === 'named' ? `${name} = ${value}` : value
      })
      const message = `ECHO:${values.length ? ` ${values.join(', ')}` : ''}`
      ctx.builder.warn(
        null,
        'W_OPENSCAD_ECHO',
        [],
        message,
        { start: node.p, end: node.end },
        false,
      )
      const viewportSlots = viewportSemanticSlots(node, ctx)
      return evalNodes(node.children, viewportSlots.length === 0
        ? ctx
        : { ...ctx, pendingSlots: [...ctx.pendingSlots, ...viewportSlots] })
    }
    case 'if': {
      const condition = arg(node, '_0', 0, false, ctx)
      const branch = isStableProfile(ctx) ? openScadTruthy(condition) : truthy(condition)
      const occurrence = invoke(node, ctx, [semanticSlot('$branch', branch)])
      return frameChildren(node, branch ? node.children : node.alternative, occurrence, ctx, !branch)
    }
    case 'let': {
      if (isStableProfile(ctx)) {
        const args = callExpressionArguments(node)
        const env = evaluateSequentialBindings(args, ctx, 0)
        const slots: SemanticDynamicSlot[] = []
        for (const argument of args) {
          if (argument.name !== undefined && env.has(argument.name)) {
            slots.push(semanticSlot(argument.name, env.get(argument.name)))
          }
        }
        const occurrence = invoke(node, ctx, slots)
        return frameChildren(node, node.children, occurrence, stableOverlayContext(ctx, env))
      }
      const env = new Map(ctx.env)
      const slots: SemanticDynamicSlot[] = []
      const bindings = Object.entries(node.args)
        .filter(([name]) => !name.startsWith('_'))
        .map(([name, expression]) => [name, evalExpression(expression, ctx)] as const)
      for (const [name, value] of bindings) {
        env.set(name, value)
        slots.push(semanticSlot(name, value))
      }
      const occurrence = invoke(node, ctx, slots)
      return frameChildren(node, node.children, occurrence, { ...ctx, env })
    }
    case 'for': {
      if (isStableProfile(ctx)) {
        const bindings = callExpressionArguments(node)
        const output: SemanticShape[] = []
        const visit = (
          bindingIndex: number,
          iterationContext: EvalContext,
          slots: readonly SemanticDynamicSlot[],
        ): void => {
          if (bindingIndex >= bindings.length) {
            const occurrence = invoke(node, ctx, slots)
            output.push(...frameChildren(node, node.children, occurrence, iterationContext))
            if (output.length > MAX_SHAPES) {
              evaluationError(ctx, node.p, `Model exceeds the ${MAX_SHAPES.toLocaleString()} object limit`)
            }
            return
          }
          const binding = bindings[bindingIndex]
          const values = stableIterable(
            evalExpression(binding.value, iterationContext),
            iterationContext,
            binding.p,
          )
          if (binding.name === undefined) {
            stableWarning(ctx, binding.p, 'Ignoring for() iterator without variable name')
            return
          }
          for (const value of values) {
            if (++ctx.budget.ops > MAX_EVAL_OPS) {
              evaluationError(ctx, node.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
            }
            const env = new Map(iterationContext.env)
            env.set(binding.name, value)
            visit(
              bindingIndex + 1,
              stableOverlayContext(iterationContext, env),
              [...slots, semanticSlot(binding.name, value)],
            )
          }
        }
        visit(0, ctx, [])
        return output
      }
      const entries = Object.entries(node.args).filter(([name]) => !name.startsWith('_'))
      if (entries.length !== 1) evaluationError(ctx, node.p, 'for() currently requires one named iterator')
      const [name, expression] = entries[0]
      const values = evalExpression(expression, ctx)
      if (!Array.isArray(values)) evaluationError(ctx, node.p, 'for() iterator must be a vector or range')
      const output: SemanticShape[] = []
      for (const value of values) {
        if (++ctx.budget.ops > MAX_EVAL_OPS) evaluationError(ctx, node.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
        const env = new Map(ctx.env)
        env.set(name, value)
        const occurrence = invoke(node, ctx, [semanticSlot(name, value)])
        output.push(...frameChildren(node, node.children, occurrence, { ...ctx, env }))
        if (output.length > MAX_SHAPES) evaluationError(ctx, node.p, `Model exceeds the ${MAX_SHAPES.toLocaleString()} object limit`)
      }
      return output
    }
    case 'child': {
      if (!isStableProfile(ctx)) evaluationError(ctx, node.p, 'Unsupported geometry operation child()')
      stableCompatibilityDeprecation(node, ctx, 'children()')
      const authored = callExpressionArguments(node)
      const indexValue = authored[0] === undefined
        ? 0
        : evalExpression(authored[0].value, ctx)
      const childIndex = typeof indexValue === 'number' && Number.isFinite(indexValue)
        ? Math.trunc(indexValue)
        : 0
      const occurrence = invoke(node, ctx, [semanticSlot('$index', indexValue)])
      if (childIndex < 0) {
        stableWarning(
          ctx,
          node.p,
          `Negative child index (${childIndex}) not allowed`,
          'W_OPENSCAD_CHILD',
          false,
        )
        return []
      }
      const passed = ctx.callChildren
      if (!passed) return []
      const statement = passed.statements[childIndex]
      if (statement === undefined) {
        stableWarning(
          ctx,
          node.p,
          `Child index (${childIndex}) out of bounds (${passed.statements.length} children)`,
          'W_OPENSCAD_CHILD',
          false,
        )
        return []
      }
      return evalPassedCallChildren(node, ctx, occurrence, indexValue, passed, [statement])
    }
    case 'children': {
      const indexValue = arg(node, '_0', 0, undefined, ctx)
      const childIndex = indexValue === undefined
        ? null
        : Math.trunc(finiteNumber(indexValue, ctx, node.p, 'children index'))
      const occurrence = invoke(node, ctx, [semanticSlot('$index', indexValue)])
      const passed = ctx.callChildren
      if (!passed) return []
      let statements = passed.statements
      if (childIndex !== null) {
        statements = passed.statements[childIndex] ? [passed.statements[childIndex]] : []
      }
      return evalPassedCallChildren(node, ctx, occurrence, indexValue, passed, statements)
    }
  }

  if (isStableProfile(ctx)) {
    const declaration = ctx.stableScope?.moduleDeclaration(node.name)
    if (declaration !== undefined) return evalUserModule(node, declaration.node, ctx, declaration.scope)
  } else {
    const module = ctx.modules.get(node.name)
    if (module) return evalUserModule(node, module, ctx)
  }
  evaluationError(ctx, node.p, `Unsupported geometry operation ${node.name}()`)
}

function evalUserModule(
  call: CallNode,
  module: ModuleNode,
  ctx: EvalContext,
  definitionScope?: OpenScadStableScope,
): SemanticShape[] {
  let env = new Map(ctx.env)
  const slots: SemanticDynamicSlot[] = []
  let moduleContext = ctx
  if (isStableProfile(ctx)) {
    if (definitionScope === undefined) {
      evaluationError(ctx, call.p, `Stable module ${module.name}() is missing its lexical scope`)
    }
    const moduleStack = [...ctx.moduleStack, module.name]
    const definitionEnv = new Map(definitionScope.env)
    overlayDynamicVariables(definitionEnv, ctx.env)
    definitionEnv.set('$children', call.children.length)
    definitionEnv.set('$parent_modules', moduleStack.length)
    moduleContext = {
      ...ctx,
      env: definitionEnv,
      stableScope: definitionScope,
      scopeVisibleBefore: Number.POSITIVE_INFINITY,
      moduleStack,
    }
    const args = callExpressionArguments(call)
    const resolved = resolveStableExpressionArguments(
      args,
      module.params.map(parameter => parameter.name),
      ctx,
    )
    const callerValues = new Map<ExpressionArgument, Value>()
    for (const argument of args) callerValues.set(argument, evalExpression(argument.value, ctx))
    const parameterValues = new Map<string, Value>()
    for (const parameter of module.params) {
      const supplied = resolved.get(parameter.name)
      const value = supplied !== undefined
        ? callerValues.get(supplied)
        : parameter.defaultValue !== undefined
          ? evalExpression(parameter.defaultValue, { ...moduleContext, env: new Map(definitionEnv) })
          : undefined
      parameterValues.set(parameter.name, value)
    }
    env = new Map(definitionEnv)
    for (const parameter of module.params) {
      const value = parameterValues.get(parameter.name)
      env.set(parameter.name, value)
      slots.push(semanticSlot(parameter.name, value))
    }
  } else {
    const positional = Object.entries(call.args)
      .filter(([name]) => name.startsWith('_'))
      .sort(([left], [right]) => Number(left.slice(1)) - Number(right.slice(1)))
    for (let index = 0; index < module.params.length; index++) {
      const parameter = module.params[index]
      const expression = call.args[parameter.name] ?? positional[index]?.[1] ?? parameter.defaultValue
      const value = expression ? evalExpression(expression, { ...ctx, env }) : undefined
      env.set(parameter.name, value)
      slots.push(semanticSlot(parameter.name, value))
    }
  }
  const callOccurrence = invoke(call, ctx, slots)
  const bodyOperation = ctx.builder.registered.bodyByStatement.get(call)
  const expansionOperation = ctx.builder.registered.expansionByCall.get(call)
  if (bodyOperation === undefined || expansionOperation === undefined) {
    evaluationError(ctx, call.p, 'User-module call is missing its semantic expansion frame')
  }
  const bodyOccurrence = ctx.builder.invocationOperation(bodyOperation, callOccurrence)
  const definitionOccurrence = ctx.builder.invocation(module, bodyOccurrence, slots)
  return frameChildren(module, module.children, definitionOccurrence, {
    ...moduleContext,
    env,
    moduleStack: isStableProfile(ctx) ? moduleContext.moduleStack : [...ctx.moduleStack, module.name],
    parentOccurrence: definitionOccurrence,
    pendingSlots: [],
    callChildren: {
      owner: call,
      bodyOccurrence,
      expansionOperation,
      statements: call.children,
      env: new Map(ctx.env),
      scope: isStableProfile(ctx) ? ctx.stableScope : undefined,
      continuation: ctx.callChildren,
    },
  })
}

function countStatements(statements: readonly Statement[]): number {
  let total = 0
  for (const statement of statements) {
    total++
    if (statement.type === 'module') total += countStatements(statement.children)
    else if (statement.type === 'call') total += countStatements(statement.children) + countStatements(statement.alternative)
  }
  return total
}

function resultFromShapes(shapes: readonly SemanticShape[]): SemanticProgramCoreV1['result'] {
  const items: SemanticOutputRef[] = shapes.map(shape => ({
    node: shape.node,
    producerOccurrence: shape.producerOccurrence,
    identityOccurrence: shape.identityOccurrence,
    color: [...shape.color] as SemanticColor,
  }))
  if (items.length === 0) return { tag: 'empty', type: 'never' }
  if (items.length === 1) return { tag: 'single', item: items[0] }
  return { tag: 'multi', items }
}

function remapSemanticNode(node: SemanticNode, id: number, oldToNew: Int32Array): SemanticNode {
  const remap = (input: number) => {
    const mapped = oldToNew[input]
    if (mapped < 0) throw new Error('Semantic DAG canonicalization encountered an unreachable input')
    return mapped
  }
  switch (node.kind) {
    case 'transform':
    case 'linear-extrude':
    case 'rotate-extrude-analytic':
    case 'rotate-extrude-polygonal':
    case 'projection':
    case 'offset':
      return { ...node, id, input: remap(node.input) }
    case 'boolean':
    case 'hull':
      return { ...node, id, inputs: node.inputs.map(remap) }
    default:
      return { ...node, id }
  }
}

/**
 * Hidden reducers are created only after their authored buckets have been
 * evaluated, so append order is not always canonical DFS postorder. Reindex
 * the completed, reachable DAG once in bounded O(nodes + edges) time instead
 * of making reducer construction depend on sibling evaluation timing.
 */
function canonicalizeCompletedDag(
  builder: SemanticProgramBuilder,
  result: SemanticResult,
  terminalDiagnostic: number | null,
  terminalOccurrence: number | null,
): Readonly<{ result: SemanticResult; execution: SemanticExecutionPlanV1 }> {
  if ((terminalDiagnostic === null) !== (terminalOccurrence === null)) {
    throw new Error('Semantic terminal diagnostic and occurrence must be captured atomically')
  }
  const consumed = new Uint8Array(builder.nodes.length)
  for (const node of builder.nodes) {
    for (const input of semanticNodeInputs(node)) consumed[input] = 1
  }
  const terminalRoots = terminalDiagnostic === null
    ? []
    : builder.evaluationOrder.filter(node => consumed[node] === 0)
  const rootCandidates = terminalDiagnostic === null
    ? [
      ...semanticResultItems(result).map(item => item.node),
      ...builder.discardedEffects.map(effect => effect.root),
    ]
    : terminalRoots
  const rootSet = new Set(rootCandidates)
  const roots = builder.evaluationOrder.filter(node => rootSet.has(node))
  const visited = new Uint8Array(builder.nodes.length)
  const postorder: number[] = []
  for (const root of roots) {
    if (visited[root]) continue
    visited[root] = 1
    const stack: Array<{ node: number; nextInput: number; inputs: readonly number[] }> = [{
      node: root,
      nextInput: 0,
      inputs: semanticNodeInputs(builder.nodes[root]),
    }]
    while (stack.length > 0) {
      const frame = stack[stack.length - 1]
      if (frame.nextInput < frame.inputs.length) {
        const input = frame.inputs[frame.nextInput++]
        if (!visited[input]) {
          visited[input] = 1
          stack.push({ node: input, nextInput: 0, inputs: semanticNodeInputs(builder.nodes[input]) })
        }
      } else {
        postorder.push(frame.node)
        stack.pop()
      }
    }
  }
  // Reachability above proves that every retained call belongs to a published
  // result, an authenticated effect, or a terminal prefix. Storage order in
  // schema 1.2 is the authored kernel schedule itself, not a DFS presentation
  // order: independent child constructors may all execute before a mapping
  // operation materializes their outputs.
  postorder.splice(
    0,
    postorder.length,
    ...builder.evaluationOrder.filter(node => visited[node] === 1),
  )
  const oldToNew = new Int32Array(builder.nodes.length)
  oldToNew.fill(-1)
  postorder.forEach((oldId, newId) => { oldToNew[oldId] = newId })
  const canonicalNodes = postorder.map((oldId, newId) => (
    remapSemanticNode(builder.nodes[oldId], newId, oldToNew)
  ))
  builder.nodes.splice(0, builder.nodes.length, ...canonicalNodes)

  // Legacy evaluation can intentionally execute a discarded branch (notably
  // difference cutters after an empty base) for its diagnostics. Keep those
  // effects in the report, but prune the unpublished geometry subtree from the
  // value DAG and occurrence frontier before validation.
  const removedOccurrenceIds = new Set<string>()
  for (const occurrence of builder.occurrences) {
    if (occurrence.node !== null && oldToNew[occurrence.node] < 0) {
      removedOccurrenceIds.add(occurrence.occurrenceId)
    }
  }
  const removeOccurrence = new Uint8Array(builder.occurrences.length)
  // Occurrences are validated/created in parent-before-child order. All rows
  // whose logical occurrence directly references an unpublished node were
  // seeded above, so one forward pass closes both ancestry relations in O(n).
  for (const occurrence of builder.occurrences) {
    if (removedOccurrenceIds.has(occurrence.occurrenceId)
      || (occurrence.parent !== null && removeOccurrence[occurrence.parent] === 1)
      || (occurrence.staticParent !== null && removeOccurrence[occurrence.staticParent] === 1)) {
      removeOccurrence[occurrence.id] = 1
      removedOccurrenceIds.add(occurrence.occurrenceId)
    }
  }
  const occurrenceOldToNew = new Int32Array(builder.occurrences.length)
  occurrenceOldToNew.fill(-1)
  const canonicalOccurrences: SemanticOccurrence[] = []
  for (const occurrence of builder.occurrences) {
    if (removeOccurrence[occurrence.id]) continue
    occurrenceOldToNew[occurrence.id] = canonicalOccurrences.length
    canonicalOccurrences.push(occurrence)
  }
  for (let index = 0; index < canonicalOccurrences.length; index++) {
    const occurrence = canonicalOccurrences[index]
    const remapOccurrence = (reference: number | null): number | null => {
      if (reference === null) return null
      const mapped = occurrenceOldToNew[reference]
      if (mapped < 0) throw new Error('Semantic occurrence pruning retained a reference to a discarded branch')
      return mapped
    }
    const mappedNode = occurrence.node === null ? null : oldToNew[occurrence.node]
    if (mappedNode !== null && mappedNode < 0) {
      throw new Error('Semantic occurrence pruning retained an unpublished node')
    }
    canonicalOccurrences[index] = {
      ...occurrence,
      id: index,
      parent: remapOccurrence(occurrence.parent),
      staticParent: remapOccurrence(occurrence.staticParent),
      node: mappedNode,
    }
  }
  builder.occurrences.splice(0, builder.occurrences.length, ...canonicalOccurrences)
  builder.tessellationIntents.splice(
    0,
    builder.tessellationIntents.length,
    ...builder.tessellationIntents.flatMap(intent => {
      const occurrence = occurrenceOldToNew[intent.occurrence]
      return occurrence < 0 ? [] : [{ ...intent, occurrence }]
    }),
  )
  const remapOutput = (item: SemanticOutputRef): SemanticOutputRef => {
    const node = oldToNew[item.node]
    const producerOccurrence = occurrenceOldToNew[item.producerOccurrence]
    const identityOccurrence = occurrenceOldToNew[item.identityOccurrence]
    if (node < 0 || producerOccurrence < 0 || identityOccurrence < 0) {
      throw new Error('Semantic DAG canonicalization discarded a published output reference')
    }
    return { ...item, node, producerOccurrence, identityOccurrence }
  }
  const canonicalResult: SemanticResult = (() => {
    switch (result.tag) {
      case 'empty': return result
      case 'single': return { tag: 'single', item: remapOutput(result.item) }
      case 'multi': return { tag: 'multi', items: result.items.map(remapOutput) }
    }
  })()
  const producerForRoot = (oldRoot: number): number | null => {
    for (const occurrence of builder.occurrences) {
      if (occurrence.node !== oldToNew[oldRoot]) continue
      const operation = builder.registered.operations[occurrence.operation]
      const node = builder.nodes[oldToNew[oldRoot]]
      if (semanticNodeProducerOperationNames(node).includes(operation.name)) return occurrence.id
    }
    return null
  }
  const canonicalEvaluationOrder = builder.evaluationOrder.map(node => {
    const mapped = oldToNew[node]
    if (mapped < 0) throw new Error('Semantic evaluation order retained a discarded node')
    return mapped
  })
  // Node storage is deliberately canonicalized to the exact authored kernel
  // schedule. Keeping this invariant explicit makes the schedule independently
  // checkable instead of accepting any topological permutation.
  if (canonicalEvaluationOrder.some((node, index) => node !== index)) {
    throw new Error('Semantic DAG canonicalization did not preserve exact kernel evaluation order')
  }
  const canonicalEffects = (terminalDiagnostic === null ? builder.discardedEffects : []).map(effect => {
    const root = oldToNew[effect.root]
    const ownerOccurrence = occurrenceOldToNew[effect.ownerOccurrence]
    if (root < 0 || ownerOccurrence < 0) {
      throw new Error('Semantic discarded effect lost its root or owner during canonicalization')
    }
    return { ...effect, root, ownerOccurrence }
  })
  const terminal = terminalDiagnostic === null ? null : {
    tag: 'legacy-language-error' as const,
    occurrence: occurrenceOldToNew[terminalOccurrence!],
    diagnosticTemplate: terminalDiagnostic,
    prefixFrontier: terminalRoots.map(oldRoot => {
      const root = oldToNew[oldRoot]
      if (root < 0) throw new Error('Semantic terminal prefix lost a root during canonicalization')
      return { root, ownerOccurrence: producerForRoot(oldRoot) }
    }),
  }
  if (terminal !== null && terminal.occurrence < 0) {
    throw new Error('Semantic terminal occurrence was removed during canonicalization')
  }
  return {
    result: canonicalResult,
    execution: {
      version: SEMANTIC_PROGRAM_EXECUTION_VERSION,
      evaluationOrder: canonicalEvaluationOrder,
      discardedEffects: canonicalEffects,
      terminal,
    },
  }
}

/** @internal Use the trust-owning semanticProgramLowerer facade. */
export function lowerOpenSCADToSemanticProgramUnchecked(
  source: string,
  options: SemanticLoweringOptions = {},
): SemanticLoweringSuccess {
  const route = parseGeometrySourceRoutingHeader(source)
  const ast = compileOpenSCAD(source, { languageProfile: options.languageProfile })
  const modules = new Map<string, ModuleNode>()
  const functions = new Map<string, FunctionNode>()
  collectModules(ast, modules)
  collectFunctions(ast, functions)
  const moduleNames = new Set(modules.keys())
  const languageProfile = options.languageProfile ?? 'openscad-viewer-subset@1'
  const registered = registerOperations(
    ast,
    moduleNames,
    languageProfile,
  )
  let builder = new SemanticProgramBuilder(source, route.languageContract, registered)
  const budget = { ops: 0 }
  const reduced = { value: false }
  const valueBudget = { used: 0 }
  const quality = options.quality ?? 'full'
  const env = languageProfile === 'openscad/stable-2021.01'
    ? new Map<string, Value>(Array.from(
        createOpenScadStableRuntimeVariables({ quality, animationTime: options.animationTime }),
        ([name, value]) => [name, Array.isArray(value) ? [...value] : value as Value],
      ))
    : new Map<string, Value>([['$fn', 0], ['$fa', 12], ['$fs', 2]])
  const context: EvalContext = {
    source,
    languageProfile,
    env,
    functions,
    modules,
    builder,
    quality,
    depth: 0,
    functionStack: [],
    moduleStack: [],
    budget,
    reduced,
    valueBudget,
    valueWeights: new WeakMap(),
    valueDepths: new WeakMap(),
    parentOccurrence: null,
    pendingSlots: [],
    shouldAbort: options.shouldAbort,
  }
  let shapes: SemanticShape[] = []
  let terminalError: OpenSCADParseError | LegacyBindingCompatibilityError | null = null
  let terminalDiagnostic: number | null = null
  const captureFailure = (error: unknown): void => {
    const capturable = error instanceof OpenSCADParseError
      || error instanceof LegacyBindingCompatibilityError
    if (!options.captureTerminalFailure || !capturable) throw error
    if (builder.terminalOccurrence === null) {
      throw new Error('Capturable semantic failure has no interrupted occurrence')
    }
    terminalError = error
    terminalDiagnostic = builder.terminal(error, builder.terminalOccurrence)
  }
  try {
    shapes = evalNodes(ast, context, false)
  } catch (error) {
    if (error instanceof StableViewportRootSelection) {
      const detachedContinuations = detachedChildrenContinuationMap(
        error.node,
        error.context.callChildren,
      )
      const rootBuilder = new SemanticProgramBuilder(
        source,
        route.languageContract,
        registerOperations(ast, moduleNames, languageProfile, error.node, detachedContinuations),
      )
      rootBuilder.inheritDiagnostics(builder, error.diagnosticsBeforeCandidate)
      builder = rootBuilder
      try {
        shapes = evalPreparedNodes([error.node], {
          ...error.context,
          builder,
          parentOccurrence: null,
          pendingSlots: [],
          viewportRootLocked: true,
          viewportRootOwner: error.node,
        })
      } catch (rootError) {
        captureFailure(rootError)
      }
    } else {
      captureFailure(error)
    }
  }
  const canonical = canonicalizeCompletedDag(
    builder,
    terminalError === null ? resultFromShapes(shapes) : { tag: 'empty', type: 'never' },
    terminalDiagnostic,
    builder.terminalOccurrence,
  )
  const coreDraft: SemanticProgramCoreV1 = {
    schema: 'semantic-program-core',
    schemaVersion: { major: 1, minor: 2 },
    requiredFeatures: [SEMANTIC_PROGRAM_EXECUTION_FEATURE],
    identityVersion: SEMANTIC_PROGRAM_IDENTITY,
    language: {
      contract: route.languageContract,
      semanticsRevision: route.languageContract === 'legacy/current' ? '1.0.0' : 'brep-1.0.0',
      capabilityGraphVersion: SEMANTIC_PROGRAM_CAPABILITY_GRAPH,
    },
    units: {
      length: 'millimeter', angle: 'degree', handedness: 'right', upAxis: 'z',
      matrixLayout: 'column-major', composition: 'parent-times-local',
    },
    operations: builder.registered.operations,
    occurrences: builder.occurrences,
    nodes: builder.nodes,
    execution: canonical.execution,
    result: canonical.result,
    declaredCapabilities: route.requiredCapabilities,
    capabilityClosure: [],
    diagnosticTemplates: builder.diagnosticTemplates,
  }
  const core: SemanticProgramCoreV1 = {
    ...coreDraft,
    capabilityClosure: deriveSemanticCapabilityClosure(coreDraft),
  }
  const envelope = {
    schema: 'semantic-program-envelope' as const,
    schemaVersion: { major: 1 as const, minor: 2 as const },
    source: semanticSourceDescriptor(source),
    core,
    provenance: builder.registered.provenance,
    tessellationIntents: [...builder.tessellationIntents].sort((left, right) => left.occurrence - right.occurrence),
    diagnostics: builder.diagnostics,
  }
  const program = normalizeSemanticProgram(envelope, source)
  const canonicalSnapshot = encodeSemanticProgram(program)
  const attestation = attestSemanticProgram(program)
  const report = Object.freeze({
    astStatements: countStatements(ast),
    operations: program.core.operations.length,
    occurrences: program.core.occurrences.length,
    nodes: program.core.nodes.length,
    evaluationSteps: budget.ops,
    allocatedValueUnits: valueBudget.used,
  })
  if (terminalError !== null) Object.freeze(terminalError)
  const success: SemanticLoweringSuccess = Object.freeze({
    tag: 'success' as const,
    program,
    sourceText: source,
    get canonicalBytes() { return new Uint8Array(canonicalSnapshot) },
    attestation,
    warnings: Object.freeze([...builder.warnings]),
    fullEquivalent: !reduced.value,
    terminalError,
    report,
  })
  return success
}
