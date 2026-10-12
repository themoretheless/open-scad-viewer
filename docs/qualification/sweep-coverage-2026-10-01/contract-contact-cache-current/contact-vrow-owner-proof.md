# Immutable contact v-interpolation cache

The candidate is derived from frozen T, independently of the projection-cache candidate V. All geometry and interval arithmetic remain Rust-owned.

For a fixed immutable surface and knot-span pair, the homogeneous source controls and source weight range are fixed. The intermediate row for output index v depends on the original v knot vector and span, v-domain endpoint bits, and v output index. It does not depend on the u-domain. The cache key includes both span indices, the two v endpoint bits, and the v index; the source is an immutable borrow owned by the search.

Only successful original interpolate_planned results are cached. Each miss runs the identical original v-plan and row interpolation. Each hit copies the stored interval endpoints. Original homogeneous-control preparation, u-plan construction, u interpolation and translation/Jacobian arithmetic remain in the same order. The independent original surface_net_on and section_jacobian routines remain available and unchanged.

Storage uses an LRU of at most32 intermediate rows and normally at most2048 retained homogeneous controls. A single individually oversized original row evicts all others and may remain alone. This is not a restriction of original supported geometry. Temporary replacement allocations may coexist with old entries. Each cached control stores four outward intervals.

The search cell/domain charges, queue, splitting proposal, max_cells and max_domain_cells are unchanged. Cache construction does not certify geometry; all existing interval tests and refusals remain required.

The exact regression compares4374 cached nets against an independent original coefficient routine across degrees0..8, nine span pairs, three full/restricted/collapsed domains, and two repetitions. Successful exact regression is necessary evidence, not a full-mode theorem or performance proof. Full native, whole global report equality, complete face-pair cover, WASM and Rush/UI deadline qualification remain separate requirements.
