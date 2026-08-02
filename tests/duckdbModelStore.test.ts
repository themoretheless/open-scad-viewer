import { createHash } from 'node:crypto'
import { chmod, link, mkdtemp, rm, stat, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { DuckDBInstance, type DuckDBConnection } from '@duckdb/node-api'
import { afterEach, describe, expect, it } from 'vitest'
import {
  DuckDbModelStore,
  MAX_STORED_ARTIFACT_COUNT,
  MAX_STORED_BUILD_COUNT,
  MAX_STORED_REVISIONS_PER_MODEL,
} from '../src/mcp/duckdbModelStore'
import {
  ModelRevisionConflictError,
  type RecordBuildInput,
} from '../src/mcp/modelStore'
import {
  GEOMETRY_MANIFEST_ARCHIVE,
  LEGACY_MANIFOLD_EXECUTION,
  type GeometryExecutionDescriptor,
} from '../src/core/geometryExecution'

const stores: DuckDbModelStore[] = []
const temporaryDirectories: string[] = []

function digest(data: Uint8Array): string {
  return createHash('sha256').update(data).digest('hex')
}

function sourceDigest(source: string): string {
  return createHash('sha256').update(source).digest('hex')
}

async function memoryStore() {
  const store = await DuckDbModelStore.open(':memory:')
  stores.push(store)
  return store
}

async function temporaryDatabasePath(fileName = 'catalog.duckdb') {
  const directory = await mkdtemp(join(tmpdir(), 'open-scad-viewer-duckdb-'))
  temporaryDirectories.push(directory)
  return join(directory, fileName)
}

async function withRawDatabase(
  path: string,
  operation: (connection: DuckDBConnection) => Promise<void>,
) {
  const instance = await DuckDBInstance.create(path)
  const connection = await instance.connect()
  try {
    await operation(connection)
  } finally {
    try {
      connection.closeSync()
    } finally {
      instance.closeSync()
    }
  }
}

function successfulBuildInput(id: string): RecordBuildInput {
  const source = 'cube(1);'
  return {
    id,
    source,
    sourceSha256: sourceDigest(source),
    quality: 'full',
    durationMs: 12.5,
    warnings: [],
    status: 'succeeded',
    execution: runtimeExecution(),
    metrics: {
      meshCount: 1,
      triangleCount: 12,
      volume: 8,
      surfaceArea: 24,
      reduced: false,
    },
  }
}

function runtimeExecution(
  overrides: Partial<GeometryExecutionDescriptor> = {},
): GeometryExecutionDescriptor {
  return {
    ...LEGACY_MANIFOLD_EXECUTION,
    evidence: 'runtime',
    effectiveLimits: { sourceCharacters: 250_000, triangles: 750_000 },
    ...overrides,
  }
}

function plannedBrepExecution(
  overrides: Partial<GeometryExecutionDescriptor> = {},
): GeometryExecutionDescriptor {
  return {
    languageContract: 'openscad-viewer/brep-1',
    requiredCapabilities: ['geometry.brep', 'nurbs.curves'],
    engineClass: 'brep',
    engineKey: 'rust-brep-reserved-v1',
    kernelFingerprint: 'not-deployed',
    semanticProgramVersion: 'semantic-program-contract-v1',
    capabilityManifestVersion: 'brep-contract-v1',
    manifestDigest: GEOMETRY_MANIFEST_ARCHIVE['brep-contract-v1'].manifestDigest,
    purpose: 'analysis',
    quality: 'full',
    representation: 'brep',
    evidence: 'planned',
    effectiveLimits: { sourceCharacters: 250_000 },
    automaticFallback: false,
    ...overrides,
  }
}

afterEach(async () => {
  await Promise.all(stores.splice(0).map(store => store.close()))
  await Promise.all(temporaryDirectories.splice(0).map(path => rm(path, { recursive: true, force: true })))
})

describe('DuckDbModelStore', () => {
  it('stores models with source-sensitive revisions and optimistic concurrency', async () => {
    const store = await memoryStore()
    const created = await store.saveModel({
      id: 'gear',
      name: 'Шестерня',
      source: 'teeth = 12; cube(teeth);',
    })
    expect(created).toMatchObject({ id: 'gear', name: 'Шестерня', revision: 0 })

    const renamed = await store.saveModel({
      id: 'gear',
      name: 'Gear',
      source: created.source,
      expectedRevision: 0,
    })
    expect(renamed.revision).toBe(0)

    const edited = await store.saveModel({
      id: 'gear',
      name: 'Gear',
      source: 'teeth = 16; cube(teeth);',
      expectedRevision: 0,
    })
    expect(edited.revision).toBe(1)
    await expect(store.saveModel({
      id: 'gear',
      name: 'Stale edit',
      source: 'cube(2);',
      expectedRevision: 0,
    })).rejects.toBeInstanceOf(ModelRevisionConflictError)
    await expect(store.getModel('gear')).resolves.toMatchObject({
      name: 'Gear',
      source: 'teeth = 16; cube(teeth);',
      revision: 1,
    })
  })

  it('preserves a concurrent name-only update during a source-only CAS save', async () => {
    const store = await memoryStore()
    await store.saveModel({ id: 'rename-race', name: 'Original', source: 'cube(1);' })

    const staleRead = await store.getModel('rename-race')
    await store.saveModel({
      id: 'rename-race',
      name: 'Renamed concurrently',
      source: 'cube(1);',
      expectedRevision: 0,
    })
    const saved = await store.saveModelSource({
      id: 'rename-race',
      source: staleRead!.source.replace('1', '2'),
      expectedRevision: staleRead!.revision,
    })

    expect(saved).toMatchObject({
      name: 'Renamed concurrently',
      source: 'cube(2);',
      revision: 1,
    })
    await expect(store.getModelRevision('rename-race', 1)).resolves.toMatchObject({
      name: 'Renamed concurrently',
      source: 'cube(2);',
    })
  })

  it('retrieves immutable source revisions and lists them newest first', async () => {
    const store = await memoryStore()
    const sources = ['cube(1);', 'cube(2);', 'sphere(3);']

    await store.saveModel({ id: 'revisioned', name: 'Revisioned', source: sources[0] })
    await store.saveModel({
      id: 'revisioned',
      name: 'Revisioned',
      source: sources[1],
      expectedRevision: 0,
    })
    await store.saveModel({
      id: 'revisioned',
      name: 'Revisioned',
      source: sources[2],
      expectedRevision: 1,
    })

    await expect(store.getModelRevision('revisioned', 0)).resolves.toMatchObject({
      id: 'revisioned',
      revision: 0,
      source: sources[0],
    })
    await expect(store.getModelRevision('revisioned', 1)).resolves.toMatchObject({
      id: 'revisioned',
      revision: 1,
      source: sources[1],
    })
    await expect(store.getModelRevision('revisioned', 2)).resolves.toMatchObject({
      id: 'revisioned',
      revision: 2,
      source: sources[2],
    })
    await expect(store.getModelRevision('revisioned', 3)).resolves.toBeNull()

    expect((await store.listModelRevisions('revisioned')).map(revision => ({
      revision: revision.revision,
      sourceLength: revision.sourceLength,
    }))).toEqual([
      { revision: 2, sourceLength: sources[2].length },
      { revision: 1, sourceLength: sources[1].length },
      { revision: 0, sourceLength: sources[0].length },
    ])
  })

  it('caps tiny alternating source revisions independently of source bytes', async () => {
    const store = await memoryStore()
    await store.saveModel({ id: 'bounded-revisions', name: 'Bounded', source: '' })
    for (let revision = 1; revision < MAX_STORED_REVISIONS_PER_MODEL; revision++) {
      await store.saveModel({
        id: 'bounded-revisions',
        name: 'Bounded',
        source: revision % 2 === 0 ? '' : ' ',
        expectedRevision: revision - 1,
      })
    }

    await expect(store.saveModel({
      id: 'bounded-revisions',
      name: 'Bounded',
      source: MAX_STORED_REVISIONS_PER_MODEL % 2 === 0 ? '' : ' ',
      expectedRevision: MAX_STORED_REVISIONS_PER_MODEL - 1,
    })).rejects.toThrow(/limited to 256 source revisions/)
    expect(await store.listModelRevisions('bounded-revisions', 500))
      .toHaveLength(MAX_STORED_REVISIONS_PER_MODEL)
  }, 30_000)

  it('uses bound parameters for values that look like SQL', async () => {
    const store = await memoryStore()
    const payload = `cube(1);'); DROP TABLE mcp_models; --`
    await store.saveModel({ id: 'injection', name: `x'); DROP TABLE mcp_models; --`, source: payload })

    expect(await store.getModel('injection')).toMatchObject({ source: payload })
    expect(await store.listModels()).toHaveLength(1)
  })

  it('records builds and round-trips bounded binary artifacts', async () => {
    const store = await memoryStore()
    const model = await store.saveModel({ id: 'part', name: 'Part', source: 'cube(2);' })
    const build = await store.recordBuild({
      id: 'build-1',
      modelId: model.id,
      modelRevision: model.revision,
      source: model.source,
      sourceSha256: sourceDigest(model.source),
      quality: 'full',
      durationMs: 12.5,
      warnings: ['test warning'],
      status: 'succeeded',
      metrics: {
        meshCount: 1,
        triangleCount: 12,
        volume: 8,
        surfaceArea: 24,
        reduced: false,
      },
      execution: runtimeExecution({ purpose: 'export' }),
    })
    expect(build).toMatchObject({
      id: 'build-1',
      modelId: 'part',
      status: 'succeeded',
      execution: {
        engineClass: 'manifold',
        purpose: 'export',
        evidence: 'runtime',
        automaticFallback: false,
        effectiveLimits: { sourceCharacters: 250_000, triangles: 750_000 },
      },
    })

    const bytes = new Uint8Array([0, 1, 2, 127, 255])
    const artifact = await store.storeArtifact({
      id: 'artifact-1',
      buildId: build.id,
      modelId: model.id,
      format: 'stl',
      fileName: 'part.stl',
      mimeType: 'model/stl',
      sha256: digest(bytes),
      data: bytes,
    })
    expect(artifact).toMatchObject({ id: 'artifact-1', byteLength: bytes.byteLength })
    expect((await store.getArtifact(artifact.id))?.data).toEqual(bytes)
    expect(await store.listBuilds({ modelId: model.id })).toEqual([build])
    expect(await store.getCatalogStats()).toMatchObject({
      modelCount: 1,
      revisionCount: 1,
      buildCount: 1,
      artifactCount: 1,
      storedSourceBytes: Buffer.byteLength(model.source),
      storedArtifactBytes: bytes.byteLength,
      buildsByStatus: { succeeded: 1, failed: 0, cancelled: 0 },
      limits: {
        models: 500,
        revisionsPerModel: MAX_STORED_REVISIONS_PER_MODEL,
        builds: MAX_STORED_BUILD_COUNT,
        artifacts: MAX_STORED_ARTIFACT_COUNT,
      },
    })
  })

  it('persists planned B-rep provenance for an unavailable-engine failure', async () => {
    const store = await memoryStore()
    const source = [
      '// @language openscad-viewer/brep-1',
      '// @requires geometry.brep nurbs.curves',
      'cube(1);',
    ].join('\n')
    const model = await store.saveModel({ id: 'brep-part', name: 'B-rep part', source })
    const execution = plannedBrepExecution()
    const build = await store.recordBuild({
      id: 'brep-unavailable',
      modelId: model.id,
      modelRevision: model.revision,
      source,
      sourceSha256: sourceDigest(source),
      quality: 'full',
      durationMs: 0,
      warnings: [],
      status: 'failed',
      error: {
        contractVersion: 1,
        name: 'EngineUnavailableError',
        message: 'Rust B-rep engine is not deployed',
        code: 'engine_unavailable',
        retryable: false,
        details: {
          language_contract: 'openscad-viewer/brep-1',
          engine_class: 'brep',
          engine_key: 'rust-brep-reserved-v1',
          availability_cause: 'not-deployed',
          automatic_fallback: false,
        },
      },
      execution,
    })

    expect(build).toMatchObject({
      status: 'failed',
      error: {
        code: 'engine_unavailable',
        retryable: false,
        details: { availability_cause: 'not-deployed', automatic_fallback: false },
      },
      execution: {
        languageContract: 'openscad-viewer/brep-1',
        engineClass: 'brep',
        evidence: 'planned',
        automaticFallback: false,
      },
    })
    await expect(store.getBuild(build.id)).resolves.toEqual(build)

    expect(() => store.recordBuild({
      id: 'brep-contradictory-diagnostic',
      modelId: model.id,
      modelRevision: model.revision,
      source,
      sourceSha256: sourceDigest(source),
      quality: 'full',
      durationMs: 0,
      warnings: [],
      status: 'failed',
      error: {
        contractVersion: 1,
        name: 'EngineUnavailableError',
        message: 'contradictory route',
        code: 'engine_unavailable',
        retryable: false,
        details: {
          language_contract: 'legacy/current',
          engine_class: 'manifold',
          engine_key: 'manifold-wasm-v1',
          availability_cause: 'not-deployed',
          automatic_fallback: false,
        },
      },
      execution,
    })).toThrow(/route details contradict execution provenance/)
  })

  it('requires versioned diagnostics from new writes and enforces the frozen retry policy', async () => {
    const store = await memoryStore()
    const base = {
      ...successfulBuildInput('diagnostic-v1'),
      status: 'failed' as const,
      metrics: undefined,
      error: {
        contractVersion: 1 as const,
        name: 'InternalError',
        message: 'bounded',
        code: 'internal_error' as const,
        retryable: true,
      },
    }
    await expect(store.recordBuild(base)).resolves.toMatchObject({
      error: { contractVersion: 1, code: 'internal_error', retryable: true },
    })
    expect(() => store.recordBuild({
      ...base,
      id: 'diagnostic-missing-version',
      error: { name: 'Error', message: 'legacy-shaped', code: 'internal_error', retryable: true },
    } as unknown as Parameters<typeof store.recordBuild>[0])).toThrow(/contract version 1/)
    expect(() => store.recordBuild({
      ...base,
      id: 'diagnostic-unknown-code',
      error: { ...base.error, code: 'invented_code' },
    } as unknown as Parameters<typeof store.recordBuild>[0])).toThrow(/frozen public taxonomy/)
    expect(() => store.recordBuild({
      ...base,
      id: 'diagnostic-wrong-retry',
      error: { ...base.error, retryable: false },
    })).toThrow(/frozen retry policy/)
    expect(() => store.recordBuild({
      ...base,
      id: 'diagnostic-extra-field',
      error: { ...base.error, extra: 'x'.repeat(200_000) },
    } as unknown as Parameters<typeof store.recordBuild>[0])).toThrow(/unknown field extra/)

    const mutable = {
      ...base,
      id: 'diagnostic-snapshot-before-await',
      error: { ...base.error, message: 'original snapshot' },
    }
    const pending = store.recordBuild(mutable)
    mutable.error.message = 'mutated after recordBuild returned'
    ;(mutable.error as typeof mutable.error & { extra?: string }).extra = 'late bypass'
    await expect(pending).resolves.toMatchObject({
      error: { message: 'original snapshot' },
    })
  })

  it('surfaces bounded pre-contract diagnostics as legacy-unattested and rejects unknown versions', async () => {
    const path = await temporaryDatabasePath()
    const original = await DuckDbModelStore.open(path)
    const input = {
      ...successfulBuildInput('legacy-diagnostic'),
      status: 'failed' as const,
      metrics: undefined,
      error: {
        contractVersion: 1 as const,
        name: 'InternalError',
        message: 'current',
        code: 'internal_error' as const,
        retryable: true,
      },
    }
    await original.recordBuild(input)
    await original.close()
    await withRawDatabase(path, async connection => {
      await connection.run('UPDATE mcp_builds SET error_json = $error WHERE id = $id', {
        id: input.id,
        error: JSON.stringify({ name: 'LegacyError', message: 'pre-contract' }),
      })
    })

    const legacy = await DuckDbModelStore.open(path)
    await expect(legacy.getBuild(input.id!)).resolves.toMatchObject({
      error: {
        contractVersion: 0,
        evidence: 'legacy-unattested',
        name: 'LegacyError',
        message: 'pre-contract',
      },
    })
    await legacy.close()

    await withRawDatabase(path, async connection => {
      await connection.run('UPDATE mcp_builds SET error_json = $error WHERE id = $id', {
        id: input.id,
        error: JSON.stringify({
          contractVersion: 2,
          name: 'FutureError',
          message: 'unknown',
          code: 'internal_error',
          retryable: true,
        }),
      })
    })
    const unknown = await DuckDbModelStore.open(path)
    stores.push(unknown)
    await expect(unknown.getBuild(input.id!)).rejects.toThrow(/unknown contract version/)
  })

  it('rejects stored V1 diagnostics whose route or top-level shape contradicts provenance', async () => {
    const path = await temporaryDatabasePath()
    const source = '// @language openscad-viewer/brep-1\ncube(1);'
    const execution = plannedBrepExecution({ requiredCapabilities: [] })
    const original = await DuckDbModelStore.open(path)
    const build = await original.recordBuild({
      id: 'tampered-diagnostic-route',
      source,
      sourceSha256: sourceDigest(source),
      quality: 'full',
      durationMs: 0,
      warnings: [],
      status: 'failed',
      error: {
        contractVersion: 1,
        name: 'GeometryEngineUnavailableError',
        message: 'not deployed',
        code: 'engine_unavailable',
        retryable: false,
        details: {
          language_contract: 'openscad-viewer/brep-1',
          engine_class: 'brep',
          engine_key: 'rust-brep-reserved-v1',
          availability_cause: 'not-deployed',
          automatic_fallback: false,
        },
      },
      execution,
    })
    await original.close()

    const contradictory = {
      ...build.error,
      details: {
        language_contract: 'legacy/current',
        engine_class: 'manifold',
        engine_key: 'manifold-wasm-v1',
        availability_cause: 'not-deployed',
        automatic_fallback: false,
      },
    }
    await withRawDatabase(path, async connection => {
      await connection.run('UPDATE mcp_builds SET error_json = $error WHERE id = $id', {
        id: build.id,
        error: JSON.stringify(contradictory),
      })
    })
    const wrongRoute = await DuckDbModelStore.open(path)
    await expect(wrongRoute.getBuild(build.id)).rejects.toThrow(/route details contradict execution provenance/)
    await wrongRoute.close()

    await withRawDatabase(path, async connection => {
      await connection.run('UPDATE mcp_builds SET error_json = $error WHERE id = $id', {
        id: build.id,
        error: JSON.stringify({ ...build.error, extra: 'x'.repeat(200_000) }),
      })
    })
    const extraField = await DuckDbModelStore.open(path)
    stores.push(extraField)
    await expect(extraField.getBuild(build.id)).rejects.toThrow(/unknown field extra/)
  })

  it('binds build provenance to the source route and immutable manifest', async () => {
    const store = await memoryStore()
    const source = '// @language openscad-viewer/brep-1\n// @requires geometry.brep\ncube(1);'
    const model = await store.saveModel({ id: 'routed-brep', name: 'Routed B-rep', source })

    expect(() => store.recordBuild({
      ...successfulBuildInput('wrong-source-route'),
      modelId: model.id,
      modelRevision: model.revision,
      source,
      sourceSha256: sourceDigest(source),
    })).toThrow(/match the exact source routing header/)

    expect(() => store.recordBuild({
      ...successfulBuildInput('wrong-inline-source-route'),
      source,
      sourceSha256: sourceDigest(source),
    })).toThrow(/match the exact source routing header/)

    expect(() => store.recordBuild({
      ...successfulBuildInput('unknown-manifest'),
      execution: runtimeExecution({ capabilityManifestVersion: 'manifold-node-v999' }),
    })).toThrow(/unknown capability manifest/)
    expect(() => store.recordBuild({
      ...successfulBuildInput('altered-engine-identity'),
      execution: runtimeExecution({ engineKey: 'manifold-wasm-v2' }),
    })).toThrow(/does not match its immutable capability manifest/)
    expect(() => store.recordBuild({
      ...successfulBuildInput('altered-manifest-digest'),
      execution: runtimeExecution({ manifestDigest: '0'.repeat(64) }),
    })).toThrow(/does not match its immutable capability manifest/)
    expect(() => store.recordBuild({
      ...successfulBuildInput('altered-manifest-limits'),
      execution: runtimeExecution({
        effectiveLimits: { sourceCharacters: 249_999, triangles: 750_000 },
      }),
    })).toThrow(/limits are not attested/)
    expect(() => store.recordBuild({
      ...successfulBuildInput('unqualified-runtime-capability'),
      execution: runtimeExecution({ requiredCapabilities: ['nurbs.curves'] }),
    })).toThrow(/unqualified capability/)

    expect(await store.listBuilds()).toEqual([])
  })

  it('rejects artifact metadata that does not match its format or bytes', async () => {
    const store = await memoryStore()
    const build = await store.recordBuild(successfulBuildInput('integrity-build'))
    const data = new Uint8Array([1, 2, 3])

    expect(() => store.storeArtifact({
      buildId: build.id, format: 'stl', fileName: 'bad.stl', mimeType: 'model/obj',
      sha256: digest(data), data,
    })).toThrow(/mimeType/)
    expect(() => store.storeArtifact({
      buildId: build.id, format: 'stl', fileName: 'bad.stl', mimeType: 'model/stl',
      sha256: '0'.repeat(64), data,
    })).toThrow(/match artifact data/)
    expect(await store.listArtifacts()).toEqual([])
  })

  it('rejects corrupted artifact rows on read instead of serving them over MCP', async () => {
    const path = await temporaryDatabasePath()
    const original = await DuckDbModelStore.open(path)
    const build = await original.recordBuild(successfulBuildInput('corrupt-build'))
    const data = new Uint8Array([4, 5, 6])
    const artifact = await original.storeArtifact({
      id: 'corrupt-artifact', buildId: build.id, format: 'stl', fileName: 'part.stl',
      mimeType: 'model/stl', sha256: digest(data), data,
    })
    await original.close()
    await withRawDatabase(path, async connection => {
      await connection.run('UPDATE mcp_artifacts SET byte_length = byte_length + 1 WHERE id = $id', { id: artifact.id })
    })

    const reopened = await DuckDbModelStore.open(path)
    stores.push(reopened)
    await expect(reopened.getArtifact(artifact.id)).rejects.toThrow(/invalid length/)
  })

  it('rejects corrupted execution provenance on read instead of serving it over MCP', async () => {
    const path = await temporaryDatabasePath()
    const original = await DuckDbModelStore.open(path)
    const build = await original.recordBuild(successfulBuildInput('corrupt-execution-build'))
    await original.close()
    await withRawDatabase(path, async connection => {
      await connection.run(`
        UPDATE mcp_builds SET execution_json = $execution_json WHERE id = $id
      `, {
        id: build.id,
        execution_json: JSON.stringify({
          ...runtimeExecution(),
          automaticFallback: true,
        }),
      })
    })

    const reopened = await DuckDbModelStore.open(path)
    stores.push(reopened)
    await expect(reopened.getBuild(build.id)).rejects.toThrow(/disable automatic fallback/)
  })

  it('re-attests referenced source digests and routes on every build read', async () => {
    const path = await temporaryDatabasePath()
    const source = '// @requires geometry.mesh\ncube(1);'
    const original = await DuckDbModelStore.open(path)
    const model = await original.saveModel({ id: 'read-attestation', name: 'Read attestation', source })
    const build = await original.recordBuild({
      ...successfulBuildInput('read-attested-build'),
      modelId: model.id,
      modelRevision: model.revision,
      source,
      sourceSha256: sourceDigest(source),
      execution: runtimeExecution({ requiredCapabilities: ['geometry.mesh'] }),
    })
    await original.close()

    await withRawDatabase(path, async connection => {
      await connection.run('UPDATE mcp_builds SET source_sha256 = $digest WHERE id = $id', {
        id: build.id,
        digest: 'b'.repeat(64),
      })
    })
    const badDigest = await DuckDbModelStore.open(path)
    await expect(badDigest.getBuild(build.id)).rejects.toThrow(/digest.*immutable model revision/)
    await badDigest.close()

    await withRawDatabase(path, async connection => {
      await connection.run(`
        UPDATE mcp_builds SET source_sha256 = $digest, execution_json = $execution WHERE id = $id
      `, {
        id: build.id,
        digest: sourceDigest(source),
        execution: JSON.stringify(runtimeExecution()),
      })
    })
    const badRoute = await DuckDbModelStore.open(path)
    stores.push(badRoute)
    await expect(badRoute.getBuild(build.id)).rejects.toThrow(/does not match its immutable source route/)
  })

  it('stores and re-attests an immutable source snapshot for inline build history', async () => {
    const path = await temporaryDatabasePath()
    const source = '// @requires geometry.mesh\ncube(1);'
    const original = await DuckDbModelStore.open(path)
    const build = await original.recordBuild({
      ...successfulBuildInput('inline-read-attestation'),
      source,
      sourceSha256: sourceDigest(source),
      execution: runtimeExecution({ requiredCapabilities: ['geometry.mesh'] }),
    })
    await original.close()

    await withRawDatabase(path, async connection => {
      const rows = await (await connection.run(`
        SELECT source_snapshot, source_attestation FROM mcp_builds WHERE id = $id
      `, { id: build.id })).getRowObjectsJS()
      expect(rows[0]?.source_snapshot).toBe(source)
      expect(rows[0]?.source_attestation).toBe('inline-snapshot')
      await expect(connection.run(`
        UPDATE mcp_builds SET source_snapshot = NULL WHERE id = $id
      `, { id: build.id })).rejects.toThrow()
    })

    await withRawDatabase(path, async connection => {
      await connection.run(`
        UPDATE mcp_builds SET source_snapshot = $source WHERE id = $id
      `, { id: build.id, source: 'sphere(1);' })
    })
    const badDigest = await DuckDbModelStore.open(path)
    await expect(badDigest.getBuild(build.id)).rejects.toThrow(/immutable source snapshot/)
    await badDigest.close()

    await withRawDatabase(path, async connection => {
      await connection.run(`
        UPDATE mcp_builds
        SET source_snapshot = $source, execution_json = $execution
        WHERE id = $id
      `, {
        id: build.id,
        source,
        execution: JSON.stringify(runtimeExecution()),
      })
    })
    const badRoute = await DuckDbModelStore.open(path)
    stores.push(badRoute)
    await expect(badRoute.getBuild(build.id)).rejects.toThrow(/immutable source route/)
  })

  it('marks schema-v4 inline history as unattested and rejects corrupted migrated markers', async () => {
    const path = await temporaryDatabasePath()
    const original = await DuckDbModelStore.open(path)
    const first = await original.recordBuild(successfulBuildInput('v4-historical-inline'))
    const second = await original.recordBuild(successfulBuildInput('v4-marker-corruption'))
    await original.close()

    await withRawDatabase(path, async connection => {
      await connection.run('DELETE FROM mcp_schema_migrations WHERE version = 5')
      await connection.run(`
        CREATE TABLE mcp_builds_v4 AS
        SELECT id, model_id, model_revision, source_sha256,
               CAST(NULL AS VARCHAR) AS source_snapshot,
               quality, status, duration_ms, warnings_json, mesh_count,
               triangle_count, volume, surface_area, reduced, error_json,
               execution_json, created_at
        FROM mcp_builds
      `)
      await connection.run('CREATE TABLE mcp_artifacts_v4 AS SELECT * FROM mcp_artifacts')
      await connection.run('DROP TABLE mcp_artifacts')
      await connection.run('DROP TABLE mcp_builds')
      await connection.run('ALTER TABLE mcp_builds_v4 RENAME TO mcp_builds')
      await connection.run('ALTER TABLE mcp_artifacts_v4 RENAME TO mcp_artifacts')
    })

    const migrated = await DuckDbModelStore.open(path)
    await expect(migrated.getBuild(first.id)).resolves.toMatchObject({
      sourceAttestation: 'historical-unattested',
    })
    await migrated.close()

    await withRawDatabase(path, async connection => {
      await connection.run(`
        UPDATE mcp_builds
        SET source_attestation = 'inline-snapshot', source_snapshot = NULL
        WHERE id = $id
      `, { id: second.id })
    })
    const missingSnapshot = await DuckDbModelStore.open(path)
    await expect(missingSnapshot.getBuild(second.id)).rejects.toThrow(/contradicts its attestation class/)
    await missingSnapshot.close()

    await withRawDatabase(path, async connection => {
      await connection.run(`
        UPDATE mcp_builds SET source_attestation = 'unknown-evidence' WHERE id = $id
      `, { id: second.id })
    })
    const unknownMarker = await DuckDbModelStore.open(path)
    stores.push(unknownMarker)
    await expect(unknownMarker.getBuild(second.id)).rejects.toThrow(/invalid build source attestation class/)
  })

  it('never accepts B-rep history disguised as legacy backfill', async () => {
    const path = await temporaryDatabasePath()
    const original = await DuckDbModelStore.open(path)
    const build = await original.recordBuild(successfulBuildInput('forged-legacy-brep'))
    await original.close()
    await withRawDatabase(path, async connection => {
      await connection.run('UPDATE mcp_builds SET execution_json = $execution WHERE id = $id', {
        id: build.id,
        execution: JSON.stringify(plannedBrepExecution({
          evidence: 'legacy-backfill',
          requiredCapabilities: [],
          effectiveLimits: {},
          purpose: 'full',
        })),
      })
    })

    const reopened = await DuckDbModelStore.open(path)
    stores.push(reopened)
    await expect(reopened.getBuild(build.id)).rejects.toThrow(/exact historical Manifold descriptor/)
  })

  it('rejects non-finite build values without adding partial history', async () => {
    const store = await memoryStore()

    expect(() => store.recordBuild({
      ...successfulBuildInput('nan-duration'),
      durationMs: Number.NaN,
    })).toThrow(/durationMs/)
    expect(() => store.recordBuild({
      ...successfulBuildInput('infinite-volume'),
      metrics: {
        ...successfulBuildInput('unused').metrics!,
        volume: Number.POSITIVE_INFINITY,
      },
    })).toThrow(/finite and non-negative/)
    expect(() => store.recordBuild({
      ...successfulBuildInput('fallback-enabled'),
      execution: {
        ...runtimeExecution(),
        automaticFallback: true,
      } as unknown as GeometryExecutionDescriptor,
    })).toThrow(/disable automatic fallback/)
    expect(() => store.recordBuild({
      ...successfulBuildInput('quality-mismatch'),
      execution: runtimeExecution({ quality: 'preview' }),
    })).toThrow(/quality must match/)
    expect(() => store.recordBuild({
      ...successfulBuildInput('route-mismatch'),
      execution: runtimeExecution({
        languageContract: 'openscad-viewer/brep-1',
        engineClass: 'manifold',
      }),
    })).toThrow(/contradicts the language-contract engine route/)
    expect(() => store.recordBuild({
      ...successfulBuildInput('planned-success'),
      execution: runtimeExecution({ evidence: 'planned' }),
    })).toThrow(/succeeded build cannot have planned-only/)
    expect(() => store.recordBuild({
      ...successfulBuildInput('missing-execution'),
      execution: undefined as unknown as GeometryExecutionDescriptor,
    })).toThrow(/execution provenance is required/)

    expect(await store.listBuilds()).toEqual([])
  })

  it('requires complete model revision pairs and derives artifact model identity from its build', async () => {
    const store = await memoryStore()
    const first = await store.saveModel({ id: 'first-model', name: 'First', source: 'cube(1);' })
    await store.saveModel({ id: 'second-model', name: 'Second', source: 'sphere(1);' })
    const second = await store.saveModel({
      id: 'second-model',
      name: 'Second',
      source: 'sphere(2);',
      expectedRevision: 0,
    })

    expect(() => store.recordBuild({
      ...successfulBuildInput('model-only'),
      modelId: first.id,
    })).toThrow(/provided together/)
    expect(() => store.recordBuild({
      ...successfulBuildInput('revision-only'),
      modelRevision: first.revision,
    })).toThrow(/provided together/)
    await expect(store.recordBuild({
      ...successfulBuildInput('cross-model-revision'),
      modelId: first.id,
      modelRevision: second.revision,
    })).rejects.toThrow()
    expect(await store.listBuilds()).toEqual([])

    const unrelatedSource = 'cube(9);'
    await expect(store.recordBuild({
      ...successfulBuildInput('wrong-snapshot-hash'),
      modelId: first.id,
      modelRevision: first.revision,
      source: unrelatedSource,
      sourceSha256: sourceDigest(unrelatedSource),
    })).rejects.toThrow(/must match the referenced immutable model revision/)

    const build = await store.recordBuild({
      ...successfulBuildInput('paired-build'),
      modelId: first.id,
      modelRevision: first.revision,
      sourceSha256: sourceDigest(first.source),
    })
    await expect(store.storeArtifact({
      id: 'mismatched-artifact',
      buildId: build.id,
      modelId: second.id,
      format: 'stl',
      fileName: 'mismatched.stl',
      mimeType: 'model/stl',
      sha256: digest(new Uint8Array([1])),
      data: new Uint8Array([1]),
    })).rejects.toThrow(/modelId must match/)
    expect(await store.listArtifacts()).toEqual([])

    const artifact = await store.storeArtifact({
      id: 'derived-artifact',
      buildId: build.id,
      format: 'stl',
      fileName: 'derived.stl',
      mimeType: 'model/stl',
      sha256: digest(new Uint8Array([2])),
      data: new Uint8Array([2]),
    })
    expect(artifact.modelId).toBe(first.id)
  })

  it('reopens a persistent DuckDB file without reapplying destructive migrations', async () => {
    const path = await temporaryDatabasePath()
    const first = await DuckDbModelStore.open(path)
    await first.saveModel({ id: 'persistent', name: 'Persistent', source: 'sphere(3);' })
    await first.close()

    const second = await DuckDbModelStore.open(path)
    stores.push(second)
    await expect(second.getModel('persistent')).resolves.toMatchObject({
      name: 'Persistent',
      source: 'sphere(3);',
    })
  })

  it('rejects duplicate hardlink opens but releases the guard on close', async () => {
    if (process.platform === 'win32') return
    const path = await temporaryDatabasePath()
    const alias = join(path.slice(0, path.lastIndexOf('/')), 'catalog-hardlink.duckdb')
    const first = await DuckDbModelStore.open(path)
    try {
      await first.saveModel({ id: 'hardlinked', name: 'Hardlinked', source: 'cube(4);' })
      await link(path, alias)
      await expect(DuckDbModelStore.open(alias)).rejects.toThrow(/already open in this process/)
    } finally {
      await first.close()
    }

    const reopened = await DuckDbModelStore.open(alias)
    stores.push(reopened)
    await expect(reopened.getModel('hardlinked')).resolves.toMatchObject({ source: 'cube(4);' })
  })

  it('rejects catalogs created by a newer schema version', async () => {
    const path = await temporaryDatabasePath()
    const current = await DuckDbModelStore.open(path)
    await current.close()
    await withRawDatabase(path, async connection => {
      await connection.run('INSERT INTO mcp_schema_migrations (version) VALUES (6)')
    })

    await expect(DuckDbModelStore.open(path)).rejects.toThrow(/schema 6 is newer than supported schema 5/)
  })

  it('migrates schema v2 builds with explicit legacy Manifold execution provenance', async () => {
    const path = await temporaryDatabasePath()
    const original = await DuckDbModelStore.open(path)
    await original.recordBuild({
      ...successfulBuildInput('pre-provenance-build'),
      quality: 'preview',
      execution: runtimeExecution({ quality: 'preview', purpose: 'preview' }),
    })
    await original.close()

    await withRawDatabase(path, async connection => {
      await connection.run('DELETE FROM mcp_schema_migrations WHERE version >= 3')
      await connection.run(`
        CREATE TABLE mcp_builds_v2_copy AS
        SELECT id, model_id, model_revision, source_sha256, quality, status,
               duration_ms, warnings_json, mesh_count, triangle_count, volume,
               surface_area, reduced, error_json, created_at
        FROM mcp_builds
      `)
      await connection.run('DROP TABLE mcp_artifacts')
      await connection.run('DROP TABLE mcp_builds')
      await connection.run(`
        CREATE TABLE mcp_builds (
          id VARCHAR PRIMARY KEY,
          model_id VARCHAR,
          model_revision BIGINT,
          source_sha256 VARCHAR NOT NULL,
          quality VARCHAR NOT NULL CHECK (quality IN ('preview', 'full')),
          status VARCHAR NOT NULL CHECK (status IN ('succeeded', 'failed', 'cancelled')),
          duration_ms DOUBLE NOT NULL,
          warnings_json JSON NOT NULL,
          mesh_count BIGINT,
          triangle_count BIGINT,
          volume DOUBLE,
          surface_area DOUBLE,
          reduced BOOLEAN,
          error_json JSON,
          created_at TIMESTAMPTZ NOT NULL
        )
      `)
      await connection.run('INSERT INTO mcp_builds SELECT * FROM mcp_builds_v2_copy')
      await connection.run(`
        CREATE TABLE mcp_artifacts (
          id VARCHAR PRIMARY KEY,
          build_id VARCHAR NOT NULL,
          model_id VARCHAR,
          format VARCHAR NOT NULL,
          file_name VARCHAR NOT NULL,
          mime_type VARCHAR NOT NULL,
          sha256 VARCHAR NOT NULL,
          byte_length BIGINT NOT NULL,
          data BLOB NOT NULL,
          created_at TIMESTAMPTZ NOT NULL,
          FOREIGN KEY (build_id) REFERENCES mcp_builds(id),
          FOREIGN KEY (model_id) REFERENCES mcp_models(id)
        )
      `)
      await connection.run('DROP TABLE mcp_builds_v2_copy')
    })

    const migrated = await DuckDbModelStore.open(path)
    stores.push(migrated)
    await expect(migrated.getBuild('pre-provenance-build')).resolves.toMatchObject({
      quality: 'preview',
      sourceAttestation: 'historical-unattested',
      execution: {
        ...LEGACY_MANIFOLD_EXECUTION,
        purpose: 'preview',
        quality: 'preview',
      },
    })
  })

  it('migrates legacy TIMESTAMP catalogs to current UTC instants and backfills their revision', async () => {
    const path = await temporaryDatabasePath()
    const previousTimezone = process.env.TZ
    process.env.TZ = 'Asia/Tbilisi'
    const seededAt = Date.now()
    try {
      await withRawDatabase(path, async connection => {
        await connection.run(`
          CREATE TABLE mcp_schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TIMESTAMP NOT NULL DEFAULT current_timestamp
          )
        `)
        await connection.run('INSERT INTO mcp_schema_migrations (version) VALUES (1)')
        await connection.run(`
          CREATE TABLE mcp_models (
            id VARCHAR PRIMARY KEY,
            name VARCHAR NOT NULL,
            source VARCHAR NOT NULL,
            revision BIGINT NOT NULL CHECK (revision >= 0),
            created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
            updated_at TIMESTAMP NOT NULL DEFAULT current_timestamp
          )
        `)
        await connection.run(`
          CREATE TABLE mcp_builds (
            id VARCHAR PRIMARY KEY,
            model_id VARCHAR,
            model_revision BIGINT,
            source_sha256 VARCHAR NOT NULL,
            quality VARCHAR NOT NULL,
            status VARCHAR NOT NULL,
            duration_ms DOUBLE NOT NULL,
            warnings_json JSON NOT NULL,
            mesh_count BIGINT,
            triangle_count BIGINT,
            volume DOUBLE,
            surface_area DOUBLE,
            reduced BOOLEAN,
            error_json JSON,
            created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
          )
        `)
        await connection.run(`
          CREATE TABLE mcp_artifacts (
            id VARCHAR PRIMARY KEY,
            build_id VARCHAR NOT NULL,
            model_id VARCHAR,
            format VARCHAR NOT NULL,
            file_name VARCHAR NOT NULL,
            mime_type VARCHAR NOT NULL,
            sha256 VARCHAR NOT NULL,
            byte_length BIGINT NOT NULL,
            data BLOB NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
          )
        `)
        await connection.run(`
          INSERT INTO mcp_models (id, name, source, revision)
          VALUES ($id, $name, $source, $revision)
        `, {
          id: 'legacy-model',
          name: 'Legacy',
          source: 'cylinder(h=5, r=2);',
          revision: 7,
        })
        await connection.run(`
          INSERT INTO mcp_builds (
            id, model_id, model_revision, source_sha256, quality, status,
            duration_ms, warnings_json, mesh_count, triangle_count, volume,
            surface_area, reduced, error_json
          ) VALUES (
            $id, $model_id, $model_revision, $source_sha256, 'full', 'failed',
            0, '[]', NULL, NULL, NULL, NULL, NULL, $error_json
          )
        `, {
          id: 'legacy-mismatched-build',
          model_id: 'legacy-model',
          model_revision: 7,
          source_sha256: 'b'.repeat(64),
          error_json: JSON.stringify({ name: 'Error', message: 'legacy failure' }),
        })
      })

      const migrated = await DuckDbModelStore.open(path)
      stores.push(migrated)
      const model = await migrated.getModel('legacy-model')
      const revision = await migrated.getModelRevision('legacy-model', 7)
      expect(model).toMatchObject({
        id: 'legacy-model',
        revision: 7,
        source: 'cylinder(h=5, r=2);',
      })
      expect(revision).toMatchObject({
        id: 'legacy-model',
        revision: 7,
        source: 'cylinder(h=5, r=2);',
      })
      expect(await migrated.listModelRevisions('legacy-model')).toHaveLength(1)
      await expect(migrated.getBuild('legacy-mismatched-build')).resolves.toMatchObject({
        modelId: null,
        modelRevision: null,
        execution: { evidence: 'legacy-backfill', engineClass: 'manifold' },
      })
      for (const timestamp of [model!.createdAt, model!.updatedAt, revision!.createdAt]) {
        expect(Math.abs(Date.parse(timestamp) - seededAt)).toBeLessThan(60_000)
      }
    } finally {
      if (previousTimezone === undefined) delete process.env.TZ
      else process.env.TZ = previousTimezone
    }
  })

  it('hardens persistent catalog permissions to 0600 on POSIX', async () => {
    if (process.platform === 'win32') return
    const path = await temporaryDatabasePath()
    const first = await DuckDbModelStore.open(path)
    await first.close()
    await chmod(path, 0o666)

    const reopened = await DuckDbModelStore.open(path)
    stores.push(reopened)
    expect((await stat(path)).mode & 0o777).toBe(0o600)
  })

  it('rejects a directory database target without changing its permissions', async () => {
    const path = await temporaryDatabasePath()
    const directory = dirname(path)
    const before = (await stat(directory)).mode & 0o777

    await expect(DuckDbModelStore.open(directory)).rejects.toThrow(/regular file/)
    expect((await stat(directory)).mode & 0o777).toBe(before)
  })

  it('does not chmod an invalid regular-file database target', async () => {
    const path = await temporaryDatabasePath('not-a-database.duckdb')
    await writeFile(path, 'not a DuckDB catalog')
    if (process.platform !== 'win32') await chmod(path, 0o644)
    const before = (await stat(path)).mode & 0o777

    await expect(DuckDbModelStore.open(path)).rejects.toThrow()
    expect((await stat(path)).mode & 0o777).toBe(before)
  })

  it('retains only the newest bounded build history', async () => {
    const store = await memoryStore()
    for (let index = 0; index <= MAX_STORED_BUILD_COUNT; index++) {
      await store.recordBuild(successfulBuildInput(`retained-build-${String(index).padStart(4, '0')}`))
    }

    await expect(store.getBuild('retained-build-0000')).resolves.toBeNull()
    await expect(store.getBuild(`retained-build-${String(MAX_STORED_BUILD_COUNT).padStart(4, '0')}`))
      .resolves.toBeTruthy()
    expect(await store.listBuilds({ limit: MAX_STORED_BUILD_COUNT })).toHaveLength(MAX_STORED_BUILD_COUNT)
  }, 30_000)

  it('retains only the newest bounded artifact history', async () => {
    const store = await memoryStore()
    for (let index = 0; index <= MAX_STORED_ARTIFACT_COUNT; index++) {
      const suffix = String(index).padStart(4, '0')
      const build = await store.recordBuild(successfulBuildInput(`artifact-build-${suffix}`))
      await store.storeArtifact({
        id: `retained-artifact-${suffix}`,
        buildId: build.id,
        format: 'stl',
        fileName: `artifact-${suffix}.stl`,
        mimeType: 'model/stl',
        sha256: digest(new Uint8Array([index])),
        data: new Uint8Array([index]),
      })
    }

    await expect(store.getArtifact('retained-artifact-0000')).resolves.toBeNull()
    await expect(store.getArtifact(`retained-artifact-${String(MAX_STORED_ARTIFACT_COUNT).padStart(4, '0')}`))
      .resolves.toBeTruthy()
    expect(await store.listArtifacts(MAX_STORED_ARTIFACT_COUNT)).toHaveLength(MAX_STORED_ARTIFACT_COUNT)
  }, 30_000)
})
