/** Host diagnostic shape shared by native transport and the compatibility adapter. */
export type BrepSemanticBackendErrorCode =
  | 'E_BREP_SEMANTIC_CONTRACT'
  | 'E_BREP_SEMANTIC_LIFECYCLE'
  | 'E_BREP_SEMANTIC_UNSUPPORTED'
  | 'E_BREP_SEMANTIC_KERNEL'
  | 'E_BREP_SEMANTIC_BUDGET'

export class BrepSemanticBackendError extends Error {
  constructor(
    readonly code: BrepSemanticBackendErrorCode,
    message: string,
    readonly kernelCause?: unknown,
  ) {
    super(message)
    this.name = 'BrepSemanticBackendError'
  }
}

