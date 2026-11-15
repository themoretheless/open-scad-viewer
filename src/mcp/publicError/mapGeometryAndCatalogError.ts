import { HandoffBundleValidationError, HandoffConflictError } from '../../core/handoffBundle'
import { SweepBudgetError } from '../parameterSweep'
import {
  ArtifactSizeError,
  CustomizerValueError,
  GeometryBusyError,
  GeometryDeadlineExceededError,
  InvalidGeometryError,
} from '../geometryService'
import { CatalogQuotaError, ModelRevisionConflictError } from '../modelStore'
import {
  GeometryCapabilityUnavailableError,
  GeometryEngineUnavailableError,
  GeometryLanguageContractError,
} from '../../kernel/geometryBuildEngine'
import { ModelNotFoundError, type PublicToolError } from '../publicError'
import { isAbortError, isQuarantinedGeometryHostError } from './isAbortError'

function mapCatalogError(error: unknown): { error: PublicToolError; internal: boolean } | null {
  if (error instanceof ModelNotFoundError) {
    return {
      error: {
        code: 'model_not_found',
        message: error.message,
        retryable: false,
        next_action: 'List saved models or provide a valid inline source.',
        details: { model_id: error.modelId, revision: error.revision ?? null },
      },
      internal: false,
    }
  }
  if (error instanceof ModelRevisionConflictError) {
    return {
      error: {
        code: 'revision_conflict',
        message: error.message,
        retryable: true,
        next_action: 'Read the latest model revision, reconcile the source, and retry with that expected_revision.',
        details: {
          model_id: error.modelId,
          expected_revision: error.expectedRevision,
          actual_revision: error.actualRevision,
        },
      },
      internal: false,
    }
  }
  if (error instanceof HandoffConflictError) {
    return {
      error: {
        code: 'revision_conflict',
        message: error.message,
        retryable: true,
        next_action: 'Read the target model revision, reconcile any edits, and retry with matching expected_revision.',
        details: {
          model_id: error.modelId,
          expected_revision: error.expectedRevision,
          actual_revision: error.actualRevision,
        },
      },
      internal: false,
    }
  }
  if (error instanceof CatalogQuotaError) {
    return {
      error: {
        code: 'quota_exceeded',
        message: error.message,
        retryable: false,
        next_action: 'Inspect openscad_catalog_stats and archive or remove catalog data before retrying.',
        details: { quota: error.quota, limit: error.limit },
      },
      internal: false,
    }
  }
  return null
}

function mapGeometryEngineError(error: unknown): { error: PublicToolError; internal: boolean } | null {
  if (error instanceof GeometryLanguageContractError) {
    return {
      error: {
        code: 'language_contract_unsupported',
        message: error.message,
        retryable: false,
        next_action: 'Use a supported leading @language contract and do not select an engine directly.',
        details: { reported_contract: error.reportedContract },
        ...(error.line === null ? {} : { line: error.line }),
      },
      internal: false,
    }
  }
  if (error instanceof GeometryEngineUnavailableError) {
    const transient = error.availabilityCause === 'readiness-timeout' || error.availabilityCause === 'readiness-failed'
    return {
      error: {
        code: 'engine_unavailable',
        message: error.message,
        retryable: transient,
        next_action: transient
          ? 'Retry after the provider readiness check completes; inspect openscad://capabilities before retrying.'
          : 'Read openscad://capabilities and deploy or restore the qualified engine required by this source contract.',
        details: {
          language_contract: error.execution.languageContract,
          engine_class: error.execution.engineClass,
          engine_key: error.execution.engineKey,
          availability_cause: error.availabilityCause,
          automatic_fallback: false,
        },
      },
      internal: false,
    }
  }
  if (isQuarantinedGeometryHostError(error)) {
    return {
      error: {
        code: 'engine_unavailable',
        message: 'The isolated geometry runtime is quarantined after a Worker shutdown failure.',
        retryable: false,
        next_action: 'Restart the local MCP server before retrying geometry operations.',
        details: {
          availability_cause: 'quarantined',
          runtime_boundary: 'worker-thread',
          restart_required: true,
          automatic_fallback: false,
        },
      },
      internal: false,
    }
  }
  if (error instanceof GeometryCapabilityUnavailableError) {
    return {
      error: {
        code: 'capability_unavailable',
        message: error.message,
        retryable: false,
        next_action: 'Read the selected engine manifest and remove or satisfy the declared @requires capabilities.',
        details: {
          language_contract: error.execution.languageContract,
          engine_class: error.execution.engineClass,
          missing_capabilities: error.missingCapabilities.join(','),
          automatic_fallback: false,
        },
      },
      internal: false,
    }
  }
  return null
}

function mapGeometryRuntimeError(error: unknown): { error: PublicToolError; internal: boolean } | null {
  if (error instanceof ArtifactSizeError) {
    return {
      error: {
        code: 'artifact_too_large',
        message: error.message,
        retryable: true,
        next_action: 'Reduce model complexity or request a larger max_bytes value within the advertised limit.',
        details: { estimated_bytes: error.estimatedBytes, max_bytes: error.maxBytes },
      },
      internal: false,
    }
  }
  if (error instanceof GeometryBusyError) {
    return {
      error: {
        code: 'server_busy',
        message: error.message,
        retryable: true,
        next_action: 'Wait for retry_after_ms, then retry the operation.',
        details: { retry_after_ms: error.retryAfterMs },
      },
      internal: false,
    }
  }
  if (error instanceof GeometryDeadlineExceededError) {
    return {
      error: {
        code: 'deadline_exceeded',
        message: error.message,
        retryable: true,
        next_action: 'Reduce model complexity or retry after other queued geometry work completes.',
        details: { deadline_ms: error.deadlineMs },
      },
      internal: false,
    }
  }
  if (error instanceof CustomizerValueError) {
    return {
      error: {
        code: 'invalid_argument',
        message: error.message,
        retryable: false,
        next_action: 'Inspect the declared Customizer parameters and retry with a valid value.',
      },
      internal: false,
    }
  }
  if (error instanceof SweepBudgetError) {
    return {
      error: {
        code: 'invalid_argument',
        message: error.message,
        retryable: false,
        next_action: 'Reduce sweep axes/values or raise the explicit sweep limits within server bounds.',
      },
      internal: false,
    }
  }
  if (error instanceof HandoffBundleValidationError) {
    return {
      error: {
        code: 'invalid_argument',
        message: error.message,
        retryable: false,
        next_action:
          'Provide an unmodified openscad-viewer/handoff-bundle-v1 JSON payload whose hashes and headers match.',
      },
      internal: false,
    }
  }
  if (error instanceof InvalidGeometryError) {
    return {
      error: {
        code: 'invalid_geometry',
        message: error.message,
        retryable: false,
        next_action: 'Adjust the source so all transforms, vertices, and computed metrics are finite.',
      },
      internal: false,
    }
  }
  if (isAbortError(error)) {
    return {
      error: {
        code: 'cancelled',
        message: 'The OpenSCAD operation was cancelled.',
        retryable: true,
        next_action: 'Retry if the result is still needed.',
      },
      internal: false,
    }
  }
  return null
}

export function mapGeometryAndCatalogError(error: unknown): { error: PublicToolError; internal: boolean } | null {
  return mapCatalogError(error) ?? mapGeometryEngineError(error) ?? mapGeometryRuntimeError(error)
}
