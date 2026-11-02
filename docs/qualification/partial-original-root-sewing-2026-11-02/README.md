# Partial original-root sewing

2026-11-02, native Rust qualification.

`Roof::split_natural_edge` transactionally splits an original natural boundary at an exact rational parameter. Child curves share the root object and exact root intervals; a single new vertex owns the common endpoint. Retained UV source lines are rebuilt from the original support, in original traversal order. The unchanged source/G2/curvature field owners remain attached.

`close_original_shell` now consumes partitions of original world edges instead of requiring each mapped edge to cover [0,1]. It checks complete source coverage, ordered adjacent intervals, shared root ownership and identical endpoint identities. Neighboring imported faces are subdivided in UV using those same source intervals, including reversed face-local traversal. Complete world/pcurve equality and closed incidence are freshly checked before returning a shell. Volume and global embedding remain independent admission gates.

## Verification

33 selected native tests passed: exact_graph_strip 19, frozen_graph_volume 4, validation_sampler 4, continuous_validation 6. Cargo check and git diff --check passed.

The new positive regression splits every natural outer edge of the compatible polynomial control body at exact 1/3 and 2/3 root parameters. Each child is used by both neighboring faces with opposite effective shell traversal (coedge sense XOR face-use sense). The resulting shell admits one native exact body, with global embedding and inherited G2/principal curvature <=10 mm^-1. Original source model and identities remain unchanged.

The negative regression checks invalid/degenerate rational parameters, exhausted work and attempted transition-edge edits. No child edge, vertex or loop is published on failure; original saved geometry revalidates.

## Open scope

This is the existing supported polynomial graph-source construction, not arbitrary rational NURBS trimming, arbitrary removal of part of a source boundary, holes/cavities, periodic contacts or mixed-radius 5+ junctions. Natural cuts are currently native API calls; frozen recipe integration and WASM/worker/UI transport for these new cuts are pending. Radius-law, unrestricted thickness, STEP/independent parts/history acceptance, full P0 and P2/P3 remain open. Packaged WASM was not rebuilt in this milestone.
