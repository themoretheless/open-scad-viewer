import type { DuckDBConnection, JS } from '@duckdb/node-api'

export type Row = Record<string, JS>

export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue }

export const MAX_LIST_LIMIT = 500
export const MAX_STORED_ARTIFACT_COUNT = 100
export const MAX_STORED_ARTIFACT_TOTAL_BYTES = 64 * 1024 * 1024
export const MAX_STORED_BUILD_COUNT = 500
export const MAX_STORED_MODEL_COUNT = 500
export const MAX_STORED_MODEL_SOURCE_BYTES = 64 * 1024 * 1024
export const MAX_STORED_MODEL_REVISION_COUNT = 5_000
export const MAX_STORED_REVISIONS_PER_MODEL = 256

export type SqlParams = Parameters<DuckDBConnection['run']>[1]

export interface DuckDbExecutor {
  run: (sql: string, values?: SqlParams) => Promise<unknown>
  rows: (sql: string, values?: SqlParams) => Promise<Row[]>
}
