# Exact Full Round source and display WASM qualification

The runner compiles an import-free WASM module and calls its real
`abi_alloc`/`abi_request`/`abi_free` interface with the shared binary codec.
The TS public lowerer generates the complete source-bound program. Rust owns
source/identity/capability admission, exact geometry construction, frozen-recipe
restoration and display sampling. There is no host geometry fallback.

Coverage includes source-to-geometry execution; display at 2/8/16 samples;
independent rounded-box display-volume convergence; exact native-to-WASM frozen
artifact geometry/IDs; rational native-artifact display; and refusal of invalid
sampling/triangle budgets, altered geometry, source text or capability closure.
Display remains explicitly uncertified for geometric error, embedding and exact
mass. Input artifacts are saved separately from display data.

This is a separately built module. The application package and scene builder
are not changed by this qualification. Actual application-worker integration,
exact artifact snapshot/edit ownership, broad regression closure and the full
initial library parity scope remain open.
