import { canonicalJson } from '../core/canonicalJson'
import { callGeometryRust } from './geometry/kernel'
import type { NurbsBrep } from './geometry/brep/core'

const op = 'brep_assemble_closed_ss_boolean' as const
const networkOp = 'brep_assemble_closed_ss_network_boolean' as const
type Range = [number, number]
type CoverageDomain = [Range, Range, Range, Range]

export interface ClosedSsBooleanRequest {
  operation: 'union' | 'intersection' | 'difference' | 'xor'
  a: NurbsBrep
  b: NurbsBrep
  toleranceMm: number
  toleranceUv: number
  volumeErrorMm3: number
  volumeAxis?: 0 | 1 | 2
}

interface BooleanTopologyReport {
  branches: number
  bodies: number
  faces: number
  edges: number
  boundaryEdges: 0
  volumeBoundsMm3: Range
}

export interface ClosedSsBooleanReport extends BooleanTopologyReport {
  source: ClosedSsBooleanRequest & { op: typeof op }
  coverageDomain: CoverageDomain
}

export interface ClosedSsNetworkBooleanReport extends BooleanTopologyReport {
  source: ClosedSsBooleanRequest & { op: typeof networkOp }
  facePairs: Array<{ faces: [number, number]; coverageDomain: CoverageDomain; branches: number }>
  topology: {
    vertices: number
    edges: Array<{ vertices: [number, number]; degenerate: boolean; root: boolean }>
    loops: Array<Array<{ edge: number; reversed: boolean }>>
    faces: Array<{ outer: number; holes: number[] }>
    shells: Array<Array<{ face: number; reversed: boolean }>>
    bodies: Array<{ outerShell: number; innerShells: number[] }>
  }
}

function booleanArguments({
  operation, a, b, toleranceMm, toleranceUv, volumeErrorMm3, volumeAxis,
}: ClosedSsBooleanRequest): ClosedSsBooleanRequest {
  const args: ClosedSsBooleanRequest = { operation, a, b, toleranceMm, toleranceUv, volumeErrorMm3 }
  if (volumeAxis !== undefined) args.volumeAxis = volumeAxis
  return args
}

function isCount(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0
}

function validCoverageDomain(value: unknown): value is CoverageDomain {
  return Array.isArray(value) && value.length === 4
    && value.every(range => Array.isArray(range) && range.length === 2
      && range.every(Number.isFinite) && range[0] < range[1])
}

function validBooleanTopology(value: unknown): value is BooleanTopologyReport {
  const report = value as Partial<BooleanTopologyReport> | null
  if (!report || report.boundaryEdges !== 0) return false
  const { volumeBoundsMm3: bounds } = report
  return Array.isArray(bounds) && bounds.length === 2 && bounds.every(Number.isFinite)
    && bounds[0] >= 0 && bounds[0] <= bounds[1]
    && [report.branches, report.bodies, report.faces, report.edges].every(isCount)
    && report.branches !== 0
}

function validNetworkIncidence(
  value: ClosedSsNetworkBooleanReport['topology'] | undefined,
  report: BooleanTopologyReport,
): boolean {
  if (!value || !isCount(value.vertices)
    || !Array.isArray(value.edges) || value.edges.length !== report.edges
    || !Array.isArray(value.faces) || value.faces.length !== report.faces
    || !Array.isArray(value.bodies) || value.bodies.length !== report.bodies
    || !Array.isArray(value.loops) || !Array.isArray(value.shells)) return false
  const index = (id: unknown, length: number) => isCount(id) && id < length
  const shapeValid = value.edges.every(edge => edge && Array.isArray(edge.vertices) && edge.vertices.length === 2
    && edge.vertices.every(id => index(id, value.vertices))
    && typeof edge.degenerate === 'boolean' && typeof edge.root === 'boolean')
    && value.loops.every(loop => Array.isArray(loop) && loop.length > 0
      && loop.every(use => use && index(use.edge, value.edges.length) && typeof use.reversed === 'boolean'))
    && value.faces.every(face => face && index(face.outer, value.loops.length)
      && Array.isArray(face.holes) && face.holes.every(id => index(id, value.loops.length)))
    && value.shells.every(shell => Array.isArray(shell) && shell.length > 0
      && shell.every(use => use && index(use.face, value.faces.length) && typeof use.reversed === 'boolean'))
    && value.bodies.every(body => body && index(body.outerShell, value.shells.length)
      && Array.isArray(body.innerShells) && body.innerShells.every(id => index(id, value.shells.length)))
  return shapeValid && validClosedNetworkShells(value)
}

function validClosedNetworkShells(topology: ClosedSsNetworkBooleanReport['topology']): boolean {
  const usedShells = new Set<number>()
  for (const body of topology.bodies) {
    for (const shell of [body.outerShell, ...body.innerShells]) {
      if (usedShells.has(shell)) return false
      usedShells.add(shell)
    }
  }
  const usedFaces = new Set<number>()
  const usedLoops = new Set<number>()
  const usedEdges = new Set<number>()
  const usedVertices = new Set<number>()
  for (const shell of topology.shells) {
    const incidence = new Map<number, number[]>()
    for (const use of shell) {
      if (usedFaces.has(use.face)) return false
      usedFaces.add(use.face)
      const face = topology.faces[use.face]
      for (const loop of [face.outer, ...face.holes]) {
        if (usedLoops.has(loop)) return false
        usedLoops.add(loop)
        for (const coedge of topology.loops[loop]) {
          const edge = topology.edges[coedge.edge]
          usedEdges.add(coedge.edge)
          edge.vertices.forEach(id => usedVertices.add(id))
          if (edge.degenerate) continue
          const counts = incidence.get(coedge.edge) ?? [0, 0]
          counts[Number(coedge.reversed !== use.reversed)] += 1
          incidence.set(coedge.edge, counts)
        }
      }
    }
    if ([...incidence.values()].some(([forward, reverse]) => forward !== 1 || reverse !== 1)) return false
  }
  return usedShells.size === topology.shells.length && usedFaces.size === topology.faces.length
    && usedLoops.size === topology.loops.length && usedEdges.size === topology.edges.length
    && usedVertices.size === topology.vertices
}

function sameSource(
  source: unknown, request: ClosedSsBooleanRequest, operation: typeof op | typeof networkOp,
): boolean {
  try {
    return canonicalJson(source) === canonicalJson({ ...booleanArguments(request), op: operation })
  } catch {
    return false
  }
}

export function validClosedSsBooleanReport(
  value: unknown,
  request: ClosedSsBooleanRequest,
): value is ClosedSsBooleanReport {
  if (!validBooleanTopology(value)) return false
  const report = value as Partial<ClosedSsBooleanReport>
  return validCoverageDomain(report.coverageDomain) && sameSource(report.source, request, op)
}

export function validClosedSsNetworkBooleanReport(
  value: unknown,
  request: ClosedSsBooleanRequest,
): value is ClosedSsNetworkBooleanReport {
  try {
    return validClosedSsNetworkBooleanResult(value, closedSsNetworkBooleanExpectation(request))
  } catch {
    return false
  }
}

export function closedSsNetworkBooleanExpectation(request: ClosedSsBooleanRequest) {
  return {
    sourceKey: canonicalJson({ ...booleanArguments(request), op: networkOp }),
    faceCounts: [request.a.faces.length, request.b.faces.length] as const,
  }
}

export function validClosedSsNetworkBooleanResult(
  value: unknown,
  expected: ReturnType<typeof closedSsNetworkBooleanExpectation>,
): value is ClosedSsNetworkBooleanReport {
  if (!validBooleanTopology(value)) return false
  const { source, facePairs, branches, topology } = value as Partial<ClosedSsNetworkBooleanReport>
  if (!Array.isArray(facePairs) || facePairs.length === 0) return false
  try {
    if (canonicalJson(source) !== expected.sourceKey) return false
  } catch {
    return false
  }
  if (!validNetworkIncidence(topology, value)) return false
  const seen = new Set<string>()
  let total = 0
  for (const pair of facePairs) {
    if (!pair || !Array.isArray(pair.faces) || pair.faces.length !== 2
      || !pair.faces.every(isCount) || !validCoverageDomain(pair.coverageDomain)
      || !isCount(pair.branches) || pair.branches === 0) return false
    const [first, second] = pair.faces
    const key = `${first}/${second}`
    if (first >= expected.faceCounts[0] || second >= expected.faceCounts[1] || seen.has(key)) return false
    seen.add(key)
    total += pair.branches
  }
  return total === branches
}

export function assembleClosedSsBoolean(request: ClosedSsBooleanRequest): ClosedSsBooleanReport {
  const report = callGeometryRust<unknown>(op, booleanArguments(request))
  if (!validClosedSsBooleanReport(report, request)) {
    throw new Error('Geometry kernel returned an invalid closed SS Boolean report')
  }
  return report
}


export function assembleClosedSsNetworkBoolean(request: ClosedSsBooleanRequest): ClosedSsNetworkBooleanReport {
  const report = callGeometryRust<unknown>(networkOp, booleanArguments(request))
  if (!validClosedSsNetworkBooleanReport(report, request)) {
    throw new Error('Geometry kernel returned an invalid closed SS network Boolean report')
  }
  return report
}
