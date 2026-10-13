import { AbortedError, OpenSCADParseError } from '../../services/openscadErrors'
import { DirectGeometryRemoteError, LANGUAGE_DIAGNOSTIC_CODES } from '../directGeometryProtocol'
import type { DirectGeometryError, DirectGeometryErrorCategory } from '../directGeometryProtocol'
import { GeometryCapabilityUnavailableError, GeometryEngineUnavailableError, GeometryLanguageContractError, GeometryProviderContractError, attachGeometryExecutionToError, geometryExecutionForError } from '../../services/geometryBuildEngine'
import type { GeometryExecutionDescriptor } from '../../core/geometryExecution'
import type { LanguageDiagnosticCode } from '../../core/languageContract'
import { compactMessage } from './isDirectGeometryBuildRequest'
import { truncateWellFormed, isWellFormedUnicode as wellFormed } from '../../core/unicode'

export function errorEnvelope(
  category: DirectGeometryErrorCategory,
  name: string,
  message: string,
  fields: Partial<Omit<DirectGeometryError, 'category' | 'name' | 'message'>> = {},
): DirectGeometryError {
  const code = typeof fields.code === 'string' && wellFormed(fields.code)
    ? truncateWellFormed(fields.code, 80)
    : null
  const reportedContract = typeof fields.reportedContract === 'string'
    && wellFormed(fields.reportedContract)
    ? truncateWellFormed(fields.reportedContract, 128)
    : null
  const missingCapabilities = (fields.missingCapabilities ?? [])
    .filter(capability => /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(capability))
    .slice(0, 32)
  return Object.freeze({
    category,
    name,
    message: compactMessage(message),
    code,
    line: fields.line ?? null,
    column: fields.column ?? null,
    start: fields.start ?? null,
    end: fields.end ?? null,
    reportedContract,
    availabilityCause: fields.availabilityCause ?? null,
    missingCapabilities: Object.freeze(missingCapabilities),
  })
}

/** Serialize only known, bounded fields; unknown errors become one fixed message. */
export function serializeDirectGeometryError(error: unknown): DirectGeometryError {
  if (error instanceof OpenSCADParseError) {
    return errorEnvelope(
      'openscad-parse',
      'OpenSCADParseError',
      error.detail || 'OpenSCAD source failed to compile.',
      {
        code: error.code ?? null,
        line: error.line,
        column: error.column,
        start: error.start,
        end: error.end,
      },
    )
  }
  if (error instanceof AbortedError
    || (error instanceof Error && error.name === 'AbortError')) {
    return errorEnvelope('aborted', 'AbortedError', 'Geometry evaluation was cancelled.')
  }
  if (error instanceof GeometryLanguageContractError) {
    return errorEnvelope(
      'language-contract',
      'GeometryLanguageContractError',
      error.message,
      { reportedContract: error.reportedContract, line: error.line },
    )
  }
  if (error instanceof GeometryEngineUnavailableError) {
    return errorEnvelope(
      'engine-unavailable',
      'GeometryEngineUnavailableError',
      error.reason,
      { availabilityCause: error.availabilityCause },
    )
  }
  if (error instanceof GeometryCapabilityUnavailableError) {
    return errorEnvelope(
      'capability-unavailable',
      'GeometryCapabilityUnavailableError',
      'Required geometry capabilities are unavailable.',
      { missingCapabilities: error.missingCapabilities.slice(0, 32) },
    )
  }
  if (error instanceof GeometryProviderContractError) {
    return errorEnvelope(
      'provider-contract',
      'GeometryProviderContractError',
      'The production geometry provider violated its result contract.',
    )
  }
  return errorEnvelope(
    'internal',
    'DirectGeometryWorkerError',
    'The production geometry worker failed.',
  )
}

export function directGeometryExecutionForError(
  error: unknown,
  planned: GeometryExecutionDescriptor | null,
): GeometryExecutionDescriptor | null {
  if (error instanceof GeometryEngineUnavailableError
    || error instanceof GeometryCapabilityUnavailableError) return error.execution
  return geometryExecutionForError(error) ?? planned
}

/** Recreate known domain errors only after the parent validates the terminal. */
export function reconstructDirectGeometryRemoteError(
  error: DirectGeometryError,
  source: string,
  execution: GeometryExecutionDescriptor | null,
): Error {
  let reconstructed: Error
  switch (error.category) {
    case 'openscad-parse': {
      const code = error.code !== null
        && LANGUAGE_DIAGNOSTIC_CODES.has(error.code as LanguageDiagnosticCode)
        ? error.code as LanguageDiagnosticCode
        : undefined
      reconstructed = new OpenSCADParseError(
        source,
        error.start ?? 0,
        error.message,
        code,
        error.end ?? error.start ?? 0,
      )
      break
    }
    case 'aborted':
      reconstructed = new AbortedError()
      break
    case 'language-contract':
      reconstructed = new GeometryLanguageContractError(
        error.message,
        error.reportedContract,
        error.line,
      )
      break
    case 'engine-unavailable':
      reconstructed = execution === null || error.availabilityCause === null
        ? new DirectGeometryRemoteError(error)
        : new GeometryEngineUnavailableError(execution, error.message, error.availabilityCause)
      break
    case 'capability-unavailable':
      reconstructed = execution === null
        ? new DirectGeometryRemoteError(error)
        : new GeometryCapabilityUnavailableError(execution, [...error.missingCapabilities])
      break
    case 'provider-contract':
      reconstructed = new GeometryProviderContractError(error.message)
      break
    default:
      reconstructed = new DirectGeometryRemoteError(error)
  }
  if (execution !== null
    && !(reconstructed instanceof GeometryEngineUnavailableError)
    && !(reconstructed instanceof GeometryCapabilityUnavailableError)) {
    attachGeometryExecutionToError(reconstructed, execution)
  }
  return reconstructed
}
