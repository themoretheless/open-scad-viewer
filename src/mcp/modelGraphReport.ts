import { evaluateModelGraphGeometry } from '../services/modelGraphChecks'
import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import type { McpGeometryService } from './geometryService'
import { compileModelGraph, modelGraphSchema } from '../services/modelGraph'
import { renderModelGraphPreviews } from './modelGraphPreview'
export function registerModelGraphReport(server: McpServer, geometry: McpGeometryService) {
  server.registerTool('modelgraph_report', {
    description: 'Build actual geometry and return measurements, source-to-node provenance and front/top/isometric PNG images. Preview limits may produce images_status=unavailable while measurements remain available. Does not certify printability.',
    inputSchema: z.object({ document: modelGraphSchema }), annotations: { readOnlyHint: true, openWorldHint: false },
  }, async (input, context) => {
    try {
      const compiled = compileModelGraph(input.document)
      const built = await geometry.compile(compiled.source, 'full', context.mcpReq.signal)
      const checks = await evaluateModelGraphGeometry(compiled.geometry_assertions, built.meshes, async source => (await geometry.compile(source, 'full', context.mcpReq.signal)).meshes)
      const references = built.meshes.flatMap((mesh, meshIndex) => mesh.provenance.map(run => {
        const line = run.source ? compiled.source.slice(0, run.source.start).split('\n').length : null
        const match = line === null ? undefined : [...compiled.source_map].reverse().find(item => item.line <= line)
        return { mesh: meshIndex, triangle_start: run.triangleStart, triangle_end: run.triangleEnd, node_id: match?.node_id ?? null, instance_path: match?.instance_path ?? null }
      }))
      let previews: ReturnType<typeof renderModelGraphPreviews> = [], imageError: string | null = null
      try { previews = renderModelGraphPreviews(built.meshes) } catch (error) { imageError = error instanceof Error ? error.message : 'Preview unavailable' }
      const report = { checks, document_sha256: compiled.document_sha256, analysis: built.analysis, constraints: compiled.constraint_report, sketch_solutions: compiled.sketch_solutions, assembly_components: compiled.assembly_components, mechanical_reports: compiled.mechanical_reports, mechanical_parts: compiled.mechanical_parts, references: references.slice(0, 2048), references_truncated: references.length > 2048, reference_scope: 'generated-operation provenance; not persistent face identifiers', images_status: imageError ? 'unavailable' : 'rendered', image_error: imageError, views: previews.map(item => item.view), printability: 'unknown' }
      return { isError: checks.some(c => c.status !== 'passed'), structuredContent: report, content: [{ type: 'text' as const, text: JSON.stringify(report) }, ...previews.map(item => ({ type: 'image' as const, mimeType: 'image/png', data: item.png.toString('base64') }))] }
    } catch { return { isError: true, content: [{ type: 'text' as const, text: 'Report failed. Call modelgraph_compile and modelgraph_check for diagnostics.' }] } }
  })
}
