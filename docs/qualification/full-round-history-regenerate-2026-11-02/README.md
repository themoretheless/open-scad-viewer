# Native earlier-selection dependency regeneration

Added `History::regenerate_selection(revision, selected_source_face_ids)`.
It edits the authored parameter of the root or an explicit reselection step and rebuilds every descendant with fresh native Full Round construction/admission. Source-rebase revisions inherit the new selection by authored IDs. Later explicit reselections retain their authored choices. Saved source models remain exact inputs; this is not source-expression reconstruction. Hidden redo is regenerated and the original cursor is retained.

Actual work is recalculated over retained prefix plus rebuilt descendants; caller material and serialized document allowances remain enforced. A clone is committed only after all descendants and storage checks succeed. Missing IDs in a later source are refused without guessing lineage or changing the original history.

Three new native tests plus eight existing history tests passed with optimized tests and debug assertions/overflow checks enabled. They cover actual geometry change, explicit override, hidden redo, exact codec replay/work, and transactional failures. See retained full log and source snapshots. Clean baseline 11 files unchanged. Six previously recorded architecture violations remain; no limits raised.

This native method is not yet exposed by the bridge or installed WASM. Earlier source-parameter edits, source-expression dependency graphs, changed-ID lineage and branch merges remain open, together with all original Plasticity library parity requirements. No proprietary history implementation was copied and external Plasticity runtime equivalence is not established.
