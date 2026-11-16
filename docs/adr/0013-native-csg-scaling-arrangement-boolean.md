# ADR 0013: Native CSG scaling through an own arrangement Boolean

- Status: proposed
- Date: 2026-11-15
- Decision: proposed (needs repository-owner acceptance)
- Evidence: `crates/polygon-core/examples/csg_ladder.rs`,
  `crates/polygon-core/examples/bench_csg_fixtures.rs` (rbench baseline in
  `tools/manifold-bench/rbench-baseline.macos-aarch64.json`),
  `crates/polygon-core/examples/bench_profile_triangulation.rs`,
  [`csg-scaling-2026-09-19.md`](../design/csg-scaling-2026-09-19.md),
  [`recommendation.md`](../recommendation.md) "Native P0 — CSG kernel scaling"

## Context

The `manifold` engine class (ADR 0002) evaluates OpenSCAD meshes with the own
`polygon-core` BSP Boolean, fronted by exact shortcuts (separated, nested and
identical operands, the prism arrangement for drilled plates) and n-ary
folding by bound connectivity. The roadmap asked to choose between two ways
of removing the remaining scaling limit:

1. an arrangement-based mesh Boolean on `cad-predicates` (exact orientation
   and incircle predicates already used by the profile triangulation);
2. depending on `manifold-rust` (a Rust port of Manifold 3.5).

## Measurements

Apple M4 Max, 16 cores, macOS, release build, single thread, 2026-11-15.
The machine was shared with other builds (load average 10 to 26 during the
runs), so treat single numbers as order of magnitude; the rbench baseline is
the median of 100 samples. Native numbers: `csg_ladder`, median of 3 after a
warmup. Manifold numbers: `manifold-3d` 3.5.1 WASM in Node from
`tools/manifold-bench` (the C++ library that `manifold-rust` ports; its
sphere tessellation has fewer triangles at the same `$fn`).

| Workload | Own kernel | manifold-3d |
| --- | --- | --- |
| `medium-csg`: panel minus 64 cylinders, `$fn=32`, bridge policy | 33 ms (44.7 ms with the previous ear clipper) | 34 ms |
| same, BSP only (no prism arrangement) | refused: work budget in result validation | — |
| panel minus 16 / 36 cylinders, BSP only | 46 / 228 ms (previous ear clipper: refused, stitch budget) | 9 / 20 ms |
| two overlapping spheres, union, `$fn=16 / 24 / 32` | 5.5 / 18 / 46 ms | 0.7 / — / 1.7 ms |
| same, `$fn=48 / 64` | refused: work budget in BSP construction | 2.7 / 4.7 ms |
| same, `$fn=96 / 128` | refused: input admission cap (10k triangles) | 9.5 / 16 ms |
| same, `$fn=256` | refused | 61 ms |
| three separated spheres, `$fn=128` | 0.4 ms (exact join, 48k triangles) | 17 ms |
| three separated spheres, `$fn=192 / 256` | refused: `MAX_MESH_TRIANGLES = 100 000` | 72 ms at `$fn=256` |

Profile triangulation (`bench_profile_triangulation`, 3204 vertices, 10×10
holes): monotone sweep 2.0 ms, previous ear clipping 12.8 ms.

Reading:

- Planar workloads are solved: with the monotone triangulation the prism
  path and coplanar `simplify` no longer refuse multi-hole caps, so even the
  BSP-only drilled panel stops re-fragmenting (16 and 36 holes now pass) and
  the bridge path matches Manifold on `medium-csg`.
- Curved intersecting operands are not: BSP construction grows roughly
  quadratically (5.5 → 18 → 46 ms for 2.4× input steps) and refuses at
  `$fn=48` (4.4k input triangles), while Manifold stays near linear up to
  `$fn=256`. No budget change fixes this; it is the BSP representation.

## Decision (proposed)

Build an own arrangement-based mesh Boolean on `cad-predicates`; do not
depend on `manifold-rust`.

1. **Provenance.** ADR 0010 forbids translating foreign kernel code and
   requires a new ADR, upstream hash, notice and approved `LicenseRef` for
   any port or vendored module. `manifold-rust` is a derived port of a
   foreign kernel, so adopting it is a provenance decision, not an
   engineering one, and would put a second Boolean (with its own tolerance
   model, not ADR 0006 predicates) under the `manifold` engine class.
2. **Measured gap is local.** Only curved intersecting operands fail. The
   own kernel already handles planar, separated and nested workloads at or
   above Manifold speed, so the new algorithm replaces one stage (BSP
   clip/stitch), not the pipeline (admission, n-ary folding, audits,
   lineage, face ids).
3. **Algorithm, from published sources only.** Triangle-triangle
   intersection by exact `orient3d` signs, intersection segments inserted
   into each face as constrained edges, faces retriangulated with the
   monotone sweep plus constrained Delaunay flips already in
   `planar-geometry`, cells classified by winding number propagated across
   the arrangement (ray parity only to seed each component). Candidate
   pairs from the existing BVH. This keeps exact topology decisions with
   the snap/tolerance policy of ADR 0006.

## Consequences

- Implementation is large and is not part of this change. It is recorded as
  a follow-up: ROADMAP row S1 keeps "arrangement Boolean" as its remaining
  scope, with acceptance `dense-sphere/overlapping-*-fn32` at or below
  Manifold-order time and `$fn=64` overlapping spheres passing.
- `bench_csg_fixtures` gates `medium-csg` and the re-scoped `dense-sphere`
  cases at the 15% budget of `rbench-regression.mjs`. When the arrangement
  Boolean lands, add `$fn=64` and `$fn=128` overlapping cases and re-record.
- `dense-sphere` as defined by `bench:cpu` (three `$fn=256` spheres in one
  mesh, 195k triangles) stays refused by `MAX_MESH_TRIANGLES`; raising that
  cap is tied to the 32 MiB transport limit and is a separate decision.
- If the owner later prefers `manifold-rust`, the prerequisite is an ADR
  under ADR 0010 (upstream hash, notice, licence) and a differential run of
  `tools/manifold-bench/differential.mjs` against it.
