import { evaluateRushGraphGeometry } from '../services/rushGraphChecks'
import { compileRushFrontend, RUSH_FRONTEND_GUIDE } from '../services/rushFrontend'
import { RUSH_GRAPH_INSTRUCTIONS } from './rushGraphInstructions'
import { registerRushGraphSvgTools } from './rushGraphSvgTools'
import { registerRushGraphGenerate } from './rushGraphGenerate'
import { MECHANICAL_GENERATOR_EXAMPLES } from '../services/mechanicalGeneratorContract'
import { registerRushGraphModify } from './rushGraphModify'
import { registerRushGraphMeshExport } from './rushGraphMeshExport'
import { registerMeshConvertTool } from './meshConvertTool'
import { registerRushGraphNurbsTools } from './rushGraphNurbsTools'
import { registerRushGraphInterference } from './rushGraphInterference'
import { registerRushGraphReport } from './rushGraphReport'
import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import { compileRushGraph, rushGraphSchema, setRushGraphParameters, RushGraphError, RUSH_GRAPH_GUIDE, RUSH_GRAPH_EXAMPLE, RUSH_GRAPH_FUNCTIONAL_GUIDE, RUSH_GRAPH_FUNCTIONAL_EXAMPLE, RUSH_GRAPH_UNITS_GUIDE, RUSH_GRAPH_UNITS_EXAMPLE, RUSH_GRAPH_SKETCH_GUIDE, RUSH_GRAPH_SKETCH_EXAMPLE, RUSH_GRAPH_ASSEMBLY_EXAMPLE, RUSH_GRAPH_LOFT_EXAMPLE } from '../services/rushGraph'
import type { McpGeometryService } from './geometryService'
import { createStaticToolRegistration } from './staticToolSchemas'

const staticToolRegistration = createStaticToolRegistration()

export function registerRushGraphTools(server: McpServer, geometry: McpGeometryService) {
  server = staticToolRegistration(server)
  registerRushGraphSvgTools(server, geometry)
  registerRushGraphGenerate(server, geometry)
  registerRushGraphModify(server, geometry)
  registerRushGraphMeshExport(server, geometry)
  registerMeshConvertTool(server)
  registerRushGraphNurbsTools(server)
  registerRushGraphReport(server, geometry)
  registerRushGraphInterference(server, geometry)
  const result = (data: Record<string, unknown>, isError = false) => ({ isError, content: [{ type: 'text' as const, text: JSON.stringify(data) }], structuredContent: data })
  const failure = (error: unknown) => error instanceof RushGraphError
    ? result({ error: { code: error.code, path: error.path, message: error.message, ...(error.details === undefined ? {} : { details: error.details }) } }, true)
    : result({ error: { code: 'geometry_check_failed', path: '/', message: 'Geometry execution failed; check generated source with openscad_check for engine diagnostics.' } }, true)
  const annotations = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false }
  const languageDocument = () => ({ text_guide: RUSH_FRONTEND_GUIDE, language: 'rush/ir-1', mechanical_examples: MECHANICAL_GENERATOR_EXAMPLES, guide: RUSH_GRAPH_GUIDE + '\n\n' + RUSH_GRAPH_FUNCTIONAL_GUIDE + '\n\n' + RUSH_GRAPH_UNITS_GUIDE + '\n\n' + RUSH_GRAPH_SKETCH_GUIDE, sketch_example: RUSH_GRAPH_SKETCH_EXAMPLE, assembly_example: RUSH_GRAPH_ASSEMBLY_EXAMPLE, loft_example: RUSH_GRAPH_LOFT_EXAMPLE, units_example: RUSH_GRAPH_UNITS_EXAMPLE, functional_example: RUSH_GRAPH_FUNCTIONAL_EXAMPLE, schema: z.toJSONSchema(rushGraphSchema), example: RUSH_GRAPH_EXAMPLE })
  server.registerTool('rush_language', { description: 'Read the complete RushGraph language, JSON Schema, examples and workflow before modeling.', inputSchema: z.object({}).strict(), annotations }, async () => result({ ...languageDocument(), instructions: RUSH_GRAPH_INSTRUCTIONS }))
  server.registerTool('rush_frontend_compile', {
    description: 'Compile compact Rush/1 to canonical RushGraph JSON and SCAD. Read rush_language text_guide. Route by execution_target: own-nurbs documents go to rush_nurbs_build/evaluate/export; legacy documents go to rush_check/report/export. Compile alone does not build geometry.',
    inputSchema: z.object({ source: z.string().max(262144) }).strict(), annotations,
  }, async ({ source }) => {
    try { return result(compileRushFrontend(source)) }
    catch (error) { if(error instanceof RushGraphError) return failure(error); return result({ error: { code: 'text_compile_failed', message: error instanceof Error ? error.message : 'Text compilation failed' } }, true) }
  })
  server.registerResource('rush-language', 'openscad://language/rush-1', {
    title: 'Rush IR/1 language for AI modeling', mimeType: 'application/json',
    description: 'Complete model prompt, JSON Schema, example and execution limitations.',
  }, async uri => ({ contents: [{ uri: uri.href, mimeType: 'application/json', text: JSON.stringify({ ...languageDocument(), instructions: RUSH_GRAPH_INSTRUCTIONS }) }] }))
  server.registerTool('rush_compile', {
    title: 'Compile RushGraph document', description: 'Preferred structured frontend for new MCP models. Validate Rush IR/1 JSON and return normalized document, revision hash, generated SCAD and node-to-line map. Does not build geometry or save data. Read openscad://language/rush-1 first.',
    inputSchema: z.object({ document: rushGraphSchema }).strict(), annotations,
  }, async input => {
    try { return result(compileRushGraph(input.document)) } catch (error) { return failure(error) }
  })
  server.registerTool('rush_check', {
    title: 'Check RushGraph geometry', description: 'Compile and actually evaluate a Rush IR/1 document through the bounded Manifold execution service. No persistence. Reports the real execution target separately from the source language.',
    inputSchema: z.object({ document: rushGraphSchema }).strict(), annotations,
  }, async (input, context) => {
    try {
      const compiled = compileRushGraph(input.document)
      const built = await geometry.compile(compiled.source, 'full', context.mcpReq.signal)
      const checks = await evaluateRushGraphGeometry(compiled.geometry_assertions, built.meshes, async source => (await geometry.compile(source, 'full', context.mcpReq.signal)).meshes)
      return result({ ...compiled, analysis: built.analysis, checks }, checks.some(c => c.status !== 'passed'))
    } catch (error) { return failure(error) }
  })
  server.registerTool('rush_set_parameters', {
    title: 'Change RushGraph parameters', description: 'Atomic validated parameter replacement guarded by the normalized document hash. Returns the new complete document; no text splicing or persistence.',
    inputSchema: z.object({ document: rushGraphSchema, expected_document_sha256: z.string().regex(/^[a-f0-9]{64}$/), updates: z.array(z.object({ id: z.string().max(32), value: z.number().finite().min(-1_000_000).max(1_000_000) }).strict()).min(1).max(64) }).strict(), annotations,
  }, async input => {
    try { return result(setRushGraphParameters(input.document, input.expected_document_sha256, input.updates)) } catch (error) { return failure(error) }
  })
}
