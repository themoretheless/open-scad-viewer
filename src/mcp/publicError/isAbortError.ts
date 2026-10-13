import { AbortedError, OpenSCADParseError } from '../../services/openscadErrors'
import { ArtifactSizeError, CustomizerValueError, GeometryBusyError, GeometryDeadlineExceededError, InvalidGeometryError } from '../geometryService'
import { MAX_STORED_DIAGNOSTIC_MESSAGE_LENGTH, MAX_STORED_DIAGNOSTIC_NAME_LENGTH } from '../modelStore'
import type { BuildDiagnostic } from '../modelStore'
import { GeometryCapabilityUnavailableError, GeometryEngineUnavailableError, GeometryLanguageContractError } from '../../services/geometryBuildEngine'
import { publicToolError } from './publicToolError'
import { truncateWellFormed } from '../../core/unicode'

export function parseErrorMessage(error: OpenSCADParseError): string {
  return error.message.split('\n', 1)[0]
}

export function isAbortError(error: unknown): boolean {
  return error instanceof AbortedError
    || (error instanceof DOMException && error.name === 'AbortError')
    || (error instanceof Error && error.name === 'AbortError')
}

export function isQuarantinedGeometryHostError(
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
