import type { WorkspaceStorage, WorkspaceDocumentSnapshot, WorkspaceRecoveryRecord } from '../workspaceDocument'

export interface RemovableWorkspaceStorage extends WorkspaceStorage {
  removeItem(key: string): void
  keys(): string[]
}

export type WorkspacePersistenceBackend = 'indexeddb' | 'localstorage'

export interface WorkspaceBootstrapOptions {
  fallbackSource: string
  importedSource?: string | null
  importedFileName?: string
}

export interface WorkspaceBootstrapResult {
  document: WorkspaceDocumentSnapshot
  backend: WorkspacePersistenceBackend
  durable: boolean
  imported: boolean
}

export interface WorkspaceRetryResult {
  saved: boolean
  /** A previously unreadable IndexedDB head recovered by an explicit retry. */
  restoredDocument?: WorkspaceDocumentSnapshot
}

export interface StoredRecovery {
  key: string
  record: WorkspaceRecoveryRecord
}
