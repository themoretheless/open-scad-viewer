/**
 * The durable draft write sequence, separated from UI state so every stage boundary can be
 * crash-tested.
 *
 * Stages, in order, all under the per-key cross-tab lock:
 *   1. `locked`            lock held; the localStorage pointer is compared with this tab's view.
 *   2. `snapshot-written`  (large drafts only) an immutable snapshot row exists; nothing refers to it.
 *   3. `head-written`      the head row `{revision, text}` was replaced by one strict-durability
 *                          compare-and-set transaction. This is the commit point.
 *   4. `pointer-published` localStorage holds the inline text or snapshot marker (cross-tab signal).
 *   5. `previous-removed`  the previous snapshot row is deleted.
 *
 * Recovery reads the head first and falls back to the pointer only when no head exists (legacy
 * drafts). The head is a single row written atomically, so a crash before stage 3 recovers the old
 * complete draft and a crash after it recovers the new one. A stale pointer after a crash between 3
 * and 4 only references the previous complete version; an orphan snapshot from a crash between 2
 * and 3 or 4 and 5 is removed by `collectSolidDraftSnapshots` after its grace period.
 *
 * Concurrent tabs: the head compare-and-set rejects a writer whose expected revision is stale, and
 * the pointer comparison rejects a writer that missed another tab's publication. Either rejection
 * is DRAFT_CONFLICT, and the rejected text is stored as a conflict copy before the error surfaces,
 * so both the winning head and the losing version stay recoverable.
 */
import { readSolidDraftHead, writeSolidDraftHead, type SolidDraftHead } from './solidDraftHeadStore'
import {
  readSolidDraftSnapshot,
  removeSolidDraftSnapshot,
  solidDraftSnapshotId,
  withSolidDraftLock,
  writeSolidDraftConflict,
  writeSolidDraftSnapshot,
} from './solidDraftStore'

/** Production storage: IndexedDB rows plus the given pointer store (safeStorage over localStorage). */
export function createSolidDraftStorageIo(pointers: {
  get(key: string): string | null
  set(key: string, value: string): boolean
}): SolidDraftStorageIo {
  // Each member resolves the export at call time so live bindings and module-level spies apply.
  return {
    readHead: key => readSolidDraftHead(key),
    writeHead: (key, expected, text, current) => writeSolidDraftHead(key, expected, text, current),
    readSnapshot: marker => readSolidDraftSnapshot(marker),
    writeSnapshot: text => writeSolidDraftSnapshot(text),
    removeSnapshot: marker => removeSolidDraftSnapshot(marker),
    writeConflict: (key, text) => writeSolidDraftConflict(key, text),
    getPointer: key => pointers.get(key),
    setPointer: (key, value) => pointers.set(key, value),
    lock: (key, action) => withSolidDraftLock(key, action),
  }
}

export type SolidDraftSaveStage =
  'locked' | 'snapshot-written' | 'head-written' | 'pointer-published' | 'previous-removed'

export interface SolidDraftStorageIo {
  readHead(key: string): Promise<SolidDraftHead | null>
  writeHead(key: string, expected: string | null, text: string, current: () => boolean): Promise<SolidDraftHead>
  readSnapshot(marker: string): Promise<string>
  writeSnapshot(text: string): Promise<string>
  removeSnapshot(marker: string | null): Promise<void>
  writeConflict(key: string, text: string): Promise<unknown>
  getPointer(key: string): string | null
  setPointer(key: string, value: string): boolean
  lock<T>(key: string, action: () => Promise<T>): Promise<T>
}

export interface SolidDraftSaveRequest {
  key: string
  text: string
  /** Read under the lock, after any earlier save from this tab has committed. */
  expectedRevision: () => string | null
  expectedPointer: string | null
  inlineLimit: number
  current: () => boolean
  /** Invoked under the lock with the committed result, before the next queued save starts. */
  committed?: (result: SolidDraftSaveResult) => void
  /** Awaited under the lock before anything is compared (initial head read). */
  ready?: Promise<unknown>
  /** Test hook invoked after each completed stage. */
  stage?: (stage: SolidDraftSaveStage) => void | Promise<void>
}

/** `pointer` is null when the head was committed but a newer local save superseded publication. */
export type SolidDraftSaveResult = { revision: string | null; pointer: string | null }

export async function saveSolidDraft(
  io: SolidDraftStorageIo,
  request: SolidDraftSaveRequest,
): Promise<SolidDraftSaveResult> {
  const { key, text, expectedPointer, inlineLimit, current } = request
  const done = (result: SolidDraftSaveResult) => {
    request.committed?.(result)
    return result
  }
  const conflict = async (): Promise<never> => {
    await io.writeConflict(key, text).catch(() => {})
    throw Error('DRAFT_CONFLICT')
  }
  return io.lock(key, async () => {
    await request.ready
    if (request.stage) await request.stage('locked')
    if (!current()) return { revision: null, pointer: null }
    if (io.getPointer(key) !== expectedPointer) await conflict()
    let marker: string | undefined,
      published = false
    try {
      if (text.length > inlineLimit) {
        marker = await io.writeSnapshot(text)
        if (request.stage) await request.stage('snapshot-written')
      }
      if (!current()) return { revision: null, pointer: null }
      let head: SolidDraftHead
      try {
        head = await io.writeHead(key, request.expectedRevision(), text, current)
      } catch (e) {
        if (e instanceof Error && e.message === 'DRAFT_CONFLICT') await conflict()
        throw e
      }
      if (request.stage) await request.stage('head-written')
      if (!current()) return done({ revision: head.revision, pointer: null })
      const pointer = marker ?? text
      if (!io.setPointer(key, pointer)) {
        done({ revision: head.revision, pointer: null })
        throw Error('DRAFT_REFERENCE_FAILED')
      }
      published = true
      if (request.stage) await request.stage('pointer-published')
      const result = done({ revision: head.revision, pointer })
      await io.removeSnapshot(expectedPointer).catch(() => {})
      if (request.stage) await request.stage('previous-removed')
      return result
    } finally {
      if (marker && !published) await io.removeSnapshot(marker).catch(() => {})
    }
  })
}

/** Resolve the recoverable text: the atomic head wins; a pointer is used only for pre-head drafts. */
export async function solidDraftRecoveryText(
  head: SolidDraftHead | null,
  pointer: string | null,
  readSnapshot: (marker: string) => Promise<string>,
): Promise<string | null> {
  if (head) return head.text
  if (!pointer) return null
  return solidDraftSnapshotId(pointer) ? readSnapshot(pointer) : pointer
}

export async function recoverSolidDraft(
  io: SolidDraftStorageIo,
  key: string,
): Promise<{ text: string; revision: string | null } | null> {
  const head = await io.readHead(key),
    text = await solidDraftRecoveryText(head, io.getPointer(key), marker => io.readSnapshot(marker))
  return text === null ? null : { text, revision: head?.revision ?? null }
}
