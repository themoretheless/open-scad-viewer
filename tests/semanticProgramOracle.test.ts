import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import {
  decodeSemanticProgram,
  decodeSemanticProgramCore,
  encodeSemanticProgram,
  encodeSemanticProgramCore,
  encodeSemanticTessellationPolicy,
  semanticProgramHash,
} from '../src/services/semanticProgramCodec'
import {
  normalizeSemanticProgram,
  SemanticProgramValidationError,
} from '../src/services/semanticProgramValidator'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'
import {
  makeReferenceLinearChainProgramV1,
  makeReferenceSemanticProgramV1,
  REFERENCE_SEMANTIC_SOURCE_V1,
  referenceEncodeSemanticProgramCoreV1,
  referenceEncodeSemanticProgramV1,
  referenceSemanticAmbiguityGroupIdV1,
  referenceSemanticCapabilityClosureV1,
  ReferenceSemanticProgramError,
  referenceSemanticOccurrenceIdV1,
  referenceSemanticOperationIdV1,
  referenceSemanticProgramHashV1,
  referenceSemanticSceneEntityIdV1,
  referenceSemanticSourceDescriptorV1,
  referenceValidateSemanticProgramV1,
  type ReferenceSemanticFailureFamily,
  type ReferenceSemanticProgramV1,
} from './support/referenceSemanticProgramV1'

function clone<T>(value: T): T {
  return structuredClone(value)
}

function sha256(bytes: Uint8Array): string {
  return createHash('sha256').update(bytes).digest('hex')
}

function reverseObjectInsertionOrder(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(reverseObjectInsertionOrder)
  if (value === null || typeof value !== 'object') return value
  return Object.fromEntries(
    Object.entries(value as Record<string, unknown>)
      .reverse()
      .map(([key, item]) => [key, reverseObjectInsertionOrder(item)]),
  )
}

function recomputeOccurrence(program: ReferenceSemanticProgramV1, index: number): void {
  const occurrence = program.core.occurrences[index]
  const operation = program.core.operations[occurrence.operation]
  const parentId = occurrence.parent === null
    ? null
    : program.core.occurrences[occurrence.parent].occurrenceId
  const staticParentId = occurrence.staticParent === null
    ? null
    : program.core.occurrences[occurrence.staticParent].occurrenceId
  occurrence.occurrenceId = referenceSemanticOccurrenceIdV1(
    parentId,
    staticParentId,
    operation.operationId,
    occurrence.dynamicSlots,
  )
  occurrence.sceneEntityId = occurrence.outputOrdinal === null
    ? null
    : referenceSemanticSceneEntityIdV1(occurrence.occurrenceId, occurrence.outputOrdinal)
}

function resetSuccessfulExecution(program: ReferenceSemanticProgramV1): void {
  program.core.execution = {
    version: 'semantic-execution-v2',
    evaluationOrder: program.core.nodes.map((node: Record<string, any>) => node.id),
    discardedEffects: [],
    terminal: null,
  }
}

function coherentlyRenumberNodes(
  program: ReferenceSemanticProgramV1,
  oldIdsInNewOrder: readonly number[],
): void {
  if (oldIdsInNewOrder.length !== program.core.nodes.length
    || new Set(oldIdsInNewOrder).size !== oldIdsInNewOrder.length) {
    throw new Error('Fixture node renumbering requires one complete permutation')
  }
  const oldToNew = new Int32Array(oldIdsInNewOrder.length)
  oldIdsInNewOrder.forEach((oldId, newId) => { oldToNew[oldId] = newId })
  const remap = (oldId: number): number => oldToNew[oldId]
  program.core.nodes = oldIdsInNewOrder.map((oldId, newId) => {
    const node = clone(program.core.nodes[oldId])
    node.id = newId
    if (typeof node.input === 'number') node.input = remap(node.input)
    if (Array.isArray(node.inputs)) node.inputs = node.inputs.map(remap)
    return node
  })
  for (const occurrence of program.core.occurrences) {
    if (occurrence.node !== null) occurrence.node = remap(occurrence.node)
  }
  if (program.core.result.tag === 'single') {
    program.core.result.item.node = remap(program.core.result.item.node)
  } else if (program.core.result.tag === 'multi') {
    for (const item of program.core.result.items) item.node = remap(item.node)
  }
  for (const effect of program.core.execution.discardedEffects) effect.root = remap(effect.root)
  program.core.execution.discardedEffects.sort(
    (left: Record<string, any>, right: Record<string, any>) => left.root - right.root,
  )
  if (program.core.execution.terminal !== null) {
    for (const entry of program.core.execution.terminal.prefixFrontier) entry.root = remap(entry.root)
    program.core.execution.terminal.prefixFrontier.sort(
      (left: Record<string, any>, right: Record<string, any>) => left.root - right.root,
    )
  }
  program.core.execution.evaluationOrder = program.core.nodes.map((node: Record<string, any>) => node.id)
}

function makeMultiOutputFixture(): ReferenceSemanticProgramV1 {
  const program = makeReferenceSemanticProgramV1()
  const path = [{ kind: 'call', name: 'square', ordinal: 0 }]
  const operationId = referenceSemanticOperationIdV1(path)
  program.core.operations.push({
    id: 5,
    operationId,
    parent: null,
    childOrdinal: 0,
    name: 'square',
    category: 'geometry',
    structuralPath: path,
    identityEvidence: 'structural-unique',
    ambiguityGroup: null,
  })
  program.core.nodes.push({
    id: 4,
    kind: 'rectangle',
    valueType: {
      geometryKind: 'region',
      space: 'd2',
      representation: 'mesh',
      evidence: { tag: 'representation-preserving' },
    },
    size: [2, 2],
    center: false,
  })
  const occurrenceId = referenceSemanticOccurrenceIdV1(null, null, operationId, [])
  program.core.occurrences.push({
    id: 5,
    occurrenceId,
    operation: 5,
    parent: null,
    staticParent: null,
    dynamicSlots: [],
    node: 4,
    outputOrdinal: 0,
    sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceId, 0),
  })
  program.provenance.push({
    operation: 5,
    span: clone(program.provenance[3].span),
    label: 'square()',
  })
  program.core.result = {
    tag: 'multi',
    items: [
      {
        node: 3,
        producerOccurrence: 1,
        identityOccurrence: 2,
        color: [0.25, 0.5, 0.75, 1],
      },
      {
        node: 4,
        producerOccurrence: 5,
        identityOccurrence: 5,
        color: [1, 0.25, 0.25, 1],
      },
    ],
  }
  resetSuccessfulExecution(program)
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return program
}

function makeSkippedStaticParentFixture(): ReferenceSemanticProgramV1 {
  const program = makeReferenceSemanticProgramV1()
  const structuralPath = [
    ...clone(program.core.operations[1].structuralPath),
    { kind: 'control', name: 'group', ordinal: 0 },
  ]
  const operationId = referenceSemanticOperationIdV1(structuralPath)
  program.core.operations.push({
    id: 5,
    operationId,
    parent: 1,
    childOrdinal: 0,
    name: 'group',
    category: 'control',
    structuralPath,
    identityEvidence: 'structural-unique',
    ambiguityGroup: null,
  })
  program.provenance.push({
    ...clone(program.provenance[2]),
    operation: 5,
    label: 'group() runtime frame',
  })
  for (const occurrence of program.core.occurrences.slice(2)) {
    occurrence.id += 1
    if (occurrence.parent !== null && occurrence.parent >= 2) occurrence.parent += 1
    if (occurrence.staticParent !== null && occurrence.staticParent >= 2) occurrence.staticParent += 1
  }
  const dynamicSlots = [{
    name: 'expansion',
    value: { tag: 'string', value: 'module-frame' },
    duplicateOrdinal: 0,
  }]
  const occurrenceId = referenceSemanticOccurrenceIdV1(
    program.core.occurrences[1].occurrenceId,
    program.core.occurrences[1].occurrenceId,
    operationId,
    dynamicSlots,
  )
  program.core.occurrences.splice(2, 0, {
    id: 2,
    occurrenceId,
    operation: 5,
    parent: 1,
    staticParent: 1,
    dynamicSlots,
    node: null,
    outputOrdinal: null,
    sceneEntityId: null,
  })
  program.core.occurrences[3].parent = 2
  program.core.occurrences[3].staticParent = 1
  for (const index of [3, 4, 5]) recomputeOccurrence(program, index)
  program.core.result.item.identityOccurrence = 3
  return program
}

function makeCanonicalOccurrenceRowFixture(): ReferenceSemanticProgramV1 {
  const program = makeReferenceSemanticProgramV1()
  const paths = [
    [{ kind: 'module', name: 'scene', ordinal: 0 }],
    [{ kind: 'module', name: 'scene', ordinal: 0 }, { kind: 'call', name: 'projection', ordinal: 0 }],
    [{ kind: 'module', name: 'scene', ordinal: 0 }, { kind: 'call', name: 'projection', ordinal: 0 }, { kind: 'call', name: 'cube', ordinal: 0 }],
    [{ kind: 'module', name: 'scene', ordinal: 0 }, { kind: 'call', name: 'projection', ordinal: 0 }, { kind: 'call', name: 'sphere', ordinal: 0 }],
    [{ kind: 'module', name: 'scene', ordinal: 0 }, { kind: 'call', name: 'projection', ordinal: 0 }, { kind: 'control', name: 'group', ordinal: 0 }],
  ]
  const operationIds = paths.map(referenceSemanticOperationIdV1)
  program.core.operations = [
    { id: 0, operationId: operationIds[0], parent: null, childOrdinal: 0, name: 'scene', category: 'module', structuralPath: paths[0], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 1, operationId: operationIds[1], parent: 0, childOrdinal: 0, name: 'projection', category: 'geometry', structuralPath: paths[1], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 2, operationId: operationIds[2], parent: 1, childOrdinal: 0, name: 'cube', category: 'geometry', structuralPath: paths[2], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 3, operationId: operationIds[3], parent: 1, childOrdinal: 0, name: 'sphere', category: 'geometry', structuralPath: paths[3], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 4, operationId: operationIds[4], parent: 1, childOrdinal: 0, name: 'group', category: 'control', structuralPath: paths[4], identityEvidence: 'structural-unique', ambiguityGroup: null },
  ]
  program.core.nodes = [
    { id: 0, kind: 'box', valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, size: [1, 2, 3], center: false },
    { id: 1, kind: 'sphere-polygonal', valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, radius: 2, radialSegments: 32 },
    { id: 2, kind: 'projection', valueType: { geometryKind: 'region', space: 'd2', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, input: 0, cut: false },
    { id: 3, kind: 'projection', valueType: { geometryKind: 'region', space: 'd2', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, input: 1, cut: false },
  ]
  const occurrenceIds = [
    referenceSemanticOccurrenceIdV1(null, null, operationIds[0], []),
    '',
    '',
    '',
  ]
  occurrenceIds[1] = referenceSemanticOccurrenceIdV1(occurrenceIds[0], occurrenceIds[0], operationIds[1], [])
  occurrenceIds[2] = referenceSemanticOccurrenceIdV1(occurrenceIds[1], occurrenceIds[1], operationIds[2], [])
  occurrenceIds[3] = referenceSemanticOccurrenceIdV1(occurrenceIds[1], occurrenceIds[1], operationIds[3], [])
  const transparentChildId = referenceSemanticOccurrenceIdV1(
    occurrenceIds[1],
    occurrenceIds[1],
    operationIds[4],
    [],
  )
  program.core.occurrences = [
    { id: 0, occurrenceId: occurrenceIds[0], operation: 0, parent: null, staticParent: null, dynamicSlots: [], node: null, outputOrdinal: null, sceneEntityId: null },
    { id: 1, occurrenceId: occurrenceIds[1], operation: 1, parent: 0, staticParent: 0, dynamicSlots: [], node: 2, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceIds[1], 0) },
    { id: 2, occurrenceId: occurrenceIds[2], operation: 2, parent: 1, staticParent: 1, dynamicSlots: [], node: 0, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceIds[2], 0) },
    { id: 3, occurrenceId: occurrenceIds[3], operation: 3, parent: 1, staticParent: 1, dynamicSlots: [], node: 1, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceIds[3], 0) },
    { id: 4, occurrenceId: occurrenceIds[1], operation: 1, parent: 0, staticParent: 0, dynamicSlots: [], node: 3, outputOrdinal: 1, sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceIds[1], 1) },
    {
      id: 5,
      occurrenceId: transparentChildId,
      operation: 4,
      parent: 1,
      staticParent: 1,
      dynamicSlots: [],
      node: null,
      outputOrdinal: null,
      sceneEntityId: null,
    },
  ]
  program.core.result = {
    tag: 'multi',
    items: [
      { node: 2, producerOccurrence: 1, identityOccurrence: 1, color: [1, 1, 1, 1] },
      { node: 3, producerOccurrence: 4, identityOccurrence: 4, color: [1, 1, 1, 1] },
    ],
  }
  program.core.diagnosticTemplates = []
  program.provenance = [
    { ...clone(program.provenance[0]), operation: 0, label: 'scene()' },
    { ...clone(program.provenance[1]), operation: 1, label: 'projection()' },
    { ...clone(program.provenance[3]), operation: 2, label: 'cube()' },
    { ...clone(program.provenance[4]), operation: 3, label: 'sphere()' },
    { ...clone(program.provenance[0]), operation: 4, label: 'group()' },
  ]
  program.diagnostics = []
  resetSuccessfulExecution(program)
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return program
}

function makeCanonicalFirstTransparentWrapperFixture(): ReferenceSemanticProgramV1 {
  const program = makeReferenceSemanticProgramV1()
  const paths = [
    [{ kind: 'module', name: 'scene', ordinal: 0 }],
    [{ kind: 'module', name: 'scene', ordinal: 0 }, { kind: 'call', name: 'color', ordinal: 0 }],
    [{ kind: 'module', name: 'scene', ordinal: 0 }, { kind: 'call', name: 'color', ordinal: 0 }, { kind: 'call', name: 'cube', ordinal: 0 }],
    [{ kind: 'module', name: 'scene', ordinal: 0 }, { kind: 'call', name: 'color', ordinal: 0 }, { kind: 'call', name: 'sphere', ordinal: 0 }],
  ]
  const operationIds = paths.map(referenceSemanticOperationIdV1)
  program.core.operations = [
    { id: 0, operationId: operationIds[0], parent: null, childOrdinal: 0, name: 'scene', category: 'module', structuralPath: paths[0], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 1, operationId: operationIds[1], parent: 0, childOrdinal: 0, name: 'color', category: 'presentation', structuralPath: paths[1], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 2, operationId: operationIds[2], parent: 1, childOrdinal: 0, name: 'cube', category: 'geometry', structuralPath: paths[2], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 3, operationId: operationIds[3], parent: 1, childOrdinal: 0, name: 'sphere', category: 'geometry', structuralPath: paths[3], identityEvidence: 'structural-unique', ambiguityGroup: null },
  ]
  program.core.nodes = [
    { id: 0, kind: 'box', valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, size: [1, 2, 3], center: false },
    { id: 1, kind: 'sphere-polygonal', valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, radius: 2, radialSegments: 32 },
  ]
  const rootId = referenceSemanticOccurrenceIdV1(null, null, operationIds[0], [])
  const wrapperId = referenceSemanticOccurrenceIdV1(rootId, rootId, operationIds[1], [])
  const cubeId = referenceSemanticOccurrenceIdV1(wrapperId, wrapperId, operationIds[2], [])
  const sphereId = referenceSemanticOccurrenceIdV1(wrapperId, wrapperId, operationIds[3], [])
  program.core.occurrences = [
    { id: 0, occurrenceId: rootId, operation: 0, parent: null, staticParent: null, dynamicSlots: [], node: null, outputOrdinal: null, sceneEntityId: null },
    { id: 1, occurrenceId: wrapperId, operation: 1, parent: 0, staticParent: 0, dynamicSlots: [], node: 0, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(wrapperId, 0) },
    { id: 2, occurrenceId: wrapperId, operation: 1, parent: 0, staticParent: 0, dynamicSlots: [], node: 1, outputOrdinal: 1, sceneEntityId: referenceSemanticSceneEntityIdV1(wrapperId, 1) },
    { id: 3, occurrenceId: cubeId, operation: 2, parent: 1, staticParent: 1, dynamicSlots: [], node: 0, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(cubeId, 0) },
    { id: 4, occurrenceId: sphereId, operation: 3, parent: 1, staticParent: 1, dynamicSlots: [], node: 1, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(sphereId, 0) },
  ]
  program.core.result = {
    tag: 'multi',
    items: [
      { node: 0, producerOccurrence: 1, identityOccurrence: 3, color: [1, 0, 0, 1] },
      { node: 1, producerOccurrence: 2, identityOccurrence: 4, color: [1, 0, 0, 1] },
    ],
  }
  program.core.diagnosticTemplates = []
  program.provenance = [
    { ...clone(program.provenance[0]), operation: 0, label: 'scene()' },
    { ...clone(program.provenance[1]), operation: 1, label: 'color()' },
    { ...clone(program.provenance[3]), operation: 2, label: 'cube()' },
    { ...clone(program.provenance[4]), operation: 3, label: 'sphere()' },
  ]
  program.diagnostics = []
  resetSuccessfulExecution(program)
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return program
}

function makePrimitiveSchemaFixture(
  name: 'polyhedron' | 'polygon',
  fields: Record<string, unknown>,
  contract: 'legacy/current' | 'openscad-viewer/brep-1' = 'legacy/current',
): { program: ReferenceSemanticProgramV1; source: string } {
  const source = contract === 'legacy/current'
    ? `${name}();`
    : `// @language openscad-viewer/brep-1\n// @requires geometry.brep\n${name}();`
  const path = [{ kind: 'call', name, ordinal: 0 }]
  const operationId = referenceSemanticOperationIdV1(path)
  const occurrenceId = referenceSemanticOccurrenceIdV1(null, null, operationId, [])
  const geometryKind = name === 'polyhedron' ? 'solid-set' : 'region'
  const space = name === 'polyhedron' ? 'd3' : 'd2'
  const program = makeReferenceSemanticProgramV1()
  program.source = referenceSemanticSourceDescriptorV1(source)
  program.core.language = {
    contract,
    semanticsRevision: contract === 'legacy/current' ? '1.0.0' : 'brep-1.0.0',
    capabilityGraphVersion: 'semantic-capabilities-v1',
  }
  program.core.operations = [{
    id: 0,
    operationId,
    parent: null,
    childOrdinal: 0,
    name,
    category: 'geometry',
    structuralPath: path,
    identityEvidence: 'structural-unique',
    ambiguityGroup: null,
  }]
  program.core.nodes = [{
    id: 0,
    kind: name,
    valueType: {
      geometryKind,
      space,
      representation: contract === 'legacy/current' ? 'mesh' : 'analytic-brep',
      evidence: { tag: 'representation-preserving' },
    },
    ...fields,
  }]
  program.core.occurrences = [{
    id: 0,
    occurrenceId,
    operation: 0,
    parent: null,
    staticParent: null,
    dynamicSlots: [],
    node: 0,
    outputOrdinal: 0,
    sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceId, 0),
  }]
  program.core.result = {
    tag: 'single',
    item: { node: 0, producerOccurrence: 0, identityOccurrence: 0, color: [1, 1, 1, 1] },
  }
  program.core.declaredCapabilities = contract === 'legacy/current' ? [] : ['geometry.brep']
  program.core.diagnosticTemplates = []
  program.provenance = [{
    operation: 0,
    span: { start: source.lastIndexOf(name), end: source.length },
    label: `${name}()`,
  }]
  program.tessellationIntents = []
  program.diagnostics = []
  resetSuccessfulExecution(program)
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return { program, source }
}

function makeDiscardedDifferenceEffectFixture(
  contract: 'legacy/current' | 'openscad-viewer/brep-1' = 'legacy/current',
): { program: ReferenceSemanticProgramV1; source: string } {
  const body = 'difference(){ let(x=0); cube(1); sphere(1); }'
  const source = contract === 'legacy/current'
    ? body
    : `// @language openscad-viewer/brep-1\n// @requires geometry.brep\n${body}`
  const paths = [
    [{ kind: 'call', name: 'difference', ordinal: 0 }],
    [{ kind: 'call', name: 'difference', ordinal: 0 }, { kind: 'body', name: '$body', ordinal: 0 }],
    [{ kind: 'call', name: 'difference', ordinal: 0 }, { kind: 'body', name: '$body', ordinal: 0 }, { kind: 'control', name: 'let', ordinal: 0 }],
    [{ kind: 'call', name: 'difference', ordinal: 0 }, { kind: 'body', name: '$body', ordinal: 0 }, { kind: 'call', name: 'cube', ordinal: 0 }],
    [{ kind: 'call', name: 'difference', ordinal: 0 }, { kind: 'body', name: '$body', ordinal: 0 }, { kind: 'call', name: 'sphere', ordinal: 0 }],
  ]
  const operationIds = paths.map(referenceSemanticOperationIdV1)
  const program = makeReferenceSemanticProgramV1()
  program.source = referenceSemanticSourceDescriptorV1(source)
  program.core.language = {
    contract,
    semanticsRevision: contract === 'legacy/current' ? '1.0.0' : 'brep-1.0.0',
    capabilityGraphVersion: 'semantic-capabilities-v1',
  }
  program.core.operations = [
    { id: 0, operationId: operationIds[0], parent: null, childOrdinal: 0, name: 'difference', category: 'boolean', structuralPath: paths[0], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 1, operationId: operationIds[1], parent: 0, childOrdinal: 0, name: '$body', category: 'control', structuralPath: paths[1], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 2, operationId: operationIds[2], parent: 1, childOrdinal: 0, name: 'let', category: 'control', structuralPath: paths[2], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 3, operationId: operationIds[3], parent: 1, childOrdinal: 0, name: 'cube', category: 'geometry', structuralPath: paths[3], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 4, operationId: operationIds[4], parent: 1, childOrdinal: 0, name: 'sphere', category: 'geometry', structuralPath: paths[4], identityEvidence: 'structural-unique', ambiguityGroup: null },
  ]
  const valueType = {
    geometryKind: contract === 'legacy/current' ? 'solid-set' : 'solid',
    space: 'd3',
    representation: contract === 'legacy/current' ? 'mesh' : 'analytic-brep',
    evidence: { tag: 'representation-preserving' },
  }
  program.core.nodes = [
    { id: 0, kind: 'box', valueType: clone(valueType), size: [1, 1, 1], center: false },
    contract === 'legacy/current'
      ? { id: 1, kind: 'sphere-polygonal', valueType: clone(valueType), radius: 1, radialSegments: 16 }
      : { id: 1, kind: 'sphere-analytic', valueType: clone(valueType), radius: 1 },
    { id: 2, kind: 'boolean', valueType: clone(valueType), operation: 'union', inputs: [0, 1] },
  ]
  const occurrenceIds = [
    referenceSemanticOccurrenceIdV1(null, null, operationIds[0], []),
    '',
    '',
    '',
    '',
  ]
  occurrenceIds[1] = referenceSemanticOccurrenceIdV1(occurrenceIds[0], occurrenceIds[0], operationIds[1], [])
  occurrenceIds[2] = referenceSemanticOccurrenceIdV1(occurrenceIds[1], occurrenceIds[1], operationIds[2], [])
  occurrenceIds[3] = referenceSemanticOccurrenceIdV1(occurrenceIds[1], occurrenceIds[1], operationIds[3], [])
  occurrenceIds[4] = referenceSemanticOccurrenceIdV1(occurrenceIds[1], occurrenceIds[1], operationIds[4], [])
  program.core.occurrences = [
    { id: 0, occurrenceId: occurrenceIds[0], operation: 0, parent: null, staticParent: null, dynamicSlots: [], node: null, outputOrdinal: null, sceneEntityId: null },
    { id: 1, occurrenceId: occurrenceIds[1], operation: 1, parent: 0, staticParent: 0, dynamicSlots: [], node: null, outputOrdinal: null, sceneEntityId: null },
    { id: 2, occurrenceId: occurrenceIds[2], operation: 2, parent: 1, staticParent: 1, dynamicSlots: [], node: null, outputOrdinal: null, sceneEntityId: null },
    { id: 3, occurrenceId: occurrenceIds[3], operation: 3, parent: 1, staticParent: 1, dynamicSlots: [], node: 0, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceIds[3], 0) },
    { id: 4, occurrenceId: occurrenceIds[4], operation: 4, parent: 1, staticParent: 1, dynamicSlots: [], node: 1, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceIds[4], 0) },
  ]
  program.core.execution = {
    version: 'semantic-execution-v2',
    evaluationOrder: [0, 1, 2],
    discardedEffects: [{
      tag: 'legacy-difference-cutters',
      ownerOccurrence: 0,
      root: 2,
    }],
    terminal: null,
  }
  program.core.result = { tag: 'empty', type: 'never' }
  program.core.declaredCapabilities = contract === 'legacy/current' ? [] : ['geometry.brep']
  program.core.diagnosticTemplates = []
  program.provenance = program.core.operations.map((operation: Record<string, any>) => ({
    operation: operation.id,
    span: { start: source.lastIndexOf(operation.name === '$body' ? 'difference' : operation.name), end: source.length },
    label: `${operation.name}()`,
  }))
  program.tessellationIntents = []
  program.diagnostics = []
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return { program, source }
}

function makeEffectBeforePublishedCubeFixture(): { program: ReferenceSemanticProgramV1; source: string } {
  const fixture = makeDiscardedDifferenceEffectFixture()
  const source = `${fixture.source} cube(2);`
  const program = fixture.program
  const path = [{ kind: 'call', name: 'cube', ordinal: 0 }]
  const operationId = referenceSemanticOperationIdV1(path)
  const operation = program.core.operations.length
  const occurrence = program.core.occurrences.length
  const occurrenceId = referenceSemanticOccurrenceIdV1(null, null, operationId, [])
  program.source = referenceSemanticSourceDescriptorV1(source)
  program.core.operations.push({
    id: operation,
    operationId,
    parent: null,
    childOrdinal: 0,
    name: 'cube',
    category: 'geometry',
    structuralPath: path,
    identityEvidence: 'structural-unique',
    ambiguityGroup: null,
  })
  program.core.nodes.push({
    id: 3,
    kind: 'box',
    valueType: {
      geometryKind: 'solid-set',
      space: 'd3',
      representation: 'mesh',
      evidence: { tag: 'representation-preserving' },
    },
    size: [2, 2, 2],
    center: false,
  })
  program.core.occurrences.push({
    id: occurrence,
    occurrenceId,
    operation,
    parent: null,
    staticParent: null,
    dynamicSlots: [],
    node: 3,
    outputOrdinal: 0,
    sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceId, 0),
  })
  program.core.execution.evaluationOrder = [0, 1, 2, 3]
  program.core.result = {
    tag: 'single',
    item: {
      node: 3,
      producerOccurrence: occurrence,
      identityOccurrence: occurrence,
      color: [1, 1, 1, 1],
    },
  }
  program.provenance.push({
    operation,
    span: { start: source.lastIndexOf('cube'), end: source.length },
    label: 'cube()',
  })
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return { program, source }
}

function makeManyDiscardedDifferenceEffectsFixture(
  count = 1_001,
): { program: ReferenceSemanticProgramV1; source: string } {
  const statement = 'difference(){ let(x=0); cube(1); }\n'
  const source = statement.repeat(count)
  const program = makeReferenceSemanticProgramV1()
  const operations: Record<string, any>[] = []
  const occurrences: Record<string, any>[] = []
  const nodes: Record<string, any>[] = []
  const effects: Record<string, any>[] = []
  const provenance: Record<string, any>[] = []
  const ambiguityGroup = referenceSemanticAmbiguityGroupIdV1([], 'boolean', 'difference')
  const valueType = {
    geometryKind: 'solid-set',
    space: 'd3',
    representation: 'mesh',
    evidence: { tag: 'representation-preserving' },
  }

  for (let index = 0; index < count; index++) {
    const operationBase = operations.length
    const occurrenceBase = occurrences.length
    const rootPath = [{ kind: 'call', name: 'difference', ordinal: index }]
    const paths = [
      rootPath,
      [...rootPath, { kind: 'body', name: '$body', ordinal: 0 }],
      [...rootPath, { kind: 'body', name: '$body', ordinal: 0 }, { kind: 'control', name: 'let', ordinal: 0 }],
      [...rootPath, { kind: 'body', name: '$body', ordinal: 0 }, { kind: 'call', name: 'cube', ordinal: 0 }],
    ]
    const operationIds = paths.map(referenceSemanticOperationIdV1)
    operations.push(
      { id: operationBase, operationId: operationIds[0], parent: null, childOrdinal: index, name: 'difference', category: 'boolean', structuralPath: paths[0], identityEvidence: 'same-name-positional', ambiguityGroup },
      { id: operationBase + 1, operationId: operationIds[1], parent: operationBase, childOrdinal: 0, name: '$body', category: 'control', structuralPath: paths[1], identityEvidence: 'structural-unique', ambiguityGroup: null },
      { id: operationBase + 2, operationId: operationIds[2], parent: operationBase + 1, childOrdinal: 0, name: 'let', category: 'control', structuralPath: paths[2], identityEvidence: 'structural-unique', ambiguityGroup: null },
      { id: operationBase + 3, operationId: operationIds[3], parent: operationBase + 1, childOrdinal: 0, name: 'cube', category: 'geometry', structuralPath: paths[3], identityEvidence: 'structural-unique', ambiguityGroup: null },
    )
    const differenceId = referenceSemanticOccurrenceIdV1(null, null, operationIds[0], [])
    const bodyId = referenceSemanticOccurrenceIdV1(differenceId, differenceId, operationIds[1], [])
    const letId = referenceSemanticOccurrenceIdV1(bodyId, bodyId, operationIds[2], [])
    const cubeId = referenceSemanticOccurrenceIdV1(bodyId, bodyId, operationIds[3], [])
    occurrences.push(
      { id: occurrenceBase, occurrenceId: differenceId, operation: operationBase, parent: null, staticParent: null, dynamicSlots: [], node: null, outputOrdinal: null, sceneEntityId: null },
      { id: occurrenceBase + 1, occurrenceId: bodyId, operation: operationBase + 1, parent: occurrenceBase, staticParent: occurrenceBase, dynamicSlots: [], node: null, outputOrdinal: null, sceneEntityId: null },
      { id: occurrenceBase + 2, occurrenceId: letId, operation: operationBase + 2, parent: occurrenceBase + 1, staticParent: occurrenceBase + 1, dynamicSlots: [], node: null, outputOrdinal: null, sceneEntityId: null },
      { id: occurrenceBase + 3, occurrenceId: cubeId, operation: operationBase + 3, parent: occurrenceBase + 1, staticParent: occurrenceBase + 1, dynamicSlots: [], node: index, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(cubeId, 0) },
    )
    nodes.push({ id: index, kind: 'box', valueType: clone(valueType), size: [1, 1, 1], center: false })
    effects.push({ tag: 'legacy-difference-cutters', ownerOccurrence: occurrenceBase, root: index })

    const offset = index * statement.length
    const letStart = offset + statement.indexOf('let')
    const cubeStart = offset + statement.indexOf('cube')
    provenance.push(
      { operation: operationBase, span: { start: offset, end: offset + 'difference'.length }, label: 'difference()' },
      { operation: operationBase + 1, span: { start: offset, end: offset + 'difference'.length }, label: '$body' },
      { operation: operationBase + 2, span: { start: letStart, end: letStart + 'let'.length }, label: 'let()' },
      { operation: operationBase + 3, span: { start: cubeStart, end: cubeStart + 'cube'.length }, label: 'cube()' },
    )
  }

  program.source = referenceSemanticSourceDescriptorV1(source)
  program.core.operations = operations
  program.core.occurrences = occurrences
  program.core.nodes = nodes
  program.core.execution = {
    version: 'semantic-execution-v2',
    evaluationOrder: nodes.map(node => node.id),
    discardedEffects: effects,
    terminal: null,
  }
  program.core.result = { tag: 'empty', type: 'never' }
  program.core.declaredCapabilities = []
  program.core.diagnosticTemplates = []
  program.provenance = provenance
  program.tessellationIntents = []
  program.diagnostics = []
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return { program, source }
}

function makeTerminalPrefixFixture(message = 'later'): { program: ReferenceSemanticProgramV1; source: string } {
  const source = `polyhedron(points=[[0,0,0],[1,0,0],[0,1,0],[0,0,1]],faces=[[0,1,2]]); assert(false, ${JSON.stringify(message)}) cube(1);`
  const detail = `Assertion 'false' failed: ${message}`
  const paths = [
    [{ kind: 'call', name: 'polyhedron', ordinal: 0 }],
    [{ kind: 'call', name: 'assert', ordinal: 0 }],
  ]
  const operationIds = paths.map(referenceSemanticOperationIdV1)
  const polyhedronOccurrenceId = referenceSemanticOccurrenceIdV1(null, null, operationIds[0], [])
  const assertOccurrenceId = referenceSemanticOccurrenceIdV1(null, null, operationIds[1], [])
  const program = makeReferenceSemanticProgramV1()
  program.source = referenceSemanticSourceDescriptorV1(source)
  program.core.operations = [
    { id: 0, operationId: operationIds[0], parent: null, childOrdinal: 0, name: 'polyhedron', category: 'geometry', structuralPath: paths[0], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 1, operationId: operationIds[1], parent: null, childOrdinal: 0, name: 'assert', category: 'assertion', structuralPath: paths[1], identityEvidence: 'structural-unique', ambiguityGroup: null },
  ]
  program.core.nodes = [{
    id: 0,
    kind: 'polyhedron',
    valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } },
    vertices: [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]],
    triangles: [[0, 1, 2]],
  }]
  program.core.occurrences = [
    { id: 0, occurrenceId: polyhedronOccurrenceId, operation: 0, parent: null, staticParent: null, dynamicSlots: [], node: 0, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(polyhedronOccurrenceId, 0) },
    { id: 1, occurrenceId: assertOccurrenceId, operation: 1, parent: null, staticParent: null, dynamicSlots: [], node: null, outputOrdinal: null, sceneEntityId: null },
  ]
  program.core.execution = {
    version: 'semantic-execution-v2',
    evaluationOrder: [0],
    discardedEffects: [],
    terminal: {
      tag: 'legacy-language-error',
      occurrence: 1,
      diagnosticTemplate: 0,
      prefixFrontier: [{ root: 0, ownerOccurrence: 0 }],
    },
  }
  program.core.result = { tag: 'empty', type: 'never' }
  program.core.diagnosticTemplates = [{
    id: 0,
    code: 'LEGACY_LANGUAGE_ERROR',
    severity: 'error',
    operation: 1,
    arguments: [
      { name: 'errorName', value: { tag: 'string', value: 'OpenSCADParseError' } },
      { name: 'detailSha256', value: { tag: 'string', value: sha256(new TextEncoder().encode(detail)) } },
    ],
  }]
  program.provenance = [
    { operation: 0, span: { start: 0, end: source.indexOf(';') + 1 }, label: 'polyhedron()' },
    { operation: 1, span: { start: source.indexOf('assert'), end: source.length }, label: 'assert()' },
  ]
  program.tessellationIntents = []
  program.diagnostics = [{
    template: 0,
    message,
    span: { start: source.indexOf('assert'), end: source.length },
  }]
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return { program, source }
}

function makeTerminalSiblingScheduleFixture(): { program: ReferenceSemanticProgramV1; source: string } {
  const source = 'cube(1); sphere(1); assert(false, "later") cube(2);'
  const detail = "Assertion 'false' failed: later"
  const paths = [
    [{ kind: 'call', name: 'cube', ordinal: 0 }],
    [{ kind: 'call', name: 'sphere', ordinal: 0 }],
    [{ kind: 'call', name: 'assert', ordinal: 0 }],
  ]
  const operationIds = paths.map(referenceSemanticOperationIdV1)
  const occurrenceIds = operationIds.map(operationId => (
    referenceSemanticOccurrenceIdV1(null, null, operationId, [])
  ))
  const program = makeReferenceSemanticProgramV1()
  program.source = referenceSemanticSourceDescriptorV1(source)
  program.core.operations = [
    { id: 0, operationId: operationIds[0], parent: null, childOrdinal: 0, name: 'cube', category: 'geometry', structuralPath: paths[0], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 1, operationId: operationIds[1], parent: null, childOrdinal: 0, name: 'sphere', category: 'geometry', structuralPath: paths[1], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 2, operationId: operationIds[2], parent: null, childOrdinal: 0, name: 'assert', category: 'assertion', structuralPath: paths[2], identityEvidence: 'structural-unique', ambiguityGroup: null },
  ]
  program.core.nodes = [
    { id: 0, kind: 'box', valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, size: [1, 1, 1], center: false },
    { id: 1, kind: 'sphere-polygonal', valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, radius: 1, radialSegments: 16 },
  ]
  program.core.occurrences = [
    { id: 0, occurrenceId: occurrenceIds[0], operation: 0, parent: null, staticParent: null, dynamicSlots: [], node: 0, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceIds[0], 0) },
    { id: 1, occurrenceId: occurrenceIds[1], operation: 1, parent: null, staticParent: null, dynamicSlots: [], node: 1, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceIds[1], 0) },
    { id: 2, occurrenceId: occurrenceIds[2], operation: 2, parent: null, staticParent: null, dynamicSlots: [], node: null, outputOrdinal: null, sceneEntityId: null },
  ]
  program.core.execution = {
    version: 'semantic-execution-v2',
    evaluationOrder: [0, 1],
    discardedEffects: [],
    terminal: {
      tag: 'legacy-language-error',
      occurrence: 2,
      diagnosticTemplate: 0,
      prefixFrontier: [{ root: 0, ownerOccurrence: 0 }, { root: 1, ownerOccurrence: 1 }],
    },
  }
  program.core.result = { tag: 'empty', type: 'never' }
  program.core.declaredCapabilities = []
  program.core.diagnosticTemplates = [{
    id: 0,
    code: 'LEGACY_LANGUAGE_ERROR',
    severity: 'error',
    operation: 2,
    arguments: [
      { name: 'errorName', value: { tag: 'string', value: 'OpenSCADParseError' } },
      { name: 'detailSha256', value: { tag: 'string', value: sha256(new TextEncoder().encode(detail)) } },
    ],
  }]
  program.provenance = [
    { operation: 0, span: { start: source.indexOf('cube'), end: source.indexOf('cube') + 4 }, label: 'cube()' },
    { operation: 1, span: { start: source.indexOf('sphere'), end: source.indexOf('sphere') + 6 }, label: 'sphere()' },
    { operation: 2, span: { start: source.indexOf('assert'), end: source.length }, label: 'assert()' },
  ]
  program.tessellationIntents = []
  program.diagnostics = [{ template: 0, message: detail, span: { start: source.indexOf('assert'), end: source.indexOf('assert') + 1 } }]
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return { program, source }
}

type PostChildTerminalKind = 'difference-mix' | 'union-mix' | 'linear-extrude-solid' | 'partial-rotate'

function makePostChildTerminalFixture(
  kind: PostChildTerminalKind,
): { program: ReferenceSemanticProgramV1; source: string } {
  const specification = (() => {
    switch (kind) {
      case 'difference-mix': return {
        source: 'difference(){cube(1);square(1);}',
        rootName: 'difference', rootCategory: 'boolean', children: ['cube', 'square'],
        detail: 'difference() cannot mix 2D and 3D children', partialRoot: false,
      }
      case 'union-mix': return {
        source: 'union(){cube(1);square(1);}',
        rootName: 'union', rootCategory: 'boolean', children: ['cube', 'square'],
        detail: 'union() cannot mix 2D and 3D children', partialRoot: false,
      }
      case 'linear-extrude-solid': return {
        source: 'linear_extrude(1) cube(1);',
        rootName: 'linear_extrude', rootCategory: 'geometry', children: ['cube'],
        detail: 'linear_extrude() requires 2D children', partialRoot: false,
      }
      case 'partial-rotate': return {
        source: 'rotate(a=1,v=[0,0,0]){square(1);cube(1);}',
        rootName: 'rotate', rootCategory: 'transform', children: ['square', 'cube'],
        detail: 'Rotation axis cannot be zero', partialRoot: true,
      }
    }
  })()
  const rootPath = [{ kind: 'call', name: specification.rootName, ordinal: 0 }]
  const bodyPath = [...rootPath, { kind: 'body', name: '$body', ordinal: 0 }]
  const paths = [
    rootPath,
    bodyPath,
    ...specification.children.map(name => [...bodyPath, { kind: 'call', name, ordinal: 0 }]),
  ]
  const operationIds = paths.map(referenceSemanticOperationIdV1)
  const occurrenceIds = [referenceSemanticOccurrenceIdV1(null, null, operationIds[0], []), '', '']
  occurrenceIds[1] = referenceSemanticOccurrenceIdV1(occurrenceIds[0], occurrenceIds[0], operationIds[1], [])
  for (let index = 0; index < specification.children.length; index++) {
    occurrenceIds[index + 2] = referenceSemanticOccurrenceIdV1(
      occurrenceIds[1],
      occurrenceIds[1],
      operationIds[index + 2],
      [],
    )
  }
  const meshEvidence = { tag: 'representation-preserving' }
  const regionType = { geometryKind: 'region', space: 'd2', representation: 'mesh', evidence: meshEvidence }
  const solidType = { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: meshEvidence }
  const childNodes = specification.children.map((name, id) => name === 'cube'
    ? { id, kind: 'box', valueType: clone(solidType), size: [1, 1, 1], center: false }
    : { id, kind: 'rectangle', valueType: clone(regionType), size: [1, 1], center: false })
  const nodes: Record<string, any>[] = [...childNodes]
  if (specification.partialRoot) {
    nodes.push({
      id: 2,
      kind: 'transform',
      valueType: clone(regionType),
      input: 0,
      matrix: [
        0.9998476951563913, 0.01745240643728351, 0, 0,
        -0.01745240643728351, 0.9998476951563913, 0, 0,
        0, 0, 1, 0,
        0, 0, 0, 1,
      ],
    })
  }
  const rootNode = specification.partialRoot ? 2 : null
  const occurrences: Record<string, any>[] = [
    {
      id: 0, occurrenceId: occurrenceIds[0], operation: 0, parent: null, staticParent: null,
      dynamicSlots: [], node: rootNode, outputOrdinal: rootNode === null ? null : 0,
      sceneEntityId: rootNode === null ? null : referenceSemanticSceneEntityIdV1(occurrenceIds[0], 0),
    },
    {
      id: 1, occurrenceId: occurrenceIds[1], operation: 1, parent: 0, staticParent: 0,
      dynamicSlots: [], node: null, outputOrdinal: null, sceneEntityId: null,
    },
    ...specification.children.map((_, index) => ({
      id: index + 2,
      occurrenceId: occurrenceIds[index + 2],
      operation: index + 2,
      parent: 1,
      staticParent: 1,
      dynamicSlots: [],
      node: index,
      outputOrdinal: 0,
      sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceIds[index + 2], 0),
    })),
  ]
  const prefixFrontier = specification.partialRoot
    ? [{ root: 1, ownerOccurrence: 3 }, { root: 2, ownerOccurrence: 0 }]
    : childNodes.map((node, index) => ({ root: node.id, ownerOccurrence: index + 2 }))
  const program = makeReferenceSemanticProgramV1()
  program.source = referenceSemanticSourceDescriptorV1(specification.source)
  program.core.operations = [
    { id: 0, operationId: operationIds[0], parent: null, childOrdinal: 0, name: specification.rootName, category: specification.rootCategory, structuralPath: paths[0], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 1, operationId: operationIds[1], parent: 0, childOrdinal: 0, name: '$body', category: 'control', structuralPath: paths[1], identityEvidence: 'structural-unique', ambiguityGroup: null },
    ...specification.children.map((name, index) => ({
      id: index + 2,
      operationId: operationIds[index + 2],
      parent: 1,
      childOrdinal: 0,
      name,
      category: 'geometry',
      structuralPath: paths[index + 2],
      identityEvidence: 'structural-unique',
      ambiguityGroup: null,
    })),
  ]
  program.core.occurrences = occurrences
  program.core.nodes = nodes
  program.core.execution = {
    version: 'semantic-execution-v2',
    evaluationOrder: nodes.map(node => node.id),
    discardedEffects: [],
    terminal: {
      tag: 'legacy-language-error',
      occurrence: 0,
      diagnosticTemplate: 0,
      prefixFrontier,
    },
  }
  program.core.result = { tag: 'empty', type: 'never' }
  program.core.declaredCapabilities = []
  program.core.diagnosticTemplates = [{
    id: 0,
    code: 'LEGACY_LANGUAGE_ERROR',
    severity: 'error',
    operation: 0,
    arguments: [
      { name: 'errorName', value: { tag: 'string', value: 'OpenSCADParseError' } },
      { name: 'detailSha256', value: { tag: 'string', value: sha256(new TextEncoder().encode(specification.detail)) } },
    ],
  }]
  program.provenance = program.core.operations.map((operation: Record<string, any>) => {
    const token = operation.name === '$body' ? specification.rootName : operation.name
    const start = specification.source.indexOf(token)
    return { operation: operation.id, span: { start, end: start + token.length }, label: `${operation.name}()` }
  })
  program.tessellationIntents = []
  program.diagnostics = [{ template: 0, message: specification.detail, span: { start: 0, end: 1 } }]
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return { program, source: specification.source }
}

function makeAncestorPostChildTerminalFixture(): { program: ReferenceSemanticProgramV1; source: string } {
  const source = 'translate([1,0,0]) union(){cube(1);square(1);}'
  const detail = 'union() cannot mix 2D and 3D children'
  const paths = [
    [{ kind: 'call', name: 'translate', ordinal: 0 }],
    [{ kind: 'call', name: 'translate', ordinal: 0 }, { kind: 'body', name: '$body', ordinal: 0 }],
    [{ kind: 'call', name: 'translate', ordinal: 0 }, { kind: 'body', name: '$body', ordinal: 0 }, { kind: 'call', name: 'union', ordinal: 0 }],
    [{ kind: 'call', name: 'translate', ordinal: 0 }, { kind: 'body', name: '$body', ordinal: 0 }, { kind: 'call', name: 'union', ordinal: 0 }, { kind: 'body', name: '$body', ordinal: 0 }],
    [{ kind: 'call', name: 'translate', ordinal: 0 }, { kind: 'body', name: '$body', ordinal: 0 }, { kind: 'call', name: 'union', ordinal: 0 }, { kind: 'body', name: '$body', ordinal: 0 }, { kind: 'call', name: 'cube', ordinal: 0 }],
    [{ kind: 'call', name: 'translate', ordinal: 0 }, { kind: 'body', name: '$body', ordinal: 0 }, { kind: 'call', name: 'union', ordinal: 0 }, { kind: 'body', name: '$body', ordinal: 0 }, { kind: 'call', name: 'square', ordinal: 0 }],
  ]
  const operationIds = paths.map(referenceSemanticOperationIdV1)
  const parents: Array<number | null> = [null, 0, 1, 2, 3, 3]
  const occurrenceIds: string[] = []
  for (let index = 0; index < paths.length; index++) {
    const parent = parents[index]
    occurrenceIds.push(referenceSemanticOccurrenceIdV1(
      parent === null ? null : occurrenceIds[parent],
      parent === null ? null : occurrenceIds[parent],
      operationIds[index],
      [],
    ))
  }
  const program = makeReferenceSemanticProgramV1()
  program.source = referenceSemanticSourceDescriptorV1(source)
  const names = ['translate', '$body', 'union', '$body', 'cube', 'square']
  const categories = ['transform', 'control', 'boolean', 'control', 'geometry', 'geometry']
  program.core.operations = names.map((name, id) => ({
    id,
    operationId: operationIds[id],
    parent: parents[id],
    childOrdinal: 0,
    name,
    category: categories[id],
    structuralPath: paths[id],
    identityEvidence: 'structural-unique',
    ambiguityGroup: null,
  }))
  program.core.occurrences = names.map((_, id) => ({
    id,
    occurrenceId: occurrenceIds[id],
    operation: id,
    parent: parents[id],
    staticParent: parents[id],
    dynamicSlots: [],
    node: id === 4 ? 0 : id === 5 ? 1 : null,
    outputOrdinal: id >= 4 ? 0 : null,
    sceneEntityId: id >= 4 ? referenceSemanticSceneEntityIdV1(occurrenceIds[id], 0) : null,
  }))
  program.core.nodes = [
    { id: 0, kind: 'box', valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, size: [1, 1, 1], center: false },
    { id: 1, kind: 'rectangle', valueType: { geometryKind: 'region', space: 'd2', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, size: [1, 1], center: false },
  ]
  program.core.execution = {
    version: 'semantic-execution-v2',
    evaluationOrder: [0, 1],
    discardedEffects: [],
    terminal: {
      tag: 'legacy-language-error',
      occurrence: 2,
      diagnosticTemplate: 0,
      prefixFrontier: [{ root: 0, ownerOccurrence: 4 }, { root: 1, ownerOccurrence: 5 }],
    },
  }
  program.core.result = { tag: 'empty', type: 'never' }
  program.core.declaredCapabilities = []
  program.core.diagnosticTemplates = [{
    id: 0,
    code: 'LEGACY_LANGUAGE_ERROR',
    severity: 'error',
    operation: 2,
    arguments: [
      { name: 'errorName', value: { tag: 'string', value: 'OpenSCADParseError' } },
      { name: 'detailSha256', value: { tag: 'string', value: sha256(new TextEncoder().encode(detail)) } },
    ],
  }]
  program.provenance = names.map((name, operation) => {
    const token = name === '$body' ? (operation === 1 ? 'translate' : 'union') : name
    const start = source.indexOf(token)
    return { operation, span: { start, end: start + token.length }, label: `${name}()` }
  })
  program.tessellationIntents = []
  program.diagnostics = [{ template: 0, message: detail, span: { start: source.indexOf('union'), end: source.indexOf('union') + 1 } }]
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return { program, source }
}

function makeActiveAncestorDifferenceTerminalFixture(): {
  program: ReferenceSemanticProgramV1
  source: string
} {
  const source = 'difference(){group(){cube(1);sphere(1);} assert(false,"x") cube(1);}'
  const detail = "Assertion 'false' failed: x"
  const rootPath = [{ kind: 'call', name: 'difference', ordinal: 0 }]
  const differenceBody = [...rootPath, { kind: 'body', name: '$body', ordinal: 0 }]
  const groupPath = [...differenceBody, { kind: 'control', name: 'group', ordinal: 0 }]
  const groupBody = [...groupPath, { kind: 'body', name: '$body', ordinal: 0 }]
  const assertPath = [...differenceBody, { kind: 'call', name: 'assert', ordinal: 0 }]
  const assertBody = [...assertPath, { kind: 'body', name: '$body', ordinal: 0 }]
  const paths = [
    rootPath,
    differenceBody,
    groupPath,
    groupBody,
    [...groupBody, { kind: 'call', name: 'cube', ordinal: 0 }],
    [...groupBody, { kind: 'call', name: 'sphere', ordinal: 0 }],
    assertPath,
    assertBody,
    [...assertBody, { kind: 'call', name: 'cube', ordinal: 0 }],
  ]
  const operationIds = paths.map(referenceSemanticOperationIdV1)
  const names = ['difference', '$body', 'group', '$body', 'cube', 'sphere', 'assert', '$body', 'cube']
  const categories = ['boolean', 'control', 'control', 'control', 'geometry', 'geometry', 'assertion', 'control', 'geometry']
  const parents: Array<number | null> = [null, 0, 1, 2, 3, 3, 1, 6, 7]
  const program = makeReferenceSemanticProgramV1()
  program.source = referenceSemanticSourceDescriptorV1(source)
  program.core.operations = names.map((name, id) => ({
    id,
    operationId: operationIds[id],
    parent: parents[id],
    childOrdinal: 0,
    name,
    category: categories[id],
    structuralPath: paths[id],
    identityEvidence: 'structural-unique',
    ambiguityGroup: null,
  }))

  const assertSlots = [
    { name: 'condition', value: { tag: 'boolean', value: false }, duplicateOrdinal: 0 },
    { name: 'message', value: { tag: 'string', value: 'x' }, duplicateOrdinal: 0 },
  ]
  const occurrenceIds: string[] = []
  for (let id = 0; id <= 6; id++) {
    const parent = parents[id]
    occurrenceIds.push(referenceSemanticOccurrenceIdV1(
      parent === null ? null : occurrenceIds[parent],
      parent === null ? null : occurrenceIds[parent],
      operationIds[id],
      id === 6 ? assertSlots : [],
    ))
  }
  program.core.occurrences = names.slice(0, 7).map((_, id) => {
    const node = id === 4 ? 0 : id === 5 ? 1 : null
    return {
      id,
      occurrenceId: occurrenceIds[id],
      operation: id,
      parent: parents[id],
      staticParent: parents[id],
      dynamicSlots: id === 6 ? clone(assertSlots) : [],
      node,
      outputOrdinal: node === null ? null : 0,
      sceneEntityId: node === null ? null : referenceSemanticSceneEntityIdV1(occurrenceIds[id], 0),
    }
  })
  const valueType = {
    geometryKind: 'solid-set',
    space: 'd3',
    representation: 'mesh',
    evidence: { tag: 'representation-preserving' },
  }
  program.core.nodes = [
    { id: 0, kind: 'box', valueType: clone(valueType), size: [1, 1, 1], center: false },
    { id: 1, kind: 'sphere-polygonal', valueType: clone(valueType), radius: 1, radialSegments: 16 },
    { id: 2, kind: 'boolean', operation: 'union', valueType: clone(valueType), inputs: [0, 1] },
  ]
  program.core.execution = {
    version: 'semantic-execution-v2',
    evaluationOrder: [0, 1, 2],
    discardedEffects: [],
    terminal: {
      tag: 'legacy-language-error',
      occurrence: 6,
      diagnosticTemplate: 0,
      prefixFrontier: [{ root: 2, ownerOccurrence: null }],
    },
  }
  program.core.result = { tag: 'empty', type: 'never' }
  program.core.declaredCapabilities = []
  program.core.diagnosticTemplates = [{
    id: 0,
    code: 'LEGACY_LANGUAGE_ERROR',
    severity: 'error',
    operation: 6,
    arguments: [
      { name: 'errorName', value: { tag: 'string', value: 'OpenSCADParseError' } },
      { name: 'detailSha256', value: { tag: 'string', value: sha256(new TextEncoder().encode(detail)) } },
    ],
  }]
  program.provenance = names.map((name, operation) => {
    const token = name === '$body'
      ? operation === 1 ? 'difference' : operation === 3 ? 'group' : 'assert'
      : name
    const start = operation === 8 ? source.lastIndexOf('cube') : source.indexOf(token)
    return { operation, span: { start, end: start + token.length }, label: `${name}()` }
  })
  program.tessellationIntents = []
  program.diagnostics = [{
    template: 0,
    message: detail,
    span: { start: source.indexOf('assert'), end: source.indexOf('assert') + 1 },
  }]
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return { program, source }
}

function makeDuplicateOrdinalFixture(): ReferenceSemanticProgramV1 {
  const program = makeReferenceSemanticProgramV1()
  const parentOperation = program.core.operations[2]
  const structuralPath = [
    ...clone(parentOperation.structuralPath),
    { kind: 'control', name: 'group', ordinal: 0 },
  ]
  const operationId = referenceSemanticOperationIdV1(structuralPath)
  program.core.operations.push({
    id: 5,
    operationId,
    parent: 2,
    childOrdinal: 0,
    name: 'group',
    category: 'control',
    structuralPath,
    identityEvidence: 'structural-unique',
    ambiguityGroup: null,
  })
  program.provenance.push({
    ...clone(program.provenance[0]),
    operation: 5,
    label: 'group()',
  })
  for (const duplicateOrdinal of [0, 1]) {
    const dynamicSlots = [{
      name: 'repeat',
      value: { tag: 'number', value: 7 },
      duplicateOrdinal,
    }]
    const parent = program.core.occurrences[2]
    const index = program.core.occurrences.length
    program.core.occurrences.push({
      id: index,
      occurrenceId: referenceSemanticOccurrenceIdV1(
        parent.occurrenceId,
        parent.occurrenceId,
        operationId,
        dynamicSlots,
      ),
      operation: 5,
      parent: 2,
      staticParent: 2,
      dynamicSlots,
      node: null,
      outputOrdinal: null,
      sceneEntityId: null,
    })
  }
  return program
}

function makeSiblingIdentityDonationFixture(): ReferenceSemanticProgramV1 {
  const program = makeReferenceSemanticProgramV1()
  const dynamicSlots = [{
    name: 'branch',
    value: { tag: 'string', value: 'sibling-producer' },
    duplicateOrdinal: 0,
  }]
  const parent = program.core.occurrences[0]
  program.core.occurrences.push({
    id: 5,
    occurrenceId: referenceSemanticOccurrenceIdV1(
      parent.occurrenceId,
      parent.occurrenceId,
      program.core.operations[1].operationId,
      dynamicSlots,
    ),
    operation: 1,
    parent: 0,
    staticParent: 0,
    dynamicSlots,
    node: 3,
    outputOrdinal: null,
    sceneEntityId: null,
  })
  program.core.result.item.producerOccurrence = 5
  return program
}

function makeSharedPrimitiveNodeFixture(): ReferenceSemanticProgramV1 {
  const program = makeReferenceSemanticProgramV1()
  const firstCube = program.core.operations[3]
  const path = [
    ...clone(program.core.operations[2].structuralPath),
    { kind: 'call', name: 'cube', ordinal: 1 },
  ]
  const operationId = referenceSemanticOperationIdV1(path)
  const ambiguityGroup = referenceSemanticAmbiguityGroupIdV1(
    firstCube.structuralPath.slice(0, -1),
    'geometry',
    'cube',
  )
  firstCube.identityEvidence = 'same-name-positional'
  firstCube.ambiguityGroup = ambiguityGroup
  program.core.operations.push({
    id: 5,
    operationId,
    parent: 2,
    childOrdinal: 1,
    name: 'cube',
    category: 'geometry',
    structuralPath: path,
    identityEvidence: 'same-name-positional',
    ambiguityGroup,
  })
  program.core.nodes[2].inputs = [0, 1, 0]
  const parent = program.core.occurrences[2]
  const occurrenceId = referenceSemanticOccurrenceIdV1(
    parent.occurrenceId,
    parent.occurrenceId,
    operationId,
    [],
  )
  program.core.occurrences.push({
    id: 5,
    occurrenceId,
    operation: 5,
    parent: 2,
    staticParent: 2,
    dynamicSlots: [],
    node: 0,
    outputOrdinal: 0,
    sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceId, 0),
  })
  program.provenance.push({
    ...clone(program.provenance[3]),
    operation: 5,
    label: 'cube() duplicate materializer',
  })
  return program
}

function makeLegacyLinearExtrudeFixture(scale: unknown): ReferenceSemanticProgramV1 {
  const program = makeReferenceSemanticProgramV1()
  program.core.nodes = [
    {
      id: 0,
      kind: 'rectangle',
      valueType: {
        geometryKind: 'region',
        space: 'd2',
        representation: 'mesh',
        evidence: { tag: 'representation-preserving' },
      },
      size: [2, 3],
      center: false,
    },
    {
      id: 1,
      kind: 'linear-extrude',
      valueType: {
        geometryKind: 'solid-set',
        space: 'd3',
        representation: 'mesh',
        evidence: { tag: 'representation-preserving' },
      },
      input: 0,
      height: 4,
      twistDegrees: 0,
      slices: 1,
      scale,
      center: false,
    },
  ]
  const paths = [
    [{ kind: 'module', name: 'scene', ordinal: 0 }],
    [{ kind: 'module', name: 'scene', ordinal: 0 }, { kind: 'call', name: 'linear_extrude', ordinal: 0 }],
    [{ kind: 'module', name: 'scene', ordinal: 0 }, { kind: 'call', name: 'linear_extrude', ordinal: 0 }, { kind: 'call', name: 'square', ordinal: 0 }],
  ]
  const ids = paths.map(referenceSemanticOperationIdV1)
  program.core.operations = [
    { id: 0, operationId: ids[0], parent: null, childOrdinal: 0, name: 'scene', category: 'module', structuralPath: paths[0], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 1, operationId: ids[1], parent: 0, childOrdinal: 0, name: 'linear_extrude', category: 'geometry', structuralPath: paths[1], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 2, operationId: ids[2], parent: 1, childOrdinal: 0, name: 'square', category: 'geometry', structuralPath: paths[2], identityEvidence: 'structural-unique', ambiguityGroup: null },
  ]
  const occurrenceIds = [
    referenceSemanticOccurrenceIdV1(null, null, ids[0], []),
    '',
    '',
  ]
  occurrenceIds[1] = referenceSemanticOccurrenceIdV1(occurrenceIds[0], occurrenceIds[0], ids[1], [])
  occurrenceIds[2] = referenceSemanticOccurrenceIdV1(occurrenceIds[1], occurrenceIds[1], ids[2], [])
  program.core.occurrences = [
    { id: 0, occurrenceId: occurrenceIds[0], operation: 0, parent: null, staticParent: null, dynamicSlots: [], node: null, outputOrdinal: null, sceneEntityId: null },
    { id: 1, occurrenceId: occurrenceIds[1], operation: 1, parent: 0, staticParent: 0, dynamicSlots: [], node: 1, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceIds[1], 0) },
    { id: 2, occurrenceId: occurrenceIds[2], operation: 2, parent: 1, staticParent: 1, dynamicSlots: [], node: 0, outputOrdinal: 0, sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceIds[2], 0) },
  ]
  program.core.result = {
    tag: 'single',
    item: {
      node: 1,
      producerOccurrence: 1,
      identityOccurrence: 1,
      color: [1, 1, 1, 1],
    },
  }
  program.core.diagnosticTemplates = []
  program.provenance = program.provenance.slice(0, 3).map((entry: Record<string, any>, index: number) => ({
    ...entry,
    operation: index,
    label: `${program.core.operations[index].name}()`,
  }))
  program.diagnostics = []
  resetSuccessfulExecution(program)
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return program
}

function makePartitionedDifferenceFixture(): ReferenceSemanticProgramV1 {
  const program = makeReferenceSemanticProgramV1()
  const root = [{ kind: 'call', name: 'difference', ordinal: 0 }]
  const body = [...root, { kind: 'body', name: '$body', ordinal: 0 }]
  const baseGroup = [...body, { kind: 'control', name: 'group', ordinal: 0 }]
  const baseBody = [...baseGroup, { kind: 'body', name: '$body', ordinal: 0 }]
  const cutterGroup = [...body, { kind: 'control', name: 'group', ordinal: 1 }]
  const cutterBody = [...cutterGroup, { kind: 'body', name: '$body', ordinal: 0 }]
  const paths = [
    root,
    body,
    baseGroup,
    baseBody,
    [...baseBody, { kind: 'call', name: 'cube', ordinal: 0 }],
    [...baseBody, { kind: 'call', name: 'sphere', ordinal: 0 }],
    cutterGroup,
    cutterBody,
    [...cutterBody, { kind: 'call', name: 'cube', ordinal: 0 }],
    [...cutterBody, { kind: 'call', name: 'sphere', ordinal: 0 }],
  ]
  const operationIds = paths.map(referenceSemanticOperationIdV1)
  const groupAmbiguity = referenceSemanticAmbiguityGroupIdV1(body, 'control', 'group')
  const specifications = [
    { parent: null, childOrdinal: 0, name: 'difference', category: 'boolean', identityEvidence: 'structural-unique', ambiguityGroup: null },
    { parent: 0, childOrdinal: 0, name: '$body', category: 'control', identityEvidence: 'structural-unique', ambiguityGroup: null },
    { parent: 1, childOrdinal: 0, name: 'group', category: 'control', identityEvidence: 'same-name-positional', ambiguityGroup: groupAmbiguity },
    { parent: 2, childOrdinal: 0, name: '$body', category: 'control', identityEvidence: 'structural-unique', ambiguityGroup: null },
    { parent: 3, childOrdinal: 0, name: 'cube', category: 'geometry', identityEvidence: 'structural-unique', ambiguityGroup: null },
    { parent: 3, childOrdinal: 0, name: 'sphere', category: 'geometry', identityEvidence: 'structural-unique', ambiguityGroup: null },
    { parent: 1, childOrdinal: 1, name: 'group', category: 'control', identityEvidence: 'same-name-positional', ambiguityGroup: groupAmbiguity },
    { parent: 6, childOrdinal: 0, name: '$body', category: 'control', identityEvidence: 'structural-unique', ambiguityGroup: null },
    { parent: 7, childOrdinal: 0, name: 'cube', category: 'geometry', identityEvidence: 'structural-unique', ambiguityGroup: null },
    { parent: 7, childOrdinal: 0, name: 'sphere', category: 'geometry', identityEvidence: 'structural-unique', ambiguityGroup: null },
  ]
  program.core.operations = specifications.map((specification, id) => ({
    id,
    operationId: operationIds[id],
    ...specification,
    structuralPath: paths[id],
  }))
  const valueType = {
    geometryKind: 'solid-set',
    space: 'd3',
    representation: 'mesh',
    evidence: { tag: 'representation-preserving' },
  }
  program.core.nodes = [
    { id: 0, kind: 'box', valueType: clone(valueType), size: [1, 1, 1], center: false },
    { id: 1, kind: 'sphere-polygonal', valueType: clone(valueType), radius: 1, radialSegments: 16 },
    { id: 2, kind: 'boolean', valueType: clone(valueType), operation: 'union', inputs: [0, 1] },
    { id: 3, kind: 'box', valueType: clone(valueType), size: [0.5, 0.5, 0.5], center: false },
    { id: 4, kind: 'sphere-polygonal', valueType: clone(valueType), radius: 0.5, radialSegments: 16 },
    { id: 5, kind: 'boolean', valueType: clone(valueType), operation: 'union', inputs: [3, 4] },
    { id: 6, kind: 'boolean', valueType: clone(valueType), operation: 'difference', inputs: [2, 5] },
  ]
  const parentRows: Array<number | null> = [null, 0, 1, 2, 3, 3, 1, 6, 7, 7]
  const nodeRows: Array<number | null> = [6, null, null, null, 0, 1, null, null, 3, 4]
  const occurrenceIds: string[] = []
  program.core.occurrences = specifications.map((_, id) => {
    const parent = parentRows[id]
    const parentId = parent === null ? null : occurrenceIds[parent]
    const occurrenceId = referenceSemanticOccurrenceIdV1(
      parentId,
      parentId,
      operationIds[id],
      [],
    )
    occurrenceIds.push(occurrenceId)
    const node = nodeRows[id]
    return {
      id,
      occurrenceId,
      operation: id,
      parent,
      staticParent: parent,
      dynamicSlots: [],
      node,
      outputOrdinal: node === null ? null : 0,
      sceneEntityId: node === null ? null : referenceSemanticSceneEntityIdV1(occurrenceId, 0),
    }
  })
  program.core.result = {
    tag: 'single',
    item: { node: 6, producerOccurrence: 0, identityOccurrence: 0, color: [1, 1, 1, 1] },
  }
  program.core.diagnosticTemplates = []
  program.provenance = specifications.map((specification, operation) => ({
    operation,
    span: clone(program.provenance[0].span),
    label: `${specification.name}()`,
  }))
  program.diagnostics = []
  resetSuccessfulExecution(program)
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return program
}

function makeChildrenContinuationFixture(): ReferenceSemanticProgramV1 {
  const program = makeReferenceSemanticProgramV1()
  const definition = [{ kind: 'module', name: 'wrap', ordinal: 0 }]
  const definitionBody = [...definition, { kind: 'body', name: '$body', ordinal: 0 }]
  const translate = [...definitionBody, { kind: 'call', name: 'translate', ordinal: 0 }]
  const translateBody = [...translate, { kind: 'body', name: '$body', ordinal: 0 }]
  const call = [{ kind: 'call', name: 'wrap', ordinal: 1 }]
  const callerBody = [...call, { kind: 'body', name: '$body', ordinal: 0 }]
  const expansion = [...callerBody, { kind: 'control', name: '$expansion', ordinal: 0 }]
  const paths = [
    definition,
    definitionBody,
    translate,
    translateBody,
    [...translateBody, { kind: 'control', name: 'children', ordinal: 0 }],
    call,
    callerBody,
    expansion,
    [...expansion, { kind: 'call', name: 'cube', ordinal: 0 }],
  ]
  const operationIds = paths.map(referenceSemanticOperationIdV1)
  const wrapAmbiguity = referenceSemanticAmbiguityGroupIdV1([], 'module', 'wrap')
  const specifications = [
    { parent: null, childOrdinal: 0, name: 'wrap', category: 'module', identityEvidence: 'same-name-positional', ambiguityGroup: wrapAmbiguity },
    { parent: 0, childOrdinal: 0, name: '$body', category: 'control', identityEvidence: 'structural-unique', ambiguityGroup: null },
    { parent: 1, childOrdinal: 0, name: 'translate', category: 'transform', identityEvidence: 'structural-unique', ambiguityGroup: null },
    { parent: 2, childOrdinal: 0, name: '$body', category: 'control', identityEvidence: 'structural-unique', ambiguityGroup: null },
    { parent: 3, childOrdinal: 0, name: 'children', category: 'control', identityEvidence: 'structural-unique', ambiguityGroup: null },
    { parent: null, childOrdinal: 1, name: 'wrap', category: 'module', identityEvidence: 'same-name-positional', ambiguityGroup: wrapAmbiguity },
    { parent: 5, childOrdinal: 0, name: '$body', category: 'control', identityEvidence: 'structural-unique', ambiguityGroup: null },
    { parent: 6, childOrdinal: 0, name: '$expansion', category: 'control', identityEvidence: 'structural-unique', ambiguityGroup: null },
    { parent: 7, childOrdinal: 0, name: 'cube', category: 'geometry', identityEvidence: 'structural-unique', ambiguityGroup: null },
  ]
  program.core.operations = specifications.map((specification, id) => ({
    id,
    operationId: operationIds[id],
    ...specification,
    structuralPath: paths[id],
  }))
  const parameterSlots = [{
    name: 'x',
    value: { tag: 'number', value: 1 },
    duplicateOrdinal: 0,
  }]
  const evaluationSlots = [{
    name: '$evaluation',
    value: { tag: 'undefined' },
    duplicateOrdinal: 0,
  }]
  const indexSlots = [{
    name: '$index',
    value: { tag: 'undefined' },
    duplicateOrdinal: 0,
  }]
  const parentRows: Array<number | null> = [null, 0, 1, 2, 3, 4, 5, 6, 7]
  const staticRows: Array<number | null> = [null, 0, null, 2, 3, 4, 5, 1, 7]
  const slots = [
    parameterSlots,
    evaluationSlots,
    parameterSlots,
    evaluationSlots,
    evaluationSlots,
    evaluationSlots,
    indexSlots,
    indexSlots,
    evaluationSlots,
  ]
  const nodeRows: Array<number | null> = [null, null, null, null, 1, null, null, null, 0]
  const runtimeOperations = [5, 6, 0, 1, 2, 3, 4, 7, 8]
  const occurrenceIds: string[] = []
  program.core.occurrences = specifications.map((_, id) => {
    const parent = parentRows[id]
    const staticParent = staticRows[id]
    const operation = runtimeOperations[id]
    const occurrenceId = referenceSemanticOccurrenceIdV1(
      parent === null ? null : occurrenceIds[parent],
      staticParent === null ? null : occurrenceIds[staticParent],
      operationIds[operation],
      slots[id],
    )
    occurrenceIds.push(occurrenceId)
    const node = nodeRows[id]
    return {
      id,
      occurrenceId,
      operation,
      parent,
      staticParent,
      dynamicSlots: clone(slots[id]),
      node,
      outputOrdinal: node === null ? null : 0,
      sceneEntityId: node === null ? null : referenceSemanticSceneEntityIdV1(occurrenceId, 0),
    }
  })
  const valueType = {
    geometryKind: 'solid-set',
    space: 'd3',
    representation: 'mesh',
    evidence: { tag: 'representation-preserving' },
  }
  program.core.nodes = [
    { id: 0, kind: 'box', valueType: clone(valueType), size: [1, 1, 1], center: false },
    { id: 1, kind: 'transform', valueType: clone(valueType), input: 0, matrix: [
      1, 0, 0, 0,
      0, 1, 0, 0,
      0, 0, 1, 0,
      1, 0, 0, 1,
    ] },
  ]
  program.core.result = {
    tag: 'single',
    item: { node: 1, producerOccurrence: 4, identityOccurrence: 8, color: [1, 1, 1, 1] },
  }
  program.core.diagnosticTemplates = []
  program.provenance = specifications.map((specification, operation) => ({
    operation,
    span: clone(program.provenance[0].span),
    label: `${specification.name}()`,
  }))
  program.diagnostics = []
  resetSuccessfulExecution(program)
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return program
}

function appendRootOperation(
  program: ReferenceSemanticProgramV1,
  kind: 'call' | 'module' | 'control' | 'branch' | 'body',
  name: string,
  category: string,
): number {
  const operation = program.core.operations.length
  const structuralPath = [{ kind, name, ordinal: 0 }]
  program.core.operations.push({
    id: operation,
    operationId: referenceSemanticOperationIdV1(structuralPath),
    parent: null,
    childOrdinal: 0,
    name,
    category,
    structuralPath,
    identityEvidence: 'structural-unique',
    ambiguityGroup: null,
  })
  program.provenance.push({
    operation,
    span: clone(program.provenance[0].span),
    label: `${name}()`,
  })
  return operation
}

type Mutation = {
  name: string
  family: ReferenceSemanticFailureFamily
  mutate(program: ReferenceSemanticProgramV1): void
}

const PRODUCTION_CODE_BY_FAMILY: Readonly<Record<ReferenceSemanticFailureFamily, string>> = {
  schema: 'E_SEMANTIC_SCHEMA',
  field: 'E_SEMANTIC_FIELD',
  version: 'E_SEMANTIC_VERSION',
  limit: 'E_SEMANTIC_LIMIT',
  number: 'E_SEMANTIC_NUMBER',
  reference: 'E_SEMANTIC_REFERENCE',
  type: 'E_SEMANTIC_TYPE',
  graph: 'E_SEMANTIC_DAG',
  identity: 'E_SEMANTIC_IDENTITY',
  source: 'E_SEMANTIC_PROVENANCE',
  closure: 'E_SEMANTIC_CAPABILITY_CLOSURE',
  order: 'E_SEMANTIC_ORDER',
  binary: 'E_SEMANTIC_BYTES',
}

const mutations: Mutation[] = [
  {
    name: 'unknown envelope field',
    family: 'field',
    mutate: program => { program.engine = 'forged' },
  },
  {
    name: 'unknown node field',
    family: 'field',
    mutate: program => { program.core.nodes[0].provider = 'forged' },
  },
  {
    name: 'accessor field at the trust boundary',
    family: 'field',
    mutate: program => {
      Object.defineProperty(program.core.nodes[0], 'size', {
        enumerable: true,
        get: () => { throw new Error('accessor must not execute') },
      })
    },
  },
  {
    name: 'proxy getter disagrees with its snapshotted data descriptor',
    family: 'field',
    mutate: program => {
      const target = { tag: 'forged' }
      program.core.nodes[0].valueType.evidence = new Proxy(target, {
        get: (object, property, receiver) => property === 'tag'
          ? 'representation-preserving'
          : Reflect.get(object, property, receiver),
      })
    },
  },
  {
    name: 'hidden field at the trust boundary',
    family: 'field',
    mutate: program => {
      Object.defineProperty(program.core.nodes[0], 'hidden', {
        enumerable: false,
        value: true,
      })
    },
  },
  {
    name: 'non-plain object at the trust boundary',
    family: 'schema',
    mutate: program => { Object.setPrototypeOf(program.core.nodes[0], { forged: true }) },
  },
  {
    name: 'legacy dimension field smuggled beside valueType',
    family: 'field',
    mutate: program => { program.core.nodes[0].dimension = 'solid3' },
  },
  {
    name: 'valueType is missing its evidence field',
    family: 'field',
    mutate: program => { delete program.core.nodes[0].valueType.evidence },
  },
  {
    name: 'unknown geometry kind',
    family: 'type',
    mutate: program => { program.core.nodes[0].valueType.geometryKind = 'volume' },
  },
  {
    name: 'geometry kind and space are incompatible',
    family: 'type',
    mutate: program => { program.core.nodes[0].valueType.space = 'd2' },
  },
  {
    name: 'primitive representation disagrees with its language contract',
    family: 'type',
    mutate: program => { program.core.nodes[0].valueType.representation = 'analytic-brep' },
  },
  {
    name: 'unknown representation evidence tag',
    family: 'field',
    mutate: program => { program.core.nodes[0].valueType.evidence.tag = 'exact' },
  },
  {
    name: 'representation evidence has no string discriminant',
    family: 'schema',
    mutate: program => { program.core.nodes[0].valueType.evidence = {} },
  },
  {
    name: 'certified representation claims preserving evidence',
    family: 'type',
    mutate: program => { program.core.nodes[0].valueType.representation = 'certified-approx-brep' },
  },
  {
    name: 'certified evidence has a noncanonical policy hash',
    family: 'field',
    mutate: program => {
      program.core.nodes[0].valueType.representation = 'certified-approx-brep'
      program.core.nodes[0].valueType.evidence = {
        tag: 'certified-approximation',
        certificateProfile: 'oracle-v1',
        certificatePolicyHash: 'A'.repeat(64),
      }
    },
  },
  {
    name: 'certified evidence profile exceeds its frozen 96-character bound',
    family: 'limit',
    mutate: program => {
      program.core.nodes[0].valueType.representation = 'certified-approx-brep'
      program.core.nodes[0].valueType.evidence = {
        tag: 'certified-approximation',
        certificateProfile: 'p'.repeat(97),
        certificatePolicyHash: '0'.repeat(64),
      }
    },
  },
  {
    name: 'transform changes an input value axis',
    family: 'type',
    mutate: program => { program.core.nodes[3].valueType.geometryKind = 'solid' },
  },
  {
    name: 'unknown result tag',
    family: 'field',
    mutate: program => { program.core.result.tag = 'collection' },
  },
  {
    name: 'result has no string discriminant',
    family: 'schema',
    mutate: program => { delete program.core.result.tag },
  },
  {
    name: 'unsupported core version',
    family: 'version',
    mutate: program => { program.core.schemaVersion.major = 2 },
  },
  {
    name: 'negative zero geometry',
    family: 'number',
    mutate: program => { program.core.nodes[0].size[0] = -0 },
  },
  {
    name: 'unknown node tag',
    family: 'field',
    mutate: program => { program.core.nodes[0].kind = 'mesh' },
  },
  {
    name: 'forward DAG edge',
    family: 'reference',
    mutate: program => { program.core.nodes[3].input = 3 },
  },
  {
    name: 'unreachable trailing node',
    family: 'graph',
    mutate: program => {
      program.core.result.item = {
        node: 2,
        producerOccurrence: 2,
        identityOccurrence: 2,
        color: [0.25, 0.5, 0.75, 1],
      }
    },
  },
  {
    name: 'forged static operation digest',
    family: 'identity',
    mutate: program => { program.core.operations[3].operationId = `opv1:${'0'.repeat(64)}` },
  },
  {
    name: 'forged dynamic occurrence digest',
    family: 'identity',
    mutate: program => { program.core.occurrences[4].occurrenceId = `occv1:${'0'.repeat(64)}` },
  },
  {
    name: 'forged scene identity digest',
    family: 'identity',
    mutate: program => { program.core.occurrences[3].sceneEntityId = `entity:v2:${'0'.repeat(64)}` },
  },
  {
    name: 'producer occurrence points at another node',
    family: 'identity',
    mutate: program => { program.core.result.item.producerOccurrence = 2 },
  },
  {
    name: 'identity occurrence has no scene identity',
    family: 'identity',
    mutate: program => { program.core.result.item.identityOccurrence = 1 },
  },
  {
    name: 'identity occurrence crosses a non-preserving Boolean node',
    family: 'identity',
    mutate: program => { program.core.result.item.identityOccurrence = 3 },
  },
  {
    name: 'unknown typed identity value',
    family: 'field',
    mutate: program => { program.core.occurrences[4].dynamicSlots[0].value.tag = 'scalar' },
  },
  {
    name: 'typed identity value has no string discriminant',
    family: 'schema',
    mutate: program => { delete program.core.occurrences[4].dynamicSlots[0].value.tag },
  },
  {
    name: 'same-name sibling ordinal has a gap',
    family: 'order',
    mutate: program => {
      const operation = program.core.operations[4]
      operation.childOrdinal = 1
      operation.structuralPath.at(-1).ordinal = 1
      operation.operationId = referenceSemanticOperationIdV1(operation.structuralPath)
      recomputeOccurrence(program, 4)
    },
  },
  {
    name: 'duplicate static structural identity',
    family: 'identity',
    mutate: program => {
      const first = program.core.operations[3]
      const duplicate = program.core.operations[4]
      const group = referenceSemanticAmbiguityGroupIdV1(
        first.structuralPath.slice(0, -1),
        first.category,
        first.name,
      )
      first.identityEvidence = 'same-name-positional'
      first.ambiguityGroup = group
      duplicate.name = first.name
      duplicate.childOrdinal = first.childOrdinal
      duplicate.structuralPath = clone(first.structuralPath)
      duplicate.operationId = first.operationId
      duplicate.identityEvidence = 'same-name-positional'
      duplicate.ambiguityGroup = group
      recomputeOccurrence(program, 4)
    },
  },
  {
    name: 'dynamic occurrence reparented away from static path',
    family: 'identity',
    mutate: program => {
      program.core.occurrences[4].parent = 1
      recomputeOccurrence(program, 4)
    },
  },
  {
    name: 'missing inferred capability',
    family: 'closure',
    mutate: program => { program.core.capabilityClosure.pop() },
  },
  {
    name: 'unsorted capability closure',
    family: 'order',
    mutate: program => {
      const [first, second] = program.core.capabilityClosure
      program.core.capabilityClosure[0] = second
      program.core.capabilityClosure[1] = first
    },
  },
  {
    name: 'legacy envelope carries tessellation policy',
    family: 'type',
    mutate: program => {
      program.tessellationIntents = [{
        occurrence: 4,
        chordTolerance: 0.05,
        angularToleranceDegrees: 5,
        minSegments: 8,
        maxSegments: 64,
      }]
    },
  },
  {
    name: 'source digest forgery',
    family: 'source',
    mutate: program => { program.source.sha256 = '0'.repeat(64) },
  },
  {
    name: 'source routing requirements disagree with core',
    family: 'source',
    mutate: program => {
      program.core.declaredCapabilities = ['geometry.mesh']
      program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
    },
  },
  {
    name: 'source span splits a surrogate pair',
    family: 'source',
    mutate: program => {
      program.provenance[0].span.start = REFERENCE_SEMANTIC_SOURCE_V1.indexOf('🙂') + 1
    },
  },
]

describe('independent SemanticProgram V1 oracle', () => {
  it('has no dependency edge into production source', () => {
    const oracleSource = readFileSync(
      new URL('./support/referenceSemanticProgramV1.ts', import.meta.url),
      'utf8',
    )
    const imports = oracleSource.match(/^\s*import\s+.*$/gm) ?? []
    expect(imports).toEqual(["import { createHash } from 'node:crypto'"])
    expect(oracleSource).not.toMatch(/(?:from\s*|import\s*\()['"][^'"]*src\//)
    expect(oracleSource).toContain('const WIRE_TAG')
    expect(oracleSource).toContain('const NODE_KEY')
    expect(oracleSource).toContain('const EVIDENCE_KEY')
    expect(oracleSource).toContain('const CAPABILITY_BY_KIND')
    expect(oracleSource).toContain("geometryKind: ['curve', 'wire', 'region', 'sheet', 'solid', 'solid-set']")
    expect(oracleSource).not.toContain("from '../src/")
  })

  it('pins independent SPE1/SPC1 golden bytes and the domain-separated program hash', () => {
    const program = makeReferenceSemanticProgramV1()
    expect(() => referenceValidateSemanticProgramV1(
      program,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()
    expect(() => normalizeSemanticProgram(
      program,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()

    const referenceEnvelope = referenceEncodeSemanticProgramV1(program)
    const referenceCore = referenceEncodeSemanticProgramCoreV1(program)
    const productionEnvelope = encodeSemanticProgram(program)
    const productionCore = encodeSemanticProgramCore(program)
    const tessellationPolicy = encodeSemanticTessellationPolicy(program)

    expect(program.core.capabilityClosure).toEqual(expect.arrayContaining([
      'geometry.kind.solid-set',
      'geometry.space.d3',
      'representation.mesh',
      'evidence.representation-preserving',
    ]))
    expect(program.core.capabilityClosure).not.toContain('geometry.solid3')

    expect(productionEnvelope).toEqual(referenceEnvelope)
    expect(productionCore).toEqual(referenceCore)
    expect(referenceEncodeSemanticProgramCoreV1(program.core)).toEqual(referenceCore)
    expect(encodeSemanticProgramCore(program.core)).toEqual(referenceCore)
    expect(decodeSemanticProgram(referenceEnvelope)).toEqual(program)
    expect(decodeSemanticProgramCore(referenceCore)).toEqual(program.core)
    expect(referenceEnvelope).toHaveLength(7_485)
    expect(referenceCore).toHaveLength(6_592)
    expect(sha256(referenceEnvelope)).toBe('20c8e1b773a60bbf9e84443b7ff6adca804ab35ce1805e1f48237564afa0c93e')
    expect(sha256(referenceCore)).toBe('aafc85582e01c55891e8424aabc67a29b09a2b4043fb778a53146928b26cd0f9')
    expect(referenceSemanticProgramHashV1(program)).toBe('473c09ca5352e45e626398c642ddafe04647f38809f1140d4f24c970b102b260')
    expect(referenceSemanticProgramHashV1(program.core))
      .toBe(referenceSemanticProgramHashV1(program))
    expect(semanticProgramHash(program)).toBe(referenceSemanticProgramHashV1(program))
    expect(semanticProgramHash(program.core)).toBe(referenceSemanticProgramHashV1(program.core))
    expect(new TextDecoder().decode(referenceEnvelope.subarray(0, 4))).toBe('SPE1')
    expect(new TextDecoder().decode(referenceCore.subarray(0, 4))).toBe('SPC1')
    expect([...referenceEnvelope.subarray(4, 8)]).toEqual([0, 1, 0, 2])
    expect([...referenceCore.subarray(4, 8)]).toEqual([0, 1, 0, 2])
    expect(new TextDecoder().decode(tessellationPolicy.subarray(0, 4))).toBe('TSP1')
    expect([...tessellationPolicy.subarray(4, 8)]).toEqual([0, 1, 0, 0])
  })

  it('requires the exact schema-1.2 execution feature and execution object', () => {
    const valid = makeReferenceSemanticProgramV1()
    expect(valid.schemaVersion).toEqual({ major: 1, minor: 2 })
    expect(valid.core.schemaVersion).toEqual({ major: 1, minor: 2 })
    expect(valid.core.requiredFeatures).toEqual(['semantic.execution-v2'])
    expect(valid.core.execution).toEqual({
      version: 'semantic-execution-v2',
      evaluationOrder: [0, 1, 2, 3],
      discardedEffects: [],
      terminal: null,
    })

    const malformed = [
      (() => { const program = clone(valid); program.schemaVersion.minor = 0; return program })(),
      (() => { const program = clone(valid); program.core.schemaVersion.minor = 0; return program })(),
      (() => { const program = clone(valid); program.core.requiredFeatures = []; return program })(),
      (() => { const program = clone(valid); program.core.requiredFeatures = ['semantic.execution-v1']; return program })(),
      (() => { const program = clone(valid); program.core.requiredFeatures = ['semantic.execution-v1', 'semantic.execution-v2']; return program })(),
      (() => { const program = clone(valid); delete program.core.execution; return program })(),
      (() => { const program = clone(valid); program.core.execution.version = 'semantic-execution-v1'; return program })(),
    ]
    for (const program of malformed) {
      expect(() => referenceValidateSemanticProgramV1(program, REFERENCE_SEMANTIC_SOURCE_V1))
        .toThrow(ReferenceSemanticProgramError)
      expect(() => normalizeSemanticProgram(program, REFERENCE_SEMANTIC_SOURCE_V1))
        .toThrow(SemanticProgramValidationError)
    }
  })

  it('requires re-lowering for schema and frame minors 1.0/1.1', () => {
    const expectRelower = (evaluate: () => unknown): void => {
      let failure: unknown
      try {
        evaluate()
      } catch (error) {
        failure = error
      }
      expect(failure).toBeInstanceOf(SemanticProgramValidationError)
      expect(failure).toMatchObject({ code: 'E_SEMANTIC_RELOWER_REQUIRED' })
    }

    for (const minor of [0, 1]) {
      const program = makeReferenceSemanticProgramV1()
      program.schemaVersion.minor = minor
      program.core.schemaVersion.minor = minor
      expect(() => referenceValidateSemanticProgramV1(program, REFERENCE_SEMANTIC_SOURCE_V1))
        .toThrow(ReferenceSemanticProgramError)
      expectRelower(() => normalizeSemanticProgram(program, REFERENCE_SEMANTIC_SOURCE_V1))

      const envelope = referenceEncodeSemanticProgramV1(makeReferenceSemanticProgramV1())
      envelope[7] = minor
      expectRelower(() => decodeSemanticProgram(envelope))

      const core = referenceEncodeSemanticProgramCoreV1(makeReferenceSemanticProgramV1())
      core[7] = minor
      expectRelower(() => decodeSemanticProgramCore(core))
    }
  })

  it('requires a complete dependency-safe execution permutation', () => {
    const malformedOrders = [
      [0, 1, 2],
      [0, 1, 2, 2],
      [2, 0, 1, 3],
    ]
    for (const evaluationOrder of malformedOrders) {
      const program = makeReferenceSemanticProgramV1()
      program.core.execution.evaluationOrder = evaluationOrder
      expect(() => referenceValidateSemanticProgramV1(program, REFERENCE_SEMANTIC_SOURCE_V1))
        .toThrow(ReferenceSemanticProgramError)
      expect(() => normalizeSemanticProgram(program, REFERENCE_SEMANTIC_SOURCE_V1))
        .toThrow(SemanticProgramValidationError)
    }
  })

  it('rejects a topologically safe swap of authored sibling kernel calls', () => {
    const program = makeReferenceSemanticProgramV1()
    program.core.execution.evaluationOrder = [1, 0, 2, 3]
    expect(() => referenceValidateSemanticProgramV1(program, REFERENCE_SEMANTIC_SOURCE_V1))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(program, REFERENCE_SEMANTIC_SOURCE_V1))
      .toThrow(SemanticProgramValidationError)
  })

  it('rejects coherent node renumbering that hides a normal sibling schedule swap', () => {
    const forged = makeReferenceSemanticProgramV1()
    coherentlyRenumberNodes(forged, [1, 0, 2, 3])
    expect(forged.core.execution.evaluationOrder).toEqual([0, 1, 2, 3])
    expect(forged.core.nodes[2].inputs).toEqual([1, 0])
    expect(() => referenceValidateSemanticProgramV1(forged, REFERENCE_SEMANTIC_SOURCE_V1))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(forged, REFERENCE_SEMANTIC_SOURCE_V1))
      .toThrow(SemanticProgramValidationError)
  })

  it('rejects coherent renumbering that moves a later published cube before an eager effect', () => {
    const fixture = makeEffectBeforePublishedCubeFixture()
    expect(() => referenceValidateSemanticProgramV1(fixture.program, fixture.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(fixture.program, fixture.source)).not.toThrow()

    const forged = clone(fixture.program)
    coherentlyRenumberNodes(forged, [3, 0, 1, 2])
    expect(forged.core.execution.evaluationOrder).toEqual([0, 1, 2, 3])
    expect(forged.core.execution.discardedEffects[0].root).toBe(3)
    expect(forged.core.result.item.node).toBe(0)
    expect(() => referenceValidateSemanticProgramV1(forged, fixture.source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(forged, fixture.source))
      .toThrow(SemanticProgramValidationError)
  })

  it('rejects coherent renumbering that reverses completed terminal-prefix siblings', () => {
    const fixture = makeTerminalSiblingScheduleFixture()
    expect(() => referenceValidateSemanticProgramV1(fixture.program, fixture.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(fixture.program, fixture.source)).not.toThrow()

    const forged = clone(fixture.program)
    coherentlyRenumberNodes(forged, [1, 0])
    expect(forged.core.execution.evaluationOrder).toEqual([0, 1])
    expect(forged.core.execution.terminal.prefixFrontier.map((entry: Record<string, any>) => entry.root))
      .toEqual([0, 1])
    expect(() => referenceValidateSemanticProgramV1(forged, fixture.source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(forged, fixture.source))
      .toThrow(SemanticProgramValidationError)
  })

  it('refuses flat-v0 structural defaults and requires fresh exact-source lowering', () => {
    const source = 'cube([1,2,3]);'
    const descriptor = referenceSemanticSourceDescriptorV1(source)
    const flatV0 = {
      schema: 'semantic-program',
      version: 0,
      languageContract: 'legacy/current',
      sourceHash: descriptor.sha256,
      sourceUtf8Bytes: descriptor.utf8ByteLength,
      sourceUtf16CodeUnits: descriptor.utf16CodeUnitLength,
      nodes: [{ id: 0, kind: 'box', dimension: 'solid3', size: [1, 2, 3], center: false }],
      outputs: [{ node: 0, occurrence: 0, color: [1, 1, 1, 1] }],
      occurrences: [{ node: 0, span: { start: 0, end: source.length }, label: 'cube()' }],
      capabilities: [],
    }
    expect(() => referenceValidateSemanticProgramV1(flatV0, source))
      .toThrow(ReferenceSemanticProgramError)

    let failure: unknown
    try {
      normalizeSemanticProgram(flatV0, source)
    } catch (error) {
      failure = error
    }
    expect(failure).toBeInstanceOf(SemanticProgramValidationError)
    expect(failure).toMatchObject({ code: 'E_SEMANTIC_RELOWER_REQUIRED' })
    expect((failure as Error).message).toMatch(/fresh exact-source lowering|re-lower/i)
  })

  it('recomputes the exact legacy empty-difference cutter effect', () => {
    const valid = makeDiscardedDifferenceEffectFixture()
    expect(() => referenceValidateSemanticProgramV1(valid.program, valid.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(valid.program, valid.source)).not.toThrow()
    expect(encodeSemanticProgram(valid.program)).toEqual(referenceEncodeSemanticProgramV1(valid.program))
    expect(semanticProgramHash(valid.program)).toBe(referenceSemanticProgramHashV1(valid.program))

    const nonEmptyBaseSource = 'difference(){ cube(1); sphere(1); }'
    const forgedNonEmptyBase = clone(valid.program)
    forgedNonEmptyBase.source = referenceSemanticSourceDescriptorV1(nonEmptyBaseSource)
    forgedNonEmptyBase.core.operations.splice(2, 1)
    forgedNonEmptyBase.core.operations.forEach((operation: Record<string, any>, id: number) => {
      operation.id = id
    })
    forgedNonEmptyBase.core.occurrences.splice(2, 1)
    forgedNonEmptyBase.core.occurrences.forEach((occurrence: Record<string, any>, id: number) => {
      occurrence.id = id
      if (occurrence.operation > 2) occurrence.operation--
    })
    forgedNonEmptyBase.provenance = forgedNonEmptyBase.core.operations.map((operation: Record<string, any>) => ({
      operation: operation.id,
      span: { start: 0, end: nonEmptyBaseSource.length },
      label: `${operation.name}()`,
    }))
    expect(() => referenceValidateSemanticProgramV1(forgedNonEmptyBase, nonEmptyBaseSource))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(forgedNonEmptyBase, nonEmptyBaseSource))
      .toThrow(SemanticProgramValidationError)

    const forgedOwner = clone(valid.program)
    forgedOwner.core.execution.discardedEffects[0].ownerOccurrence = 1
    expect(() => referenceValidateSemanticProgramV1(forgedOwner, valid.source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(forgedOwner, valid.source))
      .toThrow(SemanticProgramValidationError)

    const forgedRoot = clone(valid.program)
    forgedRoot.core.execution.discardedEffects[0].root = 1
    expect(() => referenceValidateSemanticProgramV1(forgedRoot, valid.source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(forgedRoot, valid.source))
      .toThrow(SemanticProgramValidationError)

    const omitted = clone(valid.program)
    omitted.core.execution.discardedEffects = []
    expect(() => referenceValidateSemanticProgramV1(omitted, valid.source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(omitted, valid.source))
      .toThrow(SemanticProgramValidationError)

    const brep = makeDiscardedDifferenceEffectFixture('openscad-viewer/brep-1')
    expect(() => referenceValidateSemanticProgramV1(brep.program, brep.source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(brep.program, brep.source))
      .toThrow(SemanticProgramValidationError)
  })

  it('binds discarded cutter effects to exact authored kernel order', () => {
    const valid = makeDiscardedDifferenceEffectFixture()

    const reordered = clone(valid.program)
    reordered.core.execution.evaluationOrder = [1, 0, 2]
    expect(() => referenceValidateSemanticProgramV1(reordered, valid.source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(reordered, valid.source))
      .toThrow(SemanticProgramValidationError)
  })

  it('admits 1001 independently owned eager effects under the node-derived cap', () => {
    const fixture = makeManyDiscardedDifferenceEffectsFixture()
    expect(fixture.program.core.execution.discardedEffects).toHaveLength(1_001)
    expect(() => referenceValidateSemanticProgramV1(fixture.program, fixture.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(fixture.program, fixture.source)).not.toThrow()
  })

  it('binds the frozen legacy terminal template to the exact kernel-prefix frontier', () => {
    const valid = makeTerminalPrefixFixture()
    expect(() => referenceValidateSemanticProgramV1(valid.program, valid.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(valid.program, valid.source)).not.toThrow()
    expect(encodeSemanticProgram(valid.program)).toEqual(referenceEncodeSemanticProgramV1(valid.program))
    expect(semanticProgramHash(valid.program)).toBe(referenceSemanticProgramHashV1(valid.program))

    const admittedTypeError = clone(valid.program)
    admittedTypeError.core.diagnosticTemplates[0].arguments[0].value.value = 'TypeError'
    expect(() => referenceValidateSemanticProgramV1(admittedTypeError, valid.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(admittedTypeError, valid.source)).not.toThrow()

    const mutations: Array<(program: ReferenceSemanticProgramV1) => void> = [
      program => { program.core.execution.terminal.prefixFrontier = [] },
      program => { program.core.execution.terminal.prefixFrontier[0].ownerOccurrence = null },
      program => { program.core.execution.terminal.prefixFrontier[0].ownerOccurrence = 1 },
      program => { program.core.execution.terminal.occurrence = 0 },
      program => { program.core.execution.terminal.diagnosticTemplate = 1 },
      program => { program.core.diagnosticTemplates[0].severity = 'warning' },
      program => { program.core.diagnosticTemplates[0].code = 'ARBITRARY_ERROR' },
      program => { program.core.diagnosticTemplates[0].operation = 0 },
      program => { program.core.diagnosticTemplates[0].arguments[0].name = 'class' },
      program => { program.core.diagnosticTemplates[0].arguments[0].value.value = 'Error' },
      program => { program.core.diagnosticTemplates[0].arguments[1].value.value = '0'.repeat(63) },
      program => {
        program.core.execution.discardedEffects = [{
          tag: 'legacy-difference-cutters',
          ownerOccurrence: 0,
          root: 0,
        }]
      },
      program => { program.core.execution.terminal = null },
      program => {
        program.core.result = {
          tag: 'single',
          item: { node: 0, producerOccurrence: 0, identityOccurrence: 0, color: [1, 1, 1, 1] },
        }
      },
    ]
    for (const mutate of mutations) {
      const forged = clone(valid.program)
      mutate(forged)
      expect(() => referenceValidateSemanticProgramV1(forged, valid.source))
        .toThrow(ReferenceSemanticProgramError)
      expect(() => normalizeSemanticProgram(forged, valid.source))
        .toThrow(SemanticProgramValidationError)
    }
  })

  it.each([
    'difference-mix',
    'union-mix',
    'linear-extrude-solid',
    'partial-rotate',
  ] as const)('admits a self-consistent post-child active-subtree terminal: %s', kind => {
    const fixture = makePostChildTerminalFixture(kind)
    const terminal = fixture.program.core.execution.terminal
    expect(terminal.occurrence).toBe(0)
    expect(fixture.program.core.occurrences.length).toBeGreaterThan(terminal.occurrence + 1)
    if (kind === 'partial-rotate') {
      expect(fixture.program.core.occurrences[terminal.occurrence]).toMatchObject({
        node: 2,
        outputOrdinal: 0,
      })
    }
    expect(() => referenceValidateSemanticProgramV1(fixture.program, fixture.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(fixture.program, fixture.source)).not.toThrow()
    expect(encodeSemanticProgram(fixture.program)).toEqual(referenceEncodeSemanticProgramV1(fixture.program))
    expect(semanticProgramHash(fixture.program)).toBe(referenceSemanticProgramHashV1(fixture.program))
  })

  it.each([
    'difference(){cube(1);square(1);}',
    'union(){cube(1);square(1);}',
    'linear_extrude(1) cube(1);',
    'rotate(a=1,v=[0,0,0]){square(1);cube(1);}',
    'translate([1,0,0]) union(){cube(1);square(1);}',
  ])('validates a trusted-lowerer active-subtree artifact without claiming to derive its first error: %s', source => {
    const artifact = lowerOpenSCADToSemanticProgram(source, { captureTerminalFailure: true })
    expect(artifact.terminalError).not.toBeNull()
    expect(() => referenceValidateSemanticProgramV1(artifact.program, source)).not.toThrow()
  })

  it.each([
    ['text("x");', 'text'],
    ['x = 1/0;', '$assign'],
  ] as const)('admits only the exact unsupported/control terminal activation: %s', (source, operation) => {
    const artifact = lowerOpenSCADToSemanticProgram(source, { captureTerminalFailure: true })
    expect(artifact.terminalError).not.toBeNull()
    expect(artifact.program.core.operations.map(item => item.name)).toEqual([operation])
    expect(artifact.program.core.execution.terminal).toMatchObject({
      occurrence: 0,
      prefixFrontier: [],
    })
    expect(() => referenceValidateSemanticProgramV1(artifact.program, source)).not.toThrow()
    expect(() => normalizeSemanticProgram(artifact.program, source)).not.toThrow()
  })

  it.each([
    ['if(false) text("x"); cube(1);', 'text'],
    ['if(false) minkowski(){cube(1);sphere(1);} cube(1);', 'minkowski'],
  ] as const)('keeps unreachable unsupported static syntax execution-inert: %s', (source, unsupported) => {
    const artifact = lowerOpenSCADToSemanticProgram(source, { captureTerminalFailure: true })
    expect(artifact.terminalError).toBeNull()
    expect(artifact.program.core.operations.some(operation => operation.name === unsupported)).toBe(true)
    expect(artifact.program.core.occurrences.some(occurrence => (
      artifact.program.core.operations[occurrence.operation].name === unsupported
    ))).toBe(false)
    expect(() => referenceValidateSemanticProgramV1(artifact.program, source)).not.toThrow()
    expect(() => normalizeSemanticProgram(artifact.program, source)).not.toThrow()
  })

  it('does not grant the terminal-unsupported rule to a completed activation', () => {
    const source = 'cube(1);'
    const forged = clone(lowerOpenSCADToSemanticProgram(source).program)
    const path = [{ kind: 'call', name: 'text', ordinal: 0 }]
    forged.core.operations[0].name = 'text'
    forged.core.operations[0].operationId = referenceSemanticOperationIdV1(path)
    forged.core.operations[0].structuralPath = path
    forged.provenance[0].label = 'text()'
    recomputeOccurrence(forged, 0)
    expect(() => referenceValidateSemanticProgramV1(forged, source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(forged, source))
      .toThrow(SemanticProgramValidationError)
  })

  it('replays a completed base reducer before an active-ancestor cutter terminal', () => {
    const fixture = makeActiveAncestorDifferenceTerminalFixture()
    expect(fixture.program.core.execution.evaluationOrder).toEqual([0, 1, 2])
    expect(fixture.program.core.nodes[2]).toMatchObject({
      kind: 'boolean',
      operation: 'union',
      inputs: [0, 1],
    })
    expect(fixture.program.core.execution.terminal).toMatchObject({
      occurrence: 6,
      prefixFrontier: [{ root: 2, ownerOccurrence: null }],
    })
    expect(() => referenceValidateSemanticProgramV1(fixture.program, fixture.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(fixture.program, fixture.source)).not.toThrow()

    const artifact = lowerOpenSCADToSemanticProgram(fixture.source, { captureTerminalFailure: true })
    expect(artifact.terminalError).not.toBeNull()
    expect(artifact.program.core.execution.evaluationOrder).toEqual([0, 1, 2])
    expect(artifact.program.core.execution.terminal?.prefixFrontier)
      .toEqual([{ root: 2, ownerOccurrence: null }])
    expect(() => referenceValidateSemanticProgramV1(artifact.program, fixture.source)).not.toThrow()
  })

  it.each([
    {
      source: 'difference(){for(i=[0:2]) cube(i+1); sphere(1);}',
      order: [0, 1, 2, 3, 4, 5],
      kinds: ['box', 'box', 'box', 'union', 'sphere-polygonal', 'difference'],
      terminalRoot: null,
    },
    {
      source: 'difference(){for(i=[0:2]) cube(i+1); assert(false,"x") cube(1);}',
      order: [0, 1, 2, 3],
      kinds: ['box', 'box', 'box', 'union'],
      terminalRoot: 3,
    },
  ])('aggregates repeated runtime activations into one authored difference bucket: $source', specification => {
    const artifact = lowerOpenSCADToSemanticProgram(specification.source, {
      captureTerminalFailure: true,
    })
    expect(artifact.program.core.execution.evaluationOrder).toEqual(specification.order)
    expect(artifact.program.core.nodes.map(node => (
      node.kind === 'boolean' ? node.operation : node.kind
    ))).toEqual(specification.kinds)
    if (specification.terminalRoot === null) {
      expect(artifact.program.core.execution.terminal).toBeNull()
    } else {
      expect(artifact.program.core.execution.terminal?.prefixFrontier)
        .toEqual([{ root: specification.terminalRoot, ownerOccurrence: null }])
    }
    expect(() => referenceValidateSemanticProgramV1(artifact.program, specification.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(artifact.program, specification.source)).not.toThrow()
  })

  it.each([
    'difference(){ let(x=0); cube(1); sphere(1); }',
    'difference(){ x=1; cube(1); sphere(1); }',
    'difference(){if(false) cube(1); sphere(1);}',
    'difference(){ group(){ cube(); sphere(1); } group(){ translate([1,0,0]) cube(); sphere(0.5); } }',
    'difference(){group(){cube(1);sphere(1);} assert(false,"x") cube(1);}',
    'if(false) text("x"); cube(1);',
    'if(false) minkowski(){cube(1);sphere(1);} cube(1);',
    'x = 1/0;',
  ])('keeps trusted lowerer/reference/production corpus parity: %s', source => {
    const artifact = lowerOpenSCADToSemanticProgram(source, { captureTerminalFailure: true })
    expect(() => referenceValidateSemanticProgramV1(artifact.program, source)).not.toThrow()
    expect(() => normalizeSemanticProgram(artifact.program, source)).not.toThrow()
  })

  it('propagates a post-child terminal through its unfinished runtime ancestor chain', () => {
    const fixture = makeAncestorPostChildTerminalFixture()
    expect(fixture.program.core.execution.terminal.occurrence).toBe(2)
    expect(fixture.program.core.occurrences.slice(3).every((_: unknown, index: number) => {
      let parent = index + 3
      while (parent !== null) {
        if (parent === 2) return true
        parent = fixture.program.core.occurrences[parent].parent
      }
      return false
    })).toBe(true)
    expect(() => referenceValidateSemanticProgramV1(fixture.program, fixture.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(fixture.program, fixture.source)).not.toThrow()
  })

  it('rejects a forged prior top-level terminal when a later sibling activation exists', () => {
    const fixture = makeTerminalPrefixFixture()
    const forged = clone(fixture.program)
    forged.core.execution.terminal.occurrence = 0
    forged.core.diagnosticTemplates[0].operation = 0
    expect(() => referenceValidateSemanticProgramV1(forged, fixture.source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(forged, fixture.source))
      .toThrow(SemanticProgramValidationError)
  })

  it('does not relax completed sibling production inside an active terminal subtree', () => {
    const fixture = makePostChildTerminalFixture('partial-rotate')
    const forged = clone(fixture.program)
    forged.core.occurrences[2].node = 1
    expect(() => referenceValidateSemanticProgramV1(forged, fixture.source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(forged, fixture.source))
      .toThrow(SemanticProgramValidationError)
  })

  it('binds terminal interruption occurrence and diagnostic arguments into the core hash', () => {
    const valid = makeTerminalPrefixFixture()
    const deletedInterruption = clone(valid.program)
    deletedInterruption.core.occurrences.pop()
    expect(() => referenceValidateSemanticProgramV1(deletedInterruption, valid.source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(deletedInterruption, valid.source))
      .toThrow(SemanticProgramValidationError)

    const messageA = makeTerminalPrefixFixture('A')
    const messageB = makeTerminalPrefixFixture('B')
    expect(() => referenceValidateSemanticProgramV1(messageA.program, messageA.source)).not.toThrow()
    expect(() => referenceValidateSemanticProgramV1(messageB.program, messageB.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(messageA.program, messageA.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(messageB.program, messageB.source)).not.toThrow()
    expect(referenceSemanticProgramHashV1(messageA.program))
      .not.toBe(referenceSemanticProgramHashV1(messageB.program))
    expect(semanticProgramHash(messageA.program)).toBe(referenceSemanticProgramHashV1(messageA.program))
    expect(semanticProgramHash(messageB.program)).toBe(referenceSemanticProgramHashV1(messageB.program))
  })

  it('canonicalizes object insertion order without changing bytes or hash', () => {
    const first = makeReferenceSemanticProgramV1()
    const reordered = reverseObjectInsertionOrder(first) as ReferenceSemanticProgramV1
    referenceValidateSemanticProgramV1(reordered, REFERENCE_SEMANTIC_SOURCE_V1)
    expect(referenceEncodeSemanticProgramV1(reordered))
      .toEqual(referenceEncodeSemanticProgramV1(first))
    expect(encodeSemanticProgram(reordered)).toEqual(encodeSemanticProgram(first))
    expect(referenceSemanticProgramHashV1(reordered))
      .toBe(referenceSemanticProgramHashV1(first))
  })

  it('bounds aggregate canonical string bytes before the 64 MiB frame ceiling', () => {
    const program = makeReferenceSemanticProgramV1()
    const largeValue = 'x'.repeat(4_096)
    program.core.diagnosticTemplates = Array.from({ length: 33 }, (_, template) => ({
      id: template,
      code: `ORACLE_${template}`,
      severity: 'info',
      operation: 2,
      arguments: Array.from({ length: 128 }, (_, argument) => ({
        name: `a${argument}`,
        value: { tag: 'string', value: largeValue },
      })),
    }))
    program.diagnostics = Array.from({ length: 33 }, (_, template) => ({
      template,
      message: `diagnostic ${template}`,
      span: null,
    }))
    program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)

    expect(() => referenceValidateSemanticProgramV1(
      program,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(
      program,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(SemanticProgramValidationError)
    expect(() => referenceEncodeSemanticProgramV1(program))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => encodeSemanticProgram(program))
      .toThrow(SemanticProgramValidationError)
  })

  it('bounds source descriptors before UTF-8 encoding and hashing', () => {
    expect(() => referenceSemanticSourceDescriptorV1('x'.repeat(250_001)))
      .toThrow(ReferenceSemanticProgramError)
  })

  it('keeps source-bound presentation outside SPC1 while core mutations enter the hash', () => {
    const first = makeReferenceSemanticProgramV1()
    const presentationOnly = clone(first)
    const secondSource = `${REFERENCE_SEMANTIC_SOURCE_V1}\n// display-only change`
    presentationOnly.source = referenceSemanticSourceDescriptorV1(secondSource)
    presentationOnly.diagnostics[0].message = 'another localized presentation'
    referenceValidateSemanticProgramV1(presentationOnly, secondSource)
    normalizeSemanticProgram(presentationOnly, secondSource)

    expect(referenceEncodeSemanticProgramCoreV1(presentationOnly))
      .toEqual(referenceEncodeSemanticProgramCoreV1(first))
    expect(encodeSemanticProgramCore(presentationOnly))
      .toEqual(encodeSemanticProgramCore(first))
    expect(referenceSemanticProgramHashV1(presentationOnly))
      .toBe(referenceSemanticProgramHashV1(first))
    expect(referenceEncodeSemanticProgramV1(presentationOnly))
      .not.toEqual(referenceEncodeSemanticProgramV1(first))

    const semanticChange = clone(first)
    semanticChange.core.result.item.color[0] = 0.5
    expect(referenceSemanticProgramHashV1(semanticChange))
      .not.toBe(referenceSemanticProgramHashV1(first))
    expect(semanticProgramHash(semanticChange))
      .toBe(referenceSemanticProgramHashV1(semanticChange))
  })

  it.each(mutations)('rejects $name in both validators', ({ family, mutate }) => {
    const program = makeReferenceSemanticProgramV1()
    mutate(program)

    let oracleError: unknown
    try {
      referenceValidateSemanticProgramV1(program, REFERENCE_SEMANTIC_SOURCE_V1)
    } catch (error) {
      oracleError = error
    }
    expect(oracleError).toBeInstanceOf(ReferenceSemanticProgramError)
    expect(oracleError).toMatchObject({ family })

    let productionError: unknown
    try {
      normalizeSemanticProgram(program, REFERENCE_SEMANTIC_SOURCE_V1)
    } catch (error) {
      productionError = error
    }
    expect(productionError).toBeInstanceOf(SemanticProgramValidationError)
    expect(productionError).toMatchObject({ code: PRODUCTION_CODE_BY_FAMILY[family] })
  })

  it('rejects scene-identity swapping across multi-output node lineage', () => {
    const valid = makeMultiOutputFixture()
    expect(() => referenceValidateSemanticProgramV1(
      valid,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()
    expect(() => normalizeSemanticProgram(
      valid,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()

    const spoofed = clone(valid)
    const [first, second] = spoofed.core.result.items
    ;[first.identityOccurrence, second.identityOccurrence] = [
      second.identityOccurrence,
      first.identityOccurrence,
    ]
    expect(() => referenceValidateSemanticProgramV1(
      spoofed,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(
      spoofed,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(SemanticProgramValidationError)
  })

  it('admits self-owned and canonical-first descendant identities for multi-output occurrence rows', () => {
    const selfOwned = makeCanonicalOccurrenceRowFixture()
    const selfOwnedItems = selfOwned.core.result.items
    expect(selfOwned.core.occurrences[selfOwnedItems[0].producerOccurrence].occurrenceId)
      .toBe(selfOwned.core.occurrences[selfOwnedItems[1].producerOccurrence].occurrenceId)
    expect(selfOwnedItems.map(item => item.identityOccurrence))
      .toEqual(selfOwnedItems.map(item => item.producerOccurrence))
    expect(() => referenceValidateSemanticProgramV1(
      selfOwned,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()
    expect(() => normalizeSemanticProgram(
      selfOwned,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()

    const canonicalFirst = makeCanonicalFirstTransparentWrapperFixture()
    const [first, second] = canonicalFirst.core.result.items
    expect(canonicalFirst.core.occurrences[first.producerOccurrence].occurrenceId)
      .toBe(canonicalFirst.core.occurrences[second.producerOccurrence].occurrenceId)
    expect(second.producerOccurrence).toBe(2)
    expect(second.identityOccurrence).toBe(4)
    expect(canonicalFirst.core.occurrences[second.identityOccurrence].parent).toBe(1)
    expect(() => referenceValidateSemanticProgramV1(
      canonicalFirst,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()
    expect(() => normalizeSemanticProgram(
      canonicalFirst,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()
  })

  it('rejects a preserving multi-output wrapper that steals its child identity', () => {
    const forged = makeCanonicalFirstTransparentWrapperFixture()
    forged.core.result.items[0].identityOccurrence = 1
    expect(() => referenceValidateSemanticProgramV1(
      forged,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(
      forged,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(SemanticProgramValidationError)
  })

  it('admits only the exact legacy zero/zero polyhedron exception', () => {
    const exact = makePrimitiveSchemaFixture('polyhedron', {
      vertices: [],
      triangles: [],
    })
    expect(() => referenceValidateSemanticProgramV1(exact.program, exact.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(exact.program, exact.source)).not.toThrow()
    expect(encodeSemanticProgram(exact.program))
      .toEqual(referenceEncodeSemanticProgramV1(exact.program))
    expect(semanticProgramHash(exact.program))
      .toBe(referenceSemanticProgramHashV1(exact.program))

    const partials = [
      makePrimitiveSchemaFixture('polyhedron', {
        vertices: [],
        triangles: [[0, 0, 0]],
      }),
      makePrimitiveSchemaFixture('polyhedron', {
        vertices: [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
        triangles: [],
      }),
    ]
    for (const partial of partials) {
      expect(() => referenceValidateSemanticProgramV1(partial.program, partial.source))
        .toThrow(ReferenceSemanticProgramError)
      expect(() => normalizeSemanticProgram(partial.program, partial.source))
        .toThrow(SemanticProgramValidationError)
    }

    const brepEmpty = makePrimitiveSchemaFixture('polyhedron', {
      vertices: [],
      triangles: [],
    }, 'openscad-viewer/brep-1')
    expect(() => referenceValidateSemanticProgramV1(brepEmpty.program, brepEmpty.source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(brepEmpty.program, brepEmpty.source))
      .toThrow(SemanticProgramValidationError)
  })

  it('keeps repeated polyhedron indices legacy-only while admitting strict B-rep topology', () => {
    const vertices = [
      [0, 0, 0],
      [1, 0, 0],
      [0, 1, 0],
      [0, 0, 1],
    ]
    const repeated = makePrimitiveSchemaFixture('polyhedron', {
      vertices,
      triangles: [[0, 0, 1]],
    })
    expect(() => referenceValidateSemanticProgramV1(repeated.program, repeated.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(repeated.program, repeated.source)).not.toThrow()

    const repeatedBrep = makePrimitiveSchemaFixture('polyhedron', {
      vertices,
      triangles: [[0, 0, 1]],
    }, 'openscad-viewer/brep-1')
    expect(() => referenceValidateSemanticProgramV1(repeatedBrep.program, repeatedBrep.source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(repeatedBrep.program, repeatedBrep.source))
      .toThrow(SemanticProgramValidationError)

    const strictBrep = makePrimitiveSchemaFixture('polyhedron', {
      vertices,
      triangles: [[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]],
    }, 'openscad-viewer/brep-1')
    expect(() => referenceValidateSemanticProgramV1(strictBrep.program, strictBrep.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(strictBrep.program, strictBrep.source)).not.toThrow()
  })

  it('keeps zero-to-two-point polygon rings legacy-only and preserves nonempty ring framing', () => {
    for (let pointCount = 0; pointCount <= 2; pointCount++) {
      const points = [[0, 0], [1, 0]].slice(0, pointCount)
      const legacy = makePrimitiveSchemaFixture('polygon', {
        rings: [points],
        fillRule: 'even-odd',
      })
      expect(() => referenceValidateSemanticProgramV1(legacy.program, legacy.source)).not.toThrow()
      expect(() => normalizeSemanticProgram(legacy.program, legacy.source)).not.toThrow()

      const brep = makePrimitiveSchemaFixture('polygon', {
        rings: [points],
        fillRule: 'even-odd',
      }, 'openscad-viewer/brep-1')
      expect(() => referenceValidateSemanticProgramV1(brep.program, brep.source))
        .toThrow(ReferenceSemanticProgramError)
      expect(() => normalizeSemanticProgram(brep.program, brep.source))
        .toThrow(SemanticProgramValidationError)
    }

    const noRings = makePrimitiveSchemaFixture('polygon', {
      rings: [],
      fillRule: 'even-odd',
    })
    expect(() => referenceValidateSemanticProgramV1(noRings.program, noRings.source))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(noRings.program, noRings.source))
      .toThrow(SemanticProgramValidationError)

    const strictBrep = makePrimitiveSchemaFixture('polygon', {
      rings: [[[0, 0], [1, 0], [0, 1]]],
      fillRule: 'even-odd',
    }, 'openscad-viewer/brep-1')
    expect(() => referenceValidateSemanticProgramV1(strictBrep.program, strictBrep.source)).not.toThrow()
    expect(() => normalizeSemanticProgram(strictBrep.program, strictBrep.source)).not.toThrow()
  })

  it('rejects sibling occurrence identity donation across a valid transform node lineage', () => {
    const siblingDonation = makeSiblingIdentityDonationFixture()
    expect(() => referenceValidateSemanticProgramV1(
      siblingDonation,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(
      siblingDonation,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(SemanticProgramValidationError)
  })

  it.each([
    {
      name: 'two primitive groups claim one shared DAG node',
      make: makeSharedPrimitiveNodeFixture,
    },
    {
      name: 'operation and materialized node kind disagree',
      make: () => {
        const program = makeReferenceSemanticProgramV1()
        ;[program.core.occurrences[3].node, program.core.occurrences[4].node] = [
          program.core.occurrences[4].node,
          program.core.occurrences[3].node,
        ]
        return program
      },
    },
    {
      name: 'transparent module frame carries a non-null output',
      make: () => {
        const program = makeReferenceSemanticProgramV1()
        const frame = program.core.occurrences[0]
        frame.node = 3
        frame.outputOrdinal = 0
        frame.sceneEntityId = referenceSemanticSceneEntityIdV1(frame.occurrenceId, 0)
        return program
      },
    },
  ])('rejects closed-production forgery: $name', ({ make }) => {
    const forged = make()
    expect(() => referenceValidateSemanticProgramV1(
      forged,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(
      forged,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(SemanticProgramValidationError)
  })

  it('admits only control/module skips to the nearest static-parent occurrence', () => {
    const valid = makeSkippedStaticParentFixture()
    expect(() => referenceValidateSemanticProgramV1(valid, REFERENCE_SEMANTIC_SOURCE_V1)).not.toThrow()
    expect(() => normalizeSemanticProgram(valid, REFERENCE_SEMANTIC_SOURCE_V1)).not.toThrow()

    const nonExpansionSkip = clone(valid)
    nonExpansionSkip.core.operations[5].category = 'geometry'
    expect(() => referenceValidateSemanticProgramV1(
      nonExpansionSkip,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(
      nonExpansionSkip,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(SemanticProgramValidationError)

    const nonNearest = clone(valid)
    nonNearest.core.operations[1].category = 'control'
    nonNearest.core.occurrences[2].operation = 1
    nonNearest.core.occurrences[2].staticParent = 0
    recomputeOccurrence(nonNearest, 2)
    expect(() => referenceValidateSemanticProgramV1(
      nonNearest,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(
      nonNearest,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(SemanticProgramValidationError)
  })

  it('accepts the exact caller-body/module-definition/children continuation tuple', () => {
    const valid = makeChildrenContinuationFixture()
    expect(() => referenceValidateSemanticProgramV1(
      valid,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()
    expect(() => normalizeSemanticProgram(
      valid,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()
    expect(encodeSemanticProgram(valid)).toEqual(referenceEncodeSemanticProgramV1(valid))
    expect(semanticProgramHash(valid)).toBe(referenceSemanticProgramHashV1(valid))
  })

  it.each([
    {
      name: 'call and definition parameter bindings disagree',
      mutate: (program: ReferenceSemanticProgramV1) => {
        program.core.occurrences[2].dynamicSlots[0].value = { tag: 'number', value: 2 }
        for (let index = 2; index <= 8; index++) recomputeOccurrence(program, index)
      },
    },
    {
      name: 'caller body has a second direct definition child',
      mutate: (program: ReferenceSemanticProgramV1) => {
        const dynamicSlots = [{
          name: 'x',
          value: { tag: 'number', value: 2 },
          duplicateOrdinal: 0,
        }]
        const parent = 1
        program.core.occurrences.push({
          id: program.core.occurrences.length,
          occurrenceId: referenceSemanticOccurrenceIdV1(
            program.core.occurrences[parent].occurrenceId,
            null,
            program.core.operations[0].operationId,
            dynamicSlots,
          ),
          operation: 0,
          parent,
          staticParent: null,
          dynamicSlots,
          node: null,
          outputOrdinal: null,
          sceneEntityId: null,
        })
      },
    },
    {
      name: 'children has a second direct expansion child',
      mutate: (program: ReferenceSemanticProgramV1) => {
        const dynamicSlots = [{
          name: '$index',
          value: { tag: 'undefined' },
          duplicateOrdinal: 1,
        }]
        const parent = 6
        const staticParent = 1
        program.core.occurrences.push({
          id: program.core.occurrences.length,
          occurrenceId: referenceSemanticOccurrenceIdV1(
            program.core.occurrences[parent].occurrenceId,
            program.core.occurrences[staticParent].occurrenceId,
            program.core.operations[7].operationId,
            dynamicSlots,
          ),
          operation: 7,
          parent,
          staticParent,
          dynamicSlots,
          node: null,
          outputOrdinal: null,
          sceneEntityId: null,
        })
      },
    },
    {
      name: 'children is not lexically contained by the activated definition',
      mutate: (program: ReferenceSemanticProgramV1) => {
        const operation = appendRootOperation(program, 'control', 'children', 'control')
        program.core.occurrences[6].operation = operation
        program.core.occurrences[6].staticParent = null
        for (let index = 6; index <= 8; index++) recomputeOccurrence(program, index)
      },
    },
    {
      name: 'expanded child is an unrelated static-root injection',
      mutate: (program: ReferenceSemanticProgramV1) => {
        const operation = appendRootOperation(program, 'call', 'cube', 'geometry')
        program.core.occurrences[8].operation = operation
        program.core.occurrences[8].staticParent = null
        recomputeOccurrence(program, 8)
      },
    },
    {
      name: 'children and expansion index values disagree',
      mutate: (program: ReferenceSemanticProgramV1) => {
        program.core.occurrences[7].dynamicSlots[0].value = { tag: 'null' }
        recomputeOccurrence(program, 7)
        recomputeOccurrence(program, 8)
      },
    },
  ])('rejects malformed children continuation: $name', ({ mutate }) => {
    const forged = makeChildrenContinuationFixture()
    mutate(forged)
    expect(() => referenceValidateSemanticProgramV1(
      forged,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(
      forged,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(SemanticProgramValidationError)
  })

  it('requires parent and staticParent to target the canonical first occurrence row', () => {
    const valid = makeCanonicalOccurrenceRowFixture()
    expect(() => referenceValidateSemanticProgramV1(valid, REFERENCE_SEMANTIC_SOURCE_V1)).not.toThrow()
    expect(() => normalizeSemanticProgram(valid, REFERENCE_SEMANTIC_SOURCE_V1)).not.toThrow()

    const alias = clone(valid)
    alias.core.occurrences[5].parent = 4
    alias.core.occurrences[5].staticParent = 4
    expect(() => referenceValidateSemanticProgramV1(
      alias,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(
      alias,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(SemanticProgramValidationError)
  })

  it('assigns equal dynamic-slot duplicate ordinals in first-seen language order', () => {
    const valid = makeDuplicateOrdinalFixture()
    expect(() => referenceValidateSemanticProgramV1(valid, REFERENCE_SEMANTIC_SOURCE_V1)).not.toThrow()
    expect(() => normalizeSemanticProgram(valid, REFERENCE_SEMANTIC_SOURCE_V1)).not.toThrow()

    const reversed = clone(valid)
    reversed.core.occurrences[5].dynamicSlots[0].duplicateOrdinal = 1
    reversed.core.occurrences[6].dynamicSlots[0].duplicateOrdinal = 0
    recomputeOccurrence(reversed, 5)
    recomputeOccurrence(reversed, 6)
    expect(() => referenceValidateSemanticProgramV1(reversed, REFERENCE_SEMANTIC_SOURCE_V1))
      .toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(reversed, REFERENCE_SEMANTIC_SOURCE_V1))
      .toThrow(SemanticProgramValidationError)
  })

  it('validates legacy linear-extrude scale as Vec2 while preserving zero top-scale compatibility', () => {
    const collapsed = makeLegacyLinearExtrudeFixture([0, 0])
    expect(collapsed.core.capabilityClosure)
      .toContain('geometry.transition.region.d2.to.solid-set.d3')
    expect(() => referenceValidateSemanticProgramV1(
      collapsed,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()
    expect(() => normalizeSemanticProgram(
      collapsed,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()

    const forged = makeLegacyLinearExtrudeFixture(['forged'])
    expect(() => referenceValidateSemanticProgramV1(
      forged,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(
      forged,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(SemanticProgramValidationError)
  })

  it('recomputes first-child difference buckets and their hidden canonical union reducers', () => {
    const valid = makePartitionedDifferenceFixture()
    expect(() => referenceValidateSemanticProgramV1(
      valid,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()
    expect(() => normalizeSemanticProgram(
      valid,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).not.toThrow()

    const escapedPartition = clone(valid)
    escapedPartition.core.nodes[2].operation = 'intersection'
    escapedPartition.core.capabilityClosure = referenceSemanticCapabilityClosureV1(
      escapedPartition.core,
    )
    expect(() => referenceValidateSemanticProgramV1(
      escapedPartition,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(ReferenceSemanticProgramError)
    expect(() => normalizeSemanticProgram(
      escapedPartition,
      REFERENCE_SEMANTIC_SOURCE_V1,
    )).toThrow(SemanticProgramValidationError)
  })

  it('accepts a valid 12,000-node chain without recursive host-stack dependence', () => {
    const program = makeReferenceLinearChainProgramV1(12_000)
    expect(() => referenceValidateSemanticProgramV1(program, 'chain();')).not.toThrow()
    expect(() => normalizeSemanticProgram(program, 'chain();')).not.toThrow()
  }, 20_000)
})
