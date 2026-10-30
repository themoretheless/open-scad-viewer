// lowererBuilder: registerOperations.
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
import { PassedCallChildren, RegisteredOperations, operationCategory, semanticOperationName } from './MAX_SHAPES'


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
