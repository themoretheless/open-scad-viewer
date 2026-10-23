import type { GeometryQuality } from '../core/build'
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
} from '../core/semanticProgram'
import { sha256Hex } from '../core/sha256'
import type {
  CallNode,
  FunctionNode,
  ModuleNode,
  OpenScadLanguageProfile,
  Statement,
  Value,
} from '../services/openscadCompiler'
import { OpenSCADParseError } from '../services/openscadErrors'
import {
  formatOpenScadValue,
  isOpenScadRange,
} from '../services/openScadValueSemantics'
import type { OpenScadStableScope } from '../services/openScadStableScope'
import { isFunctionValue } from '../services/openScadStableExpressionEval'

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

export function registerOperations(
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

export function countStatements(statements: readonly Statement[]): number {
  let total = 0
  for (const statement of statements) {
    total++
    if (statement.type === 'module') total += countStatements(statement.children)
    else if (statement.type === 'call') total += countStatements(statement.children) + countStatements(statement.alternative)
  }
  return total
}

export function resultFromShapes(shapes: readonly SemanticShape[]): SemanticProgramCoreV1['result'] {
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

export function remapSemanticNode(node: SemanticNode, id: number, oldToNew: Int32Array): SemanticNode {
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
export function canonicalizeCompletedDag(
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
