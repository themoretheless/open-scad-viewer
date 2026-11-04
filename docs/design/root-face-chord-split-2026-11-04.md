# Original-root finite face chord subdivision

`RootFaceChordSplit::split` inserts a source-preserving SS chord between two distinct unique vertices of an existing outer loop. It splits the actual coedge sequence into two loops, appends one face and one edge, and gives the new edge opposite coedge directions in the child faces. Existing vertices and boundary curves remain unchanged. Shell uses inherit the original face orientation; body/shell references are preserved.

All four incident endpoint joins must have exact common-face UV authority. The chord surface support is explicit and must match the original face. A shared/ambiguously owned outer loop is refused. Existing holes are retained exactly once with explicit caller-provided child assignments; these are candidate assignments, not inferred material proofs. Lineage exposes both child face indices and the inserted world edge index. Replay revalidates source, chord and output geometry/topology.

This is an actual topology subdivision, but remains a candidate: it does not prove that the chord interior avoids all boundaries, that the assigned holes lie in their intended children, or which child is retained material. The winding stage can independently classify query regions on the constructed children. Complete arrangement/material qualification and general Boolean remain active, together with unrestricted singular/tangent/periodic/pole SS branch completeness and healing.

Regression: split a complete circular AB face with an AC chord through two algebraic vertices. Check both child loops, shared edge orientation, preserved ten boundary vertices, shell face uses, complete UV closure and independent winding queries on both child regions.

Validation: 12 tests passed across root_uv_closure, root_model and root_brep_subdivision. Architecture audit: 54 pinned oversized files, zero layer violations. Logs: docs/qualification/root-face-chord-split-2026-11-04.
