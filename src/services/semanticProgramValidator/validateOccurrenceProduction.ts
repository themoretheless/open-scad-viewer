import { SEMANTIC_PROGRAM_LIMITS, semanticNodeInputs, semanticResultItems } from '../../core/semanticProgram'
import type { SemanticNode, SemanticOccurrence, SemanticProgramCoreV1, SemanticStaticOperation } from '../../core/semanticProgram'
import type { SemanticProductionGroup, SemanticProductionItem, SemanticScheduleEvent, ValidatedSemanticExecution } from '../semanticProgramValidator'
import { equalNumberList, semanticProductionRule } from './validateResult'
import { fail } from './fail'

export function validateOccurrenceProduction(
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
