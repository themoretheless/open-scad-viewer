# Authored displacement fields and coupled original jets — 2026-11-03

Native proposal work. No closed-body, whole-body G2, cap, thickness, embedding,
WASM/UI or STEP acceptance follows from this snapshot.

## Authorship retained before world-coordinate rounding

Central layer construction now retains two base curves and two displacement
curves separately, before forming the six translated rows. All four components
are aligned to the actual original union basis under the construction work
budget. They are private proposal authorship, not source-identity certificates.
`reverify` reconstructs and compares this authorship as well as the old geometry.

Product-network endpoint targets can then be computed from compensated base
jets plus gain times field jets. This avoids differentiating independently
rounded translated world poles. Source curves, roots, weights, stock, tolerances
and the old admitted single-surface geometry are not replaced by metadata.

Actual unequal-radius corner before coupling: all four joins still rejected.
Tensor-jump witness lower bounds: 2.6684456764289317, 491.3781205642909,
0.19715022005024527, 1.7504371964104464 per mm. These are counterexample lower
bounds, not error upper bounds or whole-seam normal/G2 certificates.

## Bounded joint proposal

`propose_reconciled_patches` builds a distinct unqualified candidate. It shares
its operation allowance conservatively between original construction and
coupling, leaving all seam root columns and weights fixed. The first/second
physical cross-jets of each adjacent pair propose common targets; P2 always
uses the actually stored rounded P1. Every proposed coordinate change is
limited to one third of the caller's control-displacement budget, and this
budget is one quarter of the positional tolerance. The positional tolerance
itself is unchanged. This construction guard is not an original-source G0
receipt: full source agreement must still be rechecked.

Unweighted averaging on the actual corner was rejected for exceeding that
position budget. The revised proposal selects the endpoint with lower
second-jet coefficient sensitivity when the sensitivity ratio exceeds 16;
otherwise it averages. Target selection is heuristic, never proof. The actual
original post-rounding polynomial-seam G2 inspector must independently accept
all four joins. Unsupported cells, work exhaustion and excessive displacement
refuse the candidate.

Absolute curvature <=10/mm, exterior G2, source-face trimming and closed-body
assembly remain independent mandatory gates. A diagnostic process exit code 0
does not indicate geometry acceptance.

## Verified results

Focused `region_replacement::` regression suite: **46 passed**, including an
actual curvature-jump correction with fixed root columns and weights, plus
budget and unsupported-source rejection. These tests do not qualify the corner.

At handle 1/256 mm the conditioned coupled proposal fits the unchanged
positional guard and builds five patches. Original complete-seam inspection:

| Seam | Decision | Cells | Pending |
| --- | --- | --- | --- |
| 0 | Unresolved | 6000 | 16 |
| 1 | Unresolved | 6000 | 19 |
| 2 | WithinTolerance, regular | 1016 | 0 |
| 3 | Unresolved | 6000 | 21 |

For seam 2, original whole-seam normal upper <=3.335917661312958e-7 and tensor
jump upper <=9.876857092907333e-5/mm. The other seams have no whole-seam upper
certificate. Absence of a new violation witness is not acceptance.

Independent ORIGINAL point shape-tensor bounds reject the absolute 10/mm cap
on patches 0, 1 and 4: witness lower bounds 239.18234047934473,
14.385966211386851 and 73.66422352138885/mm respectively. The other patches
are unqualified; their checked points cannot admit whole domains.

A follow-up search tried handles 1/128, 1/64, 1/32, 1/16, 1/8 and 1/4 mm with
unchanged nominal radii/tolerances. Every constructible candidate still has an
original point witness above 10/mm. The last two handles are refused by the
original stock control-hull gate. Merely increasing the single handle is not a
solution. Neither the stock bounds nor the curvature cap has been relaxed.

## Remaining construction

Joint rebuilding needs additional interior degrees of freedom, while preserving
external boundary two-jets and owned internal seams. The degree-5 V net has all
six rows allocated to its two boundary two-jets; a single handle cannot supply
an independent interior correction. A higher-degree or multi-patch interior
proposal must be inspected against the same original seam, curvature,
material/volume and embedding gates. Source trimming and closed-body sewing
remain prohibited until these gates are satisfied.

## Reproduce

```sh
cargo test --offline --manifest-path crates/Cargo.toml -p brep-core --lib region_replacement:: --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102
cargo run --offline --manifest-path crates/Cargo.toml -p brep-core --example trihedral_corner_curvature --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102 -- --quartic-profile 0.00390625
cargo run --offline --manifest-path crates/Cargo.toml -p brep-core --example trihedral_corner_curvature --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102 -- --quartic-profile 0.0078125 0.015625 0.03125 0.0625 0.125 0.25
```
