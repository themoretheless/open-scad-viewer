# Joint central-cap and transition reconstruction — 2026-10-31

Qualified bounded construction: the origin corner of an axis-aligned 10 mm cube. Three degree-7 tensor patches share a triangular central cap; three degree-5 tapered strips meet that cap and the retained source planes. Explicit shared edge/vertex ownership closes the boundary. The source model is immutable; seven retained source vertex identities survive.

## Authored parameters

Common central contact radius 0.75 mm, strip stop 5 mm, profile handle fraction 0.214 of chord. Far contact radii along X/Y/Z are 0.13/0.20/0.35 mm. The earlier 0.10 mm contact is enlarged to 0.13 mm; the strips vary in radius, rather than preserving constant nominal radii. Radius Bernstein coefficients [R,R,R,r,r,r] make its first two derivatives vanish at both ends.

## Terminal results

- Closed boundary: WithinTolerance; all trim regions valid. 15 faces, 36 edges, 23 vertices; Euler characteristic 2.
- All 15 complete trimmed faces: absolute principal curvature Within the 10 mm^-1 limit. Largest conservative upper bound 9.99980609341556 mm^-1. These are field covers, not sampled maxima.
- All 15 faces contained in the original cube bounds with 1e-7 mm positional allowance. This is a boundary containment result, not a global embedding certificate.
- All 15 selected complete smooth interfaces: positions proven at 1e-7 mm; G2 WithinTolerance at normal tolerance 1e-5 and shape-tensor tolerance 1e-4 mm^-1. Includes six source-plane/strip contacts, six cap/strip half-seams and three internal cap seams. No tolerance relaxation.
- Independent OCCT 7.7.2.0 STEP import: one valid solid / one shell, matching topology. Numerical volume 999.2073095267439 mm^3. OCCT basic validity and numerical volume do not certify global self-intersection freedom.
- Three focused tests passed: authored control/radius jets; closed ownership/source identity; atomic refusal of inconsistent or unsupported input.

Original cube edges and the three planar far-stop boundaries remain sharp. They are outside the declared G2 seam set. General NURBS trimming, arbitrary-angle corners, global self-intersection certification, WASM/worker/UI integration and acceptance of bracket/flange/housing remain open. `qualifiedForDeclaredGates` is deliberately narrower than general CAD acceptance; `body.json` is the pre-qualification input snapshot and retains `accepted:false`.

## Reproduce

```sh
CARGO_TARGET_DIR="$PWD/output/qualification/central-graph-dev-target" cargo test --offline --manifest-path crates/Cargo.toml --config 'profile.dev.package.brep-core.opt-level=2' -p brep-core --test corner_patch_network
CARGO_TARGET_DIR="$PWD/output/qualification/central-graph-dev-target" cargo run --offline --manifest-path crates/Cargo.toml --config 'profile.dev.package.brep-core.opt-level=2' -p brep-core --example corner_patch_body_diagnostics -- output/qualification/corner-patch-body-recheck
/Users/themoretheless/.codex/tmp/projective-occt-20261013/bin/python scripts/verify-central-graft-step-occt.py output/qualification/corner-patch-body-recheck/candidate.step --faces 15 --edges 36 --vertices 23
```

`report.json` contains terminal per-face/per-seam results. `candidate.occt.json` records independent STEP evidence. `source-hashes.json` binds the principal authored implementation/test files; other concurrent workspace edits are excluded from this change's claims.
