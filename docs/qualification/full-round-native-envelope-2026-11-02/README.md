# Exact Full Round capability admission

Native envelope capability derivation recognizes `full-round-exact` as a unary
operation. Its closure includes `operation.full-round-exact` and the actual
`representation.transition.analytic-brep.to.rational-brep` dependency.

The integration test exercises native semantic result selection plus a supplied
capability closure. Missing operation, rational representation or transition
capabilities must refuse with `BREP_SEMANTIC_CONTRACT`. An out-of-range input
must fail closure derivation.

This is not a complete source-bound SemanticProgram fixture: it does not supply
operation identities or exercise occurrence-production replay. The TS node
union/validator/lowerer, native production rules, published WASM kernel and
application worker integration remain open. Earlier WASM evidence does not
qualify this new change.
