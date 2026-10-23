import {
  lowerOpenSCADToSemanticProgramUnchecked,
  type SemanticLoweringOptions,
} from './openscadSemanticLowerer'
import type { SemanticLoweringSuccess } from './semanticProgramTrust'

// The minting capability is deliberately lexical: no importable function can
// add an arbitrary object to this set.
const TRUSTED_SEMANTIC_LOWERINGS = new WeakSet<object>()
const addTrustedSemanticLowering = TRUSTED_SEMANTIC_LOWERINGS.add.bind(TRUSTED_SEMANTIC_LOWERINGS)
const hasTrustedSemanticLowering = TRUSTED_SEMANTIC_LOWERINGS.has.bind(TRUSTED_SEMANTIC_LOWERINGS)

export function lowerOpenSCADToSemanticProgram(
  source: string,
  options: SemanticLoweringOptions = {},
): SemanticLoweringSuccess {
  const artifact = lowerOpenSCADToSemanticProgramUnchecked(source, options)
  addTrustedSemanticLowering(artifact)
  return artifact
}

export function requireTrustedSemanticLowering(value: unknown): SemanticLoweringSuccess {
  if (value === null || typeof value !== 'object' || !hasTrustedSemanticLowering(value)) {
    throw new TypeError('Semantic execution requires an artifact minted by lowerOpenSCADToSemanticProgram')
  }
  return value as SemanticLoweringSuccess
}

export type {
  SemanticLoweringOptions,
} from './openscadSemanticLowerer'
export type {
  SemanticLoweringReport,
  SemanticLoweringSuccess,
} from './semanticProgramTrust'
