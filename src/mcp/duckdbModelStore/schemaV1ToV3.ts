import { requiredString, safeInteger } from './boundedLimit'
import type { DuckDbExecutor } from './types'

export async function tableExists(db: DuckDbExecutor, table: string): Promise<boolean> {
  const rows = await db.rows(
    `
    SELECT count(*) AS table_count
    FROM information_schema.tables
    WHERE table_schema = current_schema() AND table_name = $table
  `,
    { table },
  )
  return safeInteger(rows[0], 'table_count') > 0
}

export async function columnExists(db: DuckDbExecutor, table: string, column: string): Promise<boolean> {
  const rows = await db.rows(
    `
    SELECT count(*) AS column_count
    FROM information_schema.columns
    WHERE table_schema = current_schema()
      AND table_name = $table
      AND column_name = $column
  `,
    { table, column },
  )
  return safeInteger(rows[0], 'column_count') > 0
}

export async function columnIsNullable(db: DuckDbExecutor, table: string, column: string): Promise<boolean> {
  const rows = await db.rows(
    `
    SELECT is_nullable
    FROM information_schema.columns
    WHERE table_schema = current_schema()
      AND table_name = $table
      AND column_name = $column
  `,
    { table, column },
  )
  if (!rows[0]) throw new Error(`DuckDB catalog has no ${table}.${column} column`)
  return requiredString(rows[0], 'is_nullable') === 'YES'
}

export async function timestampExpression(
  db: DuckDbExecutor,
  table: string,
  column: string,
  reference: string,
): Promise<string> {
  const rows = await db.rows(
    `
    SELECT data_type
    FROM information_schema.columns
    WHERE table_schema = current_schema() AND table_name = $table AND column_name = $column
  `,
    { table, column },
  )
  const type = rows[0] ? requiredString(rows[0], 'data_type') : null
  if (type === 'TIMESTAMP WITH TIME ZONE') return reference
  if (type?.startsWith('TIMESTAMP')) return `timezone(current_setting('TimeZone'), ${reference})`
  throw new Error(`DuckDB catalog has no compatible ${table}.${column} timestamp column`)
}

/** Legacy baseline retained so previously created development catalogs have a real upgrade path. */
export async function applyMigration1(db: DuckDbExecutor): Promise<void> {
  await db.run(`
    CREATE TABLE IF NOT EXISTS mcp_models (
      id VARCHAR PRIMARY KEY,
      name VARCHAR NOT NULL,
      source VARCHAR NOT NULL,
      revision BIGINT NOT NULL CHECK (revision >= 0),
      created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
      updated_at TIMESTAMP NOT NULL DEFAULT current_timestamp
    )
  `)
  await db.run(`
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
  await db.run(`
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

export async function applyMigration2(db: DuckDbExecutor, createLatestTables: () => Promise<void>): Promise<void> {
  const migrationTimestamp = await timestampExpression(db, 'mcp_schema_migrations', 'applied_at', 'applied_at')
  if (migrationTimestamp !== 'applied_at') {
    await db.run(`
      ALTER TABLE mcp_schema_migrations ALTER applied_at SET DATA TYPE TIMESTAMPTZ
      USING ${migrationTimestamp}
    `)
  }
  const modelCreated = await timestampExpression(db, 'mcp_models', 'created_at', 'model.created_at')
  const modelUpdated = await timestampExpression(db, 'mcp_models', 'updated_at', 'model.updated_at')
  const buildCreated = await timestampExpression(db, 'mcp_builds', 'created_at', 'build.created_at')
  const artifactCreated = await timestampExpression(db, 'mcp_artifacts', 'created_at', 'artifact.created_at')
  const hasRevisions = await tableExists(db, 'mcp_model_revisions')
  const revisionCreated = hasRevisions
    ? await timestampExpression(db, 'mcp_model_revisions', 'created_at', 'snapshot.created_at')
    : 'model.updated_at'

  for (const table of [
    'mcp_artifacts_migration_v2',
    'mcp_builds_migration_v2',
    'mcp_model_revisions_migration_v2',
    'mcp_models_migration_v2',
  ]) {
    await db.run(`DROP TABLE IF EXISTS ${table}`)
  }
  await db.run(
    'CREATE TABLE mcp_models_migration_v2 (id VARCHAR, name VARCHAR, source VARCHAR, revision BIGINT, created_at TIMESTAMPTZ, updated_at TIMESTAMPTZ)',
  )
  await db.run(
    `INSERT INTO mcp_models_migration_v2 SELECT model.id, model.name, model.source, model.revision, ${modelCreated}, ${modelUpdated} FROM mcp_models AS model`,
  )
  await db.run(
    'CREATE TABLE mcp_model_revisions_migration_v2 (model_id VARCHAR, revision BIGINT, name VARCHAR, source VARCHAR, created_at TIMESTAMPTZ)',
  )
  if (hasRevisions) {
    await db.run(
      `INSERT INTO mcp_model_revisions_migration_v2 SELECT snapshot.model_id, snapshot.revision, snapshot.name, snapshot.source, ${revisionCreated} FROM mcp_model_revisions AS snapshot`,
    )
  }
  await db.run(`
    INSERT INTO mcp_model_revisions_migration_v2
    SELECT model.id, model.revision, model.name, model.source, model.updated_at
    FROM mcp_models_migration_v2 AS model
    WHERE NOT EXISTS (SELECT 1 FROM mcp_model_revisions_migration_v2 AS snapshot WHERE snapshot.model_id = model.id AND snapshot.revision = model.revision)
  `)
  await db.run(`
    CREATE TABLE mcp_builds_migration_v2 (
      id VARCHAR, model_id VARCHAR, model_revision BIGINT, source_sha256 VARCHAR,
      source_snapshot VARCHAR, source_attestation VARCHAR,
      quality VARCHAR, status VARCHAR, duration_ms DOUBLE, warnings_json JSON,
      mesh_count BIGINT, triangle_count BIGINT, volume DOUBLE, surface_area DOUBLE,
      reduced BOOLEAN, error_json JSON, execution_json JSON, created_at TIMESTAMPTZ
    )
  `)
  await db.run(`
    INSERT INTO mcp_builds_migration_v2
    SELECT build.id,
           CASE WHEN snapshot.model_id IS NULL OR lower(build.source_sha256) <> sha256(snapshot.source) THEN NULL ELSE build.model_id END,
           CASE WHEN snapshot.model_id IS NULL OR lower(build.source_sha256) <> sha256(snapshot.source) THEN NULL ELSE build.model_revision END,
           build.source_sha256, NULL,
           CASE WHEN snapshot.model_id IS NOT NULL AND lower(build.source_sha256) = sha256(snapshot.source) THEN 'model-revision' ELSE 'historical-unattested' END,
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
  await db.run(`
    CREATE TABLE mcp_artifacts_migration_v2 (
      id VARCHAR, build_id VARCHAR, model_id VARCHAR, format VARCHAR,
      file_name VARCHAR, mime_type VARCHAR, sha256 VARCHAR, byte_length BIGINT,
      data BLOB, created_at TIMESTAMPTZ
    )
  `)
  await db.run(`
    INSERT INTO mcp_artifacts_migration_v2
    SELECT artifact.id, artifact.build_id, build.model_id, artifact.format,
           artifact.file_name, artifact.mime_type, artifact.sha256,
           artifact.byte_length, artifact.data, ${artifactCreated}
    FROM mcp_artifacts AS artifact
    JOIN mcp_builds_migration_v2 AS build ON build.id = artifact.build_id
  `)
  await db.run('DROP TABLE mcp_artifacts')
  await db.run('DROP TABLE mcp_builds')
  if (hasRevisions) await db.run('DROP TABLE mcp_model_revisions')
  await db.run('DROP TABLE mcp_models')
  await createLatestTables()
  await db.run('INSERT INTO mcp_models SELECT * FROM mcp_models_migration_v2')
  await db.run('INSERT INTO mcp_model_revisions SELECT * FROM mcp_model_revisions_migration_v2')
  await db.run('INSERT INTO mcp_builds SELECT * FROM mcp_builds_migration_v2')
  await db.run('INSERT INTO mcp_artifacts SELECT * FROM mcp_artifacts_migration_v2')
  await db.run('DROP TABLE mcp_artifacts_migration_v2')
  await db.run('DROP TABLE mcp_builds_migration_v2')
  await db.run('DROP TABLE mcp_model_revisions_migration_v2')
  await db.run('DROP TABLE mcp_models_migration_v2')
}

export async function applyMigration3(db: DuckDbExecutor, createLatestBuildTables: () => Promise<void>): Promise<void> {
  const hasExecution = await columnExists(db, 'mcp_builds', 'execution_json')
  if (hasExecution && !(await columnIsNullable(db, 'mcp_builds', 'execution_json'))) return

  for (const table of ['mcp_artifacts_migration_v3', 'mcp_builds_migration_v3']) {
    await db.run(`DROP TABLE IF EXISTS ${table}`)
  }
  await db.run(`
    CREATE TABLE mcp_builds_migration_v3 (
      id VARCHAR, model_id VARCHAR, model_revision BIGINT, source_sha256 VARCHAR,
      source_snapshot VARCHAR, source_attestation VARCHAR,
      quality VARCHAR, status VARCHAR, duration_ms DOUBLE, warnings_json JSON,
      mesh_count BIGINT, triangle_count BIGINT, volume DOUBLE, surface_area DOUBLE,
      reduced BOOLEAN, error_json JSON, execution_json JSON, created_at TIMESTAMPTZ
    )
  `)
  const existingExecution = hasExecution ? 'build.execution_json' : 'NULL'
  await db.run(`
    INSERT INTO mcp_builds_migration_v3
    SELECT build.id,
           CASE WHEN (snapshot.model_id IS NULL AND build.model_id IS NOT NULL)
                      OR (snapshot.model_id IS NOT NULL AND lower(build.source_sha256) <> sha256(snapshot.source))
                THEN NULL ELSE build.model_id END,
           CASE WHEN (snapshot.model_id IS NULL AND build.model_revision IS NOT NULL)
                      OR (snapshot.model_id IS NOT NULL AND lower(build.source_sha256) <> sha256(snapshot.source))
                THEN NULL ELSE build.model_revision END,
           build.source_sha256, NULL,
           CASE WHEN snapshot.model_id IS NOT NULL AND lower(build.source_sha256) = sha256(snapshot.source)
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
  await db.run(`
    CREATE TABLE mcp_artifacts_migration_v3 (
      id VARCHAR, build_id VARCHAR, model_id VARCHAR, format VARCHAR,
      file_name VARCHAR, mime_type VARCHAR, sha256 VARCHAR, byte_length BIGINT,
      data BLOB, created_at TIMESTAMPTZ
    )
  `)
  await db.run(`
    INSERT INTO mcp_artifacts_migration_v3
    SELECT id, build_id, model_id, format, file_name, mime_type, sha256, byte_length, data, created_at
    FROM mcp_artifacts
  `)
  await db.run('DROP TABLE mcp_artifacts')
  await db.run('DROP TABLE mcp_builds')
  await createLatestBuildTables()
  await db.run('INSERT INTO mcp_builds SELECT * FROM mcp_builds_migration_v3')
  await db.run('INSERT INTO mcp_artifacts SELECT * FROM mcp_artifacts_migration_v3')
  await db.run('DROP TABLE mcp_artifacts_migration_v3')
  await db.run('DROP TABLE mcp_builds_migration_v3')
}
