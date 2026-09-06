import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import type { McpGeometryService } from './geometryService'
import { compileModelGraph, modelGraphSchema } from '../services/modelGraph'
import { svgContours, contoursSvg, contoursExtrusion, meshSvgContours, SVG_MAX_BYTES } from '../services/svgGeometry'

export function registerModelGraphSvgTools(server: McpServer, geometry: McpGeometryService) {
  const annotations = { readOnlyHint: true, openWorldHint: false }
  const failure = (e: unknown) => ({ isError: true, content: [{ type: 'text' as const, text: e instanceof Error ? e.message : 'SVG conversion failed.' }] })
  server.registerTool('modelgraph_svg_extrude', {
    description: 'Convert SVG filled shapes and strokes (including curved paths and holes) to self-contained SCAD extrusion in millimeters. Returns normalized SVG and source after an actual full geometry check. SVG text/images/masks are unsupported; outline text first. Maximum 256 KiB input and 20000 contour points.',
    inputSchema: z.object({ svg: z.string().max(SVG_MAX_BYTES), height: z.number().positive().max(100000) }).strict(), annotations,
  }, async (input, context) => {
    try {
      const contours = await svgContours(input.svg), source = contoursExtrusion(contours, input.height)
      const analysis = await geometry.analyze(source, 'full', context.mcpReq.signal)
      return { content: [{ type: 'text' as const, text: JSON.stringify({ source, svg: contoursSvg(contours), analysis }) }] }
    } catch (e) { return failure(e) }
  })
  server.registerTool('modelgraph_svg_export', {
    description: 'Create an SVG silhouette of a ModelGraph model projected along X/Y/Z, or flatten one planar mesh face selected by meshIndex and triangleIndex from a full build. Preserves holes and millimeter dimensions. Does not unwrap curved surfaces. Maximum 20000 triangles. SVG can be passed to modelgraph_svg_extrude.',
    inputSchema: z.object({ document: modelGraphSchema, axis: z.enum(['x','y','z']).default('z'), face: z.object({ meshIndex: z.number().int().nonnegative(), triangleIndex: z.number().int().nonnegative() }).strict().optional() }).strict(), annotations,
  }, async (input, context) => {
    try {
      const compiled = compileModelGraph(input.document), built = await geometry.compile(compiled.source, 'full', context.mcpReq.signal)
      const svg = contoursSvg(await meshSvgContours(built.meshes, input))
      return { content: [{ type: 'resource' as const, resource: { uri: `modelgraph://export/${compiled.document_sha256}.svg`, mimeType: 'image/svg+xml', text: svg } }] }
    } catch (e) { return failure(e) }
  })
}
