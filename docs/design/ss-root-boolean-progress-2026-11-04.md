# Material-selected Boolean results for strict root relations

`QualifiedRootBoolean` implements union, intersection, directed difference and
XOR after a sealed complete root relation proves disjoint boundaries or strict
containment. The input relation owns both original source material proofs and
both complete boundary covers. Intersecting or touching boundaries have no
shortcut into this API; their missing arrangement/trim proofs remain required.
`qualify_swapped` reverses operand order without discarding the original relation
proof, preserving directed difference semantics.

For disjoint inputs, union/XOR retain both bodies, intersection is empty and
A minus B retains A. For strict containment, union retains the outer body and
intersection the inner body. Outer minus inner and XOR create the corresponding
admitted shell forest; inner minus outer is empty. The selection policy is
checked independently against the original Boolean material predicate on every
admissible region, including operand reversal.

Results own either the immutable selected source, a newly admitted two-root
forest, or explicit empty material. Empty material has volume [0,0] and no BRep;
no invalid zero-face body is fabricated. Nonempty results require positive
independently proved volume within the requested result accuracy. Every retained
source has a snapshot-bound topology map; discarded sources have no child map.
Unchanged selected source geometry shares its exact original snapshot. Combined
results preserve actual root curves, UV clocks, material shell orientation and
body ancestry through the forest's validated topology.

Point membership follows only retained boundaries. A boundary discarded by union
cannot leave an unresolved boundary hit in its interior; an unrelated discarded
body cannot contaminate difference. Empty material classifies every finite point
outside with no ray work. Nonfinite points remain invalid. Other point queries
use the existing retained source or forest policy. Replay verifies source/target
lineage identities, freshly replays the complete relation and rebuilds material
selection, volume and lineage policy.

Native qualification passed for all four operations on separate and
strictly nested freshly admitted root bodies, plus reversed nested operands.
It compares independently scaled analytic reference volumes, retained source
snapshots, deletion/retention lineage, erased and retained boundary queries,
explicit empty material and insufficient requested accuracy. A nested difference
result receives a full fresh replay in addition to the existing forest replay.

This is the complete material-selection stage for STRICT RELATION cases. General
intersecting NURBS Boolean still requires all-face-pair SS arrangements, finite
trim and shared cross-source topology. Singular/coincident and periodic/pole
strata, healing, general mixed legacy/root inputs, splitting lineage/persistent
history, actual bridge/WASM delivery and broader quad/offset/fillet/G2 parity
remain in the original goal.

Next proof work is anchored in current source constraints, not further strict-
relation dispatch cases: `nurbs-core`'s `OriginalSsRootPoint` stores a binary64
free clock and equates selectors by graph Arc plus clock. General trim junctions
need algebraic clock ownership and enclosure without replacing the root by an
interval midpoint. The current root-wire winding gate additionally requires
the literal common source pair, zero phases, and rejects OriginalSourceNode/
CollapsedPole ports. Cross-source face loops and quotient/pole arrangements
need new face-local junction/domain proofs before arbitrary finite trim can
reuse this Boolean material-selection stage.

Frozen qualification: 15 tests passed. The native integration constructed all
12 relation/operation selections, checked independent volume references and
erased/retained boundaries, then freshly replayed both the full source forest
and a nested Boolean difference. The final policy oracle checks the original
Boolean material predicate on every admissible region. It changed only under
cfg(test) while native integration ran; the production prefix was unchanged.
Logs, tested source hashes and that production-prefix hash are frozen in
`docs/qualification/ss-root-boolean-2026-11-04/`.
