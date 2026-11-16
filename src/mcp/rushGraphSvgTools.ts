import { evaluateRushGraphGeometry, requireRushGraphChecks, RushGraphCheckError } from '../language/rushGraphChecks'
import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import type { McpGeometryService } from './geometryService'
import { compileRushGraph, rushGraphSchema } from '../language/rushGraph'
import {
  svgPreview,
  svgProfile,
  contoursSvg,
  contoursExtrusion,
  meshSvgContours,
  SVG_MAX_BYTES,
  type SvgOptions,
} from '../exchange/svgGeometry'
import { svgProfileRushGraph } from '../exchange/svgRushGraph'
import { rushSvgExportOutputSchema, rushSvgExtrudeOutputSchema, rushSvgPreviewOutputSchema } from './rushOutputSchemas'

const svgInput = {
  svg: z.string().max(SVG_MAX_BYTES),
  dpi: z.number().positive().max(100000).default(96),
  tolerance: z.number().min(0.0001).max(10).default(0.02),
  geometryMode: z.enum(['vector', 'silhouette']).default('vector'),
  rasterSize: z.number().int().min(128).max(2048).default(512),
  alphaThreshold: z.number().min(0.01).max(1).default(0.5),
  fonts: z
    .array(z.string().max(5592408))
    .max(16)
    .optional()
    .describe(
      'Optional outline TTF/OTF/TTC fonts encoded as base64; 4 MiB per font, 8 MiB total. Noto Sans is bundled. Color SVG/COLR and bitmap glyph tables are unsupported; convert colored glyphs to SVG paths first.',
    ),
}

function options(input: Omit<SvgOptions, 'fonts'> & { fonts?: string[] | undefined }): SvgOptions {
  let bytes = 0
  const fonts = input.fonts?.map(value => {
    if (value.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(value)) throw new Error('Invalid base64 SVG font.')
    const font = Uint8Array.from(atob(value), c => c.charCodeAt(0))
    bytes += font.length
    if (bytes > 8 * 1024 * 1024) throw new Error('SVG fonts exceed 8 MiB in total.')
    return font
  })
  return {
    dpi: input.dpi,
    tolerance: input.tolerance,
    geometryMode: input.geometryMode,
    rasterSize: input.rasterSize,
    alphaThreshold: input.alphaThreshold,
    fonts,
  }
}

const svgAnnotations = { readOnlyHint: true, openWorldHint: false }

function svgFailure(e: unknown) {
  const message = e instanceof Error ? e.message : 'SVG conversion failed.'
  return {
    isError: true,
    structuredContent: {
      error: { code: e instanceof RushGraphCheckError ? 'checks_failed' : 'svg_failed', message },
      ...(e instanceof RushGraphCheckError ? { checks: e.checks } : {}),
    },
    content: [{ type: 'text' as const, text: message }],
  }
}

function registerRushSvgExportTool(server: McpServer, geometry: McpGeometryService): void {
  server.registerTool(
    'rush_svg_export',
    {
      description:
        'Create an SVG silhouette of a RushGraph model projected along X/Y/Z, or flatten one planar mesh face selected by meshIndex and triangleIndex from a full build. Preserves holes and millimeter dimensions. Does not unwrap curved surfaces. Maximum 20000 triangles. SVG can be passed to rush_svg_extrude.',
      inputSchema: z
        .object({
          document: rushGraphSchema,
          axis: z.enum(['x', 'y', 'z']).default('z'),
          face: z
            .object({ meshIndex: z.number().int().nonnegative(), triangleIndex: z.number().int().nonnegative() })
            .strict()
            .optional(),
        })
        .strict(),
      outputSchema: rushSvgExportOutputSchema,
      annotations: svgAnnotations,
    },
    async (input, context) => {
      try {
        const compiled = compileRushGraph(input.document),
          built = await geometry.compile(compiled.source, 'full', context.mcpReq.signal)
        requireRushGraphChecks(
          await evaluateRushGraphGeometry(
            compiled.geometry_assertions,
            built.meshes,
            async source => (await geometry.compile(source, 'full', context.mcpReq.signal)).meshes,
          ),
        )
        const svg = contoursSvg(await meshSvgContours(built.meshes, input))
        const structuredContent = {
          document_sha256: compiled.document_sha256,
          axis: input.axis,
          mime_type: 'image/svg+xml' as const,
          byte_length: Buffer.byteLength(svg, 'utf8'),
        }
        return {
          structuredContent,
          content: [
            {
              type: 'resource' as const,
              resource: { uri: `rush://export/${compiled.document_sha256}.svg`, mimeType: 'image/svg+xml', text: svg },
            },
          ],
        }
      } catch (e) {
        return svgFailure(e)
      }
    },
  )
}

export function registerRushGraphSvgTools(server: McpServer, geometry: McpGeometryService) {
  server.registerTool(
    'rush_svg_preview',
    {
      description:
        'Validate and normalize static SVG while retaining colors, gradients, patterns, clipping, masks, filters and embedded images. Resolves CSS, references and text outlines with bundled Noto Sans or supplied fonts. Returns portable SVG and millimeter dimensions; external resources, scripting and animation are rejected. Maximum 4 MiB input.',
      inputSchema: z.object(svgInput).strict(),
      outputSchema: rushSvgPreviewOutputSchema,
      annotations: svgAnnotations,
    },
    async input => {
      try {
        const result = await svgPreview(input.svg, options(input))
        const structuredContent = { widthMm: result.widthMm, heightMm: result.heightMm, warnings: result.warnings }
        return {
          structuredContent,
          content: [
            { type: 'text' as const, text: JSON.stringify(structuredContent) },
            {
              type: 'resource' as const,
              resource: { uri: 'rush://svg/preview.svg', mimeType: 'image/svg+xml', text: result.svg },
            },
          ],
        }
      } catch (e) {
        return svgFailure(e)
      }
    },
  )
  server.registerTool(
    'rush_svg_extrude',
    {
      description:
        'Convert SVG to self-contained SCAD extrusion in millimeters, checked with a full geometry build. Vector mode supports text, CSS, use/symbol, nested viewports, clipping, holes and styled/dashed strokes; curves use tolerance in mm. For masks, filters, patterns and embedded images explicitly choose silhouette mode: alpha-threshold pixel approximation at rasterSize resolution. Returns contour SVG, source, warnings and analysis. Optional includeRushGraph returns an editable rush/ir-1 document with svg_height parameter and a separate verified full build; RushGraph limits are 256 points per ring, 128 nodes and bounded profile work. Default 96 DPI; maximum 4 MiB input and 20000 contour points.',
      inputSchema: z
        .object({
          ...svgInput,
          height: z.number().positive().max(100000),
          includeRushGraph: z.boolean().default(false),
        })
        .strict(),
      outputSchema: rushSvgExtrudeOutputSchema,
      annotations: svgAnnotations,
    },
    async (input, context) => {
      try {
        const profile = await svgProfile(input.svg, options(input)),
          source = contoursExtrusion(profile.contours, input.height)
        const rushGraph = input.includeRushGraph ? await svgProfileRushGraph(profile.contours, input.height) : undefined
        const analysis = await geometry.analyze(source, 'full', context.mcpReq.signal)
        let nativeResult = {}
        if (rushGraph) {
          const rushAnalysis = await geometry.analyze(rushGraph.source, 'full', context.mcpReq.signal)
          const sameNumber = (a: number, b: number) => Math.abs(a - b) <= 1e-6 * Math.max(1, Math.abs(a), Math.abs(b))
          if (
            analysis.volume == null ||
            rushAnalysis.volume == null ||
            !sameNumber(analysis.volume, rushAnalysis.volume) ||
            !analysis.bounds ||
            !rushAnalysis.bounds ||
            !(['min', 'max'] as const).every(side =>
              analysis.bounds![side].every((value, axis) => sameNumber(value, rushAnalysis.bounds![side][axis]!)),
            ) ||
            rushAnalysis.topology.boundary !== 0 ||
            rushAnalysis.topology.nonManifold !== 0
          ) {
            throw new Error('SVG RushGraph conversion did not preserve the checked solid geometry.')
          }
          nativeResult = {
            document: rushGraph.document,
            document_sha256: rushGraph.document_sha256,
            rush_source: rushGraph.source,
            rush_analysis: rushAnalysis,
          }
        }
        const structuredContent = {
          source,
          svg: contoursSvg(profile.contours),
          warnings: profile.warnings,
          analysis,
          ...nativeResult,
        }
        return { structuredContent, content: [{ type: 'text' as const, text: JSON.stringify(structuredContent) }] }
      } catch (e) {
        return svgFailure(e)
      }
    },
  )
  registerRushSvgExportTool(server, geometry)
}
