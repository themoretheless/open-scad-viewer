import {describe, expect, it} from 'vitest'
import {cachedMeshManifoldReport, meshManifoldDefectLabels} from '../src/services/meshManifoldDefects'
import type {KernelManifoldReport} from '../src/services/geometry/manifoldAnalysis'

const openTetra = {
  positions: Float64Array.from([0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 2]),
  indices: Uint32Array.from([0, 2, 1, 0, 1, 3, 1, 2, 3]),
}

describe('mesh manifold tree diagnostics', () => {
  it('uses the kernel report and caches it by mesh identity', () => {
    const report = cachedMeshManifoldReport(openTetra)
    expect(cachedMeshManifoldReport(openTetra)).toBe(report)
    expect(report.isManifold).toBe(false)
    expect(meshManifoldDefectLabels(report, 'en')).toEqual(['boundary edges: 3'])
    expect(meshManifoldDefectLabels(report, 'ru')).toEqual(['граничные рёбра: 3'])
  })

  it('lists every nonempty defect category in stable order', () => {
    const report: KernelManifoldReport = {
      vertexCount: 8,
      triangleCount: 4,
      componentCount: 1,
      isManifold: false,
      isManifoldWithBoundary: false,
      boundaryEdges: Uint32Array.from([0, 1]),
      nonManifoldEdges: Uint32Array.from([1, 2, 2, 3]),
      orientationEdges: Uint32Array.from([3, 4]),
      degenerateTriangles: Uint32Array.from([0, 2]),
      nonManifoldVertices: Uint32Array.from([5]),
      isolatedVertices: Uint32Array.from([6, 7]),
    }
    expect(meshManifoldDefectLabels(report, 'en')).toEqual([
      'boundary edges: 1',
      'non-manifold edges: 2',
      'non-manifold vertices: 1',
      'orientation conflicts: 1',
      'degenerate triangles: 2',
      'isolated vertices: 2',
    ])
  })
})
