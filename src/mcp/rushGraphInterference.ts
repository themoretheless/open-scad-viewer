import { z } from 'zod/v4'
import type { McpServer } from '@modelcontextprotocol/server'
import type { McpGeometryService, GeometryAnalysis } from './geometryService'
import { compileRushGraph, rushGraphSchema } from '../language/rushGraph'
import { rushInterferenceOutputSchema } from './rushOutputSchemas'
import { diagnoseManifoldInKernel } from '../kernel/geometry/manifoldAnalysis'
import type { MeshData } from '../core/mesh'

export const CONTACT_TOLERANCE_MM = 1e-6
export const DEFAULT_CLEARANCE_THRESHOLD_MM = 1

export interface PairClearance {
  status: 'contact' | 'clear' | 'beyond_threshold' | 'unknown'
  /** Closest triangle distance in world space; null beyond the threshold or when unknown. */
  min_distance_mm: number | null
  method: 'bounds_gap' | 'kernel_triangle_distance' | 'mesh_unavailable'
}

export interface InterferenceReportPair {
  a: string
  b: string
  status: 'overlap' | 'no_volume_overlap' | 'unknown'
  intersection_volume_mm3: number | null
  method: string
  clearance?: PairClearance
  min_clearance_mm?: number | null
}

export interface InterferenceReport {
  document_sha256: string
  scope: string
  clearance_threshold_mm?: number
  contact_tolerance_mm?: number
  volume_tolerance_mm3: number
  pairs: readonly InterferenceReportPair[]
  status: 'overlap' | 'no_volume_overlap' | 'unknown' | string
}

/** World-space packed xyz positions and indices of several display meshes. */
function packMeshes(meshes: readonly MeshData[]) {
  const positions: number[] = []
  const indices: number[] = []
  for (const mesh of meshes) {
    const base = positions.length / 3
    const t = mesh.transform
    for (let v = 0; v < mesh.vertices.length; v += 6) {
      const x = mesh.vertices[v]!
      const y = mesh.vertices[v + 1]!
      const z = mesh.vertices[v + 2]!
      positions.push(
        t[0]! * x + t[1]! * y + t[2]! * z + t[3]!,
        t[4]! * x + t[5]! * y + t[6]! * z + t[7]!,
        t[8]! * x + t[9]! * y + t[10]! * z + t[11]!,
      )
    }
    for (const i of mesh.indices) indices.push(base + i)
  }
  return { positions, indices }
}

/** Minimum distance between two parts below a threshold, using the kernel's shell-clearance pass. */
export function measurePairClearance(
  a: readonly MeshData[],
  b: readonly MeshData[],
  thresholdMm: number,
): PairClearance {
  const pa = packMeshes(a)
  const pb = packMeshes(b)
  if (!pa.indices.length || !pb.indices.length) {
    return { status: 'unknown', min_distance_mm: null, method: 'mesh_unavailable' }
  }
  const offset = pa.positions.length / 3
  const trianglesA = pa.indices.length / 3
  const report = diagnoseManifoldInKernel(
    new Float64Array([...pa.positions, ...pb.positions]),
    new Uint32Array([...pa.indices, ...pb.indices.map(i => i + offset)]),
    { clearanceThreshold: thresholdMm },
  )
  let best = Infinity
  for (const pair of report.clearancePairs) {
    if (pair.triangleA < trianglesA !== pair.triangleB < trianglesA) best = Math.min(best, pair.distance)
  }
  if (!Number.isFinite(best)) {
    return { status: 'beyond_threshold', min_distance_mm: null, method: 'kernel_triangle_distance' }
  }
  return {
    status: best <= CONTACT_TOLERANCE_MM ? 'contact' : 'clear',
    min_distance_mm: best,
    method: 'kernel_triangle_distance',
  }
}

const boundsGap = (a: NonNullable<GeometryAnalysis['bounds']>, b: NonNullable<GeometryAnalysis['bounds']>) =>
  Math.hypot(...[0, 1, 2].map(k => Math.max(0, a.min[k]! - b.max[k]!, b.min[k]! - a.max[k]!)))

async function checkPairOverlap(
  a: { instance_path: string; source?: string },
  b: { instance_path: string; source?: string },
  bounds: ReadonlyMap<string, GeometryAnalysis['bounds']>,
  geometry: McpGeometryService,
  boundedSignal: AbortSignal,
  checkBudget: () => void,
): Promise<InterferenceReportPair> {
  const ab = bounds.get(a.instance_path)
  const bb = bounds.get(b.instance_path)
  if (!ab || !bb) {
    return {
      a: a.instance_path,
      b: b.instance_path,
      status: 'unknown',
      intersection_volume_mm3: null,
      method: 'bounds_unavailable',
    }
  }
  if (ab.min.some((v, k) => v >= bb.max[k]! || ab.max[k]! <= bb.min[k]!)) {
    return {
      a: a.instance_path,
      b: b.instance_path,
      status: 'no_volume_overlap',
      intersection_volume_mm3: 0,
      method: 'disjoint_or_touching_bounds',
    }
  }
  try {
    checkBudget()
    const result = await geometry.analyze(
      `intersection(){${a.source}
${b.source}}`,
      'full',
      boundedSignal,
    )
    const volume = Math.abs(result.volume)
    if (!Number.isFinite(volume)) throw new Error('Invalid intersection volume.')
    return {
      a: a.instance_path,
      b: b.instance_path,
      status: volume > 1e-6 ? 'overlap' : 'no_volume_overlap',
      intersection_volume_mm3: volume,
      method: 'manifold_intersection',
    }
  } catch {
    return {
      a: a.instance_path,
      b: b.instance_path,
      status: 'unknown',
      intersection_volume_mm3: null,
      method: 'intersection_unavailable',
    }
  }
}

export async function inspectRushGraphInterference(
  document: unknown,
  geometry: McpGeometryService,
  signal?: AbortSignal,
  options: { clearanceThresholdMm?: number } = {},
) {
  const threshold = options.clearanceThresholdMm ?? DEFAULT_CLEARANCE_THRESHOLD_MM
  if (!(threshold > 0 && threshold <= 100)) throw new Error('Clearance threshold must be within (0, 100] mm.')
  const compiled = compileRushGraph(document)
  const leaves = compiled.assembly_components.filter(c => !c.is_assembly)
  if (!leaves.length) throw new Error('An assembly is required.')
  if (leaves.length > 32) throw new Error('Interference checking supports at most 32 leaf components per call.')
  const timeout = AbortSignal.timeout(20000)
  const boundedSignal = signal ? AbortSignal.any([signal, timeout]) : timeout
  const deadline = Date.now() + 20000
  const check = () => {
    if (boundedSignal.aborted || Date.now() > deadline) throw new Error('Interference budget exhausted.')
  }
  const bounds = new Map<string, GeometryAnalysis['bounds']>()
  for (const leaf of leaves) {
    check()
    bounds.set(leaf.instance_path, (await geometry.analyze(leaf.source!, 'full', boundedSignal)).bounds)
  }
  const pairs: InterferenceReportPair[] = []
  const meshes = new Map<string, Promise<MeshData[] | null>>()
  const leafMeshes = (path: string, source: string) => {
    let pending = meshes.get(path)
    if (!pending) {
      pending = geometry.compile(source, 'full', boundedSignal).then(
        r => r.meshes,
        () => null,
      )
      meshes.set(path, pending)
    }
    return pending
  }
  for (let i = 0; i < leaves.length; i++) {
    for (let j = i + 1; j < leaves.length; j++) {
      pairs.push(await checkPairOverlap(leaves[i]!, leaves[j]!, bounds, geometry, boundedSignal, check))
    }
  }
  for (const pair of pairs) {
    if (pair.status !== 'no_volume_overlap') continue
    const ab = bounds.get(pair.a)!
    const bb = bounds.get(pair.b)!
    if (boundsGap(ab, bb) > threshold) {
      pair.clearance = { status: 'beyond_threshold', min_distance_mm: null, method: 'bounds_gap' }
      continue
    }
    try {
      check()
      const a = leaves.find(l => l.instance_path === pair.a)!
      const b = leaves.find(l => l.instance_path === pair.b)!
      const [ma, mb] = await Promise.all([
        leafMeshes(a.instance_path, a.source!),
        leafMeshes(b.instance_path, b.source!),
      ])
      pair.clearance =
        ma && mb
          ? measurePairClearance(ma, mb, threshold)
          : { status: 'unknown', min_distance_mm: null, method: 'mesh_unavailable' }
    } catch {
      pair.clearance = { status: 'unknown', min_distance_mm: null, method: 'mesh_unavailable' }
    }
    pair.min_clearance_mm = pair.clearance.min_distance_mm
  }
  const baseReport = {
    document_sha256: compiled.document_sha256,
    scope:
      'current joint positions only; clearance and contact are measured on display meshes up to clearance_threshold_mm; motion sweep not checked',
    clearance_threshold_mm: threshold,
    contact_tolerance_mm: CONTACT_TOLERANCE_MM,
    volume_tolerance_mm3: 1e-6,
    pairs,
    status: pairs.some(p => p.status === 'overlap')
      ? 'overlap'
      : pairs.some(p => p.status === 'unknown')
        ? 'unknown'
        : ('no_volume_overlap' as const),
  }
  return {
    ...baseReport,
    markdown_report: formatInterferenceMarkdownReport(baseReport),
  }
}

/**
 * Renders a Markdown summary table of colliding/interfering assembly parts,
 * intersection volumes, and minimum clearances for LLM agents.
 */
export function formatInterferenceMarkdownReport(report: InterferenceReport): string {
  let overlapCount = 0
  let unknownCount = 0
  let clearCount = 0
  for (let i = 0; i < report.pairs.length; i++) {
    const status = report.pairs[i]!.status
    if (status === 'overlap') overlapCount++
    else if (status === 'unknown') unknownCount++
    else if (status === 'no_volume_overlap') clearCount++
  }

  const lines = new Array<string>(11 + report.pairs.length)
  lines[0] = '# Assembly Interference Report'
  lines[1] = ''
  lines[2] = `- **Overall Status**: \`${report.status}\``
  lines[3] = `- **Document SHA-256**: \`${report.document_sha256}\``
  lines[4] = `- **Scope**: ${report.scope}`
  lines[5] = `- **Volume Tolerance**: ${report.volume_tolerance_mm3} mm³`
  lines[6] = `- **Summary**: ${overlapCount} colliding, ${clearCount} clear, ${unknownCount} unknown (${report.pairs.length} total pairs)`
  lines[7] = ''
  lines[8] = '| Part A | Part B | Status | Intersection Volume (mm³) | Min Clearance (mm) | Method |'
  lines[9] = '| :--- | :--- | :--- | :---: | :---: | :--- |'

  let idx = 10
  for (let i = 0; i < report.pairs.length; i++) {
    const pair = report.pairs[i]!
    const volText =
      pair.intersection_volume_mm3 === null || pair.intersection_volume_mm3 === undefined
        ? 'N/A'
        : pair.intersection_volume_mm3.toFixed(4)
    const clearanceText =
      pair.min_clearance_mm !== undefined && pair.min_clearance_mm !== null
        ? pair.min_clearance_mm.toFixed(3)
        : pair.status === 'overlap'
          ? '0.000 (collision)'
          : '—'
    lines[idx++] =
      `| \`${pair.a}\` | \`${pair.b}\` | **${pair.status}** | ${volText} | ${clearanceText} | \`${pair.method}\` |`
  }
  lines.length = idx

  return lines.join('\n')
}

export function registerRushGraphInterference(server: McpServer, geometry: McpGeometryService) {
  server.registerTool(
    'rush_interference',
    {
      description:
        'Check volume overlap between up to 32 leaf assembly components at current joint positions using bounds and actual Manifold intersections, then measure contact and minimum clearance up to clearance_threshold_mm (default 1 mm) on the display meshes. Does not check the swept path of motion.',
      inputSchema: z.object({
        document: rushGraphSchema,
        clearance_threshold_mm: z.number().gt(0).max(100).optional(),
      }),
      outputSchema: rushInterferenceOutputSchema,
      annotations: { readOnlyHint: true, openWorldHint: false },
    },
    async (input, context) => {
      try {
        const report = await inspectRushGraphInterference(
          input.document,
          geometry,
          context.mcpReq.signal,
          input.clearance_threshold_mm === undefined ? {} : { clearanceThresholdMm: input.clearance_threshold_mm },
        )
        return { structuredContent: report, content: [{ type: 'text' as const, text: JSON.stringify(report) }] }
      } catch {
        return {
          isError: true,
          structuredContent: {
            error: {
              code: 'interference_unavailable',
              message:
                'Interference check unavailable. Use an assembly with at most 32 leaf components, validate it with rush_check and retry.',
            },
          },
          content: [
            {
              type: 'text' as const,
              text: 'Interference check unavailable. Use an assembly with at most 32 leaf components, validate it with rush_check and retry.',
            },
          ],
        }
      }
    },
  )
}
