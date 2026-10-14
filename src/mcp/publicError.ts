import { randomUUID } from 'node:crypto'
import { AbortedError, OpenSCADParseError } from '../services/openscadErrors'
import { OpenScadImportPositionedError } from '../services/openScadImport'
import { OpenScadProjectError } from '../services/openScadProject'
import { OpenScadProjectCompileError } from '../services/openScadProjectCompiler'
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
import {
  OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES,
  OFFICIAL_OPENSCAD_MAX_WIRE_REQUEST_BYTES,
} from './officialOpenScadRuntimeProtocol'
import { truncateWellFormed } from '../core/unicode'
export { isAbortError, buildDiagnostic } from './publicError/isAbortError'
export { publicToolError } from './publicError/publicToolError'

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
