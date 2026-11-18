import { safeInteger } from './boundedLimit'
import { pruneArtifacts, pruneBuilds } from './quotasAndPruning'
import { applyMigration1, applyMigration2, applyMigration3, columnExists } from './schemaV1ToV3'
import type { DuckDbExecutor } from './types'

export const LATEST_SCHEMA_VERSION = 7

export interface DuckDbMigrationContext extends DuckDbExecutor {
  transaction: <T>(operation: () => Promise<T>) => Promise<T>
}

async function hasPrimaryKey(db: DuckDbExecutor, tableName: string): Promise<boolean> {
  const rows = await db.rows(
    "SELECT count(*) AS pk_count FROM duckdb_constraints() WHERE table_name = $table AND constraint_type = 'PRIMARY KEY'",
    { table: tableName },
  )
  return safeInteger(rows[0] ?? {}, 'pk_count') > 0
}

async function createArtifactsTable(
  db: DuckDbExecutor,
  options: { buildFk?: boolean; modelFk?: boolean } = {},
): Promise<void> {
  const clauses = [
    'id VARCHAR PRIMARY KEY',
    'build_id VARCHAR NOT NULL',
    'model_id VARCHAR',
    "format VARCHAR NOT NULL CHECK (format IN ('stl', 'obj', 'step'))",
    'file_name VARCHAR NOT NULL',
    'mime_type VARCHAR NOT NULL',
    'sha256 VARCHAR NOT NULL',
    'byte_length BIGINT NOT NULL CHECK (byte_length >= 0)',
    'data BLOB NOT NULL',
    'created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp',
  ]
  if (options.buildFk !== false) clauses.push('FOREIGN KEY (build_id) REFERENCES mcp_builds(id)')
  if (options.modelFk !== false) clauses.push('FOREIGN KEY (model_id) REFERENCES mcp_models(id)')
  await db.run(`
    CREATE TABLE mcp_artifacts (
      ${clauses.join(',\n      ')}
    )
  `)
}

export async function createLatestBuildTables(db: DuckDbExecutor): Promise<void> {
  await db.run(`
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
  await db.run(`
    CREATE INDEX mcp_builds_model_created ON mcp_builds (model_id, created_at)
  `)
  await createArtifactsTable(db)
}

export async function createLatestTables(db: DuckDbExecutor): Promise<void> {
  await db.run(`
    CREATE TABLE mcp_models (
      id VARCHAR PRIMARY KEY,
      name VARCHAR NOT NULL,
      source VARCHAR NOT NULL,
      revision BIGINT NOT NULL CHECK (revision >= 0),
      created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
      updated_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp
    )
  `)
  await db.run(`
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
  await createLatestBuildTables(db)
}

async function applyMigration4(db: DuckDbExecutor): Promise<void> {
  if (!(await columnExists(db, 'mcp_builds', 'source_snapshot'))) {
    await db.run('ALTER TABLE mcp_builds ADD COLUMN source_snapshot VARCHAR')
  }
  await db.run(`
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
  const rows = await db.rows(`
    SELECT count(*) AS invalid_count
    FROM mcp_builds
    WHERE json_extract_string(execution_json, '$.manifestDigest') IS NULL
  `)
  if (safeInteger(rows[0] ?? {}, 'invalid_count') !== 0) {
    throw new TypeError('DuckDB migration found execution provenance with an unknown manifest')
  }
}

async function applyMigration5(db: DuckDbExecutor): Promise<void> {
  if (!(await columnExists(db, 'mcp_builds', 'source_attestation'))) {
    await db.run('ALTER TABLE mcp_builds ADD COLUMN source_attestation VARCHAR')
  }
  await db.run(`
    UPDATE mcp_builds
    SET source_attestation = CASE
      WHEN model_id IS NOT NULL AND model_revision IS NOT NULL THEN 'model-revision'
      WHEN source_snapshot IS NOT NULL THEN 'inline-snapshot'
      ELSE 'historical-unattested'
    END
    WHERE source_attestation IS NULL
  `)
}

async function applyMigration6(db: DuckDbExecutor): Promise<void> {
  if (!(await columnExists(db, 'mcp_builds', 'execution_json'))) return
  await db.run(`
    UPDATE mcp_builds
    SET execution_json = json_merge_patch(
      execution_json,
      '{"engineClass":"mesh"}'
    )
    WHERE json_extract_string(execution_json, '$.engineClass') = 'manifold'
  `)
  await db.run(`
    UPDATE mcp_builds
    SET execution_json = json_merge_patch(
      execution_json,
      '{"manifestDigest":"cc358238cabeb1d12e57458a8a94ddcdf53dadb3fd695bdbd360876e82cb9926"}'
    )
    WHERE json_extract_string(execution_json, '$.manifestDigest')
      = 'd58685b0fb5de314fbcc154d7d5d40128b1d9e71259a0f2a51452ca3986b5b74'
  `)
}

async function applyMigration7(db: DuckDbExecutor): Promise<void> {
  await db.run('DROP TABLE IF EXISTS mcp_artifacts_migration_v7')
  await db.run(`
    CREATE TABLE mcp_artifacts_migration_v7 (
      id VARCHAR,
      build_id VARCHAR,
      model_id VARCHAR,
      format VARCHAR,
      file_name VARCHAR,
      mime_type VARCHAR,
      sha256 VARCHAR,
      byte_length BIGINT,
      data BLOB,
      created_at TIMESTAMPTZ
    )
  `)
  await db.run(`
    INSERT INTO mcp_artifacts_migration_v7
    SELECT id, build_id, model_id, format, file_name, mime_type, sha256, byte_length, data, created_at
    FROM mcp_artifacts
  `)
  await db.run('DROP TABLE mcp_artifacts')
  await createArtifactsTable(db, {
    buildFk: await hasPrimaryKey(db, 'mcp_builds'),
    modelFk: await hasPrimaryKey(db, 'mcp_models'),
  })
  await db.run(`
    INSERT INTO mcp_artifacts (id, build_id, model_id, format, file_name, mime_type, sha256, byte_length, data, created_at)
    SELECT id, build_id, model_id, format, file_name, mime_type, sha256, byte_length, data, created_at
    FROM mcp_artifacts_migration_v7
  `)
  await db.run('DROP TABLE mcp_artifacts_migration_v7')
}

export async function runDuckDbMigrations(db: DuckDbMigrationContext): Promise<void> {
  await db.run(`
    CREATE TABLE IF NOT EXISTS mcp_schema_migrations (
      version INTEGER PRIMARY KEY,
      applied_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp
    )
  `)
  const versions = (
    await db.rows(`
    SELECT version FROM mcp_schema_migrations ORDER BY version
  `)
  ).map(row => safeInteger(row, 'version'))
  const current = versions.at(-1) ?? 0
  if (current > LATEST_SCHEMA_VERSION) {
    throw new Error(`DuckDB catalog schema ${current} is newer than supported schema ${LATEST_SCHEMA_VERSION}`)
  }
  if (versions.some((version, index) => version !== index + 1)) {
    throw new Error('DuckDB catalog has a non-contiguous migration history')
  }
  for (let version = current + 1; version <= LATEST_SCHEMA_VERSION; version++) {
    await db.transaction(async () => {
      if (version === 1) await applyMigration1(db)
      else if (version === 2) await applyMigration2(db, () => createLatestTables(db))
      else if (version === 3) await applyMigration3(db, () => createLatestBuildTables(db))
      else if (version === 4) await applyMigration4(db)
      else if (version === 5) await applyMigration5(db)
      else if (version === 6) await applyMigration6(db)
      else if (version === 7) await applyMigration7(db)
      await db.run(
        `
        INSERT INTO mcp_schema_migrations (version) VALUES ($version)
      `,
        { version },
      )
    })
  }
  await db.transaction(async () => {
    await pruneArtifacts(db, '')
    await pruneBuilds(db, '')
  })
}
