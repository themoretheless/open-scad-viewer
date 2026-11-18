import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import { solveBondedSolid } from '../kernel/bondedSolid'
import type { BondedSolidModel } from '../kernel/bondedSolidProtocol'
import { warmGeometryKernel } from '../kernel/geometry/kernel'
import { validatePrintStrengthProfile, type PrintStrengthProfile } from '../kernel/trussScreening'
import { caeBool3Schema, caeVec3Schema } from './caeGuide'
import { toolOutputSchema } from './createServer/officialToolInputSchema'
import { errorResult, textResult } from './createServer/persistedFailureStatus'
import { InvalidToolInputError } from './publicError'

const annotations = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false }

const printedSolidInputSchema = z
  .object({
    nodes_mm: z.array(caeVec3Schema).min(4).max(125),
    tets: z
      .array(
        z
          .object({
            nodes: z.tuple([
              z.number().int().nonnegative(),
              z.number().int().nonnegative(),
              z.number().int().nonnegative(),
              z.number().int().nonnegative(),
            ]),
            region: z.enum(['shell', 'infill']),
            young_mpa: z.number().positive(),
            poisson: z.number().gt(-1).lt(0.5),
          })
          .strict(),
      )
      .min(1)
      .max(400),
    bonds: z
      .array(
        z
          .object({
            shell: z.tuple([
              z.number().int().nonnegative(),
              z.number().int().nonnegative(),
              z.number().int().nonnegative(),
            ]),
            infill: z.tuple([
              z.number().int().nonnegative(),
              z.number().int().nonnegative(),
              z.number().int().nonnegative(),
            ]),
            normal_stiffness_mpa_per_mm: z.number().positive(),
            shear_stiffness_mpa_per_mm: z.number().positive(),
            tension_mpa: z.number().positive(),
            shear_mpa: z.number().positive(),
            compression_mpa: z.number().positive(),
            property_source: z.string().min(1).max(200),
          })
          .strict(),
      )
      .min(1)
      .max(200),
    restrained: z.array(caeBool3Schema).min(4).max(125),
    forces_n: z.array(caeVec3Schema).min(4).max(125),
    safety_factor: z.number().positive().max(100),
    profile: z
      .object({
        material: z.string().min(1).max(64),
        grade: z.string().min(1).max(64),
        property_source: z.string().min(1).max(200),
        nozzle_mm: z.number().positive(),
        line_width_mm: z.number().positive(),
        layer_height_mm: z.number().positive(),
        nozzle_temp_c: z.number().min(0).max(500),
        bed_temp_c: z.number().min(0).max(200),
      })
      .strict(),
    service_temp_c: z.number().min(-273).max(400),
  })
  .strict()

const printedSolidOutputSchema = toolOutputSchema(
  z.object({
    model_kind: z.string(),
    geometry_binding: z.string(),
    property_model: z.string(),
    max_deflection_mm: z.number(),
    max_relative_residual: z.number(),
    limit_reached: z.boolean(),
    max_bond_utilization: z.number(),
    profile_warnings: z.array(z.string()),
    displacements_mm: z.array(caeVec3Schema),
    reactions_n: z.array(caeVec3Schema),
    bonds: z.array(
      z.object({
        area_mm2: z.number(),
        normal: caeVec3Schema,
        force_on_shell_n: caeVec3Schema,
        opening_mm: caeVec3Schema,
        normal_traction_mpa: caeVec3Schema,
        shear_traction_mpa: caeVec3Schema,
        utilization: z.number(),
      }),
    ),
  }),
)

export function registerCaePrintedSolidAnalyzeTool(server: McpServer): void {
  server.registerTool(
    'cae_printed_solid_analyze',
    {
      title: 'Analyze bonded Tet4 shell/infill assembly with FDM print strength calibration',
      description:
        'Validate FDM process parameters and solve a bonded 4-node tetrahedral shell/infill assembly (max 125 nodes, 400 tets, 200 bonds) for displacements, bond tractions, and strength utilization.',
      inputSchema: printedSolidInputSchema,
      outputSchema: printedSolidOutputSchema,
      annotations,
    },
    async input => {
      try {
        await warmGeometryKernel()
        if (input.restrained.length !== input.nodes_mm.length || input.forces_n.length !== input.nodes_mm.length) {
          throw new InvalidToolInputError('restrained and forces_n lengths must match nodes_mm.length.')
        }
        const profile: PrintStrengthProfile = {
          material: input.profile.material,
          grade: input.profile.grade,
          propertySource: input.profile.property_source,
          nozzleMm: input.profile.nozzle_mm,
          lineWidthMm: input.profile.line_width_mm,
          layerHeightMm: input.profile.layer_height_mm,
          nozzleTempC: input.profile.nozzle_temp_c,
          bedTempC: input.profile.bed_temp_c,
        }
        const model: BondedSolidModel = {
          nodesMm: input.nodes_mm,
          tets: input.tets.map(t => ({ nodes: t.nodes, region: t.region, youngMpa: t.young_mpa, poisson: t.poisson })),
          bonds: input.bonds.map(b => ({
            shell: b.shell,
            infill: b.infill,
            normalStiffnessMpaPerMm: b.normal_stiffness_mpa_per_mm,
            shearStiffnessMpaPerMm: b.shear_stiffness_mpa_per_mm,
            tensionMpa: b.tension_mpa,
            shearMpa: b.shear_mpa,
            compressionMpa: b.compression_mpa,
            propertySource: b.property_source,
          })),
          restrained: input.restrained,
          forcesN: input.forces_n,
          safetyFactor: input.safety_factor,
          profile,
          serviceTempC: input.service_temp_c,
        }
        try {
          const validated = validatePrintStrengthProfile(profile)
          const res = solveBondedSolid(JSON.stringify(model))
          return textResult({
            model_kind: res.modelKind,
            geometry_binding: res.geometryBinding,
            property_model: validated.propertyModel,
            max_deflection_mm: res.maxDeflectionMm,
            max_relative_residual: res.maxRelativeResidual,
            limit_reached: res.limitReached,
            max_bond_utilization: Math.max(0, ...res.bonds.map(b => b.utilization)),
            profile_warnings: res.profileWarnings,
            displacements_mm: res.displacementsMm,
            reactions_n: res.reactionsN,
            bonds: res.bonds.map(b => ({
              area_mm2: b.areaMm2,
              normal: b.normal,
              force_on_shell_n: b.forceOnShellN,
              opening_mm: b.openingMm,
              normal_traction_mpa: b.normalTractionMpa,
              shear_traction_mpa: b.shearTractionMpa,
              utilization: b.utilization,
            })),
          })
        } catch (solverError) {
          throw new InvalidToolInputError(
            solverError instanceof Error ? solverError.message : 'Bonded solid solver rejected model.',
            { code: 'invalid_geometry' },
          )
        }
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}
