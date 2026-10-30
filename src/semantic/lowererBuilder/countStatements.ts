// lowererBuilder: countStatements.
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
import { SemanticShape } from './MAX_SHAPES'
import { SemanticProgramBuilder } from './SemanticProgramBuilder'


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
