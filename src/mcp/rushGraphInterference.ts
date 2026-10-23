import { z } from 'zod/v4'
import type { McpServer } from '@modelcontextprotocol/server'
import type { McpGeometryService, GeometryAnalysis } from './geometryService'
import { compileRushGraph, rushGraphSchema } from '../services/rushGraph'
export async function inspectRushGraphInterference(document: unknown, geometry: McpGeometryService, signal?: AbortSignal) {
  const compiled = compileRushGraph(document)
  const leaves = compiled.assembly_components.filter(c => !c.is_assembly)
  if (!leaves.length) throw new Error('An assembly is required.')
  if (leaves.length > 32) throw new Error('Interference checking supports at most 32 leaf components per call.')
  const timeout = AbortSignal.timeout(20000)
  const boundedSignal = signal ? AbortSignal.any([signal, timeout]) : timeout
  const deadline = Date.now() + 20000
  const check = () => { if (boundedSignal.aborted || Date.now() > deadline) throw new Error('Interference budget exhausted.') }
  const bounds = new Map<string, GeometryAnalysis['bounds']>()
  for (const leaf of leaves) { check(); bounds.set(leaf.instance_path, (await geometry.analyze(leaf.source!, 'full', boundedSignal)).bounds) }
  const pairs: Array<{ a: string; b: string; status: 'overlap' | 'no_volume_overlap' | 'unknown'; intersection_volume_mm3: number | null; method: string }> = []
  for (let i=0;i<leaves.length;i++) for (let j=i+1;j<leaves.length;j++) {
    const a=leaves[i]!,b=leaves[j]!,ab=bounds.get(a.instance_path),bb=bounds.get(b.instance_path)
    if (!ab || !bb) { pairs.push({a:a.instance_path,b:b.instance_path,status:'unknown',intersection_volume_mm3:null,method:'bounds_unavailable'});continue }
    if (ab.min.some((v,k)=>v >= bb.max[k]! || ab.max[k]! <= bb.min[k]!)) {
      pairs.push({a:a.instance_path,b:b.instance_path,status:'no_volume_overlap',intersection_volume_mm3:0,method:'disjoint_or_touching_bounds'});continue
    }
    try {
      check()
      const result=await geometry.analyze(`intersection(){${a.source}
${b.source}}`, 'full', boundedSignal)
      const volume=Math.abs(result.volume)
      if (!Number.isFinite(volume)) throw new Error('Invalid intersection volume.')
      pairs.push({a:a.instance_path,b:b.instance_path,status:volume>1e-6?'overlap':'no_volume_overlap',intersection_volume_mm3:volume,method:'manifold_intersection'})
    } catch { pairs.push({a:a.instance_path,b:b.instance_path,status:'unknown',intersection_volume_mm3:null,method:'intersection_unavailable'}) }
  }
  return {document_sha256:compiled.document_sha256,scope:'current joint positions only; contact and motion sweep not checked',volume_tolerance_mm3:1e-6,pairs,status:pairs.some(p=>p.status==='overlap')?'overlap':pairs.some(p=>p.status==='unknown')?'unknown':'no_volume_overlap' as const}
}

export interface InterferenceReportPair {
  a: string
  b: string
  status: 'overlap' | 'no_volume_overlap' | 'unknown'
  intersection_volume_mm3: number | null
  method: string
  min_clearance_mm?: number | null
}

export interface InterferenceReport {
  document_sha256: string
  scope: string
  volume_tolerance_mm3: number
  pairs: readonly InterferenceReportPair[]
  status: 'overlap' | 'no_volume_overlap' | 'unknown' | string
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
    lines[idx++] = `| \`${pair.a}\` | \`${pair.b}\` | **${pair.status}** | ${volText} | ${clearanceText} | \`${pair.method}\` |`
  }
  lines.length = idx

  return lines.join('\n')
}

export function registerRushGraphInterference(server:McpServer, geometry:McpGeometryService) {
  server.registerTool('rush_interference',{description:'Check volume overlap between up to 32 leaf assembly components at current joint positions using bounds and actual Manifold intersections. Does not check contact, clearance or the swept path of motion.',inputSchema:z.object({document:rushGraphSchema}),annotations:{readOnlyHint:true,openWorldHint:false}},async(input,context)=>{
    try {const report=await inspectRushGraphInterference(input.document,geometry,context.mcpReq.signal);return {structuredContent:report,content:[{type:'text' as const,text:JSON.stringify(report)}]}}
    catch {return {isError:true,content:[{type:'text' as const,text:'Interference check unavailable. Use an assembly with at most 32 leaf components, validate it with rush_check and retry.'}]}}
  })
}
