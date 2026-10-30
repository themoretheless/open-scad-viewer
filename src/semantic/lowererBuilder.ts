// Public entry point; implementations retain their original contracts.
export { MAX_SHAPES, MAX_EXTRUDE_SLICES, semanticValueType, StableViewportRootSelection, LegacyBindingCompatibilityError, StableBuiltinValueError, PALETTE, CSS_COLORS, canonicalNumber, identityValue, operationCategory, semanticOperationName } from './lowererBuilder/MAX_SHAPES'
export type { RGBA, SemanticShape, SemanticNodeWithoutId, RegisteredOperations, EvaluationBudget, SemanticRuntimeCheckpoint, SemanticDiagnosticCheckpoint, PassedCallChildren, EvalContext, SemanticLoweringOptions } from './lowererBuilder/MAX_SHAPES'
export { registerOperations } from './lowererBuilder/registerOperations'
export { SemanticProgramBuilder } from './lowererBuilder/SemanticProgramBuilder'
export { countStatements, resultFromShapes, remapSemanticNode, canonicalizeCompletedDag } from './lowererBuilder/countStatements'
