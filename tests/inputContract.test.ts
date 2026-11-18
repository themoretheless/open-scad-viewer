import { describe, expect, it } from 'vitest'
import { PickCycle, pickDirectMeshHits } from '../src/app/directPicking'
import { evaluateSafeMathExpression } from '../src/kernel/cadQuantity'
import { documentUnitScale, evaluateUnitExpression } from '../src/app/documentUnits'
import {
  activeFrame,
  frameFromFace,
  frameToWorld,
  moveAlongFrameAxis,
  nextCoordinateSystem,
  rotateAboutAxis,
  worldToFrame,
  WORLD_FRAME,
} from '../src/app/coordinateSystems'
import {
  cplaneFrame,
  cplaneFromFrame,
  pivotFrame,
  pivotKey,
  savedPivot,
  snapPivot,
  withSavedPivot,
} from '../src/app/directPivots'
import {
  addToIsolation,
  isolatedInView,
  isolateWithin,
  pruneIsolation,
  readIsolation,
  stepOutIsolation,
} from '../src/app/isolationStack'
import {
  emptyDirectDocument,
  parseDirectDocument,
  serializeDirectDocument,
  type DirectDocument,
} from '../src/app/directModeling'
import { appendDirectDocumentEdit, executeDirectTransaction } from '../src/app/directTransactions'
import {
  createDirectClipboardPayload,
  parseDirectClipboardPayload,
  pasteDirectClipboardPayload,
} from '../src/app/directClipboard'
import { addSolidPrimitive } from '../src/kernel/solidPrimitive'
import type { Vec3 } from '../src/kernel/geometry/math3d'

const square = (z: number) => ({ positions: [-1, -1, z, 1, -1, z, 1, 1, z, -1, 1, z], indices: [0, 1, 2, 0, 2, 3] })
const top = { yaw: 0, pitch: Math.PI / 2 }
const close = (a: readonly number[], b: readonly number[]) => a.forEach((v, i) => expect(v).toBeCloseTo(b[i], 9))

function cubeDocument(units?: DirectDocument['units']): DirectDocument {
  const base = { ...emptyDirectDocument(), ...(units ? { units } : {}) }
  return addSolidPrimitive(base, {
    kind: 'box',
    id: 'cube',
    name: 'Cube',
    size: 2,
    topRadius: 1,
    innerRadius: 0.5,
    round: 'exact',
    segments: 1,
  } as never)
}

describe('pick cycling among overlapping objects', () => {
  const hits = pickDirectMeshHits([0.5, 0.2], top, [
    { id: 'low', mesh: square(0) },
    { id: 'high', mesh: square(5) },
    { id: 'mid', mesh: square(2) },
  ])
  it('lists one hit per object front to back', () => {
    expect(hits.map(h => h.id)).toEqual(['high', 'mid', 'low'])
  })
  it('cycles by depth on repeated clicks and Alt+click, and restarts elsewhere', () => {
    const cycle = new PickCycle(4)
    expect(cycle.choose([0, 0], hits)?.id).toBe('high')
    expect(cycle.choose([1, 1], hits)?.id).toBe('mid')
    expect(cycle.choose([1, 1], hits)?.id).toBe('low')
    expect(cycle.choose([1, 1], hits)?.id).toBe('high')
    expect(cycle.choose([50, 50], hits, { cycle: true })?.id).toBe('mid')
    cycle.reset()
    expect(cycle.choose([50, 50], hits)?.id).toBe('high')
    cycle.pin([0, 0], hits, 'mid')
    expect(cycle.choose([0, 0], hits)?.id).toBe('low')
    expect(cycle.choose([0, 0], [])).toBeNull()
  })
})

describe('unit-aware expressions', () => {
  const evaluate = (text: string, options = {}) => evaluateUnitExpression(text, evaluateSafeMathExpression, options)
  it('evaluates mixed units, constants and variables', () => {
    expect(evaluate('10mm+1in')).toEqual({ valid: true, value: 35.4 })
    expect(evaluate('2*pi*r', { variables: { r: 5 } })).toEqual({ valid: true, value: 2 * Math.PI * 5 })
    expect(evaluate('1 m - 5 cm')).toEqual({ valid: true, value: 950 })
    expect(evaluate('2', { defaultUnit: 'in' })).toEqual({ valid: true, value: 50.8 })
    expect(evaluate('90deg + 0.5rad*2', { kind: 'angle' })).toEqual({ valid: true, value: 90 + 180 / Math.PI })
    expect(evaluate('1,5cm')).toEqual({ valid: true, value: 15 })
  })
  it('rejects wrong units, unknown names and range violations', () => {
    expect(evaluate('5 deg')).toEqual({ valid: false, reason: 'unit' })
    expect(evaluate('2*q')).toEqual({ valid: false, reason: 'number' })
    expect(evaluate('1in', { max: 10 })).toEqual({ valid: false, reason: 'range' })
    expect(evaluate('')).toEqual({ valid: false, reason: 'number' })
    expect(evaluate('3', { defaultUnit: 'ft' })).toEqual({ valid: false, reason: 'unit' })
  })
  it('scales between document units', () => {
    expect(documentUnitScale('in', 'mm')).toBe(25.4)
    expect(documentUnitScale('cm', 'm')).toBe(0.01)
  })
})

describe('coordinate systems', () => {
  const cplane = frameFromFace([
    [0, 0, 5],
    [10, 0, 5],
    [10, 0, 15],
    [0, 0, 15],
  ])
  it('derives a CPlane from a face', () => {
    close(cplane.origin, [5, 0, 10])
    close(cplane.z, [0, -1, 0])
    close(cplane.x, [1, 0, 0])
  })
  it('round-trips numeric input per system', () => {
    for (const frame of [WORLD_FRAME, cplane]) {
      const world: Vec3 = [3, -4, 7]
      close(frameToWorld(frame, worldToFrame(frame, world)), world)
    }
    close(worldToFrame(cplane, [6, 0, 10]), [1, 0, 0])
  })
  it('moves and rotates along the active frame', () => {
    close(moveAlongFrameAxis([[0, 0, 0]], WORLD_FRAME, 'z', 2)[0], [0, 0, 2])
    close(moveAlongFrameAxis([[0, 0, 0]], cplane, 'z', 2)[0], [0, -2, 0])
    close(rotateAboutAxis([1, 0, 0], [0, 0, 0], WORLD_FRAME.z, 90), [0, 1, 0])
    close(rotateAboutAxis([6, 0, 10], cplane.origin, cplane.z, 90), [5, 0, 11])
    const local = pivotFrame({ key: 'a', point: [1, 1, 1], snap: 'free', axes: { x: [0, 1, 0], z: [1, 0, 0] } })!
    close(moveAlongFrameAxis([[0, 0, 0]], local, 'y', 1)[0], [0, 0, 1])
  })
  it('falls back to World and cycles systems', () => {
    expect(activeFrame('cplane', {})).toBe(WORLD_FRAME)
    expect(activeFrame('cplane', { cplane })).toBe(cplane)
    expect(nextCoordinateSystem('world')).toBe('cplane')
    expect(nextCoordinateSystem('local')).toBe('world')
  })
})

describe('saved pivots', () => {
  it('snaps to a vertex, the center or the origin', () => {
    const context = { vertices: [0, 0, 0, 10, 0, 0, 10, 10, 0], center: [5, 5, 0] as Vec3 }
    expect(snapPivot('vertex', [9, 1, 0], context)).toEqual([10, 0, 0])
    expect(snapPivot('center', [9, 1, 0], context)).toEqual([5, 5, 0])
    expect(snapPivot('origin', [9, 1, 0], context)).toEqual([0, 0, 0])
  })
  it('persists in the document, omits an empty list and parses older documents', () => {
    const plain = cubeDocument()
    expect(serializeDirectDocument(plain)).not.toContain('pivots')
    const withPivot = withSavedPivot(plain, ['cube'], { point: [1, 2, 3], snap: 'vertex' })
    const parsed = parseDirectDocument(serializeDirectDocument(withPivot))
    expect(savedPivot(parsed, ['cube'])?.point).toEqual([1, 2, 3])
    expect('pivots' in withSavedPivot(parsed, ['cube'], null)).toBe(false)
    // Pivots of deleted objects are dropped rather than rejecting the document.
    const orphan = JSON.parse(serializeDirectDocument(withPivot))
    orphan.pivots[0].key = pivotKey(['gone'])
    expect('pivots' in parseDirectDocument(JSON.stringify(orphan))).toBe(false)
    orphan.pivots[0].snap = 'bogus'
    expect(() => parseDirectDocument(JSON.stringify(orphan))).toThrow(/pivot/)
  })
  it('replays pivot and CPlane edits as metadata in a transaction', () => {
    const before = cubeDocument()
    const frame = frameFromFace([
      [0, 0, 0],
      [1, 0, 0],
      [1, 1, 0],
    ])
    const after = {
      ...withSavedPivot(before, ['cube'], { point: [0, 0, 1], snap: 'center' }),
      cplane: cplaneFromFrame(frame),
    }
    const script = appendDirectDocumentEdit({ version: 1, actions: [] }, before, after)
    expect(script.actions[0]).toMatchObject({
      kind: 'documentEdit',
      edits: expect.arrayContaining([expect.objectContaining({ collection: 'metadata', id: 'pivots' })]),
    })
    const replayed = executeDirectTransaction(before, JSON.stringify(script))
    expect(replayed.pivots).toEqual(after.pivots)
    close(cplaneFrame(replayed.cplane)!.z, [0, 0, 1])
    expect(() => executeDirectTransaction(after, JSON.stringify(script))).toThrow(/metadata/)
  })
})

describe('nested isolation', () => {
  it('pushes narrower levels, steps out one level and prunes missing objects', () => {
    let stack = isolateWithin([], ['a', 'b', 'c'])
    stack = isolateWithin(stack, ['b', 'c', 'z'])
    expect(stack).toEqual([
      ['a', 'b', 'c'],
      ['b', 'c'],
    ])
    expect(isolateWithin(stack, ['b', 'c'])).toBe(stack)
    expect(isolatedInView(stack, 'a')).toBe(false)
    expect(stepOutIsolation(stack)).toEqual([['a', 'b', 'c']])
    expect(addToIsolation(stack, ['n'])).toEqual([
      ['a', 'b', 'c', 'n'],
      ['b', 'c', 'n'],
    ])
    expect(pruneIsolation(stack, id => id !== 'b' && id !== 'c')).toEqual([['a']])
  })
  it('migrates the old flat workspace list', () => {
    const exists = (id: string) => id !== 'x'
    expect(readIsolation(['a', 'x', 'b'], exists)).toEqual([['a', 'b']])
    expect(readIsolation([], exists)).toEqual([])
    expect(readIsolation([['a'], ['a', 'x']], exists)).toEqual([['a'], ['a']])
    expect(readIsolation('a', exists)).toEqual([])
  })
})

describe('copy with a base point between documents', () => {
  it('pastes onto a target point with unit conversion and fresh ids', () => {
    const source = cubeDocument('in')
    const text = createDirectClipboardPayload(source, ['cube'], [1, 0, 0])
    const payload = parseDirectClipboardPayload(text)
    expect(payload).toMatchObject({
      format: 'open-scad-viewer/direct-objects',
      version: 1,
      units: 'in',
      basePoint: [1, 0, 0],
    })
    const target = cubeDocument('mm')
    let n = 0
    const { document, ids } = pasteDirectClipboardPayload(target, payload, [125.4, 0, 0], () =>
      n++ ? 'pasted' : 'cube',
    )
    expect(ids).toEqual(['pasted'])
    const xs = (doc: DirectDocument, id: string) =>
      Array.from(doc.bodies.find(b => b.id === id)!.mesh.positions).filter((_, i) => i % 3 === 0)
    // Base point 1 in = 25.4 mm lands on 125.4 mm: the copy moves by exactly 100 mm, unscaled.
    expect(Math.min(...xs(document, 'pasted')) - Math.min(...xs(target, 'cube'))).toBeCloseTo(100, 6)
    expect(Math.max(...xs(document, 'pasted')) - Math.min(...xs(document, 'pasted'))).toBeCloseTo(2, 6)
  })
  it('carries NURBS curves and surfaces with an exact control-point offset', () => {
    const source: DirectDocument = {
      ...cubeDocument('mm'),
      curves: [
        {
          id: 'c',
          name: 'Curve',
          group: 'g',
          curve: {
            degree: 1,
            knots: [0, 0, 1, 1],
            controlPoints: [
              [0, 0, 0],
              [10, 0, 0],
            ],
            weights: [1, 1],
          },
          bridge: { sourceA: 'x', sourceB: 'y', endA: 'start', endB: 'end', tension: 1 },
        },
      ],
      surfaces: [
        {
          id: 's',
          name: 'Patch',
          segmentsU: 4,
          segmentsV: 4,
          surface: {
            degreeU: 1,
            degreeV: 1,
            knotsU: [0, 0, 1, 1],
            knotsV: [0, 0, 1, 1],
            controlPoints: [
              [
                [0, 0, 0],
                [0, 5, 0],
              ],
              [
                [5, 0, 0],
                [5, 5, 0],
              ],
            ],
            weights: [
              [1, 1],
              [1, 1],
            ],
          },
        },
      ],
    }
    const payload = parseDirectClipboardPayload(createDirectClipboardPayload(source, ['c', 's'], [0, 0, 0]))
    expect(payload.bodies).toEqual([])
    expect(payload.curves?.[0]).not.toHaveProperty('group')
    expect(payload.curves?.[0]).not.toHaveProperty('bridge')
    const { document, ids } = pasteDirectClipboardPayload(cubeDocument('cm'), payload, [1, 2, 3], () =>
      crypto.randomUUID(),
    )
    expect(ids).toHaveLength(2)
    expect(document.curves?.[0].curve.controlPoints).toEqual([
      [10, 20, 30],
      [20, 20, 30],
    ])
    expect(document.surfaces?.[0].surface.controlPoints[1][1]).toEqual([15, 25, 30])
    expect(ids).toContain(document.surfaces?.[0].id)
  })
  it('rejects foreign or newer payloads', () => {
    expect(() => parseDirectClipboardPayload('hello')).toThrow(/copied objects/)
    expect(() =>
      parseDirectClipboardPayload(JSON.stringify({ format: 'open-scad-viewer/direct-objects', version: 2 })),
    ).toThrow(/version/)
    expect(() => createDirectClipboardPayload(cubeDocument(), [], [0, 0, 0])).toThrow()
  })
})
