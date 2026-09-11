import { parseOpenScadSvg } from './openScadImport'
import type { MeshData } from '../core/mesh'
import { defaultGeometryKernel } from './manifoldGeometryKernel'
import { transformPoint } from './math3d'

export type SvgContours = [number, number][][]
export const SVG_MAX_BYTES = 262144
function bounded(contours: SvgContours) {
  if (contours.reduce((n, ring) => n + ring.length, 0) > 20000) throw new Error('SVG exceeds 20000 contour points.')
  if (!contours.length || contours.some(ring => ring.some(p => !p.every(Number.isFinite)))) throw new Error('SVG has no finite area.')
  return contours
}
export async function svgContours(svg: string): Promise<SvgContours> {
  const byteLength = new TextEncoder().encode(svg).length
  if (byteLength > SVG_MAX_BYTES) throw new Error('SVG exceeds 256 KiB.')
  const parsed = parseOpenScadSvg({ kind: 'source', path: 'profile.svg', source: svg, byteLength, sha256: '' })
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
    return bounded(session.kernel.polygons(merged))
  } finally {
    session.dispose()
  }
}
export function contoursSvg(contours: SvgContours): string {
  bounded(contours)
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity
  for (const ring of contours) for (const [x, y] of ring) { x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x); y1 = Math.max(y1, y) }
  if (x1 <= x0 || y1 <= y0) throw new Error('SVG has no area.')
  const path = contours.map(r => r.map(([x,y], i) => `${i ? 'L' : 'M'}${x} ${y0+y1-y}`).join(' ') + ' Z').join(' ')
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${x1-x0}mm" height="${y1-y0}mm" viewBox="${x0} ${y0} ${x1-x0} ${y1-y0}"><path fill="black" fill-rule="evenodd" d="${path}"/></svg>`
  if (new TextEncoder().encode(svg).length > 4*1024*1024) throw new Error('SVG export exceeds 4 MiB.')
  return svg
}
export function contoursExtrusion(contours: SvgContours, height: number): string {
  bounded(contours)
  if (!Number.isFinite(height) || height <= 0 || height > 100000) throw new Error('Extrusion height must be between 0 and 100000 mm.')
  const points: [number, number][] = [], paths: number[][] = []
  for (const ring of contours) paths.push(ring.map(p => { points.push(p); return points.length-1 }))
  return `linear_extrude(height=${height}) polygon(points=${JSON.stringify(points)},paths=${JSON.stringify(paths)});`
}
/** Silhouette, or one planar kernel face flattened into its own orthonormal plane. */
export async function meshSvgContours(meshes: readonly MeshData[], options: { axis?: 'x'|'y'|'z'; face?: { meshIndex: number; triangleIndex: number } } = {}): Promise<SvgContours> {
  if (meshes.reduce((n,m) => n+m.indices.length/3,0) > 20000) throw new Error('SVG projection is limited to 20000 triangles.')
  const triangles: [number, number][][] = []
  let origin: number[] = [0,0,0], u: number[] = options.axis === 'x' ? [0,1,0] : [1,0,0], v: number[] = options.axis === 'z' || !options.axis ? [0,1,0] : [0,0,1]
  let normal: number[] | undefined, faceId: number | undefined
  const point = (m: MeshData, index: number) => { const i=m.indices[index]*6; return transformPoint(m.transform,[m.vertices[i],m.vertices[i+1],m.vertices[i+2]]) }
  if (options.face) {
    const m=meshes[options.face.meshIndex], t=options.face.triangleIndex
    if (!m || !Number.isInteger(t) || t<0 || t*3+2>=m.indices.length) throw new Error('Select a valid planar face.')
    faceId=m.faceIds[t]; origin=point(m,t*3)
    const b=point(m,t*3+1), c=point(m,t*3+2), ab=b.map((x,i)=>x-origin[i]), ac=c.map((x,i)=>x-origin[i])
    const length=Math.hypot(...ab); u=ab.map(x=>x/length)
    normal=[ab[1]*ac[2]-ab[2]*ac[1],ab[2]*ac[0]-ab[0]*ac[2],ab[0]*ac[1]-ab[1]*ac[0]]
    const nl=Math.hypot(...normal); if (!(nl>0)) throw new Error('Degenerate face.')
    normal=normal.map(x=>x/nl); v=[normal[1]*u[2]-normal[2]*u[1],normal[2]*u[0]-normal[0]*u[2],normal[0]*u[1]-normal[1]*u[0]]
  }
  meshes.forEach((m,mi)=>{
    if(options.face && mi!==options.face.meshIndex) return
    for(let i=0;i<m.indices.length;i+=3) {
      if(options.face && (faceId === undefined ? i/3!==options.face.triangleIndex : m.faceIds[i/3]!==faceId)) continue
      const ring=[0,1,2].map(j=>{
        const p=point(m,i+j).map((x,k)=>x-origin[k])
        if(normal && Math.abs(p.reduce((s,x,k)=>s+x*normal![k],0))>1e-5) throw new Error('The selected surface is not planar; use a projection.')
        return [p.reduce((s,x,k)=>s+x*u[k],0),p.reduce((s,x,k)=>s+x*v[k],0)] as [number, number]
      })
      const area=(ring[1][0]-ring[0][0])*(ring[2][1]-ring[0][1])-(ring[1][1]-ring[0][1])*(ring[2][0]-ring[0][0])
      if(Math.abs(area)>1e-12) triangles.push(area>0?ring:ring.reverse())
    }
  })
  if (!triangles.length) return bounded([])
  const session = await defaultGeometryKernel.openEvalSession()
  try {
    return bounded(session.kernel.polygons(session.kernel.polygon(triangles, 'NonZero')))
  } finally {
    session.dispose()
  }
}
