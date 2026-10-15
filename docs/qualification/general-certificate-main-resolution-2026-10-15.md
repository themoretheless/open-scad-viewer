# General boundary certificate: main integration

This integration resolves all 154 index conflicts between certificate commit
`12bda09ecac7a53f9aea043650dd9cf3825b7fc3` and main
`e3a2d07157e7e0b8666e33355c2fba6a5c61db7b`. It preserves independently authored
work and does not import qualification credit from either parent.

## Native ownership

The certified BRep, NURBS, intersection, STEP and Rust language owners remain
authoritative. Main's exact planar Bézier root reductions, batch curve sampling,
scalar error ABI, mesh display, shell/bevel, SVG and lattice analysis additions
are integrated with these owners. The lattice weak-spot adapter calls the existing
`polygon_core::lattice_tools` owner instead of duplicating its implementation.

The general frame/cap certificate, rounding, corrections and Solid admission
remain entirely in Rust. TypeScript transports native results and schedules the
browser. Existing Rust sweep transport and worker payload helpers remain shared;
older copies introduced by recursive merge are removed. The curve/surface report
continues to distinguish completed search from a certified intersection: an
uncertified binary64 result cannot become certified by changing a test expectation.

Main's unrelated GPU preview/reference implementations are retained. Six such
TypeScript/WGSL modules are explicitly listed as remaining native migration work
in the ownership registry; they receive no Rust ownership or certificate credit.

## Historical evidence

Every differing incoming immutable qualification record is preserved with exact
Git bytes under `main-history-e3a2d0715`; the manifest binds all 124 records to
their source commit, byte lengths and hashes. Canonical existing archives remain
unchanged. Deleted historical archives are restored, and the frozen registry is
factored without dropping any previously frozen hash.

OwnRust 73, G0 87 and G1 104 describe the pre-merge certificate. Their passing
results are historical evidence only. The merged source, Cargo lockfile and WASM
require new successors and a fresh zero-credit G1 matrix. Mathematical budgets,
individual test deadlines and the no-codec/global-WASM CI checks are preserved.

## Scope

Certification is a sufficient whole-domain proof with explicit finite work
limits; singular, intersecting or unresolved input is refused. Moving-frame
approximation plus retained-body validation does not establish ideal-body
embedding or G1/G2. Sharp seams remain explicitly C0. Finite Node, Chromium and
independent STEP checks are recorded separately from the full semantic matrix.

The subsequent main `c406d1908a0ac324388516dc4b0645c4887edaa1` has exactly
one tree change relative to `e3a2d0715`: the budget-one retained quarter-chart
assertion. That assertion is applied to the resolved tree. Repeated recursive
merge conflicts arise from multiple historical merge bases, not new source
changes; the previously reviewed resolution is retained in full.

The library audit also restores centralized workspace dependency ownership in
all eight affected manifests, retaining bridge-svg's new mesh-section dependency.
The nurbs-intersect codec dependency is optional again. The strict audit includes
all 18 native kernels, including both intersection crates. Cargo's resolved
full-workspace feature sets are identical before and after this repair for all
360 packages; the no-default-features graph now excludes the wire codec.

The runtime-manifest diagnostic still distinguishes the historical qualified
active manifests from newly executed kernel bytes. Rebuilding/refreezing alone
cannot activate pending manifests or grant production qualification. This known
mismatch is retained as a diagnostic, never converted into a passing assertion.

The intersection query codec module retains its explicit `cfg(feature = "codec")`
gate, matching the restored optional dependency; typed native queries compile
without importing transport serialization. Default-feature behavior is unchanged.
