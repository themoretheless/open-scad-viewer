import { readSvgDocument, type SvgOptions } from './svgDocument'
import type { MeshData } from '../core/mesh'
import { defaultGeometryKernel } from './cadGeometryKernel'
import { transformPoint } from './math3d'
import { MAX_WORKSPACE_SOURCE_LENGTH } from './workspaceDocument'

export type SvgContours = [number, number][][]
export { SVG_MAX_BYTES, SVG_MAX_FONT_BYTES, SVG_MAX_TOTAL_FONT_BYTES, SVG_MAX_FONTS } from './svgDocument'
export type { SvgOptions } from './svgDocument'
function bounded(contours: SvgContours) {
  if (contours.reduce((n, ring) => n + ring.length, 0) > 20000) throw new Error('SVG exceeds 20000 contour points.')
  if (!contours.length || contours.some(ring => ring.length < 3 || ring.some(p => p.length !== 2 || !p.every(Number.isFinite)))) throw new Error('SVG has no finite area.')
  for (const ring of contours) {
    const [x, y] = ring[0]
    const twiceArea = ring.reduce((sum, a, i) => {
      const b = ring[(i + 1) % ring.length]
      return sum + (a[0] - x) * (b[1] - y) - (b[0] - x) * (a[1] - y)
    }, 0)
    if (!Number.isFinite(twiceArea) || twiceArea === 0) throw new Error('SVG has no finite area.')
  }
  return contours
}
/** Preserve the static artwork, including paint, masks, filters and outlined text. */
export async function svgPreview(svg: string, options: SvgOptions = {}) {
  const parsed = readSvgDocument(svg, options, 'preview')
  return { svg: parsed.normalizedSvg, widthMm: parsed.widthMm, heightMm: parsed.heightMm, warnings: parsed.warnings }
}

export async function svgContours(svg: string, options: SvgOptions = {}): Promise<SvgContours> {
  return (await svgProfile(svg, options)).contours
}

/** Geometry plus conversion diagnostics, shared by the panel and MCP. */
export async function svgProfile(svg: string, options: SvgOptions = {}) {
  const parsed = readSvgDocument(svg, options)
  const session = await defaultGeometryKernel.openEvalSession()
  try {
    const sections = parsed.regions.map(region => {
      const rings = region.contours.map(ring => ring.map(point => [point[0], point[1]] as [number, number]))
      if (!rings.length || !rings[0]?.length) return session.kernel.empty2()
      return session.kernel.polygon(
        rings,
        region.fillRule === 'evenodd' ? 'EvenOdd' : 'NonZero',
      )
    })
    const merged = sections.length === 0
      ? session.kernel.empty2()
      : sections.length === 1 ? sections[0] : session.kernel.boolean2('union', sections)
    return { contours: bounded(session.kernel.polygons(merged)), warnings: parsed.warnings, widthMm: parsed.widthMm, heightMm: parsed.heightMm }
  } finally {
    session.dispose()
  }
}
export function contoursSvg(contours: SvgContours): string {
  bounded(contours)
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity
  for (const ring of contours) for (const [x, y] of ring) { x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x); y1 = Math.max(y1, y) }
  if (x1 <= x0 || y1 <= y0 || !Number.isFinite(x1 - x0) || !Number.isFinite(y1 - y0)) throw new Error('SVG has no finite area.')
  // A cropped fabrication document has a local origin. Rebase before SVG's
  // single-precision path normalization so distant world translations cannot
  // collapse short edges or holes on reimport.
  const path = contours.map(r => r.map(([x,y], i) => `${i ? 'L' : 'M'}${x-x0} ${y1-y}`).join(' ') + ' Z').join(' ')
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${x1-x0}mm" height="${y1-y0}mm" viewBox="0 0 ${x1-x0} ${y1-y0}"><path fill="black" fill-rule="evenodd" d="${path}"/></svg>`
  if (new TextEncoder().encode(svg).length > 4*1024*1024) throw new Error('SVG export exceeds 4 MiB.')
  return svg
}
export function contoursExtrusion(contours: SvgContours, height: number): string {
  bounded(contours)
  if (!Number.isFinite(height) || height <= 0 || height > 100000) throw new Error('Extrusion height must be between 0 and 100000 mm.')
  const points: [number, number][] = [], paths: number[][] = []
  for (const ring of contours) paths.push(ring.map(p => { points.push(p); return points.length-1 }))
  const source = `linear_extrude(height=${height}) polygon(points=${JSON.stringify(points)},paths=${JSON.stringify(paths)});`
  if (source.length > MAX_WORKSPACE_SOURCE_LENGTH) throw new Error('SVG extrusion exceeds the 250000-character model limit. Increase curve tolerance or reduce silhouette resolution.')
  return source
}
export interface SvgProjectionOptions {
  axis?: 'x' | 'y' | 'z'
  face?: { meshIndex: number; triangleIndex: number }
}
export type SvgProjectionMesh = Pick<MeshData, 'vertices' | 'indices' | 'transform' | 'faceIds'>

/** Silhouette, or one planar kernel face flattened into its own orthonormal plane. */
export async function meshSvgContours(meshes: readonly SvgProjectionMesh[], options: SvgProjectionOptions = {}): Promise<SvgContours> {
  if (options.axis !== undefined && !['x', 'y', 'z'].includes(options.axis)) throw new Error('SVG projection axis must be x, y or z.')
  const face = options.face
  if (face && (!Number.isInteger(face.meshIndex) || face.meshIndex < 0 || !meshes[face.meshIndex]
    || !Number.isInteger(face.triangleIndex) || face.triangleIndex < 0
    || face.triangleIndex * 3 + 2 >= meshes[face.meshIndex].indices.length)) {
    throw new Error('Select a valid planar face.')
  }
  const selected = face ? [meshes[face.meshIndex]] : meshes
  for (const mesh of selected) {
    if (mesh.vertices.length % 6 || mesh.indices.length % 3 || mesh.transform.length !== 16
      || !mesh.transform.every(Number.isFinite)
      || mesh.transform[12] !== 0 || mesh.transform[13] !== 0 || mesh.transform[14] !== 0 || mesh.transform[15] !== 1
      || (mesh.faceIds.length !== 0 && mesh.faceIds.length !== mesh.indices.length / 3)) {
      throw new Error('SVG projection requires a valid triangle mesh and affine transform.')
    }
  }
  const faceId = face ? selected[0].faceIds[face.triangleIndex] : undefined
  const includesTriangle = (mesh: SvgProjectionMesh, triangle: number) => !face
    || (faceId === undefined ? triangle === face.triangleIndex : mesh.faceIds[triangle] === faceId)
  // A small selected face remains exportable even in a large assembly.
  let triangleCount = 0
  for (const mesh of selected) for (let triangle = 0; triangle < mesh.indices.length / 3; triangle++) {
    if (includesTriangle(mesh, triangle) && ++triangleCount > 20000) throw new Error('SVG projection is limited to 20000 triangles.')
  }
  const point = (mesh: SvgProjectionMesh, index: number): [number, number, number] => {
    const vertex = mesh.indices[index]
    if (!Number.isInteger(vertex) || vertex < 0 || vertex * 6 + 2 >= mesh.vertices.length) throw new Error('SVG projection contains an invalid vertex index.')
    const position = transformPoint(mesh.transform, [mesh.vertices[vertex * 6], mesh.vertices[vertex * 6 + 1], mesh.vertices[vertex * 6 + 2]])
    if (!position.every(Number.isFinite)) throw new Error('SVG projection contains non-finite coordinates.')
    return position
  }
  let origin = [0, 0, 0]
  let u = options.axis === 'x' ? [0, 1, 0] : [1, 0, 0]
  let v = options.axis === 'z' || !options.axis ? [0, 1, 0] : [0, 0, 1]
  let normal: number[] | undefined
  if (face) {
    const mesh = selected[0], triangle = face.triangleIndex
    origin = point(mesh, triangle * 3)
    const b = point(mesh, triangle * 3 + 1), c = point(mesh, triangle * 3 + 2)
    const ab = b.map((x, i) => x - origin[i]), ac = c.map((x, i) => x - origin[i])
    const length = Math.hypot(...ab)
    normal = [ab[1] * ac[2] - ab[2] * ac[1], ab[2] * ac[0] - ab[0] * ac[2], ab[0] * ac[1] - ab[1] * ac[0]]
    const normalLength = Math.hypot(...normal)
    if (!(normalLength > 0) || !Number.isFinite(normalLength)) throw new Error('Degenerate face.')
    u = ab.map(x => x / length)
    normal = normal.map(x => x / normalLength)
    v = [normal[1] * u[2] - normal[2] * u[1], normal[2] * u[0] - normal[0] * u[2], normal[0] * u[1] - normal[1] * u[0]]
  }
  const triangles: SvgContours = []
  for (const mesh of selected) for (let i = 0; i < mesh.indices.length; i += 3) {
    if (!includesTriangle(mesh, i / 3)) continue
    const ring = [0, 1, 2].map(j => {
      const p = point(mesh, i + j).map((x, k) => x - origin[k])
      if (normal && Math.abs(p.reduce((sum, x, k) => sum + x * normal![k], 0)) > 1e-5) {
        throw new Error('The selected surface is not planar; use a projection.')
      }
      return [p.reduce((sum, x, k) => sum + x * u[k], 0), p.reduce((sum, x, k) => sum + x * v[k], 0)] as [number, number]
    })
    const area = (ring[1][0] - ring[0][0]) * (ring[2][1] - ring[0][1]) - (ring[1][1] - ring[0][1]) * (ring[2][0] - ring[0][0])
    if (!Number.isFinite(area)) throw new Error('SVG projection coordinates exceed the numeric range.')
    if (Math.abs(area) > 1e-12) triangles.push(area > 0 ? ring : ring.reverse())
  }
  if (!triangles.length) return bounded([])
  const session = await defaultGeometryKernel.openEvalSession()
  try {
    return bounded(session.kernel.polygons(session.kernel.polygon(triangles, 'NonZero')))
  } finally {
    session.dispose()
  }
}
