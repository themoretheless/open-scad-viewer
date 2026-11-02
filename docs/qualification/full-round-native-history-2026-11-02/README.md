# Native exact Full Round feature history

`full_round_curved::admitted::history::History` owns immutable admitted bodies
through Arc. Live source rebasing reconstructs the feature by the same authored
source topology IDs; numeric indices and geometry matching cannot silently
replace missing identities. Reselect rebuilds on the current original source.
Every edit constructs/proves a candidate before replacing history state.

Undo/redo move the cursor. Editing an undone history requires an explicit prefix
branch, preserving the original redo branch. Transport retains every revision,
including hidden redo states. Replay uses each frozen recipe, freshly reconstructs
geometry/material/G1/radius, checks source-ID continuity or unchanged-source
selection semantics, and compares complete native artifacts.

Pair/atom/ray counters come from the actual material census. Their aggregate is
charged across all retained revisions. Import accepts caller revision/byte/work
budgets; stored limits cannot expand those allowances. Refinement/halving limits
remain per qualification; this is not accounting for every independent geometry
proof subsystem. Byte limits apply to serialized history with codec enabled.

This is exact Full Round history over the currently admitted extrusion family,
not a general dependency graph, arbitrary source-construction history, branch
merge, edit selection-transfer algorithm or Plasticity-equivalence certificate.
The caller supplies a rebuilt original source Model with retained topology IDs.
Integration into bridge/WASM/editor/scene history is still open; the previously
installed kernel is not claimed to expose this new native API.
