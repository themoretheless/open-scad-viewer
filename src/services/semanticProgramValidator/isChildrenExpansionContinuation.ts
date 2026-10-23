import { SEMANTIC_PROGRAM_LIMITS, deriveSemanticOccurrenceId, deriveSemanticSceneEntityId } from '../../core/semanticProgram'
import type { SemanticDynamicSlot, SemanticNode, SemanticOccurrence, SemanticStaticOperation } from '../../core/semanticProgram'
import type { SemanticOccurrenceValidationIndex } from '../semanticProgramValidator'
import { arrayValue, exactKeys, fail, integer, stringValue, tupleNumbers, validateIdentityValue } from './fail'
import { consumeOccurrenceProofBudget, identityValueKey, moduleActivationAnchor, sameDynamicSlots } from './validateNodes'

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

export function validateOccurrences(
  value: unknown,
  operations: readonly SemanticStaticOperation[],
  nodes: readonly SemanticNode[],
): readonly SemanticOccurrence[] {
  const occurrences = arrayValue(value, '$.core.occurrences', SEMANTIC_PROGRAM_LIMITS.occurrences)
  const sceneIds = new Set<string>()
  const occurrenceGroups = new Map<string, number[]>()
  const duplicateSlotGroups = new Map<string, Map<number, string>>()
  const reparentedStaticRoots: number[] = []
  const pendingContinuations: Array<Readonly<{
    occurrence: number
    operation: number
    parent: number | null
    staticParent: number
    slots: readonly SemanticDynamicSlot[]
  }>> = []
  const proofBudget = { steps: 0 }
  occurrences.forEach((occurrence, index) => {
    const path = `$.core.occurrences[${index}]`
    exactKeys(occurrence, [
      'id', 'occurrenceId', 'operation', 'parent', 'staticParent', 'dynamicSlots', 'node',
      'outputOrdinal', 'sceneEntityId',
    ], path)
    if (occurrence.id !== index) fail('E_SEMANTIC_ORDER', `${path}.id`, 'occurrence IDs must equal array positions')
    const operation = integer(occurrence.operation, `${path}.operation`, 0, Math.max(0, operations.length - 1))
    if (operations.length === 0) fail('E_SEMANTIC_REFERENCE', `${path}.operation`, 'occurrence requires an operation')
    const parent = occurrence.parent === null ? null : integer(occurrence.parent, `${path}.parent`, 0, index - 1)
    const staticParent = operations[operation].parent
    const staticParentOccurrence = occurrence.staticParent === null
      ? null
      : integer(occurrence.staticParent, `${path}.staticParent`, 0, index - 1)
    const slots = arrayValue(occurrence.dynamicSlots, `${path}.dynamicSlots`, 1_000)
    const slotNames = new Set<string>()
    slots.forEach((slot, slotIndex) => {
      const slotPath = `${path}.dynamicSlots[${slotIndex}]`
      exactKeys(slot, ['name', 'value', 'duplicateOrdinal'], slotPath)
      const name = stringValue(slot.name, `${slotPath}.name`, 128)
      if (slotNames.has(name)) fail('E_SEMANTIC_IDENTITY', `${slotPath}.name`, 'dynamic slot names must be unique in authored order')
      slotNames.add(name)
      validateIdentityValue(slot.value, `${slotPath}.value`)
      integer(slot.duplicateOrdinal, `${slotPath}.duplicateOrdinal`, 0, 1_000_000)
    })
    if (staticParent === null) {
      if (staticParentOccurrence !== null) fail('E_SEMANTIC_IDENTITY', `${path}.staticParent`, 'root static operation requires null staticParent')
      if (parent !== null) {
        const rootOperation = operations[operation]
        if (rootOperation.category !== 'module'
          || rootOperation.structuralPath.at(-1)?.kind !== 'module') {
          fail('E_SEMANTIC_IDENTITY', `${path}.parent`, 'a static-root operation cannot have a runtime parent outside a module-definition activation')
        }
        reparentedStaticRoots.push(index)
      }
    } else {
      if (staticParentOccurrence === null
        || (occurrences[staticParentOccurrence] as SemanticOccurrence).operation !== staticParent) {
        fail('E_SEMANTIC_IDENTITY', `${path}.staticParent`, 'staticParent must instantiate the static parent operation')
      }
      let cursor = parent
      let crossedNonExpansionFrame = false
      while (cursor !== staticParentOccurrence) {
        consumeOccurrenceProofBudget({ proofBudget })
        if (cursor === null) fail('E_SEMANTIC_IDENTITY', `${path}.staticParent`, 'staticParent is not on the runtime ancestor chain')
        const skipped = occurrences[cursor] as SemanticOccurrence
        if (skipped.operation === staticParent) fail('E_SEMANTIC_IDENTITY', `${path}.staticParent`, 'staticParent must be the nearest matching ancestor')
        const category = operations[skipped.operation].category
        if (category !== 'control' && category !== 'module') crossedNonExpansionFrame = true
        cursor = skipped.parent
      }
      const expansion = operations[operation].name === '$expansion'
      if (expansion) {
        pendingContinuations.push(Object.freeze({
          occurrence: index,
          operation,
          parent,
          staticParent: staticParentOccurrence,
          slots: slots as unknown as readonly SemanticDynamicSlot[],
        }))
      } else if (crossedNonExpansionFrame) {
        fail('E_SEMANTIC_IDENTITY', `${path}.parent`, 'only control/module frames may separate parent from staticParent')
      }
    }
    const parentOccurrenceId = parent === null ? null : (occurrences[parent] as SemanticOccurrence).occurrenceId
    const staticParentOccurrenceId = staticParentOccurrence === null
      ? null
      : (occurrences[staticParentOccurrence] as SemanticOccurrence).occurrenceId
    const expectedOccurrenceId = deriveSemanticOccurrenceId(
      parentOccurrenceId,
      staticParentOccurrenceId,
      operations[operation].operationId,
      slots as unknown as readonly SemanticDynamicSlot[],
    )
    if (occurrence.occurrenceId !== expectedOccurrenceId) fail('E_SEMANTIC_IDENTITY', `${path}.occurrenceId`, 'occurrence ID does not match operation, parent, and dynamic slots')
    const group = occurrenceGroups.get(expectedOccurrenceId) ?? []
    if (group.length > 0) {
      const first = occurrences[group[0]] as SemanticOccurrence
      if (first.operation !== operation || first.parent !== parent
        || first.staticParent !== staticParentOccurrence
        || !sameDynamicSlots(first.dynamicSlots, slots as unknown as readonly SemanticDynamicSlot[])) {
        fail('E_SEMANTIC_IDENTITY', path, 'rows sharing an occurrence ID must describe the same logical evaluation')
      }
    }
    group.push(index)
    occurrenceGroups.set(expectedOccurrenceId, group)
    ;(slots as unknown as readonly SemanticDynamicSlot[]).forEach((slot, slotIndex) => {
      const valueKey = identityValueKey(slot.value)
      const duplicateKey = `${String(parent)}|${operation}|${slotIndex}|${slot.name.length}:${slot.name}${valueKey.length}:${valueKey}`
      const ordinals = duplicateSlotGroups.get(duplicateKey) ?? new Map<number, string>()
      const existing = ordinals.get(slot.duplicateOrdinal)
      if (existing !== undefined && existing !== expectedOccurrenceId) {
        fail('E_SEMANTIC_IDENTITY', `${path}.dynamicSlots[${slotIndex}].duplicateOrdinal`, 'duplicate ordinals must distinguish equal dynamic values')
      }
      if (existing === undefined && slot.duplicateOrdinal !== ordinals.size) {
        fail('E_SEMANTIC_ORDER', `${path}.dynamicSlots[${slotIndex}].duplicateOrdinal`, 'duplicate ordinals must be dense in language evaluation order')
      }
      ordinals.set(slot.duplicateOrdinal, expectedOccurrenceId)
      duplicateSlotGroups.set(duplicateKey, ordinals)
    })
    if (occurrence.node === null) {
      if (occurrence.outputOrdinal !== null || occurrence.sceneEntityId !== null) fail('E_SEMANTIC_IDENTITY', path, 'non-producing occurrence cannot own an output identity')
    } else {
      integer(occurrence.node, `${path}.node`, 0, Math.max(0, nodes.length - 1))
      if (nodes.length === 0) fail('E_SEMANTIC_REFERENCE', `${path}.node`, 'occurrence references absent node')
      if (occurrence.outputOrdinal === null) {
        if (occurrence.sceneEntityId !== null) fail('E_SEMANTIC_IDENTITY', `${path}.sceneEntityId`, 'producer-only occurrence cannot claim scene identity')
      } else {
        const outputOrdinal = integer(occurrence.outputOrdinal, `${path}.outputOrdinal`, 0, 1_000_000)
        const expectedSceneId = deriveSemanticSceneEntityId(expectedOccurrenceId, outputOrdinal)
        if (occurrence.sceneEntityId !== expectedSceneId) fail('E_SEMANTIC_IDENTITY', `${path}.sceneEntityId`, 'scene entity ID does not match occurrence and output ordinal')
        if (sceneIds.has(expectedSceneId)) fail('E_SEMANTIC_IDENTITY', `${path}.sceneEntityId`, 'scene entity IDs must be unique')
        sceneIds.add(expectedSceneId)
      }
    }
  })

  const operationChildren: number[][] = Array.from({ length: operations.length }, () => [])
  operations.forEach(operation => {
    if (operation.parent !== null) operationChildren[operation.parent].push(operation.id)
  })
  const runtimeChildren: number[][] = Array.from({ length: occurrences.length }, () => [])
  occurrences.forEach((rawOccurrence, index) => {
    const parent = (rawOccurrence as SemanticOccurrence).parent
    if (parent !== null) runtimeChildren[parent].push(index)
  })
  const validationIndex: SemanticOccurrenceValidationIndex = {
    operationChildren,
    runtimeChildren,
    activationByDefinition: new Map(),
    proofBudget,
  }
  for (const definitionOccurrence of reparentedStaticRoots) {
    if (!moduleActivationAnchor(definitionOccurrence, occurrences, operations, validationIndex)) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${definitionOccurrence}].parent`, 'module-definition activation is not anchored to its exact matching call body')
    }
  }
  for (const continuation of pendingContinuations) {
    if (!isChildrenExpansionContinuation(
      continuation.occurrence,
      continuation.operation,
      continuation.parent,
      continuation.staticParent,
      continuation.slots,
      occurrences,
      operations,
      validationIndex,
    )) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${continuation.occurrence}].parent`, '$expansion must be the exact children() continuation of its matching caller')
    }
  }
  occurrences.forEach((occurrence, index) => {
    for (const field of ['parent', 'staticParent'] as const) {
      const reference = (occurrence as Record<string, unknown>)[field] as number | null
      if (reference === null) continue
      const referencedId = (occurrences[reference] as SemanticOccurrence).occurrenceId
      if (occurrenceGroups.get(referencedId)?.[0] !== reference) {
        fail('E_SEMANTIC_ORDER', `$.core.occurrences[${index}].${field}`, 'occurrence references must target the canonical first row of a logical occurrence')
      }
    }
  })
  for (const indexes of occurrenceGroups.values()) {
    const outputRows = indexes.filter(index => (occurrences[index] as Record<string, unknown>).outputOrdinal !== null)
    if (indexes.length > 1 && outputRows.length !== indexes.length) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${indexes[1]}]`, 'an occurrence ID may repeat only for its output slots')
    }
    outputRows.forEach((index, ordinal) => {
      if ((occurrences[index] as Record<string, unknown>).outputOrdinal !== ordinal) {
        fail('E_SEMANTIC_ORDER', `$.core.occurrences[${index}].outputOrdinal`, 'output ordinals must be unique, ordered, and dense from zero per occurrence ID')
      }
    })
  }
  for (const ordinals of duplicateSlotGroups.values()) {
    const ordered = [...ordinals.keys()].sort((left, right) => left - right)
    if (ordered.some((ordinal, index) => ordinal !== index)) {
      fail('E_SEMANTIC_ORDER', '$.core.occurrences', 'duplicate ordinals for equal dynamic slot values must be dense from zero')
    }
  }
  return occurrences as unknown as readonly SemanticOccurrence[]
}

export function occurrenceRoot(index: number, occurrences: readonly SemanticOccurrence[]): number {
  let current = index
  while (occurrences[current].parent !== null) current = occurrences[current].parent!
  return current
}

export function occurrenceIsDescendantOf(
  candidate: number,
  ancestor: number,
  occurrences: readonly SemanticOccurrence[],
): boolean {
  let current: number | null = candidate
  while (current !== null) {
    if (current === ancestor) return true
    current = occurrences[current].parent
  }
  return false
}

export function nodePreservesIdentity(root: number, target: number, nodes: readonly SemanticNode[]): boolean {
  let current = root
  while (true) {
    if (current === target) return true
    const node = nodes[current]
    if (node.kind !== 'transform') return false
    current = node.input
  }
}

export function validateOutputRef(
  value: unknown,
  path: string,
  nodes: readonly SemanticNode[],
  occurrences: readonly SemanticOccurrence[],
  canonicalOccurrenceRows: readonly number[],
): number {
  exactKeys(value, ['node', 'producerOccurrence', 'identityOccurrence', 'color'], path)
  const node = integer(value.node, `${path}.node`, 0, Math.max(0, nodes.length - 1))
  const producerOccurrence = integer(value.producerOccurrence, `${path}.producerOccurrence`, 0, Math.max(0, occurrences.length - 1))
  const identityOccurrence = integer(value.identityOccurrence, `${path}.identityOccurrence`, 0, Math.max(0, occurrences.length - 1))
  if (nodes.length === 0 || occurrences.length === 0) fail('E_SEMANTIC_REFERENCE', path, 'output requires a node occurrence')
  if (occurrences[producerOccurrence].node !== node) fail('E_SEMANTIC_IDENTITY', path, 'output node must match its producer occurrence')
  if (occurrences[identityOccurrence].sceneEntityId === null) fail('E_SEMANTIC_IDENTITY', path, 'output identity occurrence must own a scene entity')
  const identityNode = occurrences[identityOccurrence].node
  const canonicalProducerOccurrence = canonicalOccurrenceRows[producerOccurrence]
  const identityIsInProducingBranch = occurrenceIsDescendantOf(
    identityOccurrence,
    producerOccurrence,
    occurrences,
  ) || (
    occurrences[canonicalProducerOccurrence].occurrenceId === occurrences[producerOccurrence].occurrenceId
    && occurrenceIsDescendantOf(identityOccurrence, canonicalProducerOccurrence, occurrences)
  )
  if (identityNode === null || !nodePreservesIdentity(node, identityNode, nodes)
    || occurrenceRoot(canonicalProducerOccurrence, occurrences) !== occurrenceRoot(identityOccurrence, occurrences)
    || !identityIsInProducingBranch) {
    fail('E_SEMANTIC_IDENTITY', `${path}.identityOccurrence`, 'output identity must belong to the producing evaluation branch and node lineage')
  }
  const color = tupleNumbers(value.color, 4, `${path}.color`)
  if (color.some(channel => channel < 0 || channel > 1)) fail('E_SEMANTIC_NUMBER', `${path}.color`, 'color channels must be in [0, 1]')
  return node
}
