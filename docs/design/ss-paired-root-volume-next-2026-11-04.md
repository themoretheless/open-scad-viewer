# Independent volume and outward material: design and implementation

Updated 2026-11-13: removed 8 completed design items (original `surface_flux`
integration, Jordan-disk bounds, complete root-arc enclosure cache, root-free
cell propagation, inside/outside/unresolved cell rules, bounded interval
summation, global orientation with an immutable material owner, and the
genus-zero/genus-one/reversed-U regressions). Evidence:
`crates/brep-core/src/model/mixed_root_model/closed_cut/jordan/census/sew/{volume,material}.rs`
(`volume_bounds`, `admit_material`), `crates/brep-core/tests/nested_census_arrangement.rs`
and `ss-paired-root-volume-progress-2026-11-04.md` with its frozen native receipt.

Implemented for paired root-only boundaries only. The complete general Boolean
goal and its other missing strata remain open:

- Through-hole/rim cases still need the general cross-source stitching gate.
- All-face-pair coverage, open-rim endpoints/splits, arbitrary singular and
  coincident strata, seams/poles, nested shell material/cavities, healing,
  lineage and bridge/WASM delivery remain required by the original objective.

The paired-shell certificate is a sufficient step, not complete Boolean output
assembly.
