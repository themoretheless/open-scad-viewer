import {inspectPolygonMesh} from './polygonKernel'
import type {PhotoSurface} from './photogrammetryKernel'

/** Exports the original reconstructed coordinates, colors and all observed triangles. */
export function photoPly(surface: PhotoSurface): string {
  const lines = [
    'ply', 'format ascii 1.0', `element vertex ${surface.positions.length}`,
    'property float x', 'property float y', 'property float z',
    'property uchar red', 'property uchar green', 'property uchar blue',
    `element face ${surface.triangles.length}`,
    'property list uchar int vertex_indices', 'end_header',
  ]
  surface.positions.forEach((position, i) => lines.push([...position, ...(surface.colors[i] ?? [180, 180, 180])].join(' ')))
  for (const triangle of surface.triangles) lines.push(`3 ${triangle.join(' ')}`)
  return lines.join('\n') + '\n'
}

/** The solid CAD path cannot accept open or inconsistently oriented scan patches. */
export function photoCanAppend(surface: PhotoSurface): boolean {
  if (!surface.triangles.length) return false
  try {
    const mesh = inspectPolygonMesh({positions: surface.positions.flat(), indices: surface.triangles.flat()})
    return mesh.closed && mesh.signedVolumeMm3 > 0 && !mesh.degenerateTriangles
      && !mesh.nonManifoldEdges && !mesh.orientationConflicts
  } catch { return false }
}

export class PhotoExportError extends Error {
  constructor(readonly code: 'width' | 'solid' | 'extent' | 'budget') {
    super({width: 'Enter a finite positive width', solid: 'Surface is not a valid CAD solid',
      extent: 'Surface has no finite width', budget: 'Surface exceeds document budget'}[code])
  }
}

/** Applies a measured size to the document copy only, using the full result's extent. */
export function photoScadSource(surface: PhotoSurface, widthMm: number, sourceBudget: number): string {
  if (!Number.isFinite(widthMm) || widthMm <= 0) throw new PhotoExportError('width')
  const document = surface.documentSurface ?? surface
  if (!photoCanAppend(document)) throw new PhotoExportError('solid')
  let low = Infinity, high = -Infinity
  for (const point of surface.positions) {
    low = Math.min(low, point[0]!)
    high = Math.max(high, point[0]!)
  }
  const scale = widthMm / (high - low)
  if (!Number.isFinite(scale) || scale <= 0) throw new PhotoExportError('extent')
  const points = document.positions.map(point => point.map(value => Number((value * scale).toPrecision(8))))
  if (points.some(point => point.some(value => !Number.isFinite(value)))) throw new PhotoExportError('extent')
  const source = `// Photo reconstruction: observed surface scaled to a measured full X width.\npolyhedron(points=${JSON.stringify(points)}, faces=${JSON.stringify(document.triangles)}, convexity=10);`
  if (!Number.isFinite(sourceBudget) || source.length > sourceBudget) throw new PhotoExportError('budget')
  return source
}

export function downloadPhoto(name: string, type: string, content: string): void {
  const url = URL.createObjectURL(new Blob([content], {type}))
  try {
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = name
    anchor.click()
  } finally {
    setTimeout(() => URL.revokeObjectURL(url), 1000)
  }
}
