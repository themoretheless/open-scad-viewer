# G1 v98 complete semantic matrix

All 4740/4740 work units passed across 132/132 fresh fragments on source `3c19575792ef5002fe9fb85357bd0b36075caab5`. All six frozen environments passed: Ubuntu Node 20/22, macOS Node 20, Windows Node 20, Chromium and WebKit. No missing fragments, skipped units, retries or aggregate errors.

[Hosted execution](https://github.com/themoretheless/open-scad-viewer/actions/runs/38583819692) completed successfully. `hosted-result.json` is the downloaded aggregate. `local-verification.json` records a second successful aggregation at the exact source commit. The reports agree except for their generation timestamps. `fragments.tar.gz` preserves every downloaded fragment, preflight, stdout/stderr and checksum. Every archived file was compared byte-for-byte with its downloaded original. `manifest.json` binds the files; `github-artifacts.json` retains GitHub archive identities and digests.

To verify, check out the source commit in a separate clean checkout, extract the archive to a temporary directory, then run:

```sh
node scripts/g1-github-actions.mjs aggregate --artifacts /absolute/extracted/directory --source-sha 3c19575792ef5002fe9fb85357bd0b36075caab5 --output /absolute/result.json
```

This closes execution of the G1 v98 matrix for the frozen source and artifact. The qualification approval and production cutover remain unchanged (`not-approved`, `false`). It does not close the separate full native workspace CI gate: Linux subsequently rejected six outdated golden payload hashes, which require a verified successor. This successful matrix is historical evidence after any source change and is never imported as credit into a successor run.
