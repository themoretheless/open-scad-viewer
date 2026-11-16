import { GeometryBusyError } from '../../geometryService'
import type { GeometryAnalysis } from '../../geometryService'
import {
  analysisSchema,
  buildSchema,
  comparisonErrorResultSchema,
  comparisonSuccessSchema,
  idSchema,
  modelMetadataSchema,
  qualitySchema,
  scalarSchema,
  sha256Schema,
  sourceReferenceSchema,
} from '../../createServer'
import type { CustomizerValue } from '../../../language/scadCustomizer'
import type { GeometryExecutionDescriptor } from '../../../core/geometryExecution'
import { attachGeometryExecutionToError } from '../../../kernel/geometryBuildEngine'
import { ModelRevisionConflictError } from '../../modelStore'
import type { StoredBuild } from '../../modelStore'
import { ModelNotFoundError } from '../../publicError'
import {
  analysisToWire,
  modelRevisionUri,
  modelToWire,
  selectorSchema,
  sha256,
  toolOutputSchema,
} from '../officialToolInputSchema'
import {
  buildToWire,
  comparisonDelta,
  comparisonSide,
  errorResult,
  recordFailure,
  resolveSource,
  sameExecutionContext,
  textResult,
} from '../persistedFailureStatus'
import { z } from 'zod/v4'
import type { McpServerContext } from './context'
import { registerExportAndHistoryTools } from './registerExportAndHistoryTools'

/** Revisions and direct geometry: check, compare, analyze, customize, export and build history. */
export function registerBuildTools(ctx: McpServerContext): void {
  registerOpenscadListModelRevisions(ctx)
  registerOpenscadCheck(ctx)
  registerOpenscadCompare(ctx)
  registerOpenscadAnalyze(ctx)
  registerOpenscadCustomize(ctx)
  registerOpenscadCustomizeModel(ctx)
  registerExportAndHistoryTools(ctx)
}

function registerOpenscadListModelRevisions(ctx: McpServerContext): void {
  const { store, server } = ctx
  server.registerTool(
    'openscad_list_model_revisions',
    {
      title: 'List OpenSCAD model revisions',
      description: 'List immutable source snapshots for one saved DuckDB model.',
      inputSchema: z.object({
        id: idSchema,
        limit: z.number().int().min(1).max(500).default(100),
      }),
      outputSchema: toolOutputSchema(
        z.object({
          revisions: z.array(
            z.object({
              id: z.string(),
              name: z.string(),
              revision: z.number().int().nonnegative(),
              created_at: z.string(),
              source_length: z.number().int().nonnegative(),
              resource_uri: z.string(),
            }),
          ),
        }),
      ),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
    },
    async ({ id, limit }) => {
      try {
        const revisions = await store.listModelRevisions(id, limit)
        return textResult({
          revisions: revisions.map(revision => ({
            id: revision.id,
            name: revision.name,
            revision: revision.revision,
            created_at: revision.createdAt,
            source_length: revision.sourceLength,
            resource_uri: modelRevisionUri(revision.id, revision.revision),
          })),
        })
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

function registerOpenscadCheck(ctx: McpServerContext): void {
  const { store, geometry, server } = ctx
  server.registerTool(
    'openscad_check',
    {
      title: 'Check OpenSCAD geometry',
      description:
        'Compile inline source or an exact saved-model revision without writing build history. Use preview for fast iteration and full for authoritative metrics.',
      inputSchema: selectorSchema({ quality: qualitySchema.default('preview') }),
      outputSchema: toolOutputSchema(
        z.object({
          source_sha256: sha256Schema,
          model_id: idSchema.nullable(),
          model_revision: z.number().int().nonnegative().nullable(),
          analysis: analysisSchema,
        }),
      ),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
    },
    async (input, context) => {
      try {
        const selected = await resolveSource(store, input)
        const analysis = await geometry.analyze(selected.source, input.quality, context.mcpReq.signal)
        return textResult({
          source_sha256: sha256(selected.source),
          model_id: selected.model?.id ?? null,
          model_revision: selected.model?.revision ?? null,
          analysis: analysisToWire(analysis),
        })
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

function registerOpenscadCompare(ctx: McpServerContext): void {
  const { store, geometry, server } = ctx
  server.registerTool(
    'openscad_compare',
    {
      title: 'Compare OpenSCAD geometry',
      description:
        'Compile two inline or saved-revision sources without persistence and return bounded metric, dimension, and topology deltas. This is not an exact geometric boolean diff.',
      inputSchema: z
        .object({
          left: sourceReferenceSchema,
          right: sourceReferenceSchema,
          quality: qualitySchema.default('full'),
        })
        .strict(),
      outputSchema: z.union([comparisonSuccessSchema, comparisonErrorResultSchema]),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
    },
    async ({ left, right, quality }, context) => {
      let leftSource: Awaited<ReturnType<typeof resolveSource>>
      let rightSource: Awaited<ReturnType<typeof resolveSource>>
      let leftAnalysis: GeometryAnalysis
      try {
        leftSource = await resolveSource(store, left)
      } catch (error) {
        return errorResult(error, { comparison_failure: { failed_side: 'left' } })
      }
      try {
        rightSource = await resolveSource(store, right)
      } catch (error) {
        return errorResult(error, { comparison_failure: { failed_side: 'right' } })
      }
      try {
        leftAnalysis = await geometry.analyze(leftSource.source, quality, context.mcpReq.signal)
      } catch (error) {
        return errorResult(error, { comparison_failure: { failed_side: 'left' } })
      }
      let rightAnalysis: GeometryAnalysis
      try {
        rightAnalysis =
          leftSource.source === rightSource.source
            ? leftAnalysis
            : await geometry.analyze(rightSource.source, quality, context.mcpReq.signal)
      } catch (error) {
        return errorResult(error, {
          comparison_failure: {
            failed_side: 'right',
            completed_left: comparisonSide(leftSource, leftAnalysis),
          },
        })
      }
      try {
        const leftSummary = comparisonSide(leftSource, leftAnalysis)
        const rightSummary = comparisonSide(rightSource, rightAnalysis)
        return textResult({
          quality,
          comparison_mode: sameExecutionContext(leftAnalysis.execution, rightAnalysis.execution)
            ? ('same_engine_metrics' as const)
            : ('cross_engine_metrics_only' as const),
          geometric_equivalence: false as const,
          left: leftSummary,
          right: rightSummary,
          delta: comparisonDelta(leftSummary, rightSummary),
        })
      } catch (error) {
        return errorResult(error, {
          comparison_failure: {
            failed_side: 'right',
            completed_left: comparisonSide(leftSource, leftAnalysis),
          },
        })
      }
    },
  )
}

function registerOpenscadAnalyze(ctx: McpServerContext): void {
  const { store, geometry, server, notifyResourceListChanged } = ctx
  server.registerTool(
    'openscad_analyze',
    {
      title: 'Analyze OpenSCAD geometry',
      description:
        'Compile OpenSCAD source or a saved model and return bounded geometry, topology, provenance, and Customizer facts. The build is recorded in DuckDB.',
      inputSchema: selectorSchema({ quality: qualitySchema.default('full') }),
      outputSchema: toolOutputSchema(z.object({ build: buildSchema, analysis: analysisSchema })),
      annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false },
    },
    async (input, context) => {
      const startedAt = performance.now()
      let completedExecution: GeometryExecutionDescriptor | undefined
      let selected: Awaited<ReturnType<typeof resolveSource>>
      try {
        selected = await resolveSource(store, input)
      } catch (error) {
        return errorResult(error)
      }
      try {
        const analysis = await geometry.analyze(selected.source, input.quality, context.mcpReq.signal)
        completedExecution = analysis.execution
        let build: StoredBuild
        try {
          build = await store.recordBuild({
            modelId: selected.model?.id,
            modelRevision: selected.model?.revision,
            source: selected.source,
            sourceSha256: sha256(selected.source),
            execution: analysis.execution,
            quality: input.quality,
            durationMs: analysis.durationMs,
            warnings: analysis.warnings,
            status: 'succeeded',
            metrics: {
              meshCount: analysis.meshCount,
              triangleCount: analysis.triangleCount,
              volume: analysis.volume,
              surfaceArea: analysis.surfaceArea,
              reduced: analysis.reduced,
            },
          })
        } catch (error) {
          attachGeometryExecutionToError(error, analysis.execution)
          throw error
        }
        notifyResourceListChanged()
        return textResult({ build: buildToWire(build), analysis: analysisToWire(analysis) })
      } catch (error) {
        if (error instanceof GeometryBusyError) return errorResult(error)
        const build = await recordFailure(store, geometry, {
          ...selected,
          quality: input.quality,
          purpose: 'analysis',
          durationMs: performance.now() - startedAt,
          error,
          execution: completedExecution,
        })
        if (build) notifyResourceListChanged()
        return errorResult(error, build ? { build: buildToWire(build) } : {})
      }
    },
  )
}

function registerOpenscadCustomize(ctx: McpServerContext): void {
  const { store, geometry, server } = ctx
  server.registerTool(
    'openscad_customize',
    {
      title: 'Customize OpenSCAD source',
      description:
        'Apply validated values to top-level OpenSCAD Customizer parameters and return updated source without saving it.',
      inputSchema: selectorSchema({ values: z.record(z.string(), scalarSchema) }),
      outputSchema: toolOutputSchema(
        z.object({
          source: z.string(),
          applied: z.array(z.string()),
          parameters: analysisSchema.shape.parameters,
          parameters_truncated: z.boolean(),
        }),
      ),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
    },
    async input => {
      try {
        const selected = await resolveSource(store, input)
        const result = geometry.customize(selected.source, input.values as Record<string, CustomizerValue>)
        return textResult({
          source: result.source,
          applied: result.applied,
          parameters: result.parameters,
          parameters_truncated: result.parametersTruncated,
        })
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}

function registerOpenscadCustomizeModel(ctx: McpServerContext): void {
  const { store, geometry, server, notifyResourceListChanged } = ctx
  server.registerTool(
    'openscad_customize_model',
    {
      title: 'Customize and save an OpenSCAD model',
      description:
        'Atomically apply validated top-level Customizer values to the current saved model and create a revision guarded by expected_revision.',
      inputSchema: z
        .object({
          model_id: idSchema,
          expected_revision: z.number().int().nonnegative(),
          values: z.record(z.string(), scalarSchema),
        })
        .strict(),
      outputSchema: toolOutputSchema(
        z.object({
          model: modelMetadataSchema,
          applied: z.array(z.string()),
          parameters: analysisSchema.shape.parameters,
          parameters_truncated: z.boolean(),
        }),
      ),
      annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false },
    },
    async input => {
      try {
        const model = await store.getModel(input.model_id)
        if (!model) throw new ModelNotFoundError(input.model_id)
        if (model.revision !== input.expected_revision) {
          throw new ModelRevisionConflictError(model.id, input.expected_revision, model.revision)
        }
        const result = geometry.customize(model.source, input.values as Record<string, CustomizerValue>)
        const saved = await store.saveModelSource({
          id: model.id,
          source: result.source,
          expectedRevision: input.expected_revision,
        })
        notifyResourceListChanged()
        return textResult({
          model: modelToWire(saved),
          applied: result.applied,
          parameters: result.parameters,
          parameters_truncated: result.parametersTruncated,
        })
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}
