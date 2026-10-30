// occurrenceValidator: validateResult.
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
import { validateOutputRef } from './validateOccurrences'


export function validateResult(
  value: unknown,
  nodes: readonly SemanticNode[],
  occurrences: readonly SemanticOccurrence[],
): readonly number[] {
  const firstRowByOccurrenceId = new Map<string, number>()
  const canonicalOccurrenceRows = occurrences.map((occurrence, index) => {
    const first = firstRowByOccurrenceId.get(occurrence.occurrenceId)
    if (first !== undefined) return first
    firstRowByOccurrenceId.set(occurrence.occurrenceId, index)
    return index
  })
  if (!isRecord(value) || typeof value.tag !== 'string') fail('E_SEMANTIC_SCHEMA', '$.core.result', 'expected a discriminated result')
  const rootNodes: number[] = []
  const outputOccurrences = new Set<number>()
  let priorIdentityOccurrence = -1
  const recordIdentityOccurrence = (item: Record<string, unknown>, path: string) => {
    const occurrence = item.identityOccurrence as number
    if (occurrence <= priorIdentityOccurrence) {
      fail('E_SEMANTIC_ORDER', `${path}.identityOccurrence`, 'root outputs must follow language occurrence/output-slot order')
    }
    priorIdentityOccurrence = occurrence
    if (outputOccurrences.has(occurrence)) fail('E_SEMANTIC_IDENTITY', `${path}.identityOccurrence`, 'one occurrence cannot own multiple result entries')
    outputOccurrences.add(occurrence)
  }
  if (value.tag === 'empty') {
    exactKeys(value, ['tag', 'type'], '$.core.result')
    if (value.type !== 'never') fail('E_SEMANTIC_TYPE', '$.core.result.type', 'empty result has bottom type never')
  } else if (value.tag === 'single') {
    exactKeys(value, ['tag', 'item'], '$.core.result')
    const node = validateOutputRef(
      value.item, '$.core.result.item', nodes, occurrences, canonicalOccurrenceRows,
    )
    rootNodes.push(node)
    recordIdentityOccurrence(value.item as Record<string, unknown>, '$.core.result.item')
  } else if (value.tag === 'multi') {
    exactKeys(value, ['tag', 'items'], '$.core.result')
    const items = arrayValue(value.items, '$.core.result.items', SEMANTIC_PROGRAM_LIMITS.outputs)
    if (items.length < 2) fail('E_SEMANTIC_TYPE', '$.core.result.items', 'multi requires at least two ordered outputs')
    items.forEach((item, index) => {
      rootNodes.push(validateOutputRef(
        item,
        `$.core.result.items[${index}]`,
        nodes,
        occurrences,
        canonicalOccurrenceRows,
      ))
      recordIdentityOccurrence(item as Record<string, unknown>, `$.core.result.items[${index}]`)
    })
  } else {
    fail('E_SEMANTIC_FIELD', '$.core.result.tag', 'unknown result tag')
  }

  return rootNodes
}
