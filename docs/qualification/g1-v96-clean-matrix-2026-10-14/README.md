# G1 v96 complete semantic matrix

All 4740/4740 work units passed across 132/132 clean fragments on source `de021272cbe5c26ad0778945c00b2cf5ad2fbbf6`. All six frozen environments passed: Ubuntu Node 20/22, macOS Node 20, Windows Node 20, Chromium and WebKit. No missing fragments, skipped units, retries or aggregate errors.

[Hosted execution](https://github.com/themoretheless/open-scad-viewer/actions/runs/38522799538) completed successfully. `hosted-result.json` is the exact downloaded aggregate. `local-verification.json` records a second successful aggregation of all downloaded evidence with the frozen harness at the same source commit. `fragments.tar.gz` preserves all downloaded fragment records, preflight identities, stdout/stderr and checksums. Every archived file was compared byte-for-byte with its downloaded original. `manifest.json` binds the evidence files; `github-artifacts.tsv` preserves GitHub archive identities.

To reproduce verification, check out the source commit in a separate clean checkout, extract the archive to a temporary directory, then run:

```sh
node scripts/g1-github-actions.mjs aggregate --artifacts /absolute/extracted/directory --source-sha de021272cbe5c26ad0778945c00b2cf5ad2fbbf6 --output /absolute/result.json
```

This closes execution of the full G1 v96 matrix for the frozen source and artifact. The frozen qualification approval and production cutover remain unchanged (`not-approved`, `false`); no approval is inferred from matrix success. Subsequent source or artifact changes require a new frozen candidate run.
