/**
 * Executes a recorded exact graph and turns each root into a Solid workspace body.
 *
 * A `DirectBody` carries both representations: the exact `brep` the Solid tools operate
 * on, and the `mesh` that is drawn. The NURBS kernel produces both from one build, the
 * body's own validation then checks that they agree, so nothing here invents geometry.
 *
 * A root that the recorder marked inexact is refused by name rather than approximated.
 */
import { buildOwnNurbs } from '../modelGraphNurbsKernel'
import { brepGearFaceCount, type NurbsBrep } from '../geometry/brep'
import { normalizePolygonMesh } from '../geometry/polygon'
import type { DirectBody } from '../directModeling'
import type { BrepNode } from './brepGraph'

/** Display tessellation only; the stored body keeps its exact topology. */
const DISPLAY_SEGMENTS = 8

/**
 * The bridge refuses a display mesh above this many triangles. A face at `segments`
 * per edge costs about 2*segments^2 of them, so a body with many faces (a gear has
 * six per tooth) is drawn coarser rather than refused.
 */
const TRIANGLE_BUDGET = 16000

/** The NURBS document contract caps a graph at 128 nodes. */
const MAX_NODES = 128

export type ExactSolidRoot =
  | { name: string; id: string }
  | { name: string; inexact: string }

export class InexactSolidError extends Error {
  constructor(readonly bodyName: string, readonly operation: string) {
    super(`"${bodyName}" uses ${operation}, which has no exact solid form. Open this model in Mesh instead.`)
    this.name = 'InexactSolidError'
  }
}

/**
 * Builds one body per root. Roots are built separately so the Solid workspace keeps
 * them as distinct bodies, which is also what the source expressed.
 */
export function buildExactSolidBodies(
  nodes: readonly BrepNode[],
  roots: readonly ExactSolidRoot[],
): DirectBody[] {
  const bodies: DirectBody[] = []
  for (const root of roots) {
    if ('inexact' in root) throw new InexactSolidError(root.name, root.inexact)
    bodies.push(buildExactSolidBody(nodes, root.id, root.name))
  }
  return bodies
}

function buildExactSolidBody(
  nodes: readonly BrepNode[],
  root: string,
  name: string,
): DirectBody {
  const reachable = reachableFrom(nodes, root)
  if (reachable.length > MAX_NODES) {
    throw new Error(
      `"${name}" needs ${reachable.length} exact operations, more than the ${MAX_NODES} a NURBS document allows.`,
    )
  }
  const document = {
    language: 'modelgraph/nurbs-1',
    units: 'mm',
    parameters: [],
    nodes: reachable,
    root,
  }
  const built = buildOwnNurbs(document, {
    action: 'build',
    display: { segments: displaySegments(reachable), subdivisionLevels: 1 },
  })
  const mesh = (built as { mesh?: { positions: ArrayLike<number>; indices: ArrayLike<number> } }).mesh
  if (!mesh) throw new Error(`"${name}" produced no displayable geometry.`)
  const definitions = built.report.definitions as Record<string, { kind: string } & Record<string, unknown>>
  const definition = definitions[root]
  if (definition?.kind !== 'brep') {
    throw new Error(`"${name}" did not resolve to an exact solid.`)
  }
  const { kind, ...brep } = definition
  return {
    id: crypto.randomUUID(),
    name,
    mesh: normalizePolygonMesh({ positions: mesh.positions, indices: mesh.indices }),
    brep: brep as unknown as NurbsBrep,
  }
}

/** Coarser display for bodies whose face count would overrun the triangle budget. */
function displaySegments(nodes: readonly BrepNode[]): number {
  let faces = 0
  for (const node of nodes) {
    if (node.op !== 'brep_gear') continue
    const gear = node as unknown as { teeth: number; herringbone?: boolean; bore?: number; internal?: boolean }
    faces += brepGearFaceCount(gear)
  }
  let segments = DISPLAY_SEGMENTS
  while (segments > 1 && 2 * faces * segments * segments > TRIANGLE_BUDGET) segments--
  return segments
}

/**
 * The nodes one root actually needs. Roots are built independently, so sending the
 * whole recording every time would spend the node budget on unrelated bodies.
 */
function reachableFrom(nodes: readonly BrepNode[], root: string): BrepNode[] {
  const byId = new Map(nodes.map(node => [node.id, node]))
  const seen = new Set<string>()
  const order: BrepNode[] = []
  const visit = (id: string) => {
    if (seen.has(id)) return
    const node = byId.get(id)
    if (!node) throw new Error(`Exact graph references a missing node ${id}.`)
    seen.add(id)
    for (const reference of references(node)) visit(reference)
    order.push(node)
  }
  visit(root)
  return order
}

/** Node ids a node depends on, across every shape the contract uses. */
function references(node: BrepNode): string[] {
  const out: string[] = []
  const input = node.input
  if (typeof input === 'string') out.push(input)
  const inputs = node.inputs
  if (Array.isArray(inputs)) for (const value of inputs) if (typeof value === 'string') out.push(value)
  const loops = node.loops
  if (Array.isArray(loops)) {
    for (const loop of loops) {
      if (!Array.isArray(loop)) continue
      for (const value of loop) if (typeof value === 'string') out.push(value)
    }
  }
  return out
}
