import { idSchema, promptGoalSchema } from '../../createServer'
import { z } from 'zod/v4'
import type { McpServerContext } from './context'

/** Guided prompts. */
export function registerPrompts(ctx: McpServerContext): void {
  const { geometry, server } = ctx
  const promptRevisionSchema = z.string().max(16).regex(/^\d+$/)
    .refine(value => Number.isSafeInteger(Number(value)), 'Revision must be a safe integer')
    .describe('Optional immutable model revision written as a decimal integer.')

  server.registerPrompt('openscad_review_model', {
    title: 'Review a saved OpenSCAD model',
    description: 'Guide a reproducible geometry and print-readiness review without modifying the catalog.',
    argsSchema: z.object({
      model_id: idSchema,
      revision: promptRevisionSchema.optional(),
      goal: promptGoalSchema.optional(),
    }),
  }, ({ model_id, revision, goal }) => {
    const selector = revision === undefined
      ? { model_id }
      : { model_id, revision: Number(revision) }
    return {
      messages: [{
        role: 'user' as const,
        content: {
          type: 'text' as const,
          text: `Review the saved OpenSCAD model selected by ${JSON.stringify(selector)}.${goal ? ` Goal: ${goal}` : ''}
Call openscad_check with full quality. Report dimensions, volume, surface area, mesh and triangle counts, boundary/non-manifold/degenerate topology, warnings, and relevant source provenance. Distinguish proven facts from printability assumptions. Do not save or export unless I explicitly ask.`,
        },
      }],
    }
  })

  server.registerPrompt('openscad_customize_workflow', {
    title: 'Customize a saved OpenSCAD model',
    description: 'Guide safe Customizer exploration with an explicit revision check before any save.',
    argsSchema: z.object({
      model_id: idSchema,
      revision: promptRevisionSchema.optional(),
      goal: promptGoalSchema,
    }),
  }, ({ model_id, revision, goal }) => {
    const selector = revision === undefined
      ? { model_id }
      : { model_id, revision: Number(revision) }
    return {
      messages: [{
        role: 'user' as const,
        content: {
          type: 'text' as const,
          text: `Prepare a safe customization of ${JSON.stringify(selector)} for this goal: ${goal}
First call openscad_check in preview quality and inspect its declared top-level Customizer parameters. Explore only valid declared values with openscad_customize and check the returned source. Do not save automatically. If I approve a save, read the current model first and call openscad_customize_model with its revision as expected_revision so concurrent changes cannot be overwritten.`,
        },
      }],
    }
  })
}
