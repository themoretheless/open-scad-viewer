import { createHash, randomUUID } from 'node:crypto'
import { chmod, lstat, realpath, stat } from 'node:fs/promises'
import { resolve } from 'node:path'
import {
  DuckDBConnection,
  DuckDBInstance,
  blobValue,
  type JS,
} from '@duckdb/node-api'
import {
  CatalogQuotaError,
  ModelRevisionConflictError,
  MAX_STORED_ARTIFACT_BYTES,
  MAX_STORED_BUILD_WARNINGS,
  MAX_STORED_DIAGNOSTIC_MESSAGE_LENGTH,
  MAX_STORED_DIAGNOSTIC_NAME_LENGTH,
  MAX_STORED_DIAGNOSTIC_DETAIL_COUNT,
  MAX_STORED_DIAGNOSTIC_DETAIL_LENGTH,
  MAX_STORED_DIAGNOSTIC_JSON_BYTES,
  MAX_STORED_EXECUTION_JSON_BYTES,
  MAX_STORED_WARNING_LENGTH,
  isWellFormedUnicode,
  validateModelId,
  validateModelName,
  validateModelSource,
  type CatalogStats,
  type BuildDiagnostic,
  type BuildMetrics,
  type ModelStore,
  type RecordBuildInput,
  type SaveModelInput,
  type SaveModelSourceInput,
  type StoreArtifactInput,
  type StoredArtifact,
  type StoredArtifactSummary,
  type StoredBuild,
  type StoredBuildDiagnostic,
  type StoredModel,
  type StoredModelRevision,
  type StoredModelRevisionSummary,
  type StoredModelSummary,
} from './modelStore'
import {
  expectedPublicErrorRetryable,
  isPublicErrorCode,
} from './errorContract'
import {
  geometryEngineClassForLanguageContract,
  LEGACY_MANIFOLD_EXECUTION,
  parseGeometrySourceRoutingHeader,
  type GeometryExecutionDescriptor,
} from '../core/geometryExecution'
import { archivedGeometryManifest } from './engineManifest'
import { boundedLimit, isoTimestamp, requiredString, safeInteger, validateBuildDiagnostic, validateDiagnosticExecutionContext } from './duckdbModelStore/boundedLimit'
import { artifactSummaryFromRow, buildFromRow, expectedArtifactMimeType, modelFromRow, modelRevisionFromRow, serializeExecutionDescriptor, sha256 } from './duckdbModelStore/validateExecutionDescriptor'

export type Row = Record<string, JS>

export const MAX_LIST_LIMIT = 500
const LATEST_SCHEMA_VERSION = 6
export const MAX_STORED_ARTIFACT_COUNT = 100
export const MAX_STORED_ARTIFACT_TOTAL_BYTES = 64 * 1024 * 1024
export const MAX_STORED_BUILD_COUNT = 500
export const MAX_STORED_MODEL_COUNT = 500
export const MAX_STORED_MODEL_SOURCE_BYTES = 64 * 1024 * 1024
export const MAX_STORED_MODEL_REVISION_COUNT = 5_000
export const MAX_STORED_REVISIONS_PER_MODEL = 256

export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue }

/**
 * Node-only DuckDB repository used by the optional MCP process. The native
 * dependency never crosses into the Vite/browser graph.
 */
export class DuckDbModelStore implements ModelStore {
  private static readonly openDatabaseKeys = new Set<string>()
  private static openTail: Promise<void> = Promise.resolve()
  private tail: Promise<void> = Promise.resolve()
  private closed = false
  private closePromise: Promise<void> | null = null

  private constructor(
    private readonly instance: DuckDBInstance,
    private readonly connection: DuckDBConnection,
    private readonly registeredKeys: readonly string[],
    private readonly databasePath: string | null,
  ) {}

  static open(path = ':memory:'): Promise<DuckDbModelStore> {
    const result = this.openTail.then(() => this.openInternal(path), () => this.openInternal(path))
    this.openTail = result.then(() => undefined, () => undefined)
    return result
  }

  private static async openInternal(path: string): Promise<DuckDbModelStore> {
    const registeredPath = path === ':memory:' ? null : resolve(path)
    if (registeredPath) await this.validateDatabaseTarget(registeredPath)
    const initialKeys = registeredPath ? await this.databaseKeys(registeredPath) : []
    if (initialKeys.some(key => this.openDatabaseKeys.has(key))) {
      throw new Error(`DuckDB file is already open in this process: ${registeredPath}`)
    }
    initialKeys.forEach(key => this.openDatabaseKeys.add(key))
    let registeredKeys = initialKeys
    let instance: DuckDBInstance | null = null
    let connection: DuckDBConnection | null = null
    try {
      instance = await DuckDBInstance.create(registeredPath ?? path, {
        threads: '2',
        memory_limit: '512MB',
        max_temp_directory_size: '256MB',
        enable_external_access: 'false',
        autoload_known_extensions: 'false',
        autoinstall_known_extensions: 'false',
        allow_unsigned_extensions: 'false',
        allow_community_extensions: 'false',
        lock_configuration: 'true',
      })
      connection = await instance.connect()
      if (registeredPath) {
        const postOpenKeys = await this.databaseKeys(registeredPath)
        const collision = postOpenKeys.some(key => !initialKeys.includes(key) && this.openDatabaseKeys.has(key))
        if (collision) throw new Error(`DuckDB file is already open in this process: ${registeredPath}`)
        postOpenKeys.forEach(key => this.openDatabaseKeys.add(key))
        registeredKeys = [...new Set([...initialKeys, ...postOpenKeys])]
      }
      const store = new DuckDbModelStore(instance, connection, registeredKeys, registeredPath)
      await store.hardenPermissions()
      await store.migrate()
      await store.hardenPermissions()
      return store
    } catch (error) {
      try { connection?.closeSync() } catch { /* Startup already failed. */ }
      try { instance?.closeSync() } catch { /* Startup already failed. */ }
      registeredKeys.forEach(key => this.openDatabaseKeys.delete(key))
      throw error
    }
  }

  private static async databaseKeys(path: string): Promise<string[]> {
    const keys = new Set<string>([`path:${path}`])
    try {
      const canonicalPath = await realpath(path)
      keys.add(`realpath:${canonicalPath}`)
      const metadata = await stat(canonicalPath)
      if (metadata.ino !== 0) keys.add(`inode:${metadata.dev}:${metadata.ino}`)
    } catch {
      // The first opener commonly creates the file. A second key pass after
      // DuckDB initialization adds realpath/inode identities.
    }
    return [...keys]
  }

  private static async validateDatabaseTarget(path: string): Promise<void> {
    try {
      const metadata = await lstat(path)
      if (!metadata.isFile() || metadata.isSymbolicLink()) {
        throw new TypeError(`DuckDB path must be a regular file: ${path}`)
      }
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error
    }
  }

  async saveModel(input: SaveModelInput): Promise<StoredModel> {
    const id = input.id ?? randomUUID()
    validateModelId(id)
    validateModelName(input.name)
    validateModelSource(input.source)
    if (input.expectedRevision !== undefined
      && (!Number.isSafeInteger(input.expectedRevision) || input.expectedRevision < 0)) {
      throw new RangeError('expectedRevision must be a non-negative safe integer')
    }

    return this.serial(() => this.transaction(async () => {
      const existing = await this.getModelDirect(id)
      if (!existing) {
        if (input.expectedRevision !== undefined && input.expectedRevision !== 0) {
          throw new ModelRevisionConflictError(id, input.expectedRevision, null)
        }
        const [{ model_count }] = await this.rows('SELECT count(*) AS model_count FROM mcp_models')
        if (safeInteger({ model_count }, 'model_count') >= MAX_STORED_MODEL_COUNT) {
          throw new CatalogQuotaError(
            `Model catalog is limited to ${MAX_STORED_MODEL_COUNT.toLocaleString()} models`,
            'models',
            MAX_STORED_MODEL_COUNT,
          )
        }
        await this.ensureRevisionQuota(id)
        await this.ensureSourceQuota(input.source)
        await this.connection.run(`
          INSERT INTO mcp_models (id, name, source, revision)
          VALUES ($id, $name, $source, 0)
        `, { id, name: input.name, source: input.source })
        await this.connection.run(`
          INSERT INTO mcp_model_revisions (model_id, revision, name, source)
          VALUES ($id, 0, $name, $source)
        `, { id, name: input.name, source: input.source })
      } else {
        if (input.expectedRevision !== undefined && input.expectedRevision !== existing.revision) {
          throw new ModelRevisionConflictError(id, input.expectedRevision, existing.revision)
        }
        if (existing.name === input.name && existing.source === input.source) return existing
        const sourceChanged = existing.source !== input.source
        const revision = existing.revision + (sourceChanged ? 1 : 0)
        if (!Number.isSafeInteger(revision)) throw new RangeError('Model revision exceeds the safe integer range')
        if (sourceChanged) {
          await this.ensureRevisionQuota(id)
          await this.ensureSourceQuota(input.source)
        }
        await this.connection.run(`
          UPDATE mcp_models
          SET name = $name, source = $source, revision = $revision, updated_at = current_timestamp
          WHERE id = $id
        `, { id, name: input.name, source: input.source, revision })
        if (sourceChanged) {
          await this.connection.run(`
            INSERT INTO mcp_model_revisions (model_id, revision, name, source)
            VALUES ($id, $revision, $name, $source)
          `, { id, revision, name: input.name, source: input.source })
        } else {
          await this.connection.run(`
            UPDATE mcp_model_revisions SET name = $name
            WHERE model_id = $id AND revision = $revision
          `, { id, revision, name: input.name })
        }
      }
      const stored = await this.getModelDirect(id)
      if (!stored) throw new Error(`DuckDB failed to save model ${id}`)
      return stored
    }))
  }

  async saveModelSource(input: SaveModelSourceInput): Promise<StoredModel> {
    validateModelId(input.id)
    validateModelSource(input.source)
    if (!Number.isSafeInteger(input.expectedRevision) || input.expectedRevision < 0) {
      throw new RangeError('expectedRevision must be a non-negative safe integer')
    }

    return this.serial(() => this.transaction(async () => {
      const existing = await this.getModelDirect(input.id)
      if (!existing || existing.revision !== input.expectedRevision) {
        throw new ModelRevisionConflictError(
          input.id,
          input.expectedRevision,
          existing?.revision ?? null,
        )
      }
      if (existing.source === input.source) return existing
      const revision = existing.revision + 1
      if (!Number.isSafeInteger(revision)) throw new RangeError('Model revision exceeds the safe integer range')
      await this.ensureRevisionQuota(input.id)
      await this.ensureSourceQuota(input.source)
      await this.connection.run(`
        UPDATE mcp_models
        SET source = $source, revision = $revision, updated_at = current_timestamp
        WHERE id = $id
      `, { id: input.id, source: input.source, revision })
      await this.connection.run(`
        INSERT INTO mcp_model_revisions (model_id, revision, name, source)
        VALUES ($id, $revision, $name, $source)
      `, { id: input.id, revision, name: existing.name, source: input.source })
      const stored = await this.getModelDirect(input.id)
      if (!stored) throw new Error(`DuckDB failed to save model ${input.id}`)
      return stored
    }))
  }

  getModel(id: string): Promise<StoredModel | null> {
    validateModelId(id)
    return this.serial(() => this.getModelDirect(id))
  }

  getModelRevision(id: string, revision: number): Promise<StoredModelRevision | null> {
    validateModelId(id)
    this.validateRevision(revision)
    return this.serial(() => this.getModelRevisionDirect(id, revision))
  }

  listModels(limit = 100): Promise<StoredModelSummary[]> {
    const safeLimit = boundedLimit(limit)
    return this.serial(async () => {
      const rows = await this.rows(`
        SELECT id, name, revision, length(source) AS source_length, created_at, updated_at
        FROM mcp_models
        ORDER BY updated_at DESC, id
        LIMIT $limit
      `, { limit: safeLimit })
      return rows.map(row => ({
        id: requiredString(row, 'id'),
        name: requiredString(row, 'name'),
        revision: safeInteger(row, 'revision'),
        sourceLength: safeInteger(row, 'source_length'),
        createdAt: isoTimestamp(row, 'created_at'),
        updatedAt: isoTimestamp(row, 'updated_at'),
      }))
    })
  }

  listModelRevisions(id: string, limit = 100): Promise<StoredModelRevisionSummary[]> {
    validateModelId(id)
    const safeLimit = boundedLimit(limit)
    return this.serial(async () => {
      const rows = await this.rows(`
        SELECT model_id AS id, name, revision, length(source) AS source_length, created_at
        FROM mcp_model_revisions
        WHERE model_id = $id
        ORDER BY revision DESC
        LIMIT $limit
      `, { id, limit: safeLimit })
      return rows.map(row => ({
        id: requiredString(row, 'id'),
        name: requiredString(row, 'name'),
        revision: safeInteger(row, 'revision'),
        sourceLength: safeInteger(row, 'source_length'),
        createdAt: isoTimestamp(row, 'created_at'),
      }))
    })
  }

  recordBuild(input: RecordBuildInput): Promise<StoredBuild> {
    const id = input.id ?? randomUUID()
    validateModelId(id)
    validateModelSource(input.source)
    const hasModelId = input.modelId !== undefined
    const hasModelRevision = input.modelRevision !== undefined
    if (hasModelId !== hasModelRevision) {
      throw new TypeError('modelId and modelRevision must be provided together')
    }
    if (input.modelId !== undefined) validateModelId(input.modelId)
    if (input.modelRevision !== undefined) this.validateRevision(input.modelRevision)
    if (!/^[a-f0-9]{64}$/i.test(input.sourceSha256)) throw new TypeError('sourceSha256 must be a SHA-256 hex digest')
    if (sha256(input.source) !== input.sourceSha256.toLowerCase()) {
      throw new TypeError('sourceSha256 must match the exact build source')
    }
    if (input.quality !== 'preview' && input.quality !== 'full') throw new TypeError('Unsupported build quality')
    if (!['succeeded', 'failed', 'cancelled'].includes(input.status)) throw new TypeError('Unsupported build status')
    if (!Number.isFinite(input.durationMs) || input.durationMs < 0) throw new RangeError('durationMs must be non-negative')
    if (input.warnings.length > MAX_STORED_BUILD_WARNINGS
      || input.warnings.some(warning => warning.length > MAX_STORED_WARNING_LENGTH || !isWellFormedUnicode(warning))) {
      throw new RangeError(`Build warnings are limited to ${MAX_STORED_BUILD_WARNINGS} entries of ${MAX_STORED_WARNING_LENGTH} characters`)
    }
    if (input.status === 'succeeded' && !input.metrics) throw new TypeError('Successful builds require metrics')
    if (input.status !== 'succeeded' && input.metrics) throw new TypeError('Only successful builds may contain metrics')
    if (input.status === 'succeeded' && input.error) throw new TypeError('Successful builds cannot contain an error')
    if (input.status !== 'succeeded' && !input.error) {
      throw new TypeError('Failed and cancelled builds require an attested diagnostic')
    }
    if (input.metrics) {
      for (const [name, value] of [
        ['meshCount', input.metrics.meshCount],
        ['triangleCount', input.metrics.triangleCount],
      ] as const) {
        if (!Number.isSafeInteger(value) || value < 0) throw new RangeError(`${name} must be a non-negative safe integer`)
      }
      if (!Number.isFinite(input.metrics.volume) || input.metrics.volume < 0
        || !Number.isFinite(input.metrics.surfaceArea) || input.metrics.surfaceArea < 0) {
        throw new RangeError('Build volume and surface area must be finite and non-negative')
      }
    }
    const executionJson = serializeExecutionDescriptor(input.execution, input.quality, input.status)
    let errorJson: string | null = null
    if (input.error) {
      validateBuildDiagnostic(input.error, 'Build diagnostic')
      validateDiagnosticExecutionContext(input.error, input.execution, 'Build diagnostic')
      // Snapshot before the first async boundary. The caller cannot mutate a
      // previously validated diagnostic while this write waits in the serial
      // queue and thereby bypass exact-key, size or route-context admission.
      const candidateJson = JSON.stringify(input.error)
      const snapshot = JSON.parse(candidateJson) as unknown
      validateBuildDiagnostic(snapshot, 'Build diagnostic snapshot')
      validateDiagnosticExecutionContext(
        snapshot,
        input.execution,
        'Build diagnostic snapshot',
      )
      errorJson = JSON.stringify(snapshot)
    }
    const route = parseGeometrySourceRoutingHeader(input.source)
    if (route.languageContract !== input.execution.languageContract
      || route.requiredCapabilities.length !== input.execution.requiredCapabilities.length
      || route.requiredCapabilities.some((capability, index) => (
        capability !== input.execution.requiredCapabilities[index]
      ))) {
      throw new TypeError('Build execution provenance must match the exact source routing header')
    }

    return this.serial(() => this.transaction(async () => {
      if (input.modelId === undefined) await this.ensureSourceQuota(input.source)
      if (input.modelId !== undefined && input.modelRevision !== undefined) {
        const snapshot = await this.getModelRevisionDirect(input.modelId, input.modelRevision)
        if (!snapshot) {
          throw new TypeError(`Model ${input.modelId} revision ${input.modelRevision} does not exist`)
        }
        if (snapshot.source !== input.source) {
          throw new TypeError('Build source must match the referenced immutable model revision')
        }
      }
      const metrics = input.metrics
      await this.connection.run(`
        INSERT INTO mcp_builds (
          id, model_id, model_revision, source_sha256, source_snapshot, source_attestation, quality, status,
          duration_ms, warnings_json, mesh_count, triangle_count, volume,
          surface_area, reduced, error_json, execution_json
        ) VALUES (
          $id, $model_id, $model_revision, $source_sha256, $source_snapshot, $source_attestation, $quality, $status,
          $duration_ms, $warnings_json, $mesh_count, $triangle_count, $volume,
          $surface_area, $reduced, $error_json, $execution_json
        )
      `, {
        id,
        model_id: input.modelId ?? null,
        model_revision: input.modelRevision ?? null,
        source_sha256: input.sourceSha256,
        source_snapshot: input.modelId === undefined ? input.source : null,
        source_attestation: input.modelId === undefined ? 'inline-snapshot' : 'model-revision',
        quality: input.quality,
        status: input.status,
        duration_ms: input.durationMs,
        warnings_json: JSON.stringify(input.warnings),
        mesh_count: metrics?.meshCount ?? null,
        triangle_count: metrics?.triangleCount ?? null,
        volume: metrics?.volume ?? null,
        surface_area: metrics?.surfaceArea ?? null,
        reduced: metrics?.reduced ?? null,
        error_json: errorJson,
        execution_json: executionJson,
      })
      await this.pruneBuilds(id)
      const build = await this.getBuildDirect(id)
      if (!build) throw new Error(`DuckDB failed to record build ${id}`)
      return build
    }))
  }

  getBuild(id: string): Promise<StoredBuild | null> {
    validateModelId(id)
    return this.serial(() => this.getBuildDirect(id))
  }

  listBuilds(options: { modelId?: string; limit?: number } = {}): Promise<StoredBuild[]> {
    if (options.modelId) validateModelId(options.modelId)
    const limit = boundedLimit(options.limit)
    return this.serial(async () => {
      const rows = options.modelId
        ? await this.rows(`
            SELECT build.*, snapshot.source AS revision_source
            FROM mcp_builds AS build
            LEFT JOIN mcp_model_revisions AS snapshot
              ON snapshot.model_id = build.model_id AND snapshot.revision = build.model_revision
            WHERE build.model_id = $model_id
            ORDER BY build.created_at DESC, build.id
            LIMIT $limit
          `, { model_id: options.modelId, limit })
        : await this.rows(`
            SELECT build.*, snapshot.source AS revision_source
            FROM mcp_builds AS build
            LEFT JOIN mcp_model_revisions AS snapshot
              ON snapshot.model_id = build.model_id AND snapshot.revision = build.model_revision
            ORDER BY build.created_at DESC, build.id
            LIMIT $limit
          `, { limit })
      return rows.map(buildFromRow)
    })
  }

  storeArtifact(input: StoreArtifactInput): Promise<StoredArtifactSummary> {
    const id = input.id ?? randomUUID()
    validateModelId(id)
    validateModelId(input.buildId)
    if (input.modelId !== undefined) validateModelId(input.modelId)
    if (!input.fileName || input.fileName.length > 255 || /[/\\\0]/.test(input.fileName)
      || !isWellFormedUnicode(input.fileName)) {
      throw new TypeError('fileName must be a plain file name of at most 255 characters')
    }
    if (input.format !== 'stl' && input.format !== 'obj') throw new TypeError('Unsupported artifact format')
    if (input.mimeType !== expectedArtifactMimeType(input.format)) throw new TypeError('mimeType must match artifact format')
    if (!/^[a-f0-9]{64}$/i.test(input.sha256)) throw new TypeError('sha256 must be a SHA-256 hex digest')
    if (!(input.data instanceof Uint8Array)) throw new TypeError('Artifact data must be a Uint8Array')
    if (input.data.byteLength > MAX_STORED_ARTIFACT_BYTES) {
      throw new RangeError(`Artifact exceeds the ${MAX_STORED_ARTIFACT_BYTES.toLocaleString()}-byte storage limit`)
    }
    if (sha256(input.data).toLowerCase() !== input.sha256.toLowerCase()) {
      throw new TypeError('sha256 must match artifact data')
    }

    return this.serial(() => this.transaction(async () => {
      const build = await this.getBuildDirect(input.buildId)
      if (!build) throw new TypeError(`Build ${input.buildId} does not exist`)
      if (input.modelId !== undefined && input.modelId !== build.modelId) {
        throw new TypeError('Artifact modelId must match its build')
      }
      const modelId = build.modelId
      await this.connection.run(`
        INSERT INTO mcp_artifacts (
          id, build_id, model_id, format, file_name, mime_type,
          sha256, byte_length, data
        ) VALUES (
          $id, $build_id, $model_id, $format, $file_name, $mime_type,
          $sha256, $byte_length, $data
        )
      `, {
        id,
        build_id: input.buildId,
        model_id: modelId,
        format: input.format,
        file_name: input.fileName,
        mime_type: input.mimeType,
        sha256: input.sha256,
        byte_length: input.data.byteLength,
        data: blobValue(input.data),
      })
      await this.pruneArtifacts(id)
      await this.pruneBuilds(input.buildId)
      const artifact = await this.getArtifactDirect(id, false)
      if (!artifact) throw new Error(`DuckDB failed to store artifact ${id}`)
      return artifact
    }))
  }

  getArtifact(id: string): Promise<StoredArtifact | null> {
    validateModelId(id)
    return this.serial(async () => {
      const summary = await this.getArtifactDirect(id, true)
      if (!summary) return null
      if (!('data' in summary)) throw new Error(`DuckDB did not return artifact data for ${id}`)
      return summary
    })
  }

  listArtifacts(limit = 100): Promise<StoredArtifactSummary[]> {
    const safeLimit = boundedLimit(limit)
    return this.serial(async () => {
      const rows = await this.rows(`
        SELECT id, build_id, model_id, format, file_name, mime_type,
               sha256, byte_length, created_at
        FROM mcp_artifacts
        ORDER BY created_at DESC, id
        LIMIT $limit
      `, { limit: safeLimit })
      return rows.map(artifactSummaryFromRow)
    })
  }

  getCatalogStats(): Promise<CatalogStats> {
    return this.serial(async () => {
      const rows = await this.rows(`
        SELECT
          (SELECT count(*) FROM mcp_models) AS model_count,
          (SELECT count(*) FROM mcp_model_revisions) AS revision_count,
          (SELECT count(*) FROM mcp_builds) AS build_count,
          (SELECT count(*) FROM mcp_artifacts) AS artifact_count,
          (SELECT coalesce(sum(strlen(source)), 0) FROM mcp_model_revisions)
            + (SELECT coalesce(sum(strlen(source_snapshot)), 0) FROM mcp_builds) AS source_bytes,
          (SELECT coalesce(sum(byte_length), 0) FROM mcp_artifacts) AS artifact_bytes,
          (SELECT count(*) FROM mcp_builds WHERE status = 'succeeded') AS succeeded_count,
          (SELECT count(*) FROM mcp_builds WHERE status = 'failed') AS failed_count,
          (SELECT count(*) FROM mcp_builds WHERE status = 'cancelled') AS cancelled_count
      `)
      const row = rows[0]
      if (!row) throw new Error('DuckDB did not return catalog statistics')
      return {
        modelCount: safeInteger(row, 'model_count'),
        revisionCount: safeInteger(row, 'revision_count'),
        buildCount: safeInteger(row, 'build_count'),
        artifactCount: safeInteger(row, 'artifact_count'),
        storedSourceBytes: safeInteger(row, 'source_bytes'),
        storedArtifactBytes: safeInteger(row, 'artifact_bytes'),
        buildsByStatus: {
          succeeded: safeInteger(row, 'succeeded_count'),
          failed: safeInteger(row, 'failed_count'),
          cancelled: safeInteger(row, 'cancelled_count'),
        },
        limits: {
          models: MAX_STORED_MODEL_COUNT,
          revisions: MAX_STORED_MODEL_REVISION_COUNT,
          revisionsPerModel: MAX_STORED_REVISIONS_PER_MODEL,
          sourceBytes: MAX_STORED_MODEL_SOURCE_BYTES,
          builds: MAX_STORED_BUILD_COUNT,
          artifacts: MAX_STORED_ARTIFACT_COUNT,
          artifactBytes: MAX_STORED_ARTIFACT_TOTAL_BYTES,
        },
      }
    })
  }

  close(): Promise<void> {
    if (this.closePromise) return this.closePromise
    this.closed = true
    this.closePromise = (async () => {
      await this.tail
      try {
        this.connection.closeSync()
      } finally {
        try {
          this.instance.closeSync()
        } finally {
          this.registeredKeys.forEach(key => DuckDbModelStore.openDatabaseKeys.delete(key))
        }
      }
    })()
    return this.closePromise
  }

  private async migrate(): Promise<void> {
    await this.connection.run(`
      CREATE TABLE IF NOT EXISTS mcp_schema_migrations (
        version INTEGER PRIMARY KEY,
        applied_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp
      )
    `)
    const versions = (await this.rows(`
      SELECT version FROM mcp_schema_migrations ORDER BY version
    `)).map(row => safeInteger(row, 'version'))
    const current = versions.at(-1) ?? 0
    if (current > LATEST_SCHEMA_VERSION) {
      throw new Error(`DuckDB catalog schema ${current} is newer than supported schema ${LATEST_SCHEMA_VERSION}`)
    }
    if (versions.some((version, index) => version !== index + 1)) {
      throw new Error('DuckDB catalog has a non-contiguous migration history')
    }
    for (let version = current + 1; version <= LATEST_SCHEMA_VERSION; version++) {
      await this.transaction(async () => {
        if (version === 1) await this.applyMigration1()
        else if (version === 2) await this.applyMigration2()
        else if (version === 3) await this.applyMigration3()
        else if (version === 4) await this.applyMigration4()
        else if (version === 5) await this.applyMigration5()
        else if (version === 6) await this.applyMigration6()
        await this.connection.run(`
          INSERT INTO mcp_schema_migrations (version) VALUES ($version)
        `, { version })
      })
    }
    await this.transaction(async () => {
      await this.pruneArtifacts('')
      await this.pruneBuilds('')
    })
  }

  /** Legacy baseline retained so previously created development catalogs have a real upgrade path. */
  private async applyMigration1(): Promise<void> {
    await this.connection.run(`
      CREATE TABLE IF NOT EXISTS mcp_models (
        id VARCHAR PRIMARY KEY,
        name VARCHAR NOT NULL,
        source VARCHAR NOT NULL,
        revision BIGINT NOT NULL CHECK (revision >= 0),
        created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
        updated_at TIMESTAMP NOT NULL DEFAULT current_timestamp
      )
    `)
    await this.connection.run(`
      CREATE TABLE IF NOT EXISTS mcp_builds (
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
    await this.connection.run(`
      CREATE TABLE IF NOT EXISTS mcp_artifacts (
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
  }

  private async applyMigration2(): Promise<void> {
    const migrationTimestamp = await this.timestampExpression('mcp_schema_migrations', 'applied_at', 'applied_at')
    if (migrationTimestamp !== 'applied_at') {
      await this.connection.run(`
        ALTER TABLE mcp_schema_migrations ALTER applied_at SET DATA TYPE TIMESTAMPTZ
        USING ${migrationTimestamp}
      `)
    }

    const modelCreated = await this.timestampExpression('mcp_models', 'created_at', 'model.created_at')
    const modelUpdated = await this.timestampExpression('mcp_models', 'updated_at', 'model.updated_at')
    const buildCreated = await this.timestampExpression('mcp_builds', 'created_at', 'build.created_at')
    const artifactCreated = await this.timestampExpression('mcp_artifacts', 'created_at', 'artifact.created_at')
    const hasRevisions = await this.tableExists('mcp_model_revisions')
    const revisionCreated = hasRevisions
      ? await this.timestampExpression('mcp_model_revisions', 'created_at', 'snapshot.created_at')
      : 'model.updated_at'

    for (const table of [
      'mcp_artifacts_migration_v2',
      'mcp_builds_migration_v2',
      'mcp_model_revisions_migration_v2',
      'mcp_models_migration_v2',
    ]) await this.connection.run(`DROP TABLE IF EXISTS ${table}`)

    await this.connection.run(`
      CREATE TABLE mcp_models_migration_v2 (
        id VARCHAR, name VARCHAR, source VARCHAR, revision BIGINT,
        created_at TIMESTAMPTZ, updated_at TIMESTAMPTZ
      )
    `)
    await this.connection.run(`
      INSERT INTO mcp_models_migration_v2
      SELECT model.id, model.name, model.source, model.revision,
             ${modelCreated}, ${modelUpdated}
      FROM mcp_models AS model
    `)
    await this.connection.run(`
      CREATE TABLE mcp_model_revisions_migration_v2 (
        model_id VARCHAR, revision BIGINT, name VARCHAR, source VARCHAR,
        created_at TIMESTAMPTZ
      )
    `)
    if (hasRevisions) {
      await this.connection.run(`
        INSERT INTO mcp_model_revisions_migration_v2
        SELECT snapshot.model_id, snapshot.revision, snapshot.name, snapshot.source,
               ${revisionCreated}
        FROM mcp_model_revisions AS snapshot
      `)
    }
    await this.connection.run(`
      INSERT INTO mcp_model_revisions_migration_v2
      SELECT model.id, model.revision, model.name, model.source, model.updated_at
      FROM mcp_models_migration_v2 AS model
      WHERE NOT EXISTS (
        SELECT 1 FROM mcp_model_revisions_migration_v2 AS snapshot
        WHERE snapshot.model_id = model.id AND snapshot.revision = model.revision
      )
    `)
    await this.connection.run(`
      CREATE TABLE mcp_builds_migration_v2 (
        id VARCHAR, model_id VARCHAR, model_revision BIGINT, source_sha256 VARCHAR,
        source_snapshot VARCHAR, source_attestation VARCHAR,
        quality VARCHAR, status VARCHAR, duration_ms DOUBLE, warnings_json JSON,
        mesh_count BIGINT, triangle_count BIGINT, volume DOUBLE, surface_area DOUBLE,
        reduced BOOLEAN, error_json JSON, execution_json JSON, created_at TIMESTAMPTZ
      )
    `)
    await this.connection.run(`
      INSERT INTO mcp_builds_migration_v2
      SELECT build.id,
             CASE WHEN snapshot.model_id IS NULL
                        OR lower(build.source_sha256) <> sha256(snapshot.source)
                  THEN NULL ELSE build.model_id END,
             CASE WHEN snapshot.model_id IS NULL
                        OR lower(build.source_sha256) <> sha256(snapshot.source)
                  THEN NULL ELSE build.model_revision END,
             build.source_sha256, NULL,
             CASE WHEN snapshot.model_id IS NOT NULL
                         AND lower(build.source_sha256) = sha256(snapshot.source)
                  THEN 'model-revision' ELSE 'historical-unattested' END,
             build.quality, build.status, build.duration_ms,
             build.warnings_json, build.mesh_count, build.triangle_count, build.volume,
             build.surface_area, build.reduced, build.error_json,
             json_object(
               'languageContract', 'legacy/current',
               'requiredCapabilities', json_array(),
               'engineClass', 'manifold',
               'engineKey', 'manifold-wasm-v1',
               'kernelFingerprint', 'manifold-wasm-v1',
               'semanticProgramVersion', 'legacy-direct-evaluator-v1',
               'capabilityManifestVersion', 'manifold-node-v1',
               'manifestDigest', 'ae4ee188f2cf4898699318745e9eda48f96672e49b4b6d857f371ce7ed90013c',
               'purpose', build.quality,
               'quality', build.quality,
               'representation', 'mesh',
               'evidence', 'legacy-backfill',
               'effectiveLimits', json_object(),
               'automaticFallback', false
             ),
             ${buildCreated}
      FROM mcp_builds AS build
      LEFT JOIN mcp_model_revisions_migration_v2 AS snapshot
        ON snapshot.model_id = build.model_id AND snapshot.revision = build.model_revision
    `)
    await this.connection.run(`
      CREATE TABLE mcp_artifacts_migration_v2 (
        id VARCHAR, build_id VARCHAR, model_id VARCHAR, format VARCHAR,
        file_name VARCHAR, mime_type VARCHAR, sha256 VARCHAR, byte_length BIGINT,
        data BLOB, created_at TIMESTAMPTZ
      )
    `)
    await this.connection.run(`
      INSERT INTO mcp_artifacts_migration_v2
      SELECT artifact.id, artifact.build_id, build.model_id, artifact.format,
             artifact.file_name, artifact.mime_type, artifact.sha256,
             artifact.byte_length, artifact.data, ${artifactCreated}
      FROM mcp_artifacts AS artifact
      JOIN mcp_builds_migration_v2 AS build ON build.id = artifact.build_id
    `)

    await this.connection.run('DROP TABLE mcp_artifacts')
    await this.connection.run('DROP TABLE mcp_builds')
    if (hasRevisions) await this.connection.run('DROP TABLE mcp_model_revisions')
    await this.connection.run('DROP TABLE mcp_models')
    await this.createLatestTables()
    await this.connection.run('INSERT INTO mcp_models SELECT * FROM mcp_models_migration_v2')
    await this.connection.run('INSERT INTO mcp_model_revisions SELECT * FROM mcp_model_revisions_migration_v2')
    await this.connection.run('INSERT INTO mcp_builds SELECT * FROM mcp_builds_migration_v2')
    await this.connection.run('INSERT INTO mcp_artifacts SELECT * FROM mcp_artifacts_migration_v2')
    await this.connection.run('DROP TABLE mcp_artifacts_migration_v2')
    await this.connection.run('DROP TABLE mcp_builds_migration_v2')
    await this.connection.run('DROP TABLE mcp_model_revisions_migration_v2')
    await this.connection.run('DROP TABLE mcp_models_migration_v2')
  }

  private async applyMigration3(): Promise<void> {
    const hasExecution = await this.columnExists('mcp_builds', 'execution_json')
    if (hasExecution && !await this.columnIsNullable('mcp_builds', 'execution_json')) return

    for (const table of ['mcp_artifacts_migration_v3', 'mcp_builds_migration_v3']) {
      await this.connection.run(`DROP TABLE IF EXISTS ${table}`)
    }
    await this.connection.run(`
      CREATE TABLE mcp_builds_migration_v3 (
        id VARCHAR, model_id VARCHAR, model_revision BIGINT, source_sha256 VARCHAR,
        source_snapshot VARCHAR, source_attestation VARCHAR,
        quality VARCHAR, status VARCHAR, duration_ms DOUBLE, warnings_json JSON,
        mesh_count BIGINT, triangle_count BIGINT, volume DOUBLE, surface_area DOUBLE,
        reduced BOOLEAN, error_json JSON, execution_json JSON, created_at TIMESTAMPTZ
      )
    `)
    const existingExecution = hasExecution ? 'build.execution_json' : 'NULL'
    await this.connection.run(`
      INSERT INTO mcp_builds_migration_v3
      SELECT build.id,
             CASE WHEN (snapshot.model_id IS NULL AND build.model_id IS NOT NULL)
                        OR (snapshot.model_id IS NOT NULL
                            AND lower(build.source_sha256) <> sha256(snapshot.source))
                  THEN NULL ELSE build.model_id END,
             CASE WHEN (snapshot.model_id IS NULL AND build.model_revision IS NOT NULL)
                        OR (snapshot.model_id IS NOT NULL
                            AND lower(build.source_sha256) <> sha256(snapshot.source))
                  THEN NULL ELSE build.model_revision END,
             build.source_sha256, NULL,
             CASE WHEN snapshot.model_id IS NOT NULL
                         AND lower(build.source_sha256) = sha256(snapshot.source)
                  THEN 'model-revision' ELSE 'historical-unattested' END,
             build.quality, build.status, build.duration_ms, build.warnings_json,
             build.mesh_count, build.triangle_count, build.volume, build.surface_area,
             build.reduced, build.error_json,
             coalesce(
               ${existingExecution},
               json_object(
                 'languageContract', 'legacy/current',
                 'requiredCapabilities', json_array(),
                 'engineClass', 'manifold',
                 'engineKey', 'manifold-wasm-v1',
                 'kernelFingerprint', 'manifold-wasm-v1',
                 'semanticProgramVersion', 'legacy-direct-evaluator-v1',
                 'capabilityManifestVersion', 'manifold-node-v1',
                 'manifestDigest', 'ae4ee188f2cf4898699318745e9eda48f96672e49b4b6d857f371ce7ed90013c',
                 'purpose', build.quality,
                 'quality', build.quality,
                 'representation', 'mesh',
                 'evidence', 'legacy-backfill',
                 'effectiveLimits', json_object(),
                 'automaticFallback', false
               )
             ),
             build.created_at
      FROM mcp_builds AS build
      LEFT JOIN mcp_model_revisions AS snapshot
        ON snapshot.model_id = build.model_id AND snapshot.revision = build.model_revision
    `)
    await this.connection.run(`
      CREATE TABLE mcp_artifacts_migration_v3 (
        id VARCHAR, build_id VARCHAR, model_id VARCHAR, format VARCHAR,
        file_name VARCHAR, mime_type VARCHAR, sha256 VARCHAR, byte_length BIGINT,
        data BLOB, created_at TIMESTAMPTZ
      )
    `)
    await this.connection.run(`
      INSERT INTO mcp_artifacts_migration_v3
      SELECT id, build_id, model_id, format, file_name, mime_type, sha256,
             byte_length, data, created_at
      FROM mcp_artifacts
    `)

    await this.connection.run('DROP TABLE mcp_artifacts')
    await this.connection.run('DROP TABLE mcp_builds')
    await this.createLatestBuildTables()
    await this.connection.run('INSERT INTO mcp_builds SELECT * FROM mcp_builds_migration_v3')
    await this.connection.run('INSERT INTO mcp_artifacts SELECT * FROM mcp_artifacts_migration_v3')
    await this.connection.run('DROP TABLE mcp_artifacts_migration_v3')
    await this.connection.run('DROP TABLE mcp_builds_migration_v3')
  }

  private async applyMigration4(): Promise<void> {
    if (!await this.columnExists('mcp_builds', 'source_snapshot')) {
      await this.connection.run('ALTER TABLE mcp_builds ADD COLUMN source_snapshot VARCHAR')
    }
    await this.connection.run(`
      UPDATE mcp_builds
      SET execution_json = json_merge_patch(
        execution_json,
        json_object(
          'manifestDigest',
          CASE json_extract_string(execution_json, '$.capabilityManifestVersion')
            WHEN 'manifold-node-v1' THEN 'ae4ee188f2cf4898699318745e9eda48f96672e49b4b6d857f371ce7ed90013c'
            WHEN 'brep-contract-v1' THEN 'c4eefdbf1ff0add1e1705413d6ab766774487db0cb5a1d1434ab6c25f4ef1964'
            ELSE NULL
          END
        )
      )
      WHERE json_extract_string(execution_json, '$.manifestDigest') IS NULL
    `)
    const rows = await this.rows(`
      SELECT count(*) AS invalid_count
      FROM mcp_builds
      WHERE json_extract_string(execution_json, '$.manifestDigest') IS NULL
    `)
    if (safeInteger(rows[0], 'invalid_count') !== 0) {
      throw new TypeError('DuckDB migration found execution provenance with an unknown manifest')
    }
  }

  private async applyMigration5(): Promise<void> {
    if (!await this.columnExists('mcp_builds', 'source_attestation')) {
      await this.connection.run('ALTER TABLE mcp_builds ADD COLUMN source_attestation VARCHAR')
    }
    await this.connection.run(`
      UPDATE mcp_builds
      SET source_attestation = CASE
        WHEN model_id IS NOT NULL AND model_revision IS NOT NULL THEN 'model-revision'
        WHEN source_snapshot IS NOT NULL THEN 'inline-snapshot'
        ELSE 'historical-unattested'
      END
      WHERE source_attestation IS NULL
    `)
    // Fresh catalogs already have a NOT NULL + cross-field CHECK. DuckDB
    // cannot strengthen this column in place when artifact FKs/indexes depend
    // on mcp_builds, so upgraded catalogs rely on the same strict read-side
    // invariant until a future table-rebuild migration.
  }

  private async applyMigration6(): Promise<void> {
    if (!await this.columnExists('mcp_builds', 'execution_json')) return
    await this.connection.run(`
      UPDATE mcp_builds
      SET execution_json = json_merge_patch(
        execution_json,
        '{"engineClass":"mesh"}'
      )
      WHERE json_extract_string(execution_json, '$.engineClass') = 'manifold'
    `)
    await this.connection.run(`
      UPDATE mcp_builds
      SET execution_json = json_merge_patch(
        execution_json,
        '{"manifestDigest":"cc358238cabeb1d12e57458a8a94ddcdf53dadb3fd695bdbd360876e82cb9926"}'
      )
      WHERE json_extract_string(execution_json, '$.manifestDigest')
        = 'd58685b0fb5de314fbcc154d7d5d40128b1d9e71259a0f2a51452ca3986b5b74'
    `)
  }

  private async createLatestTables(): Promise<void> {
    await this.connection.run(`
      CREATE TABLE mcp_models (
        id VARCHAR PRIMARY KEY,
        name VARCHAR NOT NULL,
        source VARCHAR NOT NULL,
        revision BIGINT NOT NULL CHECK (revision >= 0),
        created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
        updated_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp
      )
    `)
    await this.connection.run(`
      CREATE TABLE mcp_model_revisions (
        model_id VARCHAR NOT NULL,
        revision BIGINT NOT NULL CHECK (revision >= 0),
        name VARCHAR NOT NULL,
        source VARCHAR NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
        PRIMARY KEY (model_id, revision),
        FOREIGN KEY (model_id) REFERENCES mcp_models(id)
      )
    `)
    await this.createLatestBuildTables()
  }

  private async createLatestBuildTables(): Promise<void> {
    await this.connection.run(`
      CREATE TABLE mcp_builds (
        id VARCHAR PRIMARY KEY,
        model_id VARCHAR,
        model_revision BIGINT,
        source_sha256 VARCHAR NOT NULL,
        source_snapshot VARCHAR,
        source_attestation VARCHAR NOT NULL CHECK (
          source_attestation IN ('model-revision', 'inline-snapshot', 'historical-unattested')
        ),
        quality VARCHAR NOT NULL CHECK (quality IN ('preview', 'full')),
        status VARCHAR NOT NULL CHECK (status IN ('succeeded', 'failed', 'cancelled')),
        duration_ms DOUBLE NOT NULL CHECK (isfinite(duration_ms) AND duration_ms >= 0),
        warnings_json JSON NOT NULL,
        mesh_count BIGINT,
        triangle_count BIGINT,
        volume DOUBLE,
        surface_area DOUBLE,
        reduced BOOLEAN,
        error_json JSON,
        execution_json JSON NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
        CHECK ((model_id IS NULL) = (model_revision IS NULL)),
        CHECK (
          (source_attestation = 'model-revision'
            AND model_id IS NOT NULL AND source_snapshot IS NULL)
          OR (source_attestation = 'inline-snapshot'
            AND model_id IS NULL AND source_snapshot IS NOT NULL)
          OR (source_attestation = 'historical-unattested'
            AND model_id IS NULL AND source_snapshot IS NULL)
        ),
        CHECK (
          (status = 'succeeded'
            AND mesh_count >= 0 AND triangle_count >= 0
            AND isfinite(volume) AND volume >= 0
            AND isfinite(surface_area) AND surface_area >= 0
            AND reduced IS NOT NULL AND error_json IS NULL)
          OR
          (status <> 'succeeded'
            AND mesh_count IS NULL AND triangle_count IS NULL
            AND volume IS NULL AND surface_area IS NULL AND reduced IS NULL)
        ),
        FOREIGN KEY (model_id, model_revision)
          REFERENCES mcp_model_revisions(model_id, revision)
      )
    `)
    await this.connection.run(`
      CREATE INDEX mcp_builds_model_created ON mcp_builds (model_id, created_at)
    `)
    await this.connection.run(`
      CREATE TABLE mcp_artifacts (
        id VARCHAR PRIMARY KEY,
        build_id VARCHAR NOT NULL,
        model_id VARCHAR,
        format VARCHAR NOT NULL CHECK (format IN ('stl', 'obj')),
        file_name VARCHAR NOT NULL,
        mime_type VARCHAR NOT NULL,
        sha256 VARCHAR NOT NULL,
        byte_length BIGINT NOT NULL CHECK (byte_length >= 0),
        data BLOB NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
        FOREIGN KEY (build_id) REFERENCES mcp_builds(id),
        FOREIGN KEY (model_id) REFERENCES mcp_models(id)
      )
    `)
  }

  private async tableExists(table: string): Promise<boolean> {
    const rows = await this.rows(`
      SELECT count(*) AS table_count
      FROM information_schema.tables
      WHERE table_schema = current_schema() AND table_name = $table
    `, { table })
    return safeInteger(rows[0], 'table_count') > 0
  }

  private async columnExists(table: string, column: string): Promise<boolean> {
    const rows = await this.rows(`
      SELECT count(*) AS column_count
      FROM information_schema.columns
      WHERE table_schema = current_schema()
        AND table_name = $table
        AND column_name = $column
    `, { table, column })
    return safeInteger(rows[0], 'column_count') > 0
  }

  private async columnIsNullable(table: string, column: string): Promise<boolean> {
    const rows = await this.rows(`
      SELECT is_nullable
      FROM information_schema.columns
      WHERE table_schema = current_schema()
        AND table_name = $table
        AND column_name = $column
    `, { table, column })
    if (!rows[0]) throw new Error(`DuckDB catalog has no ${table}.${column} column`)
    return requiredString(rows[0], 'is_nullable') === 'YES'
  }

  private async timestampExpression(table: string, column: string, reference: string): Promise<string> {
    const rows = await this.rows(`
      SELECT data_type
      FROM information_schema.columns
      WHERE table_schema = current_schema() AND table_name = $table AND column_name = $column
    `, { table, column })
    const type = rows[0] ? requiredString(rows[0], 'data_type') : null
    if (type === 'TIMESTAMP WITH TIME ZONE') return reference
    if (type?.startsWith('TIMESTAMP')) {
      return `timezone(current_setting('TimeZone'), ${reference})`
    }
    throw new Error(`DuckDB catalog has no compatible ${table}.${column} timestamp column`)
  }

  private async ensureSourceQuota(source: string): Promise<void> {
    const rows = await this.rows(`
      SELECT
        (SELECT coalesce(sum(strlen(source)), 0) FROM mcp_model_revisions)
        + (SELECT coalesce(sum(strlen(source_snapshot)), 0) FROM mcp_builds) AS source_bytes
    `)
    const storedBytes = safeInteger(rows[0], 'source_bytes')
    const additionalBytes = Buffer.byteLength(source, 'utf8')
    if (storedBytes + additionalBytes > MAX_STORED_MODEL_SOURCE_BYTES) {
      throw new CatalogQuotaError(
        `Stored immutable source snapshots are limited to ${MAX_STORED_MODEL_SOURCE_BYTES.toLocaleString()} UTF-8 bytes`,
        'source_bytes',
        MAX_STORED_MODEL_SOURCE_BYTES,
      )
    }
  }

  private async ensureRevisionQuota(modelId: string): Promise<void> {
    const rows = await this.rows(`
      SELECT count(*) AS total_count,
             count(*) FILTER (WHERE model_id = $model_id) AS model_count
      FROM mcp_model_revisions
    `, { model_id: modelId })
    if (safeInteger(rows[0], 'total_count') >= MAX_STORED_MODEL_REVISION_COUNT) {
      throw new CatalogQuotaError(
        `Model catalog is limited to ${MAX_STORED_MODEL_REVISION_COUNT.toLocaleString()} source revisions`,
        'revisions',
        MAX_STORED_MODEL_REVISION_COUNT,
      )
    }
    if (safeInteger(rows[0], 'model_count') >= MAX_STORED_REVISIONS_PER_MODEL) {
      throw new CatalogQuotaError(
        `Model ${modelId} is limited to ${MAX_STORED_REVISIONS_PER_MODEL.toLocaleString()} source revisions`,
        'revisions_per_model',
        MAX_STORED_REVISIONS_PER_MODEL,
      )
    }
  }

  private async pruneBuilds(keepId: string): Promise<void> {
    await this.connection.run(`
      DELETE FROM mcp_builds
      WHERE id IN (
        SELECT id FROM (
          SELECT id, row_number() OVER (
            ORDER BY (id = $keep_id) DESC, created_at DESC, id DESC
          ) AS ordinal
          FROM mcp_builds
        ) AS ranked
        WHERE ordinal > $max_count
      )
      AND NOT EXISTS (
        SELECT 1 FROM mcp_artifacts AS artifact
        WHERE artifact.build_id = mcp_builds.id
      )
    `, { keep_id: keepId, max_count: MAX_STORED_BUILD_COUNT })
  }

  private async pruneArtifacts(keepId: string): Promise<void> {
    await this.connection.run(`
      DELETE FROM mcp_artifacts
      WHERE id IN (
        SELECT id FROM (
          SELECT id,
                 row_number() OVER (
                   ORDER BY (id = $keep_id) DESC, created_at DESC, id DESC
                 ) AS ordinal,
                 sum(byte_length) OVER (
                   ORDER BY (id = $keep_id) DESC, created_at DESC, id DESC
                   ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
                 ) AS cumulative_bytes
          FROM mcp_artifacts
        ) AS ranked
        WHERE ordinal > $max_count OR cumulative_bytes > $max_bytes
      )
    `, {
      keep_id: keepId,
      max_count: MAX_STORED_ARTIFACT_COUNT,
      max_bytes: MAX_STORED_ARTIFACT_TOTAL_BYTES,
    })
  }

  private async hardenPermissions(): Promise<void> {
    if (!this.databasePath || process.platform === 'win32') return
    for (const path of [this.databasePath, `${this.databasePath}.wal`]) {
      try { await chmod(path, 0o600) } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error
      }
    }
  }

  private serial<T>(operation: () => Promise<T>): Promise<T> {
    if (this.closed) return Promise.reject(new Error('DuckDbModelStore is closed'))
    const result = this.tail.then(operation, operation)
    this.tail = result.then(() => undefined, () => undefined)
    return result
  }

  private async transaction<T>(operation: () => Promise<T>): Promise<T> {
    await this.connection.run('BEGIN TRANSACTION')
    let result: T
    try {
      result = await operation()
      await this.connection.run('COMMIT')
    } catch (error) {
      try { await this.connection.run('ROLLBACK') } catch { /* Preserve the original failure. */ }
      throw error
    }
    try { await this.hardenPermissions() } catch (error) {
      console.error('Could not harden DuckDB file permissions after commit:', error)
    }
    return result
  }

  private validateRevision(revision: number): void {
    if (!Number.isSafeInteger(revision) || revision < 0) {
      throw new RangeError('revision must be a non-negative safe integer')
    }
  }

  private async rows(sql: string, values?: Parameters<DuckDBConnection['run']>[1]): Promise<Row[]> {
    const result = await this.connection.run(sql, values)
    return await result.getRowObjectsJS() as Row[]
  }

  private async getModelDirect(id: string): Promise<StoredModel | null> {
    const rows = await this.rows('SELECT * FROM mcp_models WHERE id = $id', { id })
    return rows[0] ? modelFromRow(rows[0]) : null
  }

  private async getModelRevisionDirect(id: string, revision: number): Promise<StoredModelRevision | null> {
    const rows = await this.rows(`
      SELECT model_id AS id, name, source, revision, created_at
      FROM mcp_model_revisions
      WHERE model_id = $id AND revision = $revision
    `, { id, revision })
    return rows[0] ? modelRevisionFromRow(rows[0]) : null
  }

  private async getBuildDirect(id: string): Promise<StoredBuild | null> {
    const rows = await this.rows(`
      SELECT build.*, snapshot.source AS revision_source
      FROM mcp_builds AS build
      LEFT JOIN mcp_model_revisions AS snapshot
        ON snapshot.model_id = build.model_id AND snapshot.revision = build.model_revision
      WHERE build.id = $id
    `, { id })
    return rows[0] ? buildFromRow(rows[0]) : null
  }

  private async getArtifactDirect(id: string, withData: false): Promise<StoredArtifactSummary | null>
  private async getArtifactDirect(id: string, withData: true): Promise<StoredArtifact | null>
  private async getArtifactDirect(id: string, withData: boolean): Promise<StoredArtifactSummary | StoredArtifact | null> {
    const rows = await this.rows(withData
      ? 'SELECT * FROM mcp_artifacts WHERE id = $id'
      : `SELECT id, build_id, model_id, format, file_name, mime_type,
                sha256, byte_length, created_at
         FROM mcp_artifacts WHERE id = $id`, { id })
    if (!rows[0]) return null
    const summary = artifactSummaryFromRow(rows[0])
    if (!withData) return summary
    const data = rows[0].data
    if (!(data instanceof Uint8Array)) throw new TypeError('DuckDB returned invalid artifact data')
    if (data.byteLength !== summary.byteLength || data.byteLength > MAX_STORED_ARTIFACT_BYTES) {
      throw new RangeError('DuckDB returned artifact data with an invalid length')
    }
    if (sha256(data).toLowerCase() !== summary.sha256.toLowerCase()) {
      throw new TypeError('DuckDB returned artifact data with an invalid digest')
    }
    return { ...summary, data: new Uint8Array(data) }
  }
}
