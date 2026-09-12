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
- macOS `macos-node20` MCP rows executed as **discovery-only** (Node 20.19.0 archive verified; npm pinned 10.9.8; full OS-job wipe / sanitized allowlist not claimed).
- Ubuntu fragments via Colima qemu use `node:*-bookworm` linux/amd64 — also **discovery-only** until frozen Ubuntu 24.04 image digests and wipe protocol are proven.
- Early Ubuntu attempts failed (apt/dpkg under qemu; cargo missing on pretest). Later discovery retries passed for some rows. **Retries inside the same candidate-run id are forbidden for clean-post-freeze** — a future clean run must use a fresh candidateRunId / plan amendment and never reuse these discovery fragments.
- Browser Chromium/WebKit actual+memory rows still unexecuted (linux supervisor + Playwright trees).
- `completedWorkUnits` for clean-post-freeze remains **0** while only discovery-only fragments exist.

## Closing G0.14

1. Fresh OS jobs per clean run on ubuntu-24.04 / macos-15 / windows-2025 as listed.
2. Exact Node archive digests + npm 10.9.8 + sanitized env allowlist.
3. Wipe caches / `npm ci` per protocol; no result reuse; no retries.
4. Publish append-only `result.json` with `classification: clean-post-freeze` for all 4740 units.
5. Then solo dual-role may flip `qualificationApproval` (org-independent seat remains recorded-nonapproval).
