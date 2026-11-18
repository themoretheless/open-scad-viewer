import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import {
  evaluateIso286HoleBasisFit,
  formatDrawingDimensionWithTolerance,
  vectorDrawingPdf,
  type Iso286HoleZone,
  type Iso286ShaftZone,
} from '../app/drawingSheets'
import { createDrawingViews, drawingViewsSheet, type DrawingViewsOptions } from '../kernel/drawingViews'
import { warmGeometryKernel } from '../kernel/geometry/kernel'
import type { McpGeometryService } from './geometryService'
import { meshSourceOrUploadInputSchema, resolveMeshFromSourceOrUpload } from './meshInputHelper'
import { toolOutputSchema } from './createServer/officialToolInputSchema'
import { errorResult, textResult } from './createServer/persistedFailureStatus'
import { InvalidToolInputError } from './publicError'

const annotations = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false }
const VIEW_VALUES = ['front', 'top', 'right', 'iso'] as const
const HOLE_ZONES = ['H7', 'H8', 'H9'] as const satisfies readonly Iso286HoleZone[]
const SHAFT_ZONES = ['g6', 'f7', 'h6', 'k6', 'n6', 'p6', 's6'] as const satisfies readonly Iso286ShaftZone[]

const printChecksSchema = z.object({
  minWallMm: z.number().nullable(),
  thinTriangles: z.number().int().nonnegative(),
  wallOk: z.boolean(),
  minClearanceMm: z.number().nullable(),
  clearancePairs: z.number().int().nonnegative(),
  clearanceOk: z.boolean(),
  overhangAreaMm2: z.number(),
  overhangTriangles: z.number().int().nonnegative(),
  overhangThresholdDeg: z.number(),
  overhangOk: z.boolean(),
})

function registerCadDrawingSheetTool(server: McpServer, geometry: McpGeometryService): void {
  server.registerTool(
    'cad_drawing_sheet',
    {
      title: 'Generate multi-view SVG and vector PDF engineering drawing with DFM checks',
      description:
        'Render hidden-line orthographic + isometric views, optional hatched cross-section, and wall/clearance/overhang printability checks for any OpenSCAD source or uploaded mesh. Optionally returns a searchable vector PDF resource.',
      inputSchema: z
        .object({
          target: meshSourceOrUploadInputSchema,
          title: z.string().min(1).max(120).optional(),
          views: z.array(z.enum(VIEW_VALUES)).min(1).max(4).optional(),
          section: z
            .object({
              view: z.enum(VIEW_VALUES),
              offset: z.number().finite().min(-10_000).max(10_000),
              spacing: z.number().positive().max(100).optional(),
            })
            .strict()
            .optional(),
          min_wall_mm: z.number().positive().max(100).optional(),
          min_clearance_mm: z.number().positive().max(100).optional(),
          overhang_angle_deg: z.number().min(1).max(89).optional(),
          include_pdf: z.boolean().optional(),
        })
        .strict(),
      outputSchema: toolOutputSchema(
        z.object({
          title: z.string(),
          width_mm: z.number().positive(),
          height_mm: z.number().positive(),
          views: z.array(
            z.object({
              view: z.enum(VIEW_VALUES),
              visible_segments: z.number().int().nonnegative(),
              hidden_segments: z.number().int().nonnegative(),
              silhouette_segments: z.number().int().nonnegative(),
            }),
          ),
          section: z
            .object({
              view: z.enum(VIEW_VALUES),
              loop_count: z.number().int().nonnegative(),
              hatch_segment_count: z.number().int().nonnegative(),
              area_mm2: z.number(),
            })
            .nullable(),
          print_checks: printChecksSchema,
          svg: z.string(),
          pdf_byte_length: z.number().int().nonnegative().nullable(),
        }),
      ),
      annotations,
    },
    async (input, context) => {
      try {
        await warmGeometryKernel()
        const { mesh, sourceLabel } = await resolveMeshFromSourceOrUpload(input.target, geometry, context.mcpReq.signal)
        const title = input.title ?? `Drawing (${sourceLabel})`
        const options: DrawingViewsOptions = {
          title,
          ...(input.views !== undefined ? { views: input.views } : {}),
          ...(input.section !== undefined
            ? {
                section: {
                  view: input.section.view,
                  offset: input.section.offset,
                  ...(input.section.spacing !== undefined ? { spacing: input.section.spacing } : {}),
                },
              }
            : {}),
          ...(input.min_wall_mm !== undefined ? { minWallMm: input.min_wall_mm } : {}),
          ...(input.min_clearance_mm !== undefined ? { minClearanceMm: input.min_clearance_mm } : {}),
          ...(input.overhang_angle_deg !== undefined ? { overhangAngleDeg: input.overhang_angle_deg } : {}),
        }
        let drawing: ReturnType<typeof createDrawingViews>
        let sheet: ReturnType<typeof drawingViewsSheet>
        let pdfBytes: Uint8Array | null = null
        try {
          drawing = createDrawingViews(mesh, options)
          sheet = drawingViewsSheet(drawing, title)
          if (input.include_pdf !== false) {
            pdfBytes = vectorDrawingPdf([sheet])
          }
        } catch (error) {
          throw new InvalidToolInputError(error instanceof Error ? error.message : 'Drawing generation failed.', {
            code: 'invalid_geometry',
          })
        }
        const structured = {
          title,
          width_mm: sheet.width,
          height_mm: sheet.height,
          views: drawing.views.map(v => ({
            view: v.view,
            visible_segments: v.segments.filter(seg => !seg.hidden).length,
            hidden_segments: v.segments.filter(seg => seg.hidden).length,
            silhouette_segments: v.segments.filter(seg => seg.silhouette).length,
          })),
          section: drawing.section
            ? {
                view: drawing.section.view,
                loop_count: drawing.section.rings.length,
                hatch_segment_count: drawing.section.hatch.length,
                area_mm2: drawing.section.area,
              }
            : null,
          print_checks: drawing.printChecks,
          svg: drawing.svg,
          pdf_byte_length: pdfBytes ? pdfBytes.byteLength : null,
        }
        return pdfBytes
          ? {
              structuredContent: structured,
              content: [
                { type: 'text' as const, text: JSON.stringify(structured) },
                {
                  type: 'resource' as const,
                  resource: {
                    uri: 'drawing://sheet/drawing.pdf',
                    mimeType: 'application/pdf',
                    blob: Buffer.from(pdfBytes).toString('base64'),
                  },
                },
              ],
            }
          : textResult(structured)
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

function registerCadIso286FitTool(server: McpServer): void {
  server.registerTool(
    'cad_iso286_fit',
    {
      title: 'Compute ISO 286-1 hole-basis shaft/hole fit deviations and callouts',
      description:
        'Evaluate ISO 286-1 hole-basis tolerance zones (H7/H8/H9 vs g6/f7/h6/k6/n6/p6/s6) for nominal diameters in (0, 500] mm, returning fundamental tolerance unit, bilateral limits, min/max clearance, fit class, and drawing callouts.',
      inputSchema: z
        .object({
          nominal_diameter_mm: z.number().gt(0).max(500),
          hole_zone: z.enum(HOLE_ZONES),
          shaft_zone: z.enum(SHAFT_ZONES),
        })
        .strict(),
      outputSchema: toolOutputSchema(
        z.object({
          nominal_diameter_mm: z.number(),
          geometric_mean_diameter_mm: z.number(),
          fundamental_tolerance_unit_um: z.number(),
          hole_zone: z.enum(HOLE_ZONES),
          shaft_zone: z.enum(SHAFT_ZONES),
          hole_tolerance_mm: z.number(),
          shaft_tolerance_mm: z.number(),
          hole_upper_deviation_mm: z.number(),
          hole_lower_deviation_mm: z.number(),
          shaft_upper_deviation_mm: z.number(),
          shaft_lower_deviation_mm: z.number(),
          max_clearance_mm: z.number(),
          min_clearance_mm: z.number(),
          fit_type: z.enum(['clearance', 'transition', 'interference']),
          assembly_callout: z.string(),
          hole_callout: z.string(),
          shaft_callout: z.string(),
        }),
      ),
      annotations,
    },
    async input => {
      try {
        const fit = evaluateIso286HoleBasisFit(input.nominal_diameter_mm, input.hole_zone, input.shaft_zone)
        return textResult({
          nominal_diameter_mm: fit.nominalDiaMm,
          geometric_mean_diameter_mm: fit.geometricMeanDiaMm,
          fundamental_tolerance_unit_um: fit.fundamentalToleranceUnitUm,
          hole_zone: fit.holeZone,
          shaft_zone: fit.shaftZone,
          hole_tolerance_mm: fit.holeToleranceMm,
          shaft_tolerance_mm: fit.shaftToleranceMm,
          hole_upper_deviation_mm: fit.holeUpperDeviationMm,
          hole_lower_deviation_mm: fit.holeLowerDeviationMm,
          shaft_upper_deviation_mm: fit.shaftUpperDeviationMm,
          shaft_lower_deviation_mm: fit.shaftLowerDeviationMm,
          max_clearance_mm: fit.maxClearanceMm,
          min_clearance_mm: fit.minClearanceMm,
          fit_type: fit.fitType,
          assembly_callout: formatDrawingDimensionWithTolerance(fit.nominalDiaMm, {
            prefix: '⌀',
            fitCode: `${fit.holeZone}/${fit.shaftZone}`,
            decimals: 3,
          }),
          hole_callout: formatDrawingDimensionWithTolerance(fit.nominalDiaMm, {
            prefix: '⌀',
            fitCode: fit.holeZone,
            upperDeviationMm: fit.holeUpperDeviationMm,
            lowerDeviationMm: fit.holeLowerDeviationMm,
            decimals: 3,
          }),
          shaft_callout: formatDrawingDimensionWithTolerance(fit.nominalDiaMm, {
            prefix: '⌀',
            fitCode: fit.shaftZone,
            upperDeviationMm: fit.shaftUpperDeviationMm,
            lowerDeviationMm: fit.shaftLowerDeviationMm,
            decimals: 3,
          }),
        })
      } catch (error) {
        return errorResult(error instanceof RangeError ? new InvalidToolInputError(error.message) : error)
      }
    },
  )
}

export function registerDrawingTools(server: McpServer, geometry: McpGeometryService): void {
  registerCadDrawingSheetTool(server, geometry)
  registerCadIso286FitTool(server)
}
