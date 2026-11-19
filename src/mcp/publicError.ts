import { type PublicErrorCode } from './errorContract'
export { isAbortError, buildDiagnostic } from './publicError/isAbortError'
export { publicToolError } from './publicError/publicToolError'

export {
  PUBLIC_ERROR_CODES,
  describePublicErrorCode,
  type PublicErrorCode,
  type PublicErrorCodeDescription,
} from './errorContract'

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
  constructor(
    readonly modelId: string,
    readonly revision?: number,
  ) {
    super(
      revision === undefined ? `Model ${modelId} was not found` : `Model ${modelId} revision ${revision} was not found`,
    )
    this.name = 'ModelNotFoundError'
  }
}

export class InvalidToolInputError extends Error {
  readonly code: PublicErrorCode
  readonly nextAction: string | undefined
  constructor(message: string, options: { code?: PublicErrorCode | undefined; nextAction?: string | undefined } = {}) {
    super(message)
    this.name = 'InvalidToolInputError'
    this.code = options.code ?? 'invalid_argument'
    this.nextAction = options.nextAction
  }
}
