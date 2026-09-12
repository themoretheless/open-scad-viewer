# G1 v18 candidate-run execution notes

Plan: `docs/qualification/semantic-manifold-g1-plan-v18.json`  
Result: `output/qualification/semantic-manifold-g1-candidate-run-v18/result.json`  
Helpers:
- `scripts/run-g1-candidate-clean-fragment.mjs`
- `scripts/run-g1-ubuntu-docker-fragment.mjs`
- `.github/workflows/g1-qualification-clean.yml` (workflow_dispatch)

## Classification rule

| Classification | Counts toward u07 / G0.14 close? |
| --- | --- |
| `clean-post-freeze` | yes, if status=passed and cleanRunDefinition proven |
| `discovery-only` | **no** — supplemental only |

Do not import pre-freeze greens. Do not flip `qualificationApproval` while only discovery fragments exist.

## Current local posture (2026-09-12)

- Frozen plan artifact/bundle digests still match the tree (validator green).
- Harness + discovery trail committed (`d31a269` and follow-ons).
- macOS `macos-node20` MCP rows: all required discovery fragments passed (supervisor ×5, stdio ×10).
- Ubuntu via Colima qemu `node:*-bookworm` linux/amd64: **discovery-only**.
  - Fixed-runner batch run-1 finished: `oracle-differential` + `comparator-mutations` passed on node20/22; failed run-1: `semantic-special-terminals`, `node-worker-identifiers`, `dependency-and-cutover-audit`, `mcp-supervisor-hard-kill`, `mcp-stdio-store-isolation` (qemu timeouts / pin drift / missing browser Worker surface).
  - Parallel ubuntu queue may still append more discovery fragments.
- **Retries inside the same candidate-run id are forbidden for clean-post-freeze.** Discovery retries already happened after infra failures → a future clean run needs a fresh `candidateRunId` / plan amendment.
- Browser Chromium/WebKit actual+memory and Windows rows not closed.
- `completedWorkUnits` for clean-post-freeze remains **0**.

## Closing G0.14

1. Fresh OS jobs per clean run on ubuntu-24.04 / macos-15 / windows-2025 as listed.
2. Exact Node archive digests + npm 10.9.8 + sanitized env allowlist.
3. Wipe caches / `npm ci` per protocol; build geometry kernels **inside** the target OS; no result reuse; no retries.
4. Prefer native Ubuntu runners (GH `workflow_dispatch`) over Colima qemu for timeout-sensitive rows.
5. Publish append-only `result.json` with `classification: clean-post-freeze` for all 4740 units.
6. Then solo dual-role may flip `qualificationApproval` (org-independent seat remains recorded-nonapproval).
