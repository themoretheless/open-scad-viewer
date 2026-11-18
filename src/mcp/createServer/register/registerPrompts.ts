import type { McpServer } from '@modelcontextprotocol/server'
import { idSchema, promptGoalSchema } from '../../createServer'
import { hasProfile, type McpToolsetProfile } from '../../toolsetProfiles'
import { z } from 'zod/v4'
import type { McpServerContext } from './context'

/** Guided OpenSCAD prompts. */
export function registerPrompts(ctx: McpServerContext): void {
  if (!hasProfile(ctx.options.toolsetProfiles, 'openscad')) return
  const { server } = ctx
  const promptRevisionSchema = z
    .string()
    .max(16)
    .regex(/^\d+$/)
    .refine(value => Number.isSafeInteger(Number(value)), 'Revision must be a safe integer')
    .describe('Optional immutable model revision written as a decimal integer.')

  server.registerPrompt(
    'openscad_review_model',
    {
      title: 'Review a saved OpenSCAD model',
      description: 'Guide a reproducible geometry and print-readiness review without modifying the catalog.',
      argsSchema: z.object({
        model_id: idSchema,
        revision: promptRevisionSchema.optional(),
        goal: promptGoalSchema.optional(),
      }),
    },
    ({ model_id, revision, goal }) => {
      const selector = revision === undefined ? { model_id } : { model_id, revision: Number(revision) }
      return {
        messages: [
          {
            role: 'user' as const,
            content: {
              type: 'text' as const,
              text: `Review the saved OpenSCAD model selected by ${JSON.stringify(selector)}.${goal ? ` Goal: ${goal}` : ''}
Call openscad_check with full quality. Report dimensions, volume, surface area, mesh and triangle counts, boundary/non-manifold/degenerate topology, warnings, and relevant source provenance. Distinguish proven facts from printability assumptions. Do not save or export unless I explicitly ask.`,
            },
          },
        ],
      }
    },
  )

  server.registerPrompt(
    'openscad_customize_workflow',
    {
      title: 'Customize a saved OpenSCAD model',
      description: 'Guide safe Customizer exploration with an explicit revision check before any save.',
      argsSchema: z.object({
        model_id: idSchema,
        revision: promptRevisionSchema.optional(),
        goal: promptGoalSchema,
      }),
    },
    ({ model_id, revision, goal }) => {
      const selector = revision === undefined ? { model_id } : { model_id, revision: Number(revision) }
      return {
        messages: [
          {
            role: 'user' as const,
            content: {
              type: 'text' as const,
              text: `Prepare a safe customization of ${JSON.stringify(selector)} for this goal: ${goal}
First call openscad_check in preview quality and inspect its declared top-level Customizer parameters. Explore only valid declared values with openscad_customize and check the returned source. Do not save automatically. If I approve a save, read the current model first and call openscad_customize_model with its revision as expected_revision so concurrent changes cannot be overwritten.`,
            },
          },
        ],
      }
    },
  )
}

/** Guided domain prompts registered by stateless CAD/CAM/CAE profiles. */
export function registerCadDomainPrompts(server: McpServer, profiles?: ReadonlySet<McpToolsetProfile>): void {
  if (hasProfile(profiles, 'exchange') || hasProfile(profiles, 'direct')) {
    server.registerPrompt(
      'cad_step_iges_roundtrip_audit',
      {
        title: 'STEP / IGES B-rep interchange audit',
        description: 'Guide exact CAD exchange inspection, round-trip import, and B-rep contact validation.',
        argsSchema: z.object({ format: z.enum(['step', 'iges']), goal: promptGoalSchema.optional() }),
      },
      ({ format, goal }) => ({
        messages: [
          {
            role: 'user' as const,
            content: {
              type: 'text' as const,
              text: `Audit ${format.toUpperCase()} B-rep interchange.${goal ? ` Goal: ${goal}` : ''}
Inspect incoming geometry with ${format === 'step' ? 'cad_import_step' : 'cad_import_iges'}, verify face injectivity and pair contacts with brep_inspect_contacts, and validate export round-trip without falling back to polygonal meshes.`,
            },
          },
        ],
      }),
    )
  }

  if (hasProfile(profiles, 'rush')) {
    server.registerPrompt(
      'rush_assembly_clearance_review',
      {
        title: 'Rush assembly clearance and interference audit',
        description: 'Check multi-part Rush assemblies for solid overlaps, contacts, and minimum clearances.',
        argsSchema: z.object({
          min_clearance_mm: z.string().max(16).optional(),
          goal: promptGoalSchema.optional(),
        }),
      },
      ({ min_clearance_mm, goal }) => ({
        messages: [
          {
            role: 'user' as const,
            content: {
              type: 'text' as const,
              text: `Audit a Rush assembly for interference and clearance.${min_clearance_mm ? ` Target clearance >= ${min_clearance_mm} mm.` : ''}${goal ? ` Goal: ${goal}` : ''}
Read rush://language, compile with rush_compile, inspect with rush_report and rush_interference using the requested clearance_threshold_mm, and report every colliding, touching, and below-threshold pair.`,
            },
          },
        ],
      }),
    )
  }

  if (hasProfile(profiles, 'direct')) {
    server.registerPrompt(
      'direct_brep_modeling_workflow',
      {
        title: 'Direct B-rep document workflow',
        description: 'Discover the DirectDocument schema, apply atomic actions, and inspect resulting topology.',
        argsSchema: z.object({ goal: promptGoalSchema }),
      },
      ({ goal }) => ({
        messages: [
          {
            role: 'user' as const,
            content: {
              type: 'text' as const,
              text: `Build or modify a Direct B-rep model for this goal: ${goal}
Read direct://schema first, initialize a workspace with direct_create_document if starting from scratch, apply edits atomically with openscad_direct_transaction, and verify bodies/faces/sketches via direct_inspect_document.`,
            },
          },
        ],
      }),
    )
  }

  if (hasProfile(profiles, 'cam')) {
    server.registerPrompt(
      'cam_print_and_slice_workflow',
      {
        title: 'DFM printability, slicing, and G-code inspection',
        description: 'Validate manifold print readiness, generate toolpaths, and verify extruded volume and bounds.',
        argsSchema: z.object({
          layer_height_mm: z.string().max(16).optional(),
          goal: promptGoalSchema.optional(),
        }),
      },
      ({ layer_height_mm, goal }) => ({
        messages: [
          {
            role: 'user' as const,
            content: {
              type: 'text' as const,
              text: `Prepare a part for 3D printing.${layer_height_mm ? ` Target layer height: ${layer_height_mm} mm.` : ''}${goal ? ` Goal: ${goal}` : ''}
Check cam_print_readiness and mesh_inspect (repair with mesh_repair if needed), slice with cam_slice_gcode, verify output metrics via cam_inspect_gcode, and generate an engineering sheet via cad_drawing_sheet.`,
            },
          },
        ],
      }),
    )
  }

  if (hasProfile(profiles, 'cae')) {
    server.registerPrompt(
      'cae_structural_optimization',
      {
        title: 'Structural CAE analysis workflow',
        description: 'Run truss, frame, or Tet4 bonded printed-solid analysis with explicit engineering budgets.',
        argsSchema: z.object({
          target_safety_factor: z.string().max(16).optional(),
          goal: promptGoalSchema,
        }),
      },
      ({ target_safety_factor, goal }) => ({
        messages: [
          {
            role: 'user' as const,
            content: {
              type: 'text' as const,
              text: `Analyze structural performance for: ${goal}${target_safety_factor ? ` (target safety factor >= ${target_safety_factor})` : ''}.
Read cae://guide first, choose cae_truss_analyze, cae_frame_analyze, or cae_printed_solid_analyze, and report displacements, peak stress/utilization, buckling/modal margins, and geometry recommendations.`,
            },
          },
        ],
      }),
    )
  }
}
