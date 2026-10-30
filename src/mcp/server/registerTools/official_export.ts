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
export function register_official_export(deps: McpToolRegistrationDeps): void {

  const {
    server,
    store,
    geometry,
    officialRuntime,
    officialArtifacts,
    independentArtifacts,
    notifyResourceListChanged,
  } = deps


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
}
