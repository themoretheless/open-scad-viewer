# Native client geometry boundaries

Geometry proposals and numerical witness checks from the eight previously unreviewed client files now execute through `cad_client_geometry`. TypeScript keeps document edits, worker lifecycle, structural response validation and exact source identity comparisons.

- `osv-math::profile_plane` infers and projects authored controls into a common plane. It retains the 0.0000001 mm projection refusal and bounds input controls.
- `osv-math::witness_bounds` checks transported distance, material enclosure and quotient dominance consistency. These necessary checks do not create geometric certificates.
- `planar-geometry::sketch_shapes` constructs numeric rectangles and capsule arc definitions. Slot endpoints use the retained arc sampler's native arithmetic; invalid coordinates and dimensions return no shape.
- `nurbs-core::surface_injectivity` supplies source-derived projective and polar proposals. A proposal is not an injectivity certificate.
- `brep-core::wall_search` owns sample placement, normal selection and candidate rays. Budgets are 1..256 candidates, at most four source samples per requested candidate, and at most one target sample per requested candidate. Models above 10000 faces refuse. Samples never certify coverage; the original trimmed-volume audit still decides admission.
- `geometry-bridge` adapts these typed kernels and the bounded source preview allocation policy.

The optional explicit evaluator hook remains available for native query locations. It transports caller-provided values into the same native proposal selection and cannot mark a singular sample as covered.

The NURBS catalog references current implementation files, including definitions reached through module re-exports. Catalog statuses and remaining qualification are unchanged. The old ownership census is retained; `audit-algorithm-owners --check` compares the new 2026-10-09 census.
