export const WORKSPACE_SCHEMA_VERSION = 3 as const

type WorkspaceRecordV1 = Record<string, unknown> & { schemaVersion: 1 }
type WorkspaceRecordV2 = Record<string, unknown> & { schemaVersion: 2; parameterPresets: unknown }
type WorkspaceRecordV3 = Record<string, unknown> & {
  schemaVersion: 3
  parameterPresets: unknown
  projectFiles: unknown
  libraryPaths: unknown
}

/**
 * Forward-only migrations, one per schema step. Each step is total over the
 * previous version's record shape; validation runs once on the final shape.
 */
export const WORKSPACE_MIGRATIONS = Object.freeze({
  1: (record: WorkspaceRecordV1): WorkspaceRecordV2 => ({ ...record, schemaVersion: 2, parameterPresets: [] }),
  2: (record: WorkspaceRecordV2): WorkspaceRecordV3 => ({
    ...record,
    schemaVersion: 3,
    projectFiles: [],
    libraryPaths: [],
  }),
})

/** Migrate a persisted record to the current schema, or `null` for unknown or future versions. */
export function migrateWorkspaceRecord(value: unknown): WorkspaceRecordV3 | null {
  if (!value || typeof value !== 'object') return null
  let record = value as Record<string, unknown>
  if (record.schemaVersion === 1) record = WORKSPACE_MIGRATIONS[1](record as WorkspaceRecordV1)
  if (record.schemaVersion === 2) record = WORKSPACE_MIGRATIONS[2](record as WorkspaceRecordV2)
  return record.schemaVersion === WORKSPACE_SCHEMA_VERSION ? (record as WorkspaceRecordV3) : null
}
