import { blobValue } from '@duckdb/node-api'
import { chmod, lstat, realpath, stat } from 'node:fs/promises'
import {
  MAX_STORED_ARTIFACT_BYTES,
  type StoreArtifactInput,
  type StoredArtifact,
  type StoredArtifactSummary,
  type StoredBuild,
  type StoredModel,
  type StoredModelRevision,
} from '../modelStore'
import type { DuckDbExecutor } from './types'
import { pruneArtifacts, pruneBuilds } from './quotasAndPruning'
import {
  artifactSummaryFromRow,
  buildFromRow,
  modelFromRow,
  modelRevisionFromRow,
  sha256,
} from './validateExecutionDescriptor'

export async function databaseKeys(path: string): Promise<string[]> {
  const keys = new Set<string>([`path:${path}`])
  try {
    const canonicalPath = await realpath(path)
    keys.add(`realpath:${canonicalPath}`)
    const metadata = await stat(canonicalPath)
    if (metadata.ino !== 0) keys.add(`inode:${metadata.dev}:${metadata.ino}`)
  } catch {
    // A second key pass after DuckDB initialization adds realpath/inode identities.
  }
  return [...keys]
}

export async function validateDatabaseTarget(path: string): Promise<void> {
  try {
    const metadata = await lstat(path)
    if (!metadata.isFile() || metadata.isSymbolicLink()) {
      throw new TypeError(`DuckDB path must be a regular file: ${path}`)
    }
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error
  }
}

export async function hardenDatabasePermissions(databasePath: string | null): Promise<void> {
  if (!databasePath || process.platform === 'win32') return
  for (const path of [databasePath, `${databasePath}.wal`]) {
    try {
      await chmod(path, 0o600)
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error
    }
  }
}

export async function getModelDirect(db: DuckDbExecutor, id: string): Promise<StoredModel | null> {
  const rows = await db.rows('SELECT * FROM mcp_models WHERE id = $id', { id })
  return rows[0] ? modelFromRow(rows[0]) : null
}

export async function getModelRevisionDirect(
  db: DuckDbExecutor,
  id: string,
  revision: number,
): Promise<StoredModelRevision | null> {
  const rows = await db.rows(
    'SELECT model_id AS id, name, source, revision, created_at FROM mcp_model_revisions WHERE model_id = $id AND revision = $revision',
    { id, revision },
  )
  return rows[0] ? modelRevisionFromRow(rows[0]) : null
}

export async function getBuildDirect(db: DuckDbExecutor, id: string): Promise<StoredBuild | null> {
  const rows = await db.rows(
    'SELECT build.*, snapshot.source AS revision_source FROM mcp_builds AS build LEFT JOIN mcp_model_revisions AS snapshot ON snapshot.model_id = build.model_id AND snapshot.revision = build.model_revision WHERE build.id = $id',
    { id },
  )
  return rows[0] ? buildFromRow(rows[0]) : null
}

export async function getArtifactDirect(
  db: DuckDbExecutor,
  id: string,
  withData: boolean,
): Promise<StoredArtifactSummary | StoredArtifact | null> {
  const rows = await db.rows(
    withData
      ? 'SELECT * FROM mcp_artifacts WHERE id = $id'
      : 'SELECT id, build_id, model_id, format, file_name, mime_type, sha256, byte_length, created_at FROM mcp_artifacts WHERE id = $id',
    { id },
  )
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

export async function listBuildsDirect(
  db: DuckDbExecutor,
  modelId: string | undefined,
  limit: number,
): Promise<StoredBuild[]> {
  const rows = modelId
    ? await db.rows(
        'SELECT build.*, snapshot.source AS revision_source FROM mcp_builds AS build LEFT JOIN mcp_model_revisions AS snapshot ON snapshot.model_id = build.model_id AND snapshot.revision = build.model_revision WHERE build.model_id = $model_id ORDER BY build.created_at DESC, build.id LIMIT $limit',
        { model_id: modelId, limit },
      )
    : await db.rows(
        'SELECT build.*, snapshot.source AS revision_source FROM mcp_builds AS build LEFT JOIN mcp_model_revisions AS snapshot ON snapshot.model_id = build.model_id AND snapshot.revision = build.model_revision ORDER BY build.created_at DESC, build.id LIMIT $limit',
        { limit },
      )
  return rows.map(buildFromRow)
}

export async function insertArtifactAndPrune(
  db: DuckDbExecutor,
  id: string,
  input: StoreArtifactInput,
): Promise<StoredArtifactSummary> {
  const build = await getBuildDirect(db, input.buildId)
  if (!build) throw new TypeError(`Build ${input.buildId} does not exist`)
  if (input.modelId !== undefined && input.modelId !== build.modelId) {
    throw new TypeError('Artifact modelId must match its build')
  }
  await db.run(
    `
      INSERT INTO mcp_artifacts (id, build_id, model_id, format, file_name, mime_type, sha256, byte_length, data)
      VALUES ($id, $build_id, $model_id, $format, $file_name, $mime_type, $sha256, $byte_length, $data)
    `,
    {
      id,
      build_id: input.buildId,
      model_id: build.modelId,
      format: input.format,
      file_name: input.fileName,
      mime_type: input.mimeType,
      sha256: input.sha256,
      byte_length: input.data.byteLength,
      data: blobValue(input.data),
    },
  )
  await pruneArtifacts(db, id)
  await pruneBuilds(db, input.buildId)
  const artifact = (await getArtifactDirect(db, id, false)) as StoredArtifactSummary | null
  if (!artifact) throw new Error(`DuckDB failed to store artifact ${id}`)
  return artifact
}

export async function listArtifactsDirect(db: DuckDbExecutor, limit: number): Promise<StoredArtifactSummary[]> {
  const rows = await db.rows(
    'SELECT id, build_id, model_id, format, file_name, mime_type, sha256, byte_length, created_at FROM mcp_artifacts ORDER BY created_at DESC, id LIMIT $limit',
    { limit },
  )
  return rows.map(artifactSummaryFromRow)
}

export async function deleteModelDirect(db: DuckDbExecutor, id: string): Promise<boolean> {
  const existing = await getModelDirect(db, id)
  if (!existing) return false
  // DuckDB ART foreign-key validation cannot see uncommitted child deletes in the
  // same transaction, so callers serialize these bottom-up auto-commit steps.
  await db.run(
    'DELETE FROM mcp_artifacts WHERE model_id = $id OR build_id IN (SELECT id FROM mcp_builds WHERE model_id = $id)',
    { id },
  )
  await db.run('DELETE FROM mcp_builds WHERE model_id = $id', { id })
  await db.run('DELETE FROM mcp_model_revisions WHERE model_id = $id', { id })
  await db.run('DELETE FROM mcp_models WHERE id = $id', { id })
  return true
}
