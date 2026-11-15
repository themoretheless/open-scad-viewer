import { expect, it } from 'vitest'
import { createMainSolidWorkerHandler } from '../src/kernel/mainSolidWorkerRuntime'
import type { MainSolidResponse } from '../src/kernel/mainSolidProtocol'
import { applyDirectDocumentDelta } from '../src/kernel/directDocumentDelta'
import { DirectHistory, extrudeDirectSketch, serializeDirectDocument, type DirectDocument } from '../src/app/directModeling'
import { createSolidInstance } from '../src/kernel/solidInstances'
import { stringifyMeshJson } from '../src/exchange/meshJson'

const square = { id: 's', name: 'Square', closed: true, points: [[0, 0], [10, 0], [10, 10], [0, 10]] as [number, number][] }
const scene = (): DirectDocument => {
  let document: DirectDocument = { version: 1, sketches: [square], bodies: [extrudeDirectSketch(square, 10, 'source')], curves: [], surfaces: [] }
  for (let i = 0; i < 3; i++)
    document = createSolidInstance(document, 'source', `linked-${i}`, [[1, 0, 0, 20 * (i + 1)], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]])
  return document
}
const move = (x: number) => ({ operation: 'transform' as const, id: 'source', ids: ['source'], createdId: '', x, y: 0, z: 0, axis: 'z' as const, angle: 0, scale: 1 })

it('a resident drag base gives the same documents as sending the whole document every move', async () => {
  const document = scene()
  const messages: MainSolidResponse[] = [],
    handle = createMainSolidWorkerHandler(message => messages.push(message))
  await handle({ version: 1, id: 1, job: { kind: 'sceneEdit', document, options: move(3) } })
  await handle({ version: 1, id: 2, job: { kind: 'sceneEdit', document: serializeDirectDocument(document), options: move(3), retainAs: 'k1', delta: true } })
  await handle({ version: 1, id: 3, job: { kind: 'sceneEdit', document, options: move(7) } })
  await handle({ version: 1, id: 4, job: { kind: 'sceneEdit', document: { resident: 'k1' }, options: move(7), delta: true } })
  await handle({ version: 1, id: 5, job: { kind: 'sceneEdit', document: { resident: 'other' }, options: move(7), delta: true } })
  const result = (i: number) => (messages[i] as { result: unknown }).result
  const viaDelta = (i: number) => applyDirectDocumentDelta(document, result(i))!
  expect(stringifyMeshJson(viaDelta(1))).toBe(stringifyMeshJson(result(0)))
  expect(stringifyMeshJson(viaDelta(3))).toBe(stringifyMeshJson(result(2)))
  // The unmoved sketch travels as its id and is the base's own object.
  expect(viaDelta(3).sketches[0]).toBe(document.sketches[0])
  expect(messages[4]).toMatchObject({ ok: false, error: { code: 'CAD_RESIDENT_MISSING' } })
})

it('history commits of worker results reuse unchanged objects and keep the same snapshots', async () => {
  const document = scene()
  const incremental = new DirectHistory(document),
    reference = new DirectHistory(document)
  const published = incremental.syncedDocument
  const moved = { ...published, bodies: published.bodies.map(body => (body.id === 'linked-1' ? { ...body, name: 'renamed' } : body)) }
  expect(await incremental.commitAsync(async () => moved)).toBe(true)
  reference.commit(structuredClone(moved))
  expect(incremental.snapshotKey).toBe(reference.snapshotKey)
  expect(incremental.canonicalText(moved)).toBe(reference.snapshotKey)
  // Readers still get private copies.
  const copy = incremental.document
  copy.bodies[0]!.name = 'mutated'
  expect(incremental.document.bodies[0]!.name).not.toBe('mutated')
  expect(stringifyMeshJson(incremental.undo())).toBe(stringifyMeshJson(reference.undo()))
  expect(stringifyMeshJson(incremental.redo())).toBe(stringifyMeshJson(reference.redo()))
})

it('a synchronous commit still checks a linked cache that differs from the current state', () => {
  const document = scene()
  const history = new DirectHistory(document)
  const edit = history.document
  ;(edit.bodies[1]!.mesh.positions as unknown as number[] | Float64Array)[0] = 999
  // The authoritative parser rebuilds instance geometry from the source.
  history.commit(edit)
  expect(history.document.bodies[1]!.mesh.positions[0]).not.toBe(999)
  const unchanged = history.document
  unchanged.bodies[2]!.name = 'renamed'
  history.commit(unchanged)
  expect(history.document.bodies[2]!.name).toBe('renamed')
  expect(stringifyMeshJson(history.document.bodies[2]!.mesh)).toBe(stringifyMeshJson(document.bodies[2]!.mesh))
})
