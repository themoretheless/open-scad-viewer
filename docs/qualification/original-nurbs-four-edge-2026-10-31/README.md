# Original NURBS trimming and a four-edge G2 corner — 2026-10-31

## Implemented native operations

`region_replacement::trim_and_replace_contacts` partitions immutable original support faces along supplied 3D/UV contact curves, selects removed sectors by original edge/vertex ownership, and sews one replacement over the common authoritative rims. One construction cell/exact ledger spans the cuts and final replacement. Shared root isolation/edge partitioning is separate. Refusal exposes no intermediate sectors. The retained original NURBS surface definitions are unchanged; return maps and change sets refer to the caller's input. Smooth-edge IDs are requests for separate G2 qualification, not inherited positive receipts.

`propose_four_edge_corner` builds a degree-six central patch and four ruled G2 strips over a canonical closed pyramid with a genuine four-edge vertex. The original five support definitions remain unchanged under trimming. Explicit source-role provenance preserves tilted-face ancestry in the recognised `persist`/Modified change-set form. Four prescribed mid-section normal radii are 2.6, 2.8, 3.0 and 3.2 mm. Handles are quantised at 2^-32 mm and their analytic radius intervals remain within 1e-7 mm of those requests. The profiles smoothly reach zero curvature at the planes; a constant circular radius across the entire cross-section is not claimed.

## Terminal qualification

| Control | F / E / V | Whole G2 interfaces | Classified pairs | Whole-face curvature upper |
|---|---|---|---|---|
| Original rational curved stock, trimmed and sewn | 7 / 15 / 10 | 2 | 21 / 21 | 9.943636083130052 mm^-1 |
| Four-edge mixed mid-radius pyramid corner | 10 / 20 / 12 | 12 | 45 / 45 | 9.886340761951079 mm^-1 |

Both models have C0-closed boundaries, valid trim regions, exact original edge/lift agreement and wire joins, injective retained faces and complete global pair coverage. There are no unresolved or unvisited pairs. G2 uses unchanged position 1e-7 mm, normal 1e-5 and shape-tensor 1e-4 mm^-1 limits. All complete actual trimmed faces satisfy the 10 mm^-1 curvature limit. Existing sharp source/end edges are outside the selected G2 interface set.

The prism retains all six original rational support definitions. The four-edge case has 50 native filled-face containment covers: Z>=0 and 19-Z-sx*X-sy*Y>=0 for all four sign pairs, covering the convex original stock. Its three tensor probes per arm independently check the requested midline radii against actual surface geometry. Constructor radius intervals describe the analytic extruded midline law; the probes alone are not a whole-surface certificate. General source-material/nesting certification is not claimed for arbitrary NURBS input.

Both STEP files pass OCCT 7.7.2.0 import/basic B-rep validity, one-solid/one-shell topology and V/V through F/F interference search. All 12 independent STEP principal-radius probes lie inside the corresponding native intervals and within 1e-7 mm of requested radii. Exact Bernstein-volume integrals independently agree with STEP mass properties: 158.33658854165384 mm^3 for the prism (error 1.29e-11 mm^3) and 4440.006087703924 mm^3 for the four-edge body (error 9.10e-13 mm^3). The numerical volume comparison is distinct from native boundary/material proofs.

47 selected tests passed: 15 construction/integration tests and 32 low-level tests (19 hull-contact, 2 exact-retained-support and 11 face-domain tests). The stitched-contact filter has zero direct unit tests; the nonunit owned child-pcurve clocks are exercised by the closed trimming integration. These are selected regressions, not a full project/P0 command-matrix result.

## Proof changes

- Proof-only restriction of literal rectangular retained regions in a clamped unit linear tensor direction. Equal cross-direction rational weights make the map affine in that direction. Every changed pole is an exact guarded interpolation; no rounded restriction gets a receipt. Actual source surfaces, pcurves, knots and identities remain untouched. The full embedding audit runs again on the equivalent retained-face sets.
- Natural side admission accepts complete clamped positive constant-weight linear pcurves on their original nonunit clocks. Split child edges keep those clocks.
- Positive NURBS control-net hulls enclose complete supports. Exact affine chart images of closed trimmed UV boundaries bound complete filled faces. Failed enclosures consume their work and fall back to conservative inspection; zero budget remains unresolved.
- Tilted affine trimmed planes use their authoritative positive boundary-curve hulls after exact current-plane membership/noncollinearity checks. This removed four ghost-support contacts with the pyramid base. The unchanged-support-substitution counterexample initially failed during development; the guard was strengthened and all 19 hull-contact tests passed. No test or tolerance was relaxed.

## Scope still open

The generic trimming transaction requires already shared original root vertices and authored contact curves/pcurves. Automatic contact generation for arbitrary B-rep, unrestricted doubly curved NURBS fillet solving, periodic/multiple-chart source ownership and valence 5+ corner construction remain open. The four-edge builder is explicitly limited to the canonical H=19 mm, setback=3 mm pyramid and its admissible convex profile radius range. It is a qualified four-valent control, not unrestricted mixed-radius corner completion. Native qualification does not imply WASM/worker/UI integration; those were not rebuilt here.

`body.json` is the original immutable qualification input and retains `accepted:false`; terminal admission is in `report.json`. Summary model/STEP hashes bind the published data. Source hashes identify principal authored files; executable hashes identify the actual qualified drivers. Other concurrent workspace work is preserved and was not released by this task.

## Reproduce

```sh
export CARGO_TARGET_DIR="$PWD/output/qualification/central-graph-dev-target"
cargo test --offline --manifest-path crates/Cargo.toml --config 'profile.dev.package.brep-core.opt-level=2' -p brep-core --test trim_contact_replacement --test four_edge_corner --test corner_patch_network --test nurbs_edge_transition
for filter in boundary_hull_contact boundary_stitched_contact boundary_embedding::retained_rectangles face_domain; do
  cargo test --offline --manifest-path crates/Cargo.toml --config 'profile.dev.package.brep-core.opt-level=2' -p brep-core --lib "$filter"
done
cargo run --offline --manifest-path crates/Cargo.toml --config 'profile.dev.package.brep-core.opt-level=2' -p brep-core --example trim_contact_diagnostics -- output/qualification/original-nurbs-recheck
cargo run --offline --manifest-path crates/Cargo.toml --config 'profile.dev.package.brep-core.opt-level=2' -p brep-core --example four_edge_corner_diagnostics -- output/qualification/four-edge-recheck
```

Use the local OCP Python environment `/Users/themoretheless/.codex/tmp/projective-occt-20261013/bin/python` with `scripts/verify-central-graft-step-occt.py` (7/15/10 or 10/20/12 expected F/E/V), `scripts/verify-step-self-intersections-occt.py`, and `scripts/verify-four-edge-step-radii.py FOUR_EDGE_DIR`. Run `scripts/verify-contact-control-volumes.py DIR --kind prism` or `--kind pyramid` after the STEP mass-property check. These Python scripts are independent validation tools; native Rust owns construction and runtime qualification.
