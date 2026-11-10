import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { deriveSemanticCapabilityClosure, deriveSemanticOperationId, deriveSemanticOccurrenceId, deriveSemanticSceneEntityId, semanticNodeInputs, semanticNodeProducerOperationNames } from '../src/core/semanticProgram'
import type { SemanticProgramCoreV1 } from '../src/core/semanticProgram'
import { validateNodes } from '../src/language/semantic/semanticProgramValidator/validateNodes'
import { validateOperations } from '../src/language/semantic/semanticProgramValidator/fail'
import { validateOccurrenceProduction } from '../src/language/semantic/semanticProgramValidator/validateOccurrenceProduction'
import { remapSemanticNode } from '../src/language/semantic/lowererBuilder'
import { nodeMustReturnValueAfterReduction } from '../src/language/semantic/semanticProgramExecutor/checkSemanticExecutionControl'

function fixture(): SemanticProgramCoreV1 {
  const core = JSON.parse(readFileSync(new URL('../crates/geometry-bridge/src/brep_production/fixtures/nestedTransform.json', import.meta.url), 'utf8'))
  core.operations[0].name = 'full_round_exact'
  core.operations[0].category = 'geometry'
  for (const op of core.operations) {
    op.structuralPath[0].name = 'full_round_exact'
    op.operationId = deriveSemanticOperationId(op.structuralPath)
  }
  core.nodes[2] = { id: 2, kind: 'full-round-exact', input: 1, faces: [0, 1, 2], valueType: { ...core.nodes[2].valueType, representation: 'rational-brep' } }
  core.result.item.identityOccurrence = 0
  for (const row of core.occurrences) {
    row.occurrenceId = deriveSemanticOccurrenceId(row.parent === null ? null : core.occurrences[row.parent].occurrenceId,
      row.staticParent === null ? null : core.occurrences[row.staticParent].occurrenceId, core.operations[row.operation].operationId, row.dynamicSlots)
    if (row.outputOrdinal !== null) row.sceneEntityId = deriveSemanticSceneEntityId(row.occurrenceId, row.outputOrdinal)
  }
  core.declaredCapabilities = []
  core.language = { contract: 'openscad-viewer/brep-1', semanticsRevision: 'brep-1.0.0', capabilityGraphVersion: 'semantic-capabilities-v1' }
  core.diagnosticTemplates = []
  return core
}
function admit(core: SemanticProgramCoreV1) {
  validateOperations(core.operations)
  validateNodes(core.nodes, 'openscad-viewer/brep-1')
  validateOccurrenceProduction(core.operations, core.occurrences, core.nodes, core.result)
}

describe('exact Full Round semantic contract', () => {
  it('admits indexed rational output with its own production identity', () => {
    const core = fixture()
    expect(() => admit(core)).not.toThrow()
    expect(semanticNodeInputs(core.nodes[2])).toEqual([1])
    expect(semanticNodeProducerOperationNames(core.nodes[2])).toEqual(['full_round_exact'])
    expect(deriveSemanticCapabilityClosure(core)).toContain('operation.full-round-exact')
    expect(deriveSemanticCapabilityClosure(core)).toContain('representation.transition.analytic-brep.to.rational-brep')
    expect(remapSemanticNode(core.nodes[2], 3, new Int32Array([4, 5, 6]))).toMatchObject({ id: 3, input: 5 })
    expect(nodeMustReturnValueAfterReduction(core.nodes[2])).toBe(true)
  })
  it.each([[0, 1], [0, 1, 2, 3], [0, -1, 2], [0, 1.5, 2], [0, '1', 2], [0, 0, 2]])('refuses malformed face selection %j', (...faces) => {
    const core: any = fixture()
    core.nodes[2].faces = faces
    expect(() => admit(core)).toThrow()
  })
  it('validates explicit material budgets and rejects unknown fields', () => {
    const core: any = fixture()
    core.nodes[2].limits = { maxPairProjections: 0, refinements: 192 }
    core.nodes[2].maxBytes = 4194304
    expect(() => admit(core)).not.toThrow()
    for (const limits of [{ extra: 1 }, { refinements: -1 }, { maxAtoms: 1.5 }, { maxRayCells: 2 ** 32 }, null]) {
      core.nodes[2].limits = limits
      expect(() => admit(core)).toThrow()
    }
  })
  it('refuses false carriers, wrong frontier inputs and inherited identity', () => {
    const core: any = fixture()
    core.nodes[2].valueType.representation = 'analytic-brep'
    expect(() => admit(core)).toThrow()
    const wrong: any = fixture()
    wrong.nodes[2].input = 0
    expect(() => admit(wrong)).toThrow()
    const identity: any = fixture()
    identity.result.item.identityOccurrence = 4
    expect(() => admit(identity)).toThrow()
    expect(() => validateNodes(fixture().nodes, 'legacy/current')).toThrow()
  })
})

import { lowerOpenSCADToSemanticProgram } from '../src/language/semantic/semanticProgramLowerer'
import { normalizeSemanticProgram } from '../src/language/semantic/semanticProgramValidator'
import { encodeSemanticProgram, decodeSemanticProgram } from '../src/language/semantic/semanticProgramCodec'

describe('exact Full Round source lowering', () => {
  const header = '// @language openscad-viewer/brep-1\n// @requires geometry.brep\n'
  it('lowers source into a validated codec-roundtrippable rational result', () => {
    const artifact = lowerOpenSCADToSemanticProgram(header + 'full_round_exact(faces=[0,1,2]) cube([4,8,6]);')
    const program = normalizeSemanticProgram(artifact.program)
    expect(program.core.nodes.map(node => node.kind)).toEqual(['box', 'full-round-exact'])
    expect(program.core.capabilityClosure).toContain('representation.transition.analytic-brep.to.rational-brep')
    expect(decodeSemanticProgram(encodeSemanticProgram(program))).toEqual(program)
    expect(program.core.result).toMatchObject({ tag: 'single', item: { node: 1, identityOccurrence: 0 } })
  })
  it('retains and remaps dependencies across a user module', () => {
    const program = lowerOpenSCADToSemanticProgram(header + 'module rounded(){full_round_exact([0,1,2]) cube([4,8,6]);} rounded();').program
    expect(program.core.nodes).toMatchObject([{ kind: 'box' }, { kind: 'full-round-exact', input: 0 }])
    expect(() => normalizeSemanticProgram(program)).not.toThrow()
  })
  it('rejects legacy source, invalid indices and a 2D child', () => {
    for (const source of ['full_round_exact([0,1,2]) cube(1);', header + 'full_round_exact([0,0,2]) cube(1);', header + 'full_round_exact([0,1,2]) square(1);']) {
      expect(() => lowerOpenSCADToSemanticProgram(source)).toThrow()
    }
  })
})


it('keeps separate materializer identities for multiple root-map outputs', () => {
  const code='// @language openscad-viewer/brep-1\nfull_round_exact([5,4,3]) {cube([4,8,6]);cube([4,8,7]);}'
  const program=lowerOpenSCADToSemanticProgram(code).program
  expect(program.core.result.tag).toBe('multi')
  expect(() => normalizeSemanticProgram(program)).not.toThrow()
  const bad:any=structuredClone(program)
  bad.core.result.items[1].identityOccurrence=bad.core.result.items[0].identityOccurrence
  expect(() => normalizeSemanticProgram(bad)).toThrow()
})
