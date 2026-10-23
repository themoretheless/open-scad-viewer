import {
  deriveSemanticCapabilityClosure,
  SEMANTIC_PROGRAM_CAPABILITY_GRAPH,
  SEMANTIC_PROGRAM_DIAGNOSTIC_CODES,
  SEMANTIC_PROGRAM_EXECUTION_FEATURE,
  SEMANTIC_PROGRAM_EXECUTION_VERSION,
  SEMANTIC_PROGRAM_IDENTITY,
  SEMANTIC_PROGRAM_LIMITS,
  semanticNodeInputs,
  semanticNodeProducerOperationNames,
  semanticResultItems,
  type SemanticDiagnosticTemplate,
  type SemanticNode,
  type SemanticOccurrence,
  type SemanticProgramCoreV1,
  type SemanticProgramDiagnosticCode,
  type SemanticProgramEnvelopeV1,
  type SemanticProgramV1,
  type SemanticSourceDescriptor,
  type SemanticSourceSpan,
  type SemanticStaticOperation,
} from '../core/semanticProgram'
import { sha256Hex } from '../core/sha256'
import { parseGeometrySourceRoutingHeader } from '../core/geometryRouting'
import { isSemanticProgramCodecOwnedGraph } from './semanticProgramCodec'
import {
  arrayValue,
  assertSchemaVersion,
  exactKeys,
  fail,
  finiteNumber,
  integer,
  isRecord,
  nullablePositive,
  SemanticProgramValidationError,
  stringValue,
  validateDiagnosticTemplates,
  validateNodes,
  validateOperations,
  validateSortedStrings,
  wellFormedUnicode,
} from '../semantic/schemaPrimitives'
import {
  occurrenceIsDescendantOf,
  validateOccurrences,
  validateResult,
} from '../semantic/occurrenceValidator'

export { SemanticProgramValidationError }

interface ValidatedSemanticExecution {
  readonly evaluationOrder: readonly number[]
  readonly discardedEffects: readonly Readonly<{
    tag: 'legacy-difference-cutters'
    root: number
    ownerOccurrence: number
  }>[]
  readonly terminal: null | Readonly<{
    tag: 'legacy-language-error'
    occurrence: number
    diagnosticTemplate: number
    prefixFrontier: readonly Readonly<{ root: number; ownerOccurrence: number | null }>[]
  }>
}

function reachableNodes(nodes: readonly SemanticNode[], roots: readonly number[]): Uint8Array {
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

function validateExecution(
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
      occurrences.forEach(candidate => {
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

type SemanticProductionRule =
  | 'primitive' | 'transform-map' | 'projection-map' | 'offset-map'
  | 'preserving-alias' | 'transparent' | 'boolean' | 'hull' | 'difference'
  | 'linear-extrude' | 'rotate-extrude' | 'terminal-unsupported'

function semanticProductionRule(
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

interface SemanticProductionItem {
  readonly node: number
  readonly row: number
  readonly identityOwner: number
  readonly group: number
  readonly directChild: number
}

type SemanticScheduleEvent =
  | { readonly tag: 'group'; readonly group: number }
  | { readonly tag: 'node'; readonly node: number }

interface SemanticProductionGroup {
  readonly canonical: number
  readonly outputRows: readonly number[]
  readonly outputItems: readonly SemanticProductionItem[]
  readonly frontier: readonly SemanticProductionItem[]
  readonly terminalItems: readonly SemanticProductionItem[]
  readonly schedule: readonly SemanticScheduleEvent[]
  readonly transparent: boolean
}

function equalNumberList(left: readonly number[], right: readonly number[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index])
}

function validateOccurrenceProduction(
  operations: readonly SemanticStaticOperation[],
  occurrences: readonly SemanticOccurrence[],
  nodes: readonly SemanticNode[],
  result: SemanticProgramCoreV1['result'],
  discardedEffects: ValidatedSemanticExecution['discardedEffects'] = [],
  terminal: ValidatedSemanticExecution['terminal'] = null,
): void {
  const terminalPrefix = terminal?.prefixFrontier ?? null
  const indexesById = new Map<string, number[]>()
  occurrences.forEach((occurrence, index) => {
    const indexes = indexesById.get(occurrence.occurrenceId) ?? []
    indexes.push(index)
    indexesById.set(occurrence.occurrenceId, indexes)
  })
  const canonicalByRow = new Int32Array(occurrences.length)
  const canonicalGroups: number[] = []
  const rowsByCanonical = new Map<number, readonly number[]>()
  for (const indexes of indexesById.values()) {
    const canonical = indexes[0]
    canonicalGroups.push(canonical)
    rowsByCanonical.set(canonical, indexes)
    indexes.forEach(index => { canonicalByRow[index] = canonical })
    const rows = indexes.map(index => occurrences[index])
    const zero = rows.length === 1 && rows[0].node === null
      && rows[0].outputOrdinal === null && rows[0].sceneEntityId === null
    const output = rows.every((row, ordinal) => row.node !== null
      && row.outputOrdinal === ordinal && row.sceneEntityId !== null)
    if (!zero && !output) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'a logical occurrence must be exactly one zero/frame row or dense output rows')
    }
  }
  canonicalGroups.sort((left, right) => left - right)
  const activeGroups = new Set<number>()
  let interruptedGroup: number | null = null
  if (terminal !== null) {
    let cursor: number | null = terminal.occurrence
    interruptedGroup = canonicalByRow[cursor]
    while (cursor !== null) {
      const canonical: number = canonicalByRow[cursor]
      activeGroups.add(canonical)
      cursor = occurrences[canonical].parent
    }
  }
  const effectByOwner = new Map<number, ValidatedSemanticExecution['discardedEffects'][number]>()
  for (const effect of discardedEffects) {
    const owner = canonicalByRow[effect.ownerOccurrence]
    if (effectByOwner.has(owner)) {
      fail('E_SEMANTIC_IDENTITY', '$.core.execution.discardedEffects', 'one difference occurrence can own only one discarded cutter effect')
    }
    effectByOwner.set(owner, effect)
  }
  const consumedEffectOwners = new Set<number>()
  const terminalEffectRoots = new Set<number>()
  const children = new Map<number, number[]>()
  for (const canonical of canonicalGroups) {
    const parent = occurrences[canonical].parent
    if (parent === null) continue
    const parentCanonical = canonicalByRow[parent]
    const list = children.get(parentCanonical) ?? []
    list.push(canonical)
    children.set(parentCanonical, list)
  }

  const ownerByNode = new Map<number, number>()
  const productionByCanonical = new Map<number, SemanticProductionGroup>()
  let expandedFrontierItems = 0
  const appendFrontier = (
    target: SemanticProductionItem[],
    source: readonly SemanticProductionItem[],
    directChild?: number,
  ): void => {
    expandedFrontierItems += source.length
    if (expandedFrontierItems > SEMANTIC_PROGRAM_LIMITS.snapshotValues) {
      fail('E_SEMANTIC_LIMIT', '$.core.occurrences', 'expanded occurrence frontier exceeds its bounded proof budget')
    }
    for (const item of source) {
      target.push(directChild === undefined ? item : { ...item, directChild })
    }
  }
  const claimNode = (node: number, canonical: number) => {
    const existing = ownerByNode.get(node)
    if (existing !== undefined) {
      fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${node}]`, 'each DAG node must have exactly one recomputed materializer group')
    }
    ownerByNode.set(node, canonical)
  }
  const requireOutputCount = (canonical: number, rows: readonly number[], expected: number) => {
    if (rows.length !== expected) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, `production rule requires exactly ${expected} output row(s)`)
    }
  }
  const requireNodeKind = (row: number, expected: readonly SemanticNode['kind'][]): SemanticNode => {
    const reference = occurrences[row].node
    if (reference === null || !expected.includes(nodes[reference].kind)) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${row}].node`, `operation cannot materialize node kind ${reference === null ? 'null' : nodes[reference].kind}`)
    }
    return nodes[reference]
  }

  const evaluate = (canonical: number): SemanticProductionGroup => {
    const cached = productionByCanonical.get(canonical)
    if (cached) return cached
    const groupRows = rowsByCanonical.get(canonical)!
    const outputRows = groupRows.filter(row => occurrences[row].node !== null)
    const operation = operations[occurrences[canonical].operation]
    const active = activeGroups.has(canonical)
    const rule = semanticProductionRule(operation, canonical === interruptedGroup)
    const isCompilerFrontierFrame = operation.category === 'control'
      && ['$body', '$then', '$else', '$expansion'].includes(operation.name)
    const frontier: SemanticProductionItem[] = []
    let hasBody = false
    const schedule: SemanticScheduleEvent[] = []
    for (const child of children.get(canonical) ?? []) {
      const childProduction = productionByCanonical.get(child)
      if (!childProduction) fail('E_SEMANTIC_ORDER', `$.core.occurrences[${child}]`, 'runtime occurrence groups must follow parent-before-child order')
      const childEvent: SemanticScheduleEvent = { tag: 'group', group: child }
      schedule.push(childEvent)
      const visible = childProduction.terminalItems
      if (isCompilerFrontierFrame) {
        const bucket: SemanticProductionItem[] = []
        appendFrontier(bucket, visible, child)
        appendFrontier(frontier, bucket)
      } else {
        appendFrontier(frontier, visible)
      }
      const childOperation = operations[occurrences[child].operation]
      if (childOperation.category === 'control' && childOperation.name === '$body') {
        if (hasBody) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'operation has more than one compiler-owned body frontier')
        }
        hasBody = true
      }
    }
    const frontierNodes = new Set<number>()
    for (const item of frontier) {
      if (frontierNodes.has(item.node)) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'cross-branch or duplicate frontier node reuse is forbidden in v1')
      }
      frontierNodes.add(item.node)
    }
    const outputItems: SemanticProductionItem[] = []
    let terminalItems: SemanticProductionItem[] | null = null
    const scheduleNode = (node: number) => {
      claimNode(node, canonical)
      schedule.push({ tag: 'node', node })
    }
    const own = (row: number, node: number) => {
      scheduleNode(node)
      outputItems.push({ node, row, identityOwner: row, group: canonical, directChild: canonical })
    }
    const preserve = (row: number, item: SemanticProductionItem) => {
      outputItems.push({ node: item.node, row, identityOwner: item.identityOwner, group: canonical, directChild: canonical })
    }
    if (active && rule === 'difference' && outputRows.length === 0) {
      const directGroups = children.get(canonical) ?? []
      if ((hasBody && directGroups.length !== 1) || (!hasBody && directGroups.length !== 0)) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'interrupted difference has an invalid compiler-owned body')
      }
      const bodyGroup = hasBody ? directGroups[0] : null
      const bodyOperation = bodyGroup === null ? null : occurrences[bodyGroup].operation
      const authoredBucketOperations = bodyOperation === null
        ? []
        : operations.filter(candidate => candidate.parent === bodyOperation).map(candidate => candidate.id)
      const runtimeBucketRoots = bodyGroup === null ? [] : children.get(bodyGroup) ?? []
      const authoredBucketSet = new Set(authoredBucketOperations)
      if (runtimeBucketRoots.some(child => !authoredBucketSet.has(occurrences[child].operation))) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'active difference runtime activation is not owned by an authored body statement')
      }
      const activeBuckets = authoredBucketOperations.map(bucketOperation => {
        const bucket: SemanticProductionItem[] = []
        for (const child of runtimeBucketRoots) {
          if (occurrences[child].operation !== bucketOperation) continue
          appendFrontier(bucket, productionByCanonical.get(child)!.terminalItems, child)
        }
        return bucket
      })
      const activeBucketSchedules = authoredBucketOperations.map(bucketOperation => (
        runtimeBucketRoots
          .filter(child => occurrences[child].operation === bucketOperation)
          .map(child => ({ tag: 'group', group: child }) as const)
      ))
      const reconstructedFrontier: SemanticProductionItem[] = []
      for (const bucket of activeBuckets) appendFrontier(reconstructedFrontier, bucket)
      if (reconstructedFrontier.length !== frontier.length
        || reconstructedFrontier.some((item, index) => {
          const expected = frontier[index]
          return item.node !== expected.node || item.row !== expected.row
            || item.identityOwner !== expected.identityOwner
            || item.directChild !== expected.directChild
        })) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'authored active difference buckets must exactly partition its runtime frontier')
      }
      const rawBase = activeBuckets[0] ?? []
      const rawCutters: SemanticProductionItem[] = []
      for (let bucket = 1; bucket < activeBuckets.length; bucket++) appendFrontier(rawCutters, activeBuckets[bucket])
      const firstStaticStatement = authoredBucketOperations[0]
      const baseRuntimeStatements = firstStaticStatement === undefined
        ? []
        : runtimeBucketRoots.filter(child => occurrences[child].operation === firstStaticStatement)
      const baseActivationIsPresent = baseRuntimeStatements.length > 0
      const base = baseActivationIsPresent ? rawBase : []
      const cutters = baseActivationIsPresent ? rawCutters : frontier
      const baseCompleted = baseActivationIsPresent
        && baseRuntimeStatements.every(statement => !activeGroups.has(statement))
      const cutterStatements = runtimeBucketRoots.filter(statement => (
        firstStaticStatement === undefined || occurrences[statement].operation !== firstStaticStatement
      ))
      const cuttersCompleted = canonical === interruptedGroup
        && cutterStatements.every(statement => !activeGroups.has(statement))
      schedule.length = 0
      if (baseActivationIsPresent && activeBucketSchedules[0] !== undefined) {
        schedule.push(...activeBucketSchedules[0])
      }
      const completeReduction = (
        items: readonly SemanticProductionItem[],
      ): SemanticProductionItem[] => {
        if (items.length < 2) return [...items]
        const space = nodes[items[0].node].valueType.space
        if (items.some(item => nodes[item.node].valueType.space !== space)) return [...items]
        const candidates = nodes.filter(candidate => !ownerByNode.has(candidate.id)
          && candidate.kind === 'boolean' && candidate.operation === 'union'
          && equalNumberList(candidate.inputs, items.map(item => item.node)))
        if (candidates.length !== 1) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'completed interrupted partition requires one exact union reducer')
        }
        scheduleNode(candidates[0].id)
        return [{
          node: candidates[0].id,
          row: items[0].row,
          identityOwner: items[0].identityOwner,
          group: canonical,
          directChild: canonical,
        }]
      }
      const reducedBase = baseCompleted ? completeReduction(base) : [...base]
      const firstCutterBucket = baseActivationIsPresent ? 1 : 0
      for (let bucket = firstCutterBucket; bucket < activeBucketSchedules.length; bucket++) {
        schedule.push(...activeBucketSchedules[bucket])
      }
      const reducedCutters = cuttersCompleted ? completeReduction(cutters) : [...cutters]
      terminalItems = [...reducedBase, ...reducedCutters]
    } else if (active && (rule === 'transform-map' || rule === 'projection-map' || rule === 'offset-map')) {
      if (outputRows.length > frontier.length) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'partial map emits beyond its completed frontier')
      }
      const commonParameters = new Map<string, string>()
      outputRows.forEach((row, index) => {
        const expectedKind = rule === 'transform-map' ? ['transform'] as const
          : rule === 'projection-map' ? ['projection'] as const : ['offset'] as const
        const produced = requireNodeKind(row, expectedKind)
        if (!equalNumberList(semanticNodeInputs(produced), [frontier[index].node])) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${row}].node`, 'partial map output must consume the matching ordered frontier item')
        }
        const parameters = produced.kind === 'transform' ? JSON.stringify(produced.matrix)
          : produced.kind === 'projection' ? String(produced.cut)
            : produced.kind === 'offset' ? String(produced.distance) : ''
        const parameterBucket = rule === 'transform-map' && operation.name === 'rotate'
          ? nodes[frontier[index].node].valueType.space
          : 'all'
        const priorParameters = commonParameters.get(parameterBucket)
        if (priorParameters !== undefined && parameters !== priorParameters) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${row}].node`, 'partial map outputs must use field-equal common parameters')
        }
        commonParameters.set(parameterBucket, parameters)
        scheduleNode(produced.id)
        outputItems.push({
          node: produced.id,
          row,
          identityOwner: rule === 'transform-map' ? frontier[index].identityOwner : row,
          group: canonical,
          directChild: canonical,
        })
      })
      terminalItems = [...outputItems]
      appendFrontier(terminalItems, frontier.slice(outputRows.length))
    } else if (active && outputRows.length === 0) {
      // Only the active call chain may have an absent result. Completed
      // siblings still pass their full closed production rules below.
      terminalItems = [...frontier]
    } else if (rule === 'transparent') {
      requireOutputCount(canonical, outputRows, 0)
    } else if (rule === 'primitive') {
      if (frontier.length !== 0) fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'primitive production frontier must be empty')
      requireOutputCount(canonical, outputRows, 1)
      const expectedByName: Readonly<Record<string, readonly SemanticNode['kind'][]>> = {
        cube: ['box'], sphere: ['sphere-analytic', 'sphere-polygonal'],
        cylinder: ['cylinder-analytic', 'cylinder-polygonal'], polyhedron: ['polyhedron'],
        square: ['rectangle'], circle: ['circle-analytic', 'circle-polygonal'], polygon: ['polygon'],
      }
      const produced = requireNodeKind(outputRows[0], expectedByName[operation.name])
      own(outputRows[0], produced.id)
    } else if (rule === 'transform-map' || rule === 'projection-map' || rule === 'offset-map') {
      requireOutputCount(canonical, outputRows, frontier.length)
      const commonParameters = new Map<string, string>()
      outputRows.forEach((row, index) => {
        const expectedKind = rule === 'transform-map' ? ['transform'] as const
          : rule === 'projection-map' ? ['projection'] as const : ['offset'] as const
        const produced = requireNodeKind(row, expectedKind)
        const input = semanticNodeInputs(produced)
        if (input.length !== 1 || input[0] !== frontier[index].node) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${row}].node`, 'map output must consume its matching ordered frontier item')
        }
        let parameters: string
        if (produced.kind === 'transform') parameters = JSON.stringify(produced.matrix)
        else if (produced.kind === 'projection') parameters = String(produced.cut)
        else if (produced.kind === 'offset') parameters = String(produced.distance)
        else fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}]`, 'unexpected map node kind')
        const parameterBucket = rule === 'transform-map' && operation.name === 'rotate'
          ? nodes[frontier[index].node].valueType.space
          : 'all'
        const priorParameters = commonParameters.get(parameterBucket)
        if (priorParameters !== undefined && parameters !== priorParameters) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${row}].node`, 'one map occurrence must use field-equal common parameters')
        }
        commonParameters.set(parameterBucket, parameters)
        if (rule === 'transform-map') {
          scheduleNode(produced.id)
          outputItems.push({
            node: produced.id,
            row,
            identityOwner: frontier[index].identityOwner,
            group: canonical,
            directChild: canonical,
          })
        }
        else own(row, produced.id)
      })
    } else if (rule === 'preserving-alias') {
      requireOutputCount(canonical, outputRows, frontier.length)
      outputRows.forEach((row, index) => {
        if (occurrences[row].node !== frontier[index].node) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${row}].node`, 'alias output must equal its matching ordered frontier node')
        }
        preserve(row, frontier[index])
      })
    } else if (rule === 'boolean' || rule === 'hull') {
      if (frontier.length === 0) {
        requireOutputCount(canonical, outputRows, 0)
      } else if (frontier.length === 1) {
        requireOutputCount(canonical, outputRows, 1)
        if (occurrences[outputRows[0]].node !== frontier[0].node) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${outputRows[0]}].node`, 'one-item reduction must alias its sole frontier node')
        }
        if (rule === 'hull') outputItems.push({ node: frontier[0].node, row: outputRows[0], identityOwner: outputRows[0], group: canonical, directChild: canonical })
        else preserve(outputRows[0], frontier[0])
      } else {
        requireOutputCount(canonical, outputRows, 1)
        const expectedKind = rule === 'hull' ? ['hull'] as const : ['boolean'] as const
        const produced = requireNodeKind(outputRows[0], expectedKind)
        if (produced.kind === 'boolean') {
          if (produced.operation !== operation.name) {
            fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}].operation`, 'Boolean node operation does not match its static operation')
          }
        }
        if (!equalNumberList(semanticNodeInputs(produced), frontier.map(item => item.node))) {
          fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}]`, 'n-ary reduction inputs must exactly equal the ordered occurrence frontier')
        }
        own(outputRows[0], produced.id)
      }
    } else if (rule === 'difference') {
      const directChildren = children.get(canonical) ?? []
      const bodies = directChildren.filter(child => {
        const childOperation = operations[occurrences[child].operation]
        const last = childOperation.structuralPath.at(-1)
        return childOperation.parent === occurrences[canonical].operation
          && childOperation.category === 'control'
          && childOperation.name === '$body'
          && last?.kind === 'body'
          && last.name === '$body'
      })
      if (bodies.length > 1 || (bodies.length === 1 && directChildren.length !== 1)
        || (bodies.length === 0 && directChildren.length !== 0)) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'difference runtime frontier requires at most one exact compiler $body child and no direct authored children')
      }
      const body = bodies[0]
      const bodyOperation = body === undefined ? null : occurrences[body].operation
      const authoredBucketOperations = bodyOperation === null
        ? []
        : operations.filter(candidate => candidate.parent === bodyOperation).map(candidate => candidate.id)
      const runtimeBucketRoots = body === undefined ? [] : children.get(body) ?? []
      const authoredBucketSet = new Set(authoredBucketOperations)
      if (runtimeBucketRoots.some(child => !authoredBucketSet.has(occurrences[child].operation))) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'difference runtime activation is not owned by a direct authored body statement')
      }
      const buckets = authoredBucketOperations.map(bucketOperation => {
        const bucket: SemanticProductionItem[] = []
        for (const child of runtimeBucketRoots) {
          if (occurrences[child].operation !== bucketOperation) continue
          const childProduction = productionByCanonical.get(child)!
          const visible = childProduction.outputItems.length > 0
            ? childProduction.outputItems
            : childProduction.transparent ? childProduction.frontier : []
          appendFrontier(bucket, visible, child)
        }
        return bucket
      })
      const bucketFrontier: SemanticProductionItem[] = []
      for (const bucket of buckets) appendFrontier(bucketFrontier, bucket)
      if (!equalNumberList(bucketFrontier.map(item => item.row), frontier.map(item => item.row))) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'difference frontier buckets must exactly partition its ordered frontier')
      }
      const base = buckets[0] ?? []
      const cutters: SemanticProductionItem[] = []
      for (let bucket = 1; bucket < buckets.length; bucket++) appendFrontier(cutters, buckets[bucket])
      const bucketSchedules = authoredBucketOperations.map(bucketOperation => {
        const bucketSchedule: SemanticScheduleEvent[] = []
        for (const child of runtimeBucketRoots) {
          if (occurrences[child].operation !== bucketOperation) continue
          bucketSchedule.push({ tag: 'group', group: child })
        }
        return bucketSchedule
      })
      let baseReducer: number | null = null
      let cutterReducer: number | null = null
      let differenceRoot: number | null = null
      const effect = effectByOwner.get(canonical)
      if (effect !== undefined) {
        requireOutputCount(canonical, outputRows, 0)
        if (base.length !== 0) {
          fail('E_SEMANTIC_IDENTITY', '$.core.execution.discardedEffects', 'discarded cutters require an exact empty first difference activation')
        }
        if (cutters.length === 0) {
          fail('E_SEMANTIC_IDENTITY', '$.core.execution.discardedEffects', 'empty cutters cannot create a discarded effect')
        }
        consumedEffectOwners.add(canonical)
        if (cutters.length === 1) {
          if (effect.root !== cutters[0].node) {
            fail('E_SEMANTIC_IDENTITY', '$.core.execution.discardedEffects', 'single cutter effect must preserve its exact frontier node')
          }
        } else {
          const reducer = nodes[effect.root]
          if (reducer?.kind !== 'boolean' || reducer.operation !== 'union'
            || !equalNumberList(reducer.inputs, cutters.map(item => item.node))) {
            fail('E_SEMANTIC_IDENTITY', '$.core.execution.discardedEffects', 'discarded cutters require one canonical ordered union reducer')
          }
          claimNode(reducer.id, canonical)
          cutterReducer = reducer.id
        }
      } else if (base.length === 0) {
        requireOutputCount(canonical, outputRows, 0)
        if (cutters.length > 0) {
          if (terminalPrefix === null) {
            fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'empty-base difference must retain its eager cutter effect')
          }
          const root = cutters.length === 1
            ? cutters[0].node
            : terminalPrefix.find(entry => {
              const candidate = nodes[entry.root]
              return candidate?.kind === 'boolean' && candidate.operation === 'union'
                && equalNumberList(candidate.inputs, cutters.map(item => item.node))
            })?.root
          if (root === undefined || !terminalPrefix.some(entry => entry.root === root)) {
            fail('E_SEMANTIC_IDENTITY', '$.core.execution.terminal.prefixFrontier', 'terminal prefix omits a completed eager difference cutter root')
          }
          if (cutters.length > 1) {
            claimNode(root, canonical)
            cutterReducer = root
          }
          consumedEffectOwners.add(canonical)
          terminalEffectRoots.add(root)
        }
      } else if (cutters.length === 0) {
        requireOutputCount(canonical, outputRows, 1)
        const outputNode = occurrences[outputRows[0]].node!
        if (base.length === 1) {
          if (outputNode !== base[0].node) fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${outputRows[0]}].node`, 'single-item difference base must alias its source node')
          preserve(outputRows[0], base[0])
        } else {
          const reducer = nodes[outputNode]
          if (reducer?.kind !== 'boolean' || reducer.operation !== 'union'
            || !equalNumberList(reducer.inputs, base.map(item => item.node))) {
            fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${outputRows[0]}].node`, 'multi-item difference base requires one canonical union reducer')
          }
          baseReducer = reducer.id
          own(outputRows[0], reducer.id)
        }
      } else {
        requireOutputCount(canonical, outputRows, 1)
        const produced = requireNodeKind(outputRows[0], ['boolean'])
        if (produced.kind !== 'boolean' || produced.operation !== 'difference' || produced.inputs.length !== 2) {
          fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}]`, 'difference root must consume exactly reduced base and cutters')
        }
        const requireReduction = (reference: number, items: readonly SemanticProductionItem[], label: string) => {
          if (items.length === 1) {
            if (reference !== items[0].node) fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}]`, `${label} reduction must preserve its sole frontier node`)
            return
          }
          const reducer = nodes[reference]
          if (reducer?.kind !== 'boolean' || reducer.operation !== 'union'
            || !equalNumberList(reducer.inputs, items.map(item => item.node))) {
            fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}]`, `${label} requires one canonical ordered union reducer`)
          }
          claimNode(reducer.id, canonical)
          if (label === 'difference base') baseReducer = reducer.id
          else cutterReducer = reducer.id
        }
        requireReduction(produced.inputs[0], base, 'difference base')
        requireReduction(produced.inputs[1], cutters, 'difference cutters')
        differenceRoot = produced.id
        own(outputRows[0], produced.id)
      }
      // A difference is the one rule whose compiler-owned reductions are
      // interleaved with child evaluation: the base is reduced before any
      // cutter statement starts. Rebuild this group's authenticated schedule
      // from authored statement buckets instead of generic child postorder.
      schedule.length = 0
      schedule.push(...(bucketSchedules[0] ?? []))
      if (baseReducer !== null) schedule.push({ tag: 'node', node: baseReducer })
      for (let bucket = 1; bucket < bucketSchedules.length; bucket++) {
        schedule.push(...bucketSchedules[bucket])
      }
      if (cutterReducer !== null) schedule.push({ tag: 'node', node: cutterReducer })
      if (differenceRoot !== null) schedule.push({ tag: 'node', node: differenceRoot })
    } else {
      if (frontier.length === 0) {
        requireOutputCount(canonical, outputRows, 0)
      } else {
        requireOutputCount(canonical, outputRows, 1)
        const expectedKind = rule === 'linear-extrude'
          ? ['linear-extrude'] as const
          : ['rotate-extrude-analytic', 'rotate-extrude-polygonal'] as const
        const produced = requireNodeKind(outputRows[0], expectedKind)
        const directInput = semanticNodeInputs(produced)[0]
        if (frontier.length === 1) {
          if (directInput !== frontier[0].node) fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}].input`, 'extrusion must consume its sole profile frontier node')
        } else {
          const reducer = nodes[directInput]
          if (reducer?.kind !== 'boolean' || reducer.operation !== 'union'
            || !equalNumberList(reducer.inputs, frontier.map(item => item.node))) {
            fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}].input`, 'multi-profile extrusion requires one canonical internal union reduction')
          }
          claimNode(reducer.id, canonical)
          schedule.push({ tag: 'node', node: reducer.id })
        }
        own(outputRows[0], produced.id)
      }
    }

    if (terminalItems === null) {
      terminalItems = outputItems.length > 0
        ? [...outputItems]
        : rule === 'transparent' ? [...frontier] : []
    }
    const production = Object.freeze({
      canonical,
      outputRows: Object.freeze([...outputRows]),
      outputItems: Object.freeze(outputItems),
      frontier: Object.freeze(frontier),
      terminalItems: Object.freeze(terminalItems),
      schedule: Object.freeze(schedule),
      transparent: rule === 'transparent',
    })
    productionByCanonical.set(canonical, production)
    return production
  }

  for (let index = canonicalGroups.length - 1; index >= 0; index--) evaluate(canonicalGroups[index])
  if ([...effectByOwner.keys()].some(owner => !consumedEffectOwners.has(owner))) {
    fail('E_SEMANTIC_IDENTITY', '$.core.execution.discardedEffects', 'discarded effect is not owned by an exact empty-base difference occurrence')
  }
  if (ownerByNode.size !== nodes.length) {
    const unowned = nodes.find(node => !ownerByNode.has(node.id))
    fail('E_SEMANTIC_IDENTITY', unowned ? `$.core.nodes[${unowned.id}]` : '$.core.nodes', 'every reachable DAG node needs exactly one recomputed materializer group')
  }

  const scheduleStack: SemanticScheduleEvent[] = []
  for (let index = canonicalGroups.length - 1; index >= 0; index--) {
    const canonical = canonicalGroups[index]
    if (occurrences[canonical].parent === null) {
      scheduleStack.push({ tag: 'group', group: canonical })
    }
  }
  const authenticatedSchedule: number[] = []
  let scheduleEvents = 0
  while (scheduleStack.length > 0) {
    if (++scheduleEvents > SEMANTIC_PROGRAM_LIMITS.occurrences + SEMANTIC_PROGRAM_LIMITS.nodes * 2) {
      fail('E_SEMANTIC_LIMIT', '$.core.execution.evaluationOrder', 'occurrence-production schedule exceeds its bounded replay budget')
    }
    const event = scheduleStack.pop()!
    if (event.tag === 'node') {
      authenticatedSchedule.push(event.node)
      if (authenticatedSchedule.length > nodes.length) {
        fail('E_SEMANTIC_ORDER', '$.core.execution.evaluationOrder', 'occurrence-production schedule creates a node more than once')
      }
      continue
    }
    const childSchedule = productionByCanonical.get(event.group)?.schedule
    if (childSchedule === undefined) {
      fail('E_SEMANTIC_REFERENCE', '$.core.execution.evaluationOrder', 'occurrence-production schedule references an absent group')
    }
    for (let index = childSchedule.length - 1; index >= 0; index--) {
      scheduleStack.push(childSchedule[index])
    }
  }
  if (authenticatedSchedule.length !== nodes.length
    || authenticatedSchedule.some((node, index) => node !== index)) {
    fail('E_SEMANTIC_ORDER', '$.core.execution.evaluationOrder', 'node IDs must equal the exact occurrence-production schedule')
  }

  const programFrontier: SemanticProductionItem[] = []
  for (const canonical of canonicalGroups) {
    if (occurrences[canonical].parent !== null) continue
    const production = productionByCanonical.get(canonical)!
    appendFrontier(programFrontier, production.terminalItems)
  }
  const rootOutputs = semanticResultItems(result)
  const reachableOutputGroups = new Set<number>()
  const rootGroups: number[] = []
  if (terminalPrefix === null) {
    if (rootOutputs.length !== programFrontier.length) {
      fail('E_SEMANTIC_IDENTITY', '$.core.result', 'root result must exactly equal the synthetic program frontier')
    }
    rootOutputs.forEach((output, index) => {
      const slot = programFrontier[index]
      if (slot.row !== output.producerOccurrence || slot.node !== output.node) {
        fail('E_SEMANTIC_IDENTITY', '$.core.result', 'root producer must be an exact output row derived by its occurrence production rule')
      }
      if (slot.identityOwner !== output.identityOccurrence) {
        fail('E_SEMANTIC_IDENTITY', '$.core.result', 'root identity occurrence does not match the recursively derived production owner')
      }
      rootGroups.push(slot.group)
    })
  } else {
    if (rootOutputs.length !== 0) {
      fail('E_SEMANTIC_IDENTITY', '$.core.result', 'terminal result must remain bottom')
    }
    const derivedRoots = [...new Set([
      ...programFrontier.map(item => item.node),
      ...terminalEffectRoots,
    ])].sort((left, right) => left - right)
    if (derivedRoots.length !== terminalPrefix.length
      || derivedRoots.some((root, index) => root !== terminalPrefix[index].root)) {
      fail('E_SEMANTIC_IDENTITY', '$.core.execution.terminal.prefixFrontier', 'terminal prefix must exactly equal the completed production frontier')
    }
    for (const slot of programFrontier) rootGroups.push(slot.group)
  }
  for (const owner of consumedEffectOwners) rootGroups.push(owner)
  while (rootGroups.length > 0) {
    const canonical = rootGroups.pop()!
    if (reachableOutputGroups.has(canonical)) continue
    reachableOutputGroups.add(canonical)
    for (const item of productionByCanonical.get(canonical)!.frontier) rootGroups.push(item.group)
  }
  for (const canonical of canonicalGroups) {
    const production = productionByCanonical.get(canonical)!
    if (production.outputRows.length > 0 && !reachableOutputGroups.has(canonical)) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'node-bearing occurrence group is not consumed by a root production path')
    }
  }
}

function validateCore(value: unknown): asserts value is SemanticProgramCoreV1 {
  exactKeys(value, [
    'schema', 'schemaVersion', 'requiredFeatures', 'identityVersion', 'language',
    'units', 'operations', 'occurrences', 'nodes', 'execution', 'result', 'declaredCapabilities',
    'capabilityClosure', 'diagnosticTemplates',
  ], '$.core')
  if (value.schema !== 'semantic-program-core') fail('E_SEMANTIC_SCHEMA', '$.core.schema', 'expected semantic-program-core')
  assertSchemaVersion(value.schemaVersion, '$.core.schemaVersion')
  const features = validateSortedStrings(value.requiredFeatures, '$.core.requiredFeatures', 64, /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/)
  if (features.length !== 1 || features[0] !== SEMANTIC_PROGRAM_EXECUTION_FEATURE) {
    fail('E_SEMANTIC_VERSION', '$.core.requiredFeatures', 'v1.2 requires exactly semantic.execution-v2')
  }
  if (value.identityVersion !== SEMANTIC_PROGRAM_IDENTITY) fail('E_SEMANTIC_VERSION', '$.core.identityVersion', 'unknown core identity version')
  exactKeys(value.language, ['contract', 'semanticsRevision', 'capabilityGraphVersion'], '$.core.language')
  if (value.language.contract !== 'legacy/current' && value.language.contract !== 'openscad-viewer/brep-1') fail('E_SEMANTIC_FIELD', '$.core.language.contract', 'unknown language contract')
  const expectedRevision = value.language.contract === 'legacy/current' ? '1.0.0' : 'brep-1.0.0'
  if (value.language.semanticsRevision !== expectedRevision) fail('E_SEMANTIC_VERSION', '$.core.language.semanticsRevision', `expected ${expectedRevision}`)
  if (value.language.capabilityGraphVersion !== SEMANTIC_PROGRAM_CAPABILITY_GRAPH) fail('E_SEMANTIC_VERSION', '$.core.language.capabilityGraphVersion', 'unknown capability graph')
  exactKeys(value.units, ['length', 'angle', 'handedness', 'upAxis', 'matrixLayout', 'composition'], '$.core.units')
  if (value.units.length !== 'millimeter' || value.units.angle !== 'degree'
    || value.units.handedness !== 'right' || value.units.upAxis !== 'z'
    || value.units.matrixLayout !== 'column-major' || value.units.composition !== 'parent-times-local') {
    fail('E_SEMANTIC_TYPE', '$.core.units', 'v1 units/frame/matrix convention is fixed')
  }
  const operations = validateOperations(value.operations)
  const nodes = validateNodes(value.nodes, value.language.contract)
  const occurrences = validateOccurrences(value.occurrences, operations, nodes)
  const templates = validateDiagnosticTemplates(value.diagnosticTemplates, operations)
  const outputRoots = validateResult(value.result, nodes, occurrences)
  const execution = validateExecution(
    value.execution,
    nodes,
    occurrences,
    operations,
    outputRoots,
    templates,
    value.language.contract,
  )
  validateOccurrenceProduction(
    operations,
    occurrences,
    nodes,
    value.result as SemanticProgramCoreV1['result'],
    execution.discardedEffects,
    execution.terminal,
  )
  const declared = validateSortedStrings(value.declaredCapabilities, '$.core.declaredCapabilities', SEMANTIC_PROGRAM_LIMITS.declaredCapabilities, /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/)
  const closure = validateSortedStrings(value.capabilityClosure, '$.core.capabilityClosure', SEMANTIC_PROGRAM_LIMITS.capabilityClosure, /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/)
  const expectedClosure = deriveSemanticCapabilityClosure({
    language: value.language as unknown as SemanticProgramCoreV1['language'],
    nodes,
    result: value.result as SemanticProgramCoreV1['result'],
    operations,
    occurrences,
    declaredCapabilities: declared,
    diagnosticTemplates: templates,
  })
  if (closure.length !== expectedClosure.length || closure.some((capability, index) => capability !== expectedClosure[index])) {
    fail('E_SEMANTIC_CAPABILITY_CLOSURE', '$.core.capabilityClosure', 'closure does not exactly match authored and inferred capabilities')
  }
}

function validateSpan(value: unknown, path: string, sourceLength: number, emptyAllowed: boolean): asserts value is SemanticSourceSpan {
  exactKeys(value, ['start', 'end'], path)
  const start = integer(value.start, `${path}.start`, 0, sourceLength)
  const end = integer(value.end, `${path}.end`, 0, sourceLength)
  if (end < start || (!emptyAllowed && end === start)) fail('E_SEMANTIC_PROVENANCE', path, 'invalid half-open UTF-16 source span')
}

export function validateSemanticProgramV1(value: unknown): asserts value is SemanticProgramEnvelopeV1 {
  exactKeys(value, ['schema', 'schemaVersion', 'source', 'core', 'provenance', 'tessellationIntents', 'diagnostics'], '$')
  if (value.schema !== 'semantic-program-envelope') fail('E_SEMANTIC_SCHEMA', '$.schema', 'expected semantic-program-envelope')
  assertSchemaVersion(value.schemaVersion, '$.schemaVersion')
  exactKeys(value.source, ['sha256', 'utf8ByteLength', 'utf16CodeUnitLength'], '$.source')
  if (typeof value.source.sha256 !== 'string' || !/^[a-f0-9]{64}$/.test(value.source.sha256)) fail('E_SEMANTIC_FIELD', '$.source.sha256', 'expected lowercase SHA-256')
  integer(value.source.utf8ByteLength, '$.source.utf8ByteLength', 0, 4_000_000)
  const sourceLength = integer(value.source.utf16CodeUnitLength, '$.source.utf16CodeUnitLength', 0, 250_000)
  const core = value.core
  validateCore(core)

  const provenance = arrayValue(value.provenance, '$.provenance', SEMANTIC_PROGRAM_LIMITS.operations)
  if (provenance.length !== core.operations.length) fail('E_SEMANTIC_PROVENANCE', '$.provenance', 'every static operation needs exactly one source record')
  provenance.forEach((record, index) => {
    const path = `$.provenance[${index}]`
    exactKeys(record, ['operation', 'span', 'label'], path)
    if (record.operation !== index) fail('E_SEMANTIC_ORDER', `${path}.operation`, 'provenance must follow operation order')
    validateSpan(record.span, `${path}.span`, sourceLength, false)
    stringValue(record.label, `${path}.label`, 512)
  })

  const intents = arrayValue(value.tessellationIntents, '$.tessellationIntents', SEMANTIC_PROGRAM_LIMITS.tessellationIntents)
  if (core.language.contract === 'legacy/current' && intents.length !== 0) {
    fail('E_SEMANTIC_TYPE', '$.tessellationIntents', 'legacy/current materializes tessellation in SPC1 and cannot carry TSP1 intents')
  }
  let priorOccurrence = -1
  intents.forEach((intent, index) => {
    const path = `$.tessellationIntents[${index}]`
    exactKeys(intent, ['occurrence', 'chordTolerance', 'angularToleranceDegrees', 'minSegments', 'maxSegments'], path)
    const occurrence = integer(intent.occurrence, `${path}.occurrence`, 0, Math.max(0, core.occurrences.length - 1))
    if (core.occurrences.length === 0 || core.occurrences[occurrence].node === null) fail('E_SEMANTIC_REFERENCE', `${path}.occurrence`, 'tessellation intent needs a producing occurrence')
    if (occurrence <= priorOccurrence) fail('E_SEMANTIC_ORDER', `${path}.occurrence`, 'intents must be unique and ordered by occurrence')
    priorOccurrence = occurrence
    nullablePositive(intent.chordTolerance, `${path}.chordTolerance`)
    nullablePositive(intent.angularToleranceDegrees, `${path}.angularToleranceDegrees`)
    const minimum = intent.minSegments === null ? null : integer(intent.minSegments, `${path}.minSegments`, 3, 1_000_000)
    const maximum = intent.maxSegments === null ? null : integer(intent.maxSegments, `${path}.maxSegments`, 3, 1_000_000)
    if (minimum !== null && maximum !== null && maximum < minimum) fail('E_SEMANTIC_NUMBER', path, 'maxSegments cannot be below minSegments')
  })

  const diagnostics = arrayValue(value.diagnostics, '$.diagnostics', SEMANTIC_PROGRAM_LIMITS.diagnostics)
  if (diagnostics.length !== core.diagnosticTemplates.length) fail('E_SEMANTIC_PROVENANCE', '$.diagnostics', 'every diagnostic template needs exactly one source-bound presentation')
  diagnostics.forEach((diagnostic, index) => {
    const path = `$.diagnostics[${index}]`
    exactKeys(diagnostic, ['template', 'message', 'span'], path)
    if (diagnostic.template !== index) fail('E_SEMANTIC_ORDER', `${path}.template`, 'diagnostics must follow template order')
    stringValue(diagnostic.message, `${path}.message`, SEMANTIC_PROGRAM_LIMITS.diagnosticMessageCodeUnits)
    if (diagnostic.span !== null) validateSpan(diagnostic.span, `${path}.span`, sourceLength, true)
  })
}

interface SnapshotBudget { values: number }

function snapshotJsonValue(value: unknown, path: string, depth: number, ancestors: Set<object>, budget: SnapshotBudget): unknown {
  if (++budget.values > SEMANTIC_PROGRAM_LIMITS.snapshotValues) fail('E_SEMANTIC_LIMIT', path, 'snapshot value budget exceeded')
  if (depth > SEMANTIC_PROGRAM_LIMITS.snapshotDepth) fail('E_SEMANTIC_LIMIT', path, 'snapshot depth exceeded')
  if (value === null || typeof value === 'boolean') return value
  if (typeof value === 'number') return finiteNumber(value, path)
  if (typeof value === 'string') return stringValue(value, path, SEMANTIC_PROGRAM_LIMITS.stringCodeUnits)
  if (typeof value !== 'object') fail('E_SEMANTIC_SCHEMA', path, 'only JSON data is allowed')
  if (Object.getOwnPropertySymbols(value).length !== 0) fail('E_SEMANTIC_FIELD', path, 'symbol fields are forbidden')
  if (ancestors.has(value)) fail('E_SEMANTIC_DAG', path, 'wire object graph cannot be cyclic')
  ancestors.add(value)
  try {
    if (Array.isArray(value)) {
      const keys = Reflect.ownKeys(value)
      if (keys.length !== value.length + 1 || keys[keys.length - 1] !== 'length'
        || keys.slice(0, -1).some((key, index) => key !== String(index))) {
        fail('E_SEMANTIC_FIELD', path, 'arrays must be dense without symbol, hidden, or extra fields')
      }
      return Array.from({ length: value.length }, (_, index) => {
        const descriptor = Object.getOwnPropertyDescriptor(value, String(index))
        if (!descriptor || !Object.hasOwn(descriptor, 'value') || !descriptor.enumerable) fail('E_SEMANTIC_FIELD', `${path}[${index}]`, 'array entries must be enumerable data properties')
        return snapshotJsonValue(descriptor.value, `${path}[${index}]`, depth + 1, ancestors, budget)
      })
    }
    const prototype = Object.getPrototypeOf(value)
    if (prototype !== Object.prototype && prototype !== null) fail('E_SEMANTIC_SCHEMA', path, 'only plain JSON objects are allowed')
    const output: Record<string, unknown> = Object.create(null)
    for (const key of Reflect.ownKeys(value)) {
      if (typeof key !== 'string') fail('E_SEMANTIC_FIELD', path, 'symbol fields are forbidden')
      if (!wellFormedUnicode(key)) fail('E_SEMANTIC_FIELD', path, 'object keys must be well-formed Unicode')
      const descriptor = Object.getOwnPropertyDescriptor(value, key)
      if (!descriptor || !Object.hasOwn(descriptor, 'value') || !descriptor.enumerable) fail('E_SEMANTIC_FIELD', `${path}.${key}`, 'object fields must be enumerable data properties')
      Object.defineProperty(output, key, {
        value: snapshotJsonValue(descriptor.value, `${path}.${key}`, depth + 1, ancestors, budget),
        enumerable: true,
        configurable: true,
        writable: true,
      })
    }
    return output
  } finally {
    ancestors.delete(value)
  }
}

function deepFreeze<T>(value: T): T {
  if (value !== null && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.values(value as Record<string, unknown>).forEach(deepFreeze)
    Object.freeze(value)
  }
  return value
}

function validateCanonicalStringBudget(value: unknown): void {
  const encoder = new TextEncoder()
  const pending: Array<{ value: unknown; path: string }> = [{ value, path: '$' }]
  let bytes = 0
  const add = (text: string, path: string) => {
    bytes += encoder.encode(text).length
    if (bytes > SEMANTIC_PROGRAM_LIMITS.canonicalStringBytes) {
      fail('E_SEMANTIC_LIMIT', path, `canonical string/key bytes exceed ${SEMANTIC_PROGRAM_LIMITS.canonicalStringBytes}`)
    }
  }
  while (pending.length > 0) {
    const current = pending.pop()!
    if (typeof current.value === 'string') {
      add(current.value, current.path)
    } else if (Array.isArray(current.value)) {
      for (let index = current.value.length - 1; index >= 0; index--) pending.push({ value: current.value[index], path: `${current.path}[${index}]` })
    } else if (isRecord(current.value)) {
      const entries = Object.entries(current.value)
      for (let index = entries.length - 1; index >= 0; index--) {
        const [key, child] = entries[index]
        add(key, `${current.path}.<key>`)
        pending.push({ value: child, path: `${current.path}.${key}` })
      }
    }
  }
}

export function normalizeSemanticProgram(input: unknown, exactSource?: string): SemanticProgramV1 {
  const snapshot = snapshotJsonValue(input, '$', 0, new Set(), { values: 0 })
  if (!isRecord(snapshot)) fail('E_SEMANTIC_SCHEMA', '$', 'expected a semantic program envelope')
  if (snapshot.version === 0) {
    fail('E_SEMANTIC_RELOWER_REQUIRED', '$.version', 'flat semantic prototype requires exact-source re-lowering to schema 1.2')
  }
  validateCanonicalStringBudget(snapshot)
  validateSemanticProgramV1(snapshot)
  const program = deepFreeze(snapshot)
  if (exactSource !== undefined) attestSemanticProgramSource(program, exactSource)
  return program
}

/**
 * Validates and freezes the graph produced exclusively by the bounded binary
 * decoder. Unlike the public normalizer, this does not duplicate that already
 * isolated, null-prototype JSON graph. Callers with arbitrary objects must use
 * normalizeSemanticProgram so descriptors and prototypes are snapshotted.
 */
export function normalizeDecodedSemanticProgram(input: unknown): SemanticProgramV1 {
  if (!isSemanticProgramCodecOwnedGraph(input)) {
    fail('E_SEMANTIC_SCHEMA', '$', 'in-place normalization requires a codec-owned decoded graph')
  }
  if (!isRecord(input)) fail('E_SEMANTIC_SCHEMA', '$', 'expected a semantic program envelope')
  // Old wire prototypes cannot synthesize v1.2 execution evidence.
  if (input.version === 0) {
    fail('E_SEMANTIC_RELOWER_REQUIRED', '$.version', 'flat semantic prototype requires exact-source re-lowering to schema 1.2')
  }
  validateCanonicalStringBudget(input)
  validateSemanticProgramV1(input)
  return deepFreeze(input as unknown as SemanticProgramV1)
}

export function normalizeSemanticProgramCore(input: unknown): SemanticProgramCoreV1 {
  const snapshot = snapshotJsonValue(input, '$.core', 0, new Set(), { values: 0 })
  validateCanonicalStringBudget(snapshot)
  validateCore(snapshot)
  return deepFreeze(snapshot as unknown as SemanticProgramCoreV1)
}

/** In-place counterpart for the graph freshly allocated by ByteReader only. */
export function normalizeDecodedSemanticProgramCore(input: unknown): SemanticProgramCoreV1 {
  if (!isSemanticProgramCodecOwnedGraph(input)) {
    fail('E_SEMANTIC_SCHEMA', '$.core', 'in-place normalization requires a codec-owned decoded graph')
  }
  validateCanonicalStringBudget(input)
  validateCore(input)
  return deepFreeze(input as unknown as SemanticProgramCoreV1)
}

export function normalizeSemanticProgramCoreInput(input: unknown): SemanticProgramCoreV1 {
  const snapshot = snapshotJsonValue(input, '$', 0, new Set(), { values: 0 })
  if (!isRecord(snapshot)) fail('E_SEMANTIC_SCHEMA', '$', 'expected a semantic program core or envelope')
  if (snapshot.version === 0) {
    fail('E_SEMANTIC_RELOWER_REQUIRED', '$.version', 'flat semantic prototype requires exact-source re-lowering to schema 1.2')
  }
  validateCanonicalStringBudget(snapshot)
  if (snapshot.schema === 'semantic-program-core') {
    validateCore(snapshot)
    return deepFreeze(snapshot as unknown as SemanticProgramCoreV1)
  }
  validateSemanticProgramV1(snapshot)
  return deepFreeze(snapshot as unknown as SemanticProgramV1).core
}

export function semanticSourceDescriptor(source: string): SemanticSourceDescriptor {
  if (!wellFormedUnicode(source)) fail('E_SEMANTIC_BYTES', '$source', 'semantic source must be well-formed Unicode before UTF-8 encoding')
  if (source.length > SEMANTIC_PROGRAM_LIMITS.sourceCodeUnits) {
    fail('E_SEMANTIC_LIMIT', '$source', `semantic source exceeds ${SEMANTIC_PROGRAM_LIMITS.sourceCodeUnits} UTF-16 code units`)
  }
  const bytes = new TextEncoder().encode(source)
  if (bytes.length > 4_000_000) fail('E_SEMANTIC_LIMIT', '$source', 'semantic source exceeds 4000000 UTF-8 bytes')
  return Object.freeze({
    sha256: sha256Hex(bytes),
    utf8ByteLength: bytes.length,
    utf16CodeUnitLength: source.length,
  })
}

export function attestSemanticProgramSource(program: SemanticProgramV1, source: string): void {
  const actual = semanticSourceDescriptor(source)
  if (actual.sha256 !== program.source.sha256
    || actual.utf8ByteLength !== program.source.utf8ByteLength
    || actual.utf16CodeUnitLength !== program.source.utf16CodeUnitLength) {
    fail('E_SEMANTIC_PROVENANCE', '$.source', 'source bytes do not match the envelope attestation')
  }
  let route: ReturnType<typeof parseGeometrySourceRoutingHeader>
  try {
    route = parseGeometrySourceRoutingHeader(source)
  } catch (error) {
    fail('E_SEMANTIC_PROVENANCE', '$source', `source routing header is invalid: ${error instanceof Error ? error.message : String(error)}`)
  }
  if (route.languageContract !== program.core.language.contract) {
    fail('E_SEMANTIC_PROVENANCE', '$.core.language.contract', 'source routing language does not match the semantic core')
  }
  if (route.requiredCapabilities.length !== program.core.declaredCapabilities.length
    || route.requiredCapabilities.some((capability, index) => capability !== program.core.declaredCapabilities[index])) {
    fail('E_SEMANTIC_PROVENANCE', '$.core.declaredCapabilities', 'source routing requirements do not match authored semantic capabilities')
  }
  const assertScalarBoundary = (offset: number, path: string) => {
    if (offset > 0 && offset < source.length) {
      const prior = source.charCodeAt(offset - 1)
      const next = source.charCodeAt(offset)
      if (prior >= 0xd800 && prior <= 0xdbff && next >= 0xdc00 && next <= 0xdfff) {
        fail('E_SEMANTIC_PROVENANCE', path, 'UTF-16 span endpoint splits a surrogate pair')
      }
    }
  }
  program.provenance.forEach((record, index) => {
    assertScalarBoundary(record.span.start, `$.provenance[${index}].span.start`)
    assertScalarBoundary(record.span.end, `$.provenance[${index}].span.end`)
  })
  program.diagnostics.forEach((diagnostic, index) => {
    if (diagnostic.span === null) return
    assertScalarBoundary(diagnostic.span.start, `$.diagnostics[${index}].span.start`)
    assertScalarBoundary(diagnostic.span.end, `$.diagnostics[${index}].span.end`)
  })
}

export function semanticDiagnosticCodeIsPublic(value: string): value is SemanticProgramDiagnosticCode {
  return (SEMANTIC_PROGRAM_DIAGNOSTIC_CODES as readonly string[]).includes(value)
}
