import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import { assertPrintReady, formatPrintReadinessError, PrintReadinessError } from '../fabrication/printReadiness'
import { warmGeometryKernel } from '../kernel/geometry/kernel'
import {
  emitPolygonMeshGcode,
  emitPolygonMeshGcodeJob,
  GCODE_FLAVORS,
  inspectGcode,
  type GcodeJobExportResult,
  type JobSettingsInput,
  type ToolpathSettingsInput,
} from '../kernel/geometry/polygon'
import { meshBoundsSummary, registerCamLaserSectionPlanTool } from './camLaserTools'
import type { McpGeometryService } from './geometryService'
import { meshSourceOrUploadInputSchema, resolveMeshFromSourceOrUpload } from './meshInputHelper'
import { toolOutputSchema } from './createServer/officialToolInputSchema'
import { errorResult, textResult } from './createServer/persistedFailureStatus'
import { InvalidToolInputError } from './publicError'

const annotations = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false }

const gcodePreviewBoundsSchema = z
  .object({
    min: z.tuple([z.number(), z.number(), z.number()]),
    max: z.tuple([z.number(), z.number(), z.number()]),
  })
  .nullable()

const printReadinessOutputSchema = toolOutputSchema(
  z.object({
    ready: z.boolean(),
    error_code: z.string().nullable(),
    message: z.string(),
    vertex_count: z.number().int().nonnegative(),
    triangle_count: z.number().int().nonnegative(),
    component_count: z.number().int().nonnegative().nullable(),
    contact_count: z.number().int().nonnegative(),
    details: z
      .object({
        boundaryEdges: z.number().int().nonnegative().optional(),
        nonManifoldEdges: z.number().int().nonnegative().optional(),
        nonManifoldVertices: z.number().int().nonnegative().optional(),
        degenerateTriangles: z.number().int().nonnegative().optional(),
        isolatedVertices: z.number().int().nonnegative().optional(),
        orientationConflicts: z.number().int().nonnegative().optional(),
        contacts: z.number().int().nonnegative().optional(),
        triangleIds: z.array(z.number().int().nonnegative()).optional(),
        stopReason: z.enum(['work-limit', 'contact-limit']).optional(),
        reason: z.string().optional(),
      })
      .nullable(),
  }),
)

const sliceGcodeInputSchema = z
  .object({
    target: meshSourceOrUploadInputSchema,
    mode: z.enum(['preview', 'job']).optional(),
    flavor: z.enum(GCODE_FLAVORS).optional(),
    z_min_mm: z.number().finite().min(-10_000).max(10_000).optional(),
    z_max_mm: z.number().finite().min(-10_000).max(10_000).optional(),
    layer_height_mm: z.number().min(0.04).max(2).optional(),
    line_width_mm: z.number().min(0.1).max(4).optional(),
    wall_count: z.number().int().min(1).max(20).optional(),
    infill_spacing_mm: z.number().min(0.2).max(100).optional(),
    feedrate_mm_s: z.number().min(1).max(1000).optional(),
    travel_feedrate_mm_s: z.number().min(1).max(1000).optional(),
    nozzle_temp_c: z.number().min(0).max(500).optional(),
    bed_temp_c: z.number().min(0).max(200).optional(),
    skip_readiness_gate: z.boolean().optional(),
  })
  .strict()

const sliceGcodeOutputSchema = toolOutputSchema(
  z.object({
    mode: z.enum(['preview', 'job']),
    dialect: z.string(),
    flavor: z.string().nullable(),
    layer_count: z.number().int().nonnegative(),
    extrusion_mm: z.number(),
    deposited_volume_mm3: z.number(),
    print_distance_mm: z.number(),
    travel_distance_mm: z.number(),
    estimated_time_s: z.number(),
    move_count: z.number().int().nonnegative(),
    bounds: gcodePreviewBoundsSchema,
    gcode_byte_length: z.number().int().nonnegative(),
    gcode3mf_byte_length: z.number().int().nonnegative().nullable(),
  }),
)

function registerCamPrintReadinessTool(server: McpServer, geometry: McpGeometryService): void {
  server.registerTool(
    'cam_print_readiness',
    {
      title: 'Check 3D print readiness (watertightness, winding, self-intersections)',
      description:
        'Run the mandatory Rust slicer gate (manifoldness, consistent orientation, self-intersection proof) on an OpenSCAD source or uploaded mesh.',
      inputSchema: z
        .object({
          target: meshSourceOrUploadInputSchema,
          max_contacts: z.number().int().min(1).max(10_000).optional(),
        })
        .strict(),
      outputSchema: printReadinessOutputSchema,
      annotations,
    },
    async (input, context) => {
      try {
        await warmGeometryKernel()
        const { mesh } = await resolveMeshFromSourceOrUpload(input.target, geometry, context.mcpReq.signal)
        try {
          const report = assertPrintReady(
            mesh,
            ...(input.max_contacts !== undefined ? [{ maxContacts: input.max_contacts }] : []),
          )
          return textResult({
            ready: true,
            error_code: null,
            message: 'Mesh passed watertightness, orientation, and self-intersection checks.',
            vertex_count: report.manifold.vertexCount,
            triangle_count: report.manifold.triangleCount,
            component_count: report.manifold.componentCount,
            contact_count: 0,
            details: null,
          })
        } catch (err) {
          if (err instanceof PrintReadinessError) {
            return textResult({
              ready: false,
              error_code: err.code,
              message: formatPrintReadinessError(err, 'en'),
              vertex_count: Math.floor(mesh.positions.length / 3),
              triangle_count: Math.floor(mesh.indices.length / 3),
              component_count: null,
              contact_count: err.details.contacts ?? 0,
              details: {
                ...(err.details.boundaryEdges !== undefined ? { boundaryEdges: err.details.boundaryEdges } : {}),
                ...(err.details.nonManifoldEdges !== undefined
                  ? { nonManifoldEdges: err.details.nonManifoldEdges }
                  : {}),
                ...(err.details.nonManifoldVertices !== undefined
                  ? { nonManifoldVertices: err.details.nonManifoldVertices }
                  : {}),
                ...(err.details.degenerateTriangles !== undefined
                  ? { degenerateTriangles: err.details.degenerateTriangles }
                  : {}),
                ...(err.details.isolatedVertices !== undefined
                  ? { isolatedVertices: err.details.isolatedVertices }
                  : {}),
                ...(err.details.orientationConflicts !== undefined
                  ? { orientationConflicts: err.details.orientationConflicts }
                  : {}),
                ...(err.details.contacts !== undefined ? { contacts: err.details.contacts } : {}),
                ...(err.details.triangleIds !== undefined ? { triangleIds: err.details.triangleIds } : {}),
                ...(err.details.stopReason !== undefined ? { stopReason: err.details.stopReason } : {}),
                ...(err.details.reason !== undefined ? { reason: err.details.reason } : {}),
              },
            })
          }
          throw err
        }
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

function registerCamSliceGcodeTool(server: McpServer, geometry: McpGeometryService): void {
  server.registerTool(
    'cam_slice_gcode',
    {
      title: 'Slice a solid mesh into FDM toolpaths and G-code',
      description:
        'Enforce the print-readiness gate (unless skip_readiness_gate=true), slice a mesh horizontally, and emit preview G-code or full machine job G-code (+ `.gcode.3mf`). Returns time/filament metrics and embedded G-code resource.',
      inputSchema: sliceGcodeInputSchema,
      outputSchema: sliceGcodeOutputSchema,
      annotations,
    },
    async (input, context) => {
      try {
        await warmGeometryKernel()
        const { mesh } = await resolveMeshFromSourceOrUpload(input.target, geometry, context.mcpReq.signal)
        if (input.skip_readiness_gate !== true) {
          try {
            assertPrintReady(mesh)
          } catch (err) {
            if (err instanceof PrintReadinessError) {
              throw new InvalidToolInputError(formatPrintReadinessError(err, 'en'), { code: 'invalid_geometry' })
            }
            throw err
          }
        }
        const { minZ, maxZ } = meshBoundsSummary(mesh)
        const zMin = input.z_min_mm ?? minZ
        const zMax = input.z_max_mm ?? maxZ
        if (!(zMax > zMin)) throw new InvalidToolInputError('Slice range requires z_max_mm > z_min_mm.')
        const toolSettings: ToolpathSettingsInput = {
          ...(input.layer_height_mm !== undefined ? { layerHeightMm: input.layer_height_mm } : {}),
          ...(input.line_width_mm !== undefined ? { lineWidthMm: input.line_width_mm } : {}),
          ...(input.wall_count !== undefined ? { wallCount: input.wall_count } : {}),
          ...(input.infill_spacing_mm !== undefined ? { infillSpacingMm: input.infill_spacing_mm } : {}),
          ...(input.feedrate_mm_s !== undefined ? { feedrateMmS: input.feedrate_mm_s } : {}),
          ...(input.travel_feedrate_mm_s !== undefined ? { travelFeedrateMmS: input.travel_feedrate_mm_s } : {}),
        }
        const mode = input.mode ?? 'preview'
        let result: ReturnType<typeof emitPolygonMeshGcode> | GcodeJobExportResult
        try {
          if (mode === 'job') {
            const jobSettings: JobSettingsInput = {
              ...toolSettings,
              ...(input.flavor !== undefined ? { flavor: input.flavor } : {}),
              ...(input.nozzle_temp_c !== undefined ? { nozzleTempC: input.nozzle_temp_c } : {}),
              ...(input.bed_temp_c !== undefined ? { bedTempC: input.bed_temp_c } : {}),
            }
            result = emitPolygonMeshGcodeJob(mesh, zMin, zMax, jobSettings)
          } else {
            result = emitPolygonMeshGcode(mesh, zMin, zMax, toolSettings)
          }
        } catch (sliceError) {
          throw new InvalidToolInputError(sliceError instanceof Error ? sliceError.message : 'Slicing failed.', {
            code: 'invalid_geometry',
          })
        }
        const jobResult = mode === 'job' ? (result as GcodeJobExportResult) : null
        const structured = {
          mode,
          dialect: result.dialect,
          flavor: jobResult ? jobResult.flavor : null,
          layer_count: result.layerCount,
          extrusion_mm: result.preview.extrusionMm,
          deposited_volume_mm3: result.preview.depositedVolumeMm3,
          print_distance_mm: result.preview.printDistanceMm,
          travel_distance_mm: result.preview.travelDistanceMm,
          estimated_time_s: result.preview.estimatedTimeS,
          move_count: result.preview.moves.length,
          bounds: result.preview.bounds,
          gcode_byte_length: Buffer.byteLength(result.gcode, 'utf8'),
          gcode3mf_byte_length: jobResult ? Buffer.from(jobResult.gcode3mfBase64, 'base64').byteLength : null,
        }
        return {
          structuredContent: structured,
          content: [
            { type: 'text' as const, text: JSON.stringify(structured) },
            {
              type: 'resource' as const,
              resource: { uri: 'cam://gcode/output.gcode', mimeType: 'text/x-gcode', text: result.gcode },
            },
            ...(jobResult
              ? [
                  {
                    type: 'resource' as const,
                    resource: {
                      uri: 'cam://gcode/output.gcode.3mf',
                      mimeType: 'application/vnd.ms-package.3dmanufacturing-3dmodel+xml',
                      blob: jobResult.gcode3mfBase64,
                    },
                  },
                ]
              : []),
          ],
        }
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

function registerCamInspectGcodeTool(server: McpServer): void {
  server.registerTool(
    'cam_inspect_gcode',
    {
      title: 'Inspect native or foreign G-code file',
      description:
        'Parse native print-preview/print-job or third-party slicer G-code into layers, filament usage, bounds, and estimated time.',
      inputSchema: z
        .object({
          gcode: z
            .string()
            .min(1)
            .max(16 * 1024 * 1024),
        })
        .strict(),
      outputSchema: toolOutputSchema(
        z.object({
          dialect: z.string(),
          native: z.boolean(),
          generator: z.string(),
          flavor: z.string().nullable(),
          filament_diameter_mm: z.number().nullable(),
          layers: z.number().int().nonnegative(),
          extrusion_mm: z.number(),
          deposited_volume_mm3: z.number(),
          print_distance_mm: z.number(),
          travel_distance_mm: z.number(),
          estimated_time_s: z.number(),
          move_count: z.number().int().nonnegative(),
          bounds: gcodePreviewBoundsSchema,
          firmware_error: z.string().nullable(),
        }),
      ),
      annotations,
    },
    async input => {
      try {
        await warmGeometryKernel()
        const inspected = inspectGcode(input.gcode)
        return textResult({
          dialect: inspected.dialect,
          native: inspected.native,
          generator: inspected.generator,
          flavor: inspected.flavor,
          filament_diameter_mm: inspected.filamentDiameterMm,
          layers: inspected.preview.layers,
          extrusion_mm: inspected.preview.extrusionMm,
          deposited_volume_mm3: inspected.preview.depositedVolumeMm3,
          print_distance_mm: inspected.preview.printDistanceMm,
          travel_distance_mm: inspected.preview.travelDistanceMm,
          estimated_time_s: inspected.preview.estimatedTimeS,
          move_count: inspected.preview.moves.length,
          bounds: inspected.preview.bounds,
          firmware_error: inspected.firmware?.error ?? null,
        })
      } catch (error) {
        return errorResult(
          error instanceof InvalidToolInputError
            ? error
            : new InvalidToolInputError(error instanceof Error ? error.message : 'G-code parsing failed.'),
        )
      }
    },
  )
}

export function registerCamTools(server: McpServer, geometry: McpGeometryService): void {
  registerCamPrintReadinessTool(server, geometry)
  registerCamSliceGcodeTool(server, geometry)
  registerCamInspectGcodeTool(server)
  registerCamLaserSectionPlanTool(server, geometry)
}
