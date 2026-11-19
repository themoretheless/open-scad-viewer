import { randomUUID } from 'node:crypto'
import { resolve } from 'node:path'
import { DuckDBConnection, DuckDBInstance } from '@duckdb/node-api'
import {
  CatalogQuotaError,
  ModelRevisionConflictError,
  validateModelId,
  validateModelName,
  validateModelSource,
  type CatalogStats,
  type ModelStore,
  type RecordBuildInput,
  type SaveModelInput,
  type SaveModelSourceInput,
  type StoreArtifactInput,
  type StoredArtifact,
  type StoredArtifactSummary,
  type StoredBuild,
  type StoredModel,
  type StoredModelRevision,
  type StoredModelRevisionSummary,
  type StoredModelSummary,
} from './modelStore'
import { parseGeometrySourceRoutingHeader } from '../core/geometryExecution'
import { boundedLimit, isoTimestamp, requiredString, safeInteger } from './duckdbModelStore/boundedLimit'
import {
  databaseKeys,
  deleteModelDirect,
  getArtifactDirect,
  getBuildDirect,
  getModelDirect,
  getModelRevisionDirect,
  hardenDatabasePermissions,
  insertArtifactAndPrune,
  listArtifactsDirect,
  listBuildsDirect,
  validateDatabaseTarget,
} from './duckdbModelStore/directQueries'
import { runDuckDbMigrations } from './duckdbModelStore/migrations'
import {
  ensureRevisionQuota,
  ensureSourceQuota,
  pruneBuilds,
  queryCatalogStats,
} from './duckdbModelStore/quotasAndPruning'
import {
  validateRecordBuildInput,
  validateRevision,
  validateStoreArtifactInput,
} from './duckdbModelStore/recordBuildValidation'
import {
  MAX_STORED_MODEL_COUNT,
  type DuckDbExecutor,
  type JsonValue,
  type Row,
  type SqlParams,
} from './duckdbModelStore/types'

export {
  MAX_LIST_LIMIT,
  MAX_STORED_ARTIFACT_COUNT,
  MAX_STORED_ARTIFACT_TOTAL_BYTES,
  MAX_STORED_BUILD_COUNT,
  MAX_STORED_MODEL_COUNT,
  MAX_STORED_MODEL_REVISION_COUNT,
  MAX_STORED_MODEL_SOURCE_BYTES,
  MAX_STORED_REVISIONS_PER_MODEL,
} from './duckdbModelStore/types'
export type { JsonValue, Row }

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
    const result = this.openTail.then(
      () => this.openInternal(path),
      () => this.openInternal(path),
    )
    this.openTail = result.then(
      () => undefined,
      () => undefined,
    )
    return result
  }

  private static async openInternal(path: string): Promise<DuckDbModelStore> {
    const registeredPath = path === ':memory:' ? null : resolve(path)
    if (registeredPath) await validateDatabaseTarget(registeredPath)
    const initialKeys = registeredPath ? await databaseKeys(registeredPath) : []
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
        const postOpenKeys = await databaseKeys(registeredPath)
        if (postOpenKeys.some(key => !initialKeys.includes(key) && this.openDatabaseKeys.has(key))) {
          throw new Error(`DuckDB file is already open in this process: ${registeredPath}`)
        }
        postOpenKeys.forEach(key => this.openDatabaseKeys.add(key))
        registeredKeys = [...new Set([...initialKeys, ...postOpenKeys])]
      }
      const store = new DuckDbModelStore(instance, connection, registeredKeys, registeredPath)
      await hardenDatabasePermissions(registeredPath)
      await runDuckDbMigrations(store.executor())
      await hardenDatabasePermissions(registeredPath)
      return store
    } catch (error) {
      try {
        connection?.closeSync()
      } catch {
        /* Startup already failed. */
      }
      try {
        instance?.closeSync()
      } catch {
        /* Startup already failed. */
      }
      registeredKeys.forEach(key => this.openDatabaseKeys.delete(key))
      throw error
    }
  }

  async saveModel(input: SaveModelInput): Promise<StoredModel> {
    const id = input.id ?? randomUUID()
    validateModelId(id)
    validateModelName(input.name)
    validateModelSource(input.source)
    if (
      input.expectedRevision !== undefined &&
      (!Number.isSafeInteger(input.expectedRevision) || input.expectedRevision < 0)
    ) {
      throw new RangeError('expectedRevision must be a non-negative safe integer')
    }
    const db = this.executor()
    return this.serial(() =>
      this.transaction(async () => {
        const existing = await getModelDirect(db, id)
        if (!existing) {
          if (input.expectedRevision !== undefined && input.expectedRevision !== 0) {
            throw new ModelRevisionConflictError(id, input.expectedRevision, null)
          }
          const [countRow = {}] = await this.rows('SELECT count(*) AS model_count FROM mcp_models')
          if (safeInteger(countRow, 'model_count') >= MAX_STORED_MODEL_COUNT) {
            throw new CatalogQuotaError(
              `Model catalog is limited to ${MAX_STORED_MODEL_COUNT.toLocaleString('en-US')} models`,
              'models',
              MAX_STORED_MODEL_COUNT,
            )
          }
          await ensureRevisionQuota(db, id)
          await ensureSourceQuota(db, input.source)
          await this.connection.run(
            'INSERT INTO mcp_models (id, name, source, revision) VALUES ($id, $name, $source, 0)',
            { id, name: input.name, source: input.source },
          )
          await this.connection.run(
            'INSERT INTO mcp_model_revisions (model_id, revision, name, source) VALUES ($id, 0, $name, $source)',
            { id, name: input.name, source: input.source },
          )
        } else {
          if (input.expectedRevision !== undefined && input.expectedRevision !== existing.revision) {
            throw new ModelRevisionConflictError(id, input.expectedRevision, existing.revision)
          }
          if (existing.name === input.name && existing.source === input.source) return existing
          const sourceChanged = existing.source !== input.source
          const revision = existing.revision + (sourceChanged ? 1 : 0)
          if (!Number.isSafeInteger(revision)) throw new RangeError('Model revision exceeds the safe integer range')
          if (sourceChanged) {
            await ensureRevisionQuota(db, id)
            await ensureSourceQuota(db, input.source)
          }
          await this.connection.run(
            'UPDATE mcp_models SET name = $name, source = $source, revision = $revision, updated_at = current_timestamp WHERE id = $id',
            { id, name: input.name, source: input.source, revision },
          )
          if (sourceChanged) {
            await this.connection.run(
              'INSERT INTO mcp_model_revisions (model_id, revision, name, source) VALUES ($id, $revision, $name, $source)',
              { id, revision, name: input.name, source: input.source },
            )
          } else {
            await this.connection.run(
              'UPDATE mcp_model_revisions SET name = $name WHERE model_id = $id AND revision = $revision',
              { id, revision, name: input.name },
            )
          }
        }
        const stored = await getModelDirect(db, id)
        if (!stored) throw new Error(`DuckDB failed to save model ${id}`)
        return stored
      }),
    )
  }

  async saveModelSource(input: SaveModelSourceInput): Promise<StoredModel> {
    validateModelId(input.id)
    validateModelSource(input.source)
    if (!Number.isSafeInteger(input.expectedRevision) || input.expectedRevision < 0) {
      throw new RangeError('expectedRevision must be a non-negative safe integer')
    }
    const db = this.executor()
    return this.serial(() =>
      this.transaction(async () => {
        const existing = await getModelDirect(db, input.id)
        if (!existing || existing.revision !== input.expectedRevision) {
          throw new ModelRevisionConflictError(input.id, input.expectedRevision, existing?.revision ?? null)
        }
        if (existing.source === input.source) return existing
        const revision = existing.revision + 1
        if (!Number.isSafeInteger(revision)) throw new RangeError('Model revision exceeds the safe integer range')
        await ensureRevisionQuota(db, input.id)
        await ensureSourceQuota(db, input.source)
        await this.connection.run(
          'UPDATE mcp_models SET source = $source, revision = $revision, updated_at = current_timestamp WHERE id = $id',
          { id: input.id, source: input.source, revision },
        )
        await this.connection.run(
          'INSERT INTO mcp_model_revisions (model_id, revision, name, source) VALUES ($id, $revision, $name, $source)',
          { id: input.id, revision, name: existing.name, source: input.source },
        )
        const stored = await getModelDirect(db, input.id)
        if (!stored) throw new Error(`DuckDB failed to save model ${input.id}`)
        return stored
      }),
    )
  }

  getModel(id: string): Promise<StoredModel | null> {
    validateModelId(id)
    return this.serial(() => getModelDirect(this.executor(), id))
  }

  getModelRevision(id: string, revision: number): Promise<StoredModelRevision | null> {
    validateModelId(id)
    validateRevision(revision)
    return this.serial(() => getModelRevisionDirect(this.executor(), id, revision))
  }

  listModels(limit = 100): Promise<StoredModelSummary[]> {
    const safeLimit = boundedLimit(limit)
    return this.serial(async () => {
      const rows = await this.rows(
        'SELECT id, name, revision, length(source) AS source_length, created_at, updated_at FROM mcp_models ORDER BY updated_at DESC, id LIMIT $limit',
        { limit: safeLimit },
      )
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
      const rows = await this.rows(
        'SELECT model_id AS id, name, revision, length(source) AS source_length, created_at FROM mcp_model_revisions WHERE model_id = $id ORDER BY revision DESC LIMIT $limit',
        { id, limit: safeLimit },
      )
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
    const { executionJson, errorJson } = validateRecordBuildInput(id, input)
    const route = parseGeometrySourceRoutingHeader(input.source)
    if (
      route.languageContract !== input.execution.languageContract ||
      route.requiredCapabilities.length !== input.execution.requiredCapabilities.length ||
      route.requiredCapabilities.some((capability, index) => capability !== input.execution.requiredCapabilities[index])
    ) {
      throw new TypeError('Build execution provenance must match the exact source routing header')
    }
    const db = this.executor()
    return this.serial(() =>
      this.transaction(async () => {
        if (input.modelId === undefined) await ensureSourceQuota(db, input.source)
        if (input.modelId !== undefined && input.modelRevision !== undefined) {
          const snapshot = await getModelRevisionDirect(db, input.modelId, input.modelRevision)
          if (!snapshot) throw new TypeError(`Model ${input.modelId} revision ${input.modelRevision} does not exist`)
          if (snapshot.source !== input.source)
            throw new TypeError('Build source must match the referenced immutable model revision')
        }
        const metrics = input.metrics
        await this.connection.run(
          `
        INSERT INTO mcp_builds (
          id, model_id, model_revision, source_sha256, source_snapshot, source_attestation, quality, status,
          duration_ms, warnings_json, mesh_count, triangle_count, volume, surface_area, reduced, error_json, execution_json
        ) VALUES (
          $id, $model_id, $model_revision, $source_sha256, $source_snapshot, $source_attestation, $quality, $status,
          $duration_ms, $warnings_json, $mesh_count, $triangle_count, $volume, $surface_area, $reduced, $error_json, $execution_json
        )
      `,
          {
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
          },
        )
        await pruneBuilds(db, id)
        const build = await getBuildDirect(db, id)
        if (!build) throw new Error(`DuckDB failed to record build ${id}`)
        return build
      }),
    )
  }

  getBuild(id: string): Promise<StoredBuild | null> {
    validateModelId(id)
    return this.serial(() => getBuildDirect(this.executor(), id))
  }

  listBuilds(options: { modelId?: string | undefined; limit?: number | undefined } = {}): Promise<StoredBuild[]> {
    if (options.modelId) validateModelId(options.modelId)
    const limit = boundedLimit(options.limit)
    return this.serial(() => listBuildsDirect(this.executor(), options.modelId, limit))
  }

  storeArtifact(input: StoreArtifactInput): Promise<StoredArtifactSummary> {
    const id = input.id ?? randomUUID()
    validateStoreArtifactInput(id, input)
    const db = this.executor()
    return this.serial(() => this.transaction(() => insertArtifactAndPrune(db, id, input)))
  }

  getArtifact(id: string): Promise<StoredArtifact | null> {
    validateModelId(id)
    return this.serial(async () => {
      const summary = await getArtifactDirect(this.executor(), id, true)
      if (!summary) return null
      if (!('data' in summary)) throw new Error(`DuckDB did not return artifact data for ${id}`)
      return summary
    })
  }

  listArtifacts(limit = 100): Promise<StoredArtifactSummary[]> {
    const safeLimit = boundedLimit(limit)
    return this.serial(() => listArtifactsDirect(this.executor(), safeLimit))
  }

  getCatalogStats(): Promise<CatalogStats> {
    return this.serial(() => queryCatalogStats(this.executor()))
  }

  deleteModel(id: string): Promise<boolean> {
    validateModelId(id)
    const db = this.executor()
    return this.serial(async () => {
      const deleted = await deleteModelDirect(db, id)
      if (deleted) {
        try {
          await hardenDatabasePermissions(this.databasePath)
        } catch (error) {
          console.error('Could not harden DuckDB file permissions after delete:', error)
        }
      }
      return deleted
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

  private executor(): DuckDbExecutor & { transaction: <T>(op: () => Promise<T>) => Promise<T> } {
    return {
      run: (sql: string, values?: SqlParams) => this.connection.run(sql, values),
      rows: (sql: string, values?: SqlParams) => this.rows(sql, values),
      transaction: <T>(operation: () => Promise<T>) => this.transaction(operation),
    }
  }

  private serial<T>(operation: () => Promise<T>): Promise<T> {
    if (this.closed) return Promise.reject(new Error('DuckDbModelStore is closed'))
    const result = this.tail.then(operation, operation)
    this.tail = result.then(
      () => undefined,
      () => undefined,
    )
    return result
  }

  private async transaction<T>(operation: () => Promise<T>): Promise<T> {
    await this.connection.run('BEGIN TRANSACTION')
    let result: T
    try {
      result = await operation()
      await this.connection.run('COMMIT')
    } catch (error) {
      try {
        await this.connection.run('ROLLBACK')
      } catch {
        /* Preserve the original failure. */
      }
      throw error
    }
    try {
      await hardenDatabasePermissions(this.databasePath)
    } catch (error) {
      console.error('Could not harden DuckDB file permissions after commit:', error)
    }
    return result
  }

  private async rows(sql: string, values?: SqlParams): Promise<Row[]> {
    const result = await this.connection.run(sql, values)
    return (await result.getRowObjectsJS()) as Row[]
  }
}
