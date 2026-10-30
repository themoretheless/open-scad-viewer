// occurrenceValidator: SemanticModuleActivationAnchor.
import {
  deriveSemanticOccurrenceId,
  deriveSemanticSceneEntityId,
  SEMANTIC_PROGRAM_LIMITS,
  type SemanticDynamicSlot,
  type SemanticNode,
  type SemanticOccurrence,
  type SemanticStaticOperation,
} from '../../core/semanticProgram'

import {
  arrayValue,
  exactKeys,
  fail,
  identityValueKey,
  integer,
  isRecord,
  sameDynamicBindings,
  sameDynamicSlots,
  stringValue,
  tupleNumbers,
  validateIdentityValue,
} from '.././schemaPrimitives'



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


export function consumeOccurrenceProofBudget(
  validationIndex: Pick<SemanticOccurrenceValidationIndex, 'proofBudget'>,
  steps = 1,
): void {
  validationIndex.proofBudget.steps += steps
  if (validationIndex.proofBudget.steps > SEMANTIC_PROGRAM_LIMITS.snapshotValues) {
    fail('E_SEMANTIC_LIMIT', '$.core.occurrences', 'occurrence ancestry/continuation proof exceeds its bounded work budget')
  }
}


export function moduleActivationAnchor(
  definitionOccurrenceIndex: number,
  occurrences: readonly unknown[],
  operations: readonly SemanticStaticOperation[],
  validationIndex: SemanticOccurrenceValidationIndex,
): SemanticModuleActivationAnchor | null {
  const cached = validationIndex.activationByDefinition.get(definitionOccurrenceIndex)
  if (cached !== undefined) return cached
  const reject = (): null => {
    validationIndex.activationByDefinition.set(definitionOccurrenceIndex, null)
    return null
  }
  const definitionOccurrence = occurrences[definitionOccurrenceIndex] as SemanticOccurrence
  const definitionOperation = operations[definitionOccurrence.operation]
  if (definitionOperation.parent !== null
    || definitionOperation.category !== 'module'
    || definitionOperation.name.startsWith('$')
    || definitionOperation.structuralPath.at(-1)?.kind !== 'module') return reject()
  const bodyOccurrenceIndex = definitionOccurrence.parent
  if (bodyOccurrenceIndex === null) return reject()
  const bodyOccurrence = occurrences[bodyOccurrenceIndex] as SemanticOccurrence
  const bodyOperation = operations[bodyOccurrence.operation]
  const bodySegment = bodyOperation.structuralPath.at(-1)
  if (bodyOperation.category !== 'control'
    || bodyOperation.name !== '$body'
    || bodySegment?.kind !== 'body'
    || bodySegment.name !== '$body'
    || bodySegment.ordinal !== 0
    || bodyOperation.parent === null) return reject()
  const callOccurrenceIndex = bodyOccurrence.parent
  if (callOccurrenceIndex === null) return reject()
  const callOccurrence = occurrences[callOccurrenceIndex] as SemanticOccurrence
  const callOperation = operations[callOccurrence.operation]
  if (bodyOperation.parent !== callOccurrence.operation
    || callOperation.category !== 'module'
    || callOperation.name.startsWith('$')
    || callOperation.structuralPath.at(-1)?.kind !== 'call'
    || callOperation.name !== definitionOperation.name
    || !sameDynamicBindings(callOccurrence.dynamicSlots, definitionOccurrence.dynamicSlots)) return reject()

  const staticBodies = validationIndex.operationChildren[callOccurrence.operation] ?? []
  const staticExpansions = validationIndex.operationChildren[bodyOccurrence.operation] ?? []
  consumeOccurrenceProofBudget(validationIndex, staticBodies.length + staticExpansions.length)
  if (staticBodies.length !== 1 || staticBodies[0] !== bodyOccurrence.operation
    || staticExpansions.length !== 1) return reject()
  const soleStaticExpansion = staticExpansions[0]
  const expansionOperation = operations[soleStaticExpansion]
  const expansionSegment = expansionOperation.structuralPath.at(-1)
  if (expansionOperation.category !== 'control'
    || expansionOperation.name !== '$expansion'
    || expansionSegment?.kind !== 'control'
    || expansionSegment.name !== '$expansion'
    || expansionSegment.ordinal !== 0) return reject()

  const callRuntimeChildren = validationIndex.runtimeChildren[callOccurrenceIndex] ?? []
  const bodyRuntimeChildren = validationIndex.runtimeChildren[bodyOccurrenceIndex] ?? []
  consumeOccurrenceProofBudget(validationIndex, callRuntimeChildren.length + bodyRuntimeChildren.length)
  if (callRuntimeChildren.length !== 1 || callRuntimeChildren[0] !== bodyOccurrenceIndex
    || bodyRuntimeChildren.length !== 1 || bodyRuntimeChildren[0] !== definitionOccurrenceIndex) return reject()
  const anchor = {
    callOccurrence: callOccurrenceIndex,
    bodyOccurrence: bodyOccurrenceIndex,
    definitionOccurrence: definitionOccurrenceIndex,
    expansionOperation: soleStaticExpansion,
  }
  validationIndex.activationByDefinition.set(definitionOccurrenceIndex, anchor)
  return anchor
}


export function isChildrenExpansionContinuation(
  occurrenceIndex: number,
  operation: number,
  parent: number | null,
  staticParent: number,
  slots: readonly SemanticDynamicSlot[],
  occurrences: readonly unknown[],
  operations: readonly SemanticStaticOperation[],
  validationIndex: SemanticOccurrenceValidationIndex,
): boolean {
  if (parent === null) return false
  const expansionOperation = operations[operation]
  const childrenOccurrence = occurrences[parent] as SemanticOccurrence
  const childrenOperation = operations[childrenOccurrence.operation]
  const callerBodyOccurrence = occurrences[staticParent] as SemanticOccurrence
  const expansionSegment = expansionOperation.structuralPath.at(-1)
  const childrenSegment = childrenOperation.structuralPath.at(-1)
  if (expansionOperation.name !== '$expansion'
    || expansionOperation.category !== 'control'
    || expansionSegment?.kind !== 'control'
    || expansionSegment.name !== '$expansion'
    || expansionSegment.ordinal !== 0
    || expansionOperation.parent !== callerBodyOccurrence.operation
    || childrenOperation.name !== 'children'
    || childrenOperation.category !== 'control'
    || childrenSegment?.kind !== 'control'
    || childrenSegment.name !== 'children'
  ) return false

  const childrenSlots = childrenOccurrence.dynamicSlots
  if (childrenSlots.length !== 1
    || slots.length !== 1
    || childrenSlots[0].name !== '$index'
    || slots[0].name !== '$index'
    || identityValueKey(childrenSlots[0].value) !== identityValueKey(slots[0].value)) return false

  // A selected `!children()` is detached from the module invocation that
  // supplied its dynamic continuation. The lowerer represents each bounded
  // continuation as the sole static/runtime $expansion directly beneath its
  // consuming children() call; all expanded statements must stay in that
  // exact static subtree.
  if (staticParent === parent
    && callerBodyOccurrence.operation === childrenOccurrence.operation) {
    const staticChildren = validationIndex.operationChildren[childrenOccurrence.operation] ?? []
    const runtimeChildren = validationIndex.runtimeChildren[parent] ?? []
    const expandedChildren = validationIndex.runtimeChildren[occurrenceIndex] ?? []
    consumeOccurrenceProofBudget(
      validationIndex,
      staticChildren.length + runtimeChildren.length + expandedChildren.length,
    )
    return staticChildren.length === 1
      && staticChildren[0] === operation
      && runtimeChildren.length === 1
      && runtimeChildren[0] === occurrenceIndex
      && expandedChildren.every(candidate => (
        operations[(occurrences[candidate] as SemanticOccurrence).operation].parent === operation
      ))
  }

  let cursor = childrenOccurrence.parent
  let definitionOccurrence: SemanticOccurrence | null = null
  while (cursor !== null && cursor !== staticParent) {
    const candidate = occurrences[cursor] as SemanticOccurrence
    const candidateOperation = operations[candidate.operation]
    const candidatePath = candidateOperation.structuralPath
    const childrenPath = childrenOperation.structuralPath
    consumeOccurrenceProofBudget(validationIndex, candidatePath.length)
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
    consumeOccurrenceProofBudget(validationIndex)
    cursor = candidate.parent
  }
  if (!definitionOccurrence || definitionOccurrence.parent !== staticParent) return false
  const definitionOperation = operations[definitionOccurrence.operation]
  const activation = moduleActivationAnchor(definitionOccurrence.id, occurrences, operations, validationIndex)
  if (!activation
    || activation.bodyOccurrence !== staticParent
    || activation.expansionOperation !== operation) return false
  const definitionPath = definitionOperation.structuralPath
  const childrenPath = childrenOperation.structuralPath
  consumeOccurrenceProofBudget(validationIndex, definitionPath.length)
  const lexicallyContainsChildren = definitionPath.length < childrenPath.length
    && definitionPath.every((segment, index) => {
      const candidate = childrenPath[index]
      return segment.kind === candidate.kind
        && segment.name === candidate.name
        && segment.ordinal === candidate.ordinal
    })
  const expansionRuntimeChildren = validationIndex.runtimeChildren[parent] ?? []
  const expandedChildren = validationIndex.runtimeChildren[occurrenceIndex] ?? []
  consumeOccurrenceProofBudget(validationIndex, expansionRuntimeChildren.length + expandedChildren.length)
  const expandedChildrenStayInStaticSubtree = expandedChildren.every(candidate => (
    operations[(occurrences[candidate] as SemanticOccurrence).operation].parent === operation
  ))
  return lexicallyContainsChildren
    && expansionRuntimeChildren.length === 1
    && expansionRuntimeChildren[0] === occurrenceIndex
    && expandedChildrenStayInStaticSubtree
}
