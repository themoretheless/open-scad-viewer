import { z } from 'zod/v4'
import { idSchema, qualitySchema, scalarSchema, sha256Schema } from './commonSchemas'
import { runtimeExecutionSchema, toolErrorResultSchema } from './executionAndBuildSchemas'

export const pointSchema = z.array(z.number().finite()).length(3)

export const topologySchema = z.object({
  boundary: z.number().int().nonnegative(),
  crease: z.number().int().nonnegative(),
  non_manifold: z.number().int().nonnegative(),
  degenerate: z.number().int().nonnegative(),
})

export const boundsSchema = z.object({ min: pointSchema, max: pointSchema }).nullable()

export const parameterSchema = z.object({
  name: z.string(),
  label: z.string(),
  value: scalarSchema,
  min: z.number().nullable(),
  max: z.number().nullable(),
  step: z.number().nullable(),
  options: z.array(scalarSchema).nullable(),
})

export const objectSourceSchema = z.object({
  label: z.string(),
  start: z.number().int().nonnegative(),
  end: z.number().int().nonnegative(),
  operation_id: z.string().nullable(),
  instance_id: z.string().nullable(),
  triangles: z.number().int().nonnegative(),
})

export const objectAnalysisSchema = z.object({
  index: z.number().int().nonnegative(),
  entity_id: z.string().nullable(),
  vertices: z.number().int().nonnegative(),
  triangles: z.number().int().nonnegative(),
  bounds: boundsSchema,
  dimensions: pointSchema,
  center: pointSchema.nullable(),
  color: z.array(z.number().finite()).length(4),
  topology: topologySchema,
  sources: z.array(objectSourceSchema),
  sources_truncated: z.boolean(),
})

export const analysisSchema = z.object({
  execution: runtimeExecutionSchema,
  quality: qualitySchema,
  reduced: z.boolean(),
  duration_ms: z.number().nonnegative(),
  mesh_count: z.number().int().nonnegative(),
  vertex_count: z.number().int().nonnegative(),
  triangle_count: z.number().int().nonnegative(),
  volume: z.number(),
  surface_area: z.number().nonnegative(),
  bounds: boundsSchema,
  topology: topologySchema,
  warnings: z.array(z.string()),
  parameters: z.array(parameterSchema),
  parameters_truncated: z.boolean(),
  objects: z.array(objectAnalysisSchema),
  objects_truncated: z.boolean(),
  details_truncated: z.boolean(),
})

export const comparisonSideSchema = z.object({
  source_sha256: sha256Schema,
  model_id: idSchema.nullable(),
  model_revision: z.number().int().nonnegative().nullable(),
  execution: runtimeExecutionSchema,
  mesh_count: z.number().int().nonnegative(),
  triangle_count: z.number().int().nonnegative(),
  volume: z.number().nonnegative(),
  surface_area: z.number().nonnegative(),
  bounds: boundsSchema,
  dimensions: pointSchema,
  topology: topologySchema,
  warnings: z.array(z.string()),
})

export const comparisonMetricDeltaSchema = z.object({
  boundary: z.number(),
  crease: z.number(),
  non_manifold: z.number(),
  degenerate: z.number(),
})

export const comparisonDeltaSchema = z.object({
  mesh_count: z.number(),
  triangle_count: z.number(),
  volume: z.number(),
  surface_area: z.number(),
  dimensions: pointSchema,
  topology: comparisonMetricDeltaSchema,
})

export const comparisonSuccessSchema = z.object({
  quality: qualitySchema,
  comparison_mode: z.enum(['same_engine_metrics', 'cross_engine_metrics_only']),
  geometric_equivalence: z.literal(false),
  left: comparisonSideSchema,
  right: comparisonSideSchema,
  delta: comparisonDeltaSchema,
})

export const comparisonFailureContextSchema = z.object({
  failed_side: z.enum(['left', 'right']),
  completed_left: comparisonSideSchema.optional(),
})

export const comparisonErrorResultSchema = toolErrorResultSchema.extend({
  comparison_failure: comparisonFailureContextSchema,
})
