import { createNurbsSurfaceEvaluator, validateNurbsSurface, type NurbsSurface } from './nurbsSurface'

export type NurbsUV = number[]
export type NurbsTrim = { outer: NurbsUV[]; holes?: NurbsUV[][] }
export type NurbsMesh = {
  positions: number[]
  indices: number[]
  uv?: number[]
  report: {
    triangleCount: number
    vertexCount: number
    boundaryEdges: number
    nonManifoldEdges: number
    orientationConflicts: number
    degenerateTriangles: number
    closed: boolean
    signedVolumeMm3: number
    uvArea?: number
    sampledDeviationMm?: number
    errorBoundCertified: false
    selfIntersectionStatus: 'not_checked'
    construction: 'sampled_surface' | 'fixed_vector_thickening'
    parameterSeamsWelded?: { u: boolean; v: boolean }
    collapsedBoundaryCount?: number
  }
}
export type NurbsTessellationOptions = { segmentsU: number; segmentsV: number; trim?: NurbsTrim; maxTriangles?: number }

const MAX_TRIANGLES = 20_000
const cross2 = (a: NurbsUV, b: NurbsUV, c: NurbsUV) => (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
const fail = (message: string): never => { throw new Error(`NURBS tessellation: ${message}`) }
const area = (p: NurbsUV[]) => p.reduce((sum, a, i) => { const b = p[(i + 1) % p.length]; return sum + (a[0] - p[0][0]) * (b[1] - p[0][1]) - (b[0] - p[0][0]) * (a[1] - p[0][1]) }, 0) / 2
const close = (a: number, b: number, eps: number) => Math.abs(a - b) <= eps
const same = (a: NurbsUV, b: NurbsUV, eps: number) => close(a[0], b[0], eps) && close(a[1], b[1], eps)
const between = (a: number, low: number, high: number, eps: number) => a >= Math.min(low, high) - eps && a <= Math.max(low, high) + eps
const pointOnSegment = (p: NurbsUV, a: NurbsUV, b: NurbsUV, eps: number) => Math.abs(cross2(a, b, p)) <= eps * Math.hypot(b[0] - a[0], b[1] - a[1]) && between(p[0], a[0], b[0], eps) && between(p[1], a[1], b[1], eps)
function intersects(a: NurbsUV, b: NurbsUV, c: NurbsUV, d: NurbsUV, eps: number): boolean {
  if (pointOnSegment(a, c, d, eps) || pointOnSegment(b, c, d, eps) || pointOnSegment(c, a, b, eps) || pointOnSegment(d, a, b, eps)) return true
  return (cross2(a, b, c) > 0) !== (cross2(a, b, d) > 0) && (cross2(c, d, a) > 0) !== (cross2(c, d, b) > 0)
}
function inside(p: NurbsUV, polygon: NurbsUV[]): boolean {
  let result = false
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const a = polygon[i], b = polygon[j]
    if ((a[1] > p[1]) !== (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]) result = !result
  }
  return result
}
function clean(polygon: NurbsUV[], eps: number): NurbsUV[] {
  const result = polygon.filter((p, i) => !same(p, polygon[(i + polygon.length - 1) % polygon.length], eps))
  return result.length >= 3 && Math.abs(area(result)) > eps * eps ? result : []
}
function validateLoops(trim: NurbsTrim, domain: [number, number, number, number], eps: number): NurbsUV[][] {
  const loops = [trim.outer, ...(trim.holes ?? [])]
  if (loops.length > 17 || loops.reduce((sum, p) => sum + p.length, 0) > 512) fail('At most 16 holes and 512 total trim vertices are supported.')
  for (const loop of loops) {
    if (!Array.isArray(loop) || loop.length < 3) fail('Each trim loop needs at least three vertices.')
    for (const p of loop) {
      if (!Array.isArray(p) || p.length !== 2 || !p.every(Number.isFinite)) fail('Trim vertices must be finite UV pairs.')
      if (!between(p[0], domain[0], domain[1], eps) || !between(p[1], domain[2], domain[3], eps)) fail('Trim lies outside the surface parameter domain.')
    }
    if (Math.abs(area(loop)) <= eps * eps) fail('Trim loop has zero area.')
    for (let i = 0; i < loop.length; i++) {
      const a = loop[i], b = loop[(i + 1) % loop.length]
      if (same(a, b, eps)) fail('Trim loop has duplicate adjacent vertices; omit a repeated closing point.')
      for (let j = i + 1; j < loop.length; j++) {
        if (j === i + 1 || (i === 0 && j === loop.length - 1)) continue
        if (intersects(a, b, loop[j], loop[(j + 1) % loop.length], eps)) fail('Trim loops must be simple and cannot self-intersect.')
      }
    }
  }
  for (let i = 0; i < loops.length; i++) for (let j = i + 1; j < loops.length; j++) {
    const a = loops[i], b = loops[j]
    for (let k = 0; k < a.length; k++) for (let l = 0; l < b.length; l++) {
      if (intersects(a[k], a[(k + 1) % a.length], b[l], b[(l + 1) % b.length], eps)) fail('Trim loops cannot cross or touch one another.')
    }
    if (i > 0 && (inside(a[0], b) || inside(b[0], a))) fail('Holes cannot contain or overlap other holes.')
  }
  if (loops.slice(1).some(loop => !inside(loop[0], loops[0]))) fail('Every hole must lie strictly inside the outer loop.')
  return loops.map(loop => loop.map(p => [...p] as NurbsUV))
}
function unique(values: number[], eps: number): number[] {
  return values.sort((a, b) => a - b).filter((value, index, sorted) => index === 0 || !close(value, sorted[index - 1], eps))
}
function clipHorizontal(polygon: NurbsUV[], y: number, keepAbove: boolean, eps: number): NurbsUV[] {
  const output: NurbsUV[] = []
  const accepted = (p: NurbsUV) => keepAbove ? p[1] >= y : p[1] <= y
  for (let i = 0; i < polygon.length; i++) {
    const a = polygon[i], b = polygon[(i + 1) % polygon.length]
    if (accepted(a)) output.push(a)
    if (accepted(a) !== accepted(b)) output.push([a[0] + (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]), y])
  }
  return clean(output, eps)
}

/** Numerical polygonal UV clipping; no triangle is accepted by a centroid-only trim test. */
export function tessellateNurbsSurface(surface: NurbsSurface, options: NurbsTessellationOptions): NurbsMesh {
  validateNurbsSurface(surface)
  const evaluateSurface = createNurbsSurfaceEvaluator(surface)
  for (const segments of [options.segmentsU, options.segmentsV]) if (!Number.isInteger(segments) || segments < 1 || segments > 128) fail('Segment counts must be integers from 1 to 128.')
  const budget = options.maxTriangles ?? MAX_TRIANGLES
  if (!Number.isInteger(budget) || budget < 1 || budget > MAX_TRIANGLES) fail('Triangle budget must be an integer from 1 to 20000.')
  const actualDomain: [number, number, number, number] = [surface.knotsU[surface.degreeU], surface.knotsU[surface.controlPoints.length], surface.knotsV[surface.degreeV], surface.knotsV[surface.controlPoints[0].length]]
  const spanU = actualDomain[1] - actualDomain[0], spanV = actualDomain[3] - actualDomain[2]
  // Geometry must not depend on the arbitrary numerical scale or origin of
  // spline parameters. Clip in a normalized domain and return original UVs.
  const domain: [number, number, number, number] = [0, 1, 0, 1], eps = 1e-11
  const evaluate = (u: number, v: number) => evaluateSurface(actualDomain[0] + u * spanU, actualDomain[2] + v * spanV)
  const normalizeLoop = (loop: NurbsUV[]) => loop.map(p => {
    if (!Array.isArray(p) || p.length !== 2 || !p.every(Number.isFinite)) return fail('Trim vertices must be finite UV pairs.')
    return [(p[0] - actualDomain[0]) / spanU, (p[1] - actualDomain[2]) / spanV]
  })
  const outer: NurbsUV[] = [[domain[0], domain[2]], [domain[1], domain[2]], [domain[1], domain[3]], [domain[0], domain[3]]]
  const loops = validateLoops(options.trim ? { outer: normalizeLoop(options.trim.outer), holes: options.trim.holes?.map(normalizeLoop) } : { outer }, domain, eps)
  const gridU = Array.from({ length: options.segmentsU + 1 }, (_, i) => domain[0] + (domain[1] - domain[0]) * i / options.segmentsU)
  const gridV = Array.from({ length: options.segmentsV + 1 }, (_, i) => domain[2] + (domain[3] - domain[2]) * i / options.segmentsV)
  const xs = unique([...gridU, ...loops.flatMap(loop => loop.map(p => p[0]))], eps)
  const edges = loops.flatMap(loop => loop.map((a, i) => ({ a, b: loop[(i + 1) % loop.length] })))
  const cells: NurbsUV[][] = []
  for (let xIndex = 0; xIndex + 1 < xs.length; xIndex++) {
    const left = xs[xIndex], right = xs[xIndex + 1], mid = (left + right) / 2
    const crossing = edges.filter(({ a, b }) => mid > Math.min(a[0], b[0]) && mid < Math.max(a[0], b[0]))
    const yAt = (edge: typeof edges[number], x: number) => edge.a[1] + (edge.b[1] - edge.a[1]) * (x - edge.a[0]) / (edge.b[0] - edge.a[0])
    crossing.sort((a, b) => yAt(a, mid) - yAt(b, mid))
    if (crossing.length % 2) fail('Trim decomposition has an unmatched crossing.')
    for (let e = 0; e < crossing.length; e += 2) {
      const low = crossing[e], high = crossing[e + 1]
      const trapezoid: NurbsUV[] = [[left, yAt(low, left)], [right, yAt(low, right)], [right, yAt(high, right)], [left, yAt(high, left)]]
      for (let yIndex = 0; yIndex + 1 < gridV.length; yIndex++) {
        const clipped = clipHorizontal(clipHorizontal(trapezoid, gridV[yIndex], true, eps), gridV[yIndex + 1], false, eps)
        if (clipped.length) cells.push(clipped)
        if (cells.length * 3 > budget) fail('Trim tessellation exceeds the triangle budget; reduce segment counts or trim complexity.')
      }
    }
  }
  if (!cells.length) fail('Trim produced no surface area.')

  // Split shared cell edges at every incident vertex. This removes T junctions
  // where a concave boundary or a hole starts on a vertical decomposition line.
  const keyU = (value: number) => String(Math.round((value - domain[0]) / eps))
  const keyV = (value: number) => String(Math.round((value - domain[2]) / eps))
  const vertical = new Map<string, number[]>(), horizontal = new Map<string, number[]>()
  for (const p of cells.flat()) {
    const ys = vertical.get(keyU(p[0])) ?? []; ys.push(p[1]); vertical.set(keyU(p[0]), ys)
    const us = horizontal.get(keyV(p[1])) ?? []; us.push(p[0]); horizontal.set(keyV(p[1]), us)
  }
  for (const [k, values] of vertical) vertical.set(k, unique(values, eps))
  for (const [k, values] of horizontal) horizontal.set(k, unique(values, eps))
  const uv: number[] = [], indices: number[] = [], indexByUV = new Map<string, number>()
  const vertex = (p: NurbsUV) => {
    const k = `${keyU(p[0])},${keyV(p[1])}`
    let index = indexByUV.get(k)
    if (index === undefined) { index = uv.length / 2; indexByUV.set(k, index); uv.push(...p) }
    return index
  }
  for (const cell of cells) {
    const boundary: NurbsUV[] = []
    for (let i = 0; i < cell.length; i++) {
      const a = cell[i], b = cell[(i + 1) % cell.length]
      boundary.push(a)
      let extra: NurbsUV[] = []
      if (close(a[0], b[0], eps)) extra = (vertical.get(keyU(a[0])) ?? []).filter(y => y > Math.min(a[1], b[1]) + eps && y < Math.max(a[1], b[1]) - eps).map(y => [a[0], y])
      else if (close(a[1], b[1], eps)) extra = (horizontal.get(keyV(a[1])) ?? []).filter(x => x > Math.min(a[0], b[0]) + eps && x < Math.max(a[0], b[0]) - eps).map(x => [x, a[1]])
      extra.sort((p, q) => (p[0] - q[0]) * (b[0] - a[0]) + (p[1] - q[1]) * (b[1] - a[1]))
      boundary.push(...extra)
    }
    const center: NurbsUV = [cell.reduce((sum, p) => sum + p[0], 0) / cell.length, cell.reduce((sum, p) => sum + p[1], 0) / cell.length]
    const c = vertex(center)
    for (let i = 0; i < boundary.length; i++) {
      const a = boundary[i], b = boundary[(i + 1) % boundary.length]
      if (cross2(center, a, b) <= eps * eps) fail('Trim cell degenerates at the chosen parameter scale.')
      indices.push(c, vertex(a), vertex(b))
      if (indices.length / 3 > budget) fail('Trim tessellation exceeds the triangle budget; reduce segment counts or trim complexity.')
    }
  }
  const positions: number[] = []
  for (let i = 0; i < uv.length; i += 2) positions.push(...evaluate(uv[i], uv[i + 1]).point)
  let deviation = 0
  for (let i = 0; i < indices.length; i += 3) {
    const vertices = indices.slice(i, i + 3)
    const actual = evaluate(vertices.reduce((sum, v) => sum + uv[2 * v], 0) / 3, vertices.reduce((sum, v) => sum + uv[2 * v + 1], 0) / 3).point
    const delta = actual.map((value, axis) => value - vertices.reduce((sum, v) => sum + positions[3 * v + axis], 0) / 3)
    deviation = Math.max(deviation, Math.hypot(...delta))
  }
  const welded = options.trim ? { positions, indices, uv, seams: { u: false, v: false }, poles: 0 } : weldSurfaceBoundary(surface, positions, indices, uv, actualDomain, eps)
  let report = inspectNurbsMesh(welded.positions, welded.indices)
  if (report.closed && report.signedVolumeMm3 < 0) {
    for (let i = 0; i < welded.indices.length; i += 3) [welded.indices[i + 1], welded.indices[i + 2]] = [welded.indices[i + 2], welded.indices[i + 1]]
    report = inspectNurbsMesh(welded.positions, welded.indices)
  }
  const originalUV = welded.uv.map((value, i) => i % 2 === 0 ? actualDomain[0] + value * spanU : actualDomain[2] + value * spanV)
  return { positions: welded.positions, indices: welded.indices, uv: originalUV, report: { ...report, construction: 'sampled_surface', uvArea: (Math.abs(area(loops[0])) - loops.slice(1).reduce((sum, loop) => sum + Math.abs(area(loop)), 0)) * spanU * spanV, sampledDeviationMm: deviation, parameterSeamsWelded: welded.seams, collapsedBoundaryCount: welded.poles } }
}

/** Weld only parameter boundaries whose own spline data establishes closure. */
function weldSurfaceBoundary(surface: NurbsSurface, positions: number[], indices: number[], uv: number[], domain: [number, number, number, number], eps: number) {
  const controls = surface.controlPoints, weights = surface.weights
  const samePoint = (a: number[], b: number[]) => a.every((value, i) => value === b[i])
  const clamped = (knots: number[], degree: number, low: number, high: number) => knots.slice(0, degree + 1).every(value => value === low) && knots.slice(-degree - 1).every(value => value === high)
  const clampedU = clamped(surface.knotsU, surface.degreeU, domain[0], domain[1]), clampedV = clamped(surface.knotsV, surface.degreeV, domain[2], domain[3])
  const firstU = controls[0], lastU = controls.at(-1)!, firstV = controls.map(row => row[0]), lastV = controls.map(row => row.at(-1)!)
  const equalBoundary = (a: number[][], b: number[][], wa: number[], wb: number[]) => {
    const ratio = wb[0] / wa[0]
    return a.every((point, i) => samePoint(point, b[i]) && Math.abs(wb[i] / wa[i] - ratio) <= Math.abs(ratio) * 1e-12)
  }
  const seams = {
    u: !!surface.periodicU || (clampedU && equalBoundary(firstU, lastU, weights[0], weights.at(-1)!)),
    v: !!surface.periodicV || (clampedV && equalBoundary(firstV, lastV, weights.map(row => row[0]), weights.map(row => row.at(-1)!))),
  }
  const collapsed = [clampedU && firstU.every(p => samePoint(p, firstU[0])), clampedU && lastU.every(p => samePoint(p, lastU[0])), clampedV && firstV.every(p => samePoint(p, firstV[0])), clampedV && lastV.every(p => samePoint(p, lastV[0]))]
  if (!seams.u && !seams.v && !collapsed.some(Boolean)) return { positions, indices, uv, seams, poles: 0 }
  const outPositions: number[] = [], outUV: number[] = [], aliases: number[] = [], vertexByKey = new Map<string, number>()
  const coordinateTolerance = positions.reduce((max, value) => Math.max(max, Math.abs(value)), 1) * Number.EPSILON * 128
  for (let i = 0; i < uv.length; i += 2) {
    const u = uv[i], v = uv[i + 1]
    const boundaries = [close(u, 0, eps), close(u, 1, eps), close(v, 0, eps), close(v, 1, eps)]
    const pole = collapsed.findIndex((value, index) => value && boundaries[index])
    const canonicalU = seams.u && boundaries[1] ? 0 : u, canonicalV = seams.v && boundaries[3] ? 0 : v
    const key = pole >= 0 ? `pole:${[firstU, lastU, firstV, lastV][pole][0].join(',')}` : `${Math.round(canonicalU / eps)},${Math.round(canonicalV / eps)}`
    let mapped = vertexByKey.get(key)
    if (mapped === undefined) {
      mapped = outPositions.length / 3; vertexByKey.set(key, mapped)
      outPositions.push(...positions.slice(i / 2 * 3, i / 2 * 3 + 3)); outUV.push(u, v)
    } else if (Math.hypot(...[0, 1, 2].map(axis => positions[i / 2 * 3 + axis] - outPositions[mapped! * 3 + axis])) > coordinateTolerance) fail('Declared parameter seam does not close within floating-point tolerance.')
    aliases.push(mapped)
  }
  const outIndices: number[] = []
  for (let i = 0; i < indices.length; i += 3) {
    const [a, b, c] = indices.slice(i, i + 3).map(index => aliases[index])
    if (a !== b && b !== c && c !== a) outIndices.push(a, b, c)
  }
  return { positions: outPositions, indices: outIndices, uv: outUV, seams, poles: collapsed.filter(Boolean).length }
}

function meshEdges(indices: number[]) {
  const edges = new Map<string, Array<[number, number]>>()
  for (let i = 0; i < indices.length; i += 3) for (let k = 0; k < 3; k++) {
    const a = indices[i + k], b = indices[i + (k + 1) % 3], key = a < b ? `${a}:${b}` : `${b}:${a}`
    const uses = edges.get(key) ?? []; uses.push([a, b]); edges.set(key, uses)
  }
  return edges
}
export function inspectNurbsMesh(positions: number[], indices: number[]): NurbsMesh['report'] {
  if (positions.length % 3 || indices.length % 3 || !positions.every(Number.isFinite) || indices.some(i => !Number.isInteger(i) || i < 0 || i >= positions.length / 3)) fail('Malformed triangle mesh.')
  const reference = positions.slice(0, 3)
  let volume = 0, degenerateTriangles = 0
  for (let i = 0; i < indices.length; i += 3) {
    const a = positions.slice(indices[i] * 3, indices[i] * 3 + 3), b = positions.slice(indices[i + 1] * 3, indices[i + 1] * 3 + 3), c = positions.slice(indices[i + 2] * 3, indices[i + 2] * 3 + 3)
    const ab = b.map((v, k) => v - a[k]), ac = c.map((v, k) => v - a[k])
    const normal = [ab[1] * ac[2] - ab[2] * ac[1], ab[2] * ac[0] - ab[0] * ac[2], ab[0] * ac[1] - ab[1] * ac[0]]
    if (Math.hypot(...normal) <= Number.EPSILON * Math.max(1, Math.hypot(...ab) * Math.hypot(...ac))) degenerateTriangles++
    // A nearby reference avoids catastrophic cancellation for models positioned
    // far from the origin. For a closed oriented mesh the volume is unchanged.
    const ar = a.map((v, k) => v - reference[k]), br = b.map((v, k) => v - reference[k]), cr = c.map((v, k) => v - reference[k])
    volume += (ar[0] * (br[1] * cr[2] - br[2] * cr[1]) + ar[1] * (br[2] * cr[0] - br[0] * cr[2]) + ar[2] * (br[0] * cr[1] - br[1] * cr[0])) / 6
  }
  const edges = [...meshEdges(indices).values()]
  const boundaryEdges = edges.filter(uses => uses.length === 1).length
  const nonManifoldEdges = edges.filter(uses => uses.length > 2).length
  const orientationConflicts = edges.filter(uses => uses.length === 2 && uses[0][0] === uses[1][0]).length
  return { triangleCount: indices.length / 3, vertexCount: positions.length / 3, boundaryEdges, nonManifoldEdges, orientationConflicts, degenerateTriangles, closed: indices.length > 0 && boundaryEdges === 0 && nonManifoldEdges === 0 && orientationConflicts === 0 && degenerateTriangles === 0, signedVolumeMm3: volume, errorBoundCertified: false, selfIntersectionStatus: 'not_checked', construction: 'sampled_surface' }
}

/** Add two sampled skins and exact mesh boundary walls along a fixed vector. */
export function thickenNurbsMesh(mesh: NurbsMesh, vector: number[]): NurbsMesh {
  if (vector.length !== 3 || !vector.every(Number.isFinite) || Math.hypot(...vector) === 0) fail('Thickening vector must be finite and nonzero.')
  const source = inspectNurbsMesh(mesh.positions, mesh.indices)
  if (source.nonManifoldEdges || source.orientationConflicts || source.degenerateTriangles || !source.boundaryEdges) fail('Thickening requires a consistently oriented open surface mesh with a boundary.')
  const count = mesh.positions.length / 3
  const positions = [...mesh.positions, ...mesh.positions.map((v, i) => v + vector[i % 3])]
  const indices: number[] = []
  for (let i = 0; i < mesh.indices.length; i += 3) {
    const [a, b, c] = mesh.indices.slice(i, i + 3)
    indices.push(a, c, b, a + count, b + count, c + count)
  }
  for (const uses of meshEdges(mesh.indices).values()) if (uses.length === 1) {
    const [a, b] = uses[0]; indices.push(a, b, b + count, a, b + count, a + count)
  }
  if (indices.length / 3 > MAX_TRIANGLES) fail('Thickened mesh exceeds 20000 triangles; reduce surface segment counts.')
  let report = inspectNurbsMesh(positions, indices)
  if (report.signedVolumeMm3 < 0) {
    for (let i = 0; i < indices.length; i += 3) [indices[i + 1], indices[i + 2]] = [indices[i + 2], indices[i + 1]]
    report = inspectNurbsMesh(positions, indices)
  }
  if (!report.closed || !(report.signedVolumeMm3 > 0)) fail('Thickening produced degenerate or open topology; the vector may be tangent to the surface.')
  return { positions, indices, report: { ...report, construction: 'fixed_vector_thickening', sampledDeviationMm: mesh.report.sampledDeviationMm } }
}

/** ASCII STL contains only the sampled mesh, never an exact NURBS representation. */
export function exportNurbsStl(mesh: NurbsMesh): string {
  const report = inspectNurbsMesh(mesh.positions, mesh.indices)
  if (!report.closed || report.signedVolumeMm3 <= 0) fail('STL export requires a closed, consistently oriented mesh with positive volume.')
  const output = ['solid modelgraph_nurbs_sampled']
  for (let i = 0; i < mesh.indices.length; i += 3) {
    const [a, b, c] = mesh.indices.slice(i, i + 3).map(index => mesh.positions.slice(index * 3, index * 3 + 3))
    const ab = b.map((v, k) => v - a[k]), ac = c.map((v, k) => v - a[k])
    const normal = [ab[1] * ac[2] - ab[2] * ac[1], ab[2] * ac[0] - ab[0] * ac[2], ab[0] * ac[1] - ab[1] * ac[0]]
    const length = Math.hypot(...normal)
    output.push(`  facet normal ${normal.map(v => v / length).join(' ')}`, '    outer loop', ...[a, b, c].map(p => `      vertex ${p.join(' ')}`), '    endloop', '  endfacet')
  }
  output.push('endsolid modelgraph_nurbs_sampled')
  return output.join('\n') + '\n'
}
