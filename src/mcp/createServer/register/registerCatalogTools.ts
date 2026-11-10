import { catalogStatsSchema, idSchema, modelMetadataSchema, sourceSchema } from '../../createServer'
import { MAX_MODEL_NAME_LENGTH, isWellFormedUnicode } from '../../modelStore'
import type { StoredModel, StoredModelRevision } from '../../modelStore'
import { ModelNotFoundError } from '../../publicError'
import { modelRevisionToWire, modelToWire, toolOutputSchema } from '../officialToolInputSchema'
import { errorResult, textResult } from '../persistedFailureStatus'
import { z } from 'zod/v4'
import type { McpServerContext } from './context'

/** Model catalog: save, read, list and statistics. */
export function registerCatalogTools(ctx: McpServerContext): void {
  const { store, geometry, server, notifyResourceListChanged } = ctx
  server.registerTool('openscad_save_model', {
    title: 'Save OpenSCAD model',
    description: 'Create or update a versioned OpenSCAD model in the local DuckDB catalog.',
    inputSchema: z.object({
      id: idSchema.optional(),
      name: z.string().min(1).max(MAX_MODEL_NAME_LENGTH)
        .refine(isWellFormedUnicode, 'Name must contain well-formed Unicode')
        .refine(name => name.trim().length > 0, 'Name must contain a non-whitespace character'),
      source: sourceSchema,
      expected_revision: z.number().int().nonnegative().optional(),
    }).strict(),
    outputSchema: toolOutputSchema(z.object({ model: modelMetadataSchema })),
    annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false },
  }, async input => {
    try {
      geometry.planSource(input.source, 'preview', 'preview')
      const model = await store.saveModel({
        id: input.id,
        name: input.name,
        source: input.source,
        expectedRevision: input.expected_revision,
      })
      notifyResourceListChanged()
      return textResult({ model: modelToWire(model) })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_get_model', {
    title: 'Get OpenSCAD model',
    description: 'Read a saved OpenSCAD model and its current revision from DuckDB.',
    inputSchema: z.object({
      id: idSchema,
      revision: z.number().int().nonnegative().optional(),
    }).strict(),
    outputSchema: toolOutputSchema(z.object({ model: modelMetadataSchema.extend({ source: z.string() }) })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async ({ id, revision }) => {
    try {
      const model = revision === undefined
        ? await store.getModel(id)
        : await store.getModelRevision(id, revision)
      if (!model) return errorResult(new ModelNotFoundError(id, revision))
      const metadata = revision === undefined
        ? modelToWire(model as StoredModel)
        : modelRevisionToWire(model as StoredModelRevision)
      return textResult({ model: { ...metadata, source: model.source } })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_list_models', {
    title: 'List OpenSCAD models',
    description: 'List saved OpenSCAD models from the local DuckDB catalog.',
    inputSchema: z.object({ limit: z.number().int().min(1).max(500).default(100) }),
    outputSchema: toolOutputSchema(z.object({
      models: z.array(modelMetadataSchema.extend({ source_length: z.number().int().nonnegative() })),
    })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async ({ limit }) => {
    try {
      const models = await store.listModels(limit)
      return textResult({
        models: models.map(model => ({
          ...modelToWire({ ...model, source: '' }),
          source_length: model.sourceLength,
        })),
      })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_catalog_stats', {
    title: 'OpenSCAD catalog statistics',
    description: 'Return bounded DuckDB catalog usage, build outcomes, and quota limits without exposing SQL.',
    inputSchema: z.object({}),
    outputSchema: toolOutputSchema(z.object({ stats: catalogStatsSchema })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async () => {
    try {
      const stats = await store.getCatalogStats()
      return textResult({
        stats: {
          models: stats.modelCount,
          revisions: stats.revisionCount,
          builds: stats.buildCount,
          artifacts: stats.artifactCount,
          stored_source_bytes: stats.storedSourceBytes,
          stored_artifact_bytes: stats.storedArtifactBytes,
          builds_by_status: stats.buildsByStatus,
          limits: {
            models: stats.limits.models,
            revisions: stats.limits.revisions,
            revisions_per_model: stats.limits.revisionsPerModel,
            source_bytes: stats.limits.sourceBytes,
            builds: stats.limits.builds,
            artifacts: stats.limits.artifacts,
            artifact_bytes: stats.limits.artifactBytes,
          },
        },
      })
    } catch (error) {
      return errorResult(error)
    }
  })
}
