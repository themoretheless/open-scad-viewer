import { AbortedError } from '../openscadErrors'
import type { CallNode, FunctionNode, ModuleNode, OpenScadLanguageProfile, Statement, Value } from '../openscadCompiler'
import type { EvalContext, PassedCallChildren, RegisteredOperations } from '../openscadSemanticLowerer'
import { MAX_EVALUATED_VALUE_UNITS, evaluationError, isFunctionValue } from '../openScadStableExpressionEval'
import { deriveSemanticAmbiguityGroupId, deriveSemanticOperationId } from '../../core/semanticProgram'
import type { SemanticIdentityValue, SemanticOperationCategory, SemanticStaticOperation, SemanticStructuralPathSegment, SemanticValueType } from '../../core/semanticProgram'
import { evalExpression } from './evalExpression'
import { formatOpenScadValue, isOpenScadRange } from '../openScadValueSemantics'

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

export function poll(ctx: EvalContext): void {
  if (ctx.shouldAbort?.()) throw new AbortedError()
}

export function registerStringValue(value: string, ctx: EvalContext, position: number, label = 'Evaluation'): string {
  ctx.valueBudget.used += Math.max(1, value.length)
  if (ctx.valueBudget.used > MAX_EVALUATED_VALUE_UNITS) evaluationError(ctx, position, `${label} exceeds the ${MAX_EVALUATED_VALUE_UNITS.toLocaleString('en-US')} value-allocation budget`)
  return value
}

export function finiteNumber(value: Value, ctx: EvalContext, position: number, label: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) evaluationError(ctx, position, `${label} must be a finite number`)
  return canonicalNumber(value)
}

export function vectorValue(value: Value, ctx: EvalContext, position: number, label: string): number[] {
  if (!Array.isArray(value)) evaluationError(ctx, position, `${label} must be a vector`)
  return value.map(item => finiteNumber(item, ctx, position, label))
}

export function truthy(value: Value): boolean {
  return value !== false && value !== undefined && value !== 0 && value !== ''
    && (!Array.isArray(value) || value.length > 0)
}

export function deepEqual(left: Value, right: Value): boolean {
  if (Array.isArray(left) && Array.isArray(right)) {
    return left.length === right.length && left.every((value, index) => deepEqual(value, right[index]))
  }
  return left === right
}

export function valueToString(value: Value): string {
  if (Array.isArray(value)) return `[${value.map(valueToString).join(', ')}]`
  if (isFunctionValue(value)) return 'function(...)'
  if (value === undefined) return 'undef'
  return String(value)
}

export function resolveStableVariable(
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

export function stableWarning(
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
