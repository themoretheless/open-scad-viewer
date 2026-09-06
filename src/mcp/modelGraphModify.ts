import { z } from 'zod/v4'
import type { McpServer } from '@modelcontextprotocol/server'
import type { McpGeometryService } from './geometryService'
import { compileModelGraph, modelGraphSchema } from '../services/modelGraph'
const length = z.number().finite().positive().max(1e6)
export const modificationSchema = z.discriminatedUnion('operation', [
  z.object({ operation:z.literal('resize'), size:z.tuple([length,length,length]), anchor:z.enum(['origin','minimum','center']).default('minimum') }).strict(),
  z.object({ operation:z.literal('split'), axis:z.enum(['x','y','z']), position:z.number().finite().min(-1e6).max(1e6) }).strict(),
])
export async function modifyModelGraph(document: unknown, modification:z.infer<typeof modificationSchema>, geometry:McpGeometryService, signal?:AbortSignal) {
  const compiled = compileModelGraph(document)
  if (compiled.assembly_components.length) throw new Error('Modify individual solids, not assemblies.')
  const measured = await geometry.analyze(compiled.source,'full',signal)
  if (!measured.bounds) throw new Error('Modification requires a nonempty solid.')
  const {min,max} = measured.bounds
  const size = max.map((v,i) => v-min[i])
  if (size.some(v => !Number.isFinite(v) || v <= 0)) throw new Error('Modification requires a nonempty solid with positive bounds.')
  const ids = new Set(compiled.document.nodes.map(n => n.id))
  const id = () => { for (let i=0;;i++) { const candidate=`edit${i}`; if (!ids.has(candidate)) {ids.add(candidate); return candidate} } }
  const build = async (added:unknown[]) => {
    const result = compileModelGraph({...compiled.document,nodes:[...compiled.document.nodes,...added],root:(added.at(-1) as {id:string}).id})
    const analysis = await geometry.analyze(result.source,'full',signal).catch(error => {throw new Error(`Modification build failed: ${result.source}`,{cause:error})})
    return {...result,analysis}
  }
  if (modification.operation === 'resize') {
    const scale = modification.size.map((v,i) => v/size[i])
    const anchor = min.map((v,i) => modification.anchor === 'origin' ? 0 : modification.anchor === 'center' ? (v+max[i])/2 : v)
    const result = await build([
      {id:id(),op:'affine',input:compiled.document.root,rows:scale.map((v,i)=>[...scale.map((_,j)=>i===j?v:0),anchor[i]*(1-v)])},
    ])
    return {operation:'resize',measurement_document_sha256:compiled.document_sha256,results:[result],note:'Scale is measured at the current parameter values. Repeat resize after changing source parameters.'}
  }
  const axis = ['x','y','z'].indexOf(modification.axis)
  if (modification.position <= min[axis] || modification.position >= max[axis]) throw new Error('Split plane must lie strictly inside the measured bounds.')
  const margin = Math.max(...size)*0.01+0.001
  const results = []
  for (const side of ['negative','positive']) {
    const low=min.map(v=>v-margin), high=max.map(v=>v+margin)
    if (side === 'negative') high[axis]=modification.position
    else low[axis]=modification.position
    const a=id(),b=id(),c=id()
    const result = await build([
      {id:a,op:'box',size:high.map((v,i)=>v-low[i])},
      {id:b,op:'translate',input:a,vector:low},
      {id:c,op:'intersection',inputs:[compiled.document.root,b]},
    ])
    results.push({side,...result})
  }
  return {operation:'split',measurement_document_sha256:compiled.document_sha256,results,note:'Two independent documents, with no kerf. Cutting boxes use current measured bounds; repeat split after changing source parameters.'}
}
export function registerModelGraphModify(server:McpServer,geometry:McpGeometryService) {
  server.registerTool('modelgraph_modify',{
    description:'Measure and resize a solid to XYZ dimensions, or split it at an axis-aligned plane into two complete documents. Actually builds and checks each result. Transforms/cutters are resolved at current parameter values; rerun after parameter edits. No persistence. Assemblies are excluded.',
    inputSchema:z.object({document:modelGraphSchema,modification:modificationSchema}).strict(),
    annotations:{readOnlyHint:true,destructiveHint:false,idempotentHint:true,openWorldHint:false},
  },async(input,context)=>{
    try {
      const result = await modifyModelGraph(input.document,input.modification,geometry,context.mcpReq.signal)
      return {content:[{type:'text' as const,text:JSON.stringify(result)}],structuredContent:result}
    } catch(error) {return {isError:true,content:[{type:'text' as const,text:error instanceof Error ? error.message : 'Modification failed.'}]}}
  })
}
