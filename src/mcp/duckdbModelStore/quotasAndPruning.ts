import { CatalogQuotaError, type CatalogStats } from '../modelStore'
import { safeInteger } from './boundedLimit'
import {
  MAX_STORED_ARTIFACT_COUNT,
  MAX_STORED_ARTIFACT_TOTAL_BYTES,
  MAX_STORED_BUILD_COUNT,
  MAX_STORED_MODEL_COUNT,
  MAX_STORED_MODEL_REVISION_COUNT,
  MAX_STORED_MODEL_SOURCE_BYTES,
  MAX_STORED_REVISIONS_PER_MODEL,
  type DuckDbExecutor,
} from './types'

export async function ensureSourceQuota(db: DuckDbExecutor, source: string): Promise<void> {
  const rows = await db.rows(`
    SELECT
      (SELECT coalesce(sum(strlen(source)), 0) FROM mcp_model_revisions)
      + (SELECT coalesce(sum(strlen(source_snapshot)), 0) FROM mcp_builds) AS source_bytes
  `)
  const storedBytes = safeInteger(rows[0], 'source_bytes')
  const additionalBytes = Buffer.byteLength(source, 'utf8')
  if (storedBytes + additionalBytes > MAX_STORED_MODEL_SOURCE_BYTES) {
    throw new CatalogQuotaError(
      `Stored immutable source snapshots are limited to ${MAX_STORED_MODEL_SOURCE_BYTES.toLocaleString('en-US')} UTF-8 bytes`,
      'source_bytes',
      MAX_STORED_MODEL_SOURCE_BYTES,
    )
  }
}

export async function ensureRevisionQuota(db: DuckDbExecutor, modelId: string): Promise<void> {
  const rows = await db.rows(
    `
    SELECT count(*) AS total_count,
           count(*) FILTER (WHERE model_id = $model_id) AS model_count
    FROM mcp_model_revisions
  `,
    { model_id: modelId },
  )
  if (safeInteger(rows[0], 'total_count') >= MAX_STORED_MODEL_REVISION_COUNT) {
    throw new CatalogQuotaError(
      `Model catalog is limited to ${MAX_STORED_MODEL_REVISION_COUNT.toLocaleString('en-US')} source revisions`,
      'revisions',
      MAX_STORED_MODEL_REVISION_COUNT,
    )
  }
  if (safeInteger(rows[0], 'model_count') >= MAX_STORED_REVISIONS_PER_MODEL) {
    throw new CatalogQuotaError(
      `Model ${modelId} is limited to ${MAX_STORED_REVISIONS_PER_MODEL.toLocaleString('en-US')} source revisions`,
      'revisions_per_model',
      MAX_STORED_REVISIONS_PER_MODEL,
    )
  }
}

export async function pruneBuilds(db: DuckDbExecutor, keepId: string): Promise<void> {
  await db.run(
    `
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
  `,
    { keep_id: keepId, max_count: MAX_STORED_BUILD_COUNT },
  )
}

export async function pruneArtifacts(db: DuckDbExecutor, keepId: string): Promise<void> {
  await db.run(
    `
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
  `,
    {
      keep_id: keepId,
      max_count: MAX_STORED_ARTIFACT_COUNT,
      max_bytes: MAX_STORED_ARTIFACT_TOTAL_BYTES,
    },
  )
}

export async function queryCatalogStats(db: DuckDbExecutor): Promise<CatalogStats> {
  const rows = await db.rows(`
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
}
