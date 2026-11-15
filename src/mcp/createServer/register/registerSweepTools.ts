import { z } from 'zod/v4'
import {
  DEFAULT_SWEEP_MAX_COMBINATIONS,
  DEFAULT_SWEEP_MAX_TOTAL_TRIANGLES,
  DEFAULT_SWEEP_TIMEOUT_MS,
  MAX_SWEEP_AXES,
  MAX_SWEEP_AXIS_VALUES,
  MAX_SWEEP_COMBINATIONS,
  MAX_SWEEP_TIMEOUT_MS,
  MAX_SWEEP_TOTAL_TRIANGLES,
  runOpenScadParameterSweep,
  SWEEP_RANK_OPTIONS,
} from '../../parameterSweep'
import { PUBLIC_ERROR_CODES } from '../../publicError'
import { executionToWire, selectorSchema, toolOutputSchema } from '../officialToolInputSchema'
import { errorResult, resolveSource, textResult } from '../persistedFailureStatus'
import { boundsSchema } from '../schemas/analysisAndComparisonSchemas'
import { qualitySchema, scalarSchema, sha256Schema, wellFormedText } from '../schemas/commonSchemas'
import { executionSchema } from '../schemas/executionAndBuildSchemas'
import type { McpServerContext } from './context'

const sweepAxisInputSchema = z
  .object({
    name: wellFormedText(128, 'Parameter name').min(1),
    values: z.array(scalarSchema).min(1).max(MAX_SWEEP_AXIS_VALUES),
  })
  .strict()

const fixedValuesSchema = z
  .record(wellFormedText(128, 'Parameter name').min(1), scalarSchema)
  .refine(record => Object.keys(record).length <= 64, 'fixed_values may contain at most 64 parameters')

const sweepVariantSchema = z.object({
  index: z.number().int().nonnegative(),
  rank: z.number().int().positive().nullable(),
  parameters: z.record(z.string(), scalarSchema),
  source_sha256: sha256Schema,
  status: z.enum(['succeeded', 'failed']),
  duration_ms: z.number().nonnegative().optional(),
  metrics: z
    .object({
      mesh_count: z.number().int().nonnegative(),
      triangle_count: z.number().int().nonnegative(),
      vertex_count: z.number().int().nonnegative(),
      volume: z.number(),
      surface_area: z.number().nonnegative(),
      bounds: boundsSchema,
    })
    .optional(),
  error: z
    .object({
      code: z.enum(PUBLIC_ERROR_CODES),
      message: z.string(),
    })
    .optional(),
})

const sweepOutputSchema = z.object({
  quality: qualitySchema,
  execution: executionSchema,
  rank_by: z.enum(SWEEP_RANK_OPTIONS),
  combination_count: z.number().int().nonnegative(),
  evaluated_count: z.number().int().nonnegative(),
  succeeded_count: z.number().int().nonnegative(),
  failed_count: z.number().int().nonnegative(),
  total_triangles: z.number().int().nonnegative(),
  truncated_by_budget: z.boolean(),
  stop_reason: z.enum(['completed', 'triangle_budget_exceeded', 'timeout']),
  ranking_order: z.array(z.number().int().nonnegative()),
  variants: z.array(sweepVariantSchema),
})

/** Read-only bounded parameter sweep across Customizer parameter combinations. */
export function registerSweepTools(ctx: McpServerContext): void {
  const { store, geometry, server } = ctx
  server.registerTool(
    'openscad_parameter_sweep',
    {
      title: 'Run bounded Customizer parameter sweep',
      description:
        'Evaluate a bounded Cartesian product of OpenSCAD Customizer parameter values in preview-first headless mode, ranking valid variants by geometry metrics without persisting catalog writes.',
      inputSchema: selectorSchema({
        axes: z.array(sweepAxisInputSchema).min(1).max(MAX_SWEEP_AXES),
        fixed_values: fixedValuesSchema.optional(),
        quality: qualitySchema.default('preview'),
        max_combinations: z.number().int().min(1).max(MAX_SWEEP_COMBINATIONS).default(DEFAULT_SWEEP_MAX_COMBINATIONS),
        max_total_triangles: z
          .number()
          .int()
          .min(1)
          .max(MAX_SWEEP_TOTAL_TRIANGLES)
          .default(DEFAULT_SWEEP_MAX_TOTAL_TRIANGLES),
        timeout_ms: z.number().int().min(100).max(MAX_SWEEP_TIMEOUT_MS).default(DEFAULT_SWEEP_TIMEOUT_MS),
        rank_by: z.enum(SWEEP_RANK_OPTIONS).default('volume_asc'),
      }),
      outputSchema: toolOutputSchema(sweepOutputSchema),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
    },
    async (input, context) => {
      try {
        const selected = await resolveSource(store, input)
        const result = await runOpenScadParameterSweep(geometry, {
          source: selected.source,
          axes: input.axes,
          fixedValues: input.fixed_values,
          quality: input.quality,
          maxCombinations: input.max_combinations,
          maxTotalTriangles: input.max_total_triangles,
          timeoutMs: input.timeout_ms,
          rankBy: input.rank_by,
          signal: context.mcpReq.signal,
        })
        return textResult({
          quality: result.quality,
          execution: executionToWire(result.execution),
          rank_by: result.rank_by,
          combination_count: result.combination_count,
          evaluated_count: result.evaluated_count,
          succeeded_count: result.succeeded_count,
          failed_count: result.failed_count,
          total_triangles: result.total_triangles,
          truncated_by_budget: result.truncated_by_budget,
          stop_reason: result.stop_reason,
          ranking_order: result.ranking_order,
          variants: result.variants,
        })
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}
