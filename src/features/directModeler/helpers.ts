// Pure helpers of DirectModeler.vue: no component state, extracted by scripts/vue-extract-pure.mjs.
import { type PolygonMesh } from '../../services/geometry/polygon'
import { type DisplayMesh } from '../../services/solidDisplayCache'

export type Pane = '2d' | '3d'

export const touchPointers = new Map<number, { x: number; y: number; pane: Pane; svg: SVGSVGElement }>()

export const panes: Pane[] = ['2d', '3d']

/**
 * Elements offset directly while a body drag is in flight.
 *
 * The 3D view renders one SVG polygon per triangle, and this scene has thousands. Both
 * re-deriving the projection and re-rendering the list through Vue cost tens of
 * milliseconds per pointer move. An orthographic projection is affine, so translating a
 * body in the world is a constant screen offset: the drag sets that offset straight on
 * the dragged body's existing elements, leaving the reactive scene untouched until the
 * pointer is released.
 */
/**
 * Elements that must travel with dragged bodies: their hit polygons (when rendered) and
 * the SVG overlays drawn for the selection, such as vertex handles, feature edges and
 * the gizmo. Overlays are keyed by whitespace-separated body ids, so a gizmo shared by a
 * multi-selection matches any of its bodies. Without this the GPU surface moved while
 * the handles stayed behind, which reads as lag.
 */
export function collectDragNodes(svg: SVGSVGElement | undefined, ids: readonly string[]): SVGElement[] {
  if (typeof svg?.querySelectorAll !== 'function') return []
  const seen = new Set<SVGElement>()
  for (const id of ids) {
    const escaped = id.replace(/["\\]/g, '\\$&')
    for (const node of svg.querySelectorAll<SVGElement>(`[data-body="${escaped}"], [data-body-overlay~="${escaped}"]`)) seen.add(node)
  }
  return [...seen]
}

export function profileBooleanOperation(id:string|null):'union'|'difference'|'intersection'|null {
 return id==='profile-union'?'union':id==='profile-difference'?'difference':id==='profile-intersection'?'intersection':null
}

export function isProfileCommand(id:string|null){return id==='profile-prepare'||!!profileBooleanOperation(id)}

export function selectIndexKey(e:KeyboardEvent,current:number,last:number,min=0){
 if(!['ArrowUp','ArrowDown','Home','End'].includes(e.key))return null
 e.preventDefault();e.stopPropagation()
 return e.key==='Home'?min:e.key==='End'?last:Math.max(min,Math.min(last,current+(e.key==='ArrowDown'?1:-1)))
}

export function download(text: string, name: string, mimeType = 'text/plain') { const url = URL.createObjectURL(new Blob([text], { type: mimeType })); const a = window.document.createElement('a'); a.href = url; a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(url), 1000) }

export function resizeSplit(e: PointerEvent) {
  if (e.button !== 0) return
  const target = e.currentTarget as HTMLElement
  target.setPointerCapture(e.pointerId)
}

export function triangleNormals(mesh: PolygonMesh) {
  const count = mesh.indices.length / 3, normals: number[][] = []
  for (let t = 0; t < count; t++) {
    const [a, b, c] = [0, 1, 2].map(k => { const i = mesh.indices[t * 3 + k]; return mesh.positions.slice(i * 3, i * 3 + 3) })
    const u = b.map((v, k) => v - a[k]), w = c.map((v, k) => v - a[k])
    const n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]], len = Math.hypot(...n) || 1
    normals.push(n.map(v => v / len))
  }
  return normals
}

export const COARSE_DISPLAY: Pick<DisplayMesh, 'map'> = { map: null }

export function canvasOf(e: PointerEvent): SVGSVGElement { const target = e.target as SVGElement; return (target instanceof SVGSVGElement ? target : target.ownerSVGElement)! }

export const primitiveKinds = ['box','wedge','cylinder','frustum','tube','cone','sphere','torus'] as const

// 24x24 stroke icons for the primitive bar; labels stay in the tooltip and aria-label.
export const PRIMITIVE_ICONS: Record<typeof primitiveKinds[number] | 'curve' | 'surface', string> = {
  box: 'M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7M12 11v10',
  wedge: 'M4 17V9l16 8zM4 9l8-4 12 12-4 0M12 5v12',
  cylinder: 'M12 3c-4.4 0-8 1.3-8 3s3.6 3 8 3 8-1.3 8-3-3.6-3-8-3zM4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6',
  frustum: 'M9 4h6l5 14c0 1.7-3.6 3-8 3s-8-1.3-8-3zM9 4c0 1 1.3 1.5 3 1.5S15 5 15 4',
  tube: 'M12 3c-4.4 0-8 1.3-8 3s3.6 3 8 3 8-1.3 8-3-3.6-3-8-3zM4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6M12 4.5c-2 0-3.5.7-3.5 1.5s1.5 1.5 3.5 1.5 3.5-.7 3.5-1.5-1.5-1.5-3.5-1.5',
  cone: 'M12 3l8 15c0 1.7-3.6 3-8 3s-8-1.3-8-3zM4 18c0-1.7 3.6-3 8-3s8 1.3 8 3',
  sphere: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM3 12h18M12 3c-3 2.5-3 15.5 0 18M12 3c3 2.5 3 15.5 0 18',
  torus: 'M12 6c-5.5 0-10 2.7-10 6s4.5 6 10 6 10-2.7 10-6-4.5-6-10-6zM12 10c-2.2 0-4 .9-4 2s1.8 2 4 2 4-.9 4-2-1.8-2-4-2',
  curve: 'M3 18c4-12 8 12 12 0s3-6 6-6M3 18h.01M21 12h.01',
  surface: 'M3 8c3-3 6 3 9 0s6-3 9 0v9c-3 3-6-3-9 0s-6 3-9 0zM3 12.5c3-3 6 3 9 0s6-3 9 0',
}

export const DEFAULT_GROUP_SOURCE = 'cube([20, 20, 20], center = true);\n'
