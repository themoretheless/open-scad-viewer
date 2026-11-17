/** Face and edge selections keyed by a stable identity instead of a tessellation index, so
 * Undo/Redo and unrelated edits restore the same face/edge or drop it, never a different one.
 * Faces are identified by their plane (unit normal) and centroid; edges by a persistent B-rep
 * topology id when the body has one, otherwise by their end points. */
type Vec3 = readonly [number, number, number] | readonly number[]

export interface FaceSignature {
  readonly normal: readonly number[]
  readonly center: readonly number[]
}
export interface EdgeSignature {
  /** Persistent topology id, or '' when the body only has index-derived edge ids. */
  readonly id: string
  /** End points in lexicographic order, so a reversed edge has the same signature. */
  readonly ends: readonly (readonly number[])[]
}
export interface SubshapeAnchor {
  readonly body: string
  readonly face: FaceSignature | null
  readonly openings: readonly FaceSignature[]
  readonly edge: EdgeSignature | null
  readonly edges: readonly EdgeSignature[]
}

export interface AnchorFace {
  readonly normal: Vec3
  readonly center: Vec3
}
export interface AnchorEdge {
  readonly id: string
  readonly points: readonly Vec3[]
}

/** Matching tolerance in model millimetres (normals are unit vectors). */
export const SUBSHAPE_TOLERANCE = 1e-5

const close = (a: readonly number[], b: readonly number[]) =>
  a.length === b.length && a.every((v, i) => Math.abs(v - b[i]) <= SUBSHAPE_TOLERANCE)
const lexLess = (a: readonly number[], b: readonly number[]) => {
  for (let i = 0; i < Math.min(a.length, b.length); i++) if (a[i] !== b[i]) return a[i] < b[i]
  return a.length < b.length
}
// Ids built from the edge index (`body:edge:3`, `body:mesh-edge:3`) do not survive renumbering.
const persistentId = (id: string) => (/:(mesh-)?edge:\d+$/.test(id) ? '' : id)

export function faceSignature(face: AnchorFace): FaceSignature {
  return { normal: [...face.normal], center: [...face.center] }
}
export function edgeSignature(edge: AnchorEdge): EdgeSignature {
  const first = [...(edge.points[0] ?? [])],
    last = [...(edge.points.at(-1) ?? [])]
  return { id: persistentId(edge.id), ends: lexLess(last, first) ? [last, first] : [first, last] }
}

/** Index of the only face matching `signature`, or -1 when none or several match. */
export function resolveFace(faces: readonly AnchorFace[], signature: FaceSignature): number {
  let found = -1
  for (let i = 0; i < faces.length; i++) {
    if (!close(faces[i].normal as number[], signature.normal) || !close(faces[i].center as number[], signature.center))
      continue
    if (found >= 0) return -1
    found = i
  }
  return found
}
/** Index of the edge with the persistent id, else the only edge with the same end points, else -1. */
export function resolveEdge(edges: readonly AnchorEdge[], signature: EdgeSignature): number {
  if (signature.id) {
    const byId = edges.findIndex(edge => edge.id === signature.id)
    if (byId >= 0) return byId
  }
  let found = -1
  for (let i = 0; i < edges.length; i++) {
    const other = edgeSignature(edges[i])
    if (other.ends.length !== signature.ends.length || !other.ends.every((p, k) => close(p, signature.ends[k])))
      continue
    if (found >= 0) return -1
    found = i
  }
  return found
}

export function captureFaces(faces: readonly AnchorFace[], faceIndex: number, openings: readonly number[]) {
  return {
    face: faces[faceIndex] ? faceSignature(faces[faceIndex]) : null,
    openings: openings.flatMap(i => (faces[i] ? [faceSignature(faces[i])] : [])),
  }
}
/** `edgeIndex`/`edgeIndexes` refer to `AnchorEdge` positions in `edges`, which must be indexed by edge index. */
export function captureEdges(edges: readonly AnchorEdge[], edgeIndex: number, edgeIndexes: readonly number[]) {
  return {
    edge: edges[edgeIndex] ? edgeSignature(edges[edgeIndex]) : null,
    edges: edgeIndexes.flatMap(i => (edges[i] ? [edgeSignature(edges[i])] : [])),
  }
}

const resolveAll = <T, S>(
  items: readonly T[],
  signatures: readonly S[],
  resolve: (items: readonly T[], s: S) => number,
) => [...new Set(signatures.map(s => resolve(items, s)).filter(i => i >= 0))]

/** Face selection for `anchor` in `faces`: identities that no longer exist are dropped. */
export function resolveFaces(faces: readonly AnchorFace[], anchor: Pick<SubshapeAnchor, 'face' | 'openings'>) {
  const openings = resolveAll(faces, anchor.openings, resolveFace)
  const primary = anchor.face ? resolveFace(faces, anchor.face) : -1
  return { faceIndex: primary >= 0 ? primary : (openings[0] ?? -1), openingFaces: openings }
}
/** Edge selection for `anchor` in `edges`: identities that no longer exist are dropped. */
export function resolveEdges(edges: readonly AnchorEdge[], anchor: Pick<SubshapeAnchor, 'edge' | 'edges'>) {
  const edgeIndexes = resolveAll(edges, anchor.edges, resolveEdge)
  const primary = anchor.edge ? resolveEdge(edges, anchor.edge) : -1
  return { edgeIndex: primary >= 0 ? primary : (edgeIndexes[0] ?? -1), edgeIndexes }
}
