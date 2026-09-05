import { z } from 'zod/v4'
import type { McpServer } from '@modelcontextprotocol/server'
import type { McpGeometryService, GeometryAnalysis } from './geometryService'
import { compileModelGraph, modelGraphSchema } from '../services/modelGraph'
export async function inspectModelGraphInterference(document: unknown, geometry: McpGeometryService, signal?: AbortSignal) {
  const compiled = compileModelGraph(document)
  const leaves = compiled.assembly_components.filter(c => !c.is_assembly)
  if (!leaves.length) throw new Error('An assembly is required.')
  if (leaves.length > 8) throw new Error('Interference checking supports at most 8 leaf components per call.')
  const timeout = AbortSignal.timeout(20000)
  const boundedSignal = signal ? AbortSignal.any([signal, timeout]) : timeout
  const deadline = Date.now() + 20000
  const check = () => { if (boundedSignal.aborted || Date.now() > deadline) throw new Error('Interference budget exhausted.') }
  const bounds = new Map<string, GeometryAnalysis['bounds']>()
  for (const leaf of leaves) { check(); bounds.set(leaf.instance_path, (await geometry.analyze(leaf.source!, 'full', boundedSignal)).bounds) }
  const pairs: Array<{ a: string; b: string; status: 'overlap' | 'no_volume_overlap' | 'unknown'; intersection_volume_mm3: number | null; method: string }> = []
  for (let i=0;i<leaves.length;i++) for (let j=i+1;j<leaves.length;j++) {
    const a=leaves[i]!,b=leaves[j]!,ab=bounds.get(a.instance_path),bb=bounds.get(b.instance_path)
    if (!ab || !bb || ab.min.some((v,k)=>v >= bb.max[k]! || ab.max[k]! <= bb.min[k]!)) {
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
  return {document_sha256:compiled.document_sha256,scope:'current joint positions only; contact and motion sweep not checked',volume_tolerance_mm3:1e-6,pairs,status:pairs.some(p=>p.status==='overlap')?'overlap':pairs.some(p=>p.status==='unknown')?'unknown':'no_volume_overlap'}
}
export function registerModelGraphInterference(server:McpServer, geometry:McpGeometryService) {
  server.registerTool('modelgraph_interference',{description:'Check volume overlap between up to 8 leaf assembly components at current joint positions using bounds and actual Manifold intersections. Does not check contact, clearance or the swept path of motion.',inputSchema:z.object({document:modelGraphSchema}),annotations:{readOnlyHint:true,openWorldHint:false}},async(input,context)=>{
    try {const report=await inspectModelGraphInterference(input.document,geometry,context.mcpReq.signal);return {structuredContent:report,content:[{type:'text' as const,text:JSON.stringify(report)}]}}
    catch {return {isError:true,content:[{type:'text' as const,text:'Interference check unavailable. Use an assembly with at most 8 leaf components, validate it with modelgraph_check and retry.'}]}}
  })
}
