import type { DirectBody } from './directModeling'
import { callGeometryRust } from './geometry/kernel'
import type { SnapGeometry, SnapSegment } from './modelingSnaps'

const cache = new WeakMap<DirectBody, SnapGeometry>()
type NativeSegment = Pick<SnapSegment, 'a' | 'b'> & {
  interval?: { curve: number; start: number; end: number }
}
/** Stable topological targets. Rust owns geometry; this adapter retains curve references and the body cache. */
export function bodySnapGeometry(body: DirectBody): SnapGeometry {
  const cached = cache.get(body)
  if (cached) return cached
  const result = callGeometryRust<{ points: SnapGeometry['points']; segments: NativeSegment[] }>(
    'cad_body_keypoints', { mesh: body.mesh, brep: body.brep },
  )
  const geometry: SnapGeometry = {
    points: result.points,
    segments: result.segments.map(({ a, b, interval }) => {
      if (!interval) return { a, b }
      const curve = body.brep?.edges[interval.curve]?.curve
      if (!curve) throw new Error('Native snap interval references a missing BRep curve')
      return { a, b, nurbs: { curve, start: interval.start, end: interval.end } }
    }),
  }
  cache.set(body, geometry)
  return geometry
}
