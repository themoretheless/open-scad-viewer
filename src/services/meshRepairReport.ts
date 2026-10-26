import type { MeshRepairResult } from './meshEditing'
import { meshManifoldDefectLabels } from './meshManifoldDefects'

/** Localized outcome for the explicit Full repair history operation. */
export function formatMeshRepairNotice(result: MeshRepairResult, locale: 'ru' | 'en'): string {
  const ru = locale === 'ru'
  if (!result.repair) return ru ? 'Меш уже является manifold; изменения и запись в истории не потребовались.' : 'Mesh is already manifold; no changes or history entry were needed.'
  const changes: string[] = []
  const repair = result.repair
  if (repair.weldedVertices) changes.push(ru ? `сварено вершин: ${repair.weldedVertices}` : `vertices welded: ${repair.weldedVertices}`)
  if (repair.removedDegenerateTriangles) changes.push(ru ? `удалено вырожденных треугольников: ${repair.removedDegenerateTriangles}` : `degenerate triangles removed: ${repair.removedDegenerateTriangles}`)
  if (repair.flippedTriangles) changes.push(ru ? `перевёрнуто треугольников: ${repair.flippedTriangles}` : `triangles flipped: ${repair.flippedTriangles}`)
  if (repair.splitVertices) changes.push(ru ? `разделено вершин: ${repair.splitVertices}` : `vertices split: ${repair.splitVertices}`)
  if (repair.filledHoles) changes.push(ru ? `зашито отверстий: ${repair.filledHoles}` : `holes filled: ${repair.filledHoles}`)
  if (repair.filledTriangles) changes.push(ru ? `добавлено треугольников: ${repair.filledTriangles}` : `triangles added: ${repair.filledTriangles}`)
  const residual = meshManifoldDefectLabels(result.after, locale)
  return [
    result.after.isManifold
      ? ru ? 'Full repair выполнен: топология manifold подтверждена.' : 'Full repair completed: manifold topology confirmed.'
      : ru ? 'Full repair выполнен, но дефекты остались.' : 'Full repair completed, but defects remain.',
    changes.length ? `${ru ? 'Исправления' : 'Changes'}: ${changes.join(', ')}.` : ru ? 'Геометрия не изменилась.' : 'Geometry did not change.',
    residual.length ? `${ru ? 'Остались дефекты' : 'Remaining defects'}: ${residual.join(', ')}.` : '',
  ].filter(Boolean).join(' ')
}
