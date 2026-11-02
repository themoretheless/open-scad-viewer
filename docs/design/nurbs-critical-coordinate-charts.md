# Critical-coordinate chart injection

`surface_injectivity::certify_contraction` retains its strict interval-Jacobian
contract: a successful global contraction proves injection and rank two.
Some original maps are injective without an invertible fixed coordinate
projection. `surface_injectivity::certify_chart` may additionally establish
injection from two independent strictly monotone coordinates. This entry gives
**no rank/immersion guarantee**; consumers needing regularity must prove it
separately. The existing general `certify` pipeline keeps its previous strict-proof
behavior because some volume owners also rely on its rank guarantee.

The new source owner requires an unchanged nonperiodic, clamped single-span
polynomial surface within the existing validated source degree limits. The
internal monotonicity primitive retains its existing ceiling of 32; this does
not raise the public source limit. Literal coefficient comparisons
prove one world coordinate depends only on U, and a different coordinate only
on V. A common positive weight cancels exactly. Strict coordinate monotonicity
is proved by the existing exact dyadic subdivision owner; rounded subdivision
refuses. Isolated zero projected derivatives are permitted; constant intervals,
folds, unsupported weights and missing work never supply an injection proof.
Every knot/weight/dependence comparison and monotonicity attempt debits the same
original work ledger, including failed candidates. No ceiling was raised.

The successful method is `global-separable-coordinate-monotonicity`, with
`projection` identifying the two independent coordinates and
`contraction_upper: None`. BRep face injection uses this contract in both batch
and resumable paths. The ideal-volume intrinsic owner still calls the strict
contraction entry because it requires rank as well as injection.

For intersections with a planar face, the new bounded opposite-row support
owner proves that the entire open original Bezier strip is strictly on one
side of the exact plane, while both opposite natural boundary rows lie on it.
All weights must be finite and positive. Exact complete rational boundary
identities and shared authored edge IDs bind both zero rows. Missing ownership,
side changes, coplanar overlap, failed exact predicates or insufficient shared
pair work refuse. No endpoint-only or sampled contact establishes admission.

The complete boundary certificate still independently requires exact lifts,
valid material trims, every face injective, all face pairs classified, full
regularity, shell nesting/roles, and orientation. Source/retained positional
bounds and applicable smoothness remain separate. In particular, an injective
cusp does not acquire rank, and sharp cap joins remain C0.
