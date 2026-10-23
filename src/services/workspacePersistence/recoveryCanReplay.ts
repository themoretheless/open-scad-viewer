import type { StoredRecovery } from '../workspacePersistence'
import { workspaceDocumentsEqual } from '../workspaceDocument'
import type { WorkspaceDocumentSnapshot, WorkspaceRecoveryRecord } from '../workspaceDocument'

export function recoveryCanReplay(
  indexed: WorkspaceDocumentSnapshot | null,
  recovery: WorkspaceRecoveryRecord | null,
): boolean {
  if (!recovery) return false
  if (workspaceDocumentsEqual(indexed, recovery.snapshot)) return true
  // A legacy journal has no recorded base and may seed an empty database. A
  // journal that explicitly branched from a now-missing head is ambiguous and
  // must not be replayed as if that head had never existed.
  if (!indexed) return recovery.base.status !== 'found'
  if (recovery.base.status !== 'found'
    || !workspaceDocumentsEqual(indexed, recovery.base.snapshot)) return false
  const base = recovery.base.snapshot
  return recovery.snapshot.documentId === base.documentId
    && recovery.snapshot.mutation > base.mutation
    && recovery.snapshot.revision >= base.revision
}

export function fallbackWriterId(): string {
  const cryptoApi = globalThis.crypto
  if (cryptoApi && typeof cryptoApi.randomUUID === 'function') return cryptoApi.randomUUID()
  return `writer-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
}

export function distinctRecoveries(entries: StoredRecovery[]): StoredRecovery[] {
  const distinct: StoredRecovery[] = []
  for (const entry of entries) {
    if (!distinct.some(candidate => workspaceDocumentsEqual(candidate.record.snapshot, entry.record.snapshot))) {
      distinct.push(entry)
    }
  }
  return distinct
}

export function newestRecovery(entries: StoredRecovery[]): StoredRecovery | null {
  return [...entries].sort((left, right) =>
    right.record.snapshot.updatedAt - left.record.snapshot.updatedAt
    || right.record.snapshot.mutation - left.record.snapshot.mutation
    || right.key.localeCompare(left.key))[0] ?? null
}
