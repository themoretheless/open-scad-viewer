import {describe, expect, it} from 'vitest'
import {createBoxMesh, repairMeshObject} from '../src/services/meshEditing'
import {formatMeshRepairNotice} from '../src/services/meshRepairReport'
import {stringifyMeshJson} from '../src/services/meshJson'

const openTetra = () => ({
  positions: Float64Array.from([0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 2]),
  indices: Uint32Array.from([0, 2, 1, 0, 1, 3, 1, 2, 3]),
})

describe('explicit mesh Full repair', () => {
  it('fills a boundary loop without mutating the source and reports the result', () => {
    const source = openTetra()
    const before = stringifyMeshJson(source)
    const result = repairMeshObject(source)
    expect(stringifyMeshJson(source)).toBe(before)
    expect(result.before.isManifold).toBe(false)
    expect(result.after.isManifold).toBe(true)
    expect(result.mesh.indices).toHaveLength(12)
    expect(result.repair?.filledHoles).toBe(1)
    expect(formatMeshRepairNotice(result, 'en')).toContain('manifold topology confirmed')
    expect(formatMeshRepairNotice(result, 'ru')).toContain('зашито отверстий: 1')
  })

  it('returns an unchanged copy and no repair stats for an already manifold mesh', () => {
    const source = createBoxMesh()
    const result = repairMeshObject(source)
    expect(result.repair).toBeNull()
    expect(result.after.isManifold).toBe(true)
    expect(result.mesh.positions).toEqual(source.positions)
    expect(result.mesh.positions).not.toBe(source.positions)
    expect(formatMeshRepairNotice(result, 'en')).toContain('no changes or history entry')
  })
})
