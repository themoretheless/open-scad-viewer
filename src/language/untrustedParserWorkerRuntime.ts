import { buildStepRawArtifact } from '../exchange/stepRawArtifact'
import { preflightIgesInput, preflightStlInput, ParserInputError } from './parserInputBudgets'
import type { UntrustedParserJob, UntrustedParserResult } from './untrustedParserWorker'

export function executeUntrustedParserJob(job: UntrustedParserJob): UntrustedParserResult {
  switch (job.kind) {
    case 'step-raw': return buildStepRawArtifact(job.text, job.budgets)
    case 'iges-preflight': return preflightIgesInput(job.text, job.budgets)
    case 'stl-preflight': return preflightStlInput(job.bytes, job.budgets)
  }
}

export function parserWorkerError(error: unknown): { name: string; code: string; message: string } {
  if (error instanceof ParserInputError) return { name: error.name, code: error.code, message: error.message }
  return {
    name: error instanceof Error ? error.name : 'Error',
    code: 'PARSER_INVALID_INPUT',
    message: error instanceof Error ? error.message : String(error),
  }
}
