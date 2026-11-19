import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import { planLaserMeshSection, type LaserProcessSettings } from '../fabrication/laserSectionPlanning'
import { warmGeometryKernel } from '../kernel/geometry/kernel'
import { emitLaserGrbl, preflightLaserPlan, type LaserMachineProfile } from '../kernel/geometry/laser'
import type { PolygonMesh } from '../kernel/geometry/polygon'
import type { McpGeometryService } from './geometryService'
import { meshSourceOrUploadInputSchema, resolveMeshFromSourceOrUpload } from './meshInputHelper'
import { toolOutputSchema } from './createServer/officialToolInputSchema'
import { errorResult, textResult } from './createServer/persistedFailureStatus'
import { InvalidToolInputError } from './publicError'

const annotations = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false }

export function meshBoundsSummary(mesh: PolygonMesh): { minZ: number; maxZ: number; minX: number; minY: number } {
  let minZ = Infinity
  let maxZ = -Infinity
  let minX = Infinity
  let minY = Infinity
  for (let i = 0; i < mesh.positions.length; i += 3) {
    const x = mesh.positions[i]!
    const y = mesh.positions[i + 1]!
    const z = mesh.positions[i + 2]!
    if (x < minX) minX = x
    if (y < minY) minY = y
    if (z < minZ) minZ = z
    if (z > maxZ) maxZ = z
  }
  if (!Number.isFinite(minZ) || !Number.isFinite(maxZ)) {
    throw new InvalidToolInputError('Mesh contains no finite vertices.', { code: 'invalid_geometry' })
  }
  return { minZ, maxZ, minX, minY }
}

const laserSectionInputSchema = z
  .object({
    target: meshSourceOrUploadInputSchema,
    z_mm: z.number().finite().min(-10_000).max(10_000),
    bed_width_mm: z.number().min(10).max(5000).optional(),
    bed_height_mm: z.number().min(10).max(5000).optional(),
    max_power: z.number().min(1).max(100_000).optional(),
    power_mode: z.enum(['m3', 'm4']).optional(),
    laser_mode_confirmed: z.boolean().optional(),
    line_speed_mm_min: z.number().min(1).max(100_000).optional(),
    line_power: z.number().min(0).max(100_000).optional(),
    passes: z.number().int().min(1).max(50).optional(),
    kerf_mm: z.number().min(0).max(10).optional(),
    kerf_mode: z.enum(['center', 'part', 'cavity']).optional(),
    fill_enabled: z.boolean().optional(),
    fill_spacing_mm: z.number().min(0.05).max(50).optional(),
    offset_xy_mm: z.tuple([z.number().finite(), z.number().finite()]).optional(),
  })
  .strict()

const laserSectionOutputSchema = toolOutputSchema(
  z.object({
    offset_xy_mm: z.tuple([z.number(), z.number()]),
    operation_count: z.number().int().nonnegative(),
    path_count: z.number().int().nonnegative(),
    segment_count: z.number().int().nonnegative(),
    cut_distance_mm: z.number(),
    rapid_distance_mm: z.number(),
    estimated_time_s: z.number(),
    bounds: z.object({
      min: z.tuple([z.number(), z.number()]),
      max: z.tuple([z.number(), z.number()]),
    }),
    gcode_emitted: z.boolean(),
    dialect: z.string().nullable(),
    gcode_byte_length: z.number().int().nonnegative().nullable(),
  }),
)

export function registerCamLaserSectionPlanTool(server: McpServer, geometry: McpGeometryService): void {
  server.registerTool(
    'cam_laser_section_plan',
    {
      title: 'Plan 2D laser cutting/engraving operations from a horizontal mesh cross-section',
      description:
        'Section a mesh at Z (mm), apply optional kerf compensation and hatch fill, run machine bed preflight bounds checks, and optionally emit GRBL G-code when laser_mode_confirmed=true.',
      inputSchema: laserSectionInputSchema,
      outputSchema: laserSectionOutputSchema,
      annotations,
    },
    async (input, context) => {
      try {
        await warmGeometryKernel()
        const { mesh } = await resolveMeshFromSourceOrUpload(input.target, geometry, context.mcpReq.signal)
        const { minX, minY } = meshBoundsSummary(mesh)
        const offset: [number, number] = input.offset_xy_mm ?? [minX < 5 ? 5 - minX : 0, minY < 5 ? 5 - minY : 0]
        const machine: LaserMachineProfile = {
          widthMm: input.bed_width_mm ?? 300,
          heightMm: input.bed_height_mm ?? 200,
          maxPower: input.max_power ?? 1000,
          estimatedRapidMmMin: 6000,
          powerMode: input.power_mode ?? 'm4',
          laserModeConfirmed: input.laser_mode_confirmed ?? false,
          supportsAirAssist: false,
          flipY: false,
          returnToOrigin: true,
        }
        const lineSpeed = input.line_speed_mm_min ?? 900
        const linePower = input.line_power ?? 250
        const settings: LaserProcessSettings = {
          line: {
            output: true,
            speedMmMin: lineSpeed,
            power: linePower,
            passes: input.passes ?? 1,
            airAssist: false,
            kerfMm: input.kerf_mm ?? 0,
            kerfMode: input.kerf_mode ?? 'center',
            pathOrder: 'inner-first-nearest',
          },
          fill: {
            output: input.fill_enabled ?? false,
            speedMmMin: lineSpeed * 1.5,
            power: Math.round(linePower * 0.5),
            passes: 1,
            airAssist: false,
            spacingMm: input.fill_spacing_mm ?? 1.5,
            pathOrder: 'nearest',
          },
          offset,
        }
        let summary: ReturnType<typeof preflightLaserPlan>
        let program: ReturnType<typeof emitLaserGrbl> | null = null
        try {
          const plan = planLaserMeshSection(mesh, input.z_mm, machine, settings)
          summary = preflightLaserPlan(plan)
          if (machine.laserModeConfirmed) program = emitLaserGrbl(plan)
        } catch (laserError) {
          throw new InvalidToolInputError(laserError instanceof Error ? laserError.message : 'Laser planning failed.', {
            code: 'invalid_geometry',
          })
        }
        const structured = {
          offset_xy_mm: offset,
          operation_count: summary.operationCount,
          path_count: summary.pathCount,
          segment_count: summary.segmentCount,
          cut_distance_mm: summary.cutDistanceMm,
          rapid_distance_mm: summary.rapidDistanceMm,
          estimated_time_s: summary.estimatedTimeS,
          bounds: summary.bounds,
          gcode_emitted: program !== null,
          dialect: program ? program.dialect : null,
          gcode_byte_length: program ? Buffer.byteLength(program.gcode, 'utf8') : null,
        }
        return program
          ? {
              structuredContent: structured,
              content: [
                { type: 'text' as const, text: JSON.stringify(structured) },
                {
                  type: 'resource' as const,
                  resource: { uri: 'cam://laser/section.nc', mimeType: 'text/x-gcode', text: program.gcode },
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
