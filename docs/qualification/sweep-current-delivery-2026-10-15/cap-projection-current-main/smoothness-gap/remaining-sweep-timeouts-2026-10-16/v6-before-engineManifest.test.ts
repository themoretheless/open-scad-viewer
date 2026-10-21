import { createHash } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import { describe, expect, it } from 'vitest'
import {
  CURRENT_GEOMETRY_MANIFEST_VERSIONS,
  GEOMETRY_MANIFEST_ARCHIVE,
} from '../src/core/geometryExecution'
import {
  archivedGeometryManifest,
  computeGeometryManifestDigest,
  immutableEngineManifestToWire,
  staticEngineManifestToWire,
} from '../src/mcp/engineManifest'

function sha256(bytes: Uint8Array): string {
  return createHash('sha256').update(bytes).digest('hex')
}

function independentCanonicalJson(value: unknown): string {
  if (value === null || typeof value !== 'object') return JSON.stringify(value)
  if (Array.isArray(value)) return `[${value.map(independentCanonicalJson).join(',')}]`
  return `{${Object.entries(value as Record<string, unknown>)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([key, item]) => `${JSON.stringify(key)}:${independentCanonicalJson(item)}`)
    .join(',')}}`
}

describe('immutable geometry manifest archive', () => {
  it('pins every version to its canonical public payload and freezes nested evidence', () => {
    for (const manifest of Object.values(GEOMETRY_MANIFEST_ARCHIVE)) {
      expect(computeGeometryManifestDigest(manifest)).toBe(manifest.manifestDigest)
      const {
        manifest_digest: _digest,
        manifest_resource_uri: _resource,
        ...payload
      } = staticEngineManifestToWire(manifest)
      expect(createHash('sha256').update(independentCanonicalJson(payload)).digest('hex'))
        .toBe(manifest.manifestDigest)
      expect(Object.isFrozen(manifest)).toBe(true)
      expect(Object.isFrozen(manifest.capabilities)).toBe(true)
      expect(Object.isFrozen(manifest.qualification)).toBe(true)
      expect(Object.isFrozen(manifest.dependency)).toBe(true)
      expect(Object.isFrozen(manifest.rollbackCompatibility)).toBe(true)
    }

    expect(() => {
      (GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'].capabilities as string[]).push('tampered')
    }).toThrow()
  })

  it('binds the current own Rust evidence to repository artifacts', async () => {
    const manifest = GEOMETRY_MANIFEST_ARCHIVE[CURRENT_GEOMETRY_MANIFEST_VERSIONS.mesh]
    const notices = await readFile(new URL('../THIRD_PARTY_NOTICES.md', import.meta.url))
    const lockfile = await readFile(new URL('../package-lock.json', import.meta.url))
    const parsedLockfile = JSON.parse(lockfile.toString('utf8')) as {
      packages: Record<string, { version?: string }>
    }

    expect(manifest.capabilityManifestVersion).toBe('own-rust-node-v2')
    expect(manifest.engineClass).toBe('mesh')
    expect(manifest.qualification).toMatchObject({
      status: 'baseline-pending',
      recordId: null,
      corpusVersion: null,
    })
    expect(sha256(notices)).toBe(manifest.dependency.sbomSha256)
    expect(sha256(lockfile)).toBe(manifest.dependency.lockfileSha256)
    expect(parsedLockfile.packages['node_modules/manifold-3d']).toBeUndefined()
    expect(JSON.parse(lockfile.toString('utf8')).packages['tools/manifold-bench/node_modules/manifold-3d']).toBeUndefined()
    expect(manifest.dependency.packageName).toBe('workspace:geometry-bridge')
    const wasm = await readFile(new URL('../src/generated/geometry-kernels/kernel_bg.wasm', import.meta.url))
    expect(manifest.kernelFingerprint).toBe(`sha256:${sha256(wasm)}`)
  })

  it('resolves only archived version and engine-class pairs', () => {
    expect(archivedGeometryManifest('own-rust-node-v1')?.engineClass).toBe('mesh')
    expect(archivedGeometryManifest('own-rust-node-v1')?.kernelFingerprint)
      .toBe('sha256:fde93f46f61330609eaab5c7470be0bb24a2ff14a64d0b83788c5c0051d82af6')
    expect(archivedGeometryManifest('own-rust-node-v2')?.qualification.status)
      .toBe('baseline-pending')
    expect(archivedGeometryManifest('missing-v1')).toBeNull()
    expect(immutableEngineManifestToWire('mesh', 'own-rust-node-v1')).toMatchObject({
      engine_class: 'mesh',
      manifest_digest: GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'].manifestDigest,
    })
    expect(immutableEngineManifestToWire('brep', 'own-rust-node-v1')).toBeNull()
    expect(immutableEngineManifestToWire('mesh', 'brep-contract-v1')).toBeNull()
  })
})
