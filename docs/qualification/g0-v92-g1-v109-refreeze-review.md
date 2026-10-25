# G0 v92 / G1 v109 re-freeze review

Preserve OwnRust77, G0v91, G1v108 and every predecessor byte for byte. Move the
PR58 predecessor identity table into its own module so the common registry
remains under the existing 500-line architecture limit. The expanded registry
retains every previous identity and adds the newly frozen predecessors.

Rust geometry, native proofs, proof budgets, tolerances and the measured
geometry WASM remain identical to e51471d52. The scoped native, Node20/Node22,
Chromium mesh/STEP and distribution receipts retain their original boundaries.

OwnRust78 / G0v92 / G1v109 bind the final metadata layout and executors. All
4740 semantic clean work units remain incomplete; no G0 closure, full G1,
full-workspace or hosted CI result is imported or approved by this re-freeze.
