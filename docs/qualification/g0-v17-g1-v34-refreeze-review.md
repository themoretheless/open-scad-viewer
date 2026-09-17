# G0 v17 / G1 v34 no-claim re-freeze

This append-only amendment preserves G0 v16, G1 v33, and all earlier artifacts byte-for-byte. V33 cannot admit GitHub Actions evidence because its frozen bindings do not include the new workflow or `scripts/g1-github-actions.mjs`.

V34 binds the exact workflow, evidence-producing GitHub harness, changed fragment selectors, and hosted-runner identity freeze. The runner freeze records Ubuntu 24.04 `20260907.300.1`, macOS 15 arm64 `20260907.0337.1`, and Windows 2025 `20260913.261.1`, along with immutable action commit pins.

The official Node archives are downloaded and hash-checked as identity evidence only. The executable runtime comes from the commit-pinned `actions/setup-node` action and is independently checked for version, platform, and architecture. npm's downloaded archive is both integrity-checked and installed. Playwright package locks and the complete Chromium/WebKit revision trees remain hash-checked.

Every dispatch supplies an exact commit SHA that must equal `github.sha`, checkout `HEAD`, and every fragment's source identity. Aggregation rejects missing, duplicate, failed, foreign-commit, wrong-environment, wrong-runner, wrong-unit, or checksum-invalid artifacts. A fragment receives credit only after all checks pass, once, for its frozen row/run/environment key.

The matrix remains 4740 planned work units. V34 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
