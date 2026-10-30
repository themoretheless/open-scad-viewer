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
export function register_customize_model(deps: McpToolRegistrationDeps): void {

  const {
    server,
    store,
    geometry,
    officialRuntime,
    officialArtifacts,
    independentArtifacts,
    notifyResourceListChanged,
  } = deps


  server.registerTool('openscad_customize_model', {
    title: 'Customize and save an OpenSCAD model',
    description: 'Atomically apply validated top-level Customizer values to the current saved model and create a revision guarded by expected_revision.',
    inputSchema: z.object({
      model_id: idSchema,
      expected_revision: z.number().int().nonnegative(),
      values: z.record(z.string(), scalarSchema),
    }).strict(),
    outputSchema: toolOutputSchema(z.object({
      model: modelMetadataSchema,
      applied: z.array(z.string()),
      parameters: analysisSchema.shape.parameters,
      parameters_truncated: z.boolean(),
    })),
    annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false },
  }, async input => {
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
  })


  server.registerTool('openscad_export', {
    title: 'Export OpenSCAD geometry',
    description: 'Compile a full-quality model, store a bounded STL or OBJ artifact in DuckDB, and return an MCP resource link.',
    inputSchema: selectorSchema({
      format: z.enum(['stl', 'obj']),
      file_name: z.string().min(1).max(255)
        .refine(isWellFormedUnicode, 'File name must contain well-formed Unicode').optional(),
      max_bytes: z.number().int().positive().max(MAX_ARTIFACT_BYTES).default(DEFAULT_MAX_ARTIFACT_BYTES),
    }),
    outputSchema: toolOutputSchema(z.object({
      build: buildSchema,
      analysis: analysisSchema,
      artifact: z.object({
        id: z.string(),
        resource_uri: z.string(),
        file_name: z.string(),
        format: z.enum(['stl', 'obj']),
        mime_type: z.string(),
        sha256: z.string(),
        byte_length: z.number().int().nonnegative(),
      }),
    })),
    annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false },
  }, async (input, context) => {
    const startedAt = performance.now()
    let completedBuild: StoredBuild | null = null
    let completedExecution: GeometryExecutionDescriptor | undefined
    let selected: Awaited<ReturnType<typeof resolveSource>>
    try {
      selected = await resolveSource(store, input)
    } catch (error) {
      return errorResult(error)
    }
    try {
      const exported = await geometry.export(selected.source, input.format, {
        maxBytes: input.max_bytes,
        signal: context.mcpReq.signal,
        name: input.file_name ?? selected.model?.name,
      })
      const analysis = exported.analysis
      completedExecution = analysis.execution
      let build: StoredBuild
      try {
        build = await store.recordBuild({
          modelId: selected.model?.id,
          modelRevision: selected.model?.revision,
          source: selected.source,
          sourceSha256: sha256(selected.source),
          execution: analysis.execution,
          quality: 'full',
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
      completedBuild = build
      const fileName = safeFileName(input.file_name ?? selected.model?.name ?? 'model', input.format)
      const artifact = await store.storeArtifact({
        buildId: build.id,
        modelId: selected.model?.id,
        format: input.format,
        fileName,
        mimeType: exported.mimeType,
        sha256: sha256(exported.data),
        data: exported.data,
      })
      notifyResourceListChanged()
      const structured = {
        build: buildToWire(build),
        analysis: analysisToWire(analysis),
        artifact: {
          id: artifact.id,
          resource_uri: artifactUri(artifact.id),
          file_name: artifact.fileName,
          format: artifact.format,
          mime_type: artifact.mimeType,
          sha256: artifact.sha256,
          byte_length: artifact.byteLength,
        },
      }
      const result = textResult(structured)
      return {
        ...result,
        content: [
          result.content[0],
          {
            type: 'resource_link' as const,
            uri: artifactUri(artifact.id),
            name: artifact.fileName,
            mimeType: artifact.mimeType,
            description: `${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString()} bytes)`,
          },
        ],
      }
    } catch (error) {
      if (completedBuild) {
        notifyResourceListChanged()
        return errorResult(error, { build: buildToWire(completedBuild) })
      }
      if (error instanceof GeometryBusyError) return errorResult(error)
      const build = await recordFailure(store, geometry, {
        ...selected,
        quality: 'full',
        purpose: 'export',
        durationMs: performance.now() - startedAt,
        error,
        execution: completedExecution,
      })
      if (build) notifyResourceListChanged()
      return errorResult(error, build ? { build: buildToWire(build) } : {})
    }
  })


  server.registerTool('openscad_build_history', {
    title: 'OpenSCAD build history',
    description: 'List recent MCP build results recorded in DuckDB, optionally for one saved model.',
    inputSchema: z.object({
      model_id: idSchema.optional(),
      limit: z.number().int().min(1).max(100).default(100),
    }),
    outputSchema: toolOutputSchema(z.object({ builds: z.array(buildSchema) })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async input => {
    try {
      const builds = await store.listBuilds({ modelId: input.model_id, limit: input.limit })
      return textResult({ builds: builds.map(buildToWire) })
    } catch (error) {
      return errorResult(error)
    }
  })
}
