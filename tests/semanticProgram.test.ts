import { describe, expect, it } from 'vitest'
import {
  deriveSemanticAmbiguityGroupId,
  deriveSemanticCapabilityClosure,
  deriveSemanticOccurrenceId,
  deriveSemanticOperationId,
  deriveSemanticSceneEntityId,
  type SemanticNode,
  type SemanticProgramV1,
  type SemanticStaticOperation,
  type SemanticStructuralPathSegment,
  type SemanticValueType,
} from '../src/core/semanticProgram'
import {
  attestSemanticProgram,
  decodeSemanticProgram,
  encodeSemanticProgram,
  encodeSemanticProgramCore,
  semanticProgramHash,
  semanticTessellationPolicyHash,
} from '../src/services/semanticProgramCodec'
import {
  normalizeDecodedSemanticProgram,
  normalizeSemanticProgram,
  semanticSourceDescriptor,
  SemanticProgramValidationError,
} from '../src/services/semanticProgramValidator'
import {
  executeSemanticProgram,
  SemanticProgramExecutionError,
  type SemanticCarrierKey,
  type SemanticBackendPayloadLease,
  type SemanticProgramBackend,
} from '../src/services/semanticProgramExecutor'
import {
  lowerOpenSCADToSemanticProgram,
  requireTrustedSemanticLowering,
} from '../src/services/semanticProgramLowerer'
import { lowerOpenSCADToSemanticProgramUnchecked } from '../src/services/openscadSemanticLowerer'
import { OpenSCADParseError } from '../src/services/openscadErrors'

function clone<T>(value: T): T { return structuredClone(value) }

const PRESERVING = Object.freeze({ tag: 'representation-preserving' as const })
function valueType(
  contract: 'legacy/current' | 'openscad-viewer/brep-1',
  geometryKind: SemanticValueType['geometryKind'],
  space: SemanticValueType['space'],
): SemanticValueType {
  return {
    geometryKind,
    space,
    representation: contract === 'legacy/current' ? 'mesh' : 'analytic-brep',
    evidence: PRESERVING,
  }
}

function operation(
  id: number,
  name: string,
  ordinal = id,
  category: SemanticStaticOperation['category'] = 'geometry',
): SemanticStaticOperation {
  const structuralPath: readonly SemanticStructuralPathSegment[] = [{ kind: 'call', name, ordinal }]
  return {
    id,
    operationId: deriveSemanticOperationId(structuralPath),
    parent: null,
    childOrdinal: ordinal,
    name,
    category,
    structuralPath,
    identityEvidence: 'structural-unique',
    ambiguityGroup: null,
  }
}

function finalize(program: Record<string, any>): SemanticProgramV1 {
  program.core.capabilityClosure = [...deriveSemanticCapabilityClosure(program.core)]
  return program as SemanticProgramV1
}

function oneNodeProgram(
  source: string,
  languageContract: 'legacy/current' | 'openscad-viewer/brep-1',
  node: SemanticNode,
  name: string,
): SemanticProgramV1 {
  const op = operation(0, name, 0)
  const occurrenceId = deriveSemanticOccurrenceId(null, null, op.operationId, [])
  return finalize({
    schema: 'semantic-program-envelope',
    schemaVersion: { major: 1, minor: 2 },
    source: semanticSourceDescriptor(source),
    core: {
      schema: 'semantic-program-core',
      schemaVersion: { major: 1, minor: 2 },
      requiredFeatures: ['semantic.execution-v2'],
      identityVersion: 'semantic-program-core-v1',
      language: {
        contract: languageContract,
        semanticsRevision: languageContract === 'legacy/current' ? '1.0.0' : 'brep-1.0.0',
        capabilityGraphVersion: 'semantic-capabilities-v1',
      },
      units: {
        length: 'millimeter',
        angle: 'degree',
        handedness: 'right',
        upAxis: 'z',
        matrixLayout: 'column-major',
        composition: 'parent-times-local',
      },
      operations: [op],
      occurrences: [{
        id: 0,
        occurrenceId,
        operation: 0,
        parent: null,
        staticParent: null,
        dynamicSlots: [],
        node: 0,
        outputOrdinal: 0,
        sceneEntityId: deriveSemanticSceneEntityId(occurrenceId, 0),
      }],
      nodes: [node],
      execution: {
        version: 'semantic-execution-v2',
        evaluationOrder: [0],
        discardedEffects: [],
        terminal: null,
      },
      result: { tag: 'single', item: { node: 0, producerOccurrence: 0, identityOccurrence: 0, color: [0.25, 0.5, 0.75, 1] } },
      declaredCapabilities: languageContract === 'openscad-viewer/brep-1' ? ['geometry.brep'] : [],
      capabilityClosure: [],
      diagnosticTemplates: [],
    },
    provenance: [{ operation: 0, span: { start: 0, end: source.length }, label: `${name}()` }],
    tessellationIntents: [],
    diagnostics: [],
  })
}

function legacyBoxProgram(source = 'cube([1, 2, 3]);'): SemanticProgramV1 {
  return oneNodeProgram(source, 'legacy/current', {
    id: 0, kind: 'box', valueType: valueType('legacy/current', 'solid-set', 'd3'), size: [1, 2, 3], center: false,
  }, 'cube')
}

function brepSphereProgram(): SemanticProgramV1 {
  const source = '// @language openscad-viewer/brep-1\n// @requires geometry.brep\nsphere(2);'
  const program = clone(oneNodeProgram(source, 'openscad-viewer/brep-1', {
    id: 0, kind: 'sphere-analytic', valueType: valueType('openscad-viewer/brep-1', 'solid', 'd3'), radius: 2,
  }, 'sphere')) as any
  const start = source.indexOf('sphere')
  program.provenance[0].span = { start, end: source.length }
  program.tessellationIntents = [{
    occurrence: 0,
    chordTolerance: 0.05,
    angularToleranceDegrees: 5,
    minSegments: 8,
    maxSegments: 128,
  }]
  return finalize(program)
}

function addIndependentNode(
  program: Record<string, any>,
  node: Record<string, any>,
  name: string,
  color: number[] = [1, 1, 1, 1],
): number {
  const nodeId = program.core.nodes.length
  node.id = nodeId
  program.core.nodes.push(node)
  program.core.execution.evaluationOrder.push(nodeId)
  const operationId = program.core.operations.length
  const ordinal = program.core.operations.filter((candidate: SemanticStaticOperation) => (
    candidate.parent === null && candidate.category === 'geometry' && candidate.name === name
  )).length
  const op = operation(operationId, name, ordinal)
  program.core.operations.push(op)
  const occurrenceIndex = program.core.occurrences.length
  const occurrenceId = deriveSemanticOccurrenceId(null, null, op.operationId, [])
  program.core.occurrences.push({
    id: occurrenceIndex,
    occurrenceId,
    operation: operationId,
    parent: null,
    staticParent: null,
    dynamicSlots: [],
    node: nodeId,
    outputOrdinal: 0,
    sceneEntityId: deriveSemanticSceneEntityId(occurrenceId, 0),
  })
  program.provenance.push({
    operation: operationId,
    span: { ...program.provenance[0].span },
    label: `${name}()`,
  })
  const prior = program.core.result.tag === 'single'
    ? [program.core.result.item]
    : program.core.result.tag === 'multi' ? program.core.result.items : []
  program.core.result = { tag: 'multi', items: [...prior, {
    node: nodeId,
    producerOccurrence: occurrenceIndex,
    identityOccurrence: occurrenceIndex,
    color,
  }] }
  program.core.capabilityClosure = [...deriveSemanticCapabilityClosure(program.core)]
  return nodeId
}

describe('SemanticProgram SPC1/SPE1 schema', () => {
  it('normalizes a strict core/envelope pair into a recursively frozen snapshot', () => {
    const input = legacyBoxProgram()
    const program = normalizeSemanticProgram(input, 'cube([1, 2, 3]);')
    expect(program).toEqual(input)
    expect(Object.isFrozen(program)).toBe(true)
    expect(Object.isFrozen(program.core)).toBe(true)
    expect(Object.isFrozen(program.core.nodes[0])).toBe(true)
    expect(program.source).toEqual({
      sha256: 'dd5a1ad0cfa00eb9daa2820aed37fda7854f29b8d794c7e0c64c8b2d687f16b6',
      utf8ByteLength: 16,
      utf16CodeUnitLength: 16,
    })
  })

  it('rejects extras, unknown required features, noncanonical floats and ill-formed Unicode', () => {
    expect(() => normalizeSemanticProgram({ ...legacyBoxProgram(), engine: 'brep' })).toThrow(/unknown or missing fields/)
    const extraNode = clone(legacyBoxProgram()) as any
    extraNode.core.nodes[0].provider = 'manifold'
    expect(() => normalizeSemanticProgram(extraNode)).toThrow(/unknown or missing fields/)

    const future = clone(legacyBoxProgram()) as any
    future.core.requiredFeatures = ['future.tag']
    expect(() => normalizeSemanticProgram(future)).toThrow(/requires exactly semantic\.execution-v2/)

    for (const bad of [NaN, Infinity, -Infinity, -0]) {
      const invalid = clone(legacyBoxProgram()) as any
      invalid.core.nodes[0].size[0] = bad
      expect(() => normalizeSemanticProgram(invalid)).toThrow(SemanticProgramValidationError)
    }
    expect(() => semanticSourceDescriptor('\ud800')).toThrow(/well-formed Unicode/)
  })

  it('recomputes operation, occurrence and scene identities from structural data', () => {
    const program = clone(legacyBoxProgram()) as any
    program.core.operations[0].operationId = `opv1:${'0'.repeat(64)}`
    expect(() => normalizeSemanticProgram(program)).toThrow(/operation ID does not match/)

    const occurrence = clone(legacyBoxProgram()) as any
    occurrence.core.occurrences[0].dynamicSlots = [{
      name: 'i', value: { tag: 'number', value: 1 }, duplicateOrdinal: 0,
    }]
    expect(() => normalizeSemanticProgram(occurrence)).toThrow(/occurrence ID does not match/)

    const entity = clone(legacyBoxProgram()) as any
    entity.core.occurrences[0].sceneEntityId = `entity:v2:${'0'.repeat(64)}`
    expect(() => normalizeSemanticProgram(entity)).toThrow(/scene entity ID does not match/)
  })

  it('requires explicit ambiguity evidence for same-name siblings', () => {
    const program = clone(legacyBoxProgram()) as any
    addIndependentNode(program, { ...program.core.nodes[0] }, 'cube')
    expect(() => normalizeSemanticProgram(program)).toThrow(/same-name siblings require explicit/)

    const group = deriveSemanticAmbiguityGroupId([], 'geometry', 'cube')
    for (const op of program.core.operations) {
      op.identityEvidence = 'same-name-positional'
      op.ambiguityGroup = group
    }
    expect(() => normalizeSemanticProgram(program)).not.toThrow()
  })

  it('enforces backward typed DAG references, no implicit coercion and canonical postorder', () => {
    const forward = clone(legacyBoxProgram()) as any
    forward.core.nodes = [
      { id: 0, kind: 'transform', valueType: valueType('legacy/current', 'solid-set', 'd3'), input: 1, matrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1] },
      { id: 1, kind: 'box', valueType: valueType('legacy/current', 'solid-set', 'd3'), size: [1, 1, 1], center: false },
    ]
    expect(() => normalizeSemanticProgram(forward)).toThrow(/earlier node/)

    const projected = clone(lowerOpenSCADToSemanticProgram('projection() cube(1);').program) as any
    expect(() => normalizeSemanticProgram(projected)).not.toThrow()

    const inputProducer = projected.core.occurrences.findIndex((occurrence: any) => occurrence.node === 0)
    projected.core.result.item = { node: 0, producerOccurrence: inputProducer, identityOccurrence: inputProducer, color: [1, 1, 1, 1] }
    expect(() => normalizeSemanticProgram(projected)).toThrow(/reachable/)
  })

  it('uses explicit empty/single/multi cardinality and preserves authored duplicates', () => {
    const emptySource = '// no geometry'
    const empty = finalize({
      ...clone(legacyBoxProgram(emptySource)),
      core: {
        ...clone(legacyBoxProgram(emptySource)).core,
        operations: [], occurrences: [], nodes: [],
        execution: {
          version: 'semantic-execution-v2', evaluationOrder: [], discardedEffects: [], terminal: null,
        },
        result: { tag: 'empty', type: 'never' },
        declaredCapabilities: [], capabilityClosure: [], diagnosticTemplates: [],
      },
      provenance: [],
    } as any)
    expect(normalizeSemanticProgram(empty).core.result).toEqual({ tag: 'empty', type: 'never' })

    const duplicated = clone(lowerOpenSCADToSemanticProgram('cube([1,2,3]); cube([1,2,3]);').program) as any
    const normalized = normalizeSemanticProgram(duplicated)
    expect(normalized.core.nodes).toHaveLength(2)
    expect(normalized.core.result.tag).toBe('multi')
    expect(semanticProgramHash(normalized)).not.toBe(semanticProgramHash(legacyBoxProgram()))

    const malformed = clone(duplicated) as any
    malformed.core.result.items = [malformed.core.result.items[0]]
    expect(() => normalizeSemanticProgram(malformed)).toThrow(/multi requires at least two/)
  })

  it('makes kernel order and discarded effects complete, topological, and hash-covered', () => {
    const union = clone(lowerOpenSCADToSemanticProgram('union(){cube(1);sphere(1);}').program) as any
    expect(union.core.execution.evaluationOrder).toEqual([0, 1, 2])

    const missing = clone(union) as any
    missing.core.execution.evaluationOrder.pop()
    expect(() => normalizeSemanticProgram(missing)).toThrow(/every node exactly once/)

    const duplicate = clone(union) as any
    duplicate.core.execution.evaluationOrder = [0, 0, 2]
    expect(() => normalizeSemanticProgram(duplicate)).toThrow(/duplicate node/)

    const dependency = clone(union) as any
    dependency.core.execution.evaluationOrder = [2, 0, 1]
    expect(() => normalizeSemanticProgram(dependency)).toThrow(/before one of its inputs/)

    const effect = clone(lowerOpenSCADToSemanticProgram(
      'difference(){if(false)cube(1);cube(1);sphere(1);}',
    ).program) as any
    expect(effect.core.execution.discardedEffects).toMatchObject([{
      tag: 'legacy-difference-cutters', root: 2,
    }])
    const forgedRoot = clone(effect) as any
    forgedRoot.core.execution.discardedEffects[0].root = 0
    expect(() => normalizeSemanticProgram(forgedRoot)).toThrow(/discarded|cutter effect|effect/)

    const brepEffect = clone(brepSphereProgram()) as any
    brepEffect.core.execution.discardedEffects = [{
      tag: 'legacy-difference-cutters', root: 0, ownerOccurrence: 0,
    }]
    expect(() => normalizeSemanticProgram(brepEffect)).toThrow(/forbidden for brep-1/)
  })

  it('captures a deterministic language terminal behind an executable kernel prefix', () => {
    const artifact = lowerOpenSCADToSemanticProgram(
      'cube(1); assert(false, "after kernel");',
      { captureTerminalFailure: true },
    )
    expect(artifact.terminalError).toBeInstanceOf(OpenSCADParseError)
    expect(artifact.program.core.result).toEqual({ tag: 'empty', type: 'never' })
    expect(artifact.program.core.execution.terminal).toMatchObject({
      tag: 'legacy-language-error',
      diagnosticTemplate: 0,
      prefixFrontier: [{ root: 0, ownerOccurrence: 0 }],
    })

    const missingRoot = clone(artifact.program) as any
    missingRoot.core.execution.terminal.prefixFrontier = []
    expect(() => normalizeSemanticProgram(missingRoot)).toThrow(/maximal completed node/)

    const wrongTemplate = clone(artifact.program) as any
    wrongTemplate.core.diagnosticTemplates[0].severity = 'warning'
    expect(() => normalizeSemanticProgram(wrongTemplate)).toThrow(/error diagnostic template/)

    expect(() => lowerOpenSCADToSemanticProgram(
      'cube(1); assert(false, "after kernel");',
    )).toThrow(/Assertion 'false' failed: after kernel/)
  })

  it('replays an already-reduced difference base before a terminal cutter descendant', () => {
    const source = 'difference(){group(){cube(1);sphere(1);} assert(false,"x") cube(1);}'
    const artifact = lowerOpenSCADToSemanticProgram(source, { captureTerminalFailure: true })
    expect(artifact.program.core.nodes.map(node => node.kind)).toEqual([
      'box', 'sphere-polygonal', 'boolean',
    ])
    expect(artifact.program.core.execution.evaluationOrder).toEqual([0, 1, 2])
    expect(artifact.program.core.execution.terminal?.prefixFrontier).toEqual([{
      root: 2,
      ownerOccurrence: null,
    }])
    expect(() => normalizeSemanticProgram(artifact.program, source)).not.toThrow()
  })

  it('aggregates repeated runtime activations into one authored difference base bucket', () => {
    const source = 'difference(){for(i=[0:2]) cube(i+1); assert(false,"x") sphere(1);}'
    const artifact = lowerOpenSCADToSemanticProgram(source, { captureTerminalFailure: true })
    expect(artifact.program.core.nodes.map(node => node.kind)).toEqual([
      'box', 'box', 'box', 'boolean',
    ])
    expect(artifact.program.core.execution.evaluationOrder).toEqual([0, 1, 2, 3])
    expect(artifact.program.core.execution.terminal?.prefixFrontier).toEqual([{
      root: 3,
      ownerOccurrence: null,
    }])
    expect(() => normalizeSemanticProgram(artifact.program, source)).not.toThrow()
  })

  it('admits only the exact interrupted occurrence for an unsupported terminal operation', () => {
    const source = 'text("nope");'
    const artifact = lowerOpenSCADToSemanticProgram(source, { captureTerminalFailure: true })
    const terminal = artifact.program.core.execution.terminal!
    expect(artifact.program.core.operations[artifact.program.core.occurrences[terminal.occurrence].operation].name)
      .toBe('text')
    expect(() => normalizeSemanticProgram(artifact.program, source)).not.toThrow()
  })

  it('separates analytic B-rep nodes from materialized legacy polygonization', () => {
    expect(() => normalizeSemanticProgram(brepSphereProgram())).not.toThrow()
    const polygonalBrep = clone(brepSphereProgram()) as any
    polygonalBrep.core.nodes[0] = {
      id: 0, kind: 'sphere-polygonal', valueType: valueType('openscad-viewer/brep-1', 'solid', 'd3'), radius: 2, radialSegments: 32,
    }
    polygonalBrep.core.capabilityClosure = [...deriveSemanticCapabilityClosure(polygonalBrep.core)]
    expect(() => normalizeSemanticProgram(polygonalBrep)).toThrow(/brep-1 topology requires analytic/)

    const analyticLegacy = clone(legacyBoxProgram()) as any
    analyticLegacy.core.nodes[0] = { id: 0, kind: 'sphere-analytic', valueType: valueType('legacy/current', 'solid-set', 'd3'), radius: 1 }
    analyticLegacy.core.capabilityClosure = [...deriveSemanticCapabilityClosure(analyticLegacy.core)]
    expect(() => normalizeSemanticProgram(analyticLegacy)).toThrow(/legacy\/current requires materialized/)
  })

  it('fixes matrix layout/composition, preserves legacy singular multmatrix and applies the B-rep nonsingular rule', () => {
    const matrixSource = 'multmatrix(m=[[0,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]) cube(1);'
    const legacySingular = clone(lowerOpenSCADToSemanticProgram(matrixSource).program) as any
    expect(() => normalizeSemanticProgram(legacySingular)).not.toThrow()

    const singular = clone(legacySingular) as any
    singular.core.language.contract = 'openscad-viewer/brep-1'
    singular.core.language.semanticsRevision = 'brep-1.0.0'
    singular.core.declaredCapabilities = ['geometry.brep']
    singular.core.nodes[0].valueType = valueType('openscad-viewer/brep-1', 'solid', 'd3')
    singular.core.nodes[1].valueType = valueType('openscad-viewer/brep-1', 'solid', 'd3')
    singular.core.capabilityClosure = [...deriveSemanticCapabilityClosure(singular.core)]
    expect(() => normalizeSemanticProgram(singular)).toThrow(/nonsingular/)

    singular.core.nodes[1].matrix[0] = -1
    expect(() => normalizeSemanticProgram(singular)).not.toThrow()
  })

  it('binds UTF-16 spans to an exact UTF-8 source attestation', () => {
    const source = '/*🙂*/ cube(1);'
    const program = clone(legacyBoxProgram(source)) as any
    program.provenance[0].span = { start: 7, end: source.length }
    expect(normalizeSemanticProgram(program, source).source).toEqual({
      sha256: expect.stringMatching(/^[a-f0-9]{64}$/),
      utf8ByteLength: 17,
      utf16CodeUnitLength: 15,
    })
    expect(() => normalizeSemanticProgram(program, `${source} `)).toThrow(/source bytes do not match/)
    program.provenance[0].span.end = source.length + 1
    expect(() => normalizeSemanticProgram(program)).toThrow()
  })

  it('requires exact authored+inferred capability closure', () => {
    const program = clone(brepSphereProgram()) as any
    expect(program.core.capabilityClosure).toContain('construct.sphere.analytic')
    expect(program.core.capabilityClosure).toContain('geometry.brep')
    program.core.capabilityClosure.pop()
    expect(() => normalizeSemanticProgram(program)).toThrow(/closure does not exactly match/)
  })

  it('requires exact-source re-lowering for the pre-freeze flat prototype', () => {
    const current = legacyBoxProgram()
    const old = {
      schema: 'semantic-program',
      version: 0,
      languageContract: 'legacy/current',
      sourceHash: current.source.sha256,
      sourceUtf8Bytes: current.source.utf8ByteLength,
      sourceUtf16CodeUnits: current.source.utf16CodeUnitLength,
      nodes: [{ id: 0, kind: 'box', dimension: 'solid3', size: [1, 2, 3], center: false }],
      outputs: [{ node: 0, occurrence: 0, color: [0.25, 0.5, 0.75, 1] }],
      occurrences: [{ node: 0, span: { start: 0, end: current.source.utf16CodeUnitLength }, label: 'cube()' }],
      capabilities: [],
    }
    expect(() => normalizeSemanticProgram(old)).toThrow(/E_SEMANTIC_RELOWER_REQUIRED/)
    expect(() => normalizeSemanticProgram({ ...old, fallback: 'manifold' })).toThrow(/E_SEMANTIC_RELOWER_REQUIRED/)
  })
})

describe('SemanticProgram canonical codec and hash domains', () => {
  it('round-trips SPE1 and emits a distinct SPC1 core', () => {
    const program = brepSphereProgram()
    const envelope = encodeSemanticProgram(program)
    const core = encodeSemanticProgramCore(program)
    expect(new TextDecoder().decode(envelope.subarray(0, 4))).toBe('SPE1')
    expect(new TextDecoder().decode(core.subarray(0, 4))).toBe('SPC1')
    const decoded = decodeSemanticProgram(envelope)
    expect(decoded).toEqual(program)
    expect(encodeSemanticProgram(decoded)).toEqual(envelope)
  })

  it('keeps source/provenance and tessellation policy outside SPC1 programHash', () => {
    const first = clone(brepSphereProgram()) as any
    const second = clone(first) as any
    const source = '// @language openscad-viewer/brep-1\n// @requires geometry.brep\n          sphere(2);'
    second.source = semanticSourceDescriptor(source)
    second.provenance[0].span = { start: source.indexOf('sphere'), end: source.length }
    second.provenance[0].label = 'same semantic sphere'
    expect(normalizeSemanticProgram(second, source)).toEqual(second)
    expect(semanticProgramHash(second)).toBe(semanticProgramHash(first))
    expect(attestSemanticProgram(second).sourceHash).not.toBe(attestSemanticProgram(first).sourceHash)
    expect(attestSemanticProgram(second).envelopeBytesSha256).not.toBe(attestSemanticProgram(first).envelopeBytesSha256)
    expect(attestSemanticProgram(second).coreBytesSha256).toBe(attestSemanticProgram(first).coreBytesSha256)

    second.tessellationIntents[0].chordTolerance = 0.01
    expect(semanticProgramHash(second)).toBe(semanticProgramHash(first))
    expect(semanticTessellationPolicyHash(second)).not.toBe(semanticTessellationPolicyHash(first))

    second.core.occurrences[0].dynamicSlots = [{
      name: 'variant', value: { tag: 'number', value: 1 }, duplicateOrdinal: 0,
    }]
    second.core.occurrences[0].occurrenceId = deriveSemanticOccurrenceId(
      null,
      null,
      second.core.operations[0].operationId,
      second.core.occurrences[0].dynamicSlots,
    )
    second.core.occurrences[0].sceneEntityId = deriveSemanticSceneEntityId(second.core.occurrences[0].occurrenceId, 0)
    second.core.result.item.identityOccurrence = 0
    expect(semanticProgramHash(second)).not.toBe(semanticProgramHash(first))
  })

  it('domain-separates all public digests', () => {
    const attestation = attestSemanticProgram(legacyBoxProgram())
    expect(attestation).toEqual({
      semanticProgramVersion: 'semantic-program-contract-v1',
      binaryFormat: 'semantic-program-binary-v1',
      sourceHash: expect.stringMatching(/^[a-f0-9]{64}$/),
      programHash: expect.stringMatching(/^[a-f0-9]{64}$/),
      tessellationPolicyHash: expect.stringMatching(/^[a-f0-9]{64}$/),
      coreBytesSha256: expect.stringMatching(/^[a-f0-9]{64}$/),
      envelopeBytesSha256: expect.stringMatching(/^[a-f0-9]{64}$/),
      canonicalBytes: expect.any(Number),
    })
    expect(new Set([
      attestation.sourceHash,
      attestation.programHash,
      attestation.tessellationPolicyHash,
      attestation.coreBytesSha256,
      attestation.envelopeBytesSha256,
    ]).size).toBe(5)
  })

  it('rejects corruption, noncanonical binary64 and seeded random bytes', () => {
    const encoded = encodeSemanticProgram(legacyBoxProgram())
    const badMagic = new Uint8Array(encoded)
    badMagic[0] ^= 0xff
    expect(() => decodeSemanticProgram(badMagic)).toThrow(/expected SPE1/)
    const trailing = new Uint8Array(encoded.length + 1)
    trailing.set(encoded)
    expect(() => decodeSemanticProgram(trailing)).toThrow(/framed length/)

    let seed = 0x5eed1234
    for (let sample = 0; sample < 500; sample++) {
      seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0
      const bytes = new Uint8Array(seed % 128)
      for (let index = 0; index < bytes.length; index++) {
        seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0
        bytes[index] = seed & 0xff
      }
      expect(() => decodeSemanticProgram(bytes)).toThrow(SemanticProgramValidationError)
    }
  })

  it('rejects shared binary memory and keeps the in-place path codec-owned', () => {
    const encoded = encodeSemanticProgram(legacyBoxProgram())
    const shared = new Uint8Array(new SharedArrayBuffer(encoded.length))
    shared.set(encoded)
    expect(() => decodeSemanticProgram(shared)).toThrow(/shared binary buffers are forbidden/)

    const forged = clone(legacyBoxProgram()) as any
    let hash = forged.source.sha256
    Object.defineProperty(forged.source, 'sha256', {
      enumerable: true,
      get: () => hash,
    })
    expect(() => normalizeDecodedSemanticProgram(forged)).toThrow(/codec-owned decoded graph/)
    hash = '0'.repeat(64)
  })
})

describe('OpenSCAD exact-source semantic lowerer', () => {
  it.each([
    ['cube([1, 2, 3]);', ['box']],
    ['translate([1, 2, 3]) sphere(2);', ['sphere-polygonal', 'transform']],
    ['union() { cube(1); sphere(1); }', ['box', 'sphere-polygonal', 'boolean']],
    ['linear_extrude(height=2) square(1);', ['rectangle', 'linear-extrude']],
    ['if(true) cube(1);', ['box']],
    ['if(false) cube(1); else sphere(1);', ['sphere-polygonal']],
    ['module solid(x){ translate([x,0,0]) cube(1); } solid(2);', ['box', 'transform']],
    ['module wrap(){ translate([1,0,0]) children(); } wrap() cube(1);', ['box', 'transform']],
    ['module wrap(x){ translate([x,0,0]) children(); } wrap(1) cube(1); wrap(1) sphere(1);', ['box', 'transform', 'sphere-polygonal', 'transform']],
    ['module wrap(){ union(){ children(0); children(1); } } wrap(){ cube(1); sphere(1); }', ['box', 'sphere-polygonal', 'boolean']],
    ['module rec(n){ if(n <= 0) children(); else rec(n - 1) cube(1); } rec(2) sphere(1);', ['box']],
  ])('lowers legacy source deterministically: %s', (source, expectedKinds) => {
    const first = lowerOpenSCADToSemanticProgram(source)
    const second = lowerOpenSCADToSemanticProgram(source)
    expect(first.program.core.nodes.map(node => node.kind)).toEqual(expectedKinds)
    expect(first.canonicalBytes).toEqual(second.canonicalBytes)
    expect(first.attestation).toEqual(second.attestation)
    expect(requireTrustedSemanticLowering(first)).toBe(first)
    expect(() => requireTrustedSemanticLowering({ ...first })).toThrow(/artifact minted/)
    const bytes = first.canonicalBytes
    bytes[0] ^= 0xff
    expect(new TextDecoder().decode(first.canonicalBytes.subarray(0, 4))).toBe('SPE1')
  })

  it('pins recursive nested children failure in legacy while brep-1 uses the corrected continuation', () => {
    const body = 'module inner(){ translate([1,0,0]) children(); } module outer(){ inner() children(); } outer() cube(1);'
    expect(() => lowerOpenSCADToSemanticProgram(body)).toThrow(/Evaluation exceeds 128 nested calls/)
    const brep = lowerOpenSCADToSemanticProgram(
      `// @language openscad-viewer/brep-1\n// @requires geometry.brep\n${body}`,
    )
    expect(brep.program.core.nodes.map(node => node.kind)).toEqual(['box', 'transform'])
  })

  it.each([
    [
      'out-of-range nested child',
      'module m(){children();} module n(){m() children(9);} n(){cube(1);sphere(1);}',
      [],
    ],
    [
      'selected non-children sibling',
      'module m(){children();}m(){children(1);cube(1);}',
      ['box', 'box'],
    ],
  ])('matches finite legacy nested children selection: %s', (_name, source, expectedKinds) => {
    expect(lowerOpenSCADToSemanticProgram(source).program.core.nodes.map(node => node.kind))
      .toEqual(expectedKinds)
  })

  it('keeps B-rep analytic geometry in SPC1 and tessellation policy outside the core hash', () => {
    const source = '// @language openscad-viewer/brep-1\n// @requires geometry.brep\nsphere(2);'
    const lowered = lowerOpenSCADToSemanticProgram(source)
    expect(lowered.program.core.nodes).toMatchObject([{
      kind: 'sphere-analytic',
      valueType: { geometryKind: 'solid', space: 'd3', representation: 'analytic-brep' },
    }])
    expect(lowered.program.tessellationIntents).toHaveLength(1)
    expect(lowered.program.core.capabilityClosure).toContain('construct.sphere.analytic')
    expect(lowered.program.core.capabilityClosure).toContain('geometry.brep')
  })

  it('materializes canonical hidden union reducers for profiles and difference buckets', () => {
    const extruded = lowerOpenSCADToSemanticProgram(
      'linear_extrude(height=2){ square(1); translate([2,0,0]) square(1); }',
    )
    expect(extruded.program.core.nodes.map(node => node.kind)).toEqual([
      'rectangle', 'rectangle', 'transform', 'boolean', 'linear-extrude',
    ])
    expect((extruded.program.core.nodes[3] as any).operation).toBe('union')

    const difference = lowerOpenSCADToSemanticProgram(
      'difference(){ union(){ cube(1); translate([2,0,0]) cube(1); } sphere(1); translate([3,0,0]) sphere(1); }',
    )
    const root = difference.program.core.nodes.at(-1)
    expect(root).toMatchObject({ kind: 'boolean', operation: 'difference' })
    expect((root as any).inputs).toHaveLength(2)

    const partitioned = lowerOpenSCADToSemanticProgram(
      'difference(){ group(){ cube(); sphere(1); } group(){ translate([1,0,0]) cube(); sphere(0.5); } }',
    )
    expect(partitioned.program.core.nodes.at(-1)).toMatchObject({
      kind: 'boolean', operation: 'difference', inputs: expect.any(Array),
    })
  })
})

describe('SemanticProgram executable backend seam', () => {
  type StringPayloadFamily = { readonly [K in SemanticCarrierKey]: string }

  function sessionBackend(implementation: {
    validatePayload(carrierKey: SemanticCarrierKey, payload: unknown): boolean
    evaluate(
      node: SemanticNode,
      inputs: readonly any[],
      context: any,
    ): any
  }): SemanticProgramBackend<StringPayloadFamily> {
    return {
      begin() {
        const leases = new Map<SemanticBackendPayloadLease, unknown>()
        let closePromise: Promise<any> | undefined
        return {
          validatePayload(carrierKey, payload, lease) {
            return leases.has(lease) && leases.get(lease) === payload
              && implementation.validatePayload(carrierKey, payload)
          },
          async evaluate(node, inputs, context) {
            const returned = await implementation.evaluate(node, inputs, context)
            if (returned !== null && typeof returned === 'object') {
              const tag = Object.getOwnPropertyDescriptor(returned, 'tag')
              const payload = Object.getOwnPropertyDescriptor(returned, 'payload')
              const existingLease = Object.getOwnPropertyDescriptor(returned, 'lease')
              if (tag && 'value' in tag && (tag.value === 'value' || tag.value === 'materialized-empty')
                && payload && 'value' in payload && existingLease === undefined) {
                const lease = Object.freeze({}) as SemanticBackendPayloadLease
                leases.set(lease, payload.value)
                return { ...returned, lease }
              }
            }
            return returned
          },
          async releasePayload(lease) { leases.delete(lease) },
          close(outcome) {
            if (closePromise) return closePromise
            closePromise = Promise.resolve().then(() => {
              if (outcome.tag !== 'commit') {
                leases.clear()
                return { tag: 'closed' as const }
              }
              const retained = new Set(outcome.retained)
              for (const lease of leases.keys()) if (!retained.has(lease)) leases.delete(lease)
              let disposePromise: Promise<void> | undefined
              return {
                tag: 'committed' as const,
                resultLease: {
                  dispose() {
                    if (!disposePromise) disposePromise = Promise.resolve().then(() => {
                      for (const lease of retained) leases.delete(lease)
                    })
                    return disposePromise
                  },
                },
              }
            })
            return closePromise
          },
        }
      },
    }
  }

  interface CountedBackendState {
    beginCalls: number
    closeCalls: number
    disposeCalls: number
    allocated: number
    released: number
    live: Set<SemanticBackendPayloadLease>
    outcomes: any[]
  }

  function countedBackend(
    implementation: (
      node: SemanticNode,
      inputs: readonly any[],
      context: any,
      allocate: (node: SemanticNode, payload: string) => any,
    ) => any,
    options: { closeReject?: boolean; disposeReject?: boolean } = {},
  ): { backend: SemanticProgramBackend<StringPayloadFamily>; state: CountedBackendState } {
    const state: CountedBackendState = {
      beginCalls: 0,
      closeCalls: 0,
      disposeCalls: 0,
      allocated: 0,
      released: 0,
      live: new Set(),
      outcomes: [],
    }
    const backend: SemanticProgramBackend<StringPayloadFamily> = {
      begin() {
        state.beginCalls++
        const entries = new Map<SemanticBackendPayloadLease, string>()
        const releasePromises = new Map<SemanticBackendPayloadLease, Promise<void>>()
        const inFlight = new Set<Promise<any>>()
        let closePromise: Promise<any> | undefined
        const release = (lease: SemanticBackendPayloadLease): Promise<void> => {
          const existing = releasePromises.get(lease)
          if (existing) return existing
          const promise = Promise.resolve().then(() => {
            if (entries.delete(lease)) {
              state.live.delete(lease)
              state.released++
            }
          })
          releasePromises.set(lease, promise)
          return promise
        }
        const allocate = (node: SemanticNode, payload: string) => {
          const lease = Object.freeze({}) as SemanticBackendPayloadLease
          entries.set(lease, payload)
          state.live.add(lease)
          state.allocated++
          return {
            tag: 'value' as const,
            valueType: node.valueType,
            evidence: PRESERVING,
            payload,
            lease,
          }
        }
        return {
          validatePayload(_carrierKey, payload, lease) {
            return entries.get(lease) === payload
          },
          evaluate(node, inputs, context) {
            const pending = Promise.resolve().then(() => implementation(
              node, inputs, context, allocate,
            ))
            inFlight.add(pending)
            pending.then(() => inFlight.delete(pending), () => inFlight.delete(pending))
            return pending
          },
          releasePayload: release,
          close(outcome) {
            if (closePromise) return closePromise
            state.closeCalls++
            state.outcomes.push(outcome)
            closePromise = Promise.resolve().then(async () => {
              await Promise.allSettled([...inFlight])
              const retained = outcome.tag === 'commit' ? new Set(outcome.retained) : new Set()
              for (const lease of [...entries.keys()]) {
                if (!retained.has(lease)) await release(lease)
              }
              if (options.closeReject) {
                for (const lease of [...entries.keys()]) await release(lease)
                throw new Error('counted close failure')
              }
              if (outcome.tag !== 'commit') return { tag: 'closed' as const }
              let disposePromise: Promise<void> | undefined
              return {
                tag: 'committed' as const,
                resultLease: {
                  dispose() {
                    if (!disposePromise) {
                      state.disposeCalls++
                      disposePromise = Promise.resolve().then(async () => {
                        for (const lease of retained) await release(lease)
                        if (options.disposeReject) throw new Error('counted dispose failure')
                      })
                    }
                    return disposePromise
                  },
                },
              }
            })
            return closePromise
          },
        }
      },
    }
    return { backend, state }
  }

  const tracingBackend: SemanticProgramBackend<StringPayloadFamily> = sessionBackend({
    validatePayload: (_carrierKey, payload) => typeof payload === 'string',
    evaluate(node, inputs) {
      return {
        tag: 'value',
        valueType: node.valueType,
        evidence: { tag: 'representation-preserving' },
        payload: `${node.kind}(${inputs.map(input => input.tag === 'value' ? input.payload : 'empty').join(',')})`,
      } as any
    },
  })

  it('executes nodes in DAG order without giving source to the backend', async () => {
    const artifact = lowerOpenSCADToSemanticProgram('translate([4, 5, 6]) cube([1, 2, 3]);')
    const progress: number[] = []
    const result = await executeSemanticProgram(artifact, tracingBackend, {
      maxNodes: 2,
      onNode: completed => progress.push(completed),
    })
    expect(result.outputs[0].value).toEqual({
      tag: 'value',
      valueType: valueType('legacy/current', 'solid-set', 'd3'),
      evidence: { tag: 'representation-preserving' },
      payload: 'transform(box())',
    })
    expect(progress).toEqual([1, 2])
    expect(Object.isFrozen(result.outputs)).toBe(true)
    await result.dispose()
  })

  it('keeps execution limits, deadlines and cancellation outside programHash', async () => {
    const artifact = lowerOpenSCADToSemanticProgram('cube([1, 2, 3]);')
    const hash = semanticProgramHash(artifact.program)
    await expect(executeSemanticProgram(artifact, tracingBackend, { maxNodes: 0 })).rejects.toMatchObject({ code: 'E_SEMANTIC_BUDGET' })
    await expect(executeSemanticProgram(artifact, tracingBackend, { now: () => 10, deadlineAt: 10 })).rejects.toMatchObject({ code: 'E_SEMANTIC_DEADLINE' })
    await expect(executeSemanticProgram(artifact, tracingBackend, { shouldAbort: () => true })).rejects.toMatchObject({ code: 'E_SEMANTIC_ABORTED' })
    expect(semanticProgramHash(artifact.program)).toBe(hash)
  })

  it('rejects a backend value-type lie', async () => {
    await expect(executeSemanticProgram(lowerOpenSCADToSemanticProgram('cube([1, 2, 3]);'), sessionBackend({
      validatePayload: (_carrierKey, payload) => typeof payload === 'string',
      evaluate: () => ({
        tag: 'value',
        valueType: valueType('legacy/current', 'region', 'd2'),
        evidence: { tag: 'representation-preserving' },
        payload: 'wrong',
      }),
    }))).rejects.toBeInstanceOf(SemanticProgramExecutionError)
  })

  it('validates exact runtime records without depending on object insertion order', async () => {
    const artifact = lowerOpenSCADToSemanticProgram('cube(1);')
    const accepted = await executeSemanticProgram(artifact, sessionBackend({
      validatePayload: (_carrierKey, payload) => typeof payload === 'string',
      evaluate(node) {
        return {
          tag: 'value' as const,
          valueType: {
            evidence: { tag: 'representation-preserving' as const },
            representation: node.valueType.representation,
            space: node.valueType.space,
            geometryKind: node.valueType.geometryKind,
          },
          evidence: { tag: 'representation-preserving' as const },
          payload: 'box',
        }
      },
    }))
    expect(accepted.outputs[0].value.tag).toBe('value')
    await accepted.dispose()

    await expect(executeSemanticProgram(artifact, sessionBackend({
      validatePayload: () => true,
      evaluate: node => ({
        tag: 'value',
        valueType: { ...node.valueType, smuggled: undefined },
        evidence: { tag: 'representation-preserving' },
        payload: 'box',
      } as any),
    }))).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_TYPE' })

    let getterReads = 0
    await expect(executeSemanticProgram(artifact, sessionBackend({
      validatePayload: () => true,
      evaluate(node) {
        const value: Record<string, unknown> = {
          valueType: node.valueType,
          evidence: { tag: 'representation-preserving' },
          payload: 'box',
        }
        Object.defineProperty(value, 'tag', { enumerable: true, get: () => { getterReads++; return 'value' } })
        return value as any
      },
    }))).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_TYPE' })
    expect(getterReads).toBe(0)

    await expect(executeSemanticProgram(artifact, sessionBackend({
      validatePayload: () => true,
      evaluate: node => ({
        tag: 'value', valueType: node.valueType,
        evidence: { tag: 'representation-preserving' }, payload: null,
      } as any),
    }))).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_TYPE' })
  })

  it('applies provider-independent typed-empty reductions and postconditions', async () => {
    const transformed = lowerOpenSCADToSemanticProgram('translate([1, 0, 0]) polygon([[0,0], [1,0], [0,1]]);')
    const calls: string[] = []
    const transformedResult = await executeSemanticProgram(transformed, sessionBackend({
      validatePayload: (_carrierKey, payload) => typeof payload === 'string',
      evaluate(node) {
        calls.push(node.kind)
        if (node.kind === 'polygon') return {
          tag: 'empty' as const, valueType: node.valueType,
          evidence: { tag: 'representation-preserving' as const },
        }
        throw new Error('empty transform must be reduced before the backend')
      },
    }))
    expect(calls).toEqual(['polygon'])
    expect(transformedResult.outputs[0].value.tag).toBe('empty')
    await transformedResult.dispose()

    const union = lowerOpenSCADToSemanticProgram('union(){ polygon([[0,0],[1,0],[0,1]]); polygon([[0,0],[2,0],[0,2]]); }')
    const seenInputs: number[][] = []
    const unionResult = await executeSemanticProgram(union, sessionBackend({
      validatePayload: (_carrierKey, payload) => typeof payload === 'string',
      evaluate(node, inputs, context) {
        if (node.id === 0) return { tag: 'empty', valueType: node.valueType, evidence: PRESERVING }
        if (node.id === 1) return { tag: 'value', valueType: node.valueType, evidence: PRESERVING, payload: 'region' }
        seenInputs.push([...context.inputNodeIndices])
        expect(inputs).toHaveLength(1)
        return { tag: 'value', valueType: node.valueType, evidence: PRESERVING, payload: 'union' }
      },
    }))
    expect(seenInputs).toEqual([[1]])
    expect(unionResult.outputs[0].value).toMatchObject({ tag: 'value', payload: 'union' })
    await unionResult.dispose()

    await expect(executeSemanticProgram(lowerOpenSCADToSemanticProgram('cube(1);'), sessionBackend({
      validatePayload: () => true,
      evaluate: node => ({ tag: 'empty', valueType: node.valueType, evidence: PRESERVING }),
    }))).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_TYPE' })

    await expect(executeSemanticProgram(lowerOpenSCADToSemanticProgram('union(){ cube(1); sphere(1); }'), sessionBackend({
      validatePayload: (_carrierKey, payload) => typeof payload === 'string',
      evaluate: node => node.kind === 'boolean'
        ? { tag: 'empty', valueType: node.valueType, evidence: PRESERVING }
        : { tag: 'value', valueType: node.valueType, evidence: PRESERVING, payload: node.kind },
    }))).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_TYPE' })
  })

  it('admits leased materialized emptiness only for the legacy runtime contract', async () => {
    const materializedBackend = sessionBackend({
      validatePayload: (_carrierKey, payload) => payload === 'empty-shape',
      evaluate: node => ({
        tag: 'materialized-empty',
        valueType: node.valueType,
        evidence: PRESERVING,
        payload: 'empty-shape',
      }),
    })
    const legacy = await executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('polygon([[0,0],[1,0]]);'),
      materializedBackend,
    )
    expect(legacy.outputs[0].value.tag).toBe('materialized-empty')
    await legacy.dispose()

    await expect(executeSemanticProgram(lowerOpenSCADToSemanticProgram(
      '// @language openscad-viewer/brep-1\n// @requires geometry.brep\ncube(1);',
    ), materializedBackend)).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_TYPE' })
  })

  it('enforces finite clocks and interrupts a non-settling backend deadline', async () => {
    const artifact = lowerOpenSCADToSemanticProgram('cube(1);')
    await expect(executeSemanticProgram(artifact, tracingBackend, {
      deadlineAt: 1,
      now: () => Number.NaN,
    })).rejects.toMatchObject({ code: 'E_SEMANTIC_DEADLINE' })

    let backendSawAbort = false
    await expect(executeSemanticProgram(artifact, sessionBackend({
      validatePayload: () => true,
      evaluate(_node, _inputs, context) {
        context.signal.addEventListener('abort', () => { backendSawAbort = true }, { once: true })
        return new Promise(() => {})
      },
    }), {
      deadlineAt: performance.now() + 15,
    })).rejects.toMatchObject({ code: 'E_SEMANTIC_DEADLINE' })
    expect(backendSawAbort).toBe(true)
  })

  it('rechecks a deadline before backend start and after payload validation', async () => {
    const artifact = lowerOpenSCADToSemanticProgram('cube(1);')
    let evaluateCalls = 0
    const beforeStartSamples = [0, 0, 10]
    await expect(executeSemanticProgram(artifact, sessionBackend({
      validatePayload: () => true,
      evaluate(node) {
        evaluateCalls++
        return { tag: 'value', valueType: node.valueType, evidence: PRESERVING, payload: 'box' }
      },
    }), {
      deadlineAt: 10,
      now: () => beforeStartSamples.shift() ?? 10,
    })).rejects.toMatchObject({ code: 'E_SEMANTIC_DEADLINE' })
    expect(evaluateCalls).toBe(0)

    let payloadChecks = 0
    const afterPayloadSamples = [0, 0, 0, 0, 10]
    await expect(executeSemanticProgram(artifact, sessionBackend({
      validatePayload: () => { payloadChecks++; return true },
      evaluate: node => ({
        tag: 'value', valueType: node.valueType, evidence: PRESERVING, payload: 'box',
      }),
    }), {
      deadlineAt: 10,
      now: () => afterPayloadSamples.shift() ?? 10,
    })).rejects.toMatchObject({ code: 'E_SEMANTIC_DEADLINE' })
    expect(payloadChecks).toBe(1)
  })

  it('transfers only root leases and disposes them exactly once', async () => {
    const counted = countedBackend((node, inputs, _context, allocate) => allocate(
      node,
      `${node.kind}(${inputs.map(input => input.tag === 'value' ? input.payload : 'empty').join(',')})`,
    ))
    const result = await executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('translate([1, 2, 3]) cube(1);'),
      counted.backend,
    )
    expect(counted.state).toMatchObject({
      beginCalls: 1, closeCalls: 1, allocated: 2, released: 1, disposeCalls: 0,
    })
    expect(counted.state.live.size).toBe(1)
    expect(counted.state.outcomes[0]).toMatchObject({ tag: 'commit' })
    expect(counted.state.outcomes[0].retained).toHaveLength(1)

    const first = result.dispose()
    const concurrent = result.dispose()
    expect(first).toBe(concurrent)
    expect(result.disposed).toBe(true)
    expect(result[Symbol.asyncDispose]()).toBe(first)
    await first
    expect(counted.state).toMatchObject({ released: 2, disposeCalls: 1 })
    expect(counted.state.live.size).toBe(0)
  })

  it('failure-close reclaims visible and hidden allocations after a later-node throw', async () => {
    const counted = countedBackend((node, _inputs, _context, allocate) => {
      const value = allocate(node, node.kind)
      if (node.id === 1) throw new Error('allocated then threw')
      return value
    })
    await expect(executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('translate([1, 0, 0]) cube(1);'),
      counted.backend,
    )).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_FAILURE', node: 1 })
    expect(counted.state).toMatchObject({
      beginCalls: 1, closeCalls: 1, allocated: 2, released: 2, disposeCalls: 0,
    })
    expect(counted.state.live.size).toBe(0)
    expect(counted.state.outcomes[0]).toMatchObject({
      tag: 'failure', code: 'E_SEMANTIC_BACKEND_FAILURE', node: 1,
    })
  })

  it('failure-close owns invalid and foreign returned leases', async () => {
    const counted = countedBackend((node, _inputs, _context, allocate) => {
      const registered = allocate(node, 'box')
      return { ...registered, lease: Object.freeze({}) as SemanticBackendPayloadLease }
    })
    await expect(executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('cube(1);'),
      counted.backend,
    )).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_TYPE', node: 0 })
    expect(counted.state).toMatchObject({ allocated: 1, released: 1, closeCalls: 1 })
    expect(counted.state.live.size).toBe(0)
  })

  it('rejects duplicate leases and releases the registered ownership once', async () => {
    const live = new Set<SemanticBackendPayloadLease>()
    const lease = Object.freeze({}) as SemanticBackendPayloadLease
    let released = 0
    let calls = 0
    const backend: SemanticProgramBackend<StringPayloadFamily> = {
      begin() {
        live.add(lease)
        return {
          validatePayload: (_key, payload, candidate) => candidate === lease && typeof payload === 'string',
          evaluate(node) {
            calls++
            return {
              tag: 'value', valueType: node.valueType, evidence: PRESERVING,
              payload: `value-${calls}`, lease,
            } as any
          },
          async releasePayload(candidate) {
            if (live.delete(candidate)) released++
          },
          async close(outcome) {
            if (outcome.tag === 'commit') throw new Error('duplicate lease must not commit')
            if (live.delete(lease)) released++
            return { tag: 'closed' }
          },
        }
      },
    }
    await expect(executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('translate([1, 0, 0]) cube(1);'),
      backend,
    )).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_TYPE', node: 1 })
    expect(released).toBe(1)
    expect(live.size).toBe(0)
  })

  it('closes leases when progress callbacks fail and commits empty without inventing leases', async () => {
    const failed = countedBackend((node, _inputs, _context, allocate) => allocate(node, node.kind))
    await expect(executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('translate([1, 0, 0]) cube(1);'),
      failed.backend,
      { onNode: () => { throw new Error('UI callback failed') } },
    )).rejects.toMatchObject({ code: 'E_SEMANTIC_CALLBACK', node: 0 })
    expect(failed.state).toMatchObject({ allocated: 1, released: 1, closeCalls: 1 })
    expect(failed.state.live.size).toBe(0)

    const empty = countedBackend(node => ({
      tag: 'empty', valueType: node.valueType, evidence: PRESERVING,
    }))
    const result = await executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('translate([1, 0, 0]) polygon([[0,0],[1,0],[0,1]]);'),
      empty.backend,
    )
    expect(result.outputs[0].value.tag).toBe('empty')
    expect(empty.state.outcomes[0]).toMatchObject({ tag: 'commit', retained: [] })
    const first = result.dispose()
    expect(result.dispose()).toBe(first)
    await first
    expect(empty.state).toMatchObject({ allocated: 0, released: 0, disposeCalls: 1 })
  })

  it('quiescent close waits for a value that settles after deadline and releases it', async () => {
    let signalObserved = false
    const counted = countedBackend((node, _inputs, context, allocate) => new Promise(resolve => {
      context.signal.addEventListener('abort', () => {
        signalObserved = true
        setTimeout(() => resolve(allocate(node, 'late-box')), 5)
      }, { once: true })
    }))
    const started = performance.now()
    await expect(executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('cube(1);'),
      counted.backend,
      { deadlineAt: performance.now() + 10 },
    )).rejects.toMatchObject({ code: 'E_SEMANTIC_DEADLINE', node: 0 })
    expect(performance.now() - started).toBeGreaterThanOrEqual(10)
    expect(signalObserved).toBe(true)
    expect(counted.state).toMatchObject({ allocated: 1, released: 1, closeCalls: 1 })
    expect(counted.state.live.size).toBe(0)
  })

  it('does not begin before trust, budget, or control checks succeed', async () => {
    const counted = countedBackend((node, _inputs, _context, allocate) => allocate(node, 'box'))
    const artifact = lowerOpenSCADToSemanticProgram('cube(1);')
    await expect(executeSemanticProgram(artifact, counted.backend, { maxNodes: 0 }))
      .rejects.toMatchObject({ code: 'E_SEMANTIC_BUDGET' })
    await expect(executeSemanticProgram(artifact, counted.backend, { shouldAbort: () => true }))
      .rejects.toMatchObject({ code: 'E_SEMANTIC_ABORTED' })
    await expect(executeSemanticProgram({ ...artifact } as any, counted.backend))
      .rejects.toThrow(/artifact minted/)
    expect(counted.state.beginCalls).toBe(0)
    expect(counted.state.closeCalls).toBe(0)
  })

  it('treats begin and commit-close failures as typed terminal failures', async () => {
    let providerLive = 0
    await expect(executeSemanticProgram(lowerOpenSCADToSemanticProgram('cube(1);'), {
      begin() {
        providerLive++
        providerLive--
        throw new Error('atomic begin failed after internal cleanup')
      },
    })).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_BEGIN', node: null })
    expect(providerLive).toBe(0)

    let publishedLive = 0
    let publishedCloseCalls = 0
    await expect(executeSemanticProgram(lowerOpenSCADToSemanticProgram('cube(1);'), {
      begin() {
        publishedLive++
        const session: Record<string, unknown> = {
          validatePayload: () => true,
          releasePayload: async () => {},
          close: async () => {
            publishedCloseCalls++
            publishedLive = 0
            return { tag: 'closed' }
          },
        }
        Object.defineProperty(session, 'evaluate', {
          enumerable: true,
          get: () => { throw new Error('invalid evaluate accessor') },
        })
        return session as any
      },
    })).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_FAILURE', node: null })
    expect(publishedCloseCalls).toBe(1)
    expect(publishedLive).toBe(0)

    const counted = countedBackend(
      (node, _inputs, _context, allocate) => allocate(node, 'box'),
      { closeReject: true },
    )
    await expect(executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('cube(1);'),
      counted.backend,
    )).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_CLOSE', node: null })
    expect(counted.state.live.size).toBe(0)
    expect(counted.state).toMatchObject({ allocated: 1, released: 1, closeCalls: 1 })

    const failedClose = countedBackend(() => { throw new Error('primary evaluation failure') }, {
      closeReject: true,
    })
    let primary: unknown
    try {
      await executeSemanticProgram(lowerOpenSCADToSemanticProgram('cube(1);'), failedClose.backend)
    } catch (error) {
      primary = error
    }
    expect(primary).toMatchObject({
      code: 'E_SEMANTIC_BACKEND_FAILURE',
      backendCause: expect.any(Error),
      cleanupError: expect.any(Error),
    })
    expect(failedClose.state.closeCalls).toBe(1)
  })

  it('disposes transferred roots when a committed close record is malformed', async () => {
    const lease = Object.freeze({}) as SemanticBackendPayloadLease
    let live = 0
    let disposeCalls = 0
    const backend: SemanticProgramBackend<StringPayloadFamily> = {
      begin() {
        return {
          validatePayload: (_key, payload, candidate) => candidate === lease && payload === 'box',
          evaluate(node) {
            live = 1
            return {
              tag: 'value', valueType: node.valueType, evidence: PRESERVING,
              payload: 'box', lease,
            } as any
          },
          async releasePayload() {},
          async close(outcome) {
            if (outcome.tag !== 'commit') {
              live = 0
              return { tag: 'closed' }
            }
            return {
              tag: 'committed',
              resultLease: {
                dispose() {
                  disposeCalls++
                  live = 0
                  return Promise.resolve()
                },
              },
              unexpected: true,
            } as any
          },
        }
      },
    }
    await expect(executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('cube(1);'),
      backend,
    )).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_CLOSE' })
    expect(disposeCalls).toBe(1)
    expect(live).toBe(0)
  })

  it('memoizes a rejected result disposal without a second free attempt', async () => {
    const counted = countedBackend(
      (node, _inputs, _context, allocate) => allocate(node, 'box'),
      { disposeReject: true },
    )
    const result = await executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('cube(1);'),
      counted.backend,
    )
    const first = result.dispose()
    const second = result.dispose()
    expect(first).toBe(second)
    await expect(first).rejects.toThrow(/counted dispose failure/)
    expect(result.dispose()).toBe(first)
    await expect(result.dispose()).rejects.toThrow(/counted dispose failure/)
    expect(counted.state).toMatchObject({ allocated: 1, released: 1, disposeCalls: 1 })
    expect(counted.state.live.size).toBe(0)
  })

  it('rejects structurally valid but untrusted SPE1 envelopes at the execution boundary', async () => {
    await expect(executeSemanticProgram(legacyBoxProgram() as any, tracingBackend)).rejects.toThrow(/artifact minted/)
    const artifact = lowerOpenSCADToSemanticProgram('cube([1, 2, 3]);')
    expect(Object.isFrozen(artifact.program.core.nodes[0])).toBe(true)
    await expect(executeSemanticProgram({ ...artifact } as any, tracingBackend)).rejects.toThrow(/artifact minted/)
    const other = lowerOpenSCADToSemanticProgram('sphere(1);')
    await expect(executeSemanticProgram({ ...artifact, program: other.program } as any, tracingBackend))
      .rejects.toThrow(/artifact minted/)

    const unchecked = lowerOpenSCADToSemanticProgramUnchecked('cube(1);')
    expect(() => requireTrustedSemanticLowering(unchecked)).toThrow(/artifact minted/)
  })

  it('keeps the lexical trust registry private under post-import prototype changes', () => {
    const originalAdd = WeakSet.prototype.add
    const originalHas = WeakSet.prototype.has
    let capturedRegistry: WeakSet<object> | null = null
    try {
      WeakSet.prototype.add = function (value: object) {
        capturedRegistry = this
        return originalAdd.call(this, value)
      }
      const artifact = lowerOpenSCADToSemanticProgram('cube(1);')
      expect(capturedRegistry).toBeNull()

      WeakSet.prototype.has = () => true
      expect(() => requireTrustedSemanticLowering({ ...artifact })).toThrow(/artifact minted/)
      expect(requireTrustedSemanticLowering(artifact)).toBe(artifact)
    } finally {
      WeakSet.prototype.add = originalAdd
      WeakSet.prototype.has = originalHas
    }
  })
})
