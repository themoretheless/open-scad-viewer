import { describe, expect, it } from 'vitest'
import { tessellateNurbsSurface, thickenNurbsMesh, exportNurbsStl, inspectNurbsMesh, type NurbsMesh, type NurbsTrim } from '../src/services/geometry/tessellation'
import type { NurbsSurface } from '../src/services/nurbsSurface'
import { revolveNurbsCurve } from '../src/services/nurbsConstructors'

const plane: NurbsSurface = {
  degreeU: 1, degreeV: 1, knotsU: [0, 0, 1, 1], knotsV: [0, 0, 1, 1],
  controlPoints: [[[0, 0, 0], [0, 10, 0]], [[10, 0, 0], [10, 10, 0]]], weights: [[1, 1], [1, 1]],
}
const square = [[0, 0], [1, 0], [1, 1], [0, 1]]
function uvArea(mesh: NurbsMesh) {
  let result = 0
  for (let i = 0; i < mesh.indices.length; i += 3) {
    const [a, b, c] = mesh.indices.slice(i, i + 3).map(index => mesh.uv!.slice(2 * index, 2 * index + 2))
    const triangle = ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) / 2
    expect(triangle).toBeGreaterThan(0)
    result += triangle
  }
  return result
}
function checkSolid(mesh: NurbsMesh, volume: number) {
  expect(mesh.report.closed).toBe(true)
  expect(mesh.report.boundaryEdges).toBe(0)
  expect(mesh.report.nonManifoldEdges).toBe(0)
  expect(mesh.report.orientationConflicts).toBe(0)
  expect(mesh.report.degenerateTriangles).toBe(0)
  expect(mesh.report.signedVolumeMm3).toBeCloseTo(volume, 7)
  expect(mesh.report.selfIntersectionStatus).toBe('not_checked')
  expect(mesh.report.errorBoundCertified).toBe(false)
}

describe('own NURBS parameter-domain tessellation', () => {
  it('samples a plane with conforming oriented topology and zero measured deviation', () => {
    const mesh = tessellateNurbsSurface(plane, { segmentsU: 4, segmentsV: 3 })
    expect(mesh.report.closed).toBe(false)
    expect(mesh.report.boundaryEdges).toBe(14)
    expect(mesh.report.nonManifoldEdges).toBe(0)
    expect(mesh.report.orientationConflicts).toBe(0)
    expect(mesh.report.degenerateTriangles).toBe(0)
    expect(mesh.report.sampledDeviationMm).toBeLessThan(1e-12)
    expect(mesh.report.uvArea).toBe(1)
    expect(uvArea(mesh)).toBeCloseTo(1, 12)
  })

  it('preserves a concave outer trim and multiple holes without T junctions', () => {
    const trim: NurbsTrim = {
      outer: [[0, 0], [1, 0], [1, 1], [0.6, 1], [0.6, 0.4], [0.4, 0.4], [0.4, 1], [0, 1]],
      holes: [
        [[0.1, 0.1], [0.3, 0.1], [0.3, 0.3], [0.1, 0.3]],
        [[0.7, 0.5], [0.9, 0.5], [0.9, 0.7], [0.7, 0.7]],
      ],
    }
    for (const winding of [trim, { outer: [...trim.outer].reverse(), holes: trim.holes!.map(loop => [...loop].reverse()) }]) {
      const mesh = tessellateNurbsSurface(plane, { segmentsU: 3, segmentsV: 4, trim: winding })
      expect(uvArea(mesh)).toBeCloseTo(0.8, 12)
      expect(mesh.report.uvArea).toBeCloseTo(0.8, 12)
      expect(mesh.report.orientationConflicts).toBe(0)
      checkSolid(thickenNurbsMesh(mesh, [0, 0, 2]), 160)
    }
  })

  it('retains a hole much narrower than a sampling cell', () => {
    const mesh = tessellateNurbsSurface(plane, { segmentsU: 1, segmentsV: 1, trim: { outer: square, holes: [[[0.499, 0.1], [0.501, 0.1], [0.501, 0.9], [0.499, 0.9]]] } })
    expect(uvArea(mesh)).toBeCloseTo(0.9984, 12)
    checkSolid(thickenNurbsMesh(mesh, [0, 0, 2]), 199.68)
  })

  it('clips diagonal outer boundaries and holes exactly in UV', () => {
    const trim = { outer: [[0.1, 0.1], [0.9, 0.2], [0.8, 0.9], [0.2, 0.8]], holes: [[[0.3, 0.3], [0.6, 0.35], [0.5, 0.65]]] }
    const polygonArea = (points: number[][]) => Math.abs(points.reduce((sum, p, i) => { const q = points[(i + 1) % points.length]; return sum + p[0] * q[1] - q[0] * p[1] }, 0) / 2)
    const expected = polygonArea(trim.outer) - polygonArea(trim.holes[0])
    const mesh = tessellateNurbsSurface(plane, { segmentsU: 5, segmentsV: 7, trim })
    expect(uvArea(mesh)).toBeCloseTo(expected, 11)
    checkSolid(thickenNurbsMesh(mesh, [0, 0, -3]), expected * 300)
  })

  it('measures spline tessellation deviation and reduces it with finer sampling', () => {
    const patch: NurbsSurface = {
      degreeU: 2, degreeV: 2, knotsU: [0, 0, 0, 1, 1, 1], knotsV: [0, 0, 0, 1, 1, 1],
      controlPoints: [[[0, 0, 0], [0, 5, 0], [0, 10, 0]], [[5, 0, 0], [5, 5, 8], [5, 10, 0]], [[10, 0, 0], [10, 5, 0], [10, 10, 0]]],
      weights: [[1, 1, 1], [1, 2, 1], [1, 1, 1]],
    }
    const coarse = tessellateNurbsSurface(patch, { segmentsU: 2, segmentsV: 2 })
    const fine = tessellateNurbsSurface(patch, { segmentsU: 12, segmentsV: 12 })
    expect(coarse.report.sampledDeviationMm).toBeGreaterThan(0.01)
    expect(fine.report.sampledDeviationMm).toBeLessThan(coarse.report.sampledDeviationMm! / 5)
    checkSolid(thickenNurbsMesh(fine, [0, 0, 2]), 200)
  })

  it.each([
    ['self intersecting outer', { outer: [[0, 0], [1, 1], [0, 1], [1, 0]] }],
    ['repeated closing point', { outer: [...square, square[0]] }],
    ['outside hole', { outer: [[0.2, 0.2], [0.8, 0.2], [0.8, 0.8], [0.2, 0.8]], holes: [[[0.01, 0.01], [0.1, 0.01], [0.1, 0.1]]] }],
    ['touching hole', { outer: square, holes: [[[0, 0.2], [0.3, 0.2], [0.3, 0.4]]] }],
    ['nested holes', { outer: square, holes: [[[0.1, 0.1], [0.8, 0.1], [0.8, 0.8], [0.1, 0.8]], [[0.2, 0.2], [0.3, 0.2], [0.3, 0.3]]] }],
    ['out of domain', { outer: [[-1, 0], [1, 0], [1, 1], [-1, 1]] }],
  ])('rejects %s rather than creating an incorrectly trimmed mesh', (_label, trim) => {
    expect(() => tessellateNurbsSurface(plane, { segmentsU: 2, segmentsV: 2, trim })).toThrow()
  })

  it('enforces work bounds and rejects a collapsed surface before producing STL', () => {
    expect(() => tessellateNurbsSurface(plane, { segmentsU: 0, segmentsV: 2 })).toThrow('Segment counts')
    expect(() => tessellateNurbsSurface(plane, { segmentsU: 10, segmentsV: 10, maxTriangles: 50 })).toThrow('triangle budget')
    const mesh = tessellateNurbsSurface(plane, { segmentsU: 2, segmentsV: 2 })
    expect(() => thickenNurbsMesh(mesh, [0, 0, 0])).toThrow('nonzero')
    expect(() => thickenNurbsMesh(mesh, [1, 0, 0])).toThrow('degenerate')
    expect(() => exportNurbsStl(mesh)).toThrow('closed')
  })

  it('writes sampled STL facets with normals only for a closed mesh', () => {
    const mesh = thickenNurbsMesh(tessellateNurbsSurface(plane, { segmentsU: 2, segmentsV: 2 }), [0, 0, 2])
    const stl = exportNurbsStl(mesh)
    expect(stl.startsWith('solid modelgraph_nurbs_sampled\n')).toBe(true)
    expect(stl.match(/facet normal/g)).toHaveLength(mesh.report.triangleCount)
    expect(stl.match(/vertex /g)).toHaveLength(mesh.report.triangleCount * 3)
    expect(stl).not.toMatch(/NaN|Infinity/)
    expect(stl.endsWith('endsolid modelgraph_nurbs_sampled\n')).toBe(true)
    checkSolid(mesh, 200)
  })

  it('measures solid volume stably when the model is far from the origin', () => {
    const shifted = { ...plane, controlPoints: plane.controlPoints.map(row => row.map(point => point.map(coordinate => coordinate + 1_000_000))) }
    const mesh = tessellateNurbsSurface(shifted, { segmentsU: 3, segmentsV: 3 })
    checkSolid(thickenNurbsMesh(mesh, [0, 0, 2]), 200)
  })

  it('supports a parameter domain translated far from zero', () => {
    const shifted = { ...plane, knotsU: [1e8, 1e8, 1e8 + 1, 1e8 + 1], knotsV: [1e8, 1e8, 1e8 + 1, 1e8 + 1] }
    const mesh = tessellateNurbsSurface(shifted, { segmentsU: 2, segmentsV: 2 })
    expect(mesh.report.uvArea).toBe(1)
    checkSolid(thickenNurbsMesh(mesh, [0, 0, 2]), 200)
  })

  it('normalizes anisotropic UV scales while preserving trim coordinates and reported UV area', () => {
    const stretched = { ...plane, knotsU: [0, 0, 1e-12, 1e-12], knotsV: [10, 10, 1010, 1010] }
    const trim = { outer: [[0, 10], [1e-12, 10], [1e-12, 1010], [0, 1010]], holes: [[[0.2e-12, 210], [0.4e-12, 210], [0.4e-12, 410], [0.2e-12, 410]]] }
    const mesh = tessellateNurbsSurface(stretched, { segmentsU: 3, segmentsV: 4, trim })
    expect(mesh.report.uvArea! / 1e-9).toBeCloseTo(0.96, 12)
    expect(Math.max(...mesh.uv!.filter((_, index) => index % 2 === 0))).toBe(1e-12)
    expect(Math.min(...mesh.uv!.filter((_, index) => index % 2 === 1))).toBe(10)
    expect(Math.max(...mesh.uv!.filter((_, index) => index % 2 === 1))).toBe(1010)
    checkSolid(thickenNurbsMesh(mesh, [0, 0, 2]), 192)
  })

  it('welds only the parameter seam of a complete cylindrical revolution', () => {
    const cylinder = revolveNurbsCurve({ degree: 1, knots: [0, 0, 1, 1], controlPoints: [[5, 0, 0], [5, 0, 10]], weights: [1, 1] }, [0, 0, 0], [0, 0, 1], 360)
    const mesh = tessellateNurbsSurface(cylinder, { segmentsU: 4, segmentsV: 16 })
    expect(mesh.report.parameterSeamsWelded).toEqual({ u: false, v: true })
    expect(mesh.report.boundaryEdges).toBe(32)
    expect(mesh.report.nonManifoldEdges).toBe(0)
    expect(mesh.report.orientationConflicts).toBe(0)
    expect(mesh.report.closed).toBe(false)
  })

  it('welds the two proven parameter seams of a torus into a closed oriented mesh', () => {
    const circle = { degree: 2, knots: [0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 4],
      controlPoints: [[12, 0, 0], [12, 0, 2], [10, 0, 2], [8, 0, 2], [8, 0, 0], [8, 0, -2], [10, 0, -2], [12, 0, -2], [12, 0, 0]],
      weights: [1, Math.SQRT1_2, 1, Math.SQRT1_2, 1, Math.SQRT1_2, 1, Math.SQRT1_2, 1] }
    const torus = revolveNurbsCurve(circle, [0, 0, 0], [0, 0, 1], 360)
    const mesh = tessellateNurbsSurface(torus, { segmentsU: 24, segmentsV: 24 })
    expect(mesh.report.parameterSeamsWelded).toEqual({ u: true, v: true })
    expect(mesh.report.closed).toBe(true)
    expect(mesh.report.boundaryEdges).toBe(0)
    expect(mesh.report.degenerateTriangles).toBe(0)
    expect(mesh.report.signedVolumeMm3 / (2 * Math.PI ** 2 * 10 * 2 ** 2)).toBeGreaterThan(0.97)
    expect(mesh.report.signedVolumeMm3 / (2 * Math.PI ** 2 * 10 * 2 ** 2)).toBeLessThan(1.01)
    expect(exportNurbsStl(mesh)).toContain('facet normal')
  })

  it('collapses spline-proven sphere poles and keeps the remaining topology closed', () => {
    const meridian = { degree: 2, knots: [0, 0, 0, 1, 1, 2, 2, 2], controlPoints: [[0, 0, 10], [10, 0, 10], [10, 0, 0], [10, 0, -10], [0, 0, -10]], weights: [1, Math.SQRT1_2, 1, Math.SQRT1_2, 1] }
    const sphere = revolveNurbsCurve(meridian, [0, 0, 0], [0, 0, 1], 360)
    const mesh = tessellateNurbsSurface(sphere, { segmentsU: 16, segmentsV: 32 })
    expect(mesh.report.parameterSeamsWelded).toEqual({ u: false, v: true })
    expect(mesh.report.collapsedBoundaryCount).toBe(2)
    expect(mesh.report.closed).toBe(true)
    expect(mesh.report.boundaryEdges).toBe(0)
    expect(mesh.report.degenerateTriangles).toBe(0)
    expect(mesh.report.signedVolumeMm3 / (4 * Math.PI * 1000 / 3)).toBeGreaterThan(0.98)
    expect(mesh.report.signedVolumeMm3 / (4 * Math.PI * 1000 / 3)).toBeLessThan(1.01)
  })

  it('detects an orientation error and a nonmanifold edge independently', () => {
    const positions = [0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1]
    expect(inspectNurbsMesh(positions, [0, 1, 2, 0, 1, 3]).orientationConflicts).toBe(1)
    expect(inspectNurbsMesh(positions, [0, 1, 2, 0, 1, 3, 1, 0, 2]).nonManifoldEdges).toBe(1)
  })
})
