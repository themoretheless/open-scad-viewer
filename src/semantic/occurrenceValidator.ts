// Public entry point; implementations retain their original contracts.
export { consumeOccurrenceProofBudget, moduleActivationAnchor, isChildrenExpansionContinuation } from './occurrenceValidator/SemanticModuleActivationAnchor'
export type { SemanticModuleActivationAnchor, SemanticOccurrenceValidationIndex } from './occurrenceValidator/SemanticModuleActivationAnchor'
export { validateOccurrences, occurrenceRoot, occurrenceIsDescendantOf, nodePreservesIdentity, validateOutputRef } from './occurrenceValidator/validateOccurrences'
export { validateResult } from './occurrenceValidator/validateResult'
