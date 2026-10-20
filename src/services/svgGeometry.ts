import { readSvgDocumentAsync, type SvgOptions } from './svgDocument'
import type { MeshData } from '../core/mesh'
import { callGeometryRust, warmGeometryKernel } from './geometry/kernel'
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
  const parsed = await readSvgDocumentAsync(svg, options, 'preview')
  return { svg: parsed.normalizedSvg, widthMm: parsed.widthMm, heightMm: parsed.heightMm, warnings: parsed.warnings }
}

export async function svgContours(svg: string, options: SvgOptions = {}): Promise<SvgContours> {
  return (await svgProfile(svg, options)).contours
}

/** Geometry plus conversion diagnostics, shared by the panel and MCP. */
export async function svgProfile(svg: string, options: SvgOptions = {}) {
  const parsed = await readSvgDocumentAsync(svg, options, 'profile')
  return { contours: bounded(parsed.contours ?? []), warnings: parsed.warnings, widthMm: parsed.widthMm, heightMm: parsed.heightMm }
}
export function formatSvgNumber(value: number, precision = 6): string {
  if (!Number.isFinite(value)) throw new Error('SVG coordinate must be finite.')
  const rounded = Number(value.toFixed(precision))
  return Object.is(rounded, -0) ? '0' : String(rounded)
}
export function contoursSvg(contours: SvgContours): string {
  bounded(contours)
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity
  for (const ring of contours) for (const [x, y] of ring) { x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x); y1 = Math.max(y1, y) }
  if (x1 <= x0 || y1 <= y0 || !Number.isFinite(x1 - x0) || !Number.isFinite(y1 - y0)) throw new Error('SVG has no finite area.')
  // A cropped fabrication document has a local origin. Rebase before SVG's
  // single-precision path normalization so distant world translations cannot
  // collapse short edges or holes on reimport.
  const w = formatSvgNumber(x1 - x0)
  const h = formatSvgNumber(y1 - y0)
  const path = contours.map(r => r.map(([x,y], i) => `${i ? 'L' : 'M'}${formatSvgNumber(x-x0)} ${formatSvgNumber(y1-y)}`).join(' ') + ' Z').join(' ')
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}mm" height="${h}mm" viewBox="0 0 ${w} ${h}"><path fill="black" fill-rule="evenodd" d="${path}"/></svg>`
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
  await warmGeometryKernel()
  return callGeometryRust<SvgContours>('mesh_svg_contours', {
    meshes: selected.map(mesh => ({
      vertexBits: new Uint32Array(mesh.vertices.buffer, mesh.vertices.byteOffset, mesh.vertices.length),
      indices: mesh.indices,
      transform: Array.from(mesh.transform),
      faceIds: mesh.faceIds,
    })),
    axis: options.axis,
    faceTriangle: face ? face.triangleIndex : undefined,
  })
}
