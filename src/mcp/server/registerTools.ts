import {register_save_model} from './registerTools/save_model'
import {register_official_export} from './registerTools/official_export'
import {register_check} from './registerTools/check'
import {register_customize_model} from './registerTools/customize_model'
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
  register_save_model(deps)
  register_official_export(deps)
  register_check(deps)
  register_customize_model(deps)
}
