# G1 v100 complete exact-artifact matrix

All 4740/4740 work units and 132/132 fresh fragments passed at source 6dff3e94a2b0eb56f7024d6c2faf9766f6badbf3. No missing units, skipped units, retries, imported results or aggregation errors. Hosted and independent local aggregation agree except for their generation timestamps. Every archived fragment, preflight, stdout/stderr and checksum was checked byte-for-byte against its downloaded original.

The exact frozen geometry artifact is 12241a08fbb3bd749d90f6ff9955d7157601b1e433cb6ac33f7e0b4784872b47 (12007711 bytes). The observed Linux CI build is another variant, byte-identical to the previous CI build; no G1 credit for that other artifact or cross-builder identity is inferred. See wasm-build-observations.json.

Hosted run: https://github.com/themoretheless/open-scad-viewer/actions/runs/38596009263

To reproduce, check out the exact source commit, extract fragments.tar.gz, and run node scripts/g1-github-actions.mjs aggregate --artifacts /absolute/extracted/directory --source-sha 6dff3e94a2b0eb56f7024d6c2faf9766f6badbf3 --output /absolute/result.json.

Matrix success does not change not-approved qualification status or authorize production cutover. Full general CI is a separate delivery gate.
