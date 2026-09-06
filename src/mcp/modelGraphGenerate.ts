import { z } from 'zod/v4'
import type { McpServer } from '@modelcontextprotocol/server'
import type { McpGeometryService } from './geometryService'
import { compileModelGraph, ModelGraphError } from '../services/modelGraph'
import { mechanicalGeneratorSchema, createMechanicalDocument, MECHANICAL_GENERATOR_EXAMPLES, MECHANICAL_GENERATOR_GUIDE } from '../services/mechanicalGeneratorContract'
import { renderModelGraphPreviews } from './modelGraphPreview'
export function registerModelGraphGenerate(server:McpServer,geometry:McpGeometryService) {
  server.registerResource('modelgraph-mechanical-generators','openscad://language/modelgraph-mechanical',{
    description:'Gear, planetary gearset and helical thread generator schema, examples and limitations.',mimeType:'application/json',
  },async uri=>({contents:[{uri:uri.href,mimeType:'application/json',text:JSON.stringify({guide:MECHANICAL_GENERATOR_GUIDE,schema:z.toJSONSchema(mechanicalGeneratorSchema),examples:MECHANICAL_GENERATOR_EXAMPLES})}]}))
  server.registerTool('modelgraph_generate',{
    description:'Generate an involute spur gear/internal ring, compatible planetary gearset, or helical external/internal thread. Accept numeric dimensions; return a parameterized ModelGraph document, design report, actual geometry analysis and preview images. Planetary set has no physical carrier or shafts. No persistence or manufacturing certification. Read openscad://language/modelgraph-mechanical for examples.',
    inputSchema:mechanicalGeneratorSchema,
    annotations:{readOnlyHint:true,destructiveHint:false,idempotentHint:true,openWorldHint:false},
  },async(input,context)=>{
    try {
      const compiled = compileModelGraph(createMechanicalDocument(input))
      const built = await geometry.compile(compiled.source,'full',context.mcpReq.signal)
      let previews:ReturnType<typeof renderModelGraphPreviews> = [], imageError:string|null = null
      try {previews=renderModelGraphPreviews(built.meshes)} catch(error) {imageError=error instanceof Error ? error.message : 'Preview unavailable'}
      const report = {...compiled,analysis:built.analysis,images_status:previews.length?'rendered':'unavailable',image_error:imageError,views:previews.map(p=>p.view),printability:'unknown'}
      return {structuredContent:report,content:[{type:'text' as const,text:JSON.stringify(report)},...previews.map(p=>({type:'image' as const,mimeType:'image/png',data:p.png.toString('base64')}))]}
    } catch(error) {
      const report = {error:{code:error instanceof ModelGraphError?error.code:'generation_failed',path:error instanceof ModelGraphError?error.path:'/',message:error instanceof Error?error.message:'Mechanical generation failed.'}}
      return {isError:true,structuredContent:report,content:[{type:'text' as const,text:JSON.stringify(report)}]}
    }
  })
}
