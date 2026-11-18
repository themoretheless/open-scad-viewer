import { tessellateNurbsBrep, type NurbsBrep } from '../../kernel/geometry/brep'
import { evaluateNurbsCurvePoints } from '../../kernel/nurbsCurve'

type Vec3 = [number, number, number]
export interface BrepFaceQuery {
  normal?: Vec3 | undefined
  angle_tolerance?: number | undefined
  type?: 'planar' | 'curved' | undefined
  min_area?: number | undefined
  max_area?: number | undefined
}
export interface BrepEdgeQuery {
  direction?: Vec3 | undefined
  angle_tolerance?: number | undefined
  min_length?: number | undefined
  max_length?: number | undefined
  convexity?: 'convex' | 'concave' | 'smooth' | undefined
  face?: BrepFaceQuery | undefined
}
export interface BrepFaceFacts {
  area: number
  normal: Vec3 | null
  type: 'planar' | 'curved'
}
export interface BrepEdgeFacts {
  length: number
  direction: Vec3 | null
  faces: number[]
  convexity: 'convex' | 'concave' | 'smooth' | 'boundary'
}

const SAMPLES = 16
const sub = (a: readonly number[], b: readonly number[]): Vec3 => [a[0]! - b[0]!, a[1]! - b[1]!, a[2]! - b[2]!]
const dot = (a: readonly number[], b: readonly number[]) => a[0]! * b[0]! + a[1]! * b[1]! + a[2]! * b[2]!
const cross = (a: Vec3, b: Vec3): Vec3 => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
]
const norm = (a: readonly number[]) => Math.hypot(a[0]!, a[1]!, a[2]!)
const unit = (a: Vec3): Vec3 | null => {
  const n = norm(a)
  return n > 1e-12 ? [a[0] / n, a[1] / n, a[2] / n] : null
}
const parallel = (a: Vec3, b: Vec3, toleranceDeg: number) =>
  Math.abs(dot(a, b)) >= Math.cos((toleranceDeg * Math.PI) / 180) - 1e-12
const aligned = (a: Vec3, b: Vec3, toleranceDeg: number) =>
  dot(a, b) >= Math.cos((toleranceDeg * Math.PI) / 180) - 1e-12

interface Triangle {
  face: number
  centroid: Vec3
  normal: Vec3
  area: number
}

/** Topology and sampled-geometry facts used by edge and face queries. */
export function analyzeBrepSelection(model: NurbsBrep): { edges: BrepEdgeFacts[]; faces: BrepFaceFacts[] } {
  const mesh = tessellateNurbsBrep(model, 4)
  const triangles: Triangle[] = []
  for (let t = 0; t < mesh.indices.length / 3; t++) {
    const p = [0, 1, 2].map(k => {
      const i = mesh.indices[t * 3 + k]! * 3
      return [mesh.positions[i]!, mesh.positions[i + 1]!, mesh.positions[i + 2]!] as Vec3
    })
    const n = cross(sub(p[1]!, p[0]!), sub(p[2]!, p[0]!))
    const area = norm(n) / 2
    const normal = unit(n)
    if (!normal) continue
    triangles.push({
      face: mesh.faceIds[t]!,
      area,
      normal,
      centroid: [0, 1, 2].map(k => (p[0]![k]! + p[1]![k]! + p[2]![k]!) / 3) as Vec3,
    })
  }
  const faces: BrepFaceFacts[] = model.faces.map((_, face) => {
    const own = triangles.filter(t => t.face === face)
    const area = own.reduce((sum, t) => sum + t.area, 0)
    const first = own[0]?.normal ?? null
    const planar = !!first && own.every(t => dot(t.normal, first) > 1 - 1e-9)
    return { area, normal: planar ? first : null, type: planar ? 'planar' : 'curved' }
  })
  const edgeFaces = model.edges.map(() => new Set<number>())
  model.faces.forEach((face, f) => {
    for (const loop of [face.outer, ...face.holes])
      for (const coedge of model.loops[loop]!.coedges) edgeFaces[coedge.edge]!.add(f)
  })
  const edges = model.edges.map((edge, e): BrepEdgeFacts => {
    const facesOfEdge = [...edgeFaces[e]!]
    if (edge.degenerate || !edge.curve) return { length: 0, direction: null, faces: facesOfEdge, convexity: 'boundary' }
    const { knots, degree, controlPoints } = edge.curve
    const a = knots[degree]!,
      b = knots[controlPoints.length]!
    const points = evaluateNurbsCurvePoints(
      edge.curve,
      Array.from({ length: SAMPLES + 1 }, (_, i) => a + ((b - a) * i) / SAMPLES),
    )
    let length = 0
    for (let i = 1; i < points.length; i++) length += norm(sub(points[i]!, points[i - 1]!))
    const chord = sub(points[points.length - 1]!, points[0]!)
    const chordDirection = unit(chord)
    const straight = !!chordDirection && Math.abs(norm(chord) - length) <= 1e-9 * Math.max(1, length)
    const mid = points[SAMPLES / 2]! as Vec3
    let convexity: BrepEdgeFacts['convexity'] = 'boundary'
    if (facesOfEdge.length === 2) {
      const nearest = (face: number) => {
        let best: Triangle | undefined
        let distance = Infinity
        for (const t of triangles) {
          if (t.face !== face) continue
          const d = norm(sub(t.centroid, mid))
          if (d < distance) [best, distance] = [t, d]
        }
        return best
      }
      const ta = nearest(facesOfEdge[0]!),
        tb = nearest(facesOfEdge[1]!)
      if (ta && tb) {
        if (dot(ta.normal, tb.normal) > Math.cos((1 * Math.PI) / 180)) convexity = 'smooth'
        else {
          const side = dot(sub(tb.centroid, mid), ta.normal)
          convexity = side < 0 ? 'convex' : 'concave'
        }
      }
    }
    return { length, direction: straight ? chordDirection : null, faces: facesOfEdge, convexity }
  })
  return { edges, faces }
}

function faceMatches(face: BrepFaceFacts, query: BrepFaceQuery) {
  if (query.type && face.type !== query.type) return false
  if (query.min_area !== undefined && face.area < query.min_area) return false
  if (query.max_area !== undefined && face.area > query.max_area) return false
  if (query.normal) {
    const wanted = unit(query.normal)
    if (!wanted) throw new Error('Face query normal must be nonzero.')
    if (!face.normal || !aligned(face.normal, wanted, query.angle_tolerance ?? 1)) return false
  }
  return true
}

/** Indices of faces matching a query, in face order. */
export function selectBrepFaces(model: NurbsBrep, query: BrepFaceQuery): number[] {
  return analyzeBrepSelection(model).faces.flatMap((face, i) => (faceMatches(face, query) ? [i] : []))
}

/** Resolves an explicit index list or an edge query to the sorted edge indices that chamfer and fillet take. */
export function selectBrepEdges(model: NurbsBrep, selection: number[] | BrepEdgeQuery): number[] {
  if (Array.isArray(selection)) return selection
  const { edges, faces } = analyzeBrepSelection(model)
  const wanted = selection.direction ? unit(selection.direction) : null
  if (selection.direction && !wanted) throw new Error('Edge query direction must be nonzero.')
  const matchingFaces = selection.face
    ? new Set(faces.flatMap((face, i) => (faceMatches(face, selection.face!) ? [i] : [])))
    : null
  const result = edges.flatMap((edge, i) => {
    if (edge.length <= 0) return []
    if (wanted && (!edge.direction || !parallel(edge.direction, wanted, selection.angle_tolerance ?? 1))) return []
    if (selection.min_length !== undefined && edge.length < selection.min_length) return []
    if (selection.max_length !== undefined && edge.length > selection.max_length) return []
    if (selection.convexity && edge.convexity !== selection.convexity) return []
    if (matchingFaces && !edge.faces.some(f => matchingFaces.has(f))) return []
    return [i]
  })
  if (!result.length) throw new Error('Edge query matched no B-rep edges.')
  if (result.length > 256) throw new Error('Edge query matched more than 256 edges.')
  return result
}
