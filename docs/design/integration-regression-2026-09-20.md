# Integration Regression Check

Source snapshot: 21acaa2b. Verification performed after bounded worker inputs,
native preview/job writer ports and signed truss field preview integration.

| Check | Result |
| --- | --- |
| Vitest, maxWorkers 2 | 3438 passed, 9 failed; 361 files, 247.34 s |
| geometry-bridge native tests | 288 passed, zero failed across 8 reported suites |
| Vue typecheck | Passed |
| MCP TypeScript project | Passed |
| Runtime manifest audit | Expected failure: actual artifact differs from admitted archived identity |

The nine JS failures are unchanged in identity: engineManifest (1),
g0ToolchainFingerprints (6), g1GithubActionsQualification (1),
qualificationPlanArtifact (1). These checks remain enabled. No frozen archive,
expected hash or qualification claim was rewritten to make the run green.
This is not a passing overall qualification gate.

The native run includes current Rust source, including the writer refactors.
The tolerant sphere/cylinder placement sweep passed after 128.19 s. Native
success does not prove that the checked-in WASM was rebuilt from that source.

The fresh runtime audit verifies embedded/public artifact equality:
7,723,512 bytes, SHA256
909b94a4b895db447a184bcc6cfb544a226b51589b94b124424da6670eb0b8ca.
Mesh and B-rep each build one cube of volume 1 mm3, but both selected qualified
archives declare fde93f46f61330609eaab5c7470be0bb24a2ff14a64d0b83788c5c0051d82af6.
Their internal canonical digest and descriptor checks pass; artifact binding
does not. The audit exits 1 with qualificationClaim none.

Logs: `/private/tmp/osv-integration-regression.log`,
`/private/tmp/osv-geometry-regression.log`,
`/private/tmp/osv-runtime-binding-current.json`.

Outstanding user-visible decision: explicit unqualified development admission
while retaining strict qualified gates, or refusal until the exact artifact
is qualified. Neither policy has been silently selected here. Remaining
branch integration and runtime binding work prevent goal completion.
