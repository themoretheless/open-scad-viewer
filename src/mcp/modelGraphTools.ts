import { registerModelGraphInterference } from './modelGraphInterference'
import { registerModelGraphReport } from './modelGraphReport'
import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import { compileModelGraph, modelGraphSchema, setModelGraphParameters, ModelGraphError, MODELGRAPH_GUIDE, MODELGRAPH_EXAMPLE, MODELGRAPH_FUNCTIONAL_GUIDE, MODELGRAPH_FUNCTIONAL_EXAMPLE, MODELGRAPH_UNITS_GUIDE, MODELGRAPH_UNITS_EXAMPLE, MODELGRAPH_SKETCH_GUIDE, MODELGRAPH_SKETCH_EXAMPLE, MODELGRAPH_ASSEMBLY_EXAMPLE } from '../services/modelGraph'
import type { McpGeometryService } from './geometryService'

export function registerModelGraphTools(server: McpServer, geometry: McpGeometryService) {
  registerModelGraphReport(server, geometry)
  registerModelGraphInterference(server, geometry)
  const result = (data: Record<string, unknown>, isError = false) => ({ isError, content: [{ type: 'text' as const, text: JSON.stringify(data) }], structuredContent: data })
  const failure = (error: unknown) => error instanceof ModelGraphError
    ? result({ error: { code: error.code, path: error.path, message: error.message, ...(error.details === undefined ? {} : { details: error.details }) } }, true)
    : result({ error: { code: 'geometry_check_failed', path: '/', message: 'Geometry execution failed; check generated source with openscad_check for engine diagnostics.' } }, true)
  const annotations = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false }
  server.registerResource('modelgraph-language', 'openscad://language/modelgraph-1', {
    title: 'ModelGraph/1 language for AI modeling', mimeType: 'application/json',
    description: 'Complete model prompt, JSON Schema, example and execution limitations.',
  }, async uri => ({ contents: [{ uri: uri.href, mimeType: 'application/json', text: JSON.stringify({ language: 'modelgraph/1', guide: MODELGRAPH_GUIDE + '\n\n' + MODELGRAPH_FUNCTIONAL_GUIDE + '\n\n' + MODELGRAPH_UNITS_GUIDE + '\n\n' + MODELGRAPH_SKETCH_GUIDE, sketch_example: MODELGRAPH_SKETCH_EXAMPLE, assembly_example: MODELGRAPH_ASSEMBLY_EXAMPLE, units_example: MODELGRAPH_UNITS_EXAMPLE, functional_example: MODELGRAPH_FUNCTIONAL_EXAMPLE, schema: z.toJSONSchema(modelGraphSchema), example: MODELGRAPH_EXAMPLE }) }] }))
  server.registerTool('modelgraph_compile', {
    title: 'Compile ModelGraph document', description: 'Preferred structured frontend for new MCP models. Validate ModelGraph/1 JSON and return normalized document, revision hash, generated SCAD and node-to-line map. Does not build geometry or save data. Read openscad://language/modelgraph-1 first.',
    inputSchema: z.object({ document: modelGraphSchema }).strict(), annotations,
  }, async input => {
    try { return result(compileModelGraph(input.document)) } catch (error) { return failure(error) }
  })
  server.registerTool('modelgraph_check', {
    title: 'Check ModelGraph geometry', description: 'Compile and actually evaluate a ModelGraph/1 document through the bounded Manifold execution service. No persistence. Reports the real execution target separately from the source language.',
    inputSchema: z.object({ document: modelGraphSchema }).strict(), annotations,
  }, async (input, context) => {
    try {
      const compiled = compileModelGraph(input.document)
      const analysis = await geometry.analyze(compiled.source, 'full', context.mcpReq.signal)
      return result({ ...compiled, analysis })
    } catch (error) { return failure(error) }
  })
  server.registerTool('modelgraph_set_parameters', {
    title: 'Change ModelGraph parameters', description: 'Atomic validated parameter replacement guarded by the normalized document hash. Returns the new complete document; no text splicing or persistence.',
    inputSchema: z.object({ document: modelGraphSchema, expected_document_sha256: z.string().regex(/^[a-f0-9]{64}$/), updates: z.array(z.object({ id: z.string().max(32), value: z.number().finite().min(-1_000_000).max(1_000_000) }).strict()).min(1).max(64) }).strict(), annotations,
  }, async input => {
    try { return result(setModelGraphParameters(input.document, input.expected_document_sha256, input.updates)) } catch (error) { return failure(error) }
  })
}
