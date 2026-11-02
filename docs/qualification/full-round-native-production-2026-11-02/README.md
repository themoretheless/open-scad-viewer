# Exact Full Round native operation production

`geometry:full_round_exact` is an authored unary map operation producing
`full-round-exact`. Production replay requires one output per ordered child
frontier item and the matching input node. Each result owns its new scene
identity, unlike a transform preserving a child identity. All outputs of one
occurrence must use field-equal face/limits/maxBytes parameters. Three unsigned
integer face indices are required; geometry admission remains responsible for
selection validity and material proof.

The added native fixture derives all operation and occurrence identity digests
from its structural paths. It is authored from an existing nested-transform
fixture, not lowered by the TS frontend. Tests reject a wrong source frontier,
wrong result identity owner, wrong output kind and malformed face indices.
A separate test executes the complete native identity/production/capability
admission and actual rational Full Round construction.

This does not prove source-language lowering, source-text attestation, host
schema acceptance, a new WASM build, packaged application integration, or
Plasticity behavioral equivalence. Those and the original full parity scope
remain open.
