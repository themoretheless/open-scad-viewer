# G0 v7 / G1 v24 rebuilt-kernel evidence correction

After G0 v6 and G1 v23 were frozen, removal of accidental rustfmt-only changes
produced the intended release-qualified-v2 kernel bytes. The rebuilt own-Rust
WASM digest changed, so the current evidence pointer and every current binding
must describe those bytes rather than the superseded generated artifact.

G0 v1-v6, G1 v1-v23, and every prior status and review remain byte-immutable
historical evidence. G0 v7 records current toolchain-bound bytes without
closing G0. G1 v24 preserves the complete finite v23 contract and binding
membership, recomputes current digests, and starts with zero completed clean
runs and zero completed work units.

Prior results remain discovery-only and cannot be imported.
`qualificationClaim` remains `none`, qualification approval remains
`not-approved`, u07 remains unresolved, and production cutover remains
prohibited. This freeze does not execute qualification work or create external
clean-run evidence.
