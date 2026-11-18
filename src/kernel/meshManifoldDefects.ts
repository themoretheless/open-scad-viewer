import { checkManifoldInKernel, type KernelManifoldReport } from './geometry/manifoldAnalysis'
import type { PolygonMesh } from './geometry/polygon'

const reports = new WeakMap<PolygonMesh, KernelManifoldReport>()

/** Reuse the immutable mesh's kernel topology report while camera/UI state changes. */
export function cachedMeshManifoldReport(mesh: PolygonMesh): KernelManifoldReport {
  const cached = reports.get(mesh)
  if (cached) return cached
  const report = checkManifoldInKernel(mesh.positions, mesh.indices)
  reports.set(mesh, report)
  return report
}

export interface MeshManifoldObject {
  readonly id: string
  readonly mesh: PolygonMesh
}

export interface MeshManifoldEntry {
  readonly id: string
  readonly report: KernelManifoldReport
}

/** Worker-side batch inspection. Every topology predicate remains in the Rust kernel. */
export function inspectMeshManifoldObjects(
  objects: readonly MeshManifoldObject[],
  onProgress?: (progress: { phase: 'check'; completed: number; total: number }) => void,
): MeshManifoldEntry[] {
  return objects.map((object, completed) => {
    onProgress?.({ phase: 'check', completed, total: objects.length })
    return { id: object.id, report: checkManifoldInKernel(object.mesh.positions, object.mesh.indices) }
  })
}

/** Only nonempty defect categories, in a stable order for tree display. */
export function meshManifoldDefectLabels(report: KernelManifoldReport, locale: 'ru' | 'en'): string[] {
  const ru = locale === 'ru'
  const labels: Array<[number, string, string]> = [
    [report.boundaryEdges.length / 2, 'граничные рёбра', 'boundary edges'],
    [report.nonManifoldEdges.length / 2, 'немногообразные рёбра', 'non-manifold edges'],
    [report.nonManifoldVertices.length, 'немногообразные вершины', 'non-manifold vertices'],
    [report.orientationEdges.length / 2, 'конфликты ориентации', 'orientation conflicts'],
    [report.degenerateTriangles.length, 'вырожденные треугольники', 'degenerate triangles'],
    [report.isolatedVertices.length, 'изолированные вершины', 'isolated vertices'],
  ]
  return labels.filter(([count]) => count > 0).map(([count, russian, english]) => `${ru ? russian : english}: ${count}`)
}
