// Path-independent wasm builds. rustc embeds source paths in panic locations,
// and registry crates live under $CARGO_HOME, so without remapping every
// machine (developer checkout, CI runner) packs different kernel bytes and the
// frozen byte fingerprints only match the machine that recorded them.
import {homedir} from 'node:os'
import {join} from 'node:path'

/** Remap prefixes: cargo home (registry/git sources) and the checkout root. */
export function remapPathPrefixFlags(root, env = process.env) {
  const trim = path => path.replace(/[\\/]+$/u, '')
  const cargoHome = trim(env.CARGO_HOME || join(homedir(), '.cargo'))
  return [`--remap-path-prefix=${cargoHome}=/cargo`, `--remap-path-prefix=${trim(root)}=/src`]
}

/**
 * Extra cargo arguments and environment for a reproducible build. `--config
 * build.rustflags` merges with rustflags from config files, but cargo ignores
 * it when RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS is set, so those are extended
 * instead of silently losing the remap. The encoded (0x1F-separated) form
 * takes precedence over RUSTFLAGS and keeps paths with spaces intact.
 */
export function reproducibleCargo(root, env = process.env, extraRustflags = []) {
  const flags = [...remapPathPrefixFlags(root, env), ...extraRustflags]
  const inherited = env.CARGO_ENCODED_RUSTFLAGS
    ? env.CARGO_ENCODED_RUSTFLAGS.split('\x1f')
    : env.RUSTFLAGS ? env.RUSTFLAGS.split(/\s+/u).filter(Boolean) : null
  if (inherited) return {args: [], env: {CARGO_ENCODED_RUSTFLAGS: [...inherited, ...flags].join('\x1f')}}
  // JSON string escaping is valid TOML basic-string escaping (Windows backslashes).
  return {args: ['--config', `build.rustflags=[${flags.map(flag => JSON.stringify(flag)).join(',')}]`], env: {}}
}
