import type { McpServer } from '@modelcontextprotocol/server'

import { z } from 'zod/v4'

import type {
  GeometryBuildPurpose,
  GeometryExecutionDescriptor,
} from '../../../core/geometryExecution'

import {
  attachGeometryExecutionToError,
  GeometryLanguageContractError,
} from '../../../services/geometryBuildEngine'

import type { CustomizerValue } from '../../../services/scadCustomizer'

import { buildBinaryStl, buildObj } from '../../../services/meshExport'

import {
  ArtifactSizeError,
  DEFAULT_MAX_ARTIFACT_BYTES,
  GeometryBusyError,
  MAX_ARTIFACT_BYTES,
  type GeometryAnalysis,
  type McpGeometryService,
} from '../../geometryService'

import {
  type IndependentArtifactCache,
  independentArtifactMimeType,
} from '../../independentArtifactCache'

import {
  executeIndependentOpenScad,
  INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION,
  independentOpenScadMetrics,
} from '../../independentOpenScadExecution'

import {
  MAX_MODEL_NAME_LENGTH,
  ModelRevisionConflictError,
  isWellFormedUnicode,
  type ModelStore,
  type StoredBuild,
  type StoredModel,
  type StoredModelRevision,
} from '../../modelStore'

import {
  buildDiagnostic,
  isAbortError,
  ModelNotFoundError,
} from '../../publicError'

import type { OfficialArtifactCache } from '../../officialArtifactCache'

import type { OfficialOpenScadRuntimeService } from '../../officialOpenScadRuntimeService'

import { officialOpenScadMimeType } from '../../officialOpenScadRuntimeProtocol'

import {
  artifactUri,
  modelRevisionUri,
  OFFICIAL_LANGUAGE_SUMMARY,
  sha256,
} from '.././contracts'

import {
  analysisSchema,
  buildSchema,
  catalogStatsSchema,
  comparisonErrorResultSchema,
  comparisonSuccessSchema,
  engineRegistrySchema,
  idSchema,
  independentArtifactSchema,
  independentCheckInputSchema,
  independentCheckSchema,
  independentExportInputSchema,
  independentExportSchema,
  modelMetadataSchema,
  officialArtifactSchema,
  officialCapabilitiesSchema,
  officialCheckSchema,
  officialExportSchema,
  officialFormatSchema,
  officialLanguageSummarySchema,
  officialToolInputSchema,
  officialToolOutputSchema,
  qualitySchema,
  scalarSchema,
  selectorSchema,
  sha256Schema,
  sourceReferenceSchema,
  sourceSchema,
  toolOutputSchema,
} from '.././schemas'

import {
  analysisToWire,
  buildToWire,
  comparisonDelta,
  comparisonSide,
  engineRegistryToWire,
  errorResult,
  estimatedIndependentExportBytes,
  executionForFailure,
  independentArtifactToWire,
  modelRevisionToWire,
  modelToWire,
  officialArtifactToWire,
  officialCapabilitiesToWire,
  officialCheckToWire,
  officialErrorResult,
  officialExportToWire,
  officialRunOptions,
  resolveSource,
  safeFileName,
  sameExecutionContext,
  textResult,
  throwIfIndependentExportAborted,
  type SelectedModel,
} from '.././wireMappers'
import {recordFailure, persistedFailureStatus} from '../registerTools'
import type {McpToolRegistrationDeps} from '../registerTools'
export function register_check(deps: McpToolRegistrationDeps): void {

  const {
    server,
    store,
    geometry,
    officialRuntime,
    officialArtifacts,
    independentArtifacts,
    notifyResourceListChanged,
  } = deps


  server.registerTool('openscad_check', {
    title: 'Check OpenSCAD geometry',
    description: 'Compile inline source or an exact saved-model revision without writing build history. Use preview for fast iteration and full for authoritative metrics.',
    inputSchema: selectorSchema({ quality: qualitySchema.default('preview') }),
    outputSchema: toolOutputSchema(z.object({
      source_sha256: sha256Schema,
      model_id: idSchema.nullable(),
      model_revision: z.number().int().nonnegative().nullable(),
      analysis: analysisSchema,
    })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async (input, context) => {
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
  })


  server.registerTool('openscad_compare', {
    title: 'Compare OpenSCAD geometry',
    description: 'Compile two inline or saved-revision sources without persistence and return bounded metric, dimension, and topology deltas. This is not an exact geometric boolean diff.',
    inputSchema: z.object({
      left: sourceReferenceSchema,
      right: sourceReferenceSchema,
      quality: qualitySchema.default('full'),
    }).strict(),
    outputSchema: z.union([comparisonSuccessSchema, comparisonErrorResultSchema]),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async ({ left, right, quality }, context) => {
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
      rightAnalysis = leftSource.source === rightSource.source
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
          ? 'same_engine_metrics' as const
          : 'cross_engine_metrics_only' as const,
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
  })


  server.registerTool('openscad_analyze', {
    title: 'Analyze OpenSCAD geometry',
    description: 'Compile OpenSCAD source or a saved model and return bounded geometry, topology, provenance, and Customizer facts. The build is recorded in DuckDB.',
    inputSchema: selectorSchema({ quality: qualitySchema.default('full') }),
    outputSchema: toolOutputSchema(z.object({ build: buildSchema, analysis: analysisSchema })),
    annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false },
  }, async (input, context) => {
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
  })


  server.registerTool('openscad_customize', {
    title: 'Customize OpenSCAD source',
    description: 'Apply validated values to top-level OpenSCAD Customizer parameters and return updated source without saving it.',
    inputSchema: selectorSchema({ values: z.record(z.string(), scalarSchema) }),
    outputSchema: toolOutputSchema(z.object({
      source: z.string(),
      applied: z.array(z.string()),
      parameters: analysisSchema.shape.parameters,
      parameters_truncated: z.boolean(),
    })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async input => {
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
  })
}
