# G0 v4 / G1 v21 repository and kernel re-freeze

After G0 v3 and G1 v20 were frozen, the Rust workspace manifest, lockfile,
qualification-bound sources, and generated geometry WASM changed. Those bytes
cannot be attributed to the old snapshots. This amendment therefore advances
the toolchain fingerprint to G0 v4 and opens a separate G1 v21 candidate.

G0 v1-v3, G1 v1-v20, and their v19/v20 status records remain byte-immutable
historical evidence. G0 v4 records current bytes and observed toolchain only; it
does not close G0. G1 v21 preserves the finite v20 contract, recomputes current
binding digests without changing membership, and starts with zero completed
clean runs and zero completed work units.

Prior results remain discovery-only and may not be imported.
`qualificationClaim` remains `none`, qualification approval remains
`not-approved`, u07 remains unresolved, and production cutover remains
prohibited.

Preparation is read-only:

```sh
node scripts/refresh-qualification-fingerprints.mjs --check
```

Publishing creates new files exclusively:

```sh
node scripts/refresh-qualification-fingerprints.mjs --write \
  --recorded-at YYYY-MM-DD \
  --reason 'Re-freeze current repository and generated-kernel bytes in G0 v4 and start G1 v21 with no imported evidence.'
```

The generator rejects changed historical anchors, source races, existing v21
results, and existing destination files. It does not execute qualification
work or create CI evidence.
