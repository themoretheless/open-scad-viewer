import { beforeAll, describe, expect, it } from 'vitest'
import { warmGeometryKernel } from '../src/kernel/geometry/kernel'
import { createBrepBox, tessellateNurbsBrep } from '../src/kernel/geometry/brep'
import { parseDirectDocument, type DirectDocument } from '../src/app/directModeling'
import { stringifyMeshJson } from '../src/exchange/meshJson'
import { applySolidFullRound, restoreSolidFullRoundBodies } from '../src/kernel/solidFullRound'
import { canRedoFullRound, canUndoFullRound, isDirectExactFullRound } from '../src/app/directExactFullRound'
import { MAIN_SOLID_JOB_KINDS } from '../src/kernel/mainSolidProtocol'

beforeAll(() => warmGeometryKernel())

function boxDocument(): DirectDocument {
  const brep = createBrepBox([0, 0, 0], [4, 8, 6])
  const mesh = tessellateNurbsBrep(brep, 4)
  return parseDirectDocument(
    stringifyMeshJson({
      version: 1,
      sketches: [],
      bodies: [{ id: 'b', name: 'Box', group: 'g', mesh: { positions: mesh.positions, indices: mesh.indices }, brep }],
      groups: [{ name: 'g', source: '' }],
    }),
  )
}
const options = { id: 'b', action: 'create' as const, faces: [5, 4, 3] as [number, number, number], segments: 4 }

describe('Full Round Solid document edit', () => {
  it('replaces the B-rep with a native history and a closed display mesh, keeping body identity', () => {
    const before = boxDocument()
    const after = applySolidFullRound(before, options)
    const body = after.bodies[0]!
    expect(body).toMatchObject({ id: 'b', name: 'Box', group: 'g' })
    expect(body.brep).toBeUndefined()
    expect(isDirectExactFullRound(body.exactFullRound)).toBe(true)
    expect(body.exactFullRound!.history.revisions).toHaveLength(1)
    expect(body.mesh.indices.length).toBeGreaterThan(0)
    expect(canUndoFullRound(body)).toBe(false)
    expect(canRedoFullRound(body)).toBe(false)
    // The source document is untouched: the app history keeps it for Undo.
    expect(before.bodies[0]!.brep).toBeDefined()
  }, 60000)

  it('survives a save and restores by native replay, rebuilding the display cache', () => {
    const saved = stringifyMeshJson(applySolidFullRound(boxDocument(), options))
    const parsed = parseDirectDocument(saved)
    const stale = structuredClone(parsed)
    stale.bodies[0]!.mesh = {
      positions: new Float64Array([0, 0, 0, 1, 0, 0, 0, 1, 0]),
      indices: new Uint32Array([0, 1, 2]),
    }
    const restored = restoreSolidFullRoundBodies(stale)
    expect(restored.bodies[0]!.exactFullRound).toEqual(parsed.bodies[0]!.exactFullRound)
    expect(Array.from(restored.bodies[0]!.mesh.indices)).toEqual(Array.from(parsed.bodies[0]!.mesh.indices))
  }, 60000)

  it('refuses forged histories, invalid inputs and mixed B-rep caches', () => {
    const applied = applySolidFullRound(boxDocument(), options)
    const forged = structuredClone(applied)
    const artifact = forged.bodies[0]!.exactFullRound!.history.revisions[0]!.artifact as any
    artifact.geometry.vertices[0].point[0] = ['999', '1']
    expect(() => restoreSolidFullRoundBodies(forged)).toThrow()
    expect(() => applySolidFullRound(boxDocument(), { ...options, faces: [5, 5, 3] })).toThrow(/three distinct/)
    expect(() => applySolidFullRound(boxDocument(), { ...options, segments: 0 })).toThrow(/1\.\.32/)
    expect(() => applySolidFullRound(applied, options)).toThrow(/B-rep body/)
    expect(() => applySolidFullRound(boxDocument(), { ...options, action: 'undo' })).toThrow(/Full Round body/)
    expect(() => applySolidFullRound(applied, { ...options, action: 'undo' })).toThrow()
    const mixed = JSON.parse(stringifyMeshJson(applied))
    mixed.bodies[0].brep = JSON.parse(stringifyMeshJson(boxDocument())).bodies[0].brep
    expect(() => parseDirectDocument(JSON.stringify(mixed))).toThrow(/Full Round/)
    const shape = JSON.parse(stringifyMeshJson(applied))
    shape.bodies[0].exactFullRound.history.cursor = 3
    expect(() => parseDirectDocument(JSON.stringify(shape))).toThrow(/Full Round/)
  }, 60000)

  it('is a registered Solid worker job', () => {
    expect(MAIN_SOLID_JOB_KINDS.fullRound).toBe(true)
  })
})
