import { SEMANTIC_PROGRAM_EXECUTION_VERSION, SEMANTIC_PROGRAM_LIMITS, semanticNodeInputs, semanticNodeProducerOperationNames } from '../../core/semanticProgram'
import type { SemanticDiagnosticTemplate, SemanticNode, SemanticOccurrence, SemanticStaticOperation } from '../../core/semanticProgram'
import type { SemanticProductionRule, ValidatedSemanticExecution } from '../semanticProgramValidator'
import { arrayValue, exactKeys, fail, integer, isRecord } from './fail'
import { occurrenceIsDescendantOf, validateOutputRef } from './isChildrenExpansionContinuation'

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

export function reachableNodes(nodes: readonly SemanticNode[], roots: readonly number[]): Uint8Array {
  const reached = new Uint8Array(nodes.length)
  const stack = [...roots]
  while (stack.length > 0) {
    const node = stack.pop()!
    if (reached[node]) continue
    reached[node] = 1
    stack.push(...semanticNodeInputs(nodes[node]))
  }
  return reached
}

export function validateExecution(
  value: unknown,
  nodes: readonly SemanticNode[],
  occurrences: readonly SemanticOccurrence[],
  operations: readonly SemanticStaticOperation[],
  outputRoots: readonly number[],
  templates: readonly SemanticDiagnosticTemplate[],
  languageContract: 'legacy/current' | 'openscad-viewer/brep-1',
): ValidatedSemanticExecution {
  exactKeys(value, ['version', 'evaluationOrder', 'discardedEffects', 'terminal'], '$.core.execution')
  if (value.version !== SEMANTIC_PROGRAM_EXECUTION_VERSION) {
    fail('E_SEMANTIC_VERSION', '$.core.execution.version', 'unknown semantic execution contract')
  }
  const order = arrayValue(value.evaluationOrder, '$.core.execution.evaluationOrder', SEMANTIC_PROGRAM_LIMITS.nodes)
  if (order.length !== nodes.length) {
    fail('E_SEMANTIC_DAG', '$.core.execution.evaluationOrder', 'evaluation order must contain every node exactly once')
  }
  const position = new Int32Array(nodes.length)
  position.fill(-1)
  order.forEach((rawNode, index) => {
    const node = integer(rawNode, `$.core.execution.evaluationOrder[${index}]`, 0, Math.max(0, nodes.length - 1))
    if (nodes.length === 0) fail('E_SEMANTIC_REFERENCE', `$.core.execution.evaluationOrder[${index}]`, 'evaluation order references absent node')
    if (position[node] !== -1) fail('E_SEMANTIC_ORDER', `$.core.execution.evaluationOrder[${index}]`, 'evaluation order contains a duplicate node')
    for (const input of semanticNodeInputs(nodes[node])) {
      if (position[input] === -1) fail('E_SEMANTIC_DAG', `$.core.execution.evaluationOrder[${index}]`, 'a node is scheduled before one of its inputs')
    }
    position[node] = index
    if (node !== index) {
      fail('E_SEMANTIC_ORDER', `$.core.execution.evaluationOrder[${index}]`, 'v1.2 node storage order is the exact authored kernel evaluation order')
    }
  })

  const firstRowByOccurrenceId = new Map<string, number>()
  occurrences.forEach((occurrence, index) => {
    if (!firstRowByOccurrenceId.has(occurrence.occurrenceId)) firstRowByOccurrenceId.set(occurrence.occurrenceId, index)
  })
  const effects = arrayValue(
    value.discardedEffects,
    '$.core.execution.discardedEffects',
    SEMANTIC_PROGRAM_LIMITS.discardedEffects,
  )
  let priorEffectPosition = -1
  const effectRoots = new Set<number>()
  const validatedEffects = effects.map((effect, index) => {
    const path = `$.core.execution.discardedEffects[${index}]`
    exactKeys(effect, ['tag', 'root', 'ownerOccurrence'], path)
    if (effect.tag !== 'legacy-difference-cutters') fail('E_SEMANTIC_FIELD', `${path}.tag`, 'unknown discarded effect kind')
    if (languageContract !== 'legacy/current') fail('E_SEMANTIC_TYPE', path, 'discarded legacy effects are forbidden for brep-1')
    const root = integer(effect.root, `${path}.root`, 0, Math.max(0, nodes.length - 1))
    if (nodes.length === 0 || position[root] < 0) fail('E_SEMANTIC_REFERENCE', `${path}.root`, 'effect root is absent from evaluation order')
    if (effectRoots.has(root)) fail('E_SEMANTIC_ORDER', `${path}.root`, 'effect roots must be unique')
    if (position[root] <= priorEffectPosition) fail('E_SEMANTIC_ORDER', `${path}.root`, 'effects must follow kernel evaluation order')
    effectRoots.add(root)
    priorEffectPosition = position[root]
    const ownerOccurrence = integer(effect.ownerOccurrence, `${path}.ownerOccurrence`, 0, Math.max(0, occurrences.length - 1))
    if (occurrences.length === 0) fail('E_SEMANTIC_REFERENCE', `${path}.ownerOccurrence`, 'effect owner occurrence is absent')
    const owner = occurrences[ownerOccurrence]
    if (firstRowByOccurrenceId.get(owner.occurrenceId) !== ownerOccurrence) {
      fail('E_SEMANTIC_ORDER', `${path}.ownerOccurrence`, 'effect owner must use the canonical occurrence row')
    }
    return { tag: effect.tag, root, ownerOccurrence } as const
  })

  let terminal: ValidatedSemanticExecution['terminal'] = null
  if (value.terminal !== null) {
    const path = '$.core.execution.terminal'
    exactKeys(value.terminal, ['tag', 'occurrence', 'diagnosticTemplate', 'prefixFrontier'], path)
    if (value.terminal.tag !== 'legacy-language-error') fail('E_SEMANTIC_FIELD', `${path}.tag`, 'unknown semantic terminal')
    if (languageContract !== 'legacy/current') fail('E_SEMANTIC_TYPE', path, 'legacy terminal traces are forbidden for brep-1')
    if (outputRoots.length !== 0) fail('E_SEMANTIC_TYPE', '$.core.result', 'a terminal plan cannot publish geometry outputs')
    const diagnosticTemplate = integer(
      value.terminal.diagnosticTemplate,
      `${path}.diagnosticTemplate`,
      0,
      Math.max(0, templates.length - 1),
    )
    if (templates.length === 0 || templates[diagnosticTemplate].severity !== 'error') {
      fail('E_SEMANTIC_REFERENCE', `${path}.diagnosticTemplate`, 'terminal must reference an error diagnostic template')
    }
    const occurrence = integer(
      value.terminal.occurrence,
      `${path}.occurrence`,
      0,
      Math.max(0, occurrences.length - 1),
    )
    if (occurrences.length === 0 || firstRowByOccurrenceId.get(occurrences[occurrence].occurrenceId) !== occurrence) {
      fail('E_SEMANTIC_ORDER', `${path}.occurrence`, 'terminal must reference the canonical interrupted occurrence row')
    }
    const interruptedId = occurrences[occurrence].occurrenceId
    // A parent operation may fail after all of its children (and even after
    // materializing earlier map outputs), so "terminal is the final row" is
    // false. What the source-free graph can prove is that the claimed frame
    // remained active: every subsequently created row belongs to that logical
    // activation or its runtime subtree. Exact deepest-error provenance is a
    // trusted-lowerer property; structural SPE/SPC graphs are never executable.
    for (let index = occurrence + 1; index < occurrences.length; index++) {
      if (occurrences[index].occurrenceId === interruptedId
        || occurrenceIsDescendantOf(index, occurrence, occurrences)) continue
      fail(
        'E_SEMANTIC_ORDER',
        `${path}.occurrence`,
        'no later sibling activation may exist outside the interrupted terminal subtree',
      )
    }
    const template = templates[diagnosticTemplate]
    if (template.operation !== occurrences[occurrence].operation) {
      fail('E_SEMANTIC_IDENTITY', `${path}.diagnosticTemplate`, 'terminal template operation must equal the interrupted occurrence operation')
    }
    if (template.code !== 'LEGACY_LANGUAGE_ERROR'
      || template.arguments.length !== 2
      || template.arguments[0].name !== 'errorName'
      || template.arguments[0].value.tag !== 'string'
      || !['OpenSCADParseError', 'TypeError'].includes(template.arguments[0].value.value)
      || template.arguments[1].name !== 'detailSha256'
      || template.arguments[1].value.tag !== 'string'
      || !/^[a-f0-9]{64}$/.test(template.arguments[1].value.value)) {
      fail('E_SEMANTIC_IDENTITY', `${path}.diagnosticTemplate`, 'terminal requires the exact frozen legacy error template shape')
    }
    if (validatedEffects.length !== 0) {
      fail('E_SEMANTIC_FIELD', '$.core.execution.discardedEffects', 'terminal prefixes subsume earlier discarded work; separate effect rows are forbidden')
    }
    const frontier = arrayValue(value.terminal.prefixFrontier, `${path}.prefixFrontier`, SEMANTIC_PROGRAM_LIMITS.nodes)
    const frontierRoots = new Set<number>()
    const validatedFrontier = frontier.map((entry, index) => {
      const entryPath = `${path}.prefixFrontier[${index}]`
      exactKeys(entry, ['root', 'ownerOccurrence'], entryPath)
      const root = integer(entry.root, `${entryPath}.root`, 0, Math.max(0, nodes.length - 1))
      if (nodes.length === 0) fail('E_SEMANTIC_REFERENCE', `${entryPath}.root`, 'terminal prefix references absent node')
      if (frontierRoots.has(root)) fail('E_SEMANTIC_ORDER', `${entryPath}.root`, 'terminal prefix roots must be unique')
      frontierRoots.add(root)
      let ownerOccurrence: number | null = null
      if (entry.ownerOccurrence !== null) {
        ownerOccurrence = integer(entry.ownerOccurrence, `${entryPath}.ownerOccurrence`, 0, Math.max(0, occurrences.length - 1))
        if (occurrences.length === 0 || occurrences[ownerOccurrence].node !== root) {
          fail('E_SEMANTIC_IDENTITY', `${entryPath}.ownerOccurrence`, 'terminal prefix owner must materialize its root')
        }
      }
      const admittedProducers = new Set<number>()
      occurrences.forEach((candidate, candidateIndex) => {
        if (candidate.node !== root || candidate.outputOrdinal === null) return
        if (!semanticNodeProducerOperationNames(nodes[root]).includes(
          operations[candidate.operation].name,
        )) return
        const canonical = firstRowByOccurrenceId.get(candidate.occurrenceId)
        if (canonical !== undefined) admittedProducers.add(canonical)
      })
      if (admittedProducers.size === 1) {
        const exactOwner = [...admittedProducers][0]
        if (ownerOccurrence !== exactOwner) {
          fail('E_SEMANTIC_IDENTITY', `${entryPath}.ownerOccurrence`, 'terminal prefix root must name its exact materializer occurrence')
        }
      } else if (admittedProducers.size === 0
        && nodes[root].kind === 'boolean' && nodes[root].operation === 'union') {
        if (ownerOccurrence !== null) {
          fail('E_SEMANTIC_IDENTITY', `${entryPath}.ownerOccurrence`, 'internal union reducer must have a null terminal prefix owner')
        }
      } else {
        fail('E_SEMANTIC_IDENTITY', `${entryPath}.ownerOccurrence`, 'terminal prefix root has ambiguous or absent materializer evidence')
      }
      return { root, ownerOccurrence }
    })
    const consumed = new Uint8Array(nodes.length)
    nodes.forEach(node => semanticNodeInputs(node).forEach(input => { consumed[input] = 1 }))
    const maximal = order.filter(node => consumed[node as number] === 0) as number[]
    if (maximal.length !== validatedFrontier.length
      || maximal.some((root, index) => root !== validatedFrontier[index].root)) {
      fail('E_SEMANTIC_DAG', `${path}.prefixFrontier`, 'terminal prefix must list every maximal completed node in evaluation order')
    }
    terminal = { tag: value.terminal.tag, occurrence, diagnosticTemplate, prefixFrontier: validatedFrontier }
  } else if (templates.some(template => template.severity === 'error')) {
    fail('E_SEMANTIC_REFERENCE', '$.core.execution.terminal', 'error templates require the matching terminal plan')
  }

  const outputReachable = reachableNodes(nodes, outputRoots)
  for (const root of effectRoots) {
    if (outputReachable[root]) fail('E_SEMANTIC_DAG', '$.core.execution.discardedEffects', 'a discarded effect cannot be reachable from a published output')
  }
  const roots = terminal === null
    ? [...outputRoots, ...effectRoots]
    : terminal.prefixFrontier.map(entry => entry.root)
  const reached = reachableNodes(nodes, roots)
  if (reached.some(value => value === 0)) {
    fail('E_SEMANTIC_DAG', '$.core.nodes', 'every node must be reachable from a result, effect, or terminal prefix root')
  }
  return { evaluationOrder: order as number[], discardedEffects: validatedEffects, terminal }
}

export function semanticProductionRule(
  operation: SemanticStaticOperation,
  allowInterruptedUnsupported = false,
): SemanticProductionRule {
  const pair = `${operation.category}:${operation.name}`
  const last = operation.structuralPath[operation.structuralPath.length - 1]
  const prior = operation.structuralPath[operation.structuralPath.length - 2]
  const authoredCall = last.kind === 'call'
  if (operation.category === 'geometry' && authoredCall && ['cube', 'sphere', 'cylinder', 'polyhedron', 'square', 'circle', 'polygon'].includes(operation.name)) return 'primitive'
  if (operation.category === 'transform' && authoredCall && ['translate', 'rotate', 'scale', 'mirror', 'multmatrix'].includes(operation.name)) return 'transform-map'
  if (pair === 'geometry:projection' && authoredCall) return 'projection-map'
  if (pair === 'geometry:offset' && authoredCall) return 'offset-map'
  if ((pair === 'presentation:color' || pair === 'assertion:assert') && authoredCall) return 'preserving-alias'
  if (operation.category === 'control') {
    if (operation.name === '$assign' && last.kind === 'control') return 'transparent'
    if (operation.name === '$body' && operation.parent !== null
      && last.kind === 'body' && last.ordinal === 0 && prior !== undefined
      && ['call', 'module', 'control'].includes(prior.kind)) return 'transparent'
    if ((operation.name === '$then' || operation.name === '$else')
      && operation.parent !== null && last.kind === 'branch' && last.ordinal === 0 && prior?.name === 'if' && prior.kind === 'control') return 'transparent'
    if (operation.name === '$expansion' && operation.parent !== null && last.kind === 'control'
      && last.ordinal === 0
      && (prior?.name === '$body' || (prior?.name === 'children' && prior.kind === 'control'))) {
      return 'transparent'
    }
    if (!operation.name.startsWith('$') && last.kind === 'control'
      && ['if', 'let', 'for', 'children', 'group', 'render'].includes(operation.name)) return 'transparent'
  }
  if (operation.category === 'module' && !operation.name.startsWith('$')
    && (last.kind === 'module' || last.kind === 'call')) return 'transparent'
  if ((pair === 'boolean:union' || pair === 'boolean:intersection') && authoredCall) return 'boolean'
  if (pair === 'boolean:hull' && authoredCall) return 'hull'
  if (pair === 'boolean:difference' && authoredCall) return 'difference'
  if (pair === 'geometry:linear_extrude' && authoredCall) return 'linear-extrude'
  if (pair === 'geometry:rotate_extrude' && authoredCall) return 'rotate-extrude'
  if (allowInterruptedUnsupported) return 'terminal-unsupported'
  fail('E_SEMANTIC_IDENTITY', `$.core.operations[${operation.id}]`, `operation pair ${pair} has no frozen v1 production rule`)
}

export function equalNumberList(left: readonly number[], right: readonly number[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index])
}
