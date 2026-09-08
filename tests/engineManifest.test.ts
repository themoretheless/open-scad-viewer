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
      (GEOMETRY_MANIFEST_ARCHIVE['manifold-node-v1'].capabilities as string[]).push('tampered')
    }).toThrow()
  })

  it('keeps v1 immutable and binds the current Manifold evidence to repository artifacts', async () => {
    const historical = GEOMETRY_MANIFEST_ARCHIVE['manifold-node-v1']
    const manifest = GEOMETRY_MANIFEST_ARCHIVE[CURRENT_GEOMETRY_MANIFEST_VERSIONS.manifold]
    const notices = await readFile(new URL('../THIRD_PARTY_NOTICES.md', import.meta.url))
    const lockfile = await readFile(new URL('../package-lock.json', import.meta.url))
    const parsedLockfile = JSON.parse(lockfile.toString('utf8')) as {
      packages: Record<string, { version?: string }>
    }

    expect(historical).toMatchObject({
      capabilityManifestVersion: 'manifold-node-v1',
      manifestDigest: 'ae4ee188f2cf4898699318745e9eda48f96672e49b4b6d857f371ce7ed90013c',
      dependency: {
        sbomSha256: 'e4a858f5dff28f544db1a126657ca096cf49ec855cfdbf071f053268b313786f',
        lockfileSha256: 'b4fc02ba7ec6cb446763577536cdef9f830797852886f15a9af174b21edc9f2f',
      },
    })
    expect(manifest.capabilityManifestVersion).toBe('manifold-node-v3')
    expect(GEOMETRY_MANIFEST_ARCHIVE['manifold-node-v2'].manifestDigest).toBe('54cf792011b36741c5ea930af0e3b303e0a1b6f3d0707dca4ae8c099256486fe')
    expect(sha256(notices)).toBe(manifest.dependency.sbomSha256)
    expect(sha256(lockfile)).toBe(manifest.dependency.lockfileSha256)
    expect(parsedLockfile.packages['node_modules/manifold-3d']?.version)
      .toBe(manifest.dependency.version)
  })

  it('resolves only archived version and engine-class pairs', () => {
    expect(archivedGeometryManifest('manifold-node-v1')?.engineClass).toBe('manifold')
    expect(archivedGeometryManifest('manifold-node-v2')?.engineClass).toBe('manifold')
    expect(archivedGeometryManifest('missing-v1')).toBeNull()
    expect(immutableEngineManifestToWire('manifold', 'manifold-node-v1')).toMatchObject({
      engine_class: 'manifold',
      manifest_digest: GEOMETRY_MANIFEST_ARCHIVE['manifold-node-v1'].manifestDigest,
    })
    expect(immutableEngineManifestToWire('manifold', 'manifold-node-v2')).toMatchObject({
      engine_class: 'manifold',
      manifest_digest: GEOMETRY_MANIFEST_ARCHIVE['manifold-node-v2'].manifestDigest,
    })
    expect(immutableEngineManifestToWire('brep', 'manifold-node-v1')).toBeNull()
  })
})
