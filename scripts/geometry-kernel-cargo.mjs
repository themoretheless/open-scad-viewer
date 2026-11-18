import {reproducibleCargo} from './reproducible-cargo.mjs'

// Release WASM has typed public diagnostics and separate debug builds. Removing
// compiler panic locations helps preserve the bounded delivery size without changing
// operations, source-language spans or native profiles. The pinned nightly
// toolchain supports this flag; keep all inherited build flags and path remaps.
export function geometryKernelCargo(root, env = process.env) {
  const inherited = env.CARGO_ENCODED_RUSTFLAGS
    ? env.CARGO_ENCODED_RUSTFLAGS.split('\x1f')
    : env.RUSTFLAGS ? env.RUSTFLAGS.split(/\s+/u).filter(Boolean) : []
  return reproducibleCargo(root, {...env,
    CARGO_ENCODED_RUSTFLAGS: [...inherited, '-Zlocation-detail=none'].join('\x1f'),
  })
}
