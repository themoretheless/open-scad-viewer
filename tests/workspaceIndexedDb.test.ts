import { IDBFactory } from 'fake-indexeddb'
import { describe, expect, it } from 'vitest'
import {
  MAX_WORKSPACE_SOURCE_LENGTH,
  WORKSPACE_LEGACY_SOURCE_KEY,
  WORKSPACE_LEGACY_ACTIVE_TAB_KEY,
  WORKSPACE_LEGACY_TABS_BACKUP_KEY,
  WORKSPACE_LEGACY_TABS_KEY,
  WORKSPACE_STORAGE_KEY,
  createWorkspaceDocument,
  loadWorkspaceRecovery,
  updateWorkspaceDocument,
  workspaceDocumentsEqual,
  type WorkspaceDocumentSnapshot,
} from '../src/services/workspaceDocument'
import {
  IndexedDbWorkspaceRepository,
  WORKSPACE_DATABASE_NAME,
  WORKSPACE_OBJECT_STORE,
  WorkspaceIndexedDbConflictError,
  type WorkspaceRepositoryLoadResult,
  type WorkspaceSnapshotRepository,
} from '../src/services/workspaceIndexedDb'
import {
  BrowserWorkspacePersistence,
  type RemovableWorkspaceStorage,
} from '../src/services/workspacePersistence'

class MemoryStorage implements RemovableWorkspaceStorage {
  readonly values = new Map<string, string>()
  failWrites = false
  getItem(key: string) { return this.values.get(key) ?? null }
  setItem(key: string, value: string) {
    if (this.failWrites) throw new Error('storage unavailable')
    this.values.set(key, value)
  }
  removeItem(key: string) { this.values.delete(key) }
  keys() { return [...this.values.keys()].sort() }
}

class ControlledRepository implements WorkspaceSnapshotRepository {
  value: WorkspaceDocumentSnapshot | null = null
  loadError: Error | null = null
  saveError: Error | null = null
  saveGate: Promise<void> | null = null
  readonly started: Array<{
    snapshot: WorkspaceDocumentSnapshot
    expected: WorkspaceDocumentSnapshot | null | undefined
  }> = []

  async load(): Promise<WorkspaceRepositoryLoadResult> {
    if (this.loadError) throw this.loadError
    return this.value
      ? { status: 'found', snapshot: { ...this.value } }
      : { status: 'empty' }
  }
  async save(snapshot: WorkspaceDocumentSnapshot, expected?: WorkspaceDocumentSnapshot | null) {
    this.started.push({ snapshot: { ...snapshot }, expected: expected ? { ...expected } : expected })
    if (this.saveGate) await this.saveGate
    if (this.saveError) throw this.saveError
    if (expected !== undefined && !workspaceDocumentsEqual(this.value, expected)) {
      throw new WorkspaceIndexedDbConflictError(this.value ? { ...this.value } : null)
    }
    this.value = { ...snapshot }
  }
  async close() {}
}

function snapshot(
  source: string,
  updatedAt: number,
  revision = 0,
  mutation = revision,
  documentId = 'workspace-test',
) {
  return createWorkspaceDocument(source, {
    documentId,
    fileName: 'part.scad',
    updatedAt,
    revision,
    mutation,
  })
}

function requestResult<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error)
  })
}

function recoveryRecords(storage: MemoryStorage) {
  return storage.keys()
    .map(key => loadWorkspaceRecovery(storage, key))
    .filter((record): record is NonNullable<typeof record> => record !== null)
}

function recoverySnapshot(storage: MemoryStorage) {
  const records = recoveryRecords(storage)
  return records.sort((left, right) => right.snapshot.updatedAt - left.snapshot.updatedAt)[0]?.snapshot ?? null
}

describe('IndexedDB workspace persistence', () => {
  it('round-trips the active snapshot across repository instances', async () => {
    const factory = new IDBFactory()
    const first = new IndexedDbWorkspaceRepository(factory)
    const document = snapshot('cube(4);', 10, 3)

    expect(await first.load()).toEqual({ status: 'empty' })
    await first.save(document, null)
    await first.close()

    const reopened = new IndexedDbWorkspaceRepository(factory)
    await expect(reopened.load()).resolves.toEqual({ status: 'found', snapshot: document })
    await reopened.close()
  })

  it('rejects an oversized save without replacing the last valid head', async () => {
    const repository = new IndexedDbWorkspaceRepository(new IDBFactory())
    const valid = snapshot('cube(2);', 11, 1)
    await repository.save(valid, null)
    const oversized: WorkspaceDocumentSnapshot = {
      ...valid,
      source: 'x'.repeat(MAX_WORKSPACE_SOURCE_LENGTH + 1),
      revision: valid.revision + 1,
      mutation: valid.mutation + 1,
      updatedAt: valid.updatedAt + 1,
    }

    await expect(repository.save(oversized, valid)).rejects.toThrow(RangeError)
    await expect(repository.load()).resolves.toEqual({ status: 'found', snapshot: valid })
    await repository.close()
  })

  it('rejects malformed structured-cloned rows instead of trusting IndexedDB', async () => {
    const factory = new IDBFactory()
    const repository = new IndexedDbWorkspaceRepository(factory)
    await repository.load()
    const database = await requestResult(factory.open(WORKSPACE_DATABASE_NAME))
    const transaction = database.transaction(WORKSPACE_OBJECT_STORE, 'readwrite')
    transaction.objectStore(WORKSPACE_OBJECT_STORE).put({
      slot: 'active',
      snapshot: { schemaVersion: 999, source: 'bad' },
    })
    await new Promise<void>((resolve, reject) => {
      transaction.oncomplete = () => resolve()
      transaction.onabort = () => reject(transaction.error)
    })
    database.close()

    await expect(repository.load()).resolves.toEqual({ status: 'invalid' })
    await repository.close()
  })

  it('migrates the exact legacy snapshot only after its IndexedDB commit', async () => {
    const storage = new MemoryStorage()
    const legacy = snapshot('cylinder(h=4, r=2);', 20, 7)
    storage.values.set(WORKSPACE_STORAGE_KEY, JSON.stringify(legacy))
    storage.values.set(WORKSPACE_LEGACY_SOURCE_KEY, 'obsolete')
    const repository = new ControlledRepository()
    const persistence = new BrowserWorkspacePersistence(repository, storage)

    const result = await persistence.initialize({ fallbackSource: 'fallback' })

    expect(result).toMatchObject({ document: legacy, backend: 'indexeddb', durable: true })
    expect(repository.value).toEqual(legacy)
    expect(recoverySnapshot(storage)).toEqual(legacy)
    expect(storage.values.has(WORKSPACE_LEGACY_SOURCE_KEY)).toBe(false)
  })

  it('keeps an exact synchronous recovery journal after each IndexedDB commit', async () => {
    const storage = new MemoryStorage()
    const repository = new ControlledRepository()
    const persistence = new BrowserWorkspacePersistence(repository, storage)
    const initialized = await persistence.initialize({ fallbackSource: 'cube(1);' })

    expect(repository.value).toEqual(initialized.document)
    expect(recoverySnapshot(storage)).toEqual(initialized.document)

    const edited = updateWorkspaceDocument(initialized.document, { source: 'cube(6);' }, 25)
    expect(await persistence.save(edited)).toBe(true)
    expect(repository.value).toEqual(edited)
    expect(recoverySnapshot(storage)).toEqual(edited)
  })

  it('keeps legacy recovery and falls back when IndexedDB cannot open', async () => {
    const storage = new MemoryStorage()
    const legacy = snapshot('sphere(3);', 30, 2)
    storage.values.set(WORKSPACE_STORAGE_KEY, JSON.stringify(legacy))
    const repository = new ControlledRepository()
    repository.loadError = new Error('IndexedDB disabled')
    const persistence = new BrowserWorkspacePersistence(repository, storage)

    const result = await persistence.initialize({ fallbackSource: 'fallback' })

    expect(result).toMatchObject({ document: legacy, backend: 'localstorage', durable: true })
    expect(recoverySnapshot(storage)).toEqual(legacy)
  })

  it('does not write a generated fallback after a temporary load error and restores the existing head on retry', async () => {
    const storage = new MemoryStorage()
    const existing = snapshot('indexed-existing();', 31, 2, 3)
    const repository = new ControlledRepository()
    repository.value = existing
    repository.loadError = new Error('IndexedDB temporarily unavailable')
    const persistence = new BrowserWorkspacePersistence(repository, storage)

    const initialized = await persistence.initialize({ fallbackSource: 'generated-fallback();' })

    expect(initialized).toMatchObject({ backend: 'localstorage', durable: false })
    expect(initialized.document.source).toBe('generated-fallback();')
    expect(storage.values.has(WORKSPACE_STORAGE_KEY)).toBe(false)
    expect(repository.started).toHaveLength(0)

    repository.loadError = null
    await expect(persistence.retry(initialized.document)).resolves.toEqual({
      saved: true,
      restoredDocument: existing,
    })
    expect(repository.value).toEqual(existing)
    expect(repository.started).toHaveLength(0)
    // The controller returns the recovered head without touching a potentially
    // newer journal; App stages it only after confirming the editor did not
    // change while retry was in flight.
    expect(storage.values.has(WORKSPACE_STORAGE_KEY)).toBe(false)
  })

  it('never lets an untimestamped raw legacy source overwrite IndexedDB', async () => {
    const storage = new MemoryStorage()
    storage.values.set(WORKSPACE_LEGACY_SOURCE_KEY, 'stale-legacy();')
    const repository = new ControlledRepository()
    repository.value = snapshot('indexed-current();', 35, 4)
    const persistence = new BrowserWorkspacePersistence(repository, storage)

    const result = await persistence.initialize({ fallbackSource: 'fallback' })

    expect(result.document.source).toBe('indexed-current();')
    expect(storage.values.has(WORKSPACE_LEGACY_SOURCE_KEY)).toBe(false)
  })

  it('migrates a raw legacy source into the exact versioned recovery snapshot', async () => {
    const storage = new MemoryStorage()
    storage.values.set(WORKSPACE_LEGACY_SOURCE_KEY, 'legacy-cylinder();')
    const repository = new ControlledRepository()
    const persistence = new BrowserWorkspacePersistence(repository, storage)

    const result = await persistence.initialize({ fallbackSource: 'fallback' })

    expect(result).toMatchObject({ backend: 'indexeddb', durable: true })
    expect(result.document.source).toBe('legacy-cylinder();')
    expect(repository.value).toEqual(result.document)
    expect(recoverySnapshot(storage)).toEqual(result.document)
    expect(storage.values.has(WORKSPACE_LEGACY_SOURCE_KEY)).toBe(false)
  })

  it('backs up every donor tab and preserves the original until multi-file migration exists', async () => {
    const storage = new MemoryStorage()
    const donorTabs = JSON.stringify([
      { id: 'one', name: 'one', code: 'cube(1);' },
      { id: 'two', name: 'two', code: 'sphere(2);' },
    ])
    storage.values.set(WORKSPACE_LEGACY_TABS_KEY, donorTabs)
    storage.values.set(WORKSPACE_LEGACY_ACTIVE_TAB_KEY, 'two')
    storage.values.set(WORKSPACE_LEGACY_SOURCE_KEY, 'stale();')
    const repository = new ControlledRepository()
    const persistence = new BrowserWorkspacePersistence(repository, storage)

    const result = await persistence.initialize({ fallbackSource: 'fallback' })

    expect(result.document).toMatchObject({ source: 'sphere(2);', fileName: 'two.scad' })
    expect(repository.value).toEqual(result.document)
    expect(storage.values.get(WORKSPACE_LEGACY_TABS_BACKUP_KEY)).toBe(donorTabs)
    expect(storage.values.get(WORKSPACE_LEGACY_TABS_KEY)).toBe(donorTabs)
    expect(storage.values.get(WORKSPACE_LEGACY_ACTIVE_TAB_KEY)).toBe('two')
    expect(storage.values.has(WORKSPACE_LEGACY_SOURCE_KEY)).toBe(false)
  })

  it('gives a shared import precedence and retains its hash recovery when every save fails', async () => {
    const storage = new MemoryStorage()
    storage.values.set(WORKSPACE_STORAGE_KEY, JSON.stringify(snapshot('old();', 40)))
    storage.failWrites = true
    const repository = new ControlledRepository()
    repository.saveError = new Error('quota exceeded')
    const persistence = new BrowserWorkspacePersistence(repository, storage)

    const result = await persistence.initialize({
      fallbackSource: 'fallback',
      importedSource: 'cube(9);',
      importedFileName: 'shared-model.scad',
    })

    expect(result.document).toMatchObject({ source: 'cube(9);', fileName: 'shared-model.scad' })
    expect(result).toMatchObject({ imported: true, durable: false, backend: 'localstorage' })
    expect(storage.values.has(WORKSPACE_STORAGE_KEY)).toBe(true)
  })

  it('does not replace an existing IndexedDB workspace until a shared import is explicitly accepted', async () => {
    const storage = new MemoryStorage()
    const repository = new ControlledRepository()
    const existing = snapshot('private-workspace();', 41, 2, 2)
    repository.value = existing
    const persistence = new BrowserWorkspacePersistence(repository, storage)

    const result = await persistence.initialize({ fallbackSource: 'fallback', importedSource: 'shared();' })

    expect(result).toMatchObject({ imported: true, durable: false, backend: 'localstorage' })
    expect(result.document.source).toBe('shared();')
    expect(repository.value).toEqual(existing)
    expect(persistence.hasConflict).toBe(true)
    await expect(persistence.resolveConflict(false, result.document)).resolves.toEqual({
      saved: true,
      restoredDocument: existing,
    })
    expect(repository.value).toEqual(existing)
  })

  it('serializes rapid saves so the older snapshot cannot finish last', async () => {
    const storage = new MemoryStorage()
    const repository = new ControlledRepository()
    const persistence = new BrowserWorkspacePersistence(repository, storage)
    const initialized = await persistence.initialize({ fallbackSource: 'cube(1);' })
    repository.started.splice(0)
    let releaseFirst!: () => void
    repository.saveGate = new Promise(resolve => { releaseFirst = resolve })
    const older = updateWorkspaceDocument(initialized.document, { source: 'cube(2);' }, 50)
    const newer = updateWorkspaceDocument(older, { source: 'cube(3);' }, 51)

    const first = persistence.save(older)
    const second = persistence.save(newer)
    await Promise.resolve()
    expect(repository.started.map(value => value.snapshot.source)).toEqual(['cube(2);'])
    releaseFirst()
    await first
    repository.saveGate = null
    await second

    expect(repository.started.map(value => value.snapshot.source)).toEqual(['cube(2);', 'cube(3);'])
    expect(repository.value).toEqual(newer)
  })

  it('rejects stale and unrelated normal saves without replacing the durable head', async () => {
    const storage = new MemoryStorage()
    const repository = new ControlledRepository()
    const persistence = new BrowserWorkspacePersistence(repository, storage)
    const initialized = await persistence.initialize({ fallbackSource: 'base();' })
    const newer = updateWorkspaceDocument(initialized.document, { source: 'newer();' }, 52)
    expect(await persistence.save(newer)).toBe(true)

    expect(await persistence.save(initialized.document)).toBe(false)
    expect(await persistence.save(snapshot('unrelated();', 53, 10, 10, 'other-document'))).toBe(false)
    expect(repository.value).toEqual(newer)
  })

  it('can retry IndexedDB after a localStorage fallback', async () => {
    const storage = new MemoryStorage()
    const repository = new ControlledRepository()
    repository.loadError = new Error('temporarily unavailable')
    const persistence = new BrowserWorkspacePersistence(repository, storage)
    await persistence.initialize({ fallbackSource: 'cube(1);' })
    repository.loadError = null
    const edited = snapshot('cube(8);', 60, 1)

    expect(await persistence.retry(edited)).toEqual({ saved: true })
    expect(persistence.backend).toBe('indexeddb')
    expect(repository.value).toEqual(edited)
    expect(recoverySnapshot(storage)).toEqual(edited)
  })

  it('uses compare-and-swap to preserve a newer IndexedDB head from another tab', async () => {
    const factory = new IDBFactory()
    const seedRepository = new IndexedDbWorkspaceRepository(factory)
    const base = snapshot('base();', 70, 1, 1)
    await seedRepository.save(base, null)

    const storage = new MemoryStorage()
    const persistenceRepository = new IndexedDbWorkspaceRepository(factory)
    const persistence = new BrowserWorkspacePersistence(persistenceRepository, storage)
    await persistence.initialize({ fallbackSource: 'fallback' })

    const remote = updateWorkspaceDocument(base, { source: 'remote();' }, 71)
    const local = updateWorkspaceDocument(base, { source: 'local();' }, 72)
    await seedRepository.save(remote, base)

    expect(await persistence.save(local)).toBe(false)
    await expect(seedRepository.load()).resolves.toEqual({ status: 'found', snapshot: remote })
    expect(recoverySnapshot(storage)).toEqual(local)

    await persistence.close()
    await seedRepository.close()
  })

  it('preserves an ambiguous recovery journal instead of overwriting it on pagehide staging', async () => {
    const storage = new MemoryStorage()
    const recovery = snapshot('recovery();', 80, 1, 1, 'recovery-document')
    storage.values.set(WORKSPACE_STORAGE_KEY, JSON.stringify(recovery))
    const repository = new ControlledRepository()
    repository.value = snapshot('indexed();', 81, 1, 1, 'indexed-document')
    const persistence = new BrowserWorkspacePersistence(repository, storage)

    const result = await persistence.initialize({ fallbackSource: 'fallback' })

    expect(result).toMatchObject({ document: recovery, durable: false, backend: 'localstorage' })
    expect(persistence.stageRecovery(result.document)).toBe(false)
    expect(recoverySnapshot(storage)).toEqual(recovery)

    await expect(persistence.resolveConflict(true, result.document)).resolves.toEqual({
      saved: true,
      restoredDocument: recovery,
    })
    expect(persistence.hasConflict).toBe(false)
    expect(repository.value).toEqual(recovery)
  })

  it('replays a recovery journal only when its causal base still matches IndexedDB', async () => {
    const storage = new MemoryStorage()
    const repository = new ControlledRepository()
    const base = snapshot('base();', 85, 1, 1)
    repository.value = base
    const firstSession = new BrowserWorkspacePersistence(repository, storage)
    await firstSession.initialize({ fallbackSource: 'fallback' })
    const recovered = updateWorkspaceDocument(base, { source: 'recovered();' }, 86)
    expect(firstSession.stageRecovery(recovered)).toBe(true)
    expect(recoveryRecords(storage).find(record => workspaceDocumentsEqual(record.snapshot, recovered))?.base)
      .toEqual({ status: 'found', snapshot: base })

    repository.started.splice(0)
    const nextSession = new BrowserWorkspacePersistence(repository, storage)
    const result = await nextSession.initialize({ fallbackSource: 'fallback' })

    expect(result).toMatchObject({ document: recovered, durable: true, backend: 'indexeddb' })
    expect(repository.value).toEqual(recovered)
    expect(repository.started.map(entry => entry.expected)).toEqual([base])
  })

  it('surfaces a conflict when the IDB head moved away from the journal base', async () => {
    const storage = new MemoryStorage()
    const repository = new ControlledRepository()
    const base = snapshot('base();', 87, 1, 1)
    repository.value = base
    const firstSession = new BrowserWorkspacePersistence(repository, storage)
    await firstSession.initialize({ fallbackSource: 'fallback' })
    const local = updateWorkspaceDocument(base, { source: 'local();' }, 88)
    expect(firstSession.stageRecovery(local)).toBe(true)
    const remote = updateWorkspaceDocument(base, { source: 'remote();' }, 89)
    repository.value = remote
    repository.started.splice(0)

    const nextSession = new BrowserWorkspacePersistence(repository, storage)
    const result = await nextSession.initialize({ fallbackSource: 'fallback' })

    expect(result).toMatchObject({ document: local, durable: false, backend: 'localstorage' })
    expect(repository.value).toEqual(remote)
    expect(repository.started).toHaveLength(0)
    expect(recoverySnapshot(storage)).toEqual(local)
    expect(nextSession.stageRecovery(remote)).toBe(false)
  })

  it('keeps separate recovery journals for concurrent live tabs', async () => {
    const storage = new MemoryStorage()
    const repository = new ControlledRepository()
    const base = snapshot('base();', 89, 1, 1)
    repository.value = base
    const firstTab = new BrowserWorkspacePersistence(repository, storage)
    const secondTab = new BrowserWorkspacePersistence(repository, storage)
    await firstTab.initialize({ fallbackSource: 'fallback' })
    await secondTab.initialize({ fallbackSource: 'fallback' })
    const firstDraft = updateWorkspaceDocument(base, { source: 'first-tab();' }, 90)
    const secondDraft = updateWorkspaceDocument(base, { source: 'second-tab();' }, 91)

    expect(await firstTab.save(firstDraft)).toBe(true)
    expect(await secondTab.save(secondDraft)).toBe(false)
    expect(repository.value).toEqual(firstDraft)
    expect(recoveryRecords(storage).map(record => record.snapshot.source).sort())
      .toEqual(['first-tab();', 'second-tab();'])
  })

  it('blocks already queued writes after the first compare-and-swap conflict', async () => {
    const storage = new MemoryStorage()
    const repository = new ControlledRepository()
    const base = snapshot('base();', 90, 1, 1)
    repository.value = base
    const persistence = new BrowserWorkspacePersistence(repository, storage)
    await persistence.initialize({ fallbackSource: 'fallback' })
    repository.started.splice(0)

    let releaseFirst!: () => void
    repository.saveGate = new Promise(resolve => { releaseFirst = resolve })
    const firstLocal = updateWorkspaceDocument(base, { source: 'local-1();' }, 91)
    const secondLocal = updateWorkspaceDocument(firstLocal, { source: 'local-2();' }, 92)
    const remote = updateWorkspaceDocument(base, { source: 'remote();' }, 93)
    const firstSave = persistence.save(firstLocal)
    const secondSave = persistence.save(secondLocal)
    await Promise.resolve()
    repository.value = remote
    releaseFirst()

    await expect(firstSave).resolves.toBe(false)
    await expect(secondSave).resolves.toBe(false)
    expect(repository.value).toEqual(remote)
    expect(repository.started.map(entry => entry.snapshot.source)).toEqual(['local-1();'])
    expect(recoverySnapshot(storage)).toEqual(secondLocal)
  })
})
