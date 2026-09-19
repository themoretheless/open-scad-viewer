import {
  parseWorkspaceDocumentValue,
  requireValidWorkspaceDocument,
  workspaceDocumentsEqual,
  type WorkspaceDocumentSnapshot,
} from './workspaceDocument'

export const WORKSPACE_DATABASE_NAME = 'open-scad-viewer'
export const WORKSPACE_DATABASE_VERSION = 2
export const WORKSPACE_OBJECT_STORE = 'workspace'
export const WORKSPACE_STEP_MODEL_STORE = 'step-models'
const ACTIVE_WORKSPACE_SLOT = 'active'

interface WorkspaceRow {
  slot: typeof ACTIVE_WORKSPACE_SLOT
  snapshot: WorkspaceDocumentSnapshot
}

export type WorkspaceRepositoryLoadResult =
  | { status: 'empty' }
  | { status: 'found'; snapshot: WorkspaceDocumentSnapshot }
  | { status: 'invalid' }

export interface WorkspaceSnapshotRepository {
  load(): Promise<WorkspaceRepositoryLoadResult>
  /**
   * `expected` enables a compare-and-swap commit. `undefined` is reserved for
   * low-level migrations/tests that intentionally perform an unconditional
   * validated write.
   */
  save(snapshot: WorkspaceDocumentSnapshot, expected?: WorkspaceDocumentSnapshot | null): Promise<void>
  close(): Promise<void>
}

export class WorkspaceIndexedDbUnavailableError extends Error {
  constructor(message = 'IndexedDB is unavailable') {
    super(message)
    this.name = 'WorkspaceIndexedDbUnavailableError'
  }
}

export class WorkspaceIndexedDbBlockedError extends Error {
  constructor() {
    super('IndexedDB upgrade is blocked by another open tab')
    this.name = 'WorkspaceIndexedDbBlockedError'
  }
}

export class WorkspaceIndexedDbInvalidDataError extends Error {
  constructor() {
    super('IndexedDB contains an unsupported or invalid workspace row')
    this.name = 'WorkspaceIndexedDbInvalidDataError'
  }
}

export class WorkspaceIndexedDbConflictError extends Error {
  constructor(readonly current: WorkspaceDocumentSnapshot | null) {
    super('The IndexedDB workspace changed in another tab')
    this.name = 'WorkspaceIndexedDbConflictError'
  }
}

function requestResult<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error ?? new Error('IndexedDB request failed'))
  })
}

function transactionComplete(transaction: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve()
    transaction.onabort = () => reject(transaction.error ?? new Error('IndexedDB transaction was aborted'))
    transaction.onerror = () => reject(transaction.error ?? new Error('IndexedDB transaction failed'))
  })
}

/** Browser-only repository for the active workspace snapshot. */
export class IndexedDbWorkspaceRepository implements WorkspaceSnapshotRepository {
  private databasePromise: Promise<IDBDatabase> | null = null

  constructor(
    private readonly factory: IDBFactory | undefined = globalThis.indexedDB,
    private readonly onVersionChange?: () => void,
    private readonly openTimeoutMs = 3_000,
  ) {}

  async load(): Promise<WorkspaceRepositoryLoadResult> {
    const database = await this.database()
    const transaction = database.transaction(WORKSPACE_OBJECT_STORE, 'readonly')
    const request = transaction.objectStore(WORKSPACE_OBJECT_STORE).get(ACTIVE_WORKSPACE_SLOT)
    const [value] = await Promise.all([
      requestResult<WorkspaceRow | undefined>(request),
      transactionComplete(transaction),
    ])
    if (!value) return { status: 'empty' }
    if (value.slot !== ACTIVE_WORKSPACE_SLOT) return { status: 'invalid' }
    const snapshot = parseWorkspaceDocumentValue(value.snapshot)
    return snapshot ? { status: 'found', snapshot } : { status: 'invalid' }
  }

  async save(snapshot: WorkspaceDocumentSnapshot, expected?: WorkspaceDocumentSnapshot | null): Promise<void> {
    const validated = requireValidWorkspaceDocument(snapshot)
    const database = await this.database()
    const transaction = database.transaction(WORKSPACE_OBJECT_STORE, 'readwrite')
    const completion = transactionComplete(transaction)
    const store = transaction.objectStore(WORKSPACE_OBJECT_STORE)
    const currentRow = await requestResult<WorkspaceRow | undefined>(store.get(ACTIVE_WORKSPACE_SLOT))
    const current = currentRow ? parseWorkspaceDocumentValue(currentRow.snapshot) : null
    if (currentRow && !current) {
      await completion
      throw new WorkspaceIndexedDbInvalidDataError()
    }
    if (expected !== undefined && !workspaceDocumentsEqual(current, expected)) {
      await completion
      throw new WorkspaceIndexedDbConflictError(current)
    }
    // A retry after an uncertain response may observe the exact committed
    // value. Treat it as an idempotent success without rewriting the row.
    if (workspaceDocumentsEqual(current, validated)) {
      await completion
      return
    }
    const request = store.put({
      slot: ACTIVE_WORKSPACE_SLOT,
      snapshot: { ...validated },
    } satisfies WorkspaceRow)
    await Promise.all([requestResult(request), completion])
  }

  async close(): Promise<void> {
    const pending = this.databasePromise
    this.databasePromise = null
    if (!pending) return
    try { (await pending).close() } catch { /* A failed open has nothing to close. */ }
  }

  private database(): Promise<IDBDatabase> {
    if (this.databasePromise) return this.databasePromise
    if (!this.factory) return Promise.reject(new WorkspaceIndexedDbUnavailableError())

    const pending = new Promise<IDBDatabase>((resolve, reject) => {
      const request = this.factory!.open(WORKSPACE_DATABASE_NAME, WORKSPACE_DATABASE_VERSION)
      let settled = false
      const timeout = setTimeout(() => fail(new WorkspaceIndexedDbUnavailableError('Timed out opening IndexedDB')), this.openTimeoutMs)
      const fail = (error: Error) => {
        if (settled) return
        settled = true
        clearTimeout(timeout)
        reject(error)
      }

      request.onupgradeneeded = () => {
        const database = request.result
        if (!database.objectStoreNames.contains(WORKSPACE_OBJECT_STORE)) {
          database.createObjectStore(WORKSPACE_OBJECT_STORE, { keyPath: 'slot' })
        }
        if (!database.objectStoreNames.contains(WORKSPACE_STEP_MODEL_STORE)) {
          database.createObjectStore(WORKSPACE_STEP_MODEL_STORE, { keyPath: 'slot' })
        }
      }
      request.onblocked = () => fail(new WorkspaceIndexedDbBlockedError())
      request.onerror = () => fail(request.error ?? new WorkspaceIndexedDbUnavailableError('Could not open IndexedDB'))
      request.onsuccess = () => {
        const database = request.result
        if (settled) {
          database.close()
          return
        }
        settled = true
        clearTimeout(timeout)
        database.onversionchange = () => {
          database.close()
          if (this.databasePromise === pending) this.databasePromise = null
          this.onVersionChange?.()
        }
        resolve(database)
      }
    })
    this.databasePromise = pending
    void pending.catch(() => {
      if (this.databasePromise === pending) this.databasePromise = null
    })
    return pending
  }
}
