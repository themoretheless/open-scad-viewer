/**
 * Cross-tab mutual exclusion for draft writes when `navigator.locks` is unavailable.
 *
 * A lease row `{name, owner, expires}` lives in its own IndexedDB database. Acquisition
 * is a compare-and-set inside one readwrite transaction (IndexedDB serializes readwrite
 * transactions over the same store across tabs), so at most one live owner exists. A
 * crashed holder cannot release, so every lease expires after `ttlMs`; a live holder
 * renews it every `ttlMs / 3`. If a holder stalls past expiry and a second tab takes
 * over, the revision compare-and-set in the draft head still rejects the stale writer
 * with DRAFT_CONFLICT, so expiry can surface a conflict but never lose a write.
 */
interface Lease {
  name: string
  owner: string
  expires: number
}
export interface SolidDraftLeaseOptions {
  ttlMs?: number
  timeoutMs?: number
  retryMs?: number
  now?: () => number
}
const databaseName = 'scad-solid-draft-leases-v1',
  storeName = 'leases'
export const LOCKING_UNAVAILABLE = 'Safe draft locking is unavailable. Open a secure browser context or download JSON.'

function database(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const r = indexedDB.open(databaseName, 1)
    r.onupgradeneeded = () => r.result.createObjectStore(storeName, { keyPath: 'name' })
    r.onsuccess = () => resolve(r.result)
    r.onerror = () => reject(r.error ?? Error('Draft lease database failed.'))
  })
}

/** One atomic read-decide-write over the lease row. */
async function update(name: string, decide: (row: Lease | undefined) => Lease | null | 'delete'): Promise<boolean> {
  const db = await database()
  try {
    return await new Promise<boolean>((resolve, reject) => {
      const tx = db.transaction(storeName, 'readwrite', { durability: 'strict' }),
        store = tx.objectStore(storeName),
        r = store.get(name)
      let changed = false
      r.onsuccess = () => {
        const next = decide(r.result as Lease | undefined)
        if (next === 'delete') store.delete(name)
        else if (next) store.put(next)
        changed = next !== null
      }
      tx.oncomplete = () => resolve(changed)
      tx.onabort = tx.onerror = () => reject(tx.error ?? Error('Draft lease transaction failed.'))
    })
  } finally {
    db.close()
  }
}

export async function withSolidDraftLease<T>(
  name: string,
  action: () => T | Promise<T>,
  { ttlMs = 15_000, timeoutMs = 10_000, retryMs = 50, now = Date.now }: SolidDraftLeaseOptions = {},
): Promise<T> {
  if (typeof indexedDB === 'undefined') throw Error(LOCKING_UNAVAILABLE)
  const owner = crypto.randomUUID(),
    started = now()
  const acquire = () =>
    update(name, row => {
      const at = now()
      return row && row.owner !== owner && row.expires > at ? null : { name, owner, expires: at + ttlMs }
    })
  while (!(await acquire())) {
    if (now() - started >= timeoutMs) throw Error('DRAFT_LOCK_TIMEOUT')
    await new Promise(resolve => setTimeout(resolve, retryMs))
  }
  const renew = setInterval(() => {
    void update(name, row => (row?.owner === owner ? { name, owner, expires: now() + ttlMs } : null)).catch(() => {})
  }, ttlMs / 3)
  try {
    return await action()
  } finally {
    clearInterval(renew)
    await update(name, row => (row?.owner === owner ? 'delete' : null)).catch(() => {})
  }
}
