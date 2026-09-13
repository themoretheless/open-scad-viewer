import { evaluateModelGraphGeometry, requireModelGraphChecks, ModelGraphCheckError } from '../services/modelGraphChecks'
import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import type { McpGeometryService } from './geometryService'
import { compileModelGraph, modelGraphSchema } from '../services/modelGraph'
import { svgPreview, svgProfile, contoursSvg, contoursExtrusion, meshSvgContours, SVG_MAX_BYTES, type SvgOptions } from '../services/svgGeometry'
import { svgProfileModelGraph } from '../services/svgModelGraph'

const svgInput = {
  svg: z.string().max(SVG_MAX_BYTES),
  dpi: z.number().positive().max(100000).default(96),
  tolerance: z.number().min(0.0001).max(10).default(0.02),
  geometryMode: z.enum(['vector', 'silhouette']).default('vector'),
  rasterSize: z.number().int().min(128).max(2048).default(512),
  alphaThreshold: z.number().min(0.01).max(1).default(0.5),
  fonts: z.array(z.string().max(5592408)).max(16).optional().describe('Optional outline TTF/OTF/TTC fonts encoded as base64; 4 MiB per font, 8 MiB total. Noto Sans is bundled. Color SVG/COLR and bitmap glyph tables are unsupported; convert colored glyphs to SVG paths first.'),
}

function options(input: Omit<SvgOptions, 'fonts'> & { fonts?: string[] }): SvgOptions {
  let bytes = 0
  const fonts = input.fonts?.map(value => {
    if (value.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(value)) throw new Error('Invalid base64 SVG font.')
    const font = Uint8Array.from(atob(value), c => c.charCodeAt(0))
    bytes += font.length
    if (bytes > 8 * 1024 * 1024) throw new Error('SVG fonts exceed 8 MiB in total.')
    return font
  })
  return { dpi: input.dpi, tolerance: input.tolerance, geometryMode: input.geometryMode, rasterSize: input.rasterSize, alphaThreshold: input.alphaThreshold, fonts }
}

export function registerModelGraphSvgTools(server: McpServer, geometry: McpGeometryService) {
  const annotations = { readOnlyHint: true, openWorldHint: false }
  const failure = (e: unknown) => ({ isError: true, ...(e instanceof ModelGraphCheckError ? {structuredContent:{checks:e.checks}} : {}), content: [{ type: 'text' as const, text: e instanceof Error ? e.message : 'SVG conversion failed.' }] })
  server.registerTool('modelgraph_svg_preview', {
    description: 'Validate and normalize static SVG while retaining colors, gradients, patterns, clipping, masks, filters and embedded images. Resolves CSS, references and text outlines with bundled Noto Sans or supplied fonts. Returns portable SVG and millimeter dimensions; external resources, scripting and animation are rejected. Maximum 4 MiB input.',
    inputSchema: z.object(svgInput).strict(), annotations,
  }, async input => {
    try {
      const result = await svgPreview(input.svg, options(input))
      return { content: [
        { type: 'text' as const, text: JSON.stringify({ widthMm: result.widthMm, heightMm: result.heightMm, warnings: result.warnings }) },
        { type: 'resource' as const, resource: { uri: 'modelgraph://svg/preview.svg', mimeType: 'image/svg+xml', text: result.svg } },
      ] }
    } catch (e) { return failure(e) }
  })
  server.registerTool('modelgraph_svg_extrude', {
    description: 'Convert SVG to self-contained SCAD extrusion in millimeters, checked with a full geometry build. Vector mode supports text, CSS, use/symbol, nested viewports, clipping, holes and styled/dashed strokes; curves use tolerance in mm. For masks, filters, patterns and embedded images explicitly choose silhouette mode: alpha-threshold pixel approximation at rasterSize resolution. Returns contour SVG, source, warnings and analysis. Optional includeModelGraph returns an editable modelgraph/1 document with svg_height parameter and a separate verified full build; ModelGraph limits are 256 points per ring, 128 nodes and bounded profile work. Default 96 DPI; maximum 4 MiB input and 20000 contour points.',
    inputSchema: z.object({ ...svgInput, height: z.number().positive().max(100000), includeModelGraph: z.boolean().default(false) }).strict(), annotations,
  }, async (input, context) => {
    try {
      const profile = await svgProfile(input.svg, options(input)), source = contoursExtrusion(profile.contours, input.height)
      const modelGraph = input.includeModelGraph ? await svgProfileModelGraph(profile.contours, input.height) : undefined
      const analysis = await geometry.analyze(source, 'full', context.mcpReq.signal)
      let nativeResult = {}
      if (modelGraph) {
        const modelgraphAnalysis = await geometry.analyze(modelGraph.source, 'full', context.mcpReq.signal)
        const sameNumber = (a: number, b: number) => Math.abs(a - b) <= 1e-6 * Math.max(1, Math.abs(a), Math.abs(b))
        if (analysis.volume == null || modelgraphAnalysis.volume == null || !sameNumber(analysis.volume, modelgraphAnalysis.volume)
          || !analysis.bounds || !modelgraphAnalysis.bounds
          || !(['min', 'max'] as const).every(side => analysis.bounds![side].every((value, axis) => sameNumber(value, modelgraphAnalysis.bounds![side][axis]!)))
          || modelgraphAnalysis.topology.boundary !== 0 || modelgraphAnalysis.topology.nonManifold !== 0) {
          throw new Error('SVG ModelGraph conversion did not preserve the checked solid geometry.')
        }
        nativeResult = { document: modelGraph.document, document_sha256: modelGraph.document_sha256, modelgraph_source: modelGraph.source, modelgraph_analysis: modelgraphAnalysis }
      }
      return { content: [{ type: 'text' as const, text: JSON.stringify({ source, svg: contoursSvg(profile.contours), warnings: profile.warnings, analysis, ...nativeResult }) }] }
    } catch (e) { return failure(e) }
  })
  server.registerTool('modelgraph_svg_export', {
    description: 'Create an SVG silhouette of a ModelGraph model projected along X/Y/Z, or flatten one planar mesh face selected by meshIndex and triangleIndex from a full build. Preserves holes and millimeter dimensions. Does not unwrap curved surfaces. Maximum 20000 triangles. SVG can be passed to modelgraph_svg_extrude.',
    inputSchema: z.object({ document: modelGraphSchema, axis: z.enum(['x','y','z']).default('z'), face: z.object({ meshIndex: z.number().int().nonnegative(), triangleIndex: z.number().int().nonnegative() }).strict().optional() }).strict(), annotations,
  }, async (input, context) => {
    try {
      const compiled = compileModelGraph(input.document), built = await geometry.compile(compiled.source, 'full', context.mcpReq.signal)
      requireModelGraphChecks(await evaluateModelGraphGeometry(compiled.geometry_assertions, built.meshes, async source => (await geometry.compile(source, 'full', context.mcpReq.signal)).meshes));
      const svg = contoursSvg(await meshSvgContours(built.meshes, input))
      return { content: [{ type: 'resource' as const, resource: { uri: `modelgraph://export/${compiled.document_sha256}.svg`, mimeType: 'image/svg+xml', text: svg } }] }
    } catch (e) { return failure(e) }
  })
}
