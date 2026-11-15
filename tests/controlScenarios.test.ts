import { IDBFactory } from 'fake-indexeddb'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  DirectHistory,
  emptyDirectDocument,
  parseDirectDocument,
  serializeDirectDocument,
  type DirectBody,
  type DirectDocument,
  type DirectSketch,
} from '../src/app/directModeling'
import { applyDirectExtrusionProfile } from '../src/app/directExtrusion'
import { selectedBrepSupport, solidTopology } from '../src/app/directSolidTools'
import { addSolidPrimitive } from '../src/kernel/solidPrimitive'
import { applySolidSceneEdit, type SolidSceneEditOptions } from '../src/kernel/solidSceneEdit'
import { applySolidBodyEdit, type SolidBodyEditOptions } from '../src/kernel/solidBodyEdit'
import { applySolidBoolean } from '../src/kernel/solidBoolean'
import { exportSolidStepCurrent, prepareSolidStepImport } from '../src/kernel/solidStepExchange'
import { measureSolidMassProperties } from '../src/kernel/solidMeasurements'
import { createSolidDraftStorageIo, recoverSolidDraft, saveSolidDraft } from '../src/kernel/solidDraftSaveProtocol'
import { warmGeometryKernel } from '../src/kernel/geometry/kernel'
import { controlFailureMessage } from './controlModelSupport'

/**
 * Roadmap C3 end-to-end scenarios, driven through the document functions the
 * direct modeler commits (worker job handlers, DirectHistory, draft save
 * protocol, STEP exchange) rather than through mocks.
 */

const scene = (options: Partial<SolidSceneEditOptions>): SolidSceneEditOptions => ({
  operation: 'transform',
  id: '',
  ids: [],
  createdId: '',
  x: 0,
  y: 0,
  z: 0,
  axis: 'z',
  angle: 0,
  scale: 1,
  ...options,
})
const bodyEdit = (options: Partial<SolidBodyEditOptions>): SolidBodyEditOptions => ({
  operation: 'push',
  id: '',
  face: 0,
  edges: [],
  openings: [],
  segments: 8,
  distance: 0,
  radius: 1,
  endRadius: 1,
  filletMode: 'constant',
  axis: 'z',
  ...options,
})
const box = (id: string, size: number) => ({
  kind: 'box' as const,
  id,
  name: id,
  size,
  topRadius: 1,
  innerRadius: 0.5,
  round: 'exact' as const,
  segments: 1,
})
const top = (z: number) => ({ origin: [0, 0, z], u: [1, 0, 0], v: [0, 1, 0] }) as NonNullable<DirectSketch['plane']>
const rectangle = (
  id: string,
  x: number,
  y: number,
  w: number,
  h: number,
  plane?: DirectSketch['plane'],
): DirectSketch => ({
  id,
  name: id,
  closed: true,
  points: [
    [x, y],
    [x + w, y],
    [x + w, y + h],
    [x, y + h],
  ],
  ...(plane ? { plane } : {}),
})
const body = (d: DirectDocument, id: string) => d.bodies.find(b => b.id === id)!
const faceWhere = (b: DirectBody, axis: number, sign: 1 | -1, at: number) => {
  const index = solidTopology(b.mesh).faces.findIndex(
    f => f.normal[axis] * sign > 0.99 && Math.abs(f.center[axis] - at) < 1e-6,
  )
  if (index < 0) throw Error(`No face on axis ${axis} at ${at}`)
  return index
}
const verticalEdges = (b: DirectBody) =>
  solidTopology(b.mesh).edges.flatMap((edge, i) => {
    const p = b.mesh.positions
    return Math.abs(p[edge.a * 3] - p[edge.b * 3]) < 1e-8 && Math.abs(p[edge.a * 3 + 1] - p[edge.b * 3 + 1]) < 1e-8
      ? [i]
      : []
  })
const withSketch = (d: DirectDocument, sketch: DirectSketch) => ({ ...d, sketches: [...d.sketches, sketch] })
const extrude = (
  d: DirectDocument,
  sketchId: string,
  height: number,
  operation: 'new' | 'union' | 'difference',
  targetId: string,
) => applyDirectExtrusionProfile(d, { sketchIds: [sketchId], height, offset: 0, operation, targetId, id: targetId })

/** Bounds of every CARTESIAN_POINT in a STEP file: a text-level check independent of both kernels. */
function stepPointBounds(text: string) {
  const min = [Infinity, Infinity, Infinity],
    max = [-Infinity, -Infinity, -Infinity]
  for (const match of text.matchAll(/CARTESIAN_POINT\('[^']*',\(([^)]*)\)\)/g)) {
    const values = match[1].split(',').map(Number)
    if (values.length !== 3) continue
    values.forEach((x, k) => {
      min[k] = Math.min(min[k], x)
      max[k] = Math.max(max[k], x)
    })
  }
  return { min, max }
}

beforeAll(async () => {
  await warmGeometryKernel()
})
beforeEach(() => {
  vi.stubGlobal('indexedDB', new IDBFactory())
  vi.stubGlobal('navigator', {
    locks: { request: (_name: string, _options: unknown, action: () => unknown) => Promise.resolve(action()) },
  })
})
afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

describe('C3 scenario: 20 edits, Undo/Redo, reload, STEP export and independent reopen', () => {
  // Each step is one committed edit, as the modeler commits it after a command.
  const steps: Array<[string, (d: DirectDocument) => DirectDocument]> = [
    ['box primitive', d => addSolidPrimitive(d, box('part', 20))],
    ['move the part', d => applySolidSceneEdit(d, scene({ ids: ['part'], x: 10 }))],
    ['pocket sketch on the top face', d => withSketch(d, rectangle('pocket', 5, -5, 10, 10, top(20)))],
    ['cut the pocket', d => extrude(d, 'pocket', -4, 'difference', 'part')],
    ['tab sketch on the base plane', d => withSketch(d, rectangle('tab', 20, -10, 10, 20, top(0)))],
    ['add the tab', d => extrude(d, 'tab', 3, 'union', 'part')],
    [
      'push the tab end face',
      d => applySolidBodyEdit(d, bodyEdit({ id: 'part', face: faceWhere(body(d, 'part'), 0, 1, 30), distance: 2 })),
    ],
    ['rename the part', d => ({ ...d, bodies: d.bodies.map(b => (b.id === 'part' ? { ...b, name: 'Bracket' } : b)) })],
    ['cutter primitive', d => addSolidPrimitive(d, box('cutter', 4))],
    ['place the cutter', d => applySolidSceneEdit(d, scene({ ids: ['cutter'], x: 26, z: -0.5 }))],
    [
      'subtract the cutter',
      d => applySolidBoolean(d, { operation: 'difference', a: ['part'], b: ['cutter'], segments: 4 }).document,
    ],
    [
      'move the pocket sketch',
      d => applySolidSceneEdit(d, scene({ operation: 'sketch-transform', id: 'pocket', ids: ['pocket'], y: 1 })),
    ],
    [
      'duplicate the part',
      d => applySolidSceneEdit(d, scene({ operation: 'duplicate', ids: ['part'], copyIds: ['copy'], x: 100 })),
    ],
    ['delete the duplicate', d => ({ ...d, bodies: d.bodies.filter(b => b.id !== 'copy') })],
    ['create a group', d => applySolidSceneEdit(d, scene({ operation: 'group-create', group: 'Frame' }))],
    [
      'move the part into the group',
      d => applySolidSceneEdit(d, scene({ operation: 'group-move', ids: ['part'], group: 'Frame' })),
    ],
    ['move the part along Y', d => applySolidSceneEdit(d, scene({ ids: ['part'], y: 5 }))],
    ['rib sketch on the top face', d => withSketch(d, rectangle('rib', 1, 0, 2, 10, top(20)))],
    ['add the rib', d => extrude(d, 'rib', 2, 'union', 'part')],
    ['remove construction sketches', d => ({ ...d, sketches: [] })],
  ]
  const volume = 8000 - 400 + 600 + 120 - 48 + 40
  const bounds = { min: [0, -5, 0], max: [32, 15, 22] }

  it('replays, persists and reopens the part with identical dimensions', async () => {
    expect(steps).toHaveLength(20)
    const history = new DirectHistory(emptyDirectDocument()),
      snapshots = [serializeDirectDocument(history.document)]
    for (const [name, step] of steps) {
      const before = serializeDirectDocument(history.document)
      let next: DirectDocument
      try {
        next = step(history.document)
      } catch (error) {
        throw Error(`${name}: ${error instanceof Error ? error.message : error}`, { cause: error })
      }
      expect(serializeDirectDocument(history.document), name + ' must not mutate the committed document').toBe(before)
      history.commit(next)
      snapshots.push(serializeDirectDocument(history.document))
    }
    const final = history.document,
      part = body(final, 'part')
    expect(final.bodies.map(b => b.id)).toEqual(['part'])
    expect(part).toMatchObject({ name: 'Bracket', group: 'Frame' })
    const mass = measureSolidMassProperties(part)
    expect(Math.abs(mass.volumeMm3 - volume)).toBeLessThan(volume * 1e-6)
    for (let k = 0; k < 3; k++) {
      expect(mass.bounds.min[k]).toBeCloseTo(bounds.min[k], 6)
      expect(mass.bounds.max[k]).toBeCloseTo(bounds.max[k], 6)
    }

    // Undo every edit, checking each intermediate state, then redo all of them.
    for (let i = steps.length - 1; i >= 0; i--)
      expect(serializeDirectDocument(history.undo()), 'undo to ' + i).toBe(snapshots[i])
    expect(history.canUndo).toBe(false)
    for (let i = 1; i <= steps.length; i++)
      expect(serializeDirectDocument(history.redo()), 'redo to ' + i).toBe(snapshots[i])
    expect(history.canRedo).toBe(false)

    // Save through the production draft protocol (forced into the snapshot row) and recover it.
    const pointers = new Map<string, string>(),
      io = createSolidDraftStorageIo({
        get: key => pointers.get(key) ?? null,
        set: (key, value) => {
          pointers.set(key, value)
          return true
        },
      })
    const text = serializeDirectDocument(history.document)
    const saved = await saveSolidDraft(io, {
      key: 'c3-draft',
      text,
      expectedRevision: () => null,
      expectedPointer: null,
      inlineLimit: 1024,
      current: () => true,
    })
    expect(saved.pointer).not.toBe(text)
    const recovered = await recoverSolidDraft(io, 'c3-draft')
    expect(recovered?.text).toBe(text)
    const reloaded = parseDirectDocument(recovered!.text)
    expect(serializeDirectDocument(reloaded)).toBe(snapshots.at(-1))
    const reloadedPart = body(reloaded, 'part'),
      reloadedMass = measureSolidMassProperties(reloadedPart)
    expect(reloadedMass.volumeMm3).toBeCloseTo(mass.volumeMm3, 9)
    expect(reloadedMass.surfaceAreaMm2).toBeCloseTo(mass.surfaceAreaMm2, 9)

    // Export STEP, then reopen it through the workbench import route (cad-step parser, new document).
    const step = await exportSolidStepCurrent(reloadedPart)
    expect(step).toMatch(/^ISO-10303-21;/)
    const opened = await prepareSolidStepImport(step, emptyDirectDocument())
    expect(opened.report.retained).toBe(true)
    expect(opened.document.bodies).toHaveLength(1)
    const reopened = opened.document.bodies[0]
    expect(reopened.id).not.toBe(reloadedPart.id)
    const reopenedMass = measureSolidMassProperties(reopened)
    expect(Math.abs(reopenedMass.volumeMm3 - volume)).toBeLessThan(volume * 1e-6)
    expect(Math.abs(reopenedMass.surfaceAreaMm2 - mass.surfaceAreaMm2)).toBeLessThan(mass.surfaceAreaMm2 * 1e-6)
    for (let k = 0; k < 3; k++) {
      expect(reopenedMass.bounds.min[k]).toBeCloseTo(bounds.min[k], 6)
      expect(reopenedMass.bounds.max[k]).toBeCloseTo(bounds.max[k], 6)
      expect(reopenedMass.dimensions[k]).toBeCloseTo(bounds.max[k] - bounds.min[k], 6)
    }
    const textBounds = stepPointBounds(step)
    for (let k = 0; k < 3; k++) {
      expect(textBounds.min[k]).toBeCloseTo(bounds.min[k], 6)
      expect(textBounds.max[k]).toBeCloseTo(bounds.max[k], 6)
    }
  }, 240_000)
})

describe('C3 prototype loop: face, extrude, dimension, fillet, refusal, correction', () => {
  it('keeps the document consistent at every step', () => {
    const history = new DirectHistory(addSolidPrimitive(emptyDirectDocument(), box('proto', 20)))
    const measure = () => measureSolidMassProperties(body(history.document, 'proto'))
    expect(measure().volumeMm3).toBeCloseTo(8000, 6)

    // 1. Select the top face: the display face resolves to one exact B-rep support face.
    const start = body(history.document, 'proto'),
      face = faceWhere(start, 2, 1, 20)
    const support = selectedBrepSupport(start, solidTopology(start.mesh).faces[face].triangles)
    expect(support).toBeGreaterThanOrEqual(0)

    // 2. Extrude the face (preview) and 3. set its dimension to 8 mm before committing.
    const preview = applySolidBodyEdit(history.document, bodyEdit({ id: 'proto', face, distance: 5 }))
    expect(measureSolidMassProperties(body(preview, 'proto')).bounds.max[2]).toBeCloseTo(25, 6)
    expect(measure().bounds.max[2]).toBeCloseTo(20, 6)
    const dimensioned = applySolidBodyEdit(history.document, bodyEdit({ id: 'proto', face, distance: 8 }))
    history.commit(dimensioned)
    expect(measure().volumeMm3).toBeCloseTo(400 * 28, 6)
    expect(measure().bounds.max[2]).toBeCloseTo(28, 6)
    const extruded = serializeDirectDocument(history.document)

    // 4. Fillet the vertical edges; 5. an oversized radius is refused with an explanation.
    const edges = verticalEdges(body(history.document, 'proto'))
    expect(edges).toHaveLength(4)
    let refusal: unknown
    try {
      applySolidBodyEdit(history.document, bodyEdit({ operation: 'edge-fillet', id: 'proto', edges, radius: 15 }))
    } catch (error) {
      refusal = error
    }
    expect(refusal).toBeInstanceOf(Error)
    expect((refusal as { code?: string }).code).toMatch(/^BREP_[A-Z_]*FILLET_REFUSED$/)
    const state = { bodyName: 'Proto', edges }
    const numbers = edges.map(i => i + 1).join(', ')
    expect(controlFailureMessage(refusal, 'ru', state)).toBe(
      `Proto · рёбра ${numbers}: Размер сопряжения не помещается. Уменьшите радиус или размер фаски.`,
    )
    expect(controlFailureMessage(refusal, 'en', state)).toBe(
      `Proto · edges ${numbers}: The feature does not fit. Reduce the radius or chamfer size.`,
    )
    expect(serializeDirectDocument(history.document)).toBe(extruded)
    expect(history.canRedo).toBe(false)

    // 6. The corrected radius succeeds and is one undoable step.
    history.commit(
      applySolidBodyEdit(history.document, bodyEdit({ operation: 'edge-fillet', id: 'proto', edges, radius: 3 })),
    )
    expect(measure().volumeMm3).toBeCloseTo(400 * 28 - (4 - Math.PI) * 9 * 28, 4)
    expect(measure().bounds.max).toEqual([expect.closeTo(10, 6), expect.closeTo(10, 6), expect.closeTo(28, 6)])
    expect(serializeDirectDocument(history.undo())).toBe(extruded)
    expect(measureSolidMassProperties(body(history.redo(), 'proto')).volumeMm3).toBeCloseTo(
      400 * 28 - (4 - Math.PI) * 9 * 28,
      4,
    )
  }, 120_000)
})
