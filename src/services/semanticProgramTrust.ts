import type { SemanticProgramV1 } from '../core/semanticProgram'
import type { SemanticProgramAttestation } from './semanticProgramCodec'

export interface SemanticLoweringReport {
  readonly astStatements: number
  readonly operations: number
  readonly occurrences: number
  readonly nodes: number
  readonly evaluationSteps: number
  readonly allocatedValueUnits: number
}

export interface SemanticLoweringSuccess {
  readonly tag: 'success'
  readonly program: SemanticProgramV1
  readonly canonicalBytes: Uint8Array
  readonly attestation: SemanticProgramAttestation
  readonly warnings: readonly string[]
  readonly fullEquivalent: boolean
  /** Process-local exact legacy error corresponding to core.execution.terminal. */
  readonly terminalError: Error | null
  readonly report: SemanticLoweringReport
}
