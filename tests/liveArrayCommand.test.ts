import { describe, expect, it } from 'vitest'
import type { DirectDocument } from '../src/app/directModeling'
import { applySolidSceneEdit, isSolidSceneEdit, type SolidSceneEditOptions } from '../src/kernel/solidSceneEdit'
import { presentKernelError } from '../src/i18n/kernelErrors'

const seed: DirectDocument = {
  version: 1,
  sketches: [],
  bodies: [{ id: 'seed', name: 'Seed', mesh: { positions: [0, 0, 0, 1, 0, 0, 0, 1, 0], indices: [0, 1, 2] } }],
}
const edit = (document: DirectDocument, id: string, livePattern: SolidSceneEditOptions['livePattern']) =>
  applySolidSceneEdit(document, {
    operation: 'live-pattern',
    id,
    ids: [id],
    createdId: '',
    x: 0,
    y: 0,
    z: 0,
    axis: 'z',
    angle: 0,
    scale: 1,
    livePattern,
  })
const offsets = (document: DirectDocument) =>
  document.bodies.filter(b => b.instance?.pattern).map(b => b.instance!.matrix.slice(0, 3).map(row => row[3]))

describe('C3 live array', () => {
  it('is a scene edit that stores grid parameters on the source and expands linked members', () => {
    expect(isSolidSceneEdit('live-pattern')).toBe(true)
    const grid = edit(seed, 'seed', { kind: 'grid', rows: 2, columns: 3, spacing: [15, 25] })
    expect(grid.bodies.find(b => b.id === 'seed')!.livePattern).toEqual({
      kind: 'grid',
      rows: 2,
      columns: 3,
      spacing: [15, 25],
    })
    expect(offsets(grid)).toEqual([
      [15, 0, 0],
      [30, 0, 0],
      [0, 25, 0],
      [15, 25, 0],
      [30, 25, 0],
    ])
    expect(seed.bodies).toHaveLength(1)
  })

  it('edits the parameters later and removes the array with null', () => {
    const grid = edit(seed, 'seed', { kind: 'grid', rows: 2, columns: 3, spacing: [15, 25] })
    const radial = edit(grid, 'seed', { kind: 'radial', count: 4, axis: 'z', center: [10, 0, 0] })
    expect(radial.bodies.filter(b => b.instance?.pattern)).toHaveLength(3)
    expect(radial.bodies.find(b => b.id === 'seed')!.livePattern?.kind).toBe('radial')
    const removed = edit(radial, 'seed', null)
    expect(removed.bodies).toHaveLength(1)
    expect(removed.bodies[0]!.livePattern).toBeUndefined()
  })

  it('refuses an invalid count with the patternCount field and the source as target', () => {
    let caught: unknown
    try {
      edit(seed, 'seed', { kind: 'grid', rows: 1, columns: 1, spacing: [1, 1] })
    } catch (e) {
      caught = e
    }
    expect(caught).toMatchObject({ code: 'DIRECT_PATTERN_GRID', field: 'patternCount' })
    const presented = presentKernelError(caught, 'ru')
    expect(presented.field).toBe('patternCount')
    expect(presented.targetIds).toEqual(['seed'])
    expect(() => edit(seed, 'seed', { kind: 'radial', count: 999, axis: 'z', center: [0, 0, 0] })).toThrow(
      expect.objectContaining({ code: 'DIRECT_PATTERN_RADIAL' }),
    )
  })
})
