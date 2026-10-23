# geometry-wasm

Browser shell of the geometry linear-memory ABI: a thin `cdylib` that
re-exports the `geometry-bridge` core (`abi`) for `wasm32-unknown-unknown`.
The module must stay import-free (enforced by
`scripts/build-geometry-kernels.mjs`, which embeds the artifact into
`src/generated/geometry-kernels/`).

```sh
node scripts/build-geometry-kernels.mjs
```

The TypeScript host bindings live in `src/services/geometry/`. Responses come
back as packed u64 (`len << 32 | ptr`), sound only because wasm32 linear
memory pointers fit 32 bits.
