import { afterEach, describe, expect, it, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import {
  createSolidDraftStorageIo,
  recoverSolidDraft,
  saveSolidDraft,
  type SolidDraftSaveStage,
  type SolidDraftStorageIo,
} from '../src/kernel/solidDraftSaveProtocol'
import {
  collectSolidDraftSnapshots,
  listSolidDraftConflicts,
  readSolidDraftSnapshot,
  solidDraftSnapshotId,
} from '../src/kernel/solidDraftStore'
import { LOCKING_UNAVAILABLE, withSolidDraftLease } from '../src/kernel/solidDraftLease'

afterEach(() => vi.unstubAllGlobals())

const key = 'scad-solid-modeler-v1'
const stages: SolidDraftSaveStage[] = [
  'locked',
  'snapshot-written',
  'head-written',
  'pointer-published',
  'previous-removed',
]

/** Shared browser profile: one IndexedDB factory and one localStorage map seen by every tab. */
function profile(locks: 'web' | 'none' = 'web') {
  vi.stubGlobal('indexedDB', new IDBFactory())
  // A crashed process releases its Web Locks, so the in-process stand-in never blocks a reload.
  vi.stubGlobal(
    'navigator',
    locks === 'web'
      ? { locks: { request: (_n: string, _o: unknown, action: () => unknown) => Promise.resolve(action()) } }
      : {},
  )
  const pointers = new Map<string, string>()
  // Snapshot cleanup reads references from localStorage directly.
  vi.stubGlobal('localStorage', { getItem: (k: string) => pointers.get(k) ?? null })
  return createSolidDraftStorageIo({ get: k => pointers.get(k) ?? null, set: (k, v) => (pointers.set(k, v), true) })
}

/** One tab's view of the draft, mirroring useDraftPersistence state. */
function tab(io: SolidDraftStorageIo) {
  const state = { revision: null as string | null, pointer: null as string | null }
  return {
    state,
    async load() {
      state.revision = (await io.readHead(key))?.revision ?? null
      state.pointer = io.getPointer(key)
    },
    save(text: string, inlineLimit: number, stage?: (s: SolidDraftSaveStage) => void | Promise<void>) {
      return saveSolidDraft(io, {
        key,
        text,
        expectedRevision: () => state.revision,
        expectedPointer: state.pointer,
        inlineLimit,
        current: () => true,
        stage,
        committed: r => {
          if (r.revision) state.revision = r.revision
          if (r.pointer !== null) state.pointer = r.pointer
        },
      })
    },
  }
}

const never = () => new Promise<void>(() => {})

describe('crash at every write stage', () => {
  for (const large of [false, true])
    for (const crashAt of stages) {
      if (!large && crashAt === 'snapshot-written') continue
      it(`recovers a complete draft after a crash at ${crashAt} (${large ? 'snapshot' : 'inline'} pointer)`, async () => {
        const io = profile()
        const limit = large ? 4 : 1_000_000,
          oldText = '{"old":true}',
          newText = '{"new":true}'
        const writer = tab(io)
        await writer.save(oldText, limit)

        let reached!: () => void
        const crashed = new Promise<void>(resolve => (reached = resolve))
        // The renderer dies at `crashAt`: no later stage, cleanup or finally block runs.
        void writer.save(newText, limit, s => {
          if (s !== crashAt) return
          reached()
          return never()
        })
        await crashed

        const recovered = await recoverSolidDraft(io, key)
        const committed = stages.indexOf(crashAt) >= stages.indexOf('head-written')
        expect(recovered?.text).toBe(committed ? newText : oldText)

        // Whatever pointer survived references a complete version, never a torn or missing row.
        const pointer = io.getPointer(key)!
        const pointed = solidDraftSnapshotId(pointer) ? await readSolidDraftSnapshot(pointer) : pointer
        expect([oldText, newText]).toContain(pointed)

        // Orphan cleanup after the grace period keeps everything recovery and the pointer need.
        await collectSolidDraftSnapshots(Date.now() + 25 * 60 * 60 * 1000)
        expect((await recoverSolidDraft(io, key))?.text).toBe(recovered?.text)
        if (solidDraftSnapshotId(pointer)) expect(await readSolidDraftSnapshot(pointer)).toBe(pointed)

        // The reloaded tab saves again without a false conflict.
        const reloaded = tab(io)
        await reloaded.load()
        await expect(reloaded.save('{"after":true}', limit)).resolves.toMatchObject({ pointer: expect.any(String) })
        expect((await recoverSolidDraft(io, key))?.text).toBe('{"after":true}')
      })
    }
})

describe('two tabs', () => {
  for (const locks of ['web', 'none'] as const) {
    it(`surfaces a conflict and keeps both versions (${locks === 'web' ? 'Web Locks' : 'IndexedDB lease'})`, async () => {
      const io = profile(locks)
      const a = tab(io),
        b = tab(io)
      await a.save('{"base":1}', 1_000_000)
      await b.load()
      await a.save('{"a":1}', 1_000_000)
      await expect(b.save('{"b":1}', 1_000_000)).rejects.toThrow('DRAFT_CONFLICT')
      expect((await recoverSolidDraft(io, key))?.text).toBe('{"a":1}')
      expect((await listSolidDraftConflicts(key)).map(c => c.text)).toEqual(['{"b":1}'])
    })

    it(`rejects a stale revision even when the pointer was never published (${locks})`, async () => {
      const io = profile(locks)
      // Tab A's process is gone, so it holds no lock or lease (expiry is covered separately below).
      const a = tab({ ...io, lock: (_key, action) => action() }),
        b = tab(io)
      await a.save('{"base":1}', 1_000_000)
      await b.load()
      // Tab A commits its head and dies before publishing the pointer.
      let reached!: () => void
      const crashed = new Promise<void>(resolve => (reached = resolve))
      void a.save('{"a":1}', 1_000_000, s => (s === 'head-written' ? (reached(), never()) : undefined))
      await crashed
      expect(io.getPointer(key)).toBe(b.state.pointer)
      await expect(b.save('{"b":1}', 1_000_000)).rejects.toThrow('DRAFT_CONFLICT')
      expect((await recoverSolidDraft(io, key))?.text).toBe('{"a":1}')
      expect((await listSolidDraftConflicts(key)).map(c => c.text)).toEqual(['{"b":1}'])
    })
  }

  it('serializes simultaneous saves without Web Locks: one wins, the other is a recoverable conflict', async () => {
    const io = profile('none')
    expect(globalThis.navigator.locks).toBeUndefined()
    const a = tab(io),
      b = tab(io)
    const results = await Promise.allSettled([a.save('{"a":1}', 4), b.save('{"b":1}', 4)])
    const won = results.findIndex(r => r.status === 'fulfilled')
    expect(won).toBeGreaterThanOrEqual(0)
    expect(results[1 - won]).toMatchObject({ status: 'rejected', reason: { message: 'DRAFT_CONFLICT' } })
    const winner = won === 0 ? '{"a":1}' : '{"b":1}',
      loser = won === 0 ? '{"b":1}' : '{"a":1}'
    expect((await recoverSolidDraft(io, key))?.text).toBe(winner)
    expect(await readSolidDraftSnapshot(io.getPointer(key)!)).toBe(winner)
    expect((await listSolidDraftConflicts(key)).map(c => c.text)).toEqual([loser])
  })
})

describe('IndexedDB lease without Web Locks', () => {
  it('never runs two holders at once', async () => {
    vi.stubGlobal('indexedDB', new IDBFactory())
    let active = 0,
      peak = 0
    const work = () =>
      withSolidDraftLease(
        'x',
        async () => {
          peak = Math.max(peak, ++active)
          await new Promise(resolve => setTimeout(resolve, 5))
          active--
        },
        { retryMs: 1 },
      )
    await Promise.all([work(), work(), work(), work()])
    expect(peak).toBe(1)
  })

  it('takes over an expired lease from a crashed holder and times out on a live one', async () => {
    vi.stubGlobal('indexedDB', new IDBFactory())
    let clock = 1_000
    const now = () => clock
    // A holder that died mid-write: its row stays and nothing renews or releases it.
    await new Promise<void>((resolve, reject) => {
      const r = indexedDB.open('scad-solid-draft-leases-v1', 1)
      r.onupgradeneeded = () => r.result.createObjectStore('leases', { keyPath: 'name' })
      r.onsuccess = () => {
        const tx = r.result.transaction('leases', 'readwrite')
        tx.objectStore('leases').put({ name: 'x', owner: 'dead', expires: clock + 100 })
        tx.oncomplete = () => (r.result.close(), resolve())
        tx.onerror = () => reject(tx.error)
      }
    })
    await expect(withSolidDraftLease('x', () => 'late', { ttlMs: 100, timeoutMs: 0, now })).rejects.toThrow(
      'DRAFT_LOCK_TIMEOUT',
    )
    clock += 101
    await expect(withSolidDraftLease('x', () => 'recovered', { ttlMs: 100, now })).resolves.toBe('recovered')
  })

  it('refuses explicitly when neither Web Locks nor IndexedDB exist', async () => {
    vi.stubGlobal('indexedDB', undefined)
    await expect(withSolidDraftLease('x', () => 1)).rejects.toThrow(LOCKING_UNAVAILABLE)
  })
})
