import { describe, expect, it } from 'vitest'
import { reproducibleCargo } from '../scripts/reproducible-cargo.mjs'

/**
 * The wasm build scripts must strip machine-specific source prefixes, or the
 * packed kernels (and the frozen byte fingerprints bound to them) differ
 * between a developer checkout and a CI runner.
 */
describe('reproducible cargo flags', () => {
  it('remaps cargo home and the checkout root through build.rustflags', () => {
    const { args, env } = reproducibleCargo('/work/open-scad-viewer/', { CARGO_HOME: '/home/dev/.cargo/' })
    expect(env).toEqual({})
    expect(args).toEqual([
      '--config',
      'build.rustflags=["--remap-path-prefix=/home/dev/.cargo=/cargo","--remap-path-prefix=/work/open-scad-viewer=/src"]',
    ])
  })

  it('escapes Windows paths as TOML basic strings', () => {
    const { args } = reproducibleCargo('C:\\a\\repo\\', { CARGO_HOME: 'C:\\Users\\r\\.cargo' })
    expect(args[1]).toBe(String.raw`build.rustflags=["--remap-path-prefix=C:\\Users\\r\\.cargo=/cargo","--remap-path-prefix=C:\\a\\repo=/src"]`)
  })

  it('extends explicit RUSTFLAGS instead of being ignored by cargo', () => {
    const { args, env } = reproducibleCargo('/r', { CARGO_HOME: '/h', RUSTFLAGS: '-Ctarget-feature=+simd128  -Copt-level=3' })
    expect(args).toEqual([])
    expect(env.CARGO_ENCODED_RUSTFLAGS.split('\x1f')).toEqual([
      '-Ctarget-feature=+simd128', '-Copt-level=3', '--remap-path-prefix=/h=/cargo', '--remap-path-prefix=/r=/src',
    ])
  })

  it('keeps encoded rustflags and paths with spaces intact', () => {
    const { env } = reproducibleCargo('/my repo', { CARGO_HOME: '/h', CARGO_ENCODED_RUSTFLAGS: '-Cfoo\x1f-Cbar' })
    expect(env.CARGO_ENCODED_RUSTFLAGS.split('\x1f')).toEqual([
      '-Cfoo', '-Cbar', '--remap-path-prefix=/h=/cargo', '--remap-path-prefix=/my repo=/src',
    ])
  })
})
