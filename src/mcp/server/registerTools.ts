import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import type {
  GeometryBuildPurpose,
  GeometryExecutionDescriptor,
} from '../../core/geometryExecution'
import {
  attachGeometryExecutionToError,
  GeometryLanguageContractError,
} from '../../services/geometryBuildEngine'
import type { CustomizerValue } from '../../services/scadCustomizer'
import { buildBinaryStl, buildObj } from '../../services/meshExport'
import {
  ArtifactSizeError,
  DEFAULT_MAX_ARTIFACT_BYTES,
  GeometryBusyError,
  MAX_ARTIFACT_BYTES,
  type GeometryAnalysis,
  type McpGeometryService,
} from '../geometryService'
import {
  type IndependentArtifactCache,
  independentArtifactMimeType,
} from '../independentArtifactCache'
import {
  executeIndependentOpenScad,
  INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION,
  independentOpenScadMetrics,
} from '../independentOpenScadExecution'
import {
  MAX_MODEL_NAME_LENGTH,
  ModelRevisionConflictError,
  isWellFormedUnicode,
  type ModelStore,
  type StoredBuild,
  type StoredModel,
  type StoredModelRevision,
} from '../modelStore'
import {
  buildDiagnostic,
  isAbortError,
  ModelNotFoundError,
} from '../publicError'
import type { OfficialArtifactCache } from '../officialArtifactCache'
import type { OfficialOpenScadRuntimeService } from '../officialOpenScadRuntimeService'
import { officialOpenScadMimeType } from '../officialOpenScadRuntimeProtocol'
import {
  artifactUri,
  modelRevisionUri,
  OFFICIAL_LANGUAGE_SUMMARY,
  sha256,
} from './contracts'
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
} from './schemas'
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
} from './wireMappers'

export interface McpToolRegistrationDeps {
  server: McpServer
  store: ModelStore
  geometry: McpGeometryService
  officialRuntime: OfficialOpenScadRuntimeService
  officialArtifacts: OfficialArtifactCache
  independentArtifacts: IndependentArtifactCache
  notifyResourceListChanged: () => void
}

/** Persisted causality depends on the settled error, never a later signal state. */
export function persistedFailureStatus(error: unknown): 'cancelled' | 'failed' {
  return isAbortError(error) ? 'cancelled' : 'failed'
}

export async function recordFailure(
  store: ModelStore,
  geometry: McpGeometryService,
  input: {
    model: Pick<SelectedModel, 'id' | 'revision'> | null
    source: string
    quality: 'preview' | 'full'
    purpose: GeometryBuildPurpose
    durationMs: number
    error: unknown
    execution?: GeometryExecutionDescriptor
  },
): Promise<StoredBuild | null> {
  try {
    if (input.error instanceof GeometryLanguageContractError) return null
    const execution = input.execution
      ?? executionForFailure(input.error)
      ?? geometry.planSource(input.source, input.quality, input.purpose)
    return await store.recordBuild({
      modelId: input.model?.id,
      modelRevision: input.model?.revision,
      source: input.source,
      sourceSha256: sha256(input.source),
      execution,
      quality: input.quality,
      durationMs: input.durationMs,
      warnings: [],
      status: persistedFailureStatus(input.error),
      error: buildDiagnostic(input.error),
    })
  } catch (storeError) {
    console.error('Could not record failed MCP build:', storeError)
    return null
  }
}

export function registerMcpCatalogAndEngineTools(deps: McpToolRegistrationDeps): void {
  const {
    server,
    store,
    geometry,
    officialRuntime,
    officialArtifacts,
    independentArtifacts,
    notifyResourceListChanged,
  } = deps

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

  server.registerTool('openscad_direct_transaction', {
    title: 'Apply an atomic direct-model action batch',
    description: 'Execute a version 1 JSON action script (1–80 primitive, sceneEdit, pattern, sketchCleanup, outlineStroke, extrusion, documentEdit or checkpoint actions) on a direct document. Returns a complete document only if every action succeeds. The UI imports or replays the result with one Undo. This does not modify a live UI or saved catalog model.',
    inputSchema: z.object({ document: z.string().max(64 * 1024 * 1024), script: z.string().max(16 * 1024 * 1024) }).strict(),
    outputSchema: toolOutputSchema(z.object({ document: z.string() })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async ({ document, script }, context) => {
    try {
      const { runDirectTransaction } = await import('../directTransaction')
      return textResult({ document: await runDirectTransaction(document, script, context.mcpReq.signal) })
    } catch (error) { return errorResult(error) }
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

  server.registerTool('openscad_list_engines', {
    title: 'List geometry engines',
    description: 'Return both permanent geometry engine classes, their source-language routes, runtime availability, manifests, and the no-fallback policy.',
    inputSchema: z.object({}),
    outputSchema: toolOutputSchema(z.object({ geometry_engines: engineRegistrySchema })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async () => {
    try {
      return textResult({
        geometry_engines: engineRegistryToWire(await geometry.capabilities()),
      })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_brep_capabilities', {
    title: 'List B-rep capability maturity',
    description: 'Return the honest B-rep/NURBS capability maturity matrix. Unavailable and ResearchOnly entries never claim product readiness. B-rep failure never falls back to Manifold.',
    inputSchema: z.object({}).strict(),
    outputSchema: toolOutputSchema(z.object({
      cross_engine_fallback: z.literal(false),
      capabilities: z.array(z.object({
        id: z.string(),
        maturity: z.enum(['Unavailable', 'ResearchOnly', 'NumericUncertified', 'AnalyticComplete', 'Qualified']),
        permits_topology_change: z.boolean(),
        notes: z.string(),
      })),
    }).strict()),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async () => {
    try {
      const { BREP_CAPABILITY_MATRIX } = await import('../../services/geometry/brepCapability')
      return textResult({
        cross_engine_fallback: false as const,
        capabilities: BREP_CAPABILITY_MATRIX.map(entry => ({
          id: entry.id,
          maturity: entry.maturity,
          permits_topology_change: entry.permitsTopologyChange,
          notes: entry.notes,
        })),
      })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_official_status', {
    title: 'Upstream OpenSCAD oracle status',
    description: 'Report availability and integrity of the optional upstream qualification oracle. This provider is not the independent production engine or a fallback.',
    inputSchema: z.object({}).strict(),
    outputSchema: officialToolOutputSchema(z.object({
      language_contract: officialLanguageSummarySchema,
      official_runtime: officialCapabilitiesSchema,
    }).strict()),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async () => {
    try {
      return textResult({
        language_contract: OFFICIAL_LANGUAGE_SUMMARY,
        official_runtime: officialCapabilitiesToWire(await officialRuntime.capabilities()),
      })
    } catch (error) {
      return officialErrorResult(error)
    }
  })

  server.registerTool('openscad_official_check', {
    title: 'Check with the upstream OpenSCAD oracle',
    description: 'Run a bounded source project in the pinned upstream qualification oracle for differential comparison; this does not execute or attest the independent engine.',
    inputSchema: officialToolInputSchema({}),
    outputSchema: officialToolOutputSchema(z.object({ check: officialCheckSchema }).strict()),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async (input, context) => {
    try {
      const result = await officialRuntime.check(
        input.source,
        officialRunOptions(input, context.mcpReq.signal),
      )
      return textResult({ check: officialCheckToWire(result) })
    } catch (error) {
      return officialErrorResult(error)
    }
  })

  server.registerTool('openscad_official_export', {
    title: 'Reference export with the upstream OpenSCAD oracle',
    description: 'Render a bounded source project in the pinned upstream oracle and expose a reference artifact; this is not an independent-engine export.',
    inputSchema: officialToolInputSchema({ format: officialFormatSchema }),
    outputSchema: officialToolOutputSchema(z.object({
      export: officialExportSchema,
      artifact: officialArtifactSchema,
    }).strict()),
    annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async (input, context) => {
    try {
      const exported = await officialRuntime.export(
        input.source,
        input.format,
        officialRunOptions(input, context.mcpReq.signal),
      )
      if (sha256(exported.data) !== exported.sha256
        || exported.format !== input.format
        || exported.mimeType !== officialOpenScadMimeType(input.format)
        || exported.fileName !== `model.${input.format}`) {
        throw new Error('Official OpenSCAD export failed its MCP integrity boundary')
      }
      const artifact = officialArtifacts.put({
        format: exported.format,
        fileName: exported.fileName,
        mimeType: exported.mimeType,
        data: exported.data,
      })
      notifyResourceListChanged()
      const artifactWire = officialArtifactToWire(artifact, exported.format)
      const structured = {
        export: officialExportToWire(exported),
        artifact: artifactWire,
      }
      const result = textResult(structured)
      return {
        ...result,
        content: [
          result.content[0],
          {
            type: 'resource_link' as const,
            uri: artifactWire.resource_uri,
            name: artifact.fileName,
            mimeType: artifact.mimeType,
            description: `Official OpenSCAD ${exported.format.toUpperCase()} export (${artifact.byteLength.toLocaleString()} bytes)`,
          },
        ],
      }
    } catch (error) {
      return officialErrorResult(error)
    }
  })

  server.registerTool('openscad_independent_check', {
    title: 'Check with the independent OpenSCAD engine',
    description: 'Execute an inline source or bounded include/use project with the project-owned OpenSCAD 2021.01 frontend and geometry pipeline, without invoking the upstream OpenSCAD runtime. This remains a development surface, not a complete-language production claim.',
    inputSchema: independentCheckInputSchema,
    outputSchema: toolOutputSchema(z.object({ check: independentCheckSchema }).strict()),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: false, openWorldHint: false },
  }, async (input, context) => {
    try {
      const run = await executeIndependentOpenScad({
        source: input.source,
        files: input.files,
        quality: input.quality,
        time: input.time,
        signal: context.mcpReq.signal,
      })
      return textResult({
        check: {
          engine: INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION,
          source_sha256: sha256(input.source),
          quality: run.result.quality,
          duration_ms: run.durationMs,
          ...independentOpenScadMetrics(run.result),
          warnings: run.result.warnings,
        },
      })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_independent_export', {
    title: 'Export with the independent OpenSCAD engine',
    description: 'Render an inline source or bounded VFS project at full quality with only the repository-owned OpenSCAD 2021.01 engine, then expose a bounded session-local STL/OBJ resource. No upstream runtime or legacy DuckDB build record is used.',
    inputSchema: independentExportInputSchema,
    outputSchema: toolOutputSchema(z.object({
      export: independentExportSchema,
      artifact: independentArtifactSchema,
    }).strict()),
    annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async (input, context) => {
    const startedAt = performance.now()
    try {
      const run = await executeIndependentOpenScad({
        source: input.source,
        files: input.files,
        quality: 'full',
        time: input.time,
        signal: context.mcpReq.signal,
      })
      throwIfIndependentExportAborted(context.mcpReq.signal)
      const estimatedBytes = estimatedIndependentExportBytes(run.result.meshes, input.format)
      if (estimatedBytes > input.max_bytes) {
        throw new ArtifactSizeError(estimatedBytes, input.max_bytes)
      }

      const fileName = safeFileName(input.file_name ?? 'model', input.format)
      const mimeType = independentArtifactMimeType(input.format)
      const data = input.format === 'stl'
        ? buildBinaryStl(run.result.meshes, fileName)
        : new TextEncoder().encode(buildObj(run.result.meshes))
      throwIfIndependentExportAborted(context.mcpReq.signal)
      if (data.byteLength > input.max_bytes) {
        throw new ArtifactSizeError(data.byteLength, input.max_bytes)
      }

      const dataDigest = sha256(data)
      const artifact = independentArtifacts.put({
        format: input.format,
        fileName,
        mimeType,
        data,
      })
      if (artifact.id !== dataDigest
        || artifact.sha256 !== dataDigest
        || artifact.byteLength !== data.byteLength
        || artifact.format !== input.format
        || artifact.mimeType !== mimeType
        || artifact.fileName !== fileName) {
        throw new Error('Independent OpenSCAD export failed its MCP integrity boundary')
      }
      notifyResourceListChanged()
      const artifactWire = independentArtifactToWire(artifact)
      const structured = {
        export: {
          engine: INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION,
          source_sha256: sha256(input.source),
          quality: 'full' as const,
          format: input.format,
          duration_ms: Math.max(0, performance.now() - startedAt),
          metrics: independentOpenScadMetrics(run.result),
          warnings: run.result.warnings,
        },
        artifact: artifactWire,
      }
      const result = textResult(structured)
      return {
        ...result,
        content: [
          result.content[0],
          {
            type: 'resource_link' as const,
            uri: artifactWire.resource_uri,
            name: artifact.fileName,
            mimeType: artifact.mimeType,
            description: `Independent OpenSCAD ${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString()} bytes); session-local`,
          },
        ],
      }
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_list_model_revisions', {
    title: 'List OpenSCAD model revisions',
    description: 'List immutable source snapshots for one saved DuckDB model.',
    inputSchema: z.object({
      id: idSchema,
      limit: z.number().int().min(1).max(500).default(100),
    }),
    outputSchema: toolOutputSchema(z.object({
      revisions: z.array(z.object({
        id: z.string(),
        name: z.string(),
        revision: z.number().int().nonnegative(),
        created_at: z.string(),
        source_length: z.number().int().nonnegative(),
        resource_uri: z.string(),
      })),
    })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async ({ id, limit }) => {
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
  })

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
