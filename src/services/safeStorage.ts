/**
 * SafeStorage — the single access point for localStorage.
 *
 * Every read validates and falls back; every write catches quota errors and
 * reports them through an injectable callback (dependency inversion: the UI
 * layer registers a toast handler, this module stays framework-free).
 *
 * Rationale: the app previously had ~98 direct localStorage call sites, several
 * of which silently swallowed QuotaExceededError and lost user data
 * (see TOP-50-ISSUES.md criticals #1, #6–#10, #26).
 */

export type StorageFailureHandler = (key: string, error: unknown) => void

let onWriteFailure: StorageFailureHandler | null = null

/** UI layer registers a handler (e.g. show a toast) for failed writes. */
export function setStorageFailureHandler(handler: StorageFailureHandler) {
  onWriteFailure = handler
}

/** Raw string read; never throws (private mode / disabled storage safe). */
export function storageGet(key: string): string | null {
  try { return localStorage.getItem(key) } catch { return null }
}

/**
 * Raw string write; never throws. Returns true on success.
 * On failure invokes the registered failure handler so the user is told
 * their data was NOT saved instead of silently losing it.
 */
export function storageSet(key: string, value: string): boolean {
  try {
    localStorage.setItem(key, value)
    return true
  } catch (e) {
    onWriteFailure?.(key, e)
    return false
  }
}

/** Remove a key; never throws. */
export function storageRemove(key: string): void {
  try { localStorage.removeItem(key) } catch { /* storage unavailable */ }
}

/** JSON read with fallback and optional shape validation. */
export function storageGetJSON<T>(key: string, fallback: T, validate?: (v: unknown) => boolean): T {
  const raw = storageGet(key)
  if (raw === null) return fallback
  try {
    const v = JSON.parse(raw)
    if (v === null || v === undefined) return fallback
    if (validate && !validate(v)) return fallback
    return v as T
  } catch {
    return fallback
  }
}

/** JSON write; returns true on success, reports failures like storageSet. */
export function storageSetJSON(key: string, value: unknown): boolean {
  return storageSet(key, JSON.stringify(value))
}

/** Integer read with NaN guard and optional clamping. */
export function storageGetInt(key: string, fallback: number, min?: number, max?: number): number {
  const raw = storageGet(key)
  if (raw === null) return fallback
  const n = parseInt(raw, 10)
  if (!Number.isFinite(n)) return fallback
  let v = n
  if (min !== undefined) v = Math.max(min, v)
  if (max !== undefined) v = Math.min(max, v)
  return v
}

/** Float read with NaN guard and optional clamping. */
export function storageGetFloat(key: string, fallback: number, min?: number, max?: number): number {
  const raw = storageGet(key)
  if (raw === null) return fallback
  const n = parseFloat(raw)
  if (!Number.isFinite(n)) return fallback
  let v = n
  if (min !== undefined) v = Math.max(min, v)
  if (max !== undefined) v = Math.min(max, v)
  return v
}

/** Enum read: only returns the stored value if it is one of `allowed`. */
export function storageGetEnum<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
  const raw = storageGet(key)
  return (allowed as readonly string[]).includes(raw ?? '') ? (raw as T) : fallback
}

/** Boolean read stored as 'true'/'false'; anything else → fallback. */
export function storageGetBool(key: string, fallback: boolean): boolean {
  const raw = storageGet(key)
  if (raw === 'true') return true
  if (raw === 'false') return false
  return fallback
}
