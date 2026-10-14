import { ArtifactSizeError, DEFAULT_MAX_ARTIFACT_BYTES, GeometryBusyError, HeadlessGeometryService, MAX_ARTIFACT_BYTES, MAX_PENDING_GEOMETRY_JOBS } from '../geometryService'
import type { GeometryAnalysis } from '../geometryService'
import { CACHE_FIVE_MINUTES, CACHE_ONE_DAY, MCP_GEOMETRY_HOST_CONTRACT, OFFICIAL_LANGUAGE_RESOURCE_URI, OFFICIAL_LANGUAGE_SUMMARY, SERVER_INSTRUCTIONS, analysisSchema, buildSchema, catalogStatsSchema, comparisonErrorResultSchema, comparisonSuccessSchema, engineRegistrySchema, idSchema, independentArtifactSchema, independentCheckInputSchema, independentCheckSchema, independentExportInputSchema, independentExportSchema, modelMetadataSchema, officialArtifactSchema, officialCapabilitiesSchema, officialCheckSchema, officialExportSchema, officialFormatSchema, officialLanguageSummarySchema, promptGoalSchema, qualitySchema, scalarSchema, sha256Schema, sourceReferenceSchema, sourceSchema } from '../createServer'
import type { CreateOpenScadMcpServerOptions } from '../createServer'
import type { CustomizerValue } from '../../services/scadCustomizer'
import { DEFAULT_MAX_IN_FLIGHT_REQUESTS, DEFAULT_MAX_PENDING_CONTROL_MESSAGES, DEFAULT_MAX_PENDING_OUTBOUND_MESSAGES, MAX_MCP_STRING_REQUEST_ID_LENGTH, MAX_MCP_SUBSCRIPTIONS } from '../boundedTransport'
import { EXAMPLES, EXAMPLE_CATALOG } from '../../data/examples'
import { GEOMETRY_MANIFEST_ARCHIVE } from '../../core/geometryExecution'
import type { GeometryExecutionDescriptor } from '../../core/geometryExecution'
import { INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION, executeIndependentOpenScad, independentOpenScadMetrics } from '../independentOpenScadExecution'
import { IndependentArtifactCache, MAX_INDEPENDENT_ARTIFACT_CACHE_BYTES, MAX_INDEPENDENT_ARTIFACT_CACHE_ENTRIES, independentArtifactMimeType } from '../independentArtifactCache'
import { MAX_MODEL_NAME_LENGTH, MAX_MODEL_SOURCE_LENGTH, ModelRevisionConflictError, isWellFormedUnicode } from '../modelStore'
import type { StoredBuild, StoredModel, StoredModelRevision } from '../modelStore'
import { McpServer, ResourceNotFoundError, ResourceTemplate, SUPPORTED_PROTOCOL_VERSIONS } from '@modelcontextprotocol/server'
import type { RequestId } from '@modelcontextprotocol/server'
import { ModelNotFoundError } from '../publicError'
import { OPENSCAD_2021_01_CONTRACT } from '../../core/openScad2021Contract'
import { OPENSCAD_PROJECT_MAX_BLOB_BYTES, OPENSCAD_PROJECT_MAX_FILES, OPENSCAD_PROJECT_MAX_SOURCE_BYTES, OPENSCAD_PROJECT_MAX_TOTAL_BYTES } from '../../services/openScadProject'
import { OfficialArtifactCache } from '../officialArtifactCache'
import { OfficialOpenScadRuntimeSupervisor } from '../officialOpenScadRuntimeService'
import { analysisToWire, artifactUri, buildUri, engineRegistryToWire, independentArtifactToWire, independentArtifactUri, modelRevisionToWire, modelRevisionUri, modelToWire, modelUri, officialArtifactToWire, officialArtifactUri, officialCapabilitiesToWire, officialCheckToWire, officialExportToWire, officialRunOptions, officialToolInputSchema, officialToolOutputSchema, resourceId, resourceVariable, selectorSchema, sha256, toolOutputSchema } from './officialToolInputSchema'
import { assertGeometryManifestArchive, canonicalJson, immutableEngineManifestToWire, staticEngineManifestToWire } from '../engineManifest'
import { attachGeometryExecutionToError } from '../../services/geometryBuildEngine'
import { buildBinaryStl, buildObj } from '../../services/meshExport'
import { buildToWire, comparisonDelta, comparisonSide, errorResult, estimatedIndependentExportBytes, officialErrorResult, recordFailure, resolveSource, safeFileName, sameExecutionContext, textResult, throwIfIndependentExportAborted } from './persistedFailureStatus'
import { officialOpenScadMimeType } from '../officialOpenScadRuntimeProtocol'
import { z } from 'zod/v4'

export function createOpenScadMcpServer(options: CreateOpenScadMcpServerOptions): McpServer {
  assertGeometryManifestArchive()
  const { store } = options
  const geometry = options.geometry ?? new HeadlessGeometryService()
  const officialRuntime = options.officialRuntime ?? new OfficialOpenScadRuntimeSupervisor()
  const officialArtifacts = new OfficialArtifactCache()
  const independentArtifacts = new IndependentArtifactCache()
  const server = new McpServer(
    {
      name: 'open-scad-viewer',
      version: '0.1.0',
      description: 'Bounded local OpenSCAD analysis, customization, revision storage, and STL/OBJ export.',
    },
    {
      supportedProtocolVersions: ['2026-07-28', ...SUPPORTED_PROTOCOL_VERSIONS],
      instructions: [options.additionalInstructions, SERVER_INSTRUCTIONS].filter(Boolean).join('\n\n'),
      cacheHints: {
        'server/discover': { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
        'tools/list': { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
        'prompts/list': { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
        'resources/templates/list': { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
      },
    },
  )
  const notifyResourceListChanged = () => {
    void server.server.sendResourceListChanged().catch(error => {
      console.error('Could not send MCP resource-list notification:', error)
    })
  }

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

  server.registerResource('capabilities', 'openscad://capabilities', {
    title: 'OpenSCAD MCP capabilities',
    description: 'Machine-readable stable OpenSCAD inventory, official runtime, independent engine registry, source routes, safety limits, protocol support, and persistence boundaries.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
  }, async uri => {
    const [geometryCapabilities, officialCapabilities] = await Promise.all([
      geometry.capabilities(),
      officialRuntime.capabilities(),
    ])
    const geometryEngines = engineRegistryToWire(geometryCapabilities)
    return {
      contents: [{
        uri: uri.href,
        mimeType: 'application/json',
        text: JSON.stringify({
        server: { name: 'open-scad-viewer', version: '0.1.0' },
        protocol_versions: ['2026-07-28', ...SUPPORTED_PROTOCOL_VERSIONS],
        geometry_engines: geometryEngines,
        geometry_host: MCP_GEOMETRY_HOST_CONTRACT,
        parity_resource_uri: 'openscad://parity',
        official_language_contract: OFFICIAL_LANGUAGE_SUMMARY,
        official_runtime_resource_uri: 'openscad://official-runtime',
        official_runtime: officialCapabilitiesToWire(officialCapabilities),
        independent_runtime: {
          engine: INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION,
          tools: {
            check: 'openscad_independent_check',
            export: 'openscad_independent_export',
          },
          export_formats: ['stl', 'obj'],
          artifact_resource_template: 'openscad://independent-artifacts/{sha256}',
          defaults: { quality: 'full', time: 0 },
          limits: {
            source_bytes: OPENSCAD_PROJECT_MAX_SOURCE_BYTES,
            project_files: OPENSCAD_PROJECT_MAX_FILES,
            project_file_bytes: Math.max(
              OPENSCAD_PROJECT_MAX_SOURCE_BYTES,
              OPENSCAD_PROJECT_MAX_BLOB_BYTES,
            ),
            project_bytes: OPENSCAD_PROJECT_MAX_TOTAL_BYTES,
            artifact_bytes: MAX_ARTIFACT_BYTES,
            artifact_cache_entries: MAX_INDEPENDENT_ARTIFACT_CACHE_ENTRIES,
            artifact_cache_bytes: MAX_INDEPENDENT_ARTIFACT_CACHE_BYTES,
          },
          persistence: 'session-local-content-addressed',
          legacy_duckdb_build_schema_used: false,
        },
        compiler: {
          language_scope: 'strict OpenSCAD subset',
          supported: [
            'cube/sphere/cylinder/polyhedron',
            'square/circle/polygon',
            'union/difference/intersection/hull',
            'transforms/color',
            'modules/children/for/if/let',
            'linear_extrude/rotate_extrude/projection/offset',
            'top-level Customizer parameters',
          ],
          unsupported: [
            'include/use/import',
            'text/surface/Minkowski',
            'user-defined functions',
          ],
        },
        limits: {
          source_characters: MAX_MODEL_SOURCE_LENGTH,
          artifact_bytes: MAX_ARTIFACT_BYTES,
          pending_geometry_jobs: MAX_PENDING_GEOMETRY_JOBS,
          stdio_in_flight_requests: DEFAULT_MAX_IN_FLIGHT_REQUESTS,
          stdio_pending_output: DEFAULT_MAX_PENDING_OUTBOUND_MESSAGES,
          stdio_pending_overload_replies: DEFAULT_MAX_PENDING_CONTROL_MESSAGES,
          stdio_subscriptions: MAX_MCP_SUBSCRIPTIONS,
          stdio_string_request_id_characters: MAX_MCP_STRING_REQUEST_ID_LENGTH,
        },
        persistence: {
          catalog: 'DuckDB',
          arbitrary_sql: false,
          browser_indexeddb_synchronized: false,
        },
        }, null, 2),
      }],
    }
  })

  server.registerResource('official-runtime', 'openscad://official-runtime', {
    title: 'Official OpenSCAD runtime',
    description: 'Live availability, pinned integrity identity, isolation contract, supported exports, experiments, limits, and setup command for the opt-in upstream runtime.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
  }, async uri => ({
    contents: [{
      uri: uri.href,
      mimeType: 'application/json',
      text: JSON.stringify({
        language_contract: OFFICIAL_LANGUAGE_SUMMARY,
        official_runtime: officialCapabilitiesToWire(await officialRuntime.capabilities()),
      }, null, 2),
    }],
  }))

  server.registerResource('official-language-contract', OFFICIAL_LANGUAGE_RESOURCE_URI, {
    title: 'OpenSCAD 2021.01 stable language contract',
    description: 'Immutable source-derived inventory of stable built-ins, syntax, operators, variables, file semantics, compatibility tail, smoke fixtures, and the separately pinned execution runtime.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'public' },
  }, async uri => ({
    contents: [{
      uri: uri.href,
      mimeType: 'application/json',
      text: JSON.stringify({ contract: OPENSCAD_2021_01_CONTRACT }, null, 2),
    }],
  }))

  const engineManifestTemplate = new ResourceTemplate(
    'openscad://engines/{engine_class}/capabilities/{manifest_version}',
    {
      list: async () => ({
        resources: Object.values(GEOMETRY_MANIFEST_ARCHIVE).map(staticEngineManifestToWire).map(engine => ({
          uri: engine.manifest_resource_uri,
          name: `${engine.display_name} capability manifest`,
          description: `${engine.maturity} immutable manifest; permanent engine class`,
          mimeType: 'application/json',
        })),
      }),
    },
  )
  server.registerResource('geometry-engine-manifest', engineManifestTemplate, {
    title: 'Immutable geometry engine capability manifest',
    description: 'Versioned capabilities, deployment maturity, fingerprint, and isolation facts for one permanent engine class.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'public' },
  }, async (uri, variables) => {
    const engineClass = resourceVariable(uri, variables.engine_class)
    const manifestVersion = resourceVariable(uri, variables.manifest_version)
    if (engineClass !== 'mesh' && engineClass !== 'brep') {
      throw new ResourceNotFoundError(uri.href)
    }
    const manifest = immutableEngineManifestToWire(
      engineClass,
      manifestVersion,
    )
    if (!manifest) throw new ResourceNotFoundError(uri.href)
    return {
      contents: [{
        uri: uri.href,
        mimeType: 'application/json',
        text: JSON.stringify(manifest, null, 2),
      }],
    }
  })

  server.registerResource('geometry-parity', 'openscad://parity', {
    title: 'Geometry engine browser-to-MCP parity status',
    description: 'Honest qualification state by engine; cross-engine metric comparison is never proof of geometric equivalence.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
  }, async uri => {
    const registry = await geometry.capabilities()
    const policyPayload = {
      source_directed_routing: registry.sourceDirectedRouting,
      automatic_fallback: registry.automaticFallback,
      routes: registry.routes.map(route => ({
        language_contract: route.languageContract,
        engine_class: route.engineClass,
        fallback: route.fallback,
      })),
      manifests: registry.engines.map(engine => ({
        engine_class: engine.engineClass,
        capability_manifest_version: engine.capabilityManifestVersion,
        manifest_digest: engine.manifestDigest,
        kernel_fingerprint: engine.kernelFingerprint,
        limits: engine.limits,
      })),
      comparison: {
        mode: 'metrics-and-topology-only',
        cross_engine_geometric_equivalence: false,
        numeric_tolerances: null,
        comparator_qualification: 'pending',
      },
    }
    return {
      contents: [{
        uri: uri.href,
        mimeType: 'application/json',
        text: JSON.stringify({
        contract_version: 1,
        definition: 'Same source revision, language contract, engine class and fingerprint, policy, engine limits, and declared tolerances across browser and MCP. Mutable MCP host isolation is reported separately from immutable engine identity.',
        cross_engine_geometric_equivalence: false,
        mcp_host: MCP_GEOMETRY_HOST_CONTRACT,
        policy: {
          id: 'engine-routing-contract-v1',
          digest_algorithm: 'sha256',
          canonicalization: 'recursive-codepoint-key-sort-v1',
          payload: policyPayload,
          sha256: sha256(canonicalJson(policyPayload)),
        },
        corpus: {
          id: 'browser-mcp-engine-parity',
          version: null,
          sha256: null,
          result_sha256: null,
          qualified_at: null,
        },
        engines: registry.engines.map(engine => ({
          engine_class: engine.engineClass,
          capability_manifest_version: engine.capabilityManifestVersion,
          manifest_digest: engine.manifestDigest,
          kernel_fingerprint: engine.kernelFingerprint,
          runtime_availability: engine.availability,
          browser_target: engine.deployment === 'not-deployed' ? 'not-deployed' : 'wasm-worker',
          mcp_target: engine.deployment === 'not-deployed'
            ? 'not-deployed'
            : 'node-wasm-disposable-worker',
          browser_mcp_status: engine.deployment === 'not-deployed'
            ? 'not-deployed'
            : 'qualification-pending', // Provider qualification is not browser/MCP parity evidence.
          mcp_isolation: engine.isolation,
          mcp_provider_manifest_isolation: engine.isolation,
          mcp_host_isolation: engine.deployment === 'not-deployed'
            ? 'not-deployed'
            : MCP_GEOMETRY_HOST_CONTRACT.provider_lifecycle,
          mcp_host_contract_id: engine.deployment === 'not-deployed'
            ? null
            : MCP_GEOMETRY_HOST_CONTRACT.id,
          qualification: {
            status: engine.qualification.status,
            record_id: engine.qualification.recordId,
            corpus_version: engine.qualification.corpusVersion,
            target: engine.qualification.target,
          },
          result_sha256: null,
        })),
        }, null, 2),
      }],
    }
  })

  const modelTemplate = new ResourceTemplate('openscad://models/{id}', {
    list: async () => ({
      resources: (await store.listModels(500)).map(model => ({
        uri: modelUri(model.id),
        name: model.name,
        description: `OpenSCAD source, revision ${model.revision}`,
        mimeType: 'text/x-openscad',
      })),
    }),
    complete: { id: async value => (await store.listModels(500)).map(model => model.id).filter(id => id.startsWith(value)) },
  })
  server.registerResource('saved-model', modelTemplate, {
    title: 'Saved OpenSCAD model',
    description: 'Versioned OpenSCAD source stored in DuckDB.',
    mimeType: 'text/x-openscad',
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const model = await store.getModel(id)
    if (!model) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'text/x-openscad', text: model.source }] }
  })

  const modelRevisionTemplate = new ResourceTemplate('openscad://models/{id}/revisions/{revision}', {
    list: undefined,
  })
  server.registerResource('saved-model-revision', modelRevisionTemplate, {
    title: 'Saved OpenSCAD model revision',
    description: 'An immutable OpenSCAD source snapshot stored in DuckDB.',
    mimeType: 'text/x-openscad',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const revisionValue = resourceVariable(uri, variables.revision)
    if (!/^\d+$/.test(revisionValue)) throw new ResourceNotFoundError(uri.href)
    const revision = Number(revisionValue)
    if (!Number.isSafeInteger(revision)) throw new ResourceNotFoundError(uri.href)
    const model = await store.getModelRevision(id, revision)
    if (!model) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'text/x-openscad', text: model.source }] }
  })

  const buildTemplate = new ResourceTemplate('openscad://builds/{id}', {
    list: async () => ({
      resources: (await store.listBuilds({ limit: 500 })).map(build => ({
        uri: buildUri(build.id),
        name: `Build ${build.id}`,
        description: `${build.status} ${build.quality} build`,
        mimeType: 'application/json',
      })),
    }),
  })
  server.registerResource('build-result', buildTemplate, {
    title: 'OpenSCAD build result',
    description: 'A compact build result recorded in DuckDB.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const build = await store.getBuild(id)
    if (!build) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'application/json', text: JSON.stringify(buildToWire(build), null, 2) }] }
  })

  const artifactTemplate = new ResourceTemplate('openscad://artifacts/{id}', {
    list: async () => ({
      resources: (await store.listArtifacts(100)).map(artifact => ({
        uri: artifactUri(artifact.id),
        name: artifact.fileName,
        description: `${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString()} bytes)`,
        mimeType: artifact.mimeType,
      })),
    }),
  })
  server.registerResource('export-artifact', artifactTemplate, {
    title: 'OpenSCAD export artifact',
    description: 'A bounded STL or OBJ artifact stored in DuckDB.',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const artifact = await store.getArtifact(id)
    if (!artifact) throw new ResourceNotFoundError(uri.href)
    return artifact.format === 'obj'
      ? { contents: [{ uri: uri.href, mimeType: artifact.mimeType, text: new TextDecoder().decode(artifact.data) }] }
      : { contents: [{ uri: uri.href, mimeType: artifact.mimeType, blob: Buffer.from(artifact.data).toString('base64') }] }
  })

  const independentArtifactTemplate = new ResourceTemplate(
    'openscad://independent-artifacts/{sha256}',
    {
      list: async () => ({
        resources: independentArtifacts.list().map(artifact => ({
          uri: independentArtifactUri(artifact.id),
          name: artifact.fileName,
          description: `Independent OpenSCAD ${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString()} bytes); session-local`,
          mimeType: artifact.mimeType,
        })),
      }),
      complete: {
        sha256: value => independentArtifacts.list().map(artifact => artifact.id)
          .filter(id => id.startsWith(value)),
      },
    },
  )
  server.registerResource('independent-export-artifact', independentArtifactTemplate, {
    title: 'Independent OpenSCAD export artifact',
    description: 'A bounded content-addressed STL or OBJ produced by the repository-owned engine and retained only for this MCP server session.',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceVariable(uri, variables.sha256)
    if (!/^[a-f0-9]{64}$/.test(id)) throw new ResourceNotFoundError(uri.href)
    const artifact = independentArtifacts.get(id)
    if (!artifact || artifact.sha256 !== id || sha256(artifact.data) !== id) {
      throw new ResourceNotFoundError(uri.href)
    }
    return artifact.format === 'obj'
      ? {
          contents: [{
            uri: uri.href,
            mimeType: artifact.mimeType,
            text: new TextDecoder().decode(artifact.data),
          }],
        }
      : {
          contents: [{
            uri: uri.href,
            mimeType: artifact.mimeType,
            blob: Buffer.from(artifact.data).toString('base64'),
          }],
        }
  })

  const officialArtifactTemplate = new ResourceTemplate('openscad://official-artifacts/{id}', {
    list: async () => ({
      resources: officialArtifacts.list().map(artifact => ({
        uri: officialArtifactUri(artifact.id),
        name: artifact.fileName,
        description: `Official OpenSCAD ${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString()} bytes); session-local`,
        mimeType: artifact.mimeType,
      })),
    }),
    complete: {
      id: value => officialArtifacts.list().map(artifact => artifact.id)
        .filter(id => id.startsWith(value)),
    },
  })
  server.registerResource('official-export-artifact', officialArtifactTemplate, {
    title: 'Official OpenSCAD export artifact',
    description: 'A bounded content-addressed upstream-runtime export retained only for this MCP server session.',
  }, async (uri, variables) => {
    const id = resourceVariable(uri, variables.id)
    if (!/^[a-f0-9]{64}$/.test(id)) throw new ResourceNotFoundError(uri.href)
    const artifact = officialArtifacts.get(id)
    if (!artifact) throw new ResourceNotFoundError(uri.href)
    return {
      contents: [{
        uri: uri.href,
        mimeType: artifact.mimeType,
        blob: Buffer.from(artifact.data).toString('base64'),
      }],
    }
  })

  const exampleNames = EXAMPLE_CATALOG.map(example => example.id).sort()
  const examplesById = new Map(EXAMPLE_CATALOG.map(example => [example.id, example]))
  const exampleTemplate = new ResourceTemplate('openscad://examples/{name}', {
    list: async () => ({
      resources: exampleNames.map(name => ({
        uri: `openscad://examples/${name}`,
        name: `${name}.scad`,
        description: examplesById.get(name)?.description.en ?? `Bundled OpenSCAD example: ${name}`,
        mimeType: 'text/x-openscad',
      })),
    }),
    complete: { name: value => exampleNames.filter(name => name.startsWith(value)) },
  })
  server.registerResource('example', exampleTemplate, {
    title: 'Bundled OpenSCAD example',
    description: 'An example from the browser workspace.',
    mimeType: 'text/x-openscad',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'public' },
  }, async (uri, variables) => {
    const name = resourceVariable(uri, variables.name)
    const source = EXAMPLES[name]
    if (source === undefined) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'text/x-openscad', text: source }] }
  })

  if (options.onRequestSettled) {
    type DispatchHandler = (...args: unknown[]) => unknown
    const protocolServer = server.server as unknown as {
      setRequestHandler: (...args: unknown[]) => void
      _requestHandlers: Map<string, DispatchHandler>
    }
    const wrappedHandlers = new WeakSet<DispatchHandler>()
    const wrapDispatchHandler = (method: string) => {
      const handler = protocolServer._requestHandlers.get(method)
      if (!handler || wrappedHandlers.has(handler)) return
      const wrapped: DispatchHandler = async (...args: unknown[]) => {
        const context = args.at(-1) as {
          mcpReq?: { id?: RequestId; signal?: AbortSignal }
        } | undefined
        const requestId = context?.mcpReq?.id
        const signal = context?.mcpReq?.signal
        if (requestId !== undefined && signal) {
          const cancelled = () => options.onRequestCancelled?.(requestId)
          if (signal.aborted) cancelled()
          else signal.addEventListener('abort', cancelled, { once: true })
        }
        try {
          return await handler(...args)
        } finally {
          if (requestId !== undefined) options.onRequestSettled?.(requestId)
        }
      }
      wrappedHandlers.add(wrapped)
      protocolServer._requestHandlers.set(method, wrapped)
    }

    // MCP SDK 2.0 has no public outer-dispatch middleware. Wrap finalized
    // registry entries so lifecycle covers SDK validation and application
    // callbacks. serveStdio installs a fresh modern discover handler after the
    // factory returns, so future registrations must pass through the same seam.
    const originalSetRequestHandler = protocolServer.setRequestHandler.bind(protocolServer)
    protocolServer.setRequestHandler = (...registration: unknown[]) => {
      originalSetRequestHandler(...registration)
      if (typeof registration[0] === 'string') wrapDispatchHandler(registration[0])
    }
    for (const method of protocolServer._requestHandlers.keys()) wrapDispatchHandler(method)
  }

  return server
}
