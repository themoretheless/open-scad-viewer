import { defaultGeometryKernel } from './cadGeometryKernel'
import { compileModelGraph, type ModelGraph, type ModelGraphCompilation } from './modelGraph'
import { contoursExtrusion, type SvgContours } from './svgGeometry'
import { MAX_WORKSPACE_SOURCE_LENGTH } from './workspaceDocument'

const MAX_POLYGON_POINTS = 256
const MAX_GRAPH_NODES = 128
const MAX_PROFILE_WORK = 262144

function conversionLimit(reason: string): never {
  throw new Error(`SVG ModelGraph conversion: ${reason}. Increase SVG tolerance or reduce the artwork complexity; omit includeModelGraph to request SCAD only.`)
}

/** Convert canonical SVG profile contours into editable, native ModelGraph nodes.
 * The kernel determines containment; nested differences preserve holes and islands.
 * No tessellation reduction or filled-hole fallback is performed to fit graph limits.
 */
export async function svgProfileModelGraph(contours: SvgContours, height: number): Promise<ModelGraphCompilation> {
  // Shares the SVG/SCAD coordinate, point, height and source-size admission rules.
  contoursExtrusion(contours, height)
  if (contours.length + 1 > MAX_GRAPH_NODES) conversionLimit('the graph requires more than 128 nodes')
  if (contours.some(ring => ring.length > MAX_POLYGON_POINTS)) conversionLimit('a polygon exceeds 256 points')
  if (contours.some(ring => ring.length < 3 || ring.some(point => point.length !== 2 || point.some(value => Math.abs(value) > 1_000_000)))) {
    conversionLimit('polygons require at least three points with coordinates within ±1000000 mm')
  }
  if (contours.reduce((work, ring) => work + ring.length ** 2, 0) > MAX_PROFILE_WORK) {
    conversionLimit('polygon validation exceeds the ModelGraph profile work budget')
  }
  const session = await defaultGeometryKernel.openEvalSession()
  const containers: number[][] = contours.map(() => [])
  try {
    const kernel = session.kernel
    const sections = contours.map(ring => kernel.polygon([ring], 'EvenOdd'))
    if (sections.some(section => kernel.isEmpty(section))) conversionLimit('a contour has no area')
    const bounds = sections.map(section => kernel.bounds(section))
    for (let inner = 0; inner < sections.length; inner++) {
      for (let outer = 0; outer < sections.length; outer++) {
        if (inner === outer) continue
        if ([0, 1].some(axis => bounds[inner].min[axis]! < bounds[outer].min[axis]! || bounds[inner].max[axis]! > bounds[outer].max[axis]!)) continue
        const remaining = kernel.boolean2('difference', [sections[inner]!, sections[outer]!])
        const contained = kernel.isEmpty(remaining)
        kernel.delete(remaining)
        if (contained) containers[inner]!.push(outer)
      }
    }
  } finally {
    session.dispose()
  }

  const children: number[][] = contours.map(() => [])
  const roots: number[] = []
  for (let index = 0; index < contours.length; index++) {
    const enclosing = containers[index]!
    if (enclosing.some(other => containers[other]!.includes(index))) conversionLimit('contour containment is cyclic')
    if (!enclosing.length) { roots.push(index); continue }
    // A canonical nonintersecting contour forest has one deepest enclosing ring.
    const deepest = enclosing.filter(candidate => enclosing.every(other => candidate === other || containers[candidate]!.includes(other)))
    if (deepest.length !== 1) conversionLimit('contour containment is ambiguous')
    children[deepest[0]!]!.push(index)
  }
  if (!roots.length) conversionLimit('contour containment is cyclic')
  const nodes: ModelGraph['nodes'] = []
  let nextId = 0
  const id = () => `svg_${nextId++}`
  function add(node: ModelGraph['nodes'][number]) {
    if (nodes.length >= MAX_GRAPH_NODES - 1) conversionLimit('the graph requires more than 128 nodes')
    nodes.push(node)
    return node.id
  }
  function union(inputs: string[]): string {
    if (inputs.length === 1) return inputs[0]!
    let level = inputs
    while (level.length > 1) {
      const next: string[] = []
      for (let offset = 0; offset < level.length; offset += 32) {
        const batch = level.slice(offset, offset + 32)
        next.push(batch.length === 1 ? batch[0]! : add({ id: id(), op: 'union', inputs: batch }))
      }
      level = next
    }
    return level[0]!
  }
  function region(index: number, depth = 0): string {
    if (depth > 64) conversionLimit('contour nesting exceeds 64 levels')
    const boundary = add({ id: id(), op: 'polygon', points: contours[index]!.map(point => [point[0], point[1]]) })
    const cutouts = children[index]!.map(child => region(child, depth + 1))
    if (!cutouts.length) return boundary
    return add({ id: id(), op: 'difference', base: boundary, subtract: [union(cutouts)] })
  }
  const profile = union(roots.map(index => region(index)))
  nodes.push({ id: 'svg_extrusion', op: 'extrude', input: profile, height: { param: 'svg_height' }, center: false })
  const document: ModelGraph = {
    language: 'modelgraph/1', units: 'mm', segments: 48,
    parameters: [{ id: 'svg_height', value: height, unit: 'mm', min: 0, max: 100000 }],
    nodes, root: 'svg_extrusion',
  }
  // Bound transport size before entering the compiler's own JSON-value budget.
  if (JSON.stringify(document).length > MAX_WORKSPACE_SOURCE_LENGTH) conversionLimit('the document exceeds 250000 characters')
  let compiled: ModelGraphCompilation
  try { compiled = compileModelGraph(document) }
  catch (error) { conversionLimit(error instanceof Error ? error.message : 'the document failed ModelGraph validation') }
  if (compiled.source.length > MAX_WORKSPACE_SOURCE_LENGTH) conversionLimit('the compiled source exceeds 250000 characters')
  return compiled
}
