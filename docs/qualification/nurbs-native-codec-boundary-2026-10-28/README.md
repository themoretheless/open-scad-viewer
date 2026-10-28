# Native reports/history versus optional wire codecs — 2026-10-28

The existing brep-core README requires native geometry, operations and typed reports without value-codec; optional codec enables wire models/reports and snapshots. The extra minimal build found 121 errors. This checkpoint fixes part of that contract without disabling geometric modules or making the optional codec dependency mandatory. The minimal build is still unqualified: **80 errors remain**, all E0432/E0433 references to value_codec in the source-definition layer.

The typed defeaturing batch, dry run, native report records and changed-region computation stay in `direct_edit/defeature_report.rs`. Its complete serialization/deserialization implementations, schema parsing and JSON methods move unchanged to a child `serialization.rs` enabled only by codec. The native Journal keeps NumericContract, capture, record, seal, FeatureHistory reconstruction, typed replay and contract/hash drift checks. Record adapters and JSON Lines parsing/encoding move into its optional serialization child. JSON-only tests follow that feature; native replay tests remain unconditional.

Two geometry-only contact-search record types and their search function were incorrectly hidden behind codec. They now remain native. The inverse-shear FragmentReport also remains native alongside its already unconditional typed transport function. No computational module or native geometry API was gated off to get a smaller build, and no mandatory serialization dependency or renamed minimal feature was introduced.

## Verification

The ordinary default-feature cargo check passes. Complete four-crate library suites pass **4202 tests, zero failures, six explicitly ignored**: brep-core 1998/0/5, cad-predicates 100/0/0, nurbs-core 1996/0/1, nurbs-intersect 108/0/0 (passed/failed/ignored). Native journal capture/replay, numerical/hash drift refusal, the fifty-operation JSON Lines replay, native defeaturing and all-size JSON report round trips pass. Six token/string-preserving comparisons confirm all moved legacy wire implementation bodies remain unchanged relative to 370fbf488; runtime round-trip/replay tests supply additional evidence.

The minimal check first fell from 121 to 78 errors in an intermediate revision that also gated region transport incorrectly. That gate was removed: the final code retains the native API and reports **80** errors. The earlier logs are retained, not counted as a successful minimal build. Remaining diagnostics involve retained source-definition values, metadata identity and replay helpers; they are not ignored or downgraded.

All seven changed Rust sources match the checked/staged hashes; rustfmt, whitespace, Git diff and the 239-module / 200-catalogue API map pass. Architecture retains the same 71 existing violations without raising limits. Source hashes, raw logs and the local delivery receipt are preserved in `/Users/themoretheless/.codex/tmp/nurbs-plan-recovery-commands-20261028/qualified-native-codec-boundary-20261028.tar.gz`, with an adjacent SHA256 file.

## Remaining work

Do not solve the remaining source-definition dependency by hiding whole geometry/qualification modules, forcing codec on, or substituting sampled/approximate identity. Native source identity and direct/mapped region/carrier recipes need typed original definitions and selectors. Serialization should encode those definitions in adapters; restoration must decode and freshly run the same native gates. Preserve existing schema, strict numeric recipe rules, source ownership and proof budgets. Internal mathematical requalification should consume typed originals, not serialize and restore a dynamic value merely to compare source identities.

This native-boundary repair is incomplete. It does not finish whole-body Boolean material selection/sewing, central G2 corners, offsets/shells, exact/tolerance simplification or WASM/Rush/UI/edited-body STEP release. All five original NURBS plan directions and their release gates remain active. No main push or complete-plan claim is made.
