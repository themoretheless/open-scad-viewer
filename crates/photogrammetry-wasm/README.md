# photogrammetry-wasm

Browser shell of the photogrammetry host ABI: a thin `cdylib` that re-exports
the `photogrammetry-ffi` core for `wasm32-unknown-unknown`. The module must
stay import-free (enforced by `scripts/build-photogrammetry.mjs`, which embeds
the artifact into `src/generated/photogrammetry/bytes.ts`).

```sh
node scripts/build-photogrammetry.mjs
```

The TypeScript host bindings live in `src/services/photogrammetry/`.
Responses come back as packed u64 (`len << 32 | ptr`), sound only because
wasm32 linear memory pointers fit 32 bits.
