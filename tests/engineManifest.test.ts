import { createHash } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import { describe, expect, it } from 'vitest'
import { GEOMETRY_MANIFEST_ARCHIVE } from '../src/core/geometryExecution'
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
      (GEOMETRY_MANIFEST_ARCHIVE['manifold-node-v1'].capabilities as string[]).push('tampered')
    }).toThrow()
  })

  it('binds the Manifold dependency evidence to repository artifacts', async () => {
    const manifest = GEOMETRY_MANIFEST_ARCHIVE['manifold-node-v1']
    const notices = await readFile(new URL('../THIRD_PARTY_NOTICES.md', import.meta.url))
    const lockfile = await readFile(new URL('../package-lock.json', import.meta.url))
    const parsedLockfile = JSON.parse(lockfile.toString('utf8')) as {
      packages: Record<string, { version?: string }>
    }

    expect(sha256(notices)).toBe(manifest.dependency.sbomSha256)
    expect(sha256(lockfile)).toBe(manifest.dependency.lockfileSha256)
    expect(parsedLockfile.packages['node_modules/manifold-3d']?.version)
      .toBe(manifest.dependency.version)
  })

  it('resolves only archived version and engine-class pairs', () => {
    expect(archivedGeometryManifest('manifold-node-v1')?.engineClass).toBe('manifold')
    expect(archivedGeometryManifest('missing-v1')).toBeNull()
    expect(immutableEngineManifestToWire('manifold', 'manifold-node-v1')).toMatchObject({
      engine_class: 'manifold',
      manifest_digest: GEOMETRY_MANIFEST_ARCHIVE['manifold-node-v1'].manifestDigest,
    })
    expect(immutableEngineManifestToWire('brep', 'manifold-node-v1')).toBeNull()
  })
})
