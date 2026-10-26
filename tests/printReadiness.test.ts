import { describe, expect, it } from 'vitest'
import type { PolygonMesh } from '../src/services/geometry/polygon'
import {
  assertPrintReady,
  formatPrintReadiness,
  PrintReadinessError,
  type PrintReadinessErrorCode,
} from '../src/services/printReadiness'
import type { SolidIntersectionOptions } from '../src/services/solidDiagnostics'

const positions = Float64Array.from([
  0, 0, 0, 10, 0, 0, 10, 10, 0, 0, 10, 0,
  0, 0, 2, 10, 0, 2, 10, 10, 2, 0, 10, 2,
])
const indices = Uint32Array.from([
  0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7,
  0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5,
  2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7,
])
const box = (): PolygonMesh => ({ positions: positions.slice(), indices: indices.slice() })

function failure(mesh: PolygonMesh, code: PrintReadinessErrorCode, options?: SolidIntersectionOptions): PrintReadinessError {
  try {
    assertPrintReady(mesh, options)
  } catch (error) {
    expect(error).toBeInstanceOf(PrintReadinessError)
    expect((error as PrintReadinessError).code).toBe(code)
    return error as PrintReadinessError
  }
  throw new Error(`Expected ${code}`)
}

describe('3D print readiness', () => {
  it('accepts a closed oriented box without demanding positive signed volume', () => {
    const report = assertPrintReady(box())
    expect(report.manifold.isManifold).toBe(true)
    expect(report.intersections).toMatchObject({ complete: true, contacts: [] })
    const inward = box()
    for (let i = 0; i < inward.indices.length; i += 3) {
      const second = inward.indices[i + 1]
      inward.indices[i + 1] = inward.indices[i + 2]
      inward.indices[i + 2] = second
    }
    expect(() => assertPrintReady(inward)).not.toThrow()
  })

  it('blocks an open shell with its boundary count', () => {
    const open = {
      positions: Float64Array.from([0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 2]),
      indices: Uint32Array.from([0, 2, 1, 0, 1, 3, 1, 2, 3]),
    }
    const error = failure(open, 'PRINT_NOT_WATERTIGHT')
    expect(error.details).toMatchObject({ boundaryEdges: 3 })
    expect(error.message).toContain('boundary edges: 3')
    expect(formatPrintReadiness(error.code, error.details, 'ru')).toContain('граничные рёбра: 3')
  })

  it('distinguishes inconsistent winding from other manifold defects', () => {
    const mesh = box()
    const second = mesh.indices[1]
    mesh.indices[1] = mesh.indices[2]
    mesh.indices[2] = second
    const error = failure(mesh, 'PRINT_ORIENTATION')
    expect(error.details.orientationConflicts).toBeGreaterThan(0)
    expect(error.message).toContain('inconsistent triangle orientation')
  })

  it('blocks coincident closed components and identifies their source triangles', () => {
    const first = box()
    const mesh = {
      positions: Float64Array.from([...first.positions, ...first.positions]),
      indices: Uint32Array.from([...first.indices, ...first.indices.map(index => index + first.positions.length / 3)]),
    }
    const error = failure(mesh, 'PRINT_SELF_INTERSECTIONS')
    expect(error.details.contacts).toBeGreaterThan(0)
    expect(error.details.triangleIds?.length).toBeGreaterThan(0)
    expect(error.message).toContain('self-intersections found')
  })

  it('never treats an exhausted intersection scan as proof of a clean mesh', () => {
    const error = failure(box(), 'PRINT_CHECK_INCOMPLETE', { maxWork: 1 })
    expect(error.details).toMatchObject({ stopReason: 'work-limit', work: 1, maxWork: 1 })
    expect(error.message).toContain('Absence of intersections is unproven')
  })
})
