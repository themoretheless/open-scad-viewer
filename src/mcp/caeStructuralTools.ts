import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import {
  solveFrame,
  type FrameMember,
  type FrameMemberLoad,
  type FrameModel,
  type FrameSupport,
} from '../kernel/frameAnalysis'
import { warmGeometryKernel } from '../kernel/geometry/kernel'
import { solveTruss, solveTrussBuckling, solveTrussModal, type TrussModel } from '../kernel/trussAnalysis'
import { screenTrussMembers } from '../kernel/trussScreening'
import { caeBool3Schema, caeBool6Schema, caeVec3Schema } from './caeGuide'
import { toolOutputSchema } from './createServer/officialToolInputSchema'
import { errorResult, textResult } from './createServer/persistedFailureStatus'
import { InvalidToolInputError } from './publicError'

const annotations = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false }

const trussInputSchema = z
  .object({
    nodes_mm: z.array(caeVec3Schema).min(2).max(512),
    members: z
      .array(
        z
          .object({
            nodes: z.tuple([z.number().int().nonnegative(), z.number().int().nonnegative()]),
            young_mpa: z.number().positive(),
            area_mm2: z.number().positive(),
          })
          .strict(),
      )
      .min(1)
      .max(2048),
    restrained: z.array(caeBool3Schema).min(2).max(512),
    forces_n: z.array(caeVec3Schema).min(2).max(512),
    buckling_modes: z.number().int().min(1).max(12).optional(),
    modal: z
      .object({
        densities_t_mm3: z.array(z.number().nonnegative()).min(1).max(2048),
        mass_model: z.enum(['lumped', 'consistent']),
        modes: z.number().int().min(1).max(12),
      })
      .strict()
      .optional(),
    axial_limits: z
      .object({
        tension_mpa: z.number().positive(),
        compression_mpa: z.number().positive(),
        safety_factor: z.number().positive(),
      })
      .strict()
      .optional(),
  })
  .strict()

const trussOutputSchema = toolOutputSchema(
  z.object({
    free_dofs: z.number().int().nonnegative(),
    max_deflection_mm: z.number(),
    max_relative_residual: z.number(),
    displacements_mm: z.array(caeVec3Schema),
    reactions_n: z.array(caeVec3Schema),
    axial_forces_n: z.array(z.number()),
    axial_stresses_mpa: z.array(z.number()),
    buckling: z
      .object({
        critical_load_factor: z.number().nullable(),
        modes: z.array(z.object({ load_factor: z.number(), relative_residual: z.number() })),
      })
      .nullable(),
    modal: z
      .object({
        total_mass_t: z.number(),
        modes: z.array(z.object({ frequency_hz: z.number(), omega_rad_s: z.number(), relative_residual: z.number() })),
      })
      .nullable(),
    screening: z
      .array(
        z.object({
          member_index: z.number().int().nonnegative(),
          utilization: z.number(),
          stress_mpa: z.number(),
          status: z.enum(['overloaded', 'low-demand', 'within-axial-limit']),
        }),
      )
      .nullable(),
  }),
)

const frameLoadInputSchema = z.union([
  z
    .object({
      type: z.literal('pointForce'),
      member: z.number().int().nonnegative(),
      at_mm: z.number().finite(),
      force_n: caeVec3Schema,
      local_axes: z.boolean(),
    })
    .strict(),
  z
    .object({
      type: z.literal('pointMoment'),
      member: z.number().int().nonnegative(),
      at_mm: z.number().finite(),
      moment_nmm: caeVec3Schema,
      local_axes: z.boolean(),
    })
    .strict(),
  z
    .object({
      type: z.literal('uniform'),
      member: z.number().int().nonnegative(),
      force_n_per_mm: caeVec3Schema,
      local_axes: z.boolean(),
    })
    .strict(),
  z
    .object({
      type: z.literal('trapezoidal'),
      member: z.number().int().nonnegative(),
      from_n_per_mm: caeVec3Schema,
      to_n_per_mm: caeVec3Schema,
      local_axes: z.boolean(),
    })
    .strict(),
])

const frameSupportInputSchema = z.union([
  z
    .object({
      type: z.enum(['spring', 'lowerSpring', 'upperSpring']),
      node: z.number().int().nonnegative(),
      dof: z.number().int().min(0).max(5),
      stiffness: z.number().positive(),
    })
    .strict(),
  z
    .object({
      type: z.enum(['lowerBound', 'upperBound']),
      node: z.number().int().nonnegative(),
      dof: z.number().int().min(0).max(5),
    })
    .strict(),
])

const frameInputSchema = z
  .object({
    nodes_mm: z.array(caeVec3Schema).min(2).max(256),
    members: z
      .array(
        z
          .object({
            nodes: z.tuple([z.number().int().nonnegative(), z.number().int().nonnegative()]),
            young_mpa: z.number().positive(),
            poisson: z.number().gt(-1).lt(0.5),
            area_mm2: z.number().positive(),
            iyy_mm4: z.number().positive(),
            izz_mm4: z.number().positive(),
            j_mm4: z.number().positive(),
            shear_area_y_mm2: z.number().positive().optional(),
            shear_area_z_mm2: z.number().positive().optional(),
            local_z_hint: caeVec3Schema.optional(),
            release_a: caeBool3Schema.optional(),
            release_b: caeBool3Schema.optional(),
          })
          .strict(),
      )
      .min(1)
      .max(1024),
    restrained: z.array(caeBool6Schema).min(2).max(256),
    forces_n: z.array(caeVec3Schema).min(2).max(256),
    moments_nmm: z.array(caeVec3Schema).min(2).max(256),
    loads: z.array(frameLoadInputSchema).max(512).optional(),
    supports: z.array(frameSupportInputSchema).max(256).optional(),
    include_full_stations: z.boolean().optional(),
  })
  .strict()

const frameOutputSchema = toolOutputSchema(
  z.object({
    free_dofs: z.number().int().nonnegative(),
    max_deflection_mm: z.number(),
    max_relative_residual: z.number(),
    displacements_mm: z.array(caeVec3Schema),
    rotations_rad: z.array(caeVec3Schema),
    reactions_n: z.array(caeVec3Schema),
    reaction_moments_nmm: z.array(caeVec3Schema),
    member_envelopes: z.array(
      z.object({
        member_index: z.number().int().nonnegative(),
        max_abs_axial_n: z.number(),
        max_abs_shear_n: z.number(),
        max_abs_torsion_nmm: z.number(),
        max_abs_moment_nmm: z.number(),
        stations: z
          .array(
            z.object({
              x_mm: z.number(),
              axial_n: z.number(),
              shear_y_n: z.number(),
              shear_z_n: z.number(),
              torsion_nmm: z.number(),
              moment_y_nmm: z.number(),
              moment_z_nmm: z.number(),
            }),
          )
          .nullable(),
      }),
    ),
  }),
)

function toFrameLoads(loads: ReadonlyArray<z.infer<typeof frameLoadInputSchema>>): FrameMemberLoad[] {
  return loads.map(l => {
    if (l.type === 'pointForce')
      return { type: 'pointForce', member: l.member, atMm: l.at_mm, forceN: l.force_n, localAxes: l.local_axes }
    if (l.type === 'pointMoment')
      return { type: 'pointMoment', member: l.member, atMm: l.at_mm, momentNmm: l.moment_nmm, localAxes: l.local_axes }
    if (l.type === 'uniform')
      return { type: 'uniform', member: l.member, forceNPerMm: l.force_n_per_mm, localAxes: l.local_axes }
    return {
      type: 'trapezoidal',
      member: l.member,
      fromNPerMm: l.from_n_per_mm,
      toNPerMm: l.to_n_per_mm,
      localAxes: l.local_axes,
    }
  })
}

function registerCaeTrussAnalyzeTool(server: McpServer): void {
  server.registerTool(
    'cae_truss_analyze',
    {
      title: 'Run 3D pin-jointed truss linear static, buckling, modal, and axial screening analysis',
      description:
        'Solve 3D truss displacements, reactions, axial forces/stresses in Rust, with optional linearized buckling modes, dynamic natural frequencies, and tension/compression screening.',
      inputSchema: trussInputSchema,
      outputSchema: trussOutputSchema,
      annotations,
    },
    async input => {
      try {
        await warmGeometryKernel()
        if (input.restrained.length !== input.nodes_mm.length || input.forces_n.length !== input.nodes_mm.length) {
          throw new InvalidToolInputError('restrained and forces_n arrays must match nodes_mm.length.')
        }
        if (input.modal && input.modal.densities_t_mm3.length !== input.members.length) {
          throw new InvalidToolInputError('modal.densities_t_mm3 length must match members.length.')
        }
        const model: TrussModel = {
          nodesMm: input.nodes_mm,
          members: input.members.map(m => ({ nodes: m.nodes, youngMpa: m.young_mpa, areaMm2: m.area_mm2 })),
          restrained: input.restrained,
          forcesN: input.forces_n,
        }
        try {
          const statics = solveTruss(model)
          const buckling = input.buckling_modes !== undefined ? solveTrussBuckling(model, input.buckling_modes) : null
          const modal = input.modal
            ? solveTrussModal(
                { nodesMm: model.nodesMm, members: model.members, restrained: model.restrained },
                input.modal.densities_t_mm3,
                input.modal.mass_model,
                input.modal.modes,
              )
            : null
          const screening = input.axial_limits
            ? screenTrussMembers(model, [statics], {
                tensionMpa: input.axial_limits.tension_mpa,
                compressionMpa: input.axial_limits.compression_mpa,
                safetyFactor: input.axial_limits.safety_factor,
              })
            : null
          return textResult({
            free_dofs: statics.freeDofs,
            max_deflection_mm: statics.maxDeflectionMm,
            max_relative_residual: statics.maxRelativeResidual,
            displacements_mm: statics.displacementsMm,
            reactions_n: statics.reactionsN,
            axial_forces_n: statics.axialForcesN,
            axial_stresses_mpa: statics.axialStressesMpa,
            buckling: buckling
              ? {
                  critical_load_factor: buckling.modes[0]?.loadFactor ?? null,
                  modes: buckling.modes.map(m => ({
                    load_factor: m.loadFactor,
                    relative_residual: m.relativeResidual,
                  })),
                }
              : null,
            modal: modal
              ? {
                  total_mass_t: modal.totalMassT,
                  modes: modal.modes.map(m => ({
                    frequency_hz: m.frequencyHz,
                    omega_rad_s: m.omegaRadS,
                    relative_residual: m.relativeResidual,
                  })),
                }
              : null,
            screening: screening
              ? screening.map(row => ({
                  member_index: row.memberIndex,
                  utilization: row.utilization,
                  stress_mpa: row.stressMpa,
                  status: row.status,
                }))
              : null,
          })
        } catch (solverError) {
          throw new InvalidToolInputError(
            solverError instanceof Error ? solverError.message : 'Truss solver rejected model.',
            { code: 'invalid_geometry' },
          )
        }
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

function registerCaeFrameAnalyzeTool(server: McpServer): void {
  server.registerTool(
    'cae_frame_analyze',
    {
      title: 'Run 3D Timoshenko beam/frame linear static analysis',
      description:
        'Analyze 3D rigid-jointed or released frames with bending, shear, torsion, distributed member loads, and optional spring/contact supports. Returns nodal displacements/rotations, support reactions, and 21-station member envelopes.',
      inputSchema: frameInputSchema,
      outputSchema: frameOutputSchema,
      annotations,
    },
    async input => {
      try {
        await warmGeometryKernel()
        const n = input.nodes_mm.length
        if (input.restrained.length !== n || input.forces_n.length !== n || input.moments_nmm.length !== n) {
          throw new InvalidToolInputError('restrained, forces_n, and moments_n lengths must match nodes_mm.length.')
        }
        const members: FrameMember[] = input.members.map(m => ({
          nodes: m.nodes,
          youngMpa: m.young_mpa,
          poisson: m.poisson,
          areaMm2: m.area_mm2,
          iyyMm4: m.iyy_mm4,
          izzMm4: m.izz_mm4,
          jMm4: m.j_mm4,
          ...(m.shear_area_y_mm2 !== undefined ? { shearAreaYMm2: m.shear_area_y_mm2 } : {}),
          ...(m.shear_area_z_mm2 !== undefined ? { shearAreaZMm2: m.shear_area_z_mm2 } : {}),
          ...(m.local_z_hint !== undefined ? { localZHint: m.local_z_hint } : {}),
          ...(m.release_a !== undefined ? { releaseA: m.release_a } : {}),
          ...(m.release_b !== undefined ? { releaseB: m.release_b } : {}),
        }))
        const model: FrameModel = {
          nodesMm: input.nodes_mm,
          members,
          restrained: input.restrained,
          ...(input.supports !== undefined ? { supports: input.supports as FrameSupport[] } : {}),
          forcesN: input.forces_n,
          momentsNmm: input.moments_nmm,
          loads: toFrameLoads(input.loads ?? []),
        }
        try {
          const res = solveFrame(model)
          const includeStations = input.include_full_stations === true
          return textResult({
            free_dofs: res.freeDofs,
            max_deflection_mm: res.maxDeflectionMm,
            max_relative_residual: res.maxRelativeResidual,
            displacements_mm: res.displacementsMm,
            rotations_rad: res.rotationsRad,
            reactions_n: res.reactionsN,
            reaction_moments_nmm: res.reactionMomentsNmm,
            member_envelopes: res.members.map((m, idx) => ({
              member_index: idx,
              max_abs_axial_n: Math.max(0, ...m.stations.map(s => Math.abs(s.axialN))),
              max_abs_shear_n: Math.max(0, ...m.stations.map(s => Math.hypot(s.shearYN, s.shearZN))),
              max_abs_torsion_nmm: Math.max(0, ...m.stations.map(s => Math.abs(s.torsionNmm))),
              max_abs_moment_nmm: Math.max(0, ...m.stations.map(s => Math.hypot(s.momentYNmm, s.momentZNmm))),
              stations: includeStations
                ? m.stations.map(s => ({
                    x_mm: s.xMm,
                    axial_n: s.axialN,
                    shear_y_n: s.shearYN,
                    shear_z_n: s.shearZN,
                    torsion_nmm: s.torsionNmm,
                    moment_y_nmm: s.momentYNmm,
                    moment_z_nmm: s.momentZNmm,
                  }))
                : null,
            })),
          })
        } catch (solverError) {
          throw new InvalidToolInputError(
            solverError instanceof Error ? solverError.message : 'Frame solver rejected model.',
            { code: 'invalid_geometry' },
          )
        }
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

export function registerCaeStructuralTools(server: McpServer): void {
  registerCaeTrussAnalyzeTool(server)
  registerCaeFrameAnalyzeTool(server)
}
