import type { ImportedMesh } from './meshImport'

/** Compact, localized import outcome shown by both mesh-editing workspaces. */
export function formatMeshImportNotice(mesh: ImportedMesh, locale: 'ru' | 'en'): string {
  const ru = locale === 'ru'
  const { after, before, mode, repair } = mesh.manifold
  const status = after.isManifold
    ? ru ? 'топология manifold подтверждена' : 'manifold topology confirmed'
    : after.isManifoldWithBoundary
      ? ru ? 'многообразие с краем' : 'manifold with boundary'
      : ru ? 'топология не является manifold' : 'topology is not manifold'

  const changes: string[] = []
  const welded = Math.max(0, mesh.sourceVertexCount - before.vertexCount)
  if (welded) changes.push(ru ? `сварено вершин: ${welded}` : `vertices welded: ${welded}`)
  const removed = mesh.degenerateTriangles + (repair?.removedDegenerateTriangles ?? 0)
  if (removed) changes.push(ru ? `удалено вырожденных треугольников: ${removed}` : `degenerate triangles removed: ${removed}`)
  if (repair?.flippedTriangles) changes.push(ru ? `перевёрнуто треугольников: ${repair.flippedTriangles}` : `triangles flipped: ${repair.flippedTriangles}`)
  if (repair?.splitVertices) changes.push(ru ? `разделено вершин: ${repair.splitVertices}` : `vertices split: ${repair.splitVertices}`)
  if (repair?.filledHoles) changes.push(ru ? `зашито отверстий: ${repair.filledHoles}` : `holes filled: ${repair.filledHoles}`)
  if (repair?.filledTriangles) changes.push(ru ? `добавлено треугольников: ${repair.filledTriangles}` : `triangles added: ${repair.filledTriangles}`)

  const residual: string[] = []
  if (!after.isManifold) {
    if (after.boundaryEdges.length) residual.push(ru ? `граничные рёбра: ${after.boundaryEdges.length / 2}` : `boundary edges: ${after.boundaryEdges.length / 2}`)
    if (after.nonManifoldEdges.length) residual.push(ru ? `немногообразные рёбра: ${after.nonManifoldEdges.length / 2}` : `non-manifold edges: ${after.nonManifoldEdges.length / 2}`)
    if (after.nonManifoldVertices.length) residual.push(ru ? `немногообразные вершины: ${after.nonManifoldVertices.length}` : `non-manifold vertices: ${after.nonManifoldVertices.length}`)
    if (after.orientationEdges.length) residual.push(ru ? `конфликты ориентации: ${after.orientationEdges.length / 2}` : `orientation conflicts: ${after.orientationEdges.length / 2}`)
    if (after.degenerateTriangles.length) residual.push(ru ? `вырожденные треугольники: ${after.degenerateTriangles.length}` : `degenerate triangles: ${after.degenerateTriangles.length}`)
    if (after.isolatedVertices.length) residual.push(ru ? `изолированные вершины: ${after.isolatedVertices.length}` : `isolated vertices: ${after.isolatedVertices.length}`)
  }

  const modeLabel = mode === 'none' ? ru ? 'без repair' : 'no repair' : mode === 'full' ? 'Full' : 'Conservative'
  const parts = [
    `${mesh.format.toUpperCase()}: ${status}.`,
    ru ? `Режим проверки: ${modeLabel}.` : `Inspection mode: ${modeLabel}.`,
    changes.length
      ? `${ru ? 'Исправления' : 'Changes'}: ${changes.join(', ')}.`
      : mode === 'none' ? '' : ru ? 'Дополнительные исправления не потребовались.' : 'No additional repairs were needed.',
    residual.length ? `${ru ? 'Остались дефекты' : 'Remaining defects'}: ${residual.join(', ')}.` : '',
  ]
  return parts.filter(Boolean).join(' ')
}
