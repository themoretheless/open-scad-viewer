# Declared global sweep fixture audit — 2026-10-03

Local unpublished. Full goal remains active; this finite audit is not a universal theorem or full-boundary smoothness proof.

All 27 native/STEP fixtures were reconciled against independently imported OCCT results. All 27 have certified actual retained-wall charts within the shared 100,000-cell budget (largest usage 15,312). All external geometry, edge/pcurve ownership, holes, manifoldness and material orientation checks passed.

25 fixtures have positive native whole-boundary embedding, complete face-pair classification, complete shell-nesting forests, and orientation matching even/odd nesting depth. Filled cap reports must cover exactly the selected cap faces with no unproved wall contacts; wall-chart reports must cover exactly all other faces. The two deliberate unproved cases (uncorrected spatial cap and nondyadic affine-oblique cap) retain negative Solid admission, absent nesting and unknown orientation. Wall injectivity alone never promotes these bodies.

Each OCCT result now records the SHA256 of the imported STEP file. The global verifier requires identity between the actual file, native export manifest and imported oracle; matching names alone cannot reuse an unrelated oracle.

Seven adversarial metadata changes were rejected: reversed orientation, cyclic nesting, omitted cap certificate, unexpected admission refusal, omitted wall chart, exceeded chart budget and wrong external artifact hash. The positive matrix and these rejection checks are retained alongside the manifest and independent oracle output.

Reproduce with: node scripts/verify-sweep-global-matrix.mjs <manifest.json> <occt.json> <output.json>. Native export uses scripts/export-sweep-step-oracle.mts; OCCT imports use scripts/verify-sweep-step-occt.py.

Remaining goal work includes a requirement-by-requirement mode audit outside this finite corpus, remaining applicable smoothness and publication/CI. The existing runtime artifact remains SHA256 47e6d7c9bd822449f4d845e59ba669a45dbcab1a48c9059285422a51815802e3; source snapshot /private/tmp/open-scad-viewer-sweep-native-final-2026-10-03.
