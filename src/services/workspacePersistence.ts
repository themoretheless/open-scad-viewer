import {
  WORKSPACE_LEGACY_SOURCE_KEY,
  WORKSPACE_LEGACY_TABS_BACKUP_KEY,
  WORKSPACE_LEGACY_TABS_KEY,
  WORKSPACE_RECOVERY_KEY_PREFIX,
  WORKSPACE_STORAGE_KEY,
  createWorkspaceDocument,
  loadLegacyWorkspaceDocument,
  loadWorkspaceRecovery,
  parseWorkspaceDocumentValue,
  saveWorkspaceRecovery,
  workspaceDocumentAdvances,
  workspaceDocumentsEqual,
  type WorkspaceDocumentSnapshot,
  type WorkspaceRecoveryBase,
  type WorkspaceRecoveryRecord,
  type WorkspaceStorage,
} from './workspaceDocument'
import {
  WorkspaceIndexedDbConflictError,
  WorkspaceIndexedDbInvalidDataError,
  type WorkspaceRepositoryLoadResult,
  type WorkspaceSnapshotRepository,
} from './workspaceIndexedDb'

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

type IndexedDbState = 'ready' | 'unavailable' | 'invalid'

function recoveryCanReplay(
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

function fallbackWriterId(): string {
  const cryptoApi = globalThis.crypto
  if (cryptoApi && typeof cryptoApi.randomUUID === 'function') return cryptoApi.randomUUID()
  return `writer-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
}

interface StoredRecovery {
  key: string
  record: WorkspaceRecoveryRecord
}

function distinctRecoveries(entries: StoredRecovery[]): StoredRecovery[] {
  const distinct: StoredRecovery[] = []
  for (const entry of entries) {
    if (!distinct.some(candidate => workspaceDocumentsEqual(candidate.record.snapshot, entry.record.snapshot))) {
      distinct.push(entry)
    }
  }
  return distinct
}

function newestRecovery(entries: StoredRecovery[]): StoredRecovery | null {
  return [...entries].sort((left, right) =>
    right.record.snapshot.updatedAt - left.record.snapshot.updatedAt
    || right.record.snapshot.mutation - left.record.snapshot.mutation
    || right.key.localeCompare(left.key))[0] ?? null
}

/**
 * Coordinates primary IndexedDB persistence with a synchronous localStorage
 * recovery journal. IndexedDB commits are compare-and-swap writes and are
 * serialized within this tab; the journal protects edits during page freeze,
 * disabled IDB, quota failures, and browser shutdown before an async commit.
 */
export class BrowserWorkspacePersistence {
  private backendValue: WorkspacePersistenceBackend = 'indexeddb'
  private writeTail: Promise<void> = Promise.resolve()
  private initialized = false
  private indexedDbState: IndexedDbState = 'ready'
  private durableHead: WorkspaceDocumentSnapshot | null = null
  private durableHeadKnown = false
  private recoveryBase: WorkspaceRecoveryBase = { status: 'unknown' }
  private initialDocument: WorkspaceDocumentSnapshot | null = null
  private initialDocumentAuthoritative = false
  private importedAuthoritative = false
  private unresolvedConflict = false
  private conflictingRecovery: WorkspaceDocumentSnapshot | null = null
  private latestSaveRequest: WorkspaceDocumentSnapshot | null = null
  private readonly writerId = fallbackWriterId()
  private readonly recoveryKey = `${WORKSPACE_RECOVERY_KEY_PREFIX}${this.writerId}`

  constructor(
    private readonly repository: WorkspaceSnapshotRepository,
    private readonly legacyStorage: RemovableWorkspaceStorage,
  ) {}

  get backend(): WorkspacePersistenceBackend { return this.backendValue }
  get hasConflict(): boolean { return this.unresolvedConflict }

  async initialize(options: WorkspaceBootstrapOptions): Promise<WorkspaceBootstrapResult> {
    if (this.initialized) throw new Error('Workspace persistence is already initialized')
    this.initialized = true
    const recoveryEntries = distinctRecoveries(this.loadRecoveryEntries())
    const selectedRecovery = newestRecovery(recoveryEntries)
    const recoveryRecord = selectedRecovery?.record ?? null
    const recovery = recoveryRecord?.snapshot ?? null
    this.recoveryBase = recoveryRecord?.base ?? { status: 'unknown' }
    const rawLegacy = loadLegacyWorkspaceDocument(this.legacyStorage)
    const importedCandidate = options.importedSource === null || options.importedSource === undefined
      ? null
      : createWorkspaceDocument(options.importedSource, {
          fileName: options.importedFileName ?? 'shared-model.scad',
        })
    const imported = importedCandidate ? parseWorkspaceDocumentValue(importedCandidate) : null

    let loadResult: WorkspaceRepositoryLoadResult | null = null
    try {
      loadResult = await this.repository.load()
      this.indexedDbState = loadResult.status === 'invalid' ? 'invalid' : 'ready'
      this.durableHeadKnown = loadResult.status !== 'invalid'
    } catch {
      this.indexedDbState = 'unavailable'
      this.backendValue = 'localstorage'
    }

    const indexed = loadResult?.status === 'found' ? loadResult.snapshot : null
    this.durableHead = indexed
    // Raw `scad-code` has no identity or ordering metadata. It can seed an
    // empty/unavailable workspace, but can never overwrite a known IDB head.
    const nonMatchingRecoveries = recoveryEntries.filter(entry =>
      !workspaceDocumentsEqual(indexed, entry.record.snapshot))
    const replayableRecoveries = this.indexedDbState === 'ready'
      ? nonMatchingRecoveries.filter(entry => recoveryCanReplay(indexed, entry.record))
      : []
    const localCandidate = newestRecovery(nonMatchingRecoveries)?.record.snapshot ?? recovery
    const existingLocalDocument = indexed ?? localCandidate ?? rawLegacy
    const importConflict = imported !== null
      && existingLocalDocument !== null
      && !workspaceDocumentsEqual(imported, existingLocalDocument)
    const recoveryConflict = importConflict || (imported === null && (
      nonMatchingRecoveries.length > 1
      || (this.indexedDbState === 'ready'
        && nonMatchingRecoveries.length === 1
        && replayableRecoveries.length !== 1)
    ))
    const migration = localCandidate ?? (indexed ? null : rawLegacy)
    const document = imported
      ?? (recoveryConflict ? localCandidate : replayableRecoveries[0]?.record.snapshot)
      ?? indexed
      ?? migration
      ?? createWorkspaceDocument(options.fallbackSource)
    const initialDocumentAuthoritative = imported !== null || migration !== null || indexed !== null
    this.initialDocument = document
    this.initialDocumentAuthoritative = initialDocumentAuthoritative
    this.importedAuthoritative = imported !== null
    this.unresolvedConflict = recoveryConflict
    this.conflictingRecovery = recoveryConflict ? document : null

    if (this.indexedDbState === 'ready') {
      const journalSaved = recoveryConflict
        ? true
        : this.stageRecovery(document)
      const needsCommit = imported !== null || !workspaceDocumentsEqual(indexed, document)
      try {
        let committed = !needsCommit
        if (needsCommit && !this.unresolvedConflict) {
          await this.repository.save(document, indexed)
          committed = true
        }
        if (committed) {
          this.durableHead = document
          this.durableHeadKnown = true
        }
        this.backendValue = committed ? 'indexeddb' : 'localstorage'
        if (committed) {
          this.retireRawLegacy()
          this.retireResolvedRecoveries(indexed, document)
        }
        // An existing IDB head is durable even if mirroring its recovery copy
        // hit localStorage quota. A newly committed head is durable as well.
        return {
          document,
          backend: this.backendValue,
          durable: committed && !this.unresolvedConflict,
          imported: imported !== null,
        }
      } catch (error) {
        this.recordIndexedDbFailure(error)
        this.backendValue = 'localstorage'
        return {
          document,
          backend: 'localstorage',
          durable: error instanceof WorkspaceIndexedDbConflictError
            ? false
            : journalSaved || (indexed !== null && workspaceDocumentsEqual(indexed, document)),
          imported: imported !== null,
        }
      }
    }

    this.backendValue = 'localstorage'
    // Do not turn a generated example into an authoritative, newer recovery
    // after an uncertain/invalid IDB read. That exact sequence could overwrite
    // a real IDB document on the next healthy launch.
    const durable = !this.unresolvedConflict && initialDocumentAuthoritative
      ? (workspaceDocumentsEqual(recovery, document) || this.stageRecovery(document))
      : false
    return { document, backend: 'localstorage', durable, imported: imported !== null }
  }

  /** Synchronous crash/pagehide journal; safe to call before an async flush. */
  stageRecovery(document: WorkspaceDocumentSnapshot): boolean {
    // Each live tab owns a separate key, so synchronous pagehide writes cannot
    // erase another tab's draft. Conflicts remain frozen until explicit choice.
    if (this.unresolvedConflict) return false
    const base = this.durableHeadKnown
      ? (this.durableHead
          ? { status: 'found', snapshot: this.durableHead } as const
          : { status: 'empty' } as const)
      : this.recoveryBase
    const saved = saveWorkspaceRecovery(
      this.legacyStorage,
      document,
      base,
      this.writerId,
      this.recoveryKey,
    )
    if (saved) this.recoveryBase = base
    return saved
  }

  save(document: WorkspaceDocumentSnapshot): Promise<boolean> {
    if (!this.initialized) return Promise.reject(new Error('Workspace persistence is not initialized'))
    if (this.unresolvedConflict) return Promise.resolve(false)
    const snapshot = parseWorkspaceDocumentValue(document)
    if (!snapshot) return Promise.resolve(false)
    const requestBase = this.latestSaveRequest ?? this.durableHead
    if (requestBase && !workspaceDocumentAdvances(requestBase, snapshot)) return Promise.resolve(false)
    this.latestSaveRequest = snapshot
    // Stage synchronously before queueing. If an earlier write discovers a
    // cross-tab conflict, the newest already-requested local draft must still
    // survive in its tab-owned journal.
    const recoverySaved = this.stageRecovery(snapshot)
    return this.enqueue(() => this.saveDirect(snapshot, recoverySaved))
  }

  retry(document: WorkspaceDocumentSnapshot): Promise<WorkspaceRetryResult> {
    if (!this.initialized) return Promise.reject(new Error('Workspace persistence is not initialized'))
    if (this.unresolvedConflict) return Promise.resolve({ saved: false })
    const snapshot = parseWorkspaceDocumentValue(document)
    if (!snapshot) return Promise.resolve({ saved: false })
    return this.enqueue(() => this.retryDirect(snapshot))
  }

  resolveConflict(preferCurrentDraft: boolean, currentDocument: WorkspaceDocumentSnapshot): Promise<WorkspaceRetryResult> {
    if (!this.initialized) return Promise.reject(new Error('Workspace persistence is not initialized'))
    if (!this.unresolvedConflict) return Promise.resolve({ saved: false })
    const snapshot = parseWorkspaceDocumentValue(currentDocument)
    if (!snapshot) return Promise.resolve({ saved: false })
    return this.enqueue(() => this.resolveConflictDirect(preferCurrentDraft, snapshot))
  }

  async close(): Promise<void> {
    await this.writeTail
    await this.repository.close()
  }

  private enqueue<T>(operation: () => Promise<T>): Promise<T> {
    const result = this.writeTail.then(operation, operation)
    this.writeTail = result.then(() => undefined, () => undefined)
    return result
  }

  private async saveDirect(document: WorkspaceDocumentSnapshot, recoverySaved: boolean): Promise<boolean> {
    if (this.unresolvedConflict) return false
    if (this.durableHead && !workspaceDocumentAdvances(this.durableHead, document)) return false
    if (this.indexedDbState === 'ready') {
      try {
        const previousHead = this.durableHead
        await this.repository.save(document, previousHead)
        this.durableHead = document
        this.durableHeadKnown = true
        this.backendValue = 'indexeddb'
        this.retireRawLegacy()
        this.retireResolvedRecoveries(previousHead, document)
        return true
      } catch (error) {
        this.recordIndexedDbFailure(error)
        this.backendValue = 'localstorage'
        // A CAS conflict stays visible as an unsaved/error state even though
        // the synchronous journal preserves this tab's draft for recovery.
        if (error instanceof WorkspaceIndexedDbConflictError) return false
      }
    }
    return recoverySaved
  }

  private async retryDirect(document: WorkspaceDocumentSnapshot): Promise<WorkspaceRetryResult> {
    if (this.unresolvedConflict) return { saved: false }
    const userDocumentIsAuthoritative = this.initialDocumentAuthoritative
      || !workspaceDocumentsEqual(document, this.initialDocument)
    const recoverySaved = userDocumentIsAuthoritative ? this.stageRecovery(document) : false

    let loadResult: WorkspaceRepositoryLoadResult
    try {
      loadResult = await this.repository.load()
    } catch {
      this.indexedDbState = 'unavailable'
      this.backendValue = 'localstorage'
      return { saved: recoverySaved }
    }
    if (loadResult.status === 'invalid') {
      this.indexedDbState = 'invalid'
      this.durableHeadKnown = false
      this.backendValue = 'localstorage'
      return { saved: recoverySaved && userDocumentIsAuthoritative }
    }

    this.indexedDbState = 'ready'
    const indexed = loadResult.status === 'found' ? loadResult.snapshot : null
    this.durableHead = indexed
    this.durableHeadKnown = true
    if (indexed && workspaceDocumentsEqual(indexed, document)) {
      this.stageRecovery(indexed)
      this.backendValue = 'indexeddb'
      this.retireRawLegacy()
      this.retireResolvedRecoveries(indexed)
      return { saved: true }
    }

    if (indexed && !userDocumentIsAuthoritative) {
      this.backendValue = 'indexeddb'
      this.retireRawLegacy()
      return { saved: true, restoredDocument: indexed }
    }

    if (indexed && !this.importedAuthoritative) {
      const basedOnCurrentHead = this.recoveryBase.status === 'found'
        && workspaceDocumentsEqual(indexed, this.recoveryBase.snapshot)
      if (!basedOnCurrentHead) {
        this.unresolvedConflict = true
        this.conflictingRecovery = document
        this.backendValue = 'localstorage'
        return { saved: false }
      }
    }

    // Either IDB is empty, the local snapshot is a strictly newer generation,
    // or the user explicitly imported a shared document. The just-read head is
    // the CAS base, so another tab still cannot be overwritten silently.
    const staged = recoverySaved || this.stageRecovery(document)
    try {
      await this.repository.save(document, indexed)
      this.durableHead = document
      this.durableHeadKnown = true
      this.indexedDbState = 'ready'
      this.backendValue = 'indexeddb'
      this.retireRawLegacy()
      this.retireResolvedRecoveries(indexed, document)
      return { saved: true }
    } catch (error) {
      this.recordIndexedDbFailure(error)
      this.backendValue = 'localstorage'
      return { saved: error instanceof WorkspaceIndexedDbConflictError ? false : staged }
    }
  }

  private recordIndexedDbFailure(error: unknown): void {
    if (error instanceof WorkspaceIndexedDbConflictError) {
      this.indexedDbState = 'ready'
      this.durableHead = error.current
      this.durableHeadKnown = true
      this.unresolvedConflict = true
      this.conflictingRecovery = loadWorkspaceRecovery(this.legacyStorage, this.recoveryKey)?.snapshot ?? null
    } else if (error instanceof WorkspaceIndexedDbInvalidDataError) {
      this.indexedDbState = 'invalid'
      this.durableHeadKnown = false
    } else {
      this.indexedDbState = 'unavailable'
    }
  }

  private async resolveConflictDirect(
    preferCurrentDraft: boolean,
    currentDocument: WorkspaceDocumentSnapshot,
  ): Promise<WorkspaceRetryResult> {
    let loadResult: WorkspaceRepositoryLoadResult
    try {
      loadResult = await this.repository.load()
    } catch {
      this.backendValue = 'localstorage'
      return { saved: false }
    }
    if (loadResult.status === 'invalid') return { saved: false }
    const indexed = loadResult.status === 'found' ? loadResult.snapshot : null
    const chosen = preferCurrentDraft ? currentDocument : indexed
    if (!chosen) return { saved: false }

    this.unresolvedConflict = false
    this.conflictingRecovery = null
    this.indexedDbState = 'ready'
    this.durableHead = indexed
    this.durableHeadKnown = true
    const staged = this.stageRecovery(chosen)
    try {
      if (!workspaceDocumentsEqual(indexed, chosen)) await this.repository.save(chosen, indexed)
      this.durableHead = chosen
      this.backendValue = 'indexeddb'
      this.retireRawLegacy()
      this.retireResolvedRecoveries(indexed, chosen)
      return { saved: true, restoredDocument: chosen }
    } catch (error) {
      this.recordIndexedDbFailure(error)
      this.backendValue = 'localstorage'
      return {
        saved: error instanceof WorkspaceIndexedDbConflictError ? false : staged,
        ...(error instanceof WorkspaceIndexedDbConflictError ? {} : { restoredDocument: chosen }),
      }
    }
  }

  private retireRawLegacy(): void {
    try { this.legacyStorage.removeItem(WORKSPACE_LEGACY_SOURCE_KEY) } catch { /* Recovery snapshot is already durable. */ }
    try {
      const tabs = this.legacyStorage.getItem(WORKSPACE_LEGACY_TABS_KEY)
      if (!tabs) return
      const existingBackup = this.legacyStorage.getItem(WORKSPACE_LEGACY_TABS_BACKUP_KEY)
      if (existingBackup === null) this.legacyStorage.setItem(WORKSPACE_LEGACY_TABS_BACKUP_KEY, tabs)
      if (this.legacyStorage.getItem(WORKSPACE_LEGACY_TABS_BACKUP_KEY) !== tabs) return
      // A single-document workspace cannot yet make every donor tab user-
      // addressable. Preserve the original keys even after backing them up;
      // the authoritative IndexedDB head makes this import one-shot without
      // pretending the remaining tabs were migrated.
    } catch { /* Keep donor keys unless their complete backup is confirmed. */ }
  }

  private loadRecoveryEntries(): StoredRecovery[] {
    let keys: string[] = []
    try { keys = this.legacyStorage.keys() } catch { /* Storage enumeration is optional recovery input. */ }
    const recoveryKeys = new Set([
      WORKSPACE_STORAGE_KEY,
      ...keys.filter(key => key.startsWith(WORKSPACE_RECOVERY_KEY_PREFIX)),
    ])
    const entries: StoredRecovery[] = []
    for (const key of recoveryKeys) {
      const record = loadWorkspaceRecovery(this.legacyStorage, key)
      if (record) entries.push({ key, record })
    }
    return entries
  }

  private retireResolvedRecoveries(...resolved: Array<WorkspaceDocumentSnapshot | null>): void {
    for (const entry of this.loadRecoveryEntries()) {
      if (entry.key === this.recoveryKey) continue
      if (!resolved.some(document => workspaceDocumentsEqual(entry.record.snapshot, document))) continue
      try { this.legacyStorage.removeItem(entry.key) } catch { /* The active IDB head is already durable. */ }
    }
  }

}
