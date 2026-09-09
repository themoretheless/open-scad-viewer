import {inspectPolygonMesh} from './polygonKernel'
import type {PhotoSurface} from './photogrammetryKernel'

/** Exports the original reconstructed coordinates, colors and all observed triangles. */
export function photoPly(surface: PhotoSurface): string {
  const vertices = surface.positions.length / 3
  const lines = [
    'ply', 'format ascii 1.0', `element vertex ${vertices}`,
    'property float x', 'property float y', 'property float z',
    'property uchar red', 'property uchar green', 'property uchar blue',
    `element face ${surface.triangles.length / 3}`,
    'property list uchar int vertex_indices', 'end_header',
  ]
  for (let i = 0; i < vertices; i++) {
    const base = i * 3
    const colored = base + 2 < surface.colors.length
    lines.push([surface.positions[base], surface.positions[base + 1], surface.positions[base + 2],
      colored ? surface.colors[base] : 180, colored ? surface.colors[base + 1] : 180,
      colored ? surface.colors[base + 2] : 180].join(' '))
  }
  for (let face = 0; face < surface.triangles.length; face += 3) {
    lines.push(`3 ${surface.triangles[face]} ${surface.triangles[face + 1]} ${surface.triangles[face + 2]}`)
  }
  return lines.join('\n') + '\n'
}

/** The solid CAD path cannot accept open or inconsistently oriented scan patches. */
export function photoCanAppend(surface: PhotoSurface): boolean {
  if (!surface.triangles.length) return false
  try {
    const mesh = inspectPolygonMesh({positions: Array.from(surface.positions), indices: Array.from(surface.triangles)})
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
  for (let i = 0; i < surface.positions.length; i += 3) {
    low = Math.min(low, surface.positions[i]!)
    high = Math.max(high, surface.positions[i]!)
  }
  const scale = widthMm / (high - low)
  if (!Number.isFinite(scale) || scale <= 0) throw new PhotoExportError('extent')
  const points: number[][] = []
  for (let i = 0; i < document.positions.length; i += 3) {
    points.push([document.positions[i]!, document.positions[i + 1]!, document.positions[i + 2]!]
      .map(value => Number((value * scale).toPrecision(8))))
  }
  if (points.some(point => point.some(value => !Number.isFinite(value)))) throw new PhotoExportError('extent')
  const faces: number[][] = []
  for (let i = 0; i < document.triangles.length; i += 3) {
    faces.push([document.triangles[i]!, document.triangles[i + 1]!, document.triangles[i + 2]!])
  }
  const source = `// Photo reconstruction: observed surface scaled to a measured full X width.\npolyhedron(points=${JSON.stringify(points)}, faces=${JSON.stringify(faces)}, convexity=10);`
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
