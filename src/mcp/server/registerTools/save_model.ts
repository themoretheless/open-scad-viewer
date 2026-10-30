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
export function register_save_model(deps: McpToolRegistrationDeps): void {

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
      const { runDirectTransaction } = await import('../../directTransaction')
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
      const { BREP_CAPABILITY_MATRIX } = await import('../../../services/geometry/brepCapability')
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
}
