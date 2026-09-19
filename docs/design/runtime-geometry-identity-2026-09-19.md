# Runtime geometry identity mismatch

## Reproduction

Run `node --import tsx scripts/audit-runtime-geometry-identity.mts` after building
the geometry kernel. This read-only command loads the actual active manifest
selection from core, hashes the generated WASM, reports both providers, and exits
1 on mismatch. It does not rewrite evidence, compile or execute the kernel, or
claim qualification. Packed/raw artifact equality remains a separate dist check.

Observed with source baseline `fd03ebcf`:

- Active mesh: `own-rust-node-v1`, qualification status `qualified`.
- Active B-rep: `brep-closed-v1`, qualification status `qualified`.
- Both expected: `sha256:fde93f46f61330609eaab5c7470be0bb24a2ff14a64d0b83788c5c0051d82af6`.
- Generated 7,669,594-byte kernel: `sha256:70ef949a93d68f9a9f2ca5b81f47f878ebda62f373d6644cd25a8d17944e505d`.

## Cause and consequence

`qualifiedRuntimeManifestVersion` falls back from the pending current catalog
entry to an older admitted manifest. `GeometryBuildEngine` copies that identity
into its providers and execution descriptors, but both providers warm and execute
the shared current kernel. Manifest fallback is therefore not binary rollback.
Checking a provider's copied fields against the same manifest cannot prove its
binary identity. This mismatch is not fixed by the new diagnostic.

The prior evidence auditor checks the pending v9/current artifact binding; the
new command checks the separate active runtime binding. Both checks are needed.
The diagnostic's three unit tests cover active selection, mixed match/mismatch
reporting, archive preservation and the distinction between byte equality and
qualification. A matching hash does not establish conformance or deployment.

## Required correction

Publish a new immutable artifact/manifest/evidence tuple with current dependency
and source bindings, then perform the required qualification before admitting
that version. Runtime admission must bind to the loaded artifact, not merely to
fields copied from a catalog. Falling back to an older admitted manifest is valid
only when the corresponding older binary is also selected and verified.

Do not mark the pending manifest qualified to make tests green, overwrite v1/v9
archives, or claim that changing CURRENT/ACTIVE version selection alone resolves
this. The runtime needs a coherent artifact selection and admission contract;
until then, published qualified descriptors do not certify the current binary.
