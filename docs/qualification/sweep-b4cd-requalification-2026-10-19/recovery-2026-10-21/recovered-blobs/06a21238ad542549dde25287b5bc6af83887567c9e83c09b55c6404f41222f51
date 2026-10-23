import { z } from 'zod/v4'
import type { McpServer } from '@modelcontextprotocol/server'
import type { McpGeometryService } from './geometryService'
import { compileRushGraph, RushGraphError } from '../services/rushGraph'
import { mechanicalGeneratorSchema, createMechanicalDocument, MECHANICAL_GENERATOR_EXAMPLES, MECHANICAL_GENERATOR_GUIDE } from '../services/mechanicalGeneratorContract'
import { RUSH_GRAPH_PREVIEW_TRIANGLE_LIMIT, renderRushGraphPreviews } from './rushGraphPreview'
export function registerRushGraphGenerate(server:McpServer,geometry:McpGeometryService) {
  server.registerResource('rush-mechanical-generators','openscad://language/rush-mechanical',{
    description:'Gear, planetary gearset and helical thread generator schema, examples and limitations.',mimeType:'application/json',
  },async uri=>({contents:[{uri:uri.href,mimeType:'application/json',text:JSON.stringify({guide:MECHANICAL_GENERATOR_GUIDE,schema:z.toJSONSchema(mechanicalGeneratorSchema),examples:MECHANICAL_GENERATOR_EXAMPLES})}]}))
  server.registerTool('rush_generate',{
    description:'Generate an involute spur gear/internal ring, compatible planetary gearset, or helical external/internal thread. Accept numeric dimensions; return a parameterized RushGraph document, design report, actual geometry analysis and preview images. Planetary set has no physical carrier or shafts. No persistence or manufacturing certification. Read openscad://language/rush-mechanical for examples.',
    inputSchema:mechanicalGeneratorSchema,
    annotations:{readOnlyHint:true,destructiveHint:false,idempotentHint:true,openWorldHint:false},
  },async(input,context)=>{
    try {
      const document = createMechanicalDocument(input)
      const compiled = compileRushGraph(document)
      const built = await geometry.compile(compiled.source,'full',context.mcpReq.signal)
      let previews:ReturnType<typeof renderRushGraphPreviews> = [], imageError:string|null = null
      let previewDocumentSha256 = compiled.document_sha256
      try {
        let previewMeshes = built.meshes
        if (built.meshes.reduce((count, mesh) => count + mesh.indices.length / 3, 0) > RUSH_GRAPH_PREVIEW_TRIANGLE_LIMIT) {
          // Display-only tessellation; authoritative source and analysis stay unchanged.
          const preview = compileRushGraph({ ...document, segments: 12 })
          previewDocumentSha256 = preview.document_sha256
          previewMeshes = (await geometry.compile(preview.source, 'preview', context.mcpReq.signal)).meshes
        }
        previews = renderRushGraphPreviews(previewMeshes)
      } catch(error) {imageError=error instanceof Error ? error.message : 'Preview unavailable'}
      context.mcpReq.signal.throwIfAborted()
      const report = {...compiled,analysis:built.analysis,images_status:previews.length?'rendered':'unavailable',image_error:imageError,preview_document_sha256:previewDocumentSha256,views:previews.map(p=>p.view),printability:'unknown'}
      return {structuredContent:report,content:[{type:'text' as const,text:JSON.stringify(report)},...previews.map(p=>({type:'image' as const,mimeType:'image/png',data:p.png.toString('base64')}))]}
    } catch(error) {
      const report = {error:{code:error instanceof RushGraphError?error.code:'generation_failed',path:error instanceof RushGraphError?error.path:'/',message:error instanceof Error?error.message:'Mechanical generation failed.'}}
      return {isError:true,structuredContent:report,content:[{type:'text' as const,text:JSON.stringify(report)}]}
    }
  })
}
