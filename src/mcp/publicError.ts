import { randomUUID } from 'node:crypto'
import { AbortedError, OpenSCADParseError } from '../services/openscadErrors'
import { OpenScadImportPositionedError } from '../services/openScadImport'
import { OpenScadSurfaceError } from '../services/openScadSurface'
import { OpenScadTextPositionedError } from '../services/openScadText'
import {
  GeometryCapabilityUnavailableError,
  GeometryEngineUnavailableError,
  GeometryLanguageContractError,
} from '../services/geometryBuildEngine'
import {
  ArtifactSizeError,
  CustomizerValueError,
  GeometryBusyError,
  GeometryDeadlineExceededError,
  InvalidGeometryError,
} from './geometryService'
import {
  CatalogQuotaError,
  MAX_STORED_DIAGNOSTIC_MESSAGE_LENGTH,
  MAX_STORED_DIAGNOSTIC_NAME_LENGTH,
  ModelRevisionConflictError,
  type BuildDiagnostic,
} from './modelStore'
import { type PublicErrorCode } from './errorContract'
import {
  OfficialOpenScadRemoteError,
  OfficialOpenScadSupervisorError,
} from './officialOpenScadRuntimeService'
import { OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES } from './officialOpenScadRuntimeProtocol'

export { PUBLIC_ERROR_CODES, type PublicErrorCode } from './errorContract'

type PublicErrorDetail = string | number | boolean | null

export interface PublicToolError {
  code: PublicErrorCode
  message: string
  retryable: boolean
  next_action?: string
  correlation_id?: string
  details?: Record<string, PublicErrorDetail>
  line?: number
  column?: number
}

export class ModelNotFoundError extends Error {
  constructor(readonly modelId: string, readonly revision?: number) {
    super(revision === undefined
      ? `Model ${modelId} was not found`
      : `Model ${modelId} revision ${revision} was not found`)
    this.name = 'ModelNotFoundError'
  }
}

function truncateWellFormed(value: string, maxLength: number): string {
  if (value.length <= maxLength) return value
  let end = maxLength
  const last = value.charCodeAt(end - 1)
  if (last >= 0xD800 && last <= 0xDBFF) end--
  return value.slice(0, end)
}

function parseErrorMessage(error: OpenSCADParseError): string {
  return error.message.split('\n', 1)[0]
}

export function isAbortError(error: unknown): boolean {
  return error instanceof AbortedError
    || (error instanceof DOMException && error.name === 'AbortError')
    || (error instanceof Error && error.name === 'AbortError')
}

function isQuarantinedGeometryHostError(
  error: unknown,
): error is Error & { code: 'E_DIRECT_GEOMETRY_JOIN' | 'E_DIRECT_GEOMETRY_QUARANTINED' } {
  if (!(error instanceof Error)) return false
  const code = (error as Error & { code?: unknown }).code
  return code === 'E_DIRECT_GEOMETRY_JOIN' || code === 'E_DIRECT_GEOMETRY_QUARANTINED'
}

export function buildDiagnostic(error: unknown): BuildDiagnostic {
  let diagnostic: Omit<BuildDiagnostic, 'contractVersion' | 'code' | 'retryable'>
  if (error instanceof OpenSCADParseError) {
    diagnostic = {
      name: 'OpenSCADParseError',
      message: 'OpenSCAD source failed to compile. Re-run the check to obtain current diagnostics.',
      line: error.line,
      column: error.column,
    }
  } else if (isAbortError(error)) {
    diagnostic = { name: 'AbortError', message: 'OpenSCAD evaluation was cancelled.' }
  } else if (error instanceof ArtifactSizeError || error instanceof CustomizerValueError
    || error instanceof GeometryBusyError || error instanceof GeometryDeadlineExceededError
    || error instanceof InvalidGeometryError
    || error instanceof GeometryCapabilityUnavailableError
    || error instanceof GeometryEngineUnavailableError
    || error instanceof GeometryLanguageContractError
    || isQuarantinedGeometryHostError(error)) {
    const positioned = error as Error & { line?: unknown; column?: unknown }
    diagnostic = {
      name: truncateWellFormed(error.name || 'Error', MAX_STORED_DIAGNOSTIC_NAME_LENGTH),
      message: truncateWellFormed(error.message, MAX_STORED_DIAGNOSTIC_MESSAGE_LENGTH),
      ...(typeof positioned.line === 'number' && Number.isSafeInteger(positioned.line)
        && positioned.line > 0 ? { line: positioned.line } : {}),
      ...(typeof positioned.column === 'number' && Number.isSafeInteger(positioned.column)
        && positioned.column > 0 ? { column: positioned.column } : {}),
    }
  } else {
    diagnostic = {
      name: 'InternalError',
      message: 'OpenSCAD processing failed unexpectedly. See the server log for details.',
    }
  }

  const classified = publicToolError(error).error
  return {
    ...diagnostic,
    contractVersion: 1,
    code: classified.code,
    retryable: classified.retryable,
    ...(classified.details ? { details: classified.details } : {}),
  }
}

export function publicToolError(error: unknown): { error: PublicToolError; internal: boolean } {
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
  if (error instanceof OfficialOpenScadSupervisorError) {
    if (error.code === 'E_OFFICIAL_OPENSCAD_BUSY') {
      return {
        error: {
          code: 'server_busy',
          message: 'The official OpenSCAD runtime is at its concurrency limit.',
          retryable: true,
          next_action: 'Wait briefly, then retry the official OpenSCAD operation.',
          details: { retry_after_ms: 250, runtime: 'official-openscad' },
        },
        internal: false,
      }
    }
    if (error.code === 'E_OFFICIAL_OPENSCAD_CANCELLED') {
      return {
        error: {
          code: 'cancelled',
          message: 'The official OpenSCAD operation was cancelled.',
          retryable: true,
          next_action: 'Retry if the result is still needed.',
          details: { runtime: 'official-openscad' },
        },
        internal: false,
      }
    }
    if (error.code === 'E_OFFICIAL_OPENSCAD_DEADLINE') {
      return {
        error: {
          code: 'deadline_exceeded',
          message: 'The official OpenSCAD operation exceeded its deadline.',
          retryable: true,
          next_action: 'Reduce model complexity, increase timeout_ms within the advertised limit, or retry later.',
          details: { runtime: 'official-openscad' },
        },
        internal: false,
      }
    }
    if (error.code === 'E_OFFICIAL_OPENSCAD_UNAVAILABLE') {
      return {
        error: {
          code: 'engine_unavailable',
          message: 'The opt-in official OpenSCAD runtime is unavailable.',
          retryable: false,
          next_action: 'Read openscad://official-runtime and run its setup_command, then restart or retry the MCP server.',
          details: {
            availability_cause: 'provider-missing',
            runtime: 'official-openscad',
            automatic_fallback: false,
          },
        },
        internal: false,
      }
    }
    if (error.code === 'E_OFFICIAL_OPENSCAD_CLOSED') {
      return {
        error: {
          code: 'engine_unavailable',
          message: 'The official OpenSCAD runtime is closed.',
          retryable: false,
          next_action: 'Restart the local MCP server before retrying official OpenSCAD operations.',
          details: {
            availability_cause: 'revoked',
            runtime: 'official-openscad',
            automatic_fallback: false,
          },
        },
        internal: false,
      }
    }
    // Protocol violations and child crashes are internal faults. They fall
    // through to the redacted, correlation-id-bearing internal error below.
  }
  if (error instanceof OfficialOpenScadRemoteError) {
    if (error.code === 'E_OPENSCAD_OUTPUT_LIMIT') {
      return {
        error: {
          code: 'artifact_too_large',
          message: 'The official OpenSCAD export exceeded the advertised output limit.',
          retryable: true,
          next_action: 'Reduce model complexity or choose a more compact export format.',
          details: {
            estimated_bytes: null,
            max_bytes: OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES,
            runtime: 'official-openscad',
          },
        },
        internal: false,
      }
    }
    if (error.code === 'E_OPENSCAD_COMPILE' || error.code === 'E_OPENSCAD_NO_OUTPUT') {
      return {
        error: {
          code: 'source_syntax_error',
          message: 'The official OpenSCAD runtime rejected the source project.',
          retryable: false,
          next_action: 'Inspect the bounded logs returned by openscad_official_check, correct the source or project files, and retry.',
          details: { runtime: 'official-openscad' },
        },
        internal: false,
      }
    }
    // Unknown remote codes are runner faults, not user-facing diagnostics.
  }
  if (error instanceof OpenScadImportPositionedError) {
    return {
      error: {
        code: 'source_syntax_error',
        message: parseErrorMessage(error),
        retryable: false,
        next_action: 'Correct the import() project asset or path at the reported location, then retry.',
        line: error.line,
        column: error.column,
        details: {
          diagnostic_code: error.importCode,
          source_path: error.sourcePath,
          ...(error.specifier.length === 0 ? {} : { specifier: error.specifier }),
          ...(error.assetPath === undefined ? {} : { asset_path: error.assetPath }),
          ...(error.format === undefined ? {} : { format: error.format }),
          ...(error.limit === undefined ? {} : { limit: error.limit }),
          ...(error.actual === undefined ? {} : { actual: error.actual }),
        },
      },
      internal: false,
    }
  }
  if (error instanceof OpenScadTextPositionedError) {
    return {
      error: {
        code: 'source_syntax_error',
        message: parseErrorMessage(error),
        retryable: false,
        next_action: 'Correct the text() parameters or provide the requested font in the bounded project files, then retry.',
        line: error.line,
        column: error.column,
        details: {
          diagnostic_code: error.textCode,
          source_path: error.sourcePath,
          ...(error.font === undefined ? {} : { font: error.font }),
          ...(error.fontPath === undefined ? {} : { font_path: error.fontPath }),
          ...(error.faceIndex === undefined ? {} : { face_index: error.faceIndex }),
          ...(error.limit === undefined ? {} : { limit: error.limit }),
          ...(error.actual === undefined ? {} : { actual: error.actual }),
        },
      },
      internal: false,
    }
  }
  if (error instanceof OpenScadSurfaceError) {
    return {
      error: {
        code: 'source_syntax_error',
        message: parseErrorMessage(error),
        retryable: false,
        next_action: 'Correct the surface() project file or path at the reported location, then retry.',
        line: error.line,
        column: error.column,
        details: {
          diagnostic_code: error.code,
          source_path: error.details.sourcePath,
          ...(error.details.specifier === undefined ? {} : { specifier: error.details.specifier }),
          ...(error.details.assetPath === undefined ? {} : { asset_path: error.details.assetPath }),
          ...(error.details.limit === undefined ? {} : { limit: error.details.limit }),
          ...(error.details.actual === undefined ? {} : { actual: error.details.actual }),
        },
      },
      internal: false,
    }
  }
  if (error instanceof OpenSCADParseError) {
    return {
      error: {
        code: 'source_syntax_error',
        message: parseErrorMessage(error),
        retryable: false,
        next_action: 'Correct the source at the reported line and column, then retry.',
        line: error.line,
        column: error.column,
      },
      internal: false,
    }
  }
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
    const transient = error.availabilityCause === 'readiness-timeout'
      || error.availabilityCause === 'readiness-failed'
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
  const correlationId = randomUUID()
  return {
    error: {
      code: 'internal_error',
      message: 'The MCP server could not complete the request.',
      retryable: true,
      next_action: 'Retry once; if the failure persists, inspect the server log using the correlation_id.',
      correlation_id: correlationId,
    },
    internal: true,
  }
}
