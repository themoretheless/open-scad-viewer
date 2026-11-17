import type { KernelErrorPresentation } from '../../i18n/kernelErrors'

interface TopologyLike {
  readonly faces: readonly { readonly triangles: readonly number[] }[]
  readonly edges: readonly { readonly faces: readonly [number, number] }[]
}

export interface SubshapeSelection {
  readonly faceIndexes: readonly number[]
  readonly edgeIndexes: readonly number[]
}

/** Inspector fields whose refusal concerns the selected edges (fillet, chamfer, corner blend). */
const EDGE_FIELDS = new Set(['radius', 'endRadius', 'size'])
/** Inspector fields whose refusal concerns the selected faces (offset, shell). */
const FACE_FIELDS = new Set(['distance'])

/**
 * Working-mesh triangles of the faces or edges a refusal points at (roadmap C4), so the GPU
 * viewport can tint them instead of the whole body. Edges mark both adjacent faces. Empty
 * when the error is not about a subshape or nothing is selected; the caller then tints the body.
 */
export function invalidSubshapeTriangles(
  presented: Pick<KernelErrorPresentation, 'input' | 'field'>,
  topology: TopologyLike,
  selection: SubshapeSelection,
): number[] {
  const field = presented.field ?? ''
  const byEdge = presented.input === 'edge' || (presented.input === 'field' && EDGE_FIELDS.has(field))
  const byFace = presented.input === 'face' || (presented.input === 'field' && FACE_FIELDS.has(field))
  const faces = new Set<number>()
  if (byEdge)
    for (const index of selection.edgeIndexes) for (const face of topology.edges[index]?.faces ?? []) faces.add(face)
  if (byFace || (byEdge && !faces.size)) for (const index of selection.faceIndexes) faces.add(index)
  return [...faces].flatMap(index => (index >= 0 ? (topology.faces[index]?.triangles ?? []) : []))
}

/** Hue of one display triangle: the invalid hue on refused triangles, otherwise the body hue. */
export function subshapeTriangleHue(
  working: number,
  bodyHue: number,
  invalidTriangles: ReadonlySet<number> | undefined,
  invalidHue: number,
): number {
  return invalidTriangles?.has(working) ? invalidHue : bodyHue
}
