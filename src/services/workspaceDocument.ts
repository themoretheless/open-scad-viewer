/**
 * Versioned persistence contract for the local single-document workspace.
 *
 * Keeping the persisted payload behind this small boundary lets the UI evolve
 * to a multi-file workspace without coupling migrations to Vue components.
 */

export const WORKSPACE_SCHEMA_VERSION = 1 as const
export const WORKSPACE_STORAGE_KEY = 'open-scad-viewer.workspace'

export interface WorkspaceDocumentSnapshot {
  readonly schemaVersion: typeof WORKSPACE_SCHEMA_VERSION
  readonly documentId: string
  readonly fileName: string
  readonly source: string
  readonly revision: number
  readonly updatedAt: number
}

export interface WorkspaceStorage {
  getItem(key: string): string | null
  setItem(key: string, value: string): void
}

export interface CreateWorkspaceDocumentOptions {
  documentId?: string
  fileName?: string
  revision?: number
  updatedAt?: number
}

const MAX_SOURCE_LENGTH = 250_000
const MAX_FILE_NAME_LENGTH = 255

function finiteRevision(value: unknown): number | null {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0 ? value : null
}

function validDocumentId(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= 128
}

function validFileName(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= MAX_FILE_NAME_LENGTH
}

function fallbackDocumentId(): string {
  const cryptoApi = globalThis.crypto
  if (cryptoApi && typeof cryptoApi.randomUUID === 'function') return cryptoApi.randomUUID()
  return `document-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
}

export function createWorkspaceDocument(
  source: string,
  options: CreateWorkspaceDocumentOptions = {},
): WorkspaceDocumentSnapshot {
  return {
    schemaVersion: WORKSPACE_SCHEMA_VERSION,
    documentId: options.documentId ?? fallbackDocumentId(),
    fileName: options.fileName ?? 'model.scad',
    source,
    revision: options.revision ?? 0,
    updatedAt: options.updatedAt ?? Date.now(),
  }
}

/**
 * Advance the geometry revision only when source changed. Metadata edits still
 * update the durable snapshot, but must not invalidate an in-flight build of
 * the identical source.
 */
export function updateWorkspaceDocument(
  previous: WorkspaceDocumentSnapshot,
  update: { source?: string; fileName?: string },
  updatedAt = Date.now(),
): WorkspaceDocumentSnapshot {
  const source = update.source ?? previous.source
  const fileName = update.fileName ?? previous.fileName
  if (source === previous.source && fileName === previous.fileName) return previous
  return {
    ...previous,
    source,
    fileName,
    revision: previous.revision + (source === previous.source ? 0 : 1),
    updatedAt,
  }
}

export function parseWorkspaceDocument(serialized: string): WorkspaceDocumentSnapshot | null {
  try {
    const value: unknown = JSON.parse(serialized)
    if (!value || typeof value !== 'object') return null
    const candidate = value as Partial<Record<keyof WorkspaceDocumentSnapshot, unknown>>
    if (candidate.schemaVersion !== WORKSPACE_SCHEMA_VERSION
      || !validDocumentId(candidate.documentId)
      || !validFileName(candidate.fileName)
      || typeof candidate.source !== 'string'
      || candidate.source.length > MAX_SOURCE_LENGTH) return null
    const revision = finiteRevision(candidate.revision)
    const updatedAt = finiteRevision(candidate.updatedAt)
    if (revision === null || updatedAt === null) return null
    return {
      schemaVersion: WORKSPACE_SCHEMA_VERSION,
      documentId: candidate.documentId,
      fileName: candidate.fileName,
      source: candidate.source,
      revision,
      updatedAt,
    }
  } catch {
    return null
  }
}

export function loadWorkspaceDocument(
  storage: WorkspaceStorage,
  fallbackSource: string,
  legacySourceKey = 'scad-code',
): WorkspaceDocumentSnapshot {
  try {
    const serialized = storage.getItem(WORKSPACE_STORAGE_KEY)
    const stored = serialized ? parseWorkspaceDocument(serialized) : null
    if (stored) return stored
    const legacySource = storage.getItem(legacySourceKey)
    return createWorkspaceDocument(legacySource ?? fallbackSource)
  } catch {
    return createWorkspaceDocument(fallbackSource)
  }
}

export function saveWorkspaceDocument(storage: WorkspaceStorage, document: WorkspaceDocumentSnapshot): boolean {
  try {
    storage.setItem(WORKSPACE_STORAGE_KEY, JSON.stringify(document))
    return true
  } catch {
    return false
  }
}
