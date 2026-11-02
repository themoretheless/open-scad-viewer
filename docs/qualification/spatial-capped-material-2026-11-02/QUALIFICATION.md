# Frozen spatial moving-cap material qualification

Implementation: `e2bb8e3aced7f3f9bfd1b306c1119d3a48cf9f7e`, including parent fixes `bbccba861`
(single outer-loop audit budget) and `20bf45112` (normalized Bezier guide clocks).
All **6,595 Rust/Cargo/toolchain inputs remained byte-identical** throughout
compilation and all three commands.

- Spatial capped matrix: **4 named tests, 10 distinct cases, 0 failures**.
- Full nurbs-core library: **2064 passed, 0 failed, 1 ignored**.
- Full brep-core library: **2073 passed, 0 failed, 5 ignored**.

The case set in matrix-cases.json was independently matched against the exact
ten expected frame/clock/affine/rational keys, not just counted from a green
footer. Every case performs fresh admission, exact replay of all measured shared
budgets, one-less exact-work refusal, cap/source pointer ownership, expected
outer/hole winding, explicit wall classes, explicit C0 cap/wall joins and source
immutability checks. Generic difference regression checks original normalized
value/first-jet bounds, rational weights, raw UV ranges, budget exhaustion and
foreign-source rebinding refusal. The scalar gauge proposal never grants a
weight identity; THEOREM.md gives the all-parameter outward algebra.

| Frame | Clock | Affine laws | Cap weights / region | Cells | Exact work |
|---|---|---|---|---:|---:|
| authored | parameter | unit | gauge / outer + hole | 394 | 377553 |
| authored | parameter | varying rational | gauge / outer + hole | 406 | 377727 |
| authored | arc | unit | gauge / outer + hole | 6626 | 382856 |
| authored | arc | varying rational | gauge / outer + hole | 6657 | 383030 |
| authored | parameter | varying rational | nonuniform / outer only | 371 | 189641 |
| guide | parameter | unit | gauge / outer + hole | 2249 | 377553 |
| guide | parameter | varying rational | gauge / outer + hole | 43956 | 377727 |
| guide | arc | unit | gauge / outer + hole | 252194 | 379843 |
| guide | arc | varying rational | gauge / outer + hole | 491894 | 380017 |
| guide | parameter | varying rational | nonuniform / outer only | 65502 | 189641 |

The original path/control/cap inputs were retained from the first failing
matrix. Before the correlated carrier fix the guide cases exhausted 1,000,000
cells. All ten now pass within the same unchanged primitive and aggregate limits.
The worst case uses 491894 cells. No sampled or retained geometry replaces the
original filled-volume embedding proof, and no failed classification is promoted.

Commands, exit codes and complete summaries are in result.json. Logs are retained
losslessly compressed; SHA256SUMS binds every receipt. environment.json records
the pinned compiler and test settings. Source manifests include tracked and
untracked .rs, Cargo.toml, Cargo.lock and Rust toolchain files, not the host's
global Cargo configuration. Whitespace checks passed; new standalone Rust files
were formatted. Existing compact native modules retain their surrounding style.

This packet qualifies eight hollow combinations of two frames, two clocks and
two affine-law modes, plus two simply connected nonuniform rational-weight cases.
It does NOT qualify the full rational-weight x hole x clock Cartesian product,
general miter/loft source binding, smooth cap/wall G1/G2, singular/polar cap modes,
all original geometric strata, packaged WASM, independent STEP or remote CI.
The full thread goal remains active. Old qualification packets remain unchanged.
